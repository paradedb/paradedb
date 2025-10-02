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
use sqlx::PgConnection;
use sqlx::Row;

/// Setup function to create test table and data
pub async fn setup_benchmark_database(
    conn: &mut PgConnection,
    num_rows: usize,
    table_name: &str,
    batch_size: usize,
) -> Result<()> {
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_search;")
        .execute(&mut *conn)
        .await?;

    // First check if the table exists
    let table_exists_query =
        &format!("SELECT EXISTS (SELECT FROM pg_tables WHERE tablename = '{table_name}')");
    let table_exists: bool = sqlx::query(table_exists_query)
        .fetch_one(&mut *conn)
        .await?
        .get(0);

    println!("Table exists check: {table_exists}");

    let mut current_rows = if table_exists {
        // Count existing rows more reliably with explicit casting to bigint
        let count_result = sqlx::query(&format!("SELECT COUNT(*)::bigint FROM {table_name}"))
            .fetch_one(&mut *conn)
            .await;

        match count_result {
            Ok(row) => {
                let count: i64 = row.get(0);
                println!("Found existing table with {count} rows");
                count as usize
            }
            Err(e) => {
                println!("Error counting rows: {e}, assuming table needs recreation");
                // If we can't count rows, the table might be corrupted
                sqlx::query(&format!("DROP TABLE IF EXISTS {table_name} CASCADE"))
                    .execute(&mut *conn)
                    .await?;

                // Create the table
                create_benchmark_table(conn, table_name).await?;
                0
            }
        }
    } else {
        println!("Table doesn't exist, creating new one");
        // Create the table if it doesn't exist
        create_benchmark_table(conn, table_name).await?;
        0
    };

    println!("Table {table_name} already exists with {current_rows} rows (requested: {num_rows})");

    if current_rows == num_rows {
        // Run a full VACUUM ANALYZE to update statistics after index creation
        println!("Running VACUUM ANALYZE after index creation...");
        sqlx::query(&format!("VACUUM ANALYZE {table_name}"))
            .execute(&mut *conn)
            .await?;
        return Ok(());
    }

    if current_rows > num_rows {
        // Drop table if exists
        println!("Table has more rows than needed ({current_rows}), recreating...");
        sqlx::query(&format!("DROP TABLE IF EXISTS {table_name} CASCADE"))
            .execute(&mut *conn)
            .await?;

        // Create the table
        create_benchmark_table(conn, table_name).await?;
        current_rows = 0;
    }

    let rows_to_add = num_rows - current_rows;
    println!("Adding {rows_to_add} more rows to {table_name}");

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

    let string_array2 = [
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
    let long_text_array = ["The importance of efficient database indexing cannot be overstated in modern application development. When dealing with large datasets, the difference between milliseconds and seconds in query response time significantly impacts user experience. Fast field execution in ParadeDB leverages memory-mapped structures to reduce disk I/O and accelerate data retrieval operations. By maintaining field values in an optimized format directly accessible by the query engine, we bypass the overhead associated with traditional heap fetches and tuple reconstruction. This benchmark aims to quantify these advantages across various query patterns and data types.",
        "Cloud native architectures require databases that can scale horizontally while maintaining consistent performance characteristics. Container orchestration platforms like Kubernetes have revolutionized deployment strategies, but database systems often remain a bottleneck. ParadeDB's approach combines PostgreSQL's reliability with innovative indexing techniques specifically designed for contemporary workloads. By embedding Tantivy's search capabilities and enhancing them with custom execution paths, we achieve both flexibility and performance. The benchmarks in this test suite demonstrate real-world scenarios where these optimizations provide measurable benefits.",
        "Text search performance has traditionally involved tradeoffs between accuracy and speed. Conventional database systems either provide basic pattern matching or rely on external search engines, introducing complexity and synchronization challenges. The BM25 index implementation in ParadeDB addresses these limitations by tightly integrating full-text search capabilities within the PostgreSQL ecosystem. Fast fields extend this concept to non-text data types, providing uniform performance optimizations across heterogeneous data. This unified approach simplifies application development while delivering performance improvements that particularly benefit complex analytical queries.",
        "Data analytics workloads typically involve scanning large volumes of records, applying filters, and performing aggregations. Traditional database execution plans often struggle with these patterns, especially when they include text fields alongside numeric data. The mixed fast field execution path specifically targets these scenarios by optimizing the scanning phase with memory-efficient representations of both text and numeric values. By eliminating the need to reconstruct complete tuples from the heap for filtering operations, we reduce both CPU and I/O overhead. The benchmarks in this suite quantify these benefits across representative query patterns.",
        "Security and compliance requirements often necessitate text analysis on sensitive data fields. Encryption status, access logs, and audit trails frequently combine textual descriptions with numeric identifiers and timestamps. Efficiently querying these mixed data types presents unique challenges for database systems. ParadeDB's specialized execution paths address these use cases by maintaining both text and numeric fields in optimized in-memory structures, enabling rapid filtering and aggregation. This approach is particularly valuable for security information and event management (SIEM) systems where response time directly impacts threat detection and mitigation capabilities.",
        "Time series data analysis combines the challenges of high ingest rates with complex query patterns. Device telemetry, system metrics, and application logs typically contain both structured numeric data and semi-structured text fields. Analyzing this information efficiently requires specialized indexing strategies. The fast field execution paths in ParadeDB provide optimized access patterns for both numeric time series data and associated text annotations. This benchmark suite includes representative queries that demonstrate the performance characteristics of these optimization techniques across varying data volumes and query complexities.",
        "Machine learning operations increasingly depend on efficient data retrieval for both training and inference phases. Feature stores must handle diverse data types while providing consistent low-latency access patterns. The combination of text features (like user agent strings, product descriptions, or error messages) with numeric features (counts, measurements, or derived metrics) presents particular challenges for traditional database systems. ParadeDB's mixed fast field execution optimizes these access patterns, reducing the time required for feature extraction and transformation. The benchmarks in this suite model common ML feature access patterns to quantify these performance benefits."];

    // JSON data templates (complex nested structures)
    let json_templates = [
        r#"{"user":{"id":%ID%,"username":"%USERNAME%","profile":{"age":%AGE%,"interests":["%INTEREST1%","%INTEREST2%"],"location":{"city":"%CITY%","country":"USA","coordinates":{"lat":40.7128,"lng":-74.0060}}}},"metadata":{"last_login":"2023-10-%DAY%","device":"mobile","settings":{"notifications":true,"theme":"dark"}}}"#,
        r#"{"product":{"id":%ID%,"name":"%NAME%","details":{"price":%PRICE%.99,"category":"%CATEGORY%","tags":["%TAG1%","%TAG2%","%TAG3%"],"stock":{"warehouse_a":%STOCK_A%,"warehouse_b":%STOCK_B%}},"ratings":[%RATING1%,%RATING2%,%RATING3%,%RATING4%]},"audit":{"created":"2023-%MONTH1%-15","modified":"2023-%MONTH2%-20"}}"#,
        r#"{"transaction":{"id":"tx-%ID%","amount":%AMOUNT%.%CENTS%,"currency":"USD","status":"%STATUS%","items":[{"product_id":%PROD_ID1%,"quantity":%QTY1%,"price":%PRICE1%.99},{"product_id":%PROD_ID2%,"quantity":%QTY2%,"price":%PRICE2%.49}],"customer":{"id":"cust-%CUST_ID%","segment":"%SEGMENT%"}},"processing":{"timestamp":"2023-%MONTH%-%DAY%T10:30:00Z","gateway":"%GATEWAY%","attempt":%ATTEMPT%}}"#,
        r#"{"event":{"id":"evt-%ID%","type":"%TYPE%","source":"%SOURCE%","severity":%SEVERITY%,"timestamp":"2023-%MONTH%-%DAY%T%HOUR%:%MINUTE%:00Z","details":{"message":"%MESSAGE% occurred on %SOURCE%","affected_components":["%COMP1%","%COMP2%"],"metrics":{"duration_ms":%DURATION%,"resource_usage":%RESOURCE%.%RESOURCE_DEC%}}},"context":{"environment":"%ENV%","region":"us-west-%REGION%","trace_id":"trace-%TRACE%"}}"#,
    ];

    // City names for JSON data
    let cities = [
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
    let categories = [
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
    let tags = [
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
    let segments = ["premium", "standard", "business", "enterprise", "partner"];

    // Payment gateways
    let gateways = ["Stripe", "PayPal", "Square", "Braintree", "Adyen"];

    // Event types
    let event_types = [
        "system_error",
        "user_action",
        "api_request",
        "database_operation",
        "security_alert",
    ];

    // Event sources
    let event_sources = [
        "web_frontend",
        "mobile_app",
        "background_job",
        "scheduled_task",
        "external_api",
    ];

    // Environments
    let environments = ["production", "staging", "development", "testing", "qa"];

    // Status values
    let statuses = ["completed", "pending", "failed", "processing", "refunded"];

    let mut inserted = 0;

    while inserted < rows_to_add {
        // Create a batch insert statement
        let mut batch_query = String::from(
            &format!("INSERT INTO {table_name} (string_field1, string_field2, long_text, json_data, numeric_field1, numeric_field2, numeric_field3) VALUES "),
        );

        let batch_end = (inserted + batch_size).min(rows_to_add);
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
                        .replace("%INTEREST1%", tags[i % tags.len()])
                        .replace("%INTEREST2%", tags[(i + 3) % tags.len()])
                        .replace("%CITY%", cities[i % cities.len()])
                        .replace("%DAY%", &((i % 28) + 1).to_string())
                }
                1 => {
                    let template = json_templates[1];
                    template
                        .replace("%ID%", &i.to_string())
                        .replace("%NAME%", &format!("{string1} {string2}"))
                        .replace("%PRICE%", &((i % 100) + 10).to_string())
                        .replace("%CATEGORY%", categories[i % categories.len()])
                        .replace("%TAG1%", tags[i % tags.len()])
                        .replace("%TAG2%", tags[(i + 2) % tags.len()])
                        .replace("%TAG3%", tags[(i + 4) % tags.len()])
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
                        .replace("%STATUS%", statuses[i % statuses.len()])
                        .replace("%PROD_ID1%", &(i % 1000).to_string())
                        .replace("%QTY1%", &((i % 10) + 1).to_string())
                        .replace("%PRICE1%", &((i % 100) + 10).to_string())
                        .replace("%PROD_ID2%", &((i + 1) % 1000).to_string())
                        .replace("%QTY2%", &((i % 5) + 1).to_string())
                        .replace("%PRICE2%", &((i % 50) + 5).to_string())
                        .replace("%CUST_ID%", &(i % 10000).to_string())
                        .replace("%SEGMENT%", segments[i % segments.len()])
                        .replace("%MONTH%", &((i % 12) + 1).to_string())
                        .replace("%DAY%", &((i % 28) + 1).to_string())
                        .replace("%GATEWAY%", gateways[i % gateways.len()])
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
                        .replace("%COMP1%", &format!("service-{i}"))
                        .replace("%COMP2%", &format!("component-{}", i % 10))
                        .replace("%DURATION%", &((i % 1000) + 100).to_string())
                        .replace("%RESOURCE%", &(i % 100).to_string())
                        .replace("%RESOURCE_DEC%", &(i % 10).to_string())
                        .replace("%ENV%", environments[i % environments.len()])
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
                "('{string1}', '{string2}', '{escaped_long_text}', '{escaped_json}', {num1}, {num2}, {num3})"
            ));
        }

        // Execute batch insert
        sqlx::query(&batch_query).execute(&mut *conn).await?;

        inserted += batch_end - (current_rows + inserted);

        if inserted % 10000 == 0 && inserted > 0 {
            println!("Inserted {inserted} of {rows_to_add} rows...");
        }
    }

    println!(
        "Database setup complete with {num_rows} rows (was: {current_rows}, added: {rows_to_add})"
    );

    // Verify the actual row count
    let final_count: i64 = sqlx::query(&format!("SELECT COUNT(*)::bigint FROM {table_name}"))
        .fetch_one(&mut *conn)
        .await?
        .get(0);

    println!("Final table verification: contains {final_count} rows");

    // Ensure count matches what we expect
    assert_eq!(
        final_count as usize, num_rows,
        "Table contains {final_count} rows but should have {num_rows} rows"
    );

    create_bm25_index(conn, table_name).await?;

    // Run a full VACUUM ANALYZE to update statistics
    println!("Running VACUUM ANALYZE after index creation...");
    sqlx::query(&format!("VACUUM ANALYZE {table_name}"))
        .execute(&mut *conn)
        .await?;

    Ok(())
}

async fn create_benchmark_table(conn: &mut PgConnection, table_name: &str) -> Result<()> {
    // Create the table
    let create_table_query = format!(
        "CREATE TABLE {table_name} (
        id SERIAL PRIMARY KEY,
        string_field1 TEXT NOT NULL,
        string_field2 TEXT NOT NULL,
        long_text TEXT NOT NULL,
        json_data TEXT NOT NULL,
        numeric_field1 INTEGER NOT NULL,
        numeric_field2 FLOAT NOT NULL,
        numeric_field3 NUMERIC(10,2) NOT NULL
    )"
    );

    sqlx::query(&create_table_query).execute(&mut *conn).await?;

    Ok(())
}

