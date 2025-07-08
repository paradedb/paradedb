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
use serde_json::Value;
use sqlx::{PgConnection, Row};
use std::time::Instant;

/// JOIN Performance Benchmark Results
#[derive(Debug, Clone)]
pub struct JoinPerfResult {
    pub query_name: String,
    pub explain_analyze_time_ms: f64,
    pub actual_execution_time_ms: f64,
    pub timing_discrepancy_ratio: f64,
    pub heap_fetches: i64,
    pub exec_method: String,
    pub row_count: i64,
}

/// Runs a query with EXPLAIN ANALYZE and extracts timing and execution details
pub async fn run_with_explain_analyze(
    conn: &mut PgConnection,
    query: &str,
    query_name: &str,
) -> Result<JoinPerfResult> {
    // Run with EXPLAIN ANALYZE (JSON format for parsing)
    let explain_query = format!("EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON) {query}");
    let start = Instant::now();
    let (plan,): (Value,) = sqlx::query_as(&explain_query).fetch_one(&mut *conn).await?;
    let explain_analyze_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Also run EXPLAIN ANALYZE in text format for display
    print_query_plan(conn, query, query_name).await?;

    // Extract execution time from plan
    let plan_execution_time = extract_execution_time(&plan);
    let heap_fetches = extract_heap_fetches(&plan);
    let exec_method = extract_exec_method(&plan);

    // Run actual query to get row count and actual timing
    let start = Instant::now();
    let rows = sqlx::query(query).fetch_all(&mut *conn).await?;
    let actual_execution_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let row_count = rows.len() as i64;

    // Calculate discrepancy ratio
    let timing_discrepancy_ratio = if plan_execution_time > 0.0 {
        actual_execution_time_ms / plan_execution_time
    } else {
        actual_execution_time_ms / explain_analyze_time_ms
    };

    Ok(JoinPerfResult {
        query_name: query_name.to_string(),
        explain_analyze_time_ms,
        actual_execution_time_ms,
        timing_discrepancy_ratio,
        heap_fetches,
        exec_method,
        row_count,
    })
}

/// Print the query plan in readable text format
pub async fn print_query_plan(
    conn: &mut PgConnection,
    query: &str,
    query_name: &str,
) -> Result<()> {
    println!("\n📋 Query Plan for: {query_name}");
    println!("{}", "=".repeat(80));

    let explain_query = format!("EXPLAIN (ANALYZE, VERBOSE, BUFFERS, FORMAT TEXT) {query}");
    let rows = sqlx::query(&explain_query).fetch_all(&mut *conn).await?;

    for row in rows {
        let plan_line: String = row.get(0);
        println!("{plan_line}");
    }

    println!("{}", "=".repeat(80));
    Ok(())
}

/// Extract execution time from EXPLAIN ANALYZE JSON output
fn extract_execution_time(plan: &Value) -> f64 {
    if let Some(execution_time) = plan.get(0).and_then(|p| p.get("Execution Time")) {
        execution_time.as_f64().unwrap_or(0.0)
    } else {
        0.0
    }
}

/// Extract heap fetches from EXPLAIN ANALYZE JSON output
fn extract_heap_fetches(plan: &Value) -> i64 {
    fn find_heap_fetches(value: &Value) -> i64 {
        match value {
            Value::Object(map) => {
                if let Some(heap_fetches) = map.get("Heap Fetches") {
                    return heap_fetches.as_i64().unwrap_or(0);
                }
                for (_, v) in map {
                    let result = find_heap_fetches(v);
                    if result > 0 {
                        return result;
                    }
                }
                0
            }
            Value::Array(arr) => {
                for item in arr {
                    let result = find_heap_fetches(item);
                    if result > 0 {
                        return result;
                    }
                }
                0
            }
            _ => 0,
        }
    }
    find_heap_fetches(plan)
}

