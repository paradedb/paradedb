// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use rstest::*;
use sqlx::{Executor, PgConnection};
use tests::fixtures::*;

#[rstest]
fn aborted_segments_not_visible(mut conn: PgConnection) {
    r#"
        SET max_parallel_maintenance_workers = 0;
        SET parallel_leader_participation = false;
        DROP TABLE IF EXISTS test_table;
        CREATE TABLE test_table (id SERIAL PRIMARY KEY, value TEXT NOT NULL);
        INSERT INTO test_table (value) VALUES ('committed');

        CREATE INDEX idxtest_table ON public.test_table
        USING paradedb (id, value);
    "#
    .execute(&mut conn);

    // there's one segment, from CREATE INDEX
    let (pre_update_visible_segments,) =
        "SELECT count(*) FROM paradedb.index_info('idxtest_table')".fetch_one::<(i64,)>(&mut conn);

    assert_eq!(pre_update_visible_segments, 1);

    // this will do a merge_on_insert, creating a new segment, even tho its contents will not be
    // visible (because the xact aborted), the segment itself will be
    "BEGIN; UPDATE test_table SET value = 'aborted'; ABORT".execute(&mut conn);

    // so that means this will return two segments.  The original one made by CREATE INDEX and
    // the segment from above
    let (post_visible_segments,) =
        "SELECT count(*) FROM paradedb.index_info('idxtest_table', true)"
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(post_visible_segments, 2);

    // and even tho this will search both segments, it will not return the row from the aborted xact
    let (count,) =
        "SELECT count(*) FROM test_table WHERE value ||| 'aborted'".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);

    // because it's supposed to only return rows from live segments
    let (count,) = "SELECT count(*) FROM test_table WHERE value ||| 'committed'"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}

#[rstest]
fn search_error_caught_in_subtransaction(mut conn: PgConnection) {
    r#"
        DROP TABLE IF EXISTS subxact_table;
        CREATE TABLE subxact_table (id INT PRIMARY KEY, body TEXT, n INT);
        INSERT INTO subxact_table SELECT g, 'hello world', g % 7 FROM generate_series(1, 1000) g;
        CREATE INDEX subxact_idx ON subxact_table USING paradedb (id, body, n);
    "#
    .execute(&mut conn);

    // A subtransaction abort releases the scan's buffer pins before it frees the scan, so the
    // scan's buffers are dropped after their pins are gone.
    "BEGIN".execute(&mut conn);
    r#"
        DO $$
        BEGIN
            PERFORM id, 10 / (n - n) FROM subxact_table WHERE id @@@ paradedb.all();
        EXCEPTION WHEN division_by_zero THEN NULL;
        END $$
    "#
    .execute(&mut conn);

    let (count,) = "SELECT count(*) FROM subxact_table WHERE id @@@ paradedb.all()"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1000);
    "COMMIT".execute(&mut conn);
}

#[rstest]
#[tokio::test]
async fn cleanup_lock_wait_cancelled_then_retried(database: Db) -> anyhow::Result<()> {
    let mut reader = database.connection().await;
    let mut vacuum = database.connection().await;
    reader
        .execute(
            r#"
            CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
            DROP TABLE IF EXISTS cleanup_wait;
            CREATE TABLE cleanup_wait (id INT PRIMARY KEY, body TEXT);
            INSERT INTO cleanup_wait SELECT g, 'hello world' FROM generate_series(1, 5000) g;
            CREATE INDEX cleanup_wait_idx ON cleanup_wait USING paradedb (id, body);
            DELETE FROM cleanup_wait WHERE id <= 100;
            "#,
        )
        .await?;

    wait_for_vacuum_horizon(&mut reader).await?;

    // An open search holds cleanup admission, so VACUUM waits until the timeout cancels it.
    reader.execute("BEGIN").await?;
    reader
        .execute("DECLARE c CURSOR FOR SELECT id FROM cleanup_wait WHERE id @@@ paradedb.all()")
        .await?;
    reader.execute("FETCH 1 FROM c").await?;

    let held: bool = sqlx::query_scalar("SELECT EXISTS(SELECT FROM pg_locks WHERE pid=pg_backend_pid() AND locktype='page' AND relation='cleanup_wait_idx'::regclass AND mode='ShareLock' AND granted)").fetch_one(&mut reader).await?;
    assert!(held, "cursor did not retain cleanup admission");
    vacuum.execute("SET statement_timeout = '1s'").await?;
    let err = vacuum
        .execute("VACUUM cleanup_wait")
        .await
        .expect_err("VACUUM should wait for the cleanup lock and time out");
    let code = err.as_database_error().and_then(|e| e.code());
    assert_eq!(code.as_deref(), Some("57014"), "{err}");

    // The same backend takes the cleanup lock again once the search is gone.
    reader.execute("COMMIT").await?;
    vacuum.execute("RESET statement_timeout").await?;
    vacuum
        .execute("DELETE FROM cleanup_wait WHERE id <= 200")
        .await?;
    vacuum.execute("VACUUM cleanup_wait").await?;
    Ok(())
}

