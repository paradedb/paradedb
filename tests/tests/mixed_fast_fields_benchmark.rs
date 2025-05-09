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
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;
use std::time::Instant;

// Number of benchmark iterations for each query
const ITERATIONS: usize = 5;
// Number of warmup iterations before measuring performance
const WARMUP_ITERATIONS: usize = 2;
// Number of rows to use in the benchmark
const NUM_ROWS: usize = 10000; // Reduced for faster test runs

/// Structure to store benchmark results
#[derive(Debug, Clone)]
struct BenchmarkResult {
    test_name: String,
    exec_method: String,
    avg_time_ms: f64,
    min_time_ms: f64,
    max_time_ms: f64,
}

/// Detects which execution method was used based on the JSON execution plan
fn detect_exec_method(plan: &Value) -> String {
    // Check if this is using the CustomScan with ParadeDB
    let uses_custom_scan = plan.to_string().contains("ParadeDB Scan");

    // If the custom scan method is explicitly mentioned, extract it
    if plan.to_string().contains("Exec Method") {
        let plan_str = plan.to_string();

        if plan_str.contains("MixedFastFieldExecState") {
            return "MixedFastFieldExec".to_string();
        } else if plan_str.contains("StringMixedFastFieldExecState") {
            return "StringMixedFastFieldExec".to_string();
        } else if plan_str.contains("NumericMixedFastFieldExecState") {
            return "NumericMixedFastFieldExec".to_string();
        } else if uses_custom_scan {
            // Custom scan but not a fast field method
            let method_start = plan_str.find("Exec Method").unwrap_or(0);
            let method_end = plan_str[method_start..].find(",").unwrap_or(plan_str.len());
            return format!(
                "CustomScan: {}",
                &plan_str[method_start..method_start + method_end]
            );
        }
    }

    // Default when no specific method is found
    "NormalScanExecState".to_string()
}

/// Setup function to create test table and data
async fn setup_benchmark_database(conn: &mut PgConnection) -> Result<()> {
    // Execute each command separately to avoid the "multiple commands in prepared statement" error

    // Drop table if exists
    sqlx::query("DROP TABLE IF EXISTS benchmark_data CASCADE")
        .execute(&mut *conn)
        .await?;

    // Create the table
    sqlx::query(
        "CREATE TABLE benchmark_data (
            id SERIAL PRIMARY KEY,
            string_field1 TEXT NOT NULL,
            string_field2 TEXT NOT NULL,
            numeric_field1 INTEGER NOT NULL,
            numeric_field2 FLOAT NOT NULL,
            numeric_field3 NUMERIC(10,2) NOT NULL
        )",
    )
    .execute(&mut *conn)
    .await?;

    // Create arrays for test data
    let string_array1 = vec![
        "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa",
        "lambda", "mu", "nu", "xi", "omicron", "pi", "rho", "sigma", "tau", "upsilon", "phi",
        "chi", "psi", "omega",
    ];

    let string_array2 = vec![
        "red", "orange", "yellow", "green", "blue", "indigo", "violet", "black", "white", "gray",
    ];

    // Insert test data in batches to avoid very long SQL statements
    for i in 0..NUM_ROWS {
        let string1 = string_array1[i % string_array1.len()];
        let string2 = string_array2[i % string_array2.len()];
        let num1 = (i % 1000) as i32;
        let num2 = (i % 100) as f32;
        let num3 = (i % 10000) as i32;

        sqlx::query(
            "INSERT INTO benchmark_data (
                string_field1, 
                string_field2, 
                numeric_field1, 
                numeric_field2, 
                numeric_field3
            ) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(string1)
        .bind(string2)
        .bind(num1)
        .bind(num2)
        .bind(num3)
        .execute(&mut *conn)
        .await?;
    }

    println!("Database setup complete with {} rows", NUM_ROWS);
    Ok(())
}

