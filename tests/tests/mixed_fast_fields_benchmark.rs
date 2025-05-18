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
use sqlx::Row;
use std::time::Instant;

// Number of benchmark iterations for each query
const ITERATIONS: usize = 5;
// Number of warmup iterations before measuring performance
const WARMUP_ITERATIONS: usize = 2;
// Number of rows to use in the benchmark
const NUM_ROWS_BENCHMARK: usize = 100000; // Reduced for faster test runs
const NUM_ROWS_VALIDATION: usize = 1000; // Reduced for faster test runs
const BATCH_SIZE: usize = 100000; // For efficiency with large datasets, use batch inserts
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
    let plan_str = plan.to_string();
    let uses_custom_scan = plan_str.contains("ParadeDB Scan");

    // If the custom scan method is explicitly mentioned, extract it
    if plan_str.contains("Exec Method") {
        if plan_str.contains("MixedFastFieldExecState") {
            return "MixedFastFieldExec".to_string();
        } else if plan_str.contains("StringFastFieldExecState") {
            return "StringFastFieldExec".to_string();
        } else if plan_str.contains("NumericFastFieldExecState") {
            return "NumericFastFieldExec".to_string();
        } else if plan_str.contains("NormalScanExecState") {
            return "NormalScanExecState".to_string();
        } else if uses_custom_scan {
            panic!("Unknown execution method: {}", plan_str);
        }
    }

    // Default when no specific method is found
    "NormalScanExecState".to_string()
}

/// Setup function to create test table and data
async fn setup_benchmark_database(conn: &mut PgConnection, num_rows: usize) -> Result<()> {
    // First check if the table exists
    let table_exists_query =
        "SELECT EXISTS (SELECT FROM pg_tables WHERE tablename = 'benchmark_data')";
    let table_exists: bool = sqlx::query(table_exists_query)
        .fetch_one(&mut *conn)
        .await?
        .get(0);

    println!("Table exists check: {}", table_exists);

    let mut current_rows = if table_exists {
        // Count existing rows more reliably with explicit casting to bigint
        let count_result = sqlx::query("SELECT COUNT(*)::bigint FROM benchmark_data")
            .fetch_one(&mut *conn)
            .await;

        match count_result {
            Ok(row) => {
                let count: i64 = row.get(0);
                println!("Found existing table with {} rows", count);
                count as usize
            }
            Err(e) => {
                println!(
                    "Error counting rows: {}, assuming table needs recreation",
                    e
                );
                // If we can't count rows, the table might be corrupted
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

                0
            }
        }
    } else {
        println!("Table doesn't exist, creating new one");
        // Create the table if it doesn't exist
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
        0
    };

    println!(
        "Table benchmark_data already exists with {} rows (requested: {})",
        current_rows, num_rows
    );

    if current_rows == num_rows {
        return Ok(());
    }

    if current_rows > num_rows {
        // Drop table if exists
        println!(
            "Table has more rows than needed ({}), recreating...",
            current_rows
        );
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

        current_rows = 0;
    }

    let rows_to_add = num_rows - current_rows;
    println!("Adding {} more rows to benchmark_data", rows_to_add);

    // Create arrays for test data
    let string_array1 = vec![
        "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa",
        "lambda", "mu", "nu", "xi", "omicron", "pi", "rho", "sigma", "tau", "upsilon", "phi",
        "chi", "psi", "omega",
    ];

    let string_array2 = vec![
        "red", "orange", "yellow", "green", "blue", "indigo", "violet", "black", "white", "gray",
    ];

    let mut inserted = 0;

    while inserted < rows_to_add {
        // Create a batch insert statement
        let mut batch_query = String::from(
            "INSERT INTO benchmark_data (string_field1, string_field2, numeric_field1, numeric_field2, numeric_field3) VALUES "
        );

        let batch_end = (inserted + BATCH_SIZE).min(rows_to_add);
        for i in (current_rows + inserted)..(current_rows + batch_end) {
            if i > current_rows + inserted {
                batch_query.push_str(", ");
            }

            let string1 = string_array1[i % string_array1.len()];
            let string2 = string_array2[i % string_array2.len()];
            let num1 = (i % 1000) as i32;
            let num2 = (i % 100) as f32;
            let num3 = (i % 10000) as i32;

            // Add placeholders to query
            batch_query.push_str(&format!(
                "('{}', '{}', {}, {}, {})",
                string1, string2, num1, num2, num3
            ));
        }

        // Execute batch insert
        sqlx::query(&batch_query).execute(&mut *conn).await?;

        inserted += batch_end - (current_rows + inserted);

        if inserted % 10000 == 0 && inserted > 0 {
            println!("Inserted {} of {} rows...", inserted, rows_to_add);
        }
    }

    println!(
        "Database setup complete with {} rows (was: {}, added: {})",
        num_rows, current_rows, rows_to_add
    );

    // Run VACUUM ANALYZE after creating or updating the table
    println!("Running VACUUM ANALYZE on new data...");
    sqlx::query("VACUUM ANALYZE benchmark_data")
        .execute(&mut *conn)
        .await?;

    // Verify the actual row count
    let final_count: i64 = sqlx::query("SELECT COUNT(*)::bigint FROM benchmark_data")
        .fetch_one(&mut *conn)
        .await?
        .get(0);

    println!("Final table verification: contains {} rows", final_count);

    // Ensure count matches what we expect
    assert_eq!(
        final_count as usize, num_rows,
        "Table contains {} rows but should have {} rows",
        final_count, num_rows
    );

    create_bm25_index(conn).await?;

    Ok(())
}

