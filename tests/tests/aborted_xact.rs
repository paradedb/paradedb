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

    // An open search pins the index's cleanup-lock page, so VACUUM's ambulkdelete waits in
    // LockBufferForCleanup and the timeout cancels it there.
    reader.execute("BEGIN").await?;
    reader
        .execute("DECLARE c CURSOR FOR SELECT id FROM cleanup_wait WHERE id @@@ paradedb.all()")
        .await?;
    reader.execute("FETCH 1 FROM c").await?;

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
