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

use anyhow::Result;
use rstest::*;
use sqlx::{AssertSqlSafe, Executor};
use std::time::{Duration, Instant};
use tests::fixtures::*;

#[rstest]
#[async_std::test]
async fn periodic_sweep_discovers_idle_indexes_across_databases(database: Db) -> Result<()> {
    let other = Db::new().await;
    let mut conn = database.connection().await;
    let mut other_conn = other.connection().await;
    for conn in [&mut conn, &mut other_conn] {
        conn.execute("CREATE EXTENSION IF NOT EXISTS pg_search CASCADE")
            .await?;
        conn.execute(
            "CREATE TABLE sweep_a (id bigint PRIMARY KEY, body text);
             CREATE TABLE sweep_b (LIKE sweep_a INCLUDING ALL);
             CREATE TABLE sweep_no_background_layers (LIKE sweep_a INCLUDING ALL);
             CREATE INDEX sweep_a_idx ON sweep_a USING paradedb (id, body)
                 WITH (mutable_segment_rows = 10000);
             CREATE INDEX sweep_b_idx ON sweep_b USING paradedb (id, body)
                 WITH (mutable_segment_rows = 10000);
             CREATE INDEX sweep_no_background_layers_idx ON sweep_no_background_layers USING paradedb (id, body)
                 WITH (mutable_segment_rows = 10000, background_layer_sizes = '0');
             INSERT INTO sweep_a VALUES (1, 'batch');
             INSERT INTO sweep_b VALUES (1, 'batch');
             INSERT INTO sweep_no_background_layers VALUES (1, 'batch');",
        )
        .await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM sweep_a WHERE body ||| 'batch'")
            .fetch_one(&mut *conn)
            .await?;
        assert_eq!(count, 1);
    }
    conn.execute("SELECT pg_reload_conf()").await?;
    let mut writer = database.connection().await;
    writer
        .execute("BEGIN; INSERT INTO sweep_a VALUES (2, 'aborted')")
        .await?;
    let mut worker_pids = Vec::new();
    for phase in 0..2 {
        if phase == 1 {
            let stopped: bool = sqlx::query_scalar(
                "SELECT bool_and(pg_terminate_backend(pid)) FROM pg_stat_activity
                 WHERE backend_type = 'pg_search mutable sweep launcher'",
            )
            .fetch_one(&mut conn)
            .await?;
            assert!(stopped);
            for conn in [&mut conn, &mut other_conn] {
                conn.execute("INSERT INTO sweep_a VALUES (3, 'batch'); INSERT INTO sweep_b VALUES (3, 'batch'); INSERT INTO sweep_no_background_layers VALUES (3, 'batch')").await?;
            }
        }
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let mut drained = true;
            for conn in [&mut conn, &mut other_conn] {
                let mutable: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM
                       (SELECT * FROM paradedb.index_info('sweep_a_idx')
                        UNION ALL SELECT * FROM paradedb.index_info('sweep_b_idx')
                        UNION ALL SELECT * FROM paradedb.index_info('sweep_no_background_layers_idx')) i
                     WHERE visible AND mutable",
                )
                .fetch_one(&mut *conn)
                .await?;
                drained &= mutable == 0;
                let visible: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM sweep_a WHERE body ||| 'batch'")
                        .fetch_one(&mut *conn)
                        .await?;
                assert_eq!(visible, if phase == 0 { 1 } else { 2 });
                let aborted: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM sweep_a WHERE body ||| 'aborted'")
                        .fetch_one(&mut *conn)
                        .await?;
                assert_eq!(aborted, 0);
            }
            if drained {
                break;
            }
            anyhow::ensure!(
                Instant::now() < deadline,
                "idle mutable segments did not drain in phase {phase}"
            );
            async_std::task::sleep(Duration::from_millis(50)).await;
        }
        let mut current_pids = Vec::new();
        for conn in [&mut conn, &mut other_conn] {
            let pids: Vec<i32> = sqlx::query_scalar(
                "SELECT pid FROM pg_stat_activity WHERE backend_type='pg_search mutable sweep'
                 AND datname=current_database() ORDER BY pid",
            )
            .fetch_all(&mut *conn)
            .await?;
            assert_eq!(pids.len(), 1);
            current_pids.push(pids[0]);
        }
        if phase == 0 {
            worker_pids = current_pids;
            writer.execute("ROLLBACK").await?;
            async_std::task::sleep(Duration::from_secs(2)).await;
        } else {
            assert_eq!(
                current_pids, worker_pids,
                "launcher restart replaced database workers"
            );
        }
    }
    for conn in [&mut conn, &mut other_conn] {
        // Remove the deliberately aborted heap row before checking every indexed CTID.
        conn.execute("VACUUM (INDEX_CLEANUP ON) sweep_a").await?;
        let mutable: i64 = sqlx::query_scalar("SELECT count(*) FROM paradedb.index_info('sweep_no_background_layers_idx') WHERE visible AND mutable").fetch_one(&mut *conn).await?;
        assert_eq!(mutable, 0);
        for index in [
            "sweep_a_idx",
            "sweep_b_idx",
            "sweep_no_background_layers_idx",
        ] {
            let passed: Option<bool> = sqlx::query_scalar("SELECT bool_and(passed) FROM pdb.verify_index($1::text::regclass, heapallindexed => true)").bind(index).fetch_one(&mut *conn).await?;
            assert_eq!(passed, Some(true));
        }
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let idle: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM pg_stat_activity
             WHERE backend_type = 'pg_search mutable sweep' AND datname=current_database()
               AND wait_event_type = 'Extension' AND xact_start IS NULL AND backend_xmin IS NULL
               AND NOT EXISTS (SELECT FROM pg_locks WHERE pid = pg_stat_activity.pid AND relation IS NOT NULL))"
        ).fetch_one(&mut conn).await?;
        if idle {
            break;
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            "database worker never waited without a transaction, xmin or relation locks"
        );
        async_std::task::sleep(Duration::from_millis(20)).await;
    }
    let other_name: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&mut other_conn)
        .await?;
    drop(other_conn);
    conn.execute(AssertSqlSafe(format!(
        "DROP DATABASE \"{}\"",
        other_name.replace('"', "\"\"")
    )))
    .await?;
    Ok(())
}

