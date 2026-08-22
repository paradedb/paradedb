// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

//! Proves that the MPP leader's source manifests keep ParadeDB's cleanup-page pin alive while
//! workers consume the corresponding segment snapshot. A rebuilt FFHelper above a network
//! boundary may outlive its temporary SearchIndexReader, but VACUUM must still wait for the
//! query-level manifest before it can finish `ambulkdelete` and update the visibility map.

use anyhow::{Context, Result, bail};
use rstest::*;
use sqlx::{AssertSqlSafe, Executor, PgConnection};
use std::time::{Duration, Instant};
use tests::fixtures::*;
use tokio::time::{sleep, timeout};

const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

CREATE TABLE mpp_pin_catalog (
    id bigint PRIMARY KEY,
    title text NOT NULL
);
CREATE TABLE mpp_pin_owned (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id bigint NOT NULL,
    item_id bigint NOT NULL
);

CREATE INDEX mpp_pin_catalog_idx ON mpp_pin_catalog
USING bm25 (id, title)
WITH (
    key_field = 'id',
    target_segment_count = 8,
    background_layer_sizes = '0',
    text_fields = '{"title":{"fast":true}}'
);
CREATE INDEX mpp_pin_owned_idx ON mpp_pin_owned
USING bm25 (id, user_id, item_id)
WITH (
    key_field = 'id',
    target_segment_count = 8,
    background_layer_sizes = '0',
    numeric_fields = '{"user_id":{"fast":true},"item_id":{"fast":true}}'
);

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO mpp_pin_catalog
SELECT g, CASE WHEN g % 3 = 0 THEN 'dragon ' || g ELSE 'other ' || g END
FROM generate_series(1, 2500) AS g;
INSERT INTO mpp_pin_catalog
SELECT g, CASE WHEN g % 3 = 0 THEN 'dragon ' || g ELSE 'other ' || g END
FROM generate_series(2501, 5000) AS g;
INSERT INTO mpp_pin_owned (user_id, item_id)
SELECT 42, g FROM generate_series(1, 1500) AS g;
INSERT INTO mpp_pin_owned (user_id, item_id)
SELECT 42, g FROM generate_series(1501, 3000) AS g;
RESET paradedb.global_mutable_segment_rows;

ANALYZE mpp_pin_catalog;
ANALYZE mpp_pin_owned;
"#;

const MPP_GUCS: &str = r#"
SET paradedb.enable_join_custom_scan = on;
SET max_parallel_workers_per_gather = 2;
SET max_parallel_workers = 8;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
"#;

const MPP_QUERY: &str = r#"
SELECT l.id, l.title
FROM mpp_pin_catalog AS l
WHERE l.id @@@ 'title:dragon'
  AND NOT EXISTS (
      SELECT 1
      FROM mpp_pin_owned AS o
      WHERE o.user_id = 42
        AND o.item_id = l.id
  )
ORDER BY l.id
LIMIT 25
"#;

fn held_mpp_query(hold_lock_key: i64) -> String {
    format!(
        r#"
SELECT q.id, q.title, pg_advisory_xact_lock({hold_lock_key})::text
FROM ({MPP_QUERY}) AS q
"#
    )
}

async fn explain(conn: &mut PgConnection, prefix: &str, query: &str) -> Result<String> {
    let rows: Vec<(String,)> = sqlx::query_as(AssertSqlSafe(format!("{prefix} {query}")))
        .fetch_all(conn)
        .await?;
    Ok(rows
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n"))
}

async fn wait_for_event(
    conn: &mut PgConnection,
    pid: i32,
    expected_type: &str,
    expected_event: Option<&str>,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let state: Option<(String, String, String)> = sqlx::query_as(
            "SELECT state, coalesce(wait_event_type, ''), coalesce(wait_event, '') \
             FROM pg_stat_activity WHERE pid = $1",
        )
        .bind(pid)
        .fetch_optional(&mut *conn)
        .await?;

        match state {
            Some((_, event_type, event))
                if event_type == expected_type
                    && expected_event.is_none_or(|expected| event == expected) =>
            {
                return Ok(());
            }
            Some(_) if Instant::now() < deadline => sleep(Duration::from_millis(10)).await,
            Some((state, event_type, event)) => bail!(
                "backend {pid} did not reach wait event {expected_type}/{expected_event:?}: \
                 state={state}, wait_event_type={event_type}, wait_event={event}"
            ),
            None => bail!("backend {pid} finished before reaching the expected wait event"),
        }
    }
}