/// Creates an index with the specified configuration
pub async fn create_bm25_index(conn: &mut PgConnection, table_name: &str) -> Result<()> {
    // First drop any existing index
    let index_name = format!("{table_name}_idx");
    sqlx::query(&format!("DROP INDEX IF EXISTS {index_name} CASCADE"))
        .execute(&mut *conn)
        .await?;

    // Define configuration based on the desired execution method
    // All fields are marked as fast for MixedFastFieldExec
    // IMPORTANT: ALL fields, including ID and those used in SELECT must be fast
    // Use keyword tokenizer for string fields to ensure exact matching
    let index_definition = format!(
        "CREATE INDEX {index_name} ON {table_name} 
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
            text_fields = '{{
                \"string_field1\": {{\"fast\": true, \"tokenizer\": {{\"type\": \"keyword\"}}}}, 
                \"string_field2\": {{\"fast\": true, \"tokenizer\": {{\"type\": \"keyword\"}}}},
                \"long_text\": {{\"tokenizer\": {{\"type\": \"default\"}}}},
                \"json_data\": {{\"tokenizer\": {{\"type\": \"default\"}}}}
            }}',
            numeric_fields = '{{
                \"numeric_field1\": {{\"fast\": true}}, 
                \"numeric_field2\": {{\"fast\": true}}, 
                \"numeric_field3\": {{\"fast\": true}}
            }}'
        )"
    );

    // Create the index
    println!("Creating index {index_name}...");
    sqlx::query(&index_definition).execute(&mut *conn).await?;

    // Wait a moment for the index to be fully ready
    sqlx::query("SELECT pg_sleep(0.5)")
        .execute(&mut *conn)
        .await?;

    // Verify the index was created
    let verify_index = sqlx::query(&format!(
        "SELECT indexname FROM pg_indexes WHERE indexname = '{index_name}'"
    ))
    .fetch_optional(&mut *conn)
    .await?;

    if verify_index.is_some() {
        println!("Index '{index_name}' created successfully");
    } else {
        println!("WARNING: Index '{index_name}' not found!");
    }

    Ok(())
}
