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

//! An MPP JoinScan whose broadcast build side keeps rows in a mutable segment, run while another
//! connection keeps updating that side. The node that checks visibility resolves the build side's
//! packed `DocAddress`es with a reader it opened itself (here the leader, whose reader opens at
//! plan time, well before the workers open theirs). A mutable segment is materialized per open, so
//! unless every reader replays the leader's segment view, the addresses one reader packed land in
//! another reader's shorter (or differently ordered) `DocId` space: a bitpacker overflow panic, or
//! a silently wrong ctid.

use anyhow::Result;
use rstest::*;
use sqlx::{AssertSqlSafe, Executor, PgConnection};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tests::fixtures::*;
use tokio::time::sleep;

// The build side (`categories`) has two immutable segments plus the mutable one the writer keeps
// growing, so its scan is distributed and broadcast. The probe side stays in one segment, which
// keeps the join and its visibility check on the leader: the leader opened its `categories`
// reader at plan time, and the workers open theirs only after they attach, so an update between
// the two is easy to land.
const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

CREATE TABLE mppc_categories (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE mppc_test (
    id SERIAL8 NOT NULL PRIMARY KEY,
    message TEXT,
    category_id INTEGER NOT NULL
);

CREATE INDEX mppc_test_idx ON mppc_test USING paradedb (id, message, category_id)
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
RESET paradedb.global_mutable_segment_rows;

-- Start the build side's mutable log before the readers do.
UPDATE mppc_categories SET name = 'category ' || id::text || ' v0' WHERE id <= 10;

ANALYZE mppc_test;
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

const JOIN_QUERY: &str = r#"
SELECT t.id, c.name
FROM mppc_test t
JOIN mppc_categories c ON t.category_id = c.id
WHERE t.message @@@ 'beer'
ORDER BY t.id
LIMIT 25
"#;

// Every category id in 1..=100 exists, so the join never drops a `beer` row and the answer is the
// 25 smallest matching ids whatever the writers do to the category names.
const EXPECTED_IDS_QUERY: &str = r#"
SELECT id FROM mppc_test WHERE message LIKE '%beer%' ORDER BY id LIMIT 25
"#;

const RUN_FOR: Duration = Duration::from_secs(20);
const READERS: usize = 3;

async fn assert_query_plans_as_mpp(conn: &mut PgConnection) -> Result<()> {
    let rows: Vec<(String,)> = sqlx::query_as(AssertSqlSafe(format!(
        "EXPLAIN (COSTS OFF, VERBOSE) {JOIN_QUERY}"
    )))
    .fetch_all(&mut *conn)
    .await?;
    let explain = rows
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        explain.contains("DistributedExec"),
        "the join must plan as DistributedExec:\n{explain}"
    );
    Ok(())
}

#[rstest]
#[tokio::test]
async fn mpp_joinscan_survives_mutable_build_side_churn(database: Db) -> Result<()> {
    let mut setup = database.connection().await;
    setup.execute(SETUP_SQL).await?;
    setup.execute(MPP_GUCS).await?;
    assert_query_plans_as_mpp(&mut setup).await?;

    let expected_ids: Vec<i64> = sqlx::query_scalar(EXPECTED_IDS_QUERY)
        .fetch_all(&mut setup)
        .await?;
    assert_eq!(expected_ids.len(), 25);

    let stop = Arc::new(AtomicBool::new(false));
    let queries = Arc::new(AtomicUsize::new(0));
    let updates = Arc::new(AtomicUsize::new(0));
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
            }
        })
    };

    let mut readers = Vec::new();
    for reader_id in 0..READERS {
        let mut conn = database.connection().await;
        conn.execute(MPP_GUCS).await?;
        let stop = Arc::clone(&stop);
        let queries = Arc::clone(&queries);
        let failures = Arc::clone(&failures);
        let expected_ids = expected_ids.clone();
        readers.push(tokio::spawn(async move {
            while !stop.load(Ordering::Relaxed) {
                let rows: Result<Vec<(i64, String)>, _> =
                    sqlx::query_as(JOIN_QUERY).fetch_all(&mut conn).await;
                match rows {
                    Err(e) => failures
                        .lock()
                        .unwrap()
                        .push(format!("reader {reader_id}: query failed: {e}")),
                    Ok(rows) => {
                        let ids: Vec<i64> = rows.iter().map(|(id, _)| *id).collect();
                        if ids != expected_ids {
                            failures.lock().unwrap().push(format!(
                                "reader {reader_id}: ids {ids:?} != expected {expected_ids:?}"
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
    Ok(())
}