/// Creates an index with the specified configuration
async fn create_index_for_execution_method(
    conn: &mut PgConnection,
    exec_method: &str,
) -> Result<()> {
    // First drop any existing index
    sqlx::query("DROP INDEX IF EXISTS benchmark_data_idx CASCADE")
        .execute(&mut *conn)
        .await?;

    // Define configuration based on the desired execution method
    let index_definition = match exec_method {
        "mixed" => {
            // All fields are marked as fast for MixedFastFieldExec
            // IMPORTANT: ALL fields, including ID and those used in SELECT must be fast
            // Use keyword tokenizer for string fields to ensure exact matching
            "CREATE INDEX benchmark_data_idx ON benchmark_data 
            USING bm25(
                id, 
                string_field1,
                string_field2,
                numeric_field1,
                numeric_field2,
                numeric_field3
            ) WITH (
                key_field = 'id',
                text_fields = '{\"string_field1\": {\"fast\": true, \"tokenizer\": {\"type\": \"keyword\"}}, \"string_field2\": {\"fast\": true, \"tokenizer\": {\"type\": \"keyword\"}}}',
                numeric_fields = '{\"numeric_field1\": {\"fast\": true}, \"numeric_field2\": {\"fast\": true}, \"numeric_field3\": {\"fast\": true}}'
            )"
        }
        "normal" => {
            // No fast fields to force NormalScanExecState
            "CREATE INDEX benchmark_data_idx ON benchmark_data 
            USING bm25(
                id, 
                string_field1,
                string_field2,
                numeric_field1,
                numeric_field2,
                numeric_field3
            ) WITH (
                key_field = 'id',
                text_fields = '{\"string_field1\": {\"fast\": false}, \"string_field2\": {\"fast\": false}}',
                numeric_fields = '{\"numeric_field1\": {\"fast\": false}, \"numeric_field2\": {\"fast\": false}, \"numeric_field3\": {\"fast\": false}}'
            )"
        }
        _ => {
            panic!("Unsupported execution method: {}", exec_method);
        }
    };

    // Create the index
    println!("Creating index for {} execution method", exec_method);
    sqlx::query(index_definition).execute(&mut *conn).await?;

    // Wait a moment for the index to be fully ready
    sqlx::query("SELECT pg_sleep(0.5)")
        .execute(&mut *conn)
        .await?;

    // Ensure index scan is used
    sqlx::query("SET enable_seqscan = off")
        .execute(&mut *conn)
        .await?;
    sqlx::query("SET enable_bitmapscan = off")
        .execute(&mut *conn)
        .await?;
    sqlx::query("SET enable_indexscan = off")
        .execute(&mut *conn)
        .await?;

    // Verify the index was created
    let verify_index =
        sqlx::query("SELECT indexname FROM pg_indexes WHERE indexname = 'benchmark_data_idx'")
            .fetch_optional(&mut *conn)
            .await?;

    if verify_index.is_some() {
        println!("Index 'benchmark_data_idx' created successfully");
    } else {
        println!("WARNING: Index 'benchmark_data_idx' not found!");
    }

    // Always analyze to ensure the planner has accurate statistics
    sqlx::query("ANALYZE benchmark_data")
        .execute(&mut *conn)
        .await?;

    Ok(())
}

/// Run a benchmark for a specific query with the specified execution method (mixed or normal)
async fn run_benchmark(
    conn: &mut PgConnection,
    query: &str,
    test_name: &str,
    force_execution_method: Option<&str>,
) -> Result<BenchmarkResult> {
    let mut total_time_ms: f64 = 0.0;
    let mut min_time_ms: f64 = f64::MAX;
    let mut max_time_ms: f64 = 0.0;

    // Create appropriate index if execution method is specified
    // This should be either "mixed" or "normal"
    if let Some(exec_method) = force_execution_method {
        create_index_for_execution_method(&mut *conn, exec_method).await?;
    }

    // Reset/clear cache to ensure clean runs
    sqlx::query("SELECT pg_stat_reset()")
        .execute(&mut *conn)
        .await?;

    // The query to run, with no modification
    let query_to_run = query.to_string();

    // Warmup runs to ensure caches are primed
    for _ in 0..WARMUP_ITERATIONS {
        let _ = sqlx::query(&query_to_run).execute(&mut *conn).await?;
    }

    // Get the execution plan to determine which execution method is used
    let explain_query = format!("EXPLAIN (ANALYZE, FORMAT JSON) {}", query_to_run);
    let (plan,): (Value,) = sqlx::query_as(&explain_query).fetch_one(&mut *conn).await?;
    let exec_method = detect_exec_method(&plan);

    // Debug: print out the execution method being used
    println!(
        "Test '{}' is using execution method: {}",
        test_name, exec_method
    );

    // Run actual benchmark iterations
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        let _ = sqlx::query(&query_to_run).execute(&mut *conn).await?;
        let elapsed = start.elapsed();
        let time_ms = elapsed.as_secs_f64() * 1000.0;

        total_time_ms += time_ms;
        min_time_ms = min_time_ms.min(time_ms);
        max_time_ms = max_time_ms.max(time_ms);
    }

    let avg_time_ms = total_time_ms / ITERATIONS as f64;

    Ok(BenchmarkResult {
        test_name: test_name.to_string(),
        exec_method,
        avg_time_ms,
        min_time_ms,
        max_time_ms,
    })
}

