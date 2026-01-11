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

mod fixtures;

use std::time::Instant;

use anyhow::Result;
use fixtures::*;
use futures::future::join_all;
use pretty_assertions::assert_eq;
use rand::Rng;
use rstest::*;
use sqlx::Row;
use tokio::join;

/// This test targets the locking functionality between Tantivy writers.
/// With no locking implemented, a high number of concurrent writers will
/// cause in an error when they all try to commit to the index at once.
#[rstest]
#[tokio::test]
async fn test_simultaneous_commits_with_bm25(database: Db) -> Result<()> {
    let mut conn1 = database.connection().await;

    // Create table once using any of the connections.
    r#"CREATE EXTENSION pg_search;

    CREATE TABLE concurrent_items (
      id SERIAL PRIMARY KEY,
      description TEXT,
      category VARCHAR(255),
      created_at TIMESTAMP DEFAULT now()
    );

    CREATE INDEX concurrent_items_bm25 ON public.concurrent_items
    USING bm25 (id, description)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {}
        }'
    );
    "#
    .execute(&mut conn1);

    // Dynamically generate at least 100 rows for each connection
    let mut rng = rand::rng();
    let categories = [
        "Category 1",
        "Category 2",
        "Category 3",
        "Category 4",
        "Category 5",
    ];

    for i in 0..5 {
        let random_category = categories[rng.random_range(0..categories.len())];

        // Create new connections for this iteration and store them in a vector
        let mut connections = vec![];
        for _ in 0..50 {
            connections.push(database.connection().await);
        }

        let mut futures = vec![];
        for (n, mut conn) in connections.into_iter().enumerate() {
            let query = format!(
                "INSERT INTO concurrent_items (description, category)
                 VALUES ('Item {i} from conn{n}', '{random_category}')"
            );
            // Move the connection into the future, avoiding multiple borrows
            futures.push(async move { query.execute_async(&mut conn).await });
        }

        // Await all the futures for this iteration
        join_all(futures).await;
    }

    // Verify the number of rows in each database
    let rows1: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM concurrent_items")
        .fetch_one(&mut conn1)
        .await?;

    assert_eq!(rows1, 250);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_statement_level_locking(database: Db) -> Result<()> {
    let mut conn = database.connection().await;

    // Create tables and indexes
    r#"CREATE EXTENSION pg_search;
    CREATE TABLE index_a (
      id SERIAL PRIMARY KEY,
      content TEXT
    );
    CREATE TABLE index_b (
      id SERIAL PRIMARY KEY,
      content TEXT
    );

    CREATE INDEX index_a_bm25 ON public.index_a
    USING bm25 (id, content)
    WITH (
        key_field = 'id',
        text_fields = '{
            "content": {}
        }'
    );

    CREATE INDEX index_b_bm25 ON public.index_b
    USING bm25 (id, content)
    WITH (
        key_field = 'id',
        text_fields = '{
            "content": {}
        }'
    );
    "#
    .execute(&mut conn);

    // Create two separate connections
    let mut conn_a = database.connection().await;
    let mut conn_b = database.connection().await;

    // Define the tasks for each connection
    let task_a = async move {
        "INSERT INTO index_a (content) VALUES ('Content A1');
         SELECT pg_sleep(3);
         INSERT INTO index_b (content) VALUES ('Content B1 from A');"
            .execute_async(&mut conn_a)
            .await;
    };

    let task_b = async move {
        "INSERT INTO index_b (content) VALUES ('Content B2');
         SELECT pg_sleep(3);
         INSERT INTO index_a (content) VALUES ('Content A2 from B');"
            .execute_async(&mut conn_b)
            .await;
    };

    // We're going to check a timer to ensure both of these queries,
    // which each sleep at query time, run concurrently.
    let start_time = Instant::now();

    // Run both tasks concurrently
    join!(task_a, task_b);

    // Stop the timer and assert that the duration is close to 5 seconds
    let duration = start_time.elapsed();
    assert!(
        duration.as_secs() >= 3 && duration.as_secs() < 5,
        "Expected duration to be around 3 seconds, but it took {duration:?}"
    );

    // Verify the results
    let count_a: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM index_a")
        .fetch_one(&mut conn)
        .await?;
    let count_b: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM index_b")
        .fetch_one(&mut conn)
        .await?;

    assert_eq!(count_a, 2, "Expected 2 rows in index_a");
    assert_eq!(count_b, 2, "Expected 2 rows in index_b");

    Ok(())
}