/// Creates an index with the specified configuration
async fn create_bm25_index(conn: &mut PgConnection) -> Result<()> {
    // First drop any existing index
    sqlx::query("DROP INDEX IF EXISTS benchmark_data_idx CASCADE")
        .execute(&mut *conn)
        .await?;

    // Define configuration based on the desired execution method
    // All fields are marked as fast for MixedFastFieldExec
    // IMPORTANT: ALL fields, including ID and those used in SELECT must be fast
    // Use keyword tokenizer for string fields to ensure exact matching
    let index_definition = "CREATE INDEX benchmark_data_idx ON benchmark_data 
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
    )";

    // Create the index
    println!("Creating index...");
    sqlx::query(index_definition).execute(&mut *conn).await?;

    // Wait a moment for the index to be fully ready
    sqlx::query("SELECT pg_sleep(0.5)")
        .execute(&mut *conn)
        .await?;

    // Run a full VACUUM ANALYZE to update statistics after index creation
    println!("Running VACUUM ANALYZE after index creation...");
    sqlx::query("VACUUM ANALYZE benchmark_data")
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

    // Run a full VACUUM ANALYZE to ensure statistics are up-to-date
    // This helps the query planner make better decisions
    println!("Running VACUUM ANALYZE on benchmark_data...");
    sqlx::query("VACUUM ANALYZE benchmark_data")
        .execute(&mut *conn)
        .await?;

    // Reset/clear cache to ensure clean runs
    sqlx::query("SELECT pg_stat_reset()")
        .execute(&mut *conn)
        .await?;

    Ok(())
}

async fn set_execution_method(
    conn: &mut PgConnection,
    execution_method: &Option<&str>,
) -> Result<()> {
    // Create appropriate index if execution method is specified
    // This should be either "MixedFastFieldExec" or "StringFastFieldExec" or "NumericFastFieldExec"
    if execution_method.is_some() && execution_method.unwrap() == "MixedFastFieldExec" {
        sqlx::query("SET paradedb.enable_fast_field_exec = false")
            .execute(&mut *conn)
            .await?;
        sqlx::query("SET paradedb.enable_mixed_fast_field_exec = true")
            .execute(&mut *conn)
            .await?;
    } else if execution_method.is_some()
        && (execution_method.unwrap() == "StringFastFieldExec"
            || execution_method.unwrap() == "NumericFastFieldExec")
    {
        sqlx::query("SET paradedb.enable_fast_field_exec = true")
            .execute(&mut *conn)
            .await?;
        sqlx::query("SET paradedb.enable_mixed_fast_field_exec = false")
            .execute(&mut *conn)
            .await?;
    } else {
        sqlx::query("SET paradedb.enable_fast_field_exec = false")
            .execute(&mut *conn)
            .await?;
        sqlx::query("SET paradedb.enable_mixed_fast_field_exec = false")
            .execute(&mut *conn)
            .await?;
    }

    Ok(())
}