/// Display benchmark results and comparisons
fn display_results(results: &[BenchmarkResult]) {
    println!("\n======== BENCHMARK RESULTS ========");
    println!(
        "{:<30} {:<20} {:<15} {:<15} {:<15}",
        "Test Name", "Exec Method", "Avg Time (ms)", "Min Time (ms)", "Max Time (ms)"
    );
    println!("{}", "=".repeat(95));

    for result in results {
        println!(
            "{:<30} {:<20} {:<15.2} {:<15.2} {:<15.2}",
            result.test_name,
            result.exec_method,
            result.avg_time_ms,
            result.min_time_ms,
            result.max_time_ms
        );
    }

    // Group by test_name to compare different execution methods
    let mut test_map = std::collections::HashMap::new();
    for result in results {
        test_map
            .entry(result.test_name.clone())
            .or_insert_with(Vec::new)
            .push(result.clone());
    }

    println!("\n======== PERFORMANCE COMPARISON ========");
    println!(
        "{:<30} {:<15} {:<15} {:<15} {:<15}",
        "Test Name", "Mixed (ms)", "Normal (ms)", "Ratio", "Performance"
    );
    println!("{}", "=".repeat(90));

    for (test_name, test_results) in test_map {
        // For each test, find a fast field execution method result and a normal execution method result
        let fast_result = test_results
            .iter()
            .find(|r| r.exec_method.contains("MixedFastFieldExec"));

        let normal_result = test_results
            .iter()
            .find(|r| r.exec_method.contains("NormalScanExecState"));

        if let (Some(fast), Some(normal)) = (fast_result, normal_result) {
            let ratio = fast.avg_time_ms / normal.avg_time_ms;
            let performance = if fast.avg_time_ms > normal.avg_time_ms {
                "SLOWER"
            } else {
                "FASTER"
            };

            println!(
                "{:<30} {:<15.2} {:<15.2} {:<15.2} {:<15}",
                test_name, fast.avg_time_ms, normal.avg_time_ms, ratio, performance
            );
        }
    }
}

#[rstest]
async fn benchmark_mixed_fast_fields(mut conn: PgConnection) -> Result<()> {
    // Set up the benchmark database
    setup_benchmark_database(&mut conn).await?;

    let mut results = Vec::new();

    // Test 1: Basic query with mixed fields - use @@@ operator for string comparisons
    let basic_query =
        "SELECT id, string_field1, string_field2, numeric_field1, numeric_field2, numeric_field3 
         FROM benchmark_data 
         WHERE numeric_field1 < 500 AND string_field1 @@@ '\"alpha\"' AND string_field2 @@@ '\"red\"'
         ORDER BY id";

    // Run with all fast fields to force MixedFastFieldExec
    let mixed_result = run_benchmark(
        &mut conn,
        basic_query,
        "Basic Mixed Fields (MixedFastFieldExec)",
        Some("mixed"),
    )
    .await?;

    // ENFORCE: Validate we're actually using MixedFastFieldExec
    assert!(
        mixed_result.exec_method.contains("MixedFastFieldExec"),
        "Mixed benchmark is not using MixedFastFieldExec as intended. Got: {}",
        mixed_result.exec_method
    );
    results.push(mixed_result);

    // Run with no fast fields to force NormalScanExecState
    let normal_result = run_benchmark(
        &mut conn,
        basic_query,
        "Basic Mixed Fields (NormalScanExecState)",
        Some("normal"),
    )
    .await?;

    // ENFORCE: Validate we're actually using NormalScanExecState
    assert!(
        normal_result.exec_method.contains("NormalScanExecState"),
        "Normal benchmark is not using NormalScanExecState as intended. Got: {}",
        normal_result.exec_method
    );
    results.push(normal_result);

    // Test 2: Count query (simpler test)
    let count_query = "SELECT numeric_field1, string_field1
                      FROM benchmark_data 
                      WHERE numeric_field1 < 500 AND string_field1 @@@ '\"alpha\"'";

    // Run with mixed fast field execution
    let count_mixed_result = run_benchmark(
        &mut conn,
        count_query,
        "Count Query (MixedFastFieldExec)",
        Some("mixed"),
    )
    .await?;

    // ENFORCE: Validate we're actually using MixedFastFieldExec
    assert!(
        count_mixed_result
            .exec_method
            .contains("MixedFastFieldExec"),
        "Count Mixed benchmark is not using MixedFastFieldExec as intended. Got: {}",
        count_mixed_result.exec_method
    );
    results.push(count_mixed_result);

    // Run with normal execution
    let count_normal_result = run_benchmark(
        &mut conn,
        count_query,
        "Count Query (NormalScanExecState)",
        Some("normal"),
    )
    .await?;

    // ENFORCE: Validate we're actually using NormalScanExecState
    assert!(
        count_normal_result
            .exec_method
            .contains("NormalScanExecState"),
        "Count Normal benchmark is not using NormalScanExecState as intended. Got: {}",
        count_normal_result.exec_method
    );
    results.push(count_normal_result);

    // Display all benchmark results
    display_results(&results);

    // Find an appropriate pair to compare
    let mixed_test_cases = results
        .iter()
        .filter(|r| r.exec_method.contains("MixedFastFieldExec"))
        .collect::<Vec<_>>();

    let normal_test_cases = results
        .iter()
        .filter(|r| r.exec_method.contains("NormalScanExecState"))
        .collect::<Vec<_>>();

    if !mixed_test_cases.is_empty() && !normal_test_cases.is_empty() {
        let mixed = mixed_test_cases[0];
        let normal = normal_test_cases[0];

        // Print the ratio for verification
        let ratio = mixed.avg_time_ms / normal.avg_time_ms;
        println!(
            "\nMixedFastFieldExec to NormalScanExecState performance ratio: {:.2}",
            ratio
        );
        println!(
            "Is MixedFastFieldExec slower than NormalScanExecState? {}",
            if ratio > 1.0 { "Yes" } else { "No" }
        );

        if ratio > 2.0 {
            println!(
                "\n⚠️ WARNING: MixedFastFieldExec is more than 2x slower than NormalScanExecState!"
            );
            println!("This suggests there are significant performance issues with the fast field implementation.");
            println!(
                "Review the optimization recommendations in mixed_fast_fields_optimizations.md"
            );
        }
    }

    Ok(())
}

