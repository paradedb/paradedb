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

use anyhow::Result;
use pretty_assertions::{assert_eq, assert_ne};
use serde_json::Value;
use sqlx::{PgConnection, Row};
use std::time::Instant;

use crate::micro_benchmarks::setup_benchmark_database;

pub const ASSERT_HEAP_VIRTUAL_TUPLES: bool = false;

/// Structure to store benchmark results
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub test_name: String,
    pub exec_method: String,
    pub avg_time_ms: f64,
    pub min_time_ms: f64,
    pub max_time_ms: f64,
}

/// Benchmark configuration settings
pub struct BenchmarkConfig {
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub num_rows: usize,
    pub batch_size: usize,
    pub table_name: String,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 5,
            warmup_iterations: 2,
            num_rows: 10000,
            batch_size: 10000,
            table_name: "benchmark_data".to_string(),
        }
    }
}

/// Recursively searches a JSON structure and collects all values for a specific field name
pub fn collect_json_field_values(json: &Value, field_name: &str) -> Vec<Value> {
    let mut results = Vec::new();

    match json {
        Value::Object(map) => {
            // Check if this object contains the field we're looking for
            if let Some(value) = map.get(field_name) {
                results.push(value.clone());
            }

            // Recursively search all values in this object
            for (_, value) in map {
                results.extend(collect_json_field_values(value, field_name));
            }
        }
        Value::Array(arr) => {
            // Recursively search all elements in this array
            for item in arr {
                results.extend(collect_json_field_values(item, field_name));
            }
        }
        // Base case: other JSON value types can't contain nested fields
        _ => {}
    }

    results
}

/// Collects and prints all important metrics from an execution plan
pub fn check_execution_plan_metrics(execution_method: &str, plan: &Value) {
    let plan_str = plan.to_string();
    println!("Execution plan: {}", plan_str);

    // Define metrics to collect
    let metrics = ["Heap Fetches", "Virtual Tuples", "Invisible Tuples"];

    // Collect and print each metric
    for metric in metrics {
        let values = collect_json_field_values(plan, metric);
        if ASSERT_HEAP_VIRTUAL_TUPLES {
            if execution_method == "MixedFastFieldExec"
                || execution_method == "NumericFastFieldExec"
                || execution_method == "StringFastFieldExec"
            {
                values.iter().for_each(|v| {
                    assert!(v.is_number());
                    if metric == "Heap Fetches" {
                        assert_eq!(v.as_i64().unwrap(), 0);
                    }
                    if metric == "Virtual Tuples" {
                        // Fast fields should have virtual tuples
                        assert_ne!(v.as_i64().unwrap(), 0);
                    }
                    if metric == "Invisible Tuples" {
                        assert_eq!(v.as_i64().unwrap(), 0);
                    }
                });
            } else {
                values.iter().for_each(|v| {
                    assert!(v.is_number());
                    if metric == "Heap Fetches" {
                        // Normal scan should have heap fetches
                        assert_ne!(v.as_i64().unwrap(), 0);
                    }
                    if metric == "Virtual Tuples" {
                        assert_eq!(v.as_i64().unwrap(), 0);
                    }
                    if metric == "Invisible Tuples" {
                        assert_eq!(v.as_i64().unwrap(), 0);
                    }
                });
            }
        }
        if !values.is_empty() {
            println!(" - {}: {:?}", metric, values);
        }
    }
}