#[rstest]
#[tokio::test]
async fn mpp_manifest_holds_cleanup_pin_until_scan_ends(database: Db) -> Result<()> {
    let mut setup = database.connection().await;
    setup.execute(SETUP_SQL).await?;
    setup.execute(MPP_GUCS).await?;

    // Leave enough committed dead ctids to force PostgreSQL to perform index cleanup (a single
    // dead tuple may legitimately take VACUUM's small-table bypass). The plan-execution probe then
    // advances the global xmin beyond this delete, so the concurrent VACUUM below can classify the
    // tuples as DEAD and must enter ambulkdelete's cleanup-page barrier.
    setup
        .execute("DELETE FROM mpp_pin_catalog WHERE id <= 500 OR id = 3003")
        .await?;

    // Prove both the distributed shape and actual worker launch. VisibilityFilterExec is above
    // the shuffled anti-join, so its ctid FFHelper is rebuilt in that worker fragment.
    let analyzed = explain(
        &mut setup,
        "EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF)",
        MPP_QUERY,
    )
    .await?;
    assert!(analyzed.contains("DistributedExec"), "{analyzed}");
    assert!(analyzed.contains("NetworkShuffleExec"), "{analyzed}");
    assert!(analyzed.contains("VisibilityFilterExec"), "{analyzed}");
    assert!(analyzed.contains("MPP Launch: workers=2"), "{analyzed}");

    // The advisory lock is evaluated by PostgreSQL above the distributed custom scan. Holding it
    // lets the test pause deterministically after MPP has produced its first row but before
    // ExecutorEnd can drop the leader's source manifests.
    let mut scan = database.connection().await;
    scan.execute(MPP_GUCS).await?;
    let scan_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut scan)
        .await?;
    // Backend PIDs are unique within the test cluster, avoiding advisory-lock collisions when
    // unrelated integration tests run concurrently in other databases.
    let hold_lock_key = i64::from(scan_pid);
    let held_query = held_mpp_query(hold_lock_key);
    let held_plan = explain(&mut scan, "EXPLAIN (VERBOSE, COSTS OFF)", &held_query).await?;
    assert!(held_plan.contains("Subquery Scan on q"), "{held_plan}");
    assert!(held_plan.contains("DistributedExec"), "{held_plan}");
    assert!(held_plan.contains("VisibilityFilterExec"), "{held_plan}");
    assert!(held_plan.contains("pg_advisory_xact_lock"), "{held_plan}");

    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(hold_lock_key)
        .execute(&mut setup)
        .await?;

    let scan_task = tokio::spawn(async move {
        sqlx::query_as::<_, (i64, String, String)>(AssertSqlSafe(held_query))
            .fetch_one(&mut scan)
            .await
    });

    let mut observer = database.connection().await;
    wait_for_event(&mut observer, scan_pid, "Lock", Some("advisory")).await?;
    assert!(
        !scan_task.is_finished(),
        "MPP query finished instead of blocking above its custom scan"
    );

    let mut vacuum = database.connection().await;
    let vacuum_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut vacuum)
        .await?;
    let vacuum_task =
        tokio::spawn(async move { vacuum.execute("VACUUM mpp_pin_catalog").await.map(|_| ()) });

    if let Err(wait_error) = wait_for_event(&mut observer, vacuum_pid, "BufferPin", None).await {
        let vacuum_result = timeout(Duration::from_secs(1), vacuum_task)
            .await
            .context("VACUUM backend disappeared but its client task did not finish")?;
        bail!("{wait_error:#}; VACUUM task result: {vacuum_result:?}");
    }
    assert!(
        !vacuum_task.is_finished(),
        "VACUUM crossed the cleanup-page barrier while the MPP scan was alive"
    );

    // Let the outer projection finish. ExecutorEnd then drops the source manifests, so VACUUM can
    // acquire the cleanup lock and complete.
    let unlocked: bool = sqlx::query_scalar("SELECT pg_advisory_unlock($1)")
        .bind(hold_lock_key)
        .fetch_one(&mut setup)
        .await?;
    assert!(unlocked, "test session did not own the advisory lock");

    let first = timeout(Duration::from_secs(20), scan_task)
        .await
        .context("MPP query remained blocked after releasing its advisory lock")??;
    let first = first?;
    assert_eq!(first.0, 3006);
    assert_eq!(first.1, "dragon 3006");

    let vacuum_result = timeout(Duration::from_secs(20), vacuum_task)
        .await
        .context("VACUUM remained blocked after the MPP scan ended")??;
    vacuum_result?;

    Ok(())
}
