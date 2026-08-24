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
use sqlx::Executor;
use tests::fixtures::*;

const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

CREATE TABLE mem_a (id bigint PRIMARY KEY, state text);
CREATE TABLE mem_b (id bigint PRIMARY KEY, aid bigint, u text);
CREATE INDEX mem_a_idx ON mem_a USING bm25 (id, state)
  WITH (key_field = 'id', target_segment_count = '3', mutable_segment_rows = '0');
CREATE INDEX mem_b_idx ON mem_b USING bm25 (id, aid, u)
  WITH (key_field = 'id', target_segment_count = '3');

INSERT INTO mem_a SELECT g, 'active' FROM generate_series(1, 50000) g;
INSERT INTO mem_a SELECT g, 'merged' FROM generate_series(50001, 100000) g;
INSERT INTO mem_b SELECT g, g, 'u1'  FROM generate_series(1, 100000) g;
"#;

const MPP_GUCS: &str = r#"
SET paradedb.enable_join_custom_scan TO on;
-- The fixture tables are tiny; disable the size gate so MPP engages.
SET paradedb.mpp_min_rows TO 0;
SET max_parallel_workers_per_gather TO 2;
SET max_parallel_workers TO 2;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET paradedb.mpp_test_panic_in_worker TO on;
"#;

const MPP_QUERY: &str = r#"
SELECT count(*)
FROM mem_b le
JOIN mem_a sv ON sv.id = le.aid
WHERE le.id @@@ paradedb.term('u', 'u1')
  AND sv.id @@@ paradedb.term('state', 'merged')
"#;

#[rstest]
#[tokio::test]
async fn mpp_error_masking(database: Db) -> Result<()> {
    let mut setup = database.connection().await;
    setup.execute(SETUP_SQL).await?;
    let guc_exists: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pg_settings WHERE name = 'paradedb.mpp_test_panic_in_worker'",
    )
    .fetch_optional(&mut setup)
    .await?;

    if guc_exists.is_none() {
        println!(
            "Skipping mpp_error_masking: paradedb.mpp_test_panic_in_worker GUC is not present (non-debug build)"
        );
        return Ok(());
    }

    setup.execute(MPP_GUCS).await?;

    let iterations = 10;
    let mut real_error_count = 0;
    let mut masked_error_count = 0;

    let explain_rows: Vec<(String,)> = sqlx::query_as(&format!("EXPLAIN {}", MPP_QUERY))
        .fetch_all(&mut setup)
        .await
        .unwrap();
    let explain_str = explain_rows
        .into_iter()
        .map(|(r,)| r)
        .collect::<Vec<_>>()
        .join("\n");
    println!("EXPLAIN output:\n{explain_str}");

    assert!(
        explain_str.contains("DistributedExec") && explain_str.contains("PgSearchScan"),
        "Expected EXPLAIN plan to use MPP (DistributedExec & PgSearchScan), got:\n{explain_str}"
    );

    for i in 0..iterations {
        let res = setup.execute(MPP_QUERY).await;
        if let Err(e) = res {
            let err_str = e.to_string().to_lowercase();
            println!("Iteration {}: got error: {}", i, err_str);
            if err_str.contains("artificial panic") {
                real_error_count += 1;
            } else if err_str.contains("transport receiver detached") {
                masked_error_count += 1;
            } else {
                panic!("Unexpected error: {e}");
            }
        } else {
            panic!("Expected query to fail due to panic, but it succeeded");
        }
    }

    assert_eq!(
        masked_error_count, 0,
        "Failed: observed {masked_error_count} masked errors ('transport receiver detached') and {real_error_count} real errors."
    );
    assert_eq!(
        real_error_count, iterations,
        "Expected all {iterations} iterations to return the real error."
    );

    Ok(())
}
