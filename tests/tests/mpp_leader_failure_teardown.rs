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
//! A leader-side failure between the MPP launch and the first batch must not wedge the backend.
//!
//! The shape: a NOT IN subquery on the probe side of a broadcast join. The distributed planner
//! caps the null-aware anti join to one task and leaves it in the leader's stage, so the leader
//! itself executes that leaf — before it has dispatched plan frames for the other stages. A
//! failure there used to unwind into PostgreSQL's parallel-context teardown, which terminated
//! the workers and waited for them; a worker parked for a plan frame never checked for
//! interrupts, ignored the SIGTERM, and the backend sat in `BgworkerShutdown` until an immediate
//! restart.
//!
//! Two fault paths: `paradedb.mpp_test_panic_in_worker` raises a PostgreSQL error from the
//! leader's `execute()` (longjmp; PostgreSQL's abort path does the teardown), and
//! `paradedb.mpp_test_fail_leader_execute` returns a DataFusion error (the scans' explicit
//! abort does it before raising).

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use rstest::*;
use sqlx::{AssertSqlSafe, Executor, PgConnection};
use tests::fixtures::*;
use tokio::time::{sleep, timeout};

const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

CREATE TABLE mlft_items (id bigint PRIMARY KEY, txt text NOT NULL);
CREATE TABLE mlft_excl  (id bigint PRIMARY KEY, val bigint);
CREATE TABLE mlft_small (id bigint PRIMARY KEY, val bigint NOT NULL, txt text NOT NULL);
CREATE INDEX mlft_items_idx ON mlft_items USING paradedb (id, txt)
  WITH (key_field = 'id', target_segment_count = 8, background_layer_sizes = '0',
        text_fields = '{"txt":{"fast":true}}');
CREATE INDEX mlft_excl_idx ON mlft_excl USING paradedb (id, val)
  WITH (key_field = 'id', target_segment_count = 8, background_layer_sizes = '0',
        numeric_fields = '{"val":{"fast":true}}');
CREATE INDEX mlft_small_idx ON mlft_small USING paradedb (id, val, txt)
  WITH (key_field = 'id', target_segment_count = 8, background_layer_sizes = '0',
        numeric_fields = '{"val":{"fast":true}}', text_fields = '{"txt":{"fast":true}}');

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO mlft_items SELECT s, 'match' FROM generate_series(1, 2000) s;
INSERT INTO mlft_items SELECT s, 'match' FROM generate_series(2001, 4000) s;
INSERT INTO mlft_excl  SELECT s, s FROM generate_series(50, 300) s;
INSERT INTO mlft_excl  SELECT s, s FROM generate_series(301, 600) s;
INSERT INTO mlft_small SELECT s, s, 'small' FROM generate_series(1, 20) s;
INSERT INTO mlft_small SELECT s, s, 'small' FROM generate_series(21, 40) s;
RESET paradedb.global_mutable_segment_rows;
ANALYZE mlft_items; ANALYZE mlft_excl; ANALYZE mlft_small;
"#;

const MPP_GUCS: &str = r#"
SET paradedb.enable_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_aggregate_custom_scan TO on;
SET max_parallel_workers_per_gather TO 4;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
"#;

const JOINSCAN_QUERY: &str = r#"
SELECT i.id FROM mlft_items i JOIN mlft_small s ON s.val = i.id
WHERE i.txt @@@ 'match' AND s.txt @@@ 'small'
  AND i.id NOT IN (SELECT val FROM mlft_excl WHERE id @@@ paradedb.all())
ORDER BY i.id LIMIT 5
"#;

const AGGREGATESCAN_QUERY: &str = r#"
SELECT count(*) FROM (
  SELECT i.id FROM mlft_items i JOIN mlft_small s ON s.val = i.id
  WHERE i.txt @@@ 'match' AND s.txt @@@ 'small'
    AND i.id NOT IN (SELECT val FROM mlft_excl WHERE id @@@ paradedb.all())) q
"#;

/// Hard watchdog: a regression here wedges the backend and its workers, which would otherwise
/// hang the whole shared test cluster.
const WATCHDOG: Duration = Duration::from_secs(30);

async fn launched_worker_pids(conn: &mut PgConnection) -> Result<Vec<i32>> {
    Ok(sqlx::query_scalar(
        "SELECT pid FROM pg_stat_activity WHERE backend_type = 'parallel worker' ORDER BY pid",
    )
    .fetch_all(conn)
    .await?)
}

