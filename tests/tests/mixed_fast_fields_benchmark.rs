// Copyright (c) 2023-2025 ParadeDB, Inc.
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

mod fixtures;

use anyhow::Result;
use fixtures::*;
use paradedb::micro_benchmarks::{
    benchmark_mixed_fast_fields, detect_exec_method, set_execution_method, setup_benchmark_database,
};
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

// Number of benchmark iterations for each query
const ITERATIONS: usize = 5;
// Number of warmup iterations before measuring performance
const WARMUP_ITERATIONS: usize = 2;
// Number of rows to use in the benchmark
const NUM_ROWS_BENCHMARK: usize = 10000;
const NUM_ROWS_VALIDATION: usize = 1000; // Reduced for faster test runs
const BATCH_SIZE: usize = 10000; // For efficiency with large datasets, use batch inserts

#[rstest]
async fn benchmark_mixed_fast_fields_test(mut conn: PgConnection) -> Result<()> {
    benchmark_mixed_fast_fields(
        &mut conn,
        false,
        ITERATIONS,
        WARMUP_ITERATIONS,
        NUM_ROWS_BENCHMARK,
        BATCH_SIZE,
    )
    .await?;
    Ok(())
}

/// Validate that the different execution methods return the same results
/// and enforce that we're actually using the intended execution methods
#[rstest]
async fn validate_mixed_fast_fields_correctness(mut conn: PgConnection) -> Result<()> {
    // Set up the benchmark database
    setup_benchmark_database(
        &mut conn,
        NUM_ROWS_VALIDATION,
        "test_benchmark_data",
        BATCH_SIZE,
    )
    .await?;

    println!("Testing query correctness between execution methods...");
    println!("────────────────────────────────────────────────────────");

    // Define a test query that will use both string and numeric fast fields
    let test_query =
        "SELECT id, string_field1, string_field2, numeric_field1, numeric_field2, numeric_field3 
         FROM test_benchmark_data 
         WHERE numeric_field1 < 500 AND string_field1 @@@ '\"alpha_complex_identifier_123456789\"' AND string_field2 @@@ '\"red_velvet_cupcake_with_cream_cheese_frosting\"'
         ORDER BY id";

    println!("Testing query correctness between execution methods...");
    println!("────────────────────────────────────────────────────────");

    // Set PostgreSQL settings to ensure index usage
    sqlx::query("SET enable_seqscan = off")
        .execute(&mut conn)
        .await?;
    sqlx::query("SET enable_bitmapscan = off")
        .execute(&mut conn)
        .await?;
    sqlx::query("SET enable_indexscan = off")
        .execute(&mut conn)
        .await?;

    set_execution_method(&mut conn, "MixedFastFieldExec", "test_benchmark_data").await?;

    // Get results with MixedFastFieldExec
    let mixed_results = sqlx::query(test_query).fetch_all(&mut conn).await?;

    // Get execution plan to verify method
    let (mixed_plan,): (Value,) = sqlx::query_as(&format!("EXPLAIN (FORMAT JSON) {}", test_query))
        .fetch_one(&mut conn)
        .await?;

    let mixed_method = detect_exec_method(&mixed_plan);
    println!("✓ Mixed index using → {}", mixed_method);

    // ENFORCE: Validate we're actually using the MixedFastFieldExec method
    assert!(
        mixed_method.contains("MixedFastFieldExec"),
        "Expected MixedFastFieldExec execution method, but got: {}. Check index configuration and query settings.",
        mixed_method
    );

    set_execution_method(&mut conn, "NormalScanExecState", "test_benchmark_data").await?;

    // Get results with NormalScanExecState
    let normal_results = sqlx::query(test_query).fetch_all(&mut conn).await?;

    // Get execution plan to verify method
    let (normal_plan,): (Value,) = sqlx::query_as(&format!("EXPLAIN (FORMAT JSON) {}", test_query))
        .fetch_one(&mut conn)
        .await?;

    let normal_method = detect_exec_method(&normal_plan);
    println!("✓ Normal index using → {}", normal_method);

    // ENFORCE: Validate we're actually using the NormalScanExecState method
    assert!(
        normal_method.contains("NormalScanExecState"),
        "Expected NormalScanExecState execution method, but got: {}. Check index configuration and query settings.",
        normal_method
    );

    // Compare result counts
    println!(
        "Comparing {} rows from each execution method...",
        mixed_results.len()
    );
    assert_eq!(
        mixed_results.len(),
        normal_results.len(),
        "Mixed and Normal execution methods returned different number of rows"
    );

    // Verify that we have the same rows (by comparing the string representation of each row)
    for (i, (mixed_row, normal_row)) in mixed_results.iter().zip(normal_results.iter()).enumerate()
    {
        assert_eq!(
            format!("{:?}", mixed_row),
            format!("{:?}", normal_row),
            "Row {} differs between Mixed and Normal execution methods",
            i
        );
    }

    println!("✅ Validation passed: Both execution methods returned identical results");
    println!("────────────────────────────────────────────────────────");

    Ok(())
}
