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

//! Concurrent update tests for JoinScan.
//!
//! These tests verify that JoinScan correctly handles visibility when rows
//! are being updated concurrently by other connections. This tests the
//! interaction between:
//! - Tantivy's indexed ctids (which may become stale after UPDATE)
//! - PostgreSQL's MVCC visibility checking
//! - JoinScan's hash table lookups

mod fixtures;

use anyhow::Result;
use fixtures::*;
use rstest::*;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Helper to verify a query uses JoinScan
fn assert_uses_joinscan(conn: &mut sqlx::PgConnection, query: &str) {
    let explain_query = format!("EXPLAIN (FORMAT JSON) {}", query);
    let (plan,): (Value,) = explain_query.fetch_one(conn);
    let plan_str = format!("{:?}", plan);
    assert!(
        plan_str.contains("ParadeDB Join Scan"),
        "Query should use ParadeDB Join Scan but got plan: {}",
        plan_str
    );
}

/// Test that JoinScan returns correct results while rows are being updated concurrently.
///
/// This test:
/// 1. Creates two tables with a join relationship and BM25 indexes
/// 2. Spawns writer tasks that continuously UPDATE rows
/// 3. Spawns reader tasks that run JoinScan queries
/// 4. Verifies readers always see consistent results (no phantom rows, no missing rows)
#[rstest]
#[tokio::test]
async fn joinscan_visibility_under_concurrent_updates(database: Db) -> Result<()> {
    let mut setup_conn = database.connection().await;

    // Create tables and indexes
    r#"
    CREATE EXTENSION IF NOT EXISTS pg_search;

    -- Enable JoinScan
    SET paradedb.enable_join_custom_scan = on;

    DROP TABLE IF EXISTS items CASCADE;
    DROP TABLE IF EXISTS categories CASCADE;

    CREATE TABLE categories (
        id INTEGER PRIMARY KEY,
        name TEXT,
        description TEXT
    );

    CREATE TABLE items (
        id INTEGER PRIMARY KEY,
        name TEXT,
        content TEXT,
        category_id INTEGER REFERENCES categories(id),
        version INTEGER DEFAULT 1
    );

    -- Insert categories
    INSERT INTO categories (id, name, description) VALUES
        (1, 'Electronics', 'Electronic devices'),
        (2, 'Books', 'Physical books'),
        (3, 'Clothing', 'Apparel items');

    -- Insert items with searchable content
    INSERT INTO items (id, name, content, category_id) VALUES
        (101, 'Laptop', 'wireless portable computer', 1),
        (102, 'Phone', 'wireless mobile device', 1),
        (103, 'Novel', 'fiction book story', 2),
        (104, 'Textbook', 'educational book reference', 2),
        (105, 'Shirt', 'cotton casual wear', 3),
        (106, 'Jacket', 'wireless heated outerwear', 3);

    CREATE INDEX items_bm25_idx ON items USING bm25 (id, name, content, category_id)
        WITH (key_field = 'id', numeric_fields = '{"category_id": {"fast": true}}');
    "#
    .execute(&mut setup_conn);

    // Verify the queries use JoinScan
    assert_uses_joinscan(
        &mut setup_conn,
        r#"SELECT i.id, i.name, c.name, i.version
           FROM items i
           JOIN categories c ON i.category_id = c.id
           WHERE i.content @@@ 'wireless'
           ORDER BY i.id
           LIMIT 10"#,
    );

    assert_uses_joinscan(
        &mut setup_conn,
        r#"SELECT i.id, i.name, c.name
           FROM items i
           JOIN categories c ON i.category_id = c.id
           WHERE i.content @@@ 'book'
           ORDER BY i.id
           LIMIT 10"#,
    );

    // Control flags
    let stop_flag = Arc::new(AtomicBool::new(false));
    let update_count = Arc::new(AtomicUsize::new(0));
    let query_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    // Create all connections upfront
    let num_writers = 3;
    let num_readers = 5;

    let mut writer_conns = vec![];
    for _ in 0..num_writers {
        writer_conns.push(database.connection().await);
    }

    let mut reader_conns = vec![];
    for _ in 0..num_readers {
        reader_conns.push(database.connection().await);
    }

    // Spawn writer tasks
    let mut writer_handles = vec![];

    for mut conn in writer_conns {
        let stop = stop_flag.clone();
        let updates = update_count.clone();

        let handle = tokio::spawn(async move {
            // Enable JoinScan for this connection
            "SET paradedb.enable_join_custom_scan = on;".execute(&mut conn);

            let item_ids = [101, 102, 103, 104, 105, 106];
            let mut iteration = 0;

            while !stop.load(Ordering::Relaxed) {
                // Cycle through items
                let item_id = item_ids[iteration % item_ids.len()];
                iteration += 1;

                // Update the version (this moves the tuple to a new ctid)
                let query = format!(
                    "UPDATE items SET version = version + 1 WHERE id = {}",
                    item_id
                );
                query.execute(&mut conn);
                updates.fetch_add(1, Ordering::Relaxed);

                // Small delay to allow interleaving
                sleep(Duration::from_millis(5)).await;
            }
        });
        writer_handles.push(handle);
    }

    // Spawn reader tasks that run JoinScan queries
    let mut reader_handles = vec![];

    for (reader_id, mut conn) in reader_conns.into_iter().enumerate() {
        let stop = stop_flag.clone();
        let queries = query_count.clone();
        let errors = error_count.clone();

        let handle = tokio::spawn(async move {
            // Enable JoinScan for this connection
            "SET paradedb.enable_join_custom_scan = on;".execute(&mut conn);

            while !stop.load(Ordering::Relaxed) {
                // Query 1: Search for 'wireless' items with join
                let result: Vec<(i32, String, String, i32)> = r#"
                    SELECT i.id, i.name, c.name, i.version
                    FROM items i
                    JOIN categories c ON i.category_id = c.id
                    WHERE i.content @@@ 'wireless'
                    ORDER BY i.id
                    LIMIT 10
                "#
                .fetch_result(&mut conn)
                .unwrap_or_else(|e| {
                    eprintln!("Reader {} error: {:?}", reader_id, e);
                    errors.fetch_add(1, Ordering::Relaxed);
                    vec![]
                });

                // Verify: Should always find wireless items (101, 102, 106)
                let ids: Vec<i32> = result.iter().map(|(id, _, _, _)| *id).collect();

                // All three wireless items should be present
                if !ids.contains(&101) || !ids.contains(&102) || !ids.contains(&106) {
                    eprintln!(
                        "Reader {}: Missing expected wireless items! Got: {:?}",
                        reader_id, ids
                    );
                    errors.fetch_add(1, Ordering::Relaxed);
                }

                // Should not have more than 3 results for 'wireless'
                if result.len() > 3 {
                    eprintln!(
                        "Reader {}: Too many results for wireless! Got {} items: {:?}",
                        reader_id,
                        result.len(),
                        ids
                    );
                    errors.fetch_add(1, Ordering::Relaxed);
                }

                queries.fetch_add(1, Ordering::Relaxed);

                // Query 2: Different search term
                let result2: Vec<(i32, String, String)> = r#"
                    SELECT i.id, i.name, c.name
                    FROM items i
                    JOIN categories c ON i.category_id = c.id
                    WHERE i.content @@@ 'book'
                    ORDER BY i.id
                    LIMIT 10
                "#
                .fetch_result(&mut conn)
                .unwrap_or_else(|e| {
                    eprintln!("Reader {} error on query2: {:?}", reader_id, e);
                    errors.fetch_add(1, Ordering::Relaxed);
                    vec![]
                });

                // Should find items 103 (Novel) and 104 (Textbook)
                let ids2: Vec<i32> = result2.iter().map(|(id, _, _)| *id).collect();
                if !ids2.contains(&103) || !ids2.contains(&104) {
                    eprintln!(
                        "Reader {}: Missing expected book items! Got: {:?}",
                        reader_id, ids2
                    );
                    errors.fetch_add(1, Ordering::Relaxed);
                }

                queries.fetch_add(1, Ordering::Relaxed);

                // Small delay
                sleep(Duration::from_millis(10)).await;
            }
        });
        reader_handles.push(handle);
    }

    // Run for a fixed duration
    sleep(Duration::from_secs(5)).await;

    // Signal stop
    stop_flag.store(true, Ordering::Relaxed);

    // Wait for all tasks to complete
    for handle in writer_handles {
        handle.await?;
    }
    for handle in reader_handles {
        handle.await?;
    }

    // Report statistics
    let total_updates = update_count.load(Ordering::Relaxed);
    let total_queries = query_count.load(Ordering::Relaxed);
    let total_errors = error_count.load(Ordering::Relaxed);

    eprintln!(
        "Concurrent test completed: {} updates, {} queries, {} errors",
        total_updates, total_queries, total_errors
    );

    // Verify no errors occurred
    assert_eq!(
        total_errors, 0,
        "JoinScan returned inconsistent results during concurrent updates"
    );

    // Verify we actually did work
    assert!(total_updates > 50, "Expected at least 50 updates");
    assert!(total_queries > 20, "Expected at least 20 queries");

    Ok(())
}

