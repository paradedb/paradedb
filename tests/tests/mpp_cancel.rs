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

//! A cancel or `pg_terminate_backend` taken while an MPP query runs used to crash the leader during
//! teardown (a die serviced from inside the tokio `block_on`, or DSM control senders dropped after
//! the parallel segment was already detached), which resets the whole cluster. These tests signal a
//! running MPP query from a second connection and assert a witness connection kept open the whole
//! time survives — a cluster reset would have dropped it.

use anyhow::Result;
use rstest::*;
use sqlx::{Executor, PgConnection};
use std::time::{Duration, Instant};
use tests::fixtures::*;
use tokio::time::sleep;

// Three 20k-row tables. `age` is in [0,50), so `users.age = products.age` fans out to millions of
// intermediate rows. That scan-dominated join keeps the leader inside its MPP `block_on` long
// enough for the killer to land a signal mid-flight. A small join can finish before the signal and
// miss the teardown bug entirely.
const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE mpp_users    (id bigserial primary key, uuid uuid, name text, age int, category text);
CREATE TABLE mpp_products (id bigserial primary key, uuid uuid, name text, age int);
CREATE TABLE mpp_orders   (id bigserial primary key, uuid uuid, name text, age int);

CREATE INDEX mpp_users_idx ON mpp_users USING bm25 (id, uuid, name, age, category)
WITH (key_field='id', text_fields='{"uuid":{"tokenizer":{"type":"keyword"},"fast":true},"name":{"tokenizer":{"type":"keyword"},"fast":true},"category":{"tokenizer":{"type":"keyword"},"fast":true}}', numeric_fields='{"age":{"fast":true}}');

CREATE INDEX mpp_products_idx ON mpp_products USING bm25 (id, uuid, name, age)
WITH (key_field='id', text_fields='{"uuid":{"tokenizer":{"type":"keyword"},"fast":true},"name":{"tokenizer":{"type":"keyword"},"fast":true}}', numeric_fields='{"age":{"fast":true}}');

CREATE INDEX mpp_orders_idx ON mpp_orders USING bm25 (id, uuid, name, age)
WITH (key_field='id', text_fields='{"uuid":{"tokenizer":{"type":"keyword"},"fast":true},"name":{"tokenizer":{"type":"keyword"},"fast":true}}', numeric_fields='{"age":{"fast":true}}');

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO mpp_users (uuid, name, age, category)
SELECT gen_random_uuid(), (ARRAY['bob','alice','carol','dave'])[1 + (g % 4)], g % 50, 'c' || (g % 5)
FROM generate_series(1, 10000) AS g;

INSERT INTO mpp_users (uuid, name, age, category)
SELECT gen_random_uuid(), (ARRAY['bob','alice','carol','dave'])[1 + (g % 4)], g % 50, 'c' || (g % 5)
FROM generate_series(10001, 20000) AS g;

INSERT INTO mpp_products (uuid, name, age)
SELECT gen_random_uuid(), (ARRAY['bob','alice'])[1 + (g % 2)], g % 50
FROM generate_series(1, 10000) AS g;

INSERT INTO mpp_products (uuid, name, age)
SELECT gen_random_uuid(), (ARRAY['bob','alice'])[1 + (g % 2)], g % 50
FROM generate_series(10001, 20000) AS g;

INSERT INTO mpp_orders (uuid, name, age)
SELECT gen_random_uuid(), 'x', g % 50
FROM generate_series(1, 10000) AS g;

INSERT INTO mpp_orders (uuid, name, age)
SELECT gen_random_uuid(), 'x', g % 50
FROM generate_series(10001, 20000) AS g;

RESET paradedb.global_mutable_segment_rows;

ANALYZE mpp_users;
ANALYZE mpp_products;
ANALYZE mpp_orders;
"#;

// Forces the join through MPP and zeroes the parallel costs so the planner always picks it.
const MPP_GUCS: &str = r#"
SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_mpp TO on;
SET paradedb.mpp_worker_count TO 3;
SET max_parallel_workers_per_gather TO 4;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET work_mem TO '512MB';
"#;

// The top-level ORDER BY ... LIMIT is what makes the join JoinScan-eligible; together with the
// `age` fan-out above it plans as `Custom Scan (ParadeDB Join Scan)` -> `DistributedExec` and runs
// long enough to be signalled mid-flight. (The `orders` uuids don't match `products`, so the result
// is empty — irrelevant; the point is the in-flight join work, not the rows.)
const MPP_QUERY: &str = r#"
SELECT mpp_users.id, mpp_users.name, mpp_users.age, mpp_products.age
FROM mpp_users JOIN mpp_products ON mpp_users.age = mpp_products.age
JOIN mpp_orders ON mpp_products.uuid = mpp_orders.uuid
WHERE NOT ((mpp_users.name @@@ 'bob') AND (mpp_users.name @@@ 'bob'))
  AND mpp_users.age >= mpp_products.age
ORDER BY mpp_users.id LIMIT 31
"#;