/// Extract execution method from EXPLAIN ANALYZE JSON output
fn extract_exec_method(plan: &Value) -> String {
    let plan_str = plan.to_string();
    if plan_str.contains("MixedFastFieldExecState") {
        "MixedFastFieldExec".to_string()
    } else if plan_str.contains("StringFastFieldExecState") {
        "StringFastFieldExec".to_string()
    } else if plan_str.contains("NumericFastFieldExecState") {
        "NumericFastFieldExec".to_string()
    } else if plan_str.contains("NormalScanExecState") {
        "NormalScanExecState".to_string()
    } else if plan_str.contains("ParadeDB Scan") {
        "CustomScan".to_string()
    } else {
        "PostgreSQL".to_string()
    }
}

/// Run JOIN performance benchmark
pub async fn benchmark_join_perf(conn: &mut PgConnection) -> Result<()> {
    // Create pg_search extension if it doesn't exist
    println!("Creating pg_search extension...");
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_search;")
        .execute(&mut *conn)
        .await?;

    println!("========================================");
    println!("JOIN Performance Benchmark");
    println!("========================================");

    // Ensure proper settings
    sqlx::query("SET enable_seqscan = off")
        .execute(&mut *conn)
        .await?;
    sqlx::query("SET enable_bitmapscan = off")
        .execute(&mut *conn)
        .await?;
    sqlx::query("SET max_parallel_workers_per_gather = 8")
        .execute(&mut *conn)
        .await?;

    let mut results = Vec::new();

    // Test queries from customer_reproduction.sql - PARADEDB VERSION
    let paradedb_queries = vec![
        (
            "Basic COUNT - Denormalized Pattern (ParadeDB)",
            "SELECT count(*) FROM pages WHERE pages.content @@@ paradedb.parse('parents:\"SFR\"')",
        ),
        (
            "Basic JOIN COUNT - Original Pattern (ParadeDB)",
            "SELECT count(*) FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON files.id = pages.\"fileId\" WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.content @@@ 'Single Number Reach'",
        ),
        (
            "CTE Approach - Customer Tried (ParadeDB)",
            "WITH filtered_documents AS (SELECT DISTINCT id FROM documents WHERE parents @@@ 'SFR'), filtered_files AS (SELECT DISTINCT id, \"documentId\" FROM files WHERE title @@@ 'collab12'), filtered_pages AS (SELECT DISTINCT \"fileId\" FROM pages WHERE content @@@ 'Single Number Reach') SELECT count(*) FROM filtered_documents fd JOIN filtered_files ff ON fd.id = ff.\"documentId\" JOIN filtered_pages fp ON ff.id = fp.\"fileId\"",
        ),
        (
            "EXISTS Subquery Pattern (ParadeDB)",
            "SELECT count(*) FROM documents WHERE documents.parents @@@ 'SFR' AND EXISTS (SELECT 1 FROM files WHERE files.\"documentId\" = documents.id AND files.title @@@ 'collab12')",
        ),
        (
            "Large Aggregation - Timing Discrepancy Test (ParadeDB)",
            "SELECT LEFT(pages.title, 30) as title_prefix, count(*) as cnt FROM pages WHERE pages.content @@@ paradedb.exists('title') GROUP BY LEFT(pages.title, 30) ORDER BY cnt DESC LIMIT 20",
        ),
        (
            "Complex Data Retrieval - Full Results (ParadeDB)",
            "SELECT p.id, p.title, f.title as file_title, d.title as doc_title FROM pages p JOIN files f ON p.\"fileId\" = f.id JOIN documents d ON f.\"documentId\" = d.id WHERE p.content @@@ paradedb.exists('title') AND f.title @@@ paradedb.exists('title') AND d.parents @@@ 'SFR' ORDER BY p.\"createdAt\" DESC LIMIT 1000",
        ),
        (
            "Large Scan with Aggregation - 60s vs 10s Issue (ParadeDB)",
            "SELECT LEFT(p.title, 25) as title_prefix, COUNT(*) as total_pages, COUNT(DISTINCT p.\"fileId\") as unique_files, AVG(p.\"sizeInBytes\") as avg_size FROM pages p WHERE p.content @@@ paradedb.exists('content') GROUP BY LEFT(p.title, 25) HAVING COUNT(*) > 10 ORDER BY total_pages DESC",
        ),
        (
            "Parse Hack Aggregate - Customer Workaround (ParadeDB)",
            "SELECT * FROM paradedb.aggregate('documents_index', paradedb.boolean(must => ARRAY[paradedb.parse((SELECT concat('id:IN [', string_agg(id, ' '), ']') FROM (SELECT id::TEXT FROM files WHERE title @@@ 'collab12' GROUP BY id))), paradedb.parse('parents:\"SFR\"')]), '{\"count\": {\"value_count\": {\"field\": \"id\"}}}')",
        ),
        (
            "CTE Equivalent - What Customer Wants (ParadeDB)",
            "WITH filtered_files AS (SELECT DISTINCT id AS file_id FROM files WHERE title @@@ 'collab12'), filtered_documents AS (SELECT id AS doc_id FROM documents WHERE parents @@@ 'SFR') SELECT count(*) FROM filtered_files ff JOIN pages p ON ff.file_id = p.\"fileId\" JOIN documents d ON p.\"fileId\" IN (SELECT f.id FROM files f WHERE f.\"documentId\" = d.id) WHERE d.id IN (SELECT doc_id FROM filtered_documents)",
        ),
    ];

    // Test queries from customer_reproduction.sql - POSTGRESQL VERSION
    let postgresql_queries = vec![
        (
            "Basic COUNT - Denormalized Pattern (PostgreSQL)",
            "SELECT count(*) FROM pages WHERE pages.content LIKE '%SFR%'",
        ),
        (
            "Basic JOIN COUNT - Original Pattern (PostgreSQL)",
            "SELECT count(*) FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON files.id = pages.\"fileId\" WHERE documents.parents LIKE '%SFR%' AND files.title LIKE '%collab12%' AND pages.content LIKE '%Single Number Reach%'",
        ),
        (
            "CTE Approach - Customer Tried (PostgreSQL)",
            "WITH filtered_documents AS (SELECT DISTINCT id FROM documents WHERE parents LIKE '%SFR%'), filtered_files AS (SELECT DISTINCT id, \"documentId\" FROM files WHERE title LIKE '%collab12%'), filtered_pages AS (SELECT DISTINCT \"fileId\" FROM pages WHERE content LIKE '%Single Number Reach%') SELECT count(*) FROM filtered_documents fd JOIN filtered_files ff ON fd.id = ff.\"documentId\" JOIN filtered_pages fp ON ff.id = fp.\"fileId\"",
        ),
        (
            "EXISTS Subquery Pattern (PostgreSQL)",
            "SELECT count(*) FROM documents WHERE documents.parents LIKE '%SFR%' AND EXISTS (SELECT 1 FROM files WHERE files.\"documentId\" = documents.id AND files.title LIKE '%collab12%')",
        ),
        (
            "Large Aggregation - Timing Discrepancy Test (PostgreSQL)",
            "SELECT LEFT(pages.title, 30) as title_prefix, count(*) as cnt FROM pages WHERE pages.title IS NOT NULL AND pages.title != '' GROUP BY LEFT(pages.title, 30) ORDER BY cnt DESC LIMIT 20",
        ),
        (
            "Complex Data Retrieval - Full Results (PostgreSQL)",
            "SELECT p.id, p.title, f.title as file_title, d.title as doc_title FROM pages p JOIN files f ON p.\"fileId\" = f.id JOIN documents d ON f.\"documentId\" = d.id WHERE p.title IS NOT NULL AND p.title != '' AND f.title IS NOT NULL AND f.title != '' AND d.parents LIKE '%SFR%' ORDER BY p.\"createdAt\" DESC LIMIT 1000",
        ),
        (
            "Large Scan with Aggregation - 60s vs 10s Issue (PostgreSQL)",
            "SELECT LEFT(p.title, 25) as title_prefix, COUNT(*) as total_pages, COUNT(DISTINCT p.\"fileId\") as unique_files, AVG(p.\"sizeInBytes\") as avg_size FROM pages p WHERE p.content IS NOT NULL AND p.content != '' GROUP BY LEFT(p.title, 25) HAVING COUNT(*) > 10 ORDER BY total_pages DESC",
        ),
        (
            "Parse Hack Aggregate - Customer Workaround (PostgreSQL)",
            "SELECT count(DISTINCT d.id) FROM documents d JOIN files f ON d.id = f.\"documentId\" WHERE d.parents LIKE '%SFR%' AND f.title LIKE '%collab12%'",
        ),
        (
            "CTE Equivalent - What Customer Wants (PostgreSQL)",
            "WITH filtered_files AS (SELECT DISTINCT id AS file_id FROM files WHERE title LIKE '%collab12%'), filtered_documents AS (SELECT id AS doc_id FROM documents WHERE parents LIKE '%SFR%') SELECT count(*) FROM filtered_files ff JOIN pages p ON ff.file_id = p.\"fileId\" JOIN documents d ON p.\"fileId\" IN (SELECT f.id FROM files f WHERE f.\"documentId\" = d.id) WHERE d.id IN (SELECT doc_id FROM filtered_documents)",
        ),
    ];

    // Run ParadeDB queries first
    println!("\n🚀 Running ParadeDB Queries...");
    for (query_name, query) in paradedb_queries {
        println!("\n--- Testing: {query_name} ---");

        match run_with_explain_analyze(conn, query, query_name).await {
            Ok(result) => {
                println!("✓ Query: {}", result.query_name);
                println!(
                    "  EXPLAIN ANALYZE time: {:.2}ms",
                    result.explain_analyze_time_ms
                );
                println!(
                    "  Actual execution time: {:.2}ms",
                    result.actual_execution_time_ms
                );
                println!(
                    "  Timing discrepancy ratio: {:.2}x",
                    result.timing_discrepancy_ratio
                );
                println!("  Heap fetches: {}", result.heap_fetches);
                println!("  Execution method: {}", result.exec_method);
                println!("  Rows returned: {}", result.row_count);

                // Flag potential issues
                if result.timing_discrepancy_ratio > 2.0 {
                    println!(
                        "  ⚠️  TIMING DISCREPANCY DETECTED: {}x slower than EXPLAIN ANALYZE",
                        result.timing_discrepancy_ratio
                    );
                }
                if result.heap_fetches > 0 {
                    println!(
                        "  ⚠️  HEAP FETCHES DETECTED: {} (should be 0 for fast fields)",
                        result.heap_fetches
                    );
                }
                if result.actual_execution_time_ms > 10000.0 {
                    println!(
                        "  ⚠️  SLOW QUERY: {:.2}s (exceeds 10s target)",
                        result.actual_execution_time_ms / 1000.0
                    );
                }

                results.push(result);
            }
            Err(e) => {
                println!("✗ Failed to run query '{query_name}': {e}");
            }
        }
    }

    // Run PostgreSQL queries
    println!("\n🐘 Running PostgreSQL Equivalent Queries...");

    // Reset settings to allow standard PostgreSQL execution
    sqlx::query("SET enable_seqscan = on")
        .execute(&mut *conn)
        .await?;
    sqlx::query("SET enable_bitmapscan = on")
        .execute(&mut *conn)
        .await?;
    sqlx::query("SET enable_indexscan = on")
        .execute(&mut *conn)
        .await?;

    for (query_name, query) in postgresql_queries {
        println!("\n--- Testing: {query_name} ---");

        match run_with_explain_analyze(conn, query, query_name).await {
            Ok(result) => {
                println!("✓ Query: {}", result.query_name);
                println!(
                    "  EXPLAIN ANALYZE time: {:.2}ms",
                    result.explain_analyze_time_ms
                );
                println!(
                    "  Actual execution time: {:.2}ms",
                    result.actual_execution_time_ms
                );
                println!(
                    "  Timing discrepancy ratio: {:.2}x",
                    result.timing_discrepancy_ratio
                );
                println!("  Heap fetches: {}", result.heap_fetches);
                println!("  Execution method: {}", result.exec_method);
                println!("  Rows returned: {}", result.row_count);

                // Flag potential issues
                if result.timing_discrepancy_ratio > 2.0 {
                    println!(
                        "  ⚠️  TIMING DISCREPANCY DETECTED: {}x slower than EXPLAIN ANALYZE",
                        result.timing_discrepancy_ratio
                    );
                }
                if result.actual_execution_time_ms > 10000.0 {
                    println!(
                        "  ⚠️  SLOW QUERY: {:.2}s (exceeds 10s target)",
                        result.actual_execution_time_ms / 1000.0
                    );
                }

                results.push(result);
            }
            Err(e) => {
                println!("✗ Failed to run query '{query_name}': {e}");
            }
        }
    }

    // Print summary
    print_join_perf_summary(&results);

    Ok(())
}