/// Run a benchmark for a specific query with the specified execution method (mixed or normal)
async fn run_benchmark(
    conn: &mut PgConnection,
    query: &str,
    test_name: &str,
    execution_method: Option<&str>,
) -> Result<BenchmarkResult> {
    let mut total_time_ms: f64 = 0.0;
    let mut min_time_ms: f64 = f64::MAX;
    let mut max_time_ms: f64 = 0.0;

    set_execution_method(conn, &execution_method).await?;

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
    println!("Test '{}' → using {}", test_name, exec_method);

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
        "{:<42} {:<20} {:<15} {:<15} {:<15}",
        "Test Name", "Exec Method", "Avg Time (ms)", "Min Time (ms)", "Max Time (ms)"
    );
    println!("{}", "=".repeat(112));

    for result in results {
        println!(
            "{:<42} {:<20} {:<15.2} {:<15.2} {:<15.2}",
            result.test_name,
            result.exec_method,
            result.avg_time_ms,
            result.min_time_ms,
            result.max_time_ms
        );
    }

    // Group by base test name (without execution method specification)
    let mut test_groups = std::collections::HashMap::new();

    for result in results {
        // Extract base test name (e.g., "Basic Mixed Fields" from "Basic Mixed Fields (MixedFastFieldExec)")
        let base_name = if let Some(pos) = result.test_name.find(" (") {
            result.test_name[..pos].to_string()
        } else {
            result.test_name.clone()
        };

        test_groups
            .entry(base_name)
            .or_insert_with(Vec::new)
            .push(result.clone());
    }

    println!("\n======== PERFORMANCE COMPARISON ========");
    println!(
        "{:<30} {:<15} {:<15} {:<15} {:<15}",
        "Test Group", "MixedFF/StringFF (ms)", "Normal (ms)", "Ratio", "Performance"
    );
    println!("{}", "=".repeat(90));

    for (base_name, group_results) in test_groups {
        // Identify results by their test names, which include the execution method
        let mixed_result = group_results.iter().find(|r| {
            r.test_name.contains("MixedFastFieldExec")
                || r.test_name.contains("StringFastFieldExec")
                || r.test_name.contains("NumericFastFieldExec")
        });

        let normal_result = group_results
            .iter()
            .find(|r| r.test_name.contains("NormalScanExecState"));

        if let (Some(mixed), Some(normal)) = (mixed_result, normal_result) {
            let ratio = mixed.avg_time_ms / normal.avg_time_ms;
            let performance = if mixed.avg_time_ms > normal.avg_time_ms {
                "SLOWER"
            } else {
                "FASTER"
            };

            println!(
                "{:<30} {:<15.2} {:<15.2} {:<15.2} {:<15}",
                base_name, mixed.avg_time_ms, normal.avg_time_ms, ratio, performance
            );
        } else {
            // For debugging if no match found
            println!(
                "{:<30} {:<15} {:<15} {:<15} {:<15}",
                base_name,
                mixed_result.map_or("Not found", |_| "Found"),
                normal_result.map_or("Not found", |_| "Found"),
                "N/A",
                "N/A"
            );
        }
    }
}

/// Helper function to run benchmarks with multiple execution methods
async fn run_benchmarks_with_methods(
    conn: &mut PgConnection,
    query: &str,
    benchmark_name: &str,
    methods: &[&str], // (method_name, expected_substring_in_exec_method)
    results: &mut Vec<BenchmarkResult>,
) -> Result<()> {
    println!("Running {} test...", benchmark_name);

    for method_name in methods {
        let full_benchmark_name = format!("{} ({})", benchmark_name, method_name);

        let result = run_benchmark(conn, query, &full_benchmark_name, Some(method_name)).await?;

        // Validate we're using the expected execution method
        assert!(
            result.exec_method.contains(method_name),
            "{} benchmark is not using {} as intended. Got: {}",
            benchmark_name,
            method_name,
            result.exec_method
        );

        results.push(result);
    }

    Ok(())
}

