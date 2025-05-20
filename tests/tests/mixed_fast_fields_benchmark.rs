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
use pretty_assertions::{assert_eq, assert_ne};
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
const NUM_ROWS_BENCHMARK: usize = 10000;
const NUM_ROWS_VALIDATION: usize = 1000; // Reduced for faster test runs
const BATCH_SIZE: usize = 10000; // For efficiency with large datasets, use batch inserts
/// Structure to store benchmark results
#[derive(Debug, Clone)]
struct BenchmarkResult {
    test_name: String,
    exec_method: String,
    avg_time_ms: f64,
    min_time_ms: f64,
    max_time_ms: f64,
}

/// Recursively searches a JSON structure and collects all values for a specific field name
fn collect_json_field_values(json: &Value, field_name: &str) -> Vec<Value> {
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
fn check_execution_plan_metrics(execution_method: &str, plan: &Value) {
    // Define metrics to collect
    let metrics = [
        "Heap Fetches",
        "Virtual Tuples",
        "Invisible Tuples",
        // "Tuples",
        // "Rows",
        // "Loops",
        // "Actual Rows",
        // "Plan Rows",
        // "Actual Startup Time",
        // "Actual Total Time",
        // "Planning Time",
        // "Execution Time",
    ];

    // Collect and print each metric
    for metric in metrics {
        let values = collect_json_field_values(plan, metric);
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
        if !values.is_empty() {
            println!(" - {}: {:?}", metric, values);
        }
    }
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
                        long_text TEXT NOT NULL,        -- Added new long text field
                        json_data TEXT NOT NULL,        -- Added JSON data field
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
                long_text TEXT NOT NULL,        -- Added new long text field
                json_data TEXT NOT NULL,        -- Added JSON data field
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
                long_text TEXT NOT NULL,        -- Added new long text field
                json_data TEXT NOT NULL,        -- Added JSON data field
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

    // Create arrays for test data - with longer, more complex strings
    let string_array1 = vec![
        "alpha_complex_identifier_123456789",
        "beta_node_configuration_987654321",
        "gamma_protocol_specification_abcdef",
        "delta_encryption_algorithm_123abc",
        "epsilon_network_routing_456def",
        "zeta_database_transaction_789ghi",
        "eta_compute_instance_group_321jkl",
        "theta_service_interconnect_654mno",
        "iota_persistent_volume_claim_987pqr",
        "kappa_distributed_cache_manager_210stu",
        "lambda_reconciliation_controller_543vwx",
        "mu_orchestration_deployment_876yz0",
        "nu_authorization_policy_service_108abc",
        "xi_replication_factor_settings_435def",
        "omicron_performance_metrics_collector_762ghi",
        "pi_configuration_parameter_store_089jkl",
        "rho_microservice_discovery_agent_316mno",
        "sigma_observability_infrastructure_643pqr",
        "tau_credential_rotation_mechanism_970stu",
        "upsilon_load_balancing_strategy_297vwx",
        "phi_disaster_recovery_protocol_624yz0",
        "chi_high_availability_cluster_951abc",
        "psi_continuous_integration_pipeline_278def",
        "omega_feature_flag_management_605ghi",
    ];

    let string_array2 = vec![
        "red_velvet_cupcake_with_cream_cheese_frosting",
        "orange_marmalade_with_sourdough_toast",
        "yellow_sponge_cake_with_lemon_buttercream",
        "green_matcha_ice_cream_with_red_bean_paste",
        "blue_butterfly_pea_flower_tea_with_honey",
        "indigo_berry_sorbet_with_mint_garnish",
        "violet_lavender_macarons_with_white_chocolate",
        "black_sesame_pudding_with_kinako_powder",
        "white_chocolate_mousse_with_raspberry_coulis",
        "gray_earl_tea_infused_panna_cotta",
    ];

    // Array of longer texts (paragraphs of content)
    let long_text_array = vec![
        "The importance of efficient database indexing cannot be overstated in modern application development. When dealing with large datasets, the difference between milliseconds and seconds in query response time significantly impacts user experience. Fast field execution in ParadeDB leverages memory-mapped structures to reduce disk I/O and accelerate data retrieval operations. By maintaining field values in an optimized format directly accessible by the query engine, we bypass the overhead associated with traditional heap fetches and tuple reconstruction. This benchmark aims to quantify these advantages across various query patterns and data types.",
        "Cloud native architectures require databases that can scale horizontally while maintaining consistent performance characteristics. Container orchestration platforms like Kubernetes have revolutionized deployment strategies, but database systems often remain a bottleneck. ParadeDB's approach combines PostgreSQL's reliability with innovative indexing techniques specifically designed for contemporary workloads. By embedding Tantivy's search capabilities and enhancing them with custom execution paths, we achieve both flexibility and performance. The benchmarks in this test suite demonstrate real-world scenarios where these optimizations provide measurable benefits.",
        "Text search performance has traditionally involved tradeoffs between accuracy and speed. Conventional database systems either provide basic pattern matching or rely on external search engines, introducing complexity and synchronization challenges. The BM25 index implementation in ParadeDB addresses these limitations by tightly integrating full-text search capabilities within the PostgreSQL ecosystem. Fast fields extend this concept to non-text data types, providing uniform performance optimizations across heterogeneous data. This unified approach simplifies application development while delivering performance improvements that particularly benefit complex analytical queries.",
        "Data analytics workloads typically involve scanning large volumes of records, applying filters, and performing aggregations. Traditional database execution plans often struggle with these patterns, especially when they include text fields alongside numeric data. The mixed fast field execution path specifically targets these scenarios by optimizing the scanning phase with memory-efficient representations of both text and numeric values. By eliminating the need to reconstruct complete tuples from the heap for filtering operations, we reduce both CPU and I/O overhead. The benchmarks in this suite quantify these benefits across representative query patterns.",
        "Security and compliance requirements often necessitate text analysis on sensitive data fields. Encryption status, access logs, and audit trails frequently combine textual descriptions with numeric identifiers and timestamps. Efficiently querying these mixed data types presents unique challenges for database systems. ParadeDB's specialized execution paths address these use cases by maintaining both text and numeric fields in optimized in-memory structures, enabling rapid filtering and aggregation. This approach is particularly valuable for security information and event management (SIEM) systems where response time directly impacts threat detection and mitigation capabilities.",
        "Time series data analysis combines the challenges of high ingest rates with complex query patterns. Device telemetry, system metrics, and application logs typically contain both structured numeric data and semi-structured text fields. Analyzing this information efficiently requires specialized indexing strategies. The fast field execution paths in ParadeDB provide optimized access patterns for both numeric time series data and associated text annotations. This benchmark suite includes representative queries that demonstrate the performance characteristics of these optimization techniques across varying data volumes and query complexities.",
        "Machine learning operations increasingly depend on efficient data retrieval for both training and inference phases. Feature stores must handle diverse data types while providing consistent low-latency access patterns. The combination of text features (like user agent strings, product descriptions, or error messages) with numeric features (counts, measurements, or derived metrics) presents particular challenges for traditional database systems. ParadeDB's mixed fast field execution optimizes these access patterns, reducing the time required for feature extraction and transformation. The benchmarks in this suite model common ML feature access patterns to quantify these performance benefits."
    ];

    // JSON data templates (complex nested structures)
    let json_templates = vec![
        r#"{"user":{"id":%ID%,"username":"%USERNAME%","profile":{"age":%AGE%,"interests":["%INTEREST1%","%INTEREST2%"],"location":{"city":"%CITY%","country":"USA","coordinates":{"lat":40.7128,"lng":-74.0060}}}},"metadata":{"last_login":"2023-10-%DAY%","device":"mobile","settings":{"notifications":true,"theme":"dark"}}}"#,
        r#"{"product":{"id":%ID%,"name":"%NAME%","details":{"price":%PRICE%.99,"category":"%CATEGORY%","tags":["%TAG1%","%TAG2%","%TAG3%"],"stock":{"warehouse_a":%STOCK_A%,"warehouse_b":%STOCK_B%}},"ratings":[%RATING1%,%RATING2%,%RATING3%,%RATING4%]},"audit":{"created":"2023-%MONTH1%-15","modified":"2023-%MONTH2%-20"}}"#,
        r#"{"transaction":{"id":"tx-%ID%","amount":%AMOUNT%.%CENTS%,"currency":"USD","status":"%STATUS%","items":[{"product_id":%PROD_ID1%,"quantity":%QTY1%,"price":%PRICE1%.99},{"product_id":%PROD_ID2%,"quantity":%QTY2%,"price":%PRICE2%.49}],"customer":{"id":"cust-%CUST_ID%","segment":"%SEGMENT%"}},"processing":{"timestamp":"2023-%MONTH%-%DAY%T10:30:00Z","gateway":"%GATEWAY%","attempt":%ATTEMPT%}}"#,
        r#"{"event":{"id":"evt-%ID%","type":"%TYPE%","source":"%SOURCE%","severity":%SEVERITY%,"timestamp":"2023-%MONTH%-%DAY%T%HOUR%:%MINUTE%:00Z","details":{"message":"%MESSAGE% occurred on %SOURCE%","affected_components":["%COMP1%","%COMP2%"],"metrics":{"duration_ms":%DURATION%,"resource_usage":%RESOURCE%.%RESOURCE_DEC%}}},"context":{"environment":"%ENV%","region":"us-west-%REGION%","trace_id":"trace-%TRACE%"}}"#,
    ];

    // City names for JSON data
    let cities = vec![
        "New York",
        "Los Angeles",
        "Chicago",
        "Houston",
        "Phoenix",
        "Philadelphia",
        "San Antonio",
        "San Diego",
        "Dallas",
        "San Jose",
    ];

    // Categories for JSON data
    let categories = vec![
        "Electronics",
        "Clothing",
        "Home & Garden",
        "Sports",
        "Books",
        "Automotive",
        "Health",
        "Beauty",
        "Toys",
        "Groceries",
    ];

    // Tags for JSON data
    let tags = vec![
        "bestseller",
        "new",
        "sale",
        "limited",
        "exclusive",
        "organic",
        "handmade",
        "imported",
        "local",
        "sustainable",
    ];

    // Customer segments
    let segments = vec!["premium", "standard", "business", "enterprise", "partner"];

    // Payment gateways
    let gateways = vec!["Stripe", "PayPal", "Square", "Braintree", "Adyen"];

    // Event types
    let event_types = vec![
        "system_error",
        "user_action",
        "api_request",
        "database_operation",
        "security_alert",
    ];

    // Event sources
    let event_sources = vec![
        "web_frontend",
        "mobile_app",
        "background_job",
        "scheduled_task",
        "external_api",
    ];

    // Environments
    let environments = vec!["production", "staging", "development", "testing", "qa"];

    // Status values
    let statuses = vec!["completed", "pending", "failed", "processing", "refunded"];

    let mut inserted = 0;

    while inserted < rows_to_add {
        // Create a batch insert statement
        let mut batch_query = String::from(
            "INSERT INTO benchmark_data (string_field1, string_field2, long_text, json_data, numeric_field1, numeric_field2, numeric_field3) VALUES "
        );

        let batch_end = (inserted + BATCH_SIZE).min(rows_to_add);
        for i in (current_rows + inserted)..(current_rows + batch_end) {
            if i > current_rows + inserted {
                batch_query.push_str(", ");
            }

            let string1 = string_array1[i % string_array1.len()];
            let string2 = string_array2[i % string_array2.len()];
            let long_text = long_text_array[i % long_text_array.len()];

            // Generate complex JSON data using string replacement instead of format!
            let json_data = match i % 4 {
                0 => {
                    let template = json_templates[0];
                    template
                        .replace("%ID%", &i.to_string())
                        .replace("%USERNAME%", string1)
                        .replace("%AGE%", &((i % 50) + 20).to_string())
                        .replace("%INTEREST1%", &tags[i % tags.len()])
                        .replace("%INTEREST2%", &tags[(i + 3) % tags.len()])
                        .replace("%CITY%", &cities[i % cities.len()])
                        .replace("%DAY%", &((i % 28) + 1).to_string())
                }
                1 => {
                    let template = json_templates[1];
                    template
                        .replace("%ID%", &i.to_string())
                        .replace("%NAME%", &format!("{} {}", string1, string2))
                        .replace("%PRICE%", &((i % 100) + 10).to_string())
                        .replace("%CATEGORY%", &categories[i % categories.len()])
                        .replace("%TAG1%", &tags[i % tags.len()])
                        .replace("%TAG2%", &tags[(i + 2) % tags.len()])
                        .replace("%TAG3%", &tags[(i + 4) % tags.len()])
                        .replace("%STOCK_A%", &((i % 1000) + 100).to_string())
                        .replace("%STOCK_B%", &((i % 500) + 50).to_string())
                        .replace("%RATING1%", &((i % 5) + 1).to_string())
                        .replace("%RATING2%", &((i % 5) + 1).to_string())
                        .replace("%RATING3%", &((i % 5) + 1).to_string())
                        .replace("%RATING4%", &((i % 5) + 1).to_string())
                        .replace("%MONTH1%", &((i % 12) + 1).to_string())
                        .replace("%MONTH2%", &((i % 12) + 1).to_string())
                }
                2 => {
                    let template = json_templates[2];
                    template
                        .replace("%ID%", &i.to_string())
                        .replace("%AMOUNT%", &((i % 1000) + 10).to_string())
                        .replace("%CENTS%", &(i % 100).to_string())
                        .replace("%STATUS%", &statuses[i % statuses.len()])
                        .replace("%PROD_ID1%", &(i % 1000).to_string())
                        .replace("%QTY1%", &((i % 10) + 1).to_string())
                        .replace("%PRICE1%", &((i % 100) + 10).to_string())
                        .replace("%PROD_ID2%", &((i + 1) % 1000).to_string())
                        .replace("%QTY2%", &((i % 5) + 1).to_string())
                        .replace("%PRICE2%", &((i % 50) + 5).to_string())
                        .replace("%CUST_ID%", &(i % 10000).to_string())
                        .replace("%SEGMENT%", &segments[i % segments.len()])
                        .replace("%MONTH%", &((i % 12) + 1).to_string())
                        .replace("%DAY%", &((i % 28) + 1).to_string())
                        .replace("%GATEWAY%", &gateways[i % gateways.len()])
                        .replace("%ATTEMPT%", &((i % 3) + 1).to_string())
                }
                _ => {
                    let template = json_templates[3];
                    let event_type = event_types[i % event_types.len()];
                    let event_source = event_sources[i % event_sources.len()];

                    template
                        .replace("%ID%", &i.to_string())
                        .replace("%TYPE%", event_type)
                        .replace("%SOURCE%", event_source)
                        .replace("%SEVERITY%", &((i % 5) + 1).to_string())
                        .replace("%MONTH%", &((i % 12) + 1).to_string())
                        .replace("%DAY%", &((i % 28) + 1).to_string())
                        .replace("%HOUR%", &(i % 24).to_string())
                        .replace("%MINUTE%", &(i % 60).to_string())
                        .replace("%MESSAGE%", event_type)
                        .replace("%COMP1%", &format!("service-{}", i))
                        .replace("%COMP2%", &format!("component-{}", i % 10))
                        .replace("%DURATION%", &((i % 1000) + 100).to_string())
                        .replace("%RESOURCE%", &(i % 100).to_string())
                        .replace("%RESOURCE_DEC%", &(i % 10).to_string())
                        .replace("%ENV%", &environments[i % environments.len()])
                        .replace("%REGION%", &((i % 3) + 1).to_string())
                        .replace("%TRACE%", &(i % 100000).to_string())
                }
            };

            let num1 = (i % 1000) as i32;
            let num2 = (i % 100) as f32;
            let num3 = (i % 10000) as i32;

            // Escape single quotes in JSON and text fields
            let escaped_json = json_data.replace('\'', "''");
            let escaped_long_text = long_text.replace('\'', "''");

            // Add values to batch query
            batch_query.push_str(&format!(
                "('{}', '{}', '{}', '{}', {}, {}, {})",
                string1, string2, escaped_long_text, escaped_json, num1, num2, num3
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
            long_text,
            json_data,
            numeric_field1,
            numeric_field2,
            numeric_field3
        ) WITH (
            key_field = 'id',
            text_fields = '{
                \"string_field1\": {\"fast\": true, \"tokenizer\": {\"type\": \"keyword\"}}, 
                \"string_field2\": {\"fast\": true, \"tokenizer\": {\"type\": \"keyword\"}},
                \"long_text\": {\"fast\": true, \"tokenizer\": {\"type\": \"default\"}},
                \"json_data\": {\"fast\": true, \"tokenizer\": {\"type\": \"default\"}}
            }',
            numeric_fields = '{
                \"numeric_field1\": {\"fast\": true}, 
                \"numeric_field2\": {\"fast\": true}, 
                \"numeric_field3\": {\"fast\": true}
            }'
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

    Ok(())
}

async fn set_execution_method(conn: &mut PgConnection, execution_method: &str) -> Result<()> {
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

    // Reset/clear cache to ensure clean runs
    sqlx::query("SELECT pg_stat_reset()")
        .execute(&mut *conn)
        .await?;

    // Run a full VACUUM ANALYZE to ensure statistics are up-to-date
    // This helps the query planner make better decisions
    println!("Running VACUUM ANALYZE on benchmark_data...");
    sqlx::query("VACUUM ANALYZE benchmark_data")
        .execute(&mut *conn)
        .await?;

    Ok(())
}

/// Run a benchmark for a specific query with the specified execution method (mixed or normal)
async fn run_benchmark(
    conn: &mut PgConnection,
    query: &str,
    test_name: &str,
    execution_method: &str,
) -> Result<BenchmarkResult> {
    let mut total_time_ms: f64 = 0.0;
    let mut min_time_ms: f64 = f64::MAX;
    let mut max_time_ms: f64 = 0.0;

    set_execution_method(conn, &execution_method).await?;

    // The query to run, with no modification
    let query_to_run = query.to_string();

    // Warmup runs to ensure caches are primed
    for _ in 0..WARMUP_ITERATIONS {
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
    for _i in 0..ITERATIONS {
        let start = Instant::now();
        let _res = sqlx::query(&query_to_run).fetch_all(&mut *conn).await?;
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
            let ratio = mixed.avg_time_ms / normal.avg_time_ms;
            let performance = if mixed.avg_time_ms > normal.avg_time_ms {
                "SLOWER"
            } else {
                "FASTER"
            };

            println!(
                "{:<45} {:<15.2} {:<15.2} {:<15.2} {:<15}",
                base_name, mixed.avg_time_ms, normal.avg_time_ms, ratio, performance
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

        let result = run_benchmark(conn, query, &full_benchmark_name, method_name).await?;

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
    // Updated to use the new fields
    let basic_query =
        "SELECT id, string_field1, string_field2, json_data, numeric_field1, numeric_field2, numeric_field3 
         FROM benchmark_data 
         WHERE numeric_field1 < 500 AND string_field1 @@@ '\"alpha_complex_identifier_123456789\"' AND string_field2 @@@ '\"red_velvet_cupcake_with_cream_cheese_frosting\"'
         ORDER BY id";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        &mut conn,
        basic_query,
        "Basic - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
    )
    .await?;

    // Test 2: Count query with long text
    let count_query = "SELECT numeric_field1, string_field1
                      FROM benchmark_data 
                      WHERE long_text @@@ '\"database\"' AND numeric_field1 < 500";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        &mut conn,
        count_query,
        "Count Query - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
    )
    .await?;

    // Test 3: Complex Aggregation Query with more complex fields
    let complex_query = "
        WITH filtered_data AS (
            SELECT 
                string_field1, 
                string_field2, 
                long_text,
                json_data,
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
        &mut conn,
        complex_query,
        "Complex Aggregation - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
    )
    .await?;

    // Test 4: JSON query - should be much faster with fast fields
    let json_query = "
        SELECT 
            json_data
        FROM benchmark_data 
        WHERE 
            json_data @@@ '\"Sports\"'
        ORDER BY json_data";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        &mut conn,
        json_query,
        "JSON Query - StringFF",
        &["StringFastFieldExec", "NormalScanExecState"],
        &mut results,
    )
    .await?;

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        &mut conn,
        json_query,
        "JSON Query - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
    )
    .await?;

    // Test 5: Long text search - should show big difference with fast fields
    let long_text_query = "
        SELECT 
            long_text
        FROM benchmark_data 
        WHERE 
            long_text @@@ '\"database\" AND \"performance\"'
        ORDER BY long_text";

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        &mut conn,
        long_text_query,
        "Long Text Query - StringFF",
        &["StringFastFieldExec", "NormalScanExecState"],
        &mut results,
    )
    .await?;

    // Run the benchmarks with different execution methods
    run_benchmarks_with_methods(
        &mut conn,
        long_text_query,
        "Long Text Query - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
        &mut results,
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
        &mut conn,
        ordering_query,
        "Heavy Ordering - MixedFF",
        &["MixedFastFieldExec", "NormalScanExecState"],
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
        } else if ratio < 0.7 {
            println!(
                "\n🚀 MixedFastFieldExec is significantly faster than NormalScanExecState! ({:.2}x)",
                1.0 / ratio
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
        "SELECT id, string_field1, string_field2, long_text, json_data, numeric_field1, numeric_field2, numeric_field3 
         FROM benchmark_data 
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

    set_execution_method(&mut conn, "MixedFastFieldExec").await?;

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

    set_execution_method(&mut conn, "NormalScanExecState").await?;

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