fn print_join_perf_summary(results: &[JoinPerfResult]) {
    println!("\n========================================");
    println!("JOIN PERFORMANCE SUMMARY");
    println!("========================================");

    println!("\n📊 Performance Overview:");
    println!(
        "{:<45} {:<15} {:<15} {:<15} {:<15} {:<10}",
        "Query", "EXPLAIN (ms)", "Actual (ms)", "Discrepancy", "Heap Fetches", "Exec Method"
    );
    println!("{}", "=".repeat(130));

    for result in results {
        let status = if result.timing_discrepancy_ratio > 2.0 {
            "🔴"
        } else if result.timing_discrepancy_ratio > 1.5 {
            "🟡"
        } else {
            "🟢"
        };

        println!(
            "{} {:<42} {:<15.0} {:<15.0} {:<15.2}x {:<15} {:<10}",
            status,
            &result.query_name[..std::cmp::min(42, result.query_name.len())],
            result.explain_analyze_time_ms,
            result.actual_execution_time_ms,
            result.timing_discrepancy_ratio,
            result.heap_fetches,
            result.exec_method
        );
    }

    // Group results by base query type for comparison
    let mut query_groups = std::collections::HashMap::new();
    for result in results {
        // Extract base query name (e.g., "Basic COUNT - Denormalized Pattern")
        let base_name = if let Some(pos) = result.query_name.find(" (") {
            result.query_name[..pos].to_string()
        } else {
            result.query_name.clone()
        };

        query_groups
            .entry(base_name)
            .or_insert_with(Vec::new)
            .push(result);
    }

    println!("\n⚡ ParadeDB vs PostgreSQL Performance Comparison:");
    println!(
        "{:<45} {:<15} {:<15} {:<15} {:<15}",
        "Query Type", "ParadeDB (ms)", "PostgreSQL (ms)", "Speedup", "Status"
    );
    println!("{}", "=".repeat(120));

    let mut total_paradedb_time = 0.0;
    let mut total_postgresql_time = 0.0;
    let mut comparison_count = 0;

    for (base_name, group_results) in query_groups.iter() {
        let paradedb_result = group_results
            .iter()
            .find(|r| r.query_name.contains("(ParadeDB)"));
        let postgresql_result = group_results
            .iter()
            .find(|r| r.query_name.contains("(PostgreSQL)"));

        if let (Some(paradedb), Some(postgresql)) = (paradedb_result, postgresql_result) {
            let speedup = postgresql.actual_execution_time_ms / paradedb.actual_execution_time_ms;
            let status = if speedup > 2.0 {
                "🚀 MUCH FASTER"
            } else if speedup > 1.2 {
                "⚡ FASTER"
            } else if speedup > 0.8 {
                "📊 SIMILAR"
            } else {
                "🐌 SLOWER"
            };

            println!(
                "{:<45} {:<15.0} {:<15.0} {:<15.2}x {:<15}",
                &base_name[..std::cmp::min(45, base_name.len())],
                paradedb.actual_execution_time_ms,
                postgresql.actual_execution_time_ms,
                speedup,
                status
            );

            total_paradedb_time += paradedb.actual_execution_time_ms;
            total_postgresql_time += postgresql.actual_execution_time_ms;
            comparison_count += 1;
        }
    }

    if comparison_count > 0 {
        let overall_speedup = total_postgresql_time / total_paradedb_time;
        println!("{}", "=".repeat(120));
        println!(
            "{:<45} {:<15.0} {:<15.0} {:<15.2}x {:<15}",
            "OVERALL AVERAGE",
            total_paradedb_time / comparison_count as f64,
            total_postgresql_time / comparison_count as f64,
            overall_speedup,
            if overall_speedup > 1.5 {
                "🚀 WINNER"
            } else {
                "📊 CLOSE"
            }
        );
    }

    println!("\n🔍 Key Findings:");

    // Find queries with timing discrepancies
    let discrepancy_queries: Vec<_> = results
        .iter()
        .filter(|r| r.timing_discrepancy_ratio > 2.0)
        .collect();

    if !discrepancy_queries.is_empty() {
        println!(
            "⚠️  {} queries show significant timing discrepancies (>2x):",
            discrepancy_queries.len()
        );
        for result in &discrepancy_queries {
            println!(
                "   - {}: {:.2}x slower than EXPLAIN ANALYZE",
                result.query_name, result.timing_discrepancy_ratio
            );
        }
    }

    // Find queries with heap fetches
    let heap_fetch_queries: Vec<_> = results.iter().filter(|r| r.heap_fetches > 0).collect();

    if !heap_fetch_queries.is_empty() {
        println!(
            "⚠️  {} queries have heap fetches (should be 0 for optimal performance):",
            heap_fetch_queries.len()
        );
        for result in &heap_fetch_queries {
            println!(
                "   - {}: {} heap fetches",
                result.query_name, result.heap_fetches
            );
        }
    }

    // Find slow queries
    let slow_queries: Vec<_> = results
        .iter()
        .filter(|r| r.actual_execution_time_ms > 10000.0)
        .collect();

    if !slow_queries.is_empty() {
        println!("⚠️  {} queries exceed 10s target:", slow_queries.len());
        for result in &slow_queries {
            println!(
                "   - {}: {:.2}s",
                result.query_name,
                result.actual_execution_time_ms / 1000.0
            );
        }
    }

    // ParadeDB specific findings
    let paradedb_queries: Vec<_> = results
        .iter()
        .filter(|r| r.query_name.contains("(ParadeDB)"))
        .collect();

    let postgresql_queries: Vec<_> = results
        .iter()
        .filter(|r| r.query_name.contains("(PostgreSQL)"))
        .collect();

    println!("\n📈 Performance Insights:");
    if !paradedb_queries.is_empty() {
        let avg_paradedb_time: f64 = paradedb_queries
            .iter()
            .map(|r| r.actual_execution_time_ms)
            .sum::<f64>()
            / paradedb_queries.len() as f64;
        println!("   🚀 ParadeDB average query time: {avg_paradedb_time:.2}ms");
    }

    if !postgresql_queries.is_empty() {
        let avg_postgresql_time: f64 = postgresql_queries
            .iter()
            .map(|r| r.actual_execution_time_ms)
            .sum::<f64>()
            / postgresql_queries.len() as f64;
        println!("   🐘 PostgreSQL average query time: {avg_postgresql_time:.2}ms");
    }

    // Show ParadeDB-only queries separately
    println!("\n🚀 ParadeDB-Only Features:");
    println!("{:<45} {:<15} {:<15}", "Query Type", "Time (ms)", "Status");
    println!("{}", "=".repeat(80));

    for (base_name, group_results) in query_groups.iter() {
        let paradedb_result = group_results
            .iter()
            .find(|r| r.query_name.contains("(ParadeDB)"));
        let postgresql_result = group_results
            .iter()
            .find(|r| r.query_name.contains("(PostgreSQL)"));

        // Show ParadeDB-only queries (no PostgreSQL equivalent)
        if let (Some(paradedb), None) = (paradedb_result, postgresql_result) {
            let status = if paradedb.actual_execution_time_ms > 10000.0 {
                "🐌 SLOW"
            } else if paradedb.actual_execution_time_ms > 1000.0 {
                "⚠️  MODERATE"
            } else {
                "⚡ FAST"
            };

            println!(
                "{:<45} {:<15.0} {:<15}",
                &base_name[..std::cmp::min(45, base_name.len())],
                paradedb.actual_execution_time_ms,
                status
            );
        }
    }
}