/// Test that JoinScan handles concurrent updates to join keys correctly.
///
/// When a join key (category_id) is updated, the JoinScan should:
/// 1. Not return the old join result (item with old category)
/// 2. Return the new join result (item with new category)
#[rstest]
#[tokio::test]
async fn joinscan_join_key_updates(database: Db) -> Result<()> {
    let mut setup_conn = database.connection().await;

    r#"
    CREATE EXTENSION IF NOT EXISTS pg_search;
    SET paradedb.enable_join_custom_scan = on;

    DROP TABLE IF EXISTS products CASCADE;
    DROP TABLE IF EXISTS suppliers CASCADE;

    CREATE TABLE suppliers (
        id INTEGER PRIMARY KEY,
        name TEXT
    );

    CREATE TABLE products (
        id INTEGER PRIMARY KEY,
        name TEXT,
        content TEXT,
        supplier_id INTEGER
    );

    INSERT INTO suppliers VALUES (1, 'Supplier A'), (2, 'Supplier B'), (3, 'Supplier C');

    INSERT INTO products (id, name, content, supplier_id) VALUES
        (1, 'Widget', 'wireless widget device', 1),
        (2, 'Gadget', 'wired gadget tool', 2);

    CREATE INDEX products_bm25_idx ON products USING bm25 (id, name, content, supplier_id)
        WITH (key_field = 'id', numeric_fields = '{"supplier_id": {"fast": true}}');
    "#
    .execute(&mut setup_conn);

    // Define the query once
    let wireless_query = r#"
        SELECT p.id, p.name, s.name
        FROM products p
        JOIN suppliers s ON p.supplier_id = s.id
        WHERE p.content @@@ 'wireless'
        LIMIT 10
    "#;

    // Verify the query uses JoinScan
    assert_uses_joinscan(&mut setup_conn, wireless_query);

    // Query before update - Widget should be with Supplier A
    let before: Vec<(i32, String, String)> = wireless_query.fetch_result(&mut setup_conn)?;

    assert_eq!(before.len(), 1);
    assert_eq!(before[0].0, 1); // Widget id
    assert_eq!(before[0].2, "Supplier A");

    // Update the join key - move Widget from Supplier A to Supplier C
    "UPDATE products SET supplier_id = 3 WHERE id = 1".execute(&mut setup_conn);

    // Query after update - Widget should now be with Supplier C
    let after: Vec<(i32, String, String)> = wireless_query.fetch_result(&mut setup_conn)?;

    assert_eq!(after.len(), 1);
    assert_eq!(after[0].0, 1); // Widget id
    assert_eq!(after[0].2, "Supplier C"); // New supplier

    Ok(())
}