/// Detects which execution method was used based on the JSON execution plan
pub fn detect_exec_method(plan: &Value) -> String {
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

/// Run a benchmark for a specific query with the specified execution method (mixed or normal)
pub async fn run_benchmark(
    conn: &mut PgConnection,
    query: &str,
    test_name: &str,
    execution_method: &str,
    config: &BenchmarkConfig,
) -> Result<BenchmarkResult> {
    let mut total_time_ms: f64 = 0.0;
    let mut min_time_ms: f64 = f64::MAX;
    let mut max_time_ms: f64 = 0.0;

    set_execution_method(conn, execution_method).await?;

    // The query to run, with no modification
    let query_to_run = query.to_string();

    // Warmup runs to ensure caches are primed
    for _ in 0..config.warmup_iterations {
        let _ = sqlx::query(&query_to_run).fetch_all(&mut *conn).await?;
    }

    // Get the execution plan to determine which execution method is used
    let explain_query = format!("EXPLAIN (VERBOSE, ANALYZE, FORMAT JSON) {}", query_to_run);
    let (plan,): (Value,) = sqlx::query_as(&explain_query).fetch_one(&mut *conn).await?;

    let exec_method = detect_exec_method(&plan);

    // Debug: print out the execution method being used
    println!("Test '{}' → using {}", test_name, exec_method);

    // Print comprehensive metrics from the execution plan
    check_execution_plan_metrics(execution_method, &plan);

    // Run actual benchmark iterations
    for _i in 0..config.iterations {
        let start = Instant::now();
        let _res = sqlx::query(&query_to_run).fetch_all(&mut *conn).await?;
        let elapsed = start.elapsed();
        let time_ms = elapsed.as_secs_f64() * 1000.0;

        total_time_ms += time_ms;
        min_time_ms = min_time_ms.min(time_ms);
        max_time_ms = max_time_ms.max(time_ms);
    }

    let avg_time_ms = total_time_ms / config.iterations as f64;

    Ok(BenchmarkResult {
        test_name: test_name.to_string(),
        exec_method,
        avg_time_ms,
        min_time_ms,
        max_time_ms,
    })
}

/// Display benchmark results and comparisons
pub fn display_results(results: &[BenchmarkResult]) {
    println!("\n======== BENCHMARK RESULTS ========");
    println!(
        "{:<65} {:<20} {:<15} {:<15} {:<15}",
        "Test Name", "Exec Method", "Avg Time (ms)", "Min Time (ms)", "Max Time (ms)"
    );
    println!("{}", "=".repeat(135));

    for result in results {
        println!(
            "{:<65} {:<20} {:<15.2} {:<15.2} {:<15.2}",
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
        "{:<45} {:<15} {:<15} {:<15} {:<15}",
        "Test Group", "FastField (ms)", "Normal (ms)", "Ratio", "Performance"
    );
    println!("{}", "=".repeat(115));

    let mut test_groups = test_groups.iter().collect::<Vec<_>>();
    test_groups.sort_by_key(|(name, _)| name.to_string());
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
            let ratio = mixed.min_time_ms / normal.min_time_ms;
            let performance = if mixed.min_time_ms > normal.min_time_ms {
                "SLOWER"
            } else {
                "FASTER"
            };

            println!(
                "{:<45} {:<15.2} {:<15.2} {:<15.2} {:<15}",
                base_name, mixed.min_time_ms, normal.min_time_ms, ratio, performance
            );
        } else {
            // For debugging if no match found
            println!(
                "{:<45} {:<15} {:<15} {:<15} {:<15}",
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
pub async fn run_benchmarks_with_methods(
    conn: &mut PgConnection,
    query: &str,
    benchmark_name: &str,
    methods: &[&str],
    results: &mut Vec<BenchmarkResult>,
    config: &BenchmarkConfig,
) -> Result<()> {
    println!("Running {} test...", benchmark_name);

    for method_name in methods {
        let full_benchmark_name = format!("{} ({})", benchmark_name, method_name);

        let result = run_benchmark(conn, query, &full_benchmark_name, method_name, config).await?;

        // Validate we're using the expected execution method
        assert!(
            result.exec_method.contains(method_name),
            "{} benchmark is not using {} as intended. Got: {}",
            benchmark_name,
            method_name,
            result.exec_method
        );

        // Print the result
        println!("{:?}", result);

        results.push(result);
    }

    Ok(())
}

/// Setup PostgreSQL settings for the specific execution method
pub async fn set_execution_method(conn: &mut PgConnection, execution_method: &str) -> Result<()> {
    // Create appropriate index if execution method is specified
    // This should be either "MixedFastFieldExec" or "StringFastFieldExec" or "NumericFastFieldExec"
    if execution_method == "MixedFastFieldExec" {
        sqlx::query("SET paradedb.enable_fast_field_exec = false")
            .execute(&mut *conn)
            .await?;
        sqlx::query("SET paradedb.enable_mixed_fast_field_exec = true")
            .execute(&mut *conn)
            .await?;
    } else if execution_method == "StringFastFieldExec"
        || execution_method == "NumericFastFieldExec"
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

    let _count: i64 =
        sqlx::query("SELECT COUNT(*) FROM benchmark_data WHERE id @@@ paradedb.all()")
            .fetch_one(&mut *conn)
            .await?
            .get(0);

    sqlx::query("SET max_parallel_workers_per_gather = 0")
        .execute(&mut *conn)
        .await?;

    Ok(())
}

pub async fn benchmark_mixed_fast_fields(
    conn: &mut PgConnection,
    iterations: usize,
    warmup_iterations: usize,
    num_rows: usize,
    batch_size: usize,
) -> Result<()> {
    // Configure the benchmark
    let config = BenchmarkConfig {
        iterations,
        warmup_iterations,
        num_rows,
        batch_size,
        table_name: "benchmark_data".to_string(),
    };

    // Set up the benchmark database
    setup_benchmark_database(conn, config.num_rows, &config.table_name, config.batch_size).await?;

    println!("========================================");
    println!("Starting mixed fast fields benchmark");
    println!("========================================");

    let mut results = Vec::new();

    // Test 1: Basic query with mixed fields - use @@@ operator for string comparisons
    // Updated to use the new fields
    let basic_query =
        "SELECT id, string_field1, string_field2, numeric_field1, numeric_field2, numeric_field3 
         FROM benchmark_data 
         WHERE numeric_field1 < 500 AND string_field1 @@@ '\"alpha_complex_identifier_123456789\"' AND string_field2 @@@ '\"red_velvet_cupcake_with_cream_cheese_frosting\"'
         ORDER BY id";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        conn,
        basic_query,
        "Basic - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    // Test 2: Count query with long text
    let count_query = "SELECT numeric_field1, string_field1
                      FROM benchmark_data 
                      WHERE long_text @@@ '\"database\"' AND numeric_field1 < 500";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        conn,
        count_query,
        "Count Query - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    // Test 3: Complex Aggregation Query with more complex fields
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
                json_data @@@ '\"user\"' AND
                long_text @@@ '\"performance\"' AND
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
        conn,
        complex_query,
        "Complex Aggregation - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    // Test 4: JSON query - should be much faster with fast fields
    let json_query = "
        SELECT 
            string_field1
        FROM benchmark_data 
        WHERE 
            json_data @@@ '\"Sports\"'
        ORDER BY string_field1";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        conn,
        json_query,
        "JSON Query - StringFF",
        &["StringFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        conn,
        json_query,
        "JSON Query - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    // Test 5: Long text search - should show big difference with fast fields
    let long_text_query = "
        SELECT 
            string_field1
        FROM benchmark_data 
        WHERE 
            long_text @@@ '\"database\" AND \"performance\"'
        ORDER BY string_field1";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        conn,
        long_text_query,
        "Long Text Query - StringFF",
        &["StringFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        conn,
        long_text_query,
        "Long Text Query - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    // Test 6: Heavy ordering query - should benefit from fast fields
    let ordering_query = "
        SELECT 
            id, string_field1, string_field2, numeric_field1, numeric_field2
        FROM benchmark_data 
        WHERE 
            numeric_field1 < 800 AND
            string_field1 @@@ 'alpha_complex_identifier_123456789'
        ORDER BY numeric_field1, numeric_field2 DESC, string_field1";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        conn,
        ordering_query,
        "Heavy Ordering - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    // Test 7: Group by numeric field with filtered count
    let group_count_query = "SELECT numeric_field1, COUNT(*) FROM benchmark_data WHERE string_field1 @@@ '\"alpha_complex_identifier_123456789\"' GROUP BY numeric_field1";

    run_benchmarks_with_methods(
        conn,
        group_count_query,
        "Group By Count - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    run_benchmarks_with_methods(
        conn,
        group_count_query,
        "Group By Count - NumericFF",
        &["NumericFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    // Test 8: Select ID with filter
    let select_id_query = "SELECT id FROM benchmark_data WHERE string_field1 @@@ '\"alpha_complex_identifier_123456789\"'";

    run_benchmarks_with_methods(
        conn,
        select_id_query,
        "Select ID - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    run_benchmarks_with_methods(
        conn,
        select_id_query,
        "Select ID - NumericFF",
        &["NumericFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    // Test 9: Aggregation with sum
    let sum_query = "SELECT SUM(numeric_field1) FROM benchmark_data WHERE string_field1 @@@ '\"alpha_complex_identifier_123456789\"'";

    run_benchmarks_with_methods(
        conn,
        sum_query,
        "Sum Aggregation - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    run_benchmarks_with_methods(
        conn,
        sum_query,
        "Sum Aggregation - NumericFF",
        &["NumericFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    // Test 10: Group by string with count
    let string_group_query = "SELECT string_field1, COUNT(*) FROM benchmark_data WHERE long_text @@@ '\"database\"' GROUP BY string_field1";

    run_benchmarks_with_methods(
        conn,
        string_group_query,
        "String Group Count - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    run_benchmarks_with_methods(
        conn,
        string_group_query,
        "String Group Count - StringFF",
        &["StringFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    // Test 11: Set up a self-join to simulate a join between two tables
    let join_query = "
        WITH a AS (
            SELECT id AS a_id, numeric_field1 AS a_numeric, string_field1 AS a_string FROM benchmark_data
        ),
        b AS (
            SELECT id AS b_id, numeric_field1 AS b_numeric, string_field2 AS b_string FROM benchmark_data
        )
        SELECT a.a_numeric, b.b_string 
        FROM a, b 
        WHERE a.a_numeric = b.b_numeric AND b.b_string @@@ '\"red_velvet_cupcake_with_cream_cheese_frosting\"'";

    run_benchmarks_with_methods(
        conn,
        join_query,
        "Join Query - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
        &config,
    )
    .await?;

    // Display all benchmark results
    display_results(&results);

    Ok(())
}
