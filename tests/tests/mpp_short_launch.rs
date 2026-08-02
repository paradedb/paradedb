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

//! PostgreSQL can attach fewer query workers than a custom scan requested when the shared worker
//! pool is occupied. MPP must rebuild routing for that attached width and execute distributed,
//! rather than fall back to `CooperativeExec`.

use anyhow::Result;
use rstest::*;
use sqlx::{Executor, PgConnection};
use std::time::{Duration, Instant};
use tests::fixtures::*;
use tokio::time::sleep;

const REQUESTED_MPP_WORKERS: i64 = 5;
const HELD_PG_WORKERS: i64 = 4;
const HOLDER_APP: &str = "mpp_short_launch_holder";

// Five separate inserts with the mutable segment disabled create five durable segments per
// index. The target JoinScan therefore has a five-task producer stage, while the GUC cap asks
// PostgreSQL for five MPP workers.
const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

CREATE TABLE mpp_sl_posts (
    id bigserial PRIMARY KEY,
    body text NOT NULL,
    age int NOT NULL
);
CREATE TABLE mpp_sl_comments (
    id bigserial PRIMARY KEY,
    post_id bigint NOT NULL,
    body text NOT NULL,
    age int NOT NULL
);

CREATE INDEX mpp_sl_posts_idx ON mpp_sl_posts USING bm25 (id, body, age)
WITH (
    key_field = 'id',
    text_fields = '{"body":{"fast":true}}',
    numeric_fields = '{"age":{"fast":true}}',
    mutable_segment_rows = 1000,
    layer_sizes = '10TB',
    background_layer_sizes = '10TB'
);
CREATE INDEX mpp_sl_comments_idx ON mpp_sl_comments USING bm25 (id, post_id, body, age)
WITH (
    key_field = 'id',
    text_fields = '{"body":{"fast":true}}',
    numeric_fields = '{"age":{"fast":true}}',
    mutable_segment_rows = 1000,
    layer_sizes = '10TB',
    background_layer_sizes = '10TB'
);

SET paradedb.global_mutable_segment_rows = 0;
"#;

const MPP_GUCS: &str = r#"
SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_aggregate_custom_scan = on;
SET max_parallel_workers = 8;
SET max_parallel_workers_per_gather = 5;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
"#;

const HOLDER_GUCS: &str = r#"
SET paradedb.enable_custom_scan = off;
SET max_parallel_workers = 8;
SET max_parallel_workers_per_gather = 4;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
"#;

// The large cross join keeps all four ordinary PostgreSQL parallel workers busy until the test
// cancels this connection. The MPP query is much smaller, so it completes while the holder is
// still active.
const HOLDER_QUERY: &str = r#"
SELECT sum(length(md5(p.body)))
FROM mpp_sl_posts AS p
CROSS JOIN generate_series(1, 20000) AS g
"#;

const TARGET_QUERY: &str = r#"
SELECT p.id
FROM mpp_sl_posts AS p
JOIN mpp_sl_comments AS c ON c.post_id = p.id
WHERE p.body @@@ 'code' AND c.body @@@ 'comment'
ORDER BY p.id
LIMIT 10
"#;

const AGGREGATE_QUERY: &str = r#"
SELECT p.age, count(*)
FROM mpp_sl_posts AS p
JOIN mpp_sl_comments AS c ON c.age = p.age
WHERE p.body @@@ 'code'
GROUP BY p.age
ORDER BY p.age
"#;

async fn setup(conn: &mut PgConnection) -> Result<()> {
    conn.execute(SETUP_SQL).await?;
    for batch in 0..5_i64 {
        let post_first = batch * 1000 + 1;
        sqlx::query(
            "INSERT INTO mpp_sl_posts (body, age) \
             SELECT CASE WHEN g % 3 = 0 THEN 'code example tutorial' ELSE 'ordinary discussion' END, g % 50 \
             FROM generate_series($1, $2) AS g",
        )
        .bind(post_first)
        .bind(post_first + 999)
        .execute(&mut *conn)
        .await?;
        let comment_first = batch * 2000 + 1;
        sqlx::query(
            "INSERT INTO mpp_sl_comments (post_id, body, age) \
             SELECT ((g - 1) % 5000) + 1, 'comment body', g % 50 \
             FROM generate_series($1, $2) AS g",
        )
        .bind(comment_first)
        .bind(comment_first + 1999)
        .execute(&mut *conn)
        .await?;
    }
    conn.execute(
        "RESET paradedb.global_mutable_segment_rows; \
         ANALYZE mpp_sl_posts; ANALYZE mpp_sl_comments;",
    )
    .await?;
    Ok(())
}