/// Test rapid updates and queries to stress visibility checking.
#[rstest]
#[tokio::test]
async fn joinscan_rapid_updates_stress(database: Db) -> Result<()> {
    let mut setup_conn = database.connection().await;

    r#"
    CREATE EXTENSION IF NOT EXISTS pg_search;
    SET paradedb.enable_join_custom_scan = on;

    DROP TABLE IF EXISTS stress_items CASCADE;
    DROP TABLE IF EXISTS stress_refs CASCADE;

    CREATE TABLE stress_refs (
        id INTEGER PRIMARY KEY,
        ref_name TEXT
    );

    CREATE TABLE stress_items (
        id INTEGER PRIMARY KEY,
        content TEXT,
        ref_id INTEGER,
        counter INTEGER DEFAULT 0
    );

    INSERT INTO stress_refs VALUES (1, 'Ref A'), (2, 'Ref B');

    INSERT INTO stress_items (id, content, ref_id) VALUES
        (1, 'wireless alpha', 1),
        (2, 'wireless beta', 2),
        (3, 'wired gamma', 1);

    CREATE INDEX stress_items_bm25_idx ON stress_items USING bm25 (id, content, ref_id)
        WITH (key_field = 'id', numeric_fields = '{"ref_id": {"fast": true}}');
    "#
    .execute(&mut setup_conn);

    // Define the query once
    let stress_query = r#"
        SELECT si.id, si.content, sr.ref_name, si.counter
        FROM stress_items si
        JOIN stress_refs sr ON si.ref_id = sr.id
        WHERE si.content @@@ 'wireless'
        ORDER BY si.id
        LIMIT 10
    "#;

    // Verify the query uses JoinScan
    assert_uses_joinscan(&mut setup_conn, stress_query);

    // Perform rapid update-query cycles
    for i in 0..100 {
        // Update counter (moves ctid)
        format!(
            "UPDATE stress_items SET counter = {} WHERE content LIKE '%wireless%'",
            i
        )
        .execute(&mut setup_conn);

        // Immediately query
        let results: Vec<(i32, String, String, i32)> =
            stress_query.fetch_result(&mut setup_conn)?;

        // Should always find exactly 2 wireless items
        assert_eq!(
            results.len(),
            2,
            "Iteration {}: Expected 2 results, got {}",
            i,
            results.len()
        );

        // Counters should match the update
        for (id, _, _, counter) in &results {
            assert_eq!(
                *counter, i,
                "Iteration {}: Item {} has wrong counter {}",
                i, id, counter
            );
        }
    }

    Ok(())
}
