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

//! End-to-end abort and lifecycle safety tests for `PgTokioRuntime` and MPP DSM streams across
//! `AggregateScan` and `JoinScan` in serial and MPP execution modes (`parallel_leader_participation`
//! `on` and `off`).
//!
//! Exercises:
//! 1. Repeated `work_mem` exhaustion (`paradedb.spill_to_disk = off`) mid-shuffle / mid-execution.
//! 2. Mid-stream PostgreSQL `ERROR` (`division by zero`) raised in a parent executor node above an
//!    active `AggregateScan` or `JoinScan`.
//! 3. Subtransaction rollback (`SAVEPOINT` / `ROLLBACK TO SAVEPOINT`), both for MPP DSM detach
//!    (`AtEOSubXact_Parallel`) and for nested cursors (a healthy outer cursor surviving subxact
//!    rollback alongside a promoted nested cursor and an outer cursor whose `FETCH` failed inside
//!    the savepoint).
//! 4. Non-portal `_SPI_execute_plan` aborts inside PL/pgSQL `DO` exception blocks.
//! 5. Backend `proc_exit` (`pg_terminate_backend` on a paused portal holding a live
//!    `PgTokioRuntime`), verifying `before_shmem_exit` joins worker threads cleanly without
//!    triggering postmaster crash recovery.

use anyhow::Result;
use rstest::*;
use sqlx::{AssertSqlSafe, Executor, PgConnection};
use std::time::Duration;
use tests::fixtures::*;

#[derive(Clone, Copy, Debug)]
enum ExecutionMode {
    Serial,
    MppLeaderParticipating,
    MppCoordinatorOnly,
}

impl ExecutionMode {
    fn is_mpp(self) -> bool {
        !matches!(self, Self::Serial)
    }

    fn guc_sql(self) -> &'static str {
        match self {
            Self::Serial => {
                r#"
                SET max_parallel_workers_per_gather TO 0;
                SET parallel_leader_participation TO on;
                "#
            }
            Self::MppLeaderParticipating => {
                r#"
                SET paradedb.mpp_min_rows TO 0;
                SET paradedb.min_rows_per_worker TO 10;
                SET max_parallel_workers_per_gather TO 3;
                SET max_parallel_workers TO 16;
                SET parallel_leader_participation TO on;
                SET min_parallel_table_scan_size TO 0;
                SET parallel_setup_cost TO 0;
                SET parallel_tuple_cost TO 0;
                "#
            }
            Self::MppCoordinatorOnly => {
                r#"
                SET paradedb.mpp_min_rows TO 0;
                SET paradedb.min_rows_per_worker TO 10;
                SET max_parallel_workers_per_gather TO 3;
                SET max_parallel_workers TO 16;
                SET parallel_leader_participation TO off;
                SET min_parallel_table_scan_size TO 0;
                SET parallel_setup_cost TO 0;
                SET parallel_tuple_cost TO 0;
                "#
            }
        }
    }
}

const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

CREATE TABLE abort_categories (
    id INT PRIMARY KEY,
    tag TEXT NOT NULL,
    label TEXT NOT NULL
);

CREATE TABLE abort_events (
    id INT PRIMARY KEY,
    category_id INT NOT NULL,
    category_key TEXT NOT NULL,
    group_key TEXT NOT NULL,
    payload TEXT NOT NULL,
    content TEXT NOT NULL
);

CREATE INDEX abort_categories_idx ON abort_categories
USING bm25 (id, (tag::pdb.literal), label)
WITH (
    target_segment_count = 3,
    layer_sizes = '0',
    background_layer_sizes = '0'
);

