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

//! Concurrent update reproducer for AggregateScan COUNT(*) / issue #5548.
//!
//! Protocol (UPDATEs only in the freeze→recheck gap):
//! 1. Reader holds `pg_advisory_lock(5548)` before COUNT
//! 2. Writer (OS thread) blocks on the same lock
//! 3. COUNT with `paradedb.aggregate_mvcc_race_delay_ms > 0` materializes segments,
//!    unlocks 5548, sleeps, pushes a fresh READ COMMITTED snapshot
//! 4. Writer's autocommit UPDATEs of drink rows land during the sleep
//! 5. A late GetActiveSnapshot() would undercount; the pinned-snapshot fix must not
//!
//! The writer uses `std::thread` (not `tokio::spawn`) because a blocking
//! `pg_advisory_lock` on the current-thread tokio runtime would deadlock: COUNT
//! never runs, so the unlock never happens.

use anyhow::Result;
use rstest::*;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tests::fixtures::*;
use tokio::time::{sleep, Duration};

/// Must match `AGGREGATE_MVCC_RACE_GATE_KEY` in `pg_search/src/aggregate/mod.rs`.
const RACE_GATE_KEY: i64 = 5548;

fn assert_uses_aggregate_scan(conn: &mut sqlx::PgConnection, query: &str) {
    let explain_query = format!("EXPLAIN (FORMAT JSON) {}", query);
    let (plan,): (Value,) = explain_query.fetch_one(conn);
    let plan_str = format!("{:?}", plan);
    assert!(
        plan_str.contains("ParadeDB Aggregate Scan"),
        "Query should use ParadeDB Aggregate Scan but got plan: {}",
        plan_str
    );
}

#[rstest]
#[tokio::test]
async fn aggregate_count_stable_under_concurrent_updates(database: Db) -> Result<()> {
    let mut setup_conn = database.connection().await;

    const N_ROWS: i64 = 3_000;
    let expected_drink: i64 = (N_ROWS + 2) / 3; // ids ≡ 0 (mod 3) → 1000
    const RACE_DELAY_MS: i64 = 500;
    // Drink rows that sit in the hot update band (id < 10, id % 3 == 0).
    const HOT_IDS: &str = "3, 6, 9";

    format!(
        r#"
    CREATE EXTENSION IF NOT EXISTS pg_search;
    SET paradedb.enable_aggregate_custom_scan = on;
    SET max_parallel_workers_per_gather = 0;

    DROP TABLE IF EXISTS agg_conc_test CASCADE;
    CREATE TABLE agg_conc_test (
        id BIGINT PRIMARY KEY,
        message TEXT
    );

    INSERT INTO agg_conc_test (id, message)
    SELECT
        id,
        CASE id % 3
            WHEN 0 THEN 'drink some beer'
            WHEN 1 THEN 'sip some wine'
            ELSE 'eat some cheese'
        END
    FROM generate_series(1, {N_ROWS}) AS id;

    CREATE INDEX agg_conc_idx ON agg_conc_test
        USING bm25 (id, message)
        WITH (key_field = 'id', mutable_segment_rows = 100);
    "#
    )
    .execute(&mut setup_conn);

    let count_query = "SELECT COUNT(*) FROM agg_conc_test WHERE message @@@ 'drink'";
    assert_uses_aggregate_scan(&mut setup_conn, count_query);

    let (baseline,): (i64,) = count_query.fetch_one(&mut setup_conn);
    assert_eq!(baseline, expected_drink);

    let stop_flag = Arc::new(AtomicBool::new(false));
    let update_count = Arc::new(AtomicUsize::new(0));

    // Reader must hold the gate lock *before* the writer waits on it.
    let mut reader = database.connection().await;
    format!(
        r#"
    SET paradedb.enable_aggregate_custom_scan = on;
    SET max_parallel_workers_per_gather = 0;
    SET client_min_messages TO warning;
    SET paradedb.aggregate_mvcc_race_delay_ms = {RACE_DELAY_MS};
    SELECT pg_advisory_lock({RACE_GATE_KEY});
    "#
    )
    .execute(&mut reader);

    let mut writer_conn = database.connection().await;
    let stop = stop_flag.clone();
    let updates = update_count.clone();
    let writer = std::thread::spawn(move || {
        // Block until the aggregate unlocks after materialization.
        format!("SELECT pg_advisory_lock({RACE_GATE_KEY})").execute(&mut writer_conn);
        format!("SELECT pg_advisory_unlock({RACE_GATE_KEY})").execute(&mut writer_conn);

        if stop.load(Ordering::Relaxed) {
            return;
        }

        // Autocommit UPDATEs inside the freeze→recheck window. Changing the
        // trailing character keeps the row matching 'drink' but allocates a new
        // ctid — the classic #5548 undercount shape.
        r#"
        UPDATE agg_conc_test
        SET message = substring(message FROM 1 FOR length(message)-1)
                      || chr((trunc(random() * 26) + 65)::int)
        WHERE id IN (3, 6, 9)
        "#
        .execute(&mut writer_conn);
        updates.fetch_add(3, Ordering::Relaxed);
    });

    // Give the writer thread time to block on the advisory lock.
    sleep(Duration::from_millis(100)).await;

    let (count,): (i64,) = count_query.fetch_one(&mut reader);

    stop_flag.store(true, Ordering::Relaxed);
    // COUNT may have unlocked already; ensure we never leave a stranded lock.
    "SELECT pg_advisory_unlock_all()".execute(&mut reader);

    writer
        .join()
        .expect("writer thread panicked waiting for race gate");

    let updates = update_count.load(Ordering::Relaxed);
    assert!(
        updates > 0,
        "writer must have committed UPDATEs during the race window (updates={updates})"
    );
    assert_eq!(
        count, expected_drink,
        "COUNT(*) undercounted under concurrent UPDATEs of ids [{HOT_IDS}] with \
         paradedb.aggregate_mvcc_race_delay_ms enabled (got {count}, expected \
         {expected_drink}, writer updates={updates}). This is issue #5548."
    );

    Ok(())
}