#[rstest]
async fn benchmark_mixed_fast_fields(mut conn: PgConnection) -> Result<()> {
    // Set up the benchmark database
    setup_benchmark_database(&mut conn, NUM_ROWS_BENCHMARK).await?;

    println!("========================================");
    println!("Starting mixed fast fields benchmark");
    println!("========================================");

    let mut results = Vec::new();

    // Test 1: Basic query with mixed fields - use @@@ operator for string comparisons
    let basic_query =
        "SELECT id, string_field1, string_field2, numeric_field1, numeric_field2, numeric_field3 
         FROM benchmark_data 
         WHERE numeric_field1 < 500 AND string_field1 @@@ '\"alpha\"' AND string_field2 @@@ '\"red\"'
         ORDER BY id";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        &mut conn,
        basic_query,
        "Basic Mixed Fields",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
    )
    .await?;

    // Test 2: Count query (simpler test)
    let count_query = "SELECT numeric_field1, string_field1
                      FROM benchmark_data 
                      WHERE numeric_field1 < 500 AND string_field1 @@@ '\"alpha\"'";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        &mut conn,
        count_query,
        "Count Query",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
    )
    .await?;

    // Test 3: Complex Aggregation Query (1-2 seconds)
    // This query performs multiple aggregations across many groups with additional filtering
    let complex_query = "
        WITH filtered_data AS (
            SELECT 
                string_field1, 
                string_field2, 
                numeric_field1, 
                numeric_field2, 
                numeric_field3
            FROM benchmark_data 
            WHERE 
                (string_field1 @@@ 'IN [alpha beta gamma delta epsilon]') AND 
                (numeric_field1 BETWEEN 0 AND 900)
        ),
        agg_by_string1 AS (
            SELECT 
                string_field1,
                COUNT(*) as count,
                SUM(numeric_field1) as sum_field1,
                AVG(numeric_field2) as avg_field2,
                STDDEV(numeric_field3) as stddev_field3,
                MIN(numeric_field3) as min_field3,
                MAX(numeric_field3) as max_field3,
                COUNT(DISTINCT string_field2) as unique_string2
            FROM filtered_data
            GROUP BY string_field1
        ),
        agg_by_string2 AS (
            SELECT 
                string_field2,
                PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY numeric_field1) as median_field1,
                PERCENTILE_CONT(0.75) WITHIN GROUP (ORDER BY numeric_field1) as p75_field1,
                PERCENTILE_CONT(0.9) WITHIN GROUP (ORDER BY numeric_field1) as p90_field1,
                AVG(numeric_field3) as avg_field3
            FROM filtered_data
            GROUP BY string_field2
        )
        SELECT 
            s1.string_field1,
            s2.string_field2,
            s1.count,
            s1.sum_field1,
            s1.avg_field2,
            s1.stddev_field3,
            s1.min_field3,
            s1.max_field3,
            s1.unique_string2,
            s2.median_field1,
            s2.p75_field1,
            s2.p90_field1,
            s2.avg_field3
        FROM agg_by_string1 s1
        CROSS JOIN agg_by_string2 s2
        ORDER BY s1.sum_field1 DESC, s2.avg_field3 ASC
        LIMIT 100";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        &mut conn,
        complex_query,
        "Complex Aggregation",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
    )
    .await?;

    // Test 4: Single String Fast Field Query (1-2 seconds)
    // This query specifically tests performance with a single string fast field,
    // which should use StringFastFieldExecState instead of MixedFastFieldExecState
    let single_string_query = "
        SELECT 
            string_field1
        FROM benchmark_data 
        WHERE 
            string_field1 @@@ 'IN [alpha beta gamma delta epsilon]' AND
            string_field2 @@@ 'IN [red blue green]'
        ORDER BY string_field1";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        &mut conn,
        single_string_query,
        "Single String Field",
        &["StringFastFieldExec", "NormalScanExecState"],
        &mut results,
    )
    .await?;

    let single_string_query_with_numeric = "SELECT 
            string_field1, numeric_field1
        FROM benchmark_data 
        WHERE 
            string_field1 @@@ 'IN [alpha beta gamma delta epsilon]' AND
            string_field2 @@@ 'IN [red blue green]'
        ORDER BY string_field1";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        &mut conn,
        single_string_query_with_numeric,
        "Mixed Str/Num Field",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
    )
    .await?;

    // Test 5: Multiple Numeric Fast Field Query (1-2 seconds)
    // This query specifically tests performance with multiple numeric fast fields,
    // which should use NumericFastFieldExecState instead of MixedFastFieldExecState
    let multiple_numeric_query = "
            SELECT 
                numeric_field1, numeric_field2, numeric_field3
            FROM benchmark_data 
            WHERE 
                string_field1 @@@ 'IN [alpha beta gamma delta epsilon]' AND
                string_field2 @@@ 'IN [red blue green]'
            ORDER BY numeric_field1";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        &mut conn,
        multiple_numeric_query,
        "Multiple Numeric Fast Field",
        &["NumericFastFieldExec", "NormalScanExecState"],
        &mut results,
    )
    .await?;

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
    setup_benchmark_database(&mut conn, NUM_ROWS_VALIDATION).await?;

    println!("Testing query correctness between execution methods...");
    println!("────────────────────────────────────────────────────────");

    // Define a test query that will use both string and numeric fast fields
    let test_query =
        "SELECT id, string_field1, string_field2, numeric_field1, numeric_field2, numeric_field3 
         FROM benchmark_data 
         WHERE numeric_field1 < 500 AND string_field1 @@@ '\"alpha\"' AND string_field2 @@@ '\"red\"'
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

    set_execution_method(&mut conn, &Some("MixedFastFieldExec")).await?;

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

    set_execution_method(&mut conn, &Some("NormalScanExecState")).await?;

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