CREATE INDEX abort_events_idx ON abort_events
USING bm25 (id, category_id, (category_key::pdb.literal), (group_key::pdb.literal), (payload::pdb.literal), content)
WITH (
    target_segment_count = 3,
    layer_sizes = '0',
    background_layer_sizes = '0'
);

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO abort_categories (id, tag, label)
SELECT g, repeat('tag-payload-', 16) || g, 'cat ' || g FROM generate_series(1, 800) AS g;
INSERT INTO abort_categories (id, tag, label)
SELECT g, repeat('tag-payload-', 16) || g, 'cat ' || g FROM generate_series(801, 1600) AS g;
INSERT INTO abort_categories (id, tag, label)
SELECT g, repeat('tag-payload-', 16) || g, 'cat ' || g FROM generate_series(1601, 2500) AS g;

INSERT INTO abort_events (id, category_id, category_key, group_key, payload, content)
SELECT g, 1 + (g % 2500), repeat('tag-payload-', 16) || (1 + (g % 2500)), 'grp-' || g, repeat('x', 48) || g, 'Section alpha ' || g
FROM generate_series(1, 6000) AS g;

INSERT INTO abort_events (id, category_id, category_key, group_key, payload, content)
SELECT g, 1 + (g % 2500), repeat('tag-payload-', 16) || (1 + (g % 2500)), 'grp-' || g, repeat('y', 48) || g, 'Section beta ' || g
FROM generate_series(6001, 12000) AS g;

INSERT INTO abort_events (id, category_id, category_key, group_key, payload, content)
SELECT g, 1 + (g % 2500), repeat('tag-payload-', 16) || (1 + (g % 2500)), 'grp-' || g, repeat('z', 48) || g, 'Section gamma ' || g
FROM generate_series(12001, 18000) AS g;

RESET paradedb.global_mutable_segment_rows;

ANALYZE abort_categories;
ANALYZE abort_events;
"#;

const AGG_QUERY: &str = r#"
SELECT
    e.group_key,
    e.payload,
    COUNT(*) AS cnt,
    SUM(e.category_id) AS sum_cat
FROM abort_events e
WHERE e.content @@@ 'Section'
GROUP BY e.group_key, e.payload
"#;

const JOIN_QUERY: &str = r#"
SELECT
    e.id,
    e.group_key,
    e.payload,
    c.tag
FROM abort_events e
JOIN abort_categories c ON e.category_key = c.tag
WHERE e.content @@@ 'Section'
  AND c.label @@@ 'cat'
ORDER BY e.payload, e.id
LIMIT 15000
"#;

async fn configure_session(
    conn: &mut PgConnection,
    mode: ExecutionMode,
    work_mem: &str,
) -> Result<()> {
    let sql = format!(
        r#"
        SET paradedb.enable_custom_scan TO on;
        SET paradedb.enable_aggregate_custom_scan TO on;
        SET paradedb.enable_join_custom_scan TO on;
        SET paradedb.max_term_agg_buckets TO 1;
        SET paradedb.enable_columnar_exec TO true;
        SET paradedb.spill_to_disk TO off;
        SET enable_indexscan TO off;
        SET hash_mem_multiplier TO 1.0;
        SET work_mem TO '{work_mem}';
        {}
        "#,
        mode.guc_sql()
    );
    conn.execute(AssertSqlSafe(sql)).await?;
    Ok(())
}

async fn explain_plan(conn: &mut PgConnection, query: &str) -> Result<String> {
    let rows: Vec<(String,)> = sqlx::query_as(AssertSqlSafe(format!(
        "EXPLAIN (COSTS OFF, VERBOSE) {query}"
    )))
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n"))
}

async fn backend_pid(conn: &mut PgConnection) -> Result<i32> {
    let (pid,): (i32,) = sqlx::query_as("SELECT pg_backend_pid()")
        .fetch_one(&mut *conn)
        .await?;
    Ok(pid)
}