/// Validate that the different execution methods return the same results
/// and enforce that we're actually using the intended execution methods
#[rstest]
async fn validate_mixed_fast_fields_correctness(mut conn: PgConnection) -> Result<()> {
    // Set up the benchmark database
    setup_benchmark_database(&mut conn).await?;

    // Define a test query that will use both string and numeric fast fields
    let test_query =
        "SELECT id, string_field1, string_field2, numeric_field1, numeric_field2, numeric_field3 
         FROM benchmark_data 
         WHERE numeric_field1 < 500 AND string_field1 @@@ '\"alpha\"' AND string_field2 @@@ '\"red\"'
         ORDER BY id";

    // Create index for MixedFastFieldExec
    create_index_for_execution_method(&mut conn, "mixed").await?;

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

    // Analyze to update statistics
    sqlx::query("ANALYZE benchmark_data")
        .execute(&mut conn)
        .await?;

    // Get results with MixedFastFieldExec
    let mixed_results = sqlx::query(test_query).fetch_all(&mut conn).await?;

    // Get execution plan to verify method
    let (mixed_plan,): (Value,) = sqlx::query_as(&format!("EXPLAIN (FORMAT JSON) {}", test_query))
        .fetch_one(&mut conn)
        .await?;

    let mixed_method = detect_exec_method(&mixed_plan);
    println!("Execution method for mixed test: {}", mixed_method);

    // ENFORCE: Validate we're actually using the MixedFastFieldExec method
    assert!(
        mixed_method.contains("MixedFastFieldExec"),
        "Expected MixedFastFieldExec execution method, but got: {}. Check index configuration and query settings.",
        mixed_method
    );

    // Create index for NormalScanExecState
    create_index_for_execution_method(&mut conn, "normal").await?;

    // Get results with NormalScanExecState
    let normal_results = sqlx::query(test_query).fetch_all(&mut conn).await?;

    // Get execution plan to verify method
    let (normal_plan,): (Value,) = sqlx::query_as(&format!("EXPLAIN (FORMAT JSON) {}", test_query))
        .fetch_one(&mut conn)
        .await?;

    let normal_method = detect_exec_method(&normal_plan);
    println!("Execution method for normal test: {}", normal_method);

    // ENFORCE: Validate we're actually using the NormalScanExecState method
    assert!(
        normal_method.contains("NormalScanExecState"),
        "Expected NormalScanExecState execution method, but got: {}. Check index configuration and query settings.",
        normal_method
    );

    // Compare result counts
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

    println!("✅ Correctness validation passed: Both execution methods returned identical results");

    Ok(())
}