#[rstest]
#[async_std::test]
async fn periodic_sweep_skips_a_ddl_locked_index(database: Db) -> Result<()> {
    let mut conn = database.connection().await;
    conn.execute(
        "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
                  CREATE TABLE sweep_locked (id bigint PRIMARY KEY, body text);
                  CREATE TABLE sweep_available (LIKE sweep_locked INCLUDING ALL);
                  CREATE INDEX sweep_locked_idx ON sweep_locked USING paradedb(id,body);
                  CREATE INDEX sweep_available_idx ON sweep_available USING paradedb(id,body)
                      WITH(mutable_segment_rows=10000)",
    )
    .await?;
    let mut ddl = database.connection().await;
    ddl.execute("BEGIN; DROP INDEX sweep_locked_idx").await?;
    conn.execute("INSERT INTO sweep_available VALUES(1,'batch')")
        .await?;
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let mutable: i64 = sqlx::query_scalar("SELECT count(*) FROM paradedb.index_info('sweep_available_idx') WHERE visible AND mutable").fetch_one(&mut conn).await?;
        if mutable == 0 {
            break;
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            "a DDL lock on another index blocked the sweep"
        );
        async_std::task::sleep(Duration::from_millis(20)).await;
    }
    let held: bool = sqlx::query_scalar("SELECT EXISTS(SELECT FROM pg_locks WHERE pid=pg_backend_pid() AND locktype='relation' AND mode='AccessExclusiveLock' AND granted)").fetch_one(&mut ddl).await?;
    assert!(held);
    ddl.execute("ROLLBACK").await?;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM sweep_available WHERE body ||| 'batch'")
            .fetch_one(&mut conn)
            .await?;
    assert_eq!(count, 1);
    let passed: Option<bool> = sqlx::query_scalar("SELECT bool_and(passed) FROM pdb.verify_index('sweep_available_idx', heapallindexed=>true)").fetch_one(&mut conn).await?;
    assert_eq!(passed, Some(true));
    Ok(())
}

#[rstest]
#[async_std::test]
async fn database_cleanup_stops_persistent_worker(database: Db) -> Result<()> {
    let other = Db::new().await;
    let mut controller = database.connection().await;
    let mut conn = other.connection().await;
    conn.execute("CREATE EXTENSION IF NOT EXISTS pg_search CASCADE")
        .await?;
    let name: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&mut conn)
        .await?;
    controller.execute("SELECT pg_reload_conf()").await?;
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let started: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT FROM pg_stat_activity
             WHERE backend_type='pg_search mutable sweep' AND datname=$1)",
        )
        .bind(&name)
        .fetch_one(&mut controller)
        .await?;
        if started {
            break;
        }
        anyhow::ensure!(Instant::now() < deadline, "database worker did not start");
        async_std::task::sleep(Duration::from_millis(50)).await;
    }
    drop(conn);
    drop(other);
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT FROM pg_database WHERE datname=$1)")
            .bind(&name)
            .fetch_one(&mut controller)
            .await?;
    assert!(
        !exists,
        "test cleanup left a database and its worker behind"
    );
    Ok(())
}