#[rstest]
#[tokio::test]
async fn abort_safety_across_scan_and_lifecycle_paths(
    database: Db,
    #[values(
        ExecutionMode::Serial,
        ExecutionMode::MppLeaderParticipating,
        ExecutionMode::MppCoordinatorOnly
    )]
    mode: ExecutionMode,
) -> Result<()> {
    let mut conn = database.connection().await;
    let mut witness = database.connection().await;
    conn.execute(SETUP_SQL).await?;

    let initial_pid = backend_pid(&mut conn).await?;
    let witness_pid = backend_pid(&mut witness).await?;

    configure_session(&mut conn, mode, "64MB").await?;
    let agg_plan = explain_plan(&mut conn, AGG_QUERY).await?;
    assert!(
        agg_plan.contains("ParadeDB Aggregate Scan"),
        "expected ParadeDB Aggregate Scan for {mode:?}, got:\n{agg_plan}"
    );
    assert_eq!(
        agg_plan.contains("DistributedExec"),
        mode.is_mpp(),
        "unexpected DistributedExec presence for {mode:?} in agg plan:\n{agg_plan}"
    );

    let join_plan = explain_plan(&mut conn, JOIN_QUERY).await?;
    assert!(
        join_plan.contains("ParadeDB Join Scan"),
        "expected ParadeDB Join Scan for {mode:?}, got:\n{join_plan}"
    );
    assert_eq!(
        join_plan.contains("DistributedExec"),
        mode.is_mpp(),
        "unexpected DistributedExec presence for {mode:?} in join plan:\n{join_plan}"
    );

    // 1. Repeated `work_mem` exhaustion (`spill_to_disk = off`) across both AggregateScan and
    //    JoinScan, exercising the `DsmAsyncStream::copy_into_tx_ring` / `on_dsm_detach` race.
    configure_session(&mut conn, mode, "64kB").await?;
    for iter in 0..8 {
        for (label, query) in [("agg", AGG_QUERY), ("join", JOIN_QUERY)] {
            let Err(err) = sqlx::query(AssertSqlSafe(query)).fetch_all(&mut conn).await else {
                panic!(
                    "iter {iter} ({mode:?} {label}): expected work_mem overflow error with spill_to_disk = off, \
                     but query succeeded\nplan:\n{}",
                    if label == "agg" {
                        &agg_plan
                    } else {
                        &join_plan
                    }
                );
            };
            let msg = err.to_string();
            assert!(
                msg.contains("work_mem")
                    || msg.contains("DiskManager")
                    || msg.contains("Resources exhausted")
                    || msg.contains("temporary files are not enabled")
                    || msg.contains("transport receiver detached"),
                "iter {iter} ({mode:?} {label}): unexpected error: {msg}"
            );
        }
    }
    assert_eq!(backend_pid(&mut conn).await?, initial_pid);

    // 2. Mid-stream PostgreSQL `ERROR` (`division by zero`) raised in a parent executor node above
    //    an active AggregateScan and JoinScan while background Tokio tasks and DSM streams are live.
    configure_session(&mut conn, mode, "64MB").await?;
    let outer_err_agg =
        format!("SELECT group_key, 1 / ((cnt - cnt)::int) FROM ({AGG_QUERY} OFFSET 0) sub");
    let outer_err_join = format!("SELECT id, 1 / (id - id) FROM ({JOIN_QUERY} OFFSET 0) sub");
    for iter in 0..8 {
        for (label, query) in [("agg_div0", &outer_err_agg), ("join_div0", &outer_err_join)] {
            let Err(err) = sqlx::query(AssertSqlSafe(query.as_str()))
                .fetch_all(&mut conn)
                .await
            else {
                panic!("iter {iter} ({mode:?} {label}): expected outer division by zero error");
            };
            assert!(
                err.to_string().to_lowercase().contains("division by zero"),
                "iter {iter} ({mode:?} {label}): expected division by zero, got: {err}"
            );
        }
    }
    assert_eq!(backend_pid(&mut conn).await?, initial_pid);

    // 3a. Subtransaction rollback (`AtEOSubXact_Parallel`) with no open cursors so MPP engages
    //     inside the savepoint and detaches DSM during `AbortSubTransaction`.
    conn.execute("BEGIN").await?;
    for _ in 0..4 {
        conn.execute("SAVEPOINT mpp_sp").await?;
        let err = sqlx::query(AssertSqlSafe(outer_err_agg.as_str()))
            .fetch_all(&mut conn)
            .await
            .expect_err("expected error inside savepoint");
        assert!(err.to_string().to_lowercase().contains("division by zero"));
        conn.execute("ROLLBACK TO SAVEPOINT mpp_sp").await?;
        conn.execute("RELEASE SAVEPOINT mpp_sp").await?;
    }
    conn.execute("COMMIT").await?;

    // 3b. Cursor + savepoint interaction (`ResourceOwner` hierarchy + deferred `PortalDrop`):
    //     - `outer_ok`: declared before `s1`, paused (`PORTAL_READY`), must survive `ROLLBACK TO s1`.
    //     - `outer_fail_closed` & `outer_fail_committed`: declared before `s1`, fetched 2 rows
    //       cleanly, then fail on row 3 inside `s1` (`AtSubAbort_Portals` reparents their
    //       `resowner` onto `s1` without deleting their `portalContext`), and are subsequently
    //       dropped while `IsTransactionState()` is `true` via `CLOSE` and `COMMIT`
    //       (`PreCommit_Portals` -> `PortalDrop`) respectively.
    //     - `inner_promoted`: declared in `s2`, promoted to `s1` via `RELEASE SAVEPOINT s2`, then
    //       rolled back with `s1`.
    conn.execute("BEGIN").await?;
    conn.execute(AssertSqlSafe(format!(
        "DECLARE outer_ok NO SCROLL CURSOR FOR {JOIN_QUERY}"
    )))
    .await?;
    let first_ok_rows = sqlx::query("FETCH 3 FROM outer_ok")
        .fetch_all(&mut conn)
        .await?;
    assert_eq!(first_ok_rows.len(), 3);

    let fail_on_third_row = format!(
        "SELECT id, CASE WHEN rn >= 3 THEN 1 / (id - id) ELSE id END \
         FROM (SELECT id, row_number() OVER () AS rn FROM ({JOIN_QUERY} OFFSET 0) j) numbered"
    );
    for cursor_name in ["outer_fail_closed", "outer_fail_committed"] {
        conn.execute(AssertSqlSafe(format!(
            "DECLARE {cursor_name} NO SCROLL CURSOR FOR {fail_on_third_row}"
        )))
        .await?;
        let pre_sp_rows = sqlx::query(AssertSqlSafe(format!("FETCH 2 FROM {cursor_name}")))
            .fetch_all(&mut conn)
            .await?;
        assert_eq!(pre_sp_rows.len(), 2);
    }

    conn.execute("SAVEPOINT s1").await?;
    conn.execute("SAVEPOINT s2").await?;
    conn.execute(AssertSqlSafe(format!(
        "DECLARE inner_promoted NO SCROLL CURSOR FOR {AGG_QUERY}"
    )))
    .await?;
    let inner_rows = sqlx::query("FETCH 2 FROM inner_promoted")
        .fetch_all(&mut conn)
        .await?;
    assert_eq!(inner_rows.len(), 2);
    conn.execute("RELEASE SAVEPOINT s2").await?;

    conn.execute("SAVEPOINT s_close_branch").await?;
    let Err(fetch_err_closed) = sqlx::query("FETCH 5 FROM outer_fail_closed")
        .fetch_all(&mut conn)
        .await
    else {
        panic!("3rd row of outer_fail_closed must raise division by zero");
    };
    assert!(
        fetch_err_closed
            .to_string()
            .to_lowercase()
            .contains("division by zero")
    );
    conn.execute("ROLLBACK TO SAVEPOINT s_close_branch").await?;
    conn.execute("RELEASE SAVEPOINT s_close_branch").await?;

    let Err(fetch_err_committed) = sqlx::query("FETCH 5 FROM outer_fail_committed")
        .fetch_all(&mut conn)
        .await
    else {
        panic!("3rd row of outer_fail_committed must raise division by zero");
    };
    assert!(
        fetch_err_committed
            .to_string()
            .to_lowercase()
            .contains("division by zero")
    );

    conn.execute("ROLLBACK TO SAVEPOINT s1").await?;

    // Explicitly `CLOSE` one subxact-failed outer portal while `IsTransactionState()` is `true`,
    // and leave `outer_fail_committed` for `COMMIT` (`PreCommit_Portals` -> `PortalDrop`).
    conn.execute("CLOSE outer_fail_closed").await?;

    // `outer_ok` must still have its `PgTokioRuntime` and relation/buffer pins intact.
    let resumed_ok_rows = sqlx::query("FETCH 5 FROM outer_ok")
        .fetch_all(&mut conn)
        .await?;
    assert_eq!(resumed_ok_rows.len(), 5);
    conn.execute("CLOSE outer_ok").await?;
    conn.execute("COMMIT").await?;

    // 4. Non-portal `_SPI_execute_plan` aborts inside a PL/pgSQL `DO` block (`EXCEPTION WHEN`).
    let do_sql = format!(
        r#"
        DO $do$
        DECLARE
            rec RECORD;
        BEGIN
            BEGIN
                FOR rec IN {outer_err_agg} LOOP
                    NULL;
                END LOOP;
                RAISE EXCEPTION 'expected division_by_zero';
            EXCEPTION WHEN division_by_zero THEN
                NULL;
            END;

            BEGIN
                FOR rec IN {outer_err_join} LOOP
                    NULL;
                END LOOP;
                RAISE EXCEPTION 'expected division_by_zero';
            EXCEPTION WHEN division_by_zero THEN
                NULL;
            END;
        END;
        $do$;
        "#
    );
    conn.execute(AssertSqlSafe(do_sql)).await?;

    // 5. `proc_exit` (`pg_terminate_backend`) on a victim backend holding an open DataFusion portal
    //    (`drop_unless_exiting` skips `CustomScanState` drop; `before_shmem_exit` must join
    //    `PgTokioRuntime` worker threads before `PGSharedMemoryDetach`).
    {
        let mut victim = database.connection().await;
        configure_session(&mut victim, mode, "64MB").await?;
        let victim_pid = backend_pid(&mut victim).await?;
        victim.execute("BEGIN").await?;
        victim
            .execute(AssertSqlSafe(format!(
                "DECLARE victim_cur NO SCROLL CURSOR FOR {JOIN_QUERY}"
            )))
            .await?;
        let pulled = sqlx::query("FETCH 2 FROM victim_cur")
            .fetch_all(&mut victim)
            .await?;
        assert_eq!(pulled.len(), 2);

        let (terminated,): (bool,) = sqlx::query_as("SELECT pg_terminate_backend($1)")
            .bind(victim_pid)
            .fetch_one(&mut conn)
            .await?;
        assert!(terminated);

        // Wait for the victim backend to exit `pg_stat_activity`.
        for _ in 0..100 {
            let (still_alive,): (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM pg_stat_activity WHERE pid = $1")
                    .bind(victim_pid)
                    .fetch_one(&mut conn)
                    .await?;
            if still_alive == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    // Verify neither `conn` nor `witness` was reset by postmaster crash recovery, and that a
    // subsequent full MPP/serial execution on `conn` completes cleanly.
    assert_eq!(backend_pid(&mut conn).await?, initial_pid);
    assert_eq!(backend_pid(&mut witness).await?, witness_pid);

    let (group_count,): (i64,) = sqlx::query_as(AssertSqlSafe(format!(
        "SELECT COUNT(*) FROM ({AGG_QUERY}) t"
    )))
    .fetch_one(&mut conn)
    .await?;
    assert_eq!(group_count, 18_000);

    let (join_count,): (i64,) = sqlx::query_as(AssertSqlSafe(format!(
        "SELECT COUNT(*) FROM ({JOIN_QUERY}) t"
    )))
    .fetch_one(&mut conn)
    .await?;
    assert_eq!(join_count, 15_000);

    Ok(())
}
