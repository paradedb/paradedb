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

//! MPP JoinScans whose broadcast build side keeps rows in a mutable segment, run while another
//! connection keeps updating that side. The node that checks visibility resolves the build
//! side's packed `DocAddress`es with a reader it opened itself, and a mutable segment is
//! materialized per open, so unless every reader replays the leader's segment view the
//! addresses one reader packed land in another reader's shorter (or differently ordered)
//! `DocId` space: a bitpacker overflow panic, or a silently wrong ctid.
//!
//! Two probe tables give the two consumer placements. The single-segment probe keeps the join
//! and its visibility check on the leader, whose `categories` reader opens at plan time, well
//! before the workers open theirs; that wide window is what makes this test fail fast without
//! the replay. The multi-segment probe pushes the join into a worker stage, so the resolver is
//! rebuilt in a worker, the shape from the original report.

use anyhow::Result;
use rstest::*;
use sqlx::{AssertSqlSafe, Executor, PgConnection};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tests::fixtures::*;
use tokio::time::sleep;

// The build side (`categories`) has two immutable segments plus the mutable one the writer keeps
// growing, so its scan is distributed and broadcast. `mppc_test` stays in one segment (the
// leader-hosted shape); `mppc_test_multi` spans three (the worker-hosted shape).
const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

CREATE TABLE mppc_categories (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE mppc_test (
    id SERIAL8 NOT NULL PRIMARY KEY,
    message TEXT,
    category_id INTEGER NOT NULL
);
CREATE TABLE mppc_test_multi (
    id SERIAL8 NOT NULL PRIMARY KEY,
    message TEXT,
    category_id INTEGER NOT NULL
);

CREATE INDEX mppc_test_idx ON mppc_test USING paradedb (id, message, category_id)
WITH (key_field = 'id', numeric_fields = '{"category_id": {"fast": true}}');
CREATE INDEX mppc_test_multi_idx ON mppc_test_multi USING paradedb (id, message, category_id)
WITH (key_field = 'id', numeric_fields = '{"category_id": {"fast": true}}');
CREATE INDEX mppc_categories_idx ON mppc_categories USING paradedb (id, name)
WITH (key_field = 'id', text_fields = '{"name": {"fast": true, "tokenizer": {"type": "raw"}}}',
      layer_sizes = '0', background_layer_sizes = '0');

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO mppc_categories (id, name)
SELECT i, 'category ' || i::text FROM generate_series(1, 50) AS s(i);
INSERT INTO mppc_categories (id, name)
SELECT i, 'category ' || i::text FROM generate_series(51, 100) AS s(i);
INSERT INTO mppc_test (message, category_id)
SELECT (ARRAY['beer wine cheese', 'beer wine', 'beer cheese', 'beer',
              'wine cheese', 'wine', 'cheese', 'bread butter'])[1 + (i % 8)] || ' ' || i::text,
       1 + (i % 100)
FROM generate_series(1, 5000) AS s(i);
INSERT INTO mppc_test_multi (message, category_id)
SELECT (ARRAY['beer wine cheese', 'beer wine', 'beer cheese', 'beer',
              'wine cheese', 'wine', 'cheese', 'bread butter'])[1 + (i % 8)] || ' ' || i::text,
       1 + (i % 100)
FROM generate_series(1, 5000) AS s(i);
INSERT INTO mppc_test_multi (message, category_id)
SELECT (ARRAY['beer wine cheese', 'beer wine', 'beer cheese', 'beer',
              'wine cheese', 'wine', 'cheese', 'bread butter'])[1 + (i % 8)] || ' ' || i::text,
       1 + (i % 100)
FROM generate_series(5001, 10000) AS s(i);
INSERT INTO mppc_test_multi (message, category_id)
SELECT (ARRAY['beer wine cheese', 'beer wine', 'beer cheese', 'beer',
              'wine cheese', 'wine', 'cheese', 'bread butter'])[1 + (i % 8)] || ' ' || i::text,
       1 + (i % 100)
FROM generate_series(10001, 15000) AS s(i);
RESET paradedb.global_mutable_segment_rows;

-- Start the build side's mutable log before the readers do.
UPDATE mppc_categories SET name = 'category ' || id::text || ' v0' WHERE id <= 10;

ANALYZE mppc_test;
ANALYZE mppc_test_multi;
ANALYZE mppc_categories;
"#;

// Forces the join through MPP and zeroes the parallel costs so the planner always picks it.
const MPP_GUCS: &str = r#"
SET paradedb.enable_join_custom_scan TO on;
SET max_parallel_workers_per_gather TO 2;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
"#;

const LEADER_QUERY: &str = r#"
SELECT t.id, c.name
FROM mppc_test t
JOIN mppc_categories c ON t.category_id = c.id
WHERE t.message @@@ 'beer'
ORDER BY t.id
LIMIT 25
"#;

const WORKER_QUERY: &str = r#"
SELECT t.id, c.name
FROM mppc_test_multi t
JOIN mppc_categories c ON t.category_id = c.id
WHERE t.message @@@ 'beer'
ORDER BY t.id
LIMIT 25
"#;

// Every category id in 1..=100 exists, so the join never drops a `beer` row and the answer is the
// 25 smallest matching ids whatever the writer does to the category names.
const LEADER_EXPECTED_IDS: &str =
    "SELECT id FROM mppc_test WHERE message LIKE '%beer%' ORDER BY id LIMIT 25";
const WORKER_EXPECTED_IDS: &str =
    "SELECT id FROM mppc_test_multi WHERE message LIKE '%beer%' ORDER BY id LIMIT 25";

const RUN_FOR: Duration = Duration::from_secs(20);
const READERS: usize = 3;
// Every Nth reader iteration runs the query under EXPLAIN ANALYZE instead, whose output shows
// whether the launch actually went distributed or fell back to serial. Anchored on the FIRST
// iteration: a loaded host may only get through a handful of iterations, and the final assert
// needs at least one sample.
const EXPLAIN_EVERY: usize = 25;

async fn explain(conn: &mut PgConnection, analyze: bool, query: &str) -> Result<String> {
    let options = if analyze {
        "ANALYZE, VERBOSE, COSTS OFF, TIMING OFF, SUMMARY OFF"
    } else {
        "VERBOSE, COSTS OFF"
    };
    let rows: Vec<(String,)> =
        sqlx::query_as(AssertSqlSafe(format!("EXPLAIN ({options}) {query}")))
            .fetch_all(&mut *conn)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n"))
}

#[rstest]
#[tokio::test]
async fn mpp_joinscan_survives_mutable_build_side_churn(database: Db) -> Result<()> {
    let mut setup = database.connection().await;
    setup.execute(SETUP_SQL).await?;
    setup.execute(MPP_GUCS).await?;

    let leader_plan = explain(&mut setup, false, LEADER_QUERY).await?;
    assert!(
        leader_plan.contains("DistributedExec"),
        "the leader-hosted join must plan as DistributedExec:\n{leader_plan}"
    );
    let worker_plan = explain(&mut setup, false, WORKER_QUERY).await?;
    assert!(
        worker_plan.contains("DistributedExec") && worker_plan.contains("Stage 2"),
        "the worker-hosted join must plan with the join in a worker stage:\n{worker_plan}"
    );

    let expected_leader: Vec<i64> = sqlx::query_scalar(LEADER_EXPECTED_IDS)
        .fetch_all(&mut setup)
        .await?;
    let expected_worker: Vec<i64> = sqlx::query_scalar(WORKER_EXPECTED_IDS)
        .fetch_all(&mut setup)
        .await?;
    assert_eq!(expected_leader.len(), 25);
    assert_eq!(expected_worker.len(), 25);

    let stop = Arc::new(AtomicBool::new(false));
    let queries = Arc::new(AtomicUsize::new(0));
    let updates = Arc::new(AtomicUsize::new(0));
    let distributed = Arc::new(AtomicUsize::new(0));
    let failures = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));

    // One writer keeps the build side's mutable segment growing: every UPDATE appends to its
    // add/remove log, which is what moves the segment's `DocId` space between two opens.
    let writer = {
        let mut conn = database.connection().await;
        let stop = Arc::clone(&stop);
        let updates = Arc::clone(&updates);
        tokio::spawn(async move {
            let mut n = 0u64;
            while !stop.load(Ordering::Relaxed) {
                n += 1;
                let sql = format!(
                    "UPDATE mppc_categories SET name = 'category ' || id::text || ' v{n}' \
                     WHERE id <= 10"
                );
                if let Err(e) = conn.execute(AssertSqlSafe(sql)).await {
                    panic!("writer failed: {e}");
                }
                updates.fetch_add(1, Ordering::Relaxed);
                // Yield so the writer doesn't starve the readers on a loaded host; the race
                // window only needs the log to move between two opens, not a firehose.
                sleep(Duration::from_millis(5)).await;
            }
        })
    };

    let mut readers = Vec::new();
    for reader_id in 0..READERS {
        let mut conn = database.connection().await;
        conn.execute(MPP_GUCS).await?;
        let stop = Arc::clone(&stop);
        let queries = Arc::clone(&queries);
        let distributed = Arc::clone(&distributed);
        let failures = Arc::clone(&failures);
        let expected_leader = expected_leader.clone();
        let expected_worker = expected_worker.clone();
        readers.push(tokio::spawn(async move {
            let mut i = 0usize;
            while !stop.load(Ordering::Relaxed) {
                i += 1;
                let (query, expected) = if i.is_multiple_of(2) {
                    (LEADER_QUERY, &expected_leader)
                } else {
                    (WORKER_QUERY, &expected_worker)
                };
                if i % EXPLAIN_EVERY == 1 {
                    // EXPLAIN ANALYZE executes the query too; a serial fallback shows a plan
                    // without DistributedExec. Count rather than fail: a starved host may
                    // legitimately fall back sometimes, but never going distributed at all
                    // means this test exercised nothing.
                    match explain(&mut conn, true, query).await {
                        Ok(plan) if plan.contains("DistributedExec") => {
                            distributed.fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(_) => {}
                        Err(e) => failures
                            .lock()
                            .unwrap()
                            .push(format!("reader {reader_id}: explain analyze failed: {e}")),
                    }
                    queries.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                let rows: Result<Vec<(i64, String)>, _> =
                    sqlx::query_as(query).fetch_all(&mut conn).await;
                match rows {
                    Err(e) => failures
                        .lock()
                        .unwrap()
                        .push(format!("reader {reader_id}: query failed: {e}")),
                    Ok(rows) => {
                        let ids: Vec<i64> = rows.iter().map(|(id, _)| *id).collect();
                        if &ids != expected {
                            failures.lock().unwrap().push(format!(
                                "reader {reader_id}: ids {ids:?} != expected {expected:?}"
                            ));
                        }
                        if let Some((id, name)) =
                            rows.iter().find(|(_, name)| !name.starts_with("category "))
                        {
                            failures.lock().unwrap().push(format!(
                                "reader {reader_id}: row {id} joined a bad category name {name:?}"
                            ));
                        }
                    }
                }
                queries.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    let start = Instant::now();
    while start.elapsed() < RUN_FOR {
        sleep(Duration::from_millis(200)).await;
        if !failures.lock().unwrap().is_empty() {
            break;
        }
    }
    stop.store(true, Ordering::Relaxed);
    for reader in readers {
        reader.await?;
    }
    writer.await?;

    let failures = failures.lock().unwrap().clone();
    assert!(
        failures.is_empty(),
        "{} of {} MPP join queries failed under {} concurrent updates; first: {}",
        failures.len(),
        queries.load(Ordering::Relaxed),
        updates.load(Ordering::Relaxed),
        failures[0]
    );
    assert!(
        queries.load(Ordering::Relaxed) > 0 && updates.load(Ordering::Relaxed) > 0,
        "the readers and the writer must both have run"
    );
    assert!(
        distributed.load(Ordering::Relaxed) > 0,
        "no query ran distributed; the test exercised nothing (queries={}, updates={})",
        queries.load(Ordering::Relaxed),
        updates.load(Ordering::Relaxed)
    );
    Ok(())
}