async fn guc_present(conn: &mut PgConnection, name: &str) -> Result<bool> {
    let row: Option<(String,)> = sqlx::query_as("SELECT name FROM pg_settings WHERE name = $1")
        .bind(name)
        .fetch_optional(conn)
        .await?;
    Ok(row.is_some())
}

/// Run `query` with a leader-side fault armed; the error must come back promptly, the backend
/// must answer again, and every launched worker must exit rather than stay parked.
async fn run_faulted(
    conn: &mut PgConnection,
    observer: &mut PgConnection,
    query: &str,
    expected_error: &str,
) -> Result<()> {
    let result = timeout(WATCHDOG, conn.execute(AssertSqlSafe(query.to_string())))
        .await
        .context("leader-side failure did not return; backend wedged")?;
    let err = match result {
        Ok(_) => bail!("query succeeded although the leader-side fault was armed"),
        Err(err) => err.to_string(),
    };
    assert!(err.contains(expected_error), "unexpected error: {err}");

    let one: i32 = timeout(
        WATCHDOG,
        sqlx::query_scalar("SELECT 1").fetch_one(&mut *conn),
    )
    .await
    .context("backend did not answer after the aborted MPP run")??;
    assert_eq!(one, 1);

    let deadline = Instant::now() + WATCHDOG;
    loop {
        let pids = launched_worker_pids(observer).await?;
        if pids.is_empty() {
            return Ok(());
        }
        if Instant::now() > deadline {
            bail!("parallel workers survived the aborted MPP run: {pids:?}");
        }
        sleep(Duration::from_millis(50)).await;
    }
}

#[rstest]
#[tokio::test]
async fn mpp_leader_failure_tears_down_workers(database: Db) -> Result<()> {
    let mut conn = database.connection().await;
    conn.execute(SETUP_SQL).await?;
    if !guc_present(&mut conn, "paradedb.mpp_test_panic_in_worker").await? {
        println!(
            "Skipping mpp_leader_failure_tears_down_workers: the fault-injection GUCs are not \
             present (non-debug build)"
        );
        return Ok(());
    }
    conn.execute(MPP_GUCS).await?;

    // The shape must actually put a multi-segment leaf in the leader's box, or the faults fire
    // in a worker instead: the leader's box is everything before the first stage separator.
    let plan: Vec<(String,)> = sqlx::query_as(AssertSqlSafe(format!(
        "EXPLAIN (VERBOSE, COSTS OFF) {JOINSCAN_QUERY}"
    )))
    .fetch_all(&mut conn)
    .await?;
    let plan = plan
        .into_iter()
        .map(|(l,)| l)
        .collect::<Vec<_>>()
        .join("\n");
    let leader_box_end = plan.find("└──").context("distributed plan expected")?;
    let leaf = plan
        .find("table=mlft_excl, segments=2")
        .context("2-segment mlft_excl leaf expected")?;
    assert!(
        leaf < leader_box_end,
        "mlft_excl leaf is not leader-hosted:\n{plan}"
    );

    let mut observer = database.connection().await;

    // PostgreSQL error raised inside the leader's execute: teardown runs from the abort path.
    conn.execute("SET paradedb.mpp_test_panic_in_worker TO on")
        .await?;
    run_faulted(&mut conn, &mut observer, JOINSCAN_QUERY, "artificial panic").await?;
    run_faulted(
        &mut conn,
        &mut observer,
        AGGREGATESCAN_QUERY,
        "artificial panic",
    )
    .await?;
    conn.execute("SET paradedb.mpp_test_panic_in_worker TO off")
        .await?;

    // DataFusion error returned from the leader's execute: the scans abort the launch first.
    conn.execute("SET paradedb.mpp_test_fail_leader_execute TO on")
        .await?;
    run_faulted(
        &mut conn,
        &mut observer,
        JOINSCAN_QUERY,
        "artificial leader-side execute failure",
    )
    .await?;
    run_faulted(
        &mut conn,
        &mut observer,
        AGGREGATESCAN_QUERY,
        "artificial leader-side execute failure",
    )
    .await?;
    conn.execute("SET paradedb.mpp_test_fail_leader_execute TO off")
        .await?;

    // With the faults cleared the same connection runs the query to completion.
    let ids: Vec<(i64,)> = timeout(
        WATCHDOG,
        sqlx::query_as(AssertSqlSafe(JOINSCAN_QUERY.to_string())).fetch_all(&mut conn),
    )
    .await
    .context("query after the aborted runs did not return")??;
    assert_eq!(
        ids.into_iter().map(|(id,)| id).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5]
    );
    Ok(())
}