async fn run_holder(mut conn: PgConnection) -> Result<()> {
    conn.execute(format!("SET application_name = '{HOLDER_APP}';").as_str())
        .await?;
    conn.execute(HOLDER_GUCS).await?;

    // Cancellation is the expected completion path.
    let _ = conn.execute(HOLDER_QUERY).await;
    Ok(())
}

async fn wait_for_holder_workers(observer: &mut PgConnection) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let worker_count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity \
             WHERE application_name = $1 AND backend_type = 'parallel worker'",
        )
        .bind(HOLDER_APP)
        .fetch_one(&mut *observer)
        .await?;

        if worker_count == HELD_PG_WORKERS {
            return Ok(());
        }
        if Instant::now() >= deadline {
            anyhow::bail!(
                "holder did not reserve {HELD_PG_WORKERS} PostgreSQL workers; observed {worker_count}"
            );
        }
        sleep(Duration::from_millis(10)).await;
    }
}

async fn cancel_holder(observer: &mut PgConnection) -> Result<()> {
    let pid: Option<i32> = sqlx::query_scalar(
        "SELECT pid FROM pg_stat_activity \
         WHERE application_name = $1 AND backend_type = 'client backend'",
    )
    .bind(HOLDER_APP)
    .fetch_optional(&mut *observer)
    .await?;

    if let Some(pid) = pid {
        sqlx::query("SELECT pg_cancel_backend($1)")
            .bind(pid)
            .execute(&mut *observer)
            .await?;
    }
    Ok(())
}

fn launched_workers(explain: &str) -> Option<i64> {
    let prefix = "MPP Launch: workers=";
    let suffix = explain.split(prefix).nth(1)?;
    suffix
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()
}

async fn assert_short_distributed_launch(conn: &mut PgConnection, query: &str) -> Result<String> {
    let explain = sqlx::query_as::<_, (String,)>(&format!(
        "EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF) {query}"
    ))
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|(line,)| line)
    .collect::<Vec<_>>()
    .join("\n");
    assert!(
        explain.contains("DistributedExec") && !explain.contains("CooperativeExec"),
        "short launch must remain distributed:\n{explain}"
    );
    let launched = launched_workers(&explain)
        .expect("short distributed launch must report its actual worker count");
    assert!(
        (2..REQUESTED_MPP_WORKERS).contains(&launched),
        "expected a distributed short launch below {REQUESTED_MPP_WORKERS} workers, got {launched}:\n{explain}"
    );
    Ok(explain)
}

fn assert_aggregate_scan(explain: &str) {
    assert!(
        explain.contains("Custom Scan (ParadeDB Aggregate Scan)")
            && explain.contains("Backend: DataFusion"),
        "aggregate query must use AggregateScan:\n{explain}"
    );
}

#[rstest]
#[tokio::test]
async fn mpp_short_launch_remains_distributed(database: Db) -> Result<()> {
    let mut setup = database.connection().await;
    self::setup(&mut setup).await?;

    let holder = database.connection().await;
    let holder_handle = tokio::spawn(run_holder(holder));

    let mut observer = database.connection().await;
    let test_result = async {
        wait_for_holder_workers(&mut observer).await?;

        let mut mpp = database.connection().await;
        mpp.execute(MPP_GUCS).await?;
        let explain = assert_short_distributed_launch(&mut mpp, TARGET_QUERY).await?;

        let mpp_rows: Vec<(i64,)> = sqlx::query_as(TARGET_QUERY).fetch_all(&mut mpp).await?;

        let mut serial = database.connection().await;
        serial
            .execute("SET max_parallel_workers_per_gather = 0;")
            .await?;
        let serial_rows: Vec<(i64,)> = sqlx::query_as(TARGET_QUERY).fetch_all(&mut serial).await?;
        assert_eq!(
            mpp_rows, serial_rows,
            "short MPP launch changed query results; EXPLAIN ANALYZE:\n{explain}"
        );

        let aggregate_explain = assert_short_distributed_launch(&mut mpp, AGGREGATE_QUERY).await?;
        assert_aggregate_scan(&aggregate_explain);
        let aggregate_mpp_rows: Vec<(i32, i64)> =
            sqlx::query_as(AGGREGATE_QUERY).fetch_all(&mut mpp).await?;
        let aggregate_serial_rows: Vec<(i32, i64)> = sqlx::query_as(AGGREGATE_QUERY)
            .fetch_all(&mut serial)
            .await?;
        assert_eq!(
            aggregate_mpp_rows, aggregate_serial_rows,
            "short MPP launch changed aggregate results; EXPLAIN ANALYZE:\n{aggregate_explain}"
        );

        Ok::<_, anyhow::Error>(())
    }
    .await;

    cancel_holder(&mut observer).await?;
    holder_handle.await??;
    test_result
}