/// Test for race condition in parallel index scans with hash joins.
///
/// Root cause: In Parallel Hash Join scenarios, workers open their SearchIndexReader
/// at different times than the leader. If segment merges occur between when different
/// participants open the index, they may see different segment lists, causing panics
/// or incorrect results.
///
/// The fix ensures:
/// 1. Leader opens with Snapshot visibility and populates shared state with its segment list
/// 2. Workers wait for leader initialization, then open with ParallelWorker visibility
///    restricted to ONLY the segments in shared state
#[rstest]
#[tokio::test]
async fn test_parallel_hash_join_race_condition(database: Db) -> Result<()> {
    let mut conn = database.connection().await;

    // Create extension and tables
    r#"CREATE EXTENSION IF NOT EXISTS pg_search;

    DROP TABLE IF EXISTS document_text CASCADE;
    DROP TABLE IF EXISTS core CASCADE;

    CREATE TABLE core (
        dwf_doid BIGINT PRIMARY KEY,
        author TEXT,
        date_time_combined TIMESTAMP WITHOUT TIME ZONE
    );

    CREATE TABLE document_text (
        dwf_doid BIGINT PRIMARY KEY,
        full_text TEXT
    );

    -- Create BM25 indexes BEFORE inserting data
    CREATE INDEX idx_parade_core ON core
    USING bm25 (dwf_doid, author)
    WITH (key_field='dwf_doid');

    CREATE INDEX idx_parade_document_text ON document_text
    USING bm25 (dwf_doid, full_text)
    WITH (key_field='dwf_doid');
    "#
    .execute(&mut conn);

    // Insert data in batches to create multiple segments
    // Each batch creates new segments which is critical for reproducing the race
    for (start, end) in [(1, 5000), (5001, 10000), (10001, 15000), (15001, 20000)] {
        format!(
            r#"
            INSERT INTO core (dwf_doid, author, date_time_combined)
            SELECT 
                i,
                CASE 
                    WHEN i % 3 = 0 THEN 'brian griffin'
                    WHEN i % 3 = 1 THEN 'barabara pewterschmidt'
                    ELSE 'bonnie swanson'
                END,
                '2024-01-01'::timestamp + (i || ' days')::interval
            FROM generate_series({start}, {end}) i;

            INSERT INTO document_text (dwf_doid, full_text)
            SELECT i, 'This is document ' || i || ' with text containing ea'
            FROM generate_series({start}, {end}) i;
            "#
        )
        .execute(&mut conn);
    }

    // Create regular index on date (not in BM25 index)
    "CREATE INDEX idx_date_time_combined_date ON core (DATE(date_time_combined))"
        .execute(&mut conn);

    // CRITICAL: Disable Custom Scan to force the use of Index Only Scan (Index AM path)
    // Enable parallel workers and force parallel plans
    r#"
    SET paradedb.enable_custom_scan = false;
    SET max_parallel_workers_per_gather = 2;
    SET parallel_tuple_cost = 0;
    SET parallel_setup_cost = 0;
    SET min_parallel_table_scan_size = 0;
    SET min_parallel_index_scan_size = 0;
    "#
    .execute(&mut conn);

    // Try to force parallel mode - use the appropriate GUC for each PG version
    // PG14-17: force_parallel_mode, PG18+: debug_parallel_query
    let _ = "SET force_parallel_mode = on".execute_result(&mut conn);
    let _ = "SET debug_parallel_query = on".execute_result(&mut conn);

    let query = r#"
        SELECT COUNT(*)
        FROM document_text dt
        JOIN core c ON dt.dwf_doid = c.dwf_doid
        WHERE dt.full_text @@@ 'ea'
          AND (c.author @@@ paradedb.match('author', 'brian griffin')
               OR c.author @@@ paradedb.match('author', 'barabara pewterschmidt')
               OR c.author @@@ paradedb.match('author', 'bonnie swanson'))
          AND DATE(c.date_time_combined) >= DATE('2001-01-01')
          AND DATE(c.date_time_combined) <= DATE('2025-12-31')
    "#;

    // Check EXPLAIN to verify parallel workers are planned
    let explain_query = format!("EXPLAIN (FORMAT TEXT) {}", query);
    let explain_rows: Vec<(String,)> = explain_query.fetch(&mut conn);
    let explain_text: String = explain_rows
        .iter()
        .map(|(s,)| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let has_gather = explain_text.contains("Gather");
    let has_parallel_workers = explain_text.contains("Workers Planned:");
    let has_parallel_index_scan = explain_text.contains("Parallel Index");
    let is_parallel = has_gather || has_parallel_workers;

    println!(
        "Parallel check: Gather={}, Workers Planned={}, Parallel Index Scan={}\nEXPLAIN:\n{}",
        has_gather, has_parallel_workers, has_parallel_index_scan, explain_text
    );

    assert!(
        is_parallel,
        "Query plan should use parallel execution. EXPLAIN:\n{}",
        explain_text
    );

    // Run the query 15 times - all should return 730
    // Before the fix, this would intermittently return 0 due to the race condition
    let mut counts = Vec::new();
    for _ in 0..15 {
        let row = sqlx::query(query).fetch_one(&mut conn).await?;
        let count: i64 = row.get(0);
        counts.push(count);
    }

    // Check for any zeros - the race condition symptom
    let zeros = counts.iter().filter(|&&c| c == 0).count();
    let expected = 730i64; // All 20000 docs match (all have 'ea', all authors match, all dates in range)

    println!("Results: {:?}", counts);
    println!("Zeros: {}, Expected: {}", zeros, expected);

    // All counts should be equal and non-zero
    assert!(
        zeros == 0,
        "Race condition detected! {} queries returned 0 instead of {}. Results: {:?}",
        zeros,
        expected,
        counts
    );

    for (i, count) in counts.iter().enumerate() {
        assert_eq!(
            *count, expected,
            "Query {} returned {} but expected {} - inconsistent results!",
            i, count, expected
        );
    }

    println!(
        "All {} queries returned consistent count: {}",
        counts.len(),
        expected
    );

    Ok(())
}