#[rstest]
#[tokio::test]
async fn vacuum_admission_allows_existing_readers_to_finish(database: Db) -> anyhow::Result<()> {
    use std::time::{Duration, Instant};

    let mut observer = database.connection().await;
    let mut reader = database.connection().await;
    let mut vacuum = database.connection().await;
    let mut late_reader = database.connection().await;
    observer.execute(
        "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE; \
         CREATE TABLE cleanup_admission (id int PRIMARY KEY, body text) WITH (autovacuum_enabled=false); \
         INSERT INTO cleanup_admission SELECT g, 'hello world' FROM generate_series(1,5000) g; \
         CREATE INDEX cleanup_admission_idx ON cleanup_admission USING paradedb (id,body); \
         DELETE FROM cleanup_admission WHERE id<=100",
    ).await?;
    wait_for_vacuum_horizon(&mut reader).await?;
    reader.execute("SET statement_timeout='10s'; BEGIN; DECLARE c CURSOR FOR SELECT id FROM cleanup_admission WHERE id @@@ paradedb.all(); FETCH 1 FROM c").await?;
    let held: bool = sqlx::query_scalar("SELECT EXISTS(SELECT FROM pg_locks WHERE pid=pg_backend_pid() AND locktype='page' AND relation='cleanup_admission_idx'::regclass AND mode='ShareLock' AND granted)").fetch_one(&mut reader).await?;
    assert!(held, "cursor did not retain cleanup admission");
    vacuum.execute("SET statement_timeout='10s'").await?;
    late_reader.execute("SET statement_timeout='10s'").await?;
    let vacuum_task = tokio::spawn(async move { vacuum.execute("VACUUM cleanup_admission").await });
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let queued: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM pg_locks WHERE locktype='page' \
             AND relation='cleanup_admission_idx'::regclass AND mode='ExclusiveLock' AND NOT granted)",
        ).fetch_one(&mut observer).await?;
        if queued {
            break;
        }
        if vacuum_task.is_finished() {
            anyhow::bail!(
                "VACUUM finished before queueing for reader admission: {:?}",
                vacuum_task.await?
            );
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            "VACUUM did not queue for reader admission"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let late_task = tokio::spawn(async move {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM cleanup_admission WHERE id @@@ paradedb.all()",
        )
        .fetch_one(&mut late_reader)
        .await
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let queued: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM pg_locks WHERE locktype='page' \
             AND relation='cleanup_admission_idx'::regclass AND mode='ShareLock' AND NOT granted)",
        )
        .fetch_one(&mut observer)
        .await?;
        if queued {
            break;
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            "new reader bypassed queued VACUUM"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let nested: i64 =
        sqlx::query_scalar("SELECT count(*) FROM cleanup_admission WHERE id @@@ paradedb.all()")
            .fetch_one(&mut reader)
            .await?;
    assert_eq!(nested, 4900);
    let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
        .fetch_one(&mut reader)
        .await?;
    reader
        .execute(if version >= 160000 {
            "SET debug_parallel_query=on"
        } else {
            "SET force_parallel_mode=on"
        })
        .await?;
    reader.execute("SET paradedb.enable_custom_scan=false; SET paradedb.enable_aggregate_custom_scan=false; SET max_parallel_workers_per_gather=2; SET parallel_setup_cost=0; SET parallel_tuple_cost=0; SET min_parallel_table_scan_size=0; SET min_parallel_index_scan_size=0").await?;
    let plan: Vec<String> = sqlx::query_scalar("EXPLAIN (ANALYZE, COSTS OFF) SELECT count(*) FROM cleanup_admission WHERE id @@@ paradedb.all()")
        .fetch_all(&mut reader).await?;
    assert!(
        plan.iter().any(|line| line
            .split_once("Workers Launched:")
            .is_some_and(|(_, count)| count.trim().parse::<u32>().is_ok_and(|n| n > 0))),
        "parallel reader was not exercised: {plan:?}"
    );
    reader.execute("FETCH 1 FROM c; COMMIT").await?;
    vacuum_task.await??;
    assert_eq!(late_task.await??, 4900);
    let locks: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_locks WHERE locktype='page' AND relation='cleanup_admission_idx'::regclass")
        .fetch_one(&mut observer).await?;
    assert_eq!(locks, 0);
    let passed: Option<bool> = sqlx::query_scalar("SELECT bool_and(passed) FROM pdb.verify_index('cleanup_admission_idx',heapallindexed=>true)")
        .fetch_one(&mut observer).await?;
    assert_eq!(passed, Some(true));
    Ok(())
}

async fn wait_for_vacuum_horizon(conn: &mut PgConnection) -> anyhow::Result<()> {
    use std::time::{Duration, Instant};

    // A cursor must not retain an xmin older than the fixture's committed deletions.
    let after_setup: String =
        sqlx::query_scalar("SELECT pg_snapshot_xmax(pg_current_snapshot())::text")
            .fetch_one(&mut *conn)
            .await?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let ready: bool =
            sqlx::query_scalar("SELECT pg_snapshot_xmin(pg_current_snapshot()) >= $1::text::xid8")
                .bind(&after_setup)
                .fetch_one(&mut *conn)
                .await?;
        if ready {
            return Ok(());
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            "old snapshots prevented fixture rows from becoming vacuumable"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