// The crash needs the leader caught mid-`block_on` with its DSM senders live, not during planning
// or worker launch. After the backend first shows up `active`, wait briefly so execution gets well
// inside the join before signalling. A few attempts make the test robust against host timing; on
// the fixed build every attempt is a clean teardown.
const SIGNAL_DELAY: Duration = Duration::from_millis(800);
const ATTEMPTS: usize = 3;
const VICTIM_LOOP_TIMEOUT: Duration = Duration::from_secs(30);

async fn assert_query_plans_as_mpp(conn: &mut PgConnection) -> Result<()> {
    conn.execute(MPP_GUCS).await?;
    let rows: Vec<(String,)> = sqlx::query_as(&format!("EXPLAIN (COSTS OFF, VERBOSE) {MPP_QUERY}"))
        .fetch_all(&mut *conn)
        .await?;
    let explain = rows
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        explain.contains("DistributedExec"),
        "MPP cancel test query must plan as DistributedExec:\n{explain}"
    );
    Ok(())
}

/// Spin until `victim_app`'s backend is running the MPP query, then run `signal_fn(pid)`
/// (`pg_cancel_backend` or `pg_terminate_backend`) on it. Returns the pid that was signalled.
async fn signal_running_mpp_backend(
    killer: &mut PgConnection,
    victim_app: &str,
    signal_fn: &str,
) -> Result<i32> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        // Parallel workers inherit the leader's `application_name`, so pin to the client backend
        // to signal the leader (the connection running the top-level query), not a worker.
        let pid: Option<i32> = sqlx::query_scalar(
            "SELECT pid FROM pg_stat_activity \
             WHERE application_name = $1 AND backend_type = 'client backend' \
             AND state = 'active' AND query LIKE '%mpp_users.age = mpp_products.age%'",
        )
        .bind(victim_app)
        .fetch_optional(&mut *killer)
        .await?;

        if let Some(pid) = pid {
            // Let execution get well inside the MPP join (senders live) before signalling.
            sleep(SIGNAL_DELAY).await;
            sqlx::query(&format!("SELECT {signal_fn}($1)"))
                .bind(pid)
                .execute(&mut *killer)
                .await?;
            return Ok(pid);
        }
        if Instant::now() >= deadline {
            anyhow::bail!("victim backend never showed up active running the MPP query");
        }
        sleep(Duration::from_millis(5)).await;
    }
}

/// Run the MPP query in a loop on `victim` until its connection or query is cut off. The loop keeps
/// the backend busy so the killer has a wide window to land the signal mid-query.
async fn run_until_signalled(mut victim: PgConnection, app_name: String) -> Result<()> {
    if victim
        .execute(format!("SET application_name = '{app_name}';").as_str())
        .await
        .is_err()
    {
        return Ok(());
    }
    if victim.execute(MPP_GUCS).await.is_err() {
        return Ok(());
    }
    // A terminate drops the connection and a cancel errors the in-flight query; if the signal
    // misses, the deadline turns the would-be hang into a test failure.
    let deadline = Instant::now() + VICTIM_LOOP_TIMEOUT;
    while victim.execute(MPP_QUERY).await.is_ok() {
        if Instant::now() >= deadline {
            anyhow::bail!(
                "victim query loop was not interrupted within {:?}; the signal likely missed",
                VICTIM_LOOP_TIMEOUT
            );
        }
    }
    Ok(())
}

async fn assert_cluster_alive(witness: &mut PgConnection) -> Result<()> {
    // A backend crash during teardown makes the postmaster reset the cluster, which would have
    // killed this long-lived connection. If it still answers, no reset happened.
    let one: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&mut *witness)
        .await?;
    assert_eq!(one, 1, "witness connection lost: the backend crashed");
    Ok(())
}

#[rstest]
#[tokio::test]
async fn mpp_signal_does_not_crash_backend(database: Db) -> Result<()> {
    let mut setup = database.connection().await;
    setup.execute(SETUP_SQL).await?;

    // Opened before any signal and reused after each, so a cluster reset would surface as a failure
    // on this connection.
    let mut witness = database.connection().await;
    witness
        .execute("CREATE EXTENSION IF NOT EXISTS pg_search;")
        .await?;
    assert_cluster_alive(&mut witness).await?;
    assert_query_plans_as_mpp(&mut setup).await?;

    for (app, signal_fn) in [
        ("mpp_sig_cancel", "pg_cancel_backend"),
        ("mpp_sig_terminate", "pg_terminate_backend"),
    ] {
        for attempt in 0..ATTEMPTS {
            let app_name = format!("{app}_{attempt}");
            let victim = database.connection().await;
            let loop_handle = tokio::spawn(run_until_signalled(victim, app_name.clone()));

            let mut killer = database.connection().await;
            signal_running_mpp_backend(&mut killer, &app_name, signal_fn).await?;

            loop_handle.await??;
            assert_cluster_alive(&mut witness).await?;
        }
    }

    Ok(())
}
