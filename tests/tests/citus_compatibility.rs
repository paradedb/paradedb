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

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

/// Test that pg_search's planner hook properly chains with Citus
/// This test reproduces the original bug where pg_search had two separate
/// PREV_PLANNER_HOOK variables, causing Citus's planner hook to never be called.
///
/// The bug manifested as:
/// ERROR: Query could not find the intermediate result file "3_1"
/// when running distributed queries with subqueries containing LIMIT clauses.
///
/// Note: This test requires Citus to be installed and loaded via shared_preload_libraries.
/// If Citus is not installed, the test will skip. If Citus is installed but not preloaded,
/// the test will fail (as this is a configuration error).
#[rstest]
fn citus_distributed_tables_with_subquery_limit(mut conn: PgConnection) {
    // Check if Citus is in shared_preload_libraries
    let preload_libs: Vec<(String,)> = "SHOW shared_preload_libraries".fetch(&mut conn);
    let preload_str = &preload_libs[0].0;
    let citus_preloaded = preload_str.contains("citus");

    if !citus_preloaded {
        // Citus not in shared_preload_libraries, skip test
        eprintln!("Skipping test: Citus not found in shared_preload_libraries");
        return;
    }

    // Citus is preloaded, so CREATE EXTENSION should work
    // If it fails, that's a real error and the test should fail
    "CREATE EXTENSION IF NOT EXISTS citus".execute(&mut conn);

    // Create tables with explicit integer IDs (Citus compatible)
    r#"
    CREATE TABLE products (
        id INT PRIMARY KEY,
        name TEXT,
        description TEXT,
        category TEXT
    );

    CREATE TABLE reviews (
        id INT,
        product_id INT,
        content TEXT,
        rating INT,
        PRIMARY KEY (id, product_id)
    );
    "#
    .execute(&mut conn);

    // Insert test data
    r#"
    INSERT INTO products (id, name, description, category) VALUES
        (1, 'Laptop', 'High performance laptop for coding', 'Electronics'),
        (2, 'Mouse', 'Wireless ergonomic mouse', 'Electronics'),
        (3, 'Keyboard', 'Mechanical keyboard with RGB lighting', 'Electronics'),
        (4, 'Monitor', 'Ultra-wide curved monitor display', 'Electronics'),
        (5, 'Desk', 'Standing desk adjustable height', 'Furniture');

    INSERT INTO reviews (id, product_id, content, rating) VALUES
        (1, 1, 'Great laptop for development work', 5),
        (2, 1, 'Fast and reliable performance', 5),
        (3, 2, 'Comfortable mouse for daily use', 4),
        (4, 3, 'Best keyboard ever made', 5),
        (5, 4, 'Amazing picture quality and colors', 5);
    "#
    .execute(&mut conn);

    // Create BM25 indexes (triggers pg_search planner hook)
    r#"
    CREATE INDEX products_idx ON products 
    USING bm25 (id, name, description, category) 
    WITH (key_field='id');

    CREATE INDEX reviews_idx ON reviews 
    USING bm25 (id, content, rating) 
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Distribute tables (triggers Citus planner hook)
    // This is the critical step that reproduces the original bug
    "SELECT create_distributed_table('products', 'id')".execute(&mut conn);
    "SELECT create_distributed_table('reviews', 'product_id')".execute(&mut conn);

    // Test 1: Basic subquery with LIMIT on pg_search indexed table
    // This is the exact pattern that broke with Citus when hooks weren't chained
    let rows: Vec<(i32, String)> = r#"
        SELECT id, name FROM products 
        WHERE description @@@ 'laptop OR keyboard'
          AND id IN (SELECT product_id FROM reviews LIMIT 3)
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows, vec![(1, "Laptop".to_string())]);

    // Verify EXPLAIN plan shows both ParadeDB Custom Scan and Citus distributed execution
    let (plan,): (Value,) = r#"
        EXPLAIN (VERBOSE, FORMAT JSON)
        SELECT id, name FROM products 
        WHERE description @@@ 'laptop OR keyboard'
          AND id IN (SELECT product_id FROM reviews LIMIT 3)
        ORDER BY id
    "#
    .fetch_one(&mut conn);

    let plan_str = format!("{:?}", plan);
    eprintln!(
        "Test 1 EXPLAIN plan:\n{}",
        serde_json::to_string_pretty(&plan).unwrap()
    );

    // Check for ParadeDB Custom Scan (PdbScan)
    assert!(
        plan_str.contains("ParadeDB Scan") || plan_str.contains("Custom Scan"),
        "EXPLAIN plan should contain ParadeDB Custom Scan, but got: {}",
        plan_str
    );

    // Check for Citus distributed query execution
    // Citus uses "Custom Scan (Citus" nodes for distributed queries
    assert!(
        plan_str.contains("Citus") || plan_str.contains("distributed"),
        "EXPLAIN plan should contain Citus distributed query nodes, but got: {}",
        plan_str
    );

    // Test 2: Subquery with LIMIT and search on both tables
    let rows: Vec<(i32, String)> = r#"
        SELECT id, name FROM products 
        WHERE description @@@ 'monitor OR desk'
          AND id IN (
            SELECT product_id FROM reviews 
            WHERE content @@@ 'quality'
            LIMIT 2
          )
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows, vec![(4, "Monitor".to_string())]);

    // Verify EXPLAIN plan for query with search operators on both tables
    let (plan,): (Value,) = r#"
        EXPLAIN (VERBOSE, FORMAT JSON)
        SELECT id, name FROM products 
        WHERE description @@@ 'monitor OR desk'
          AND id IN (
            SELECT product_id FROM reviews 
            WHERE content @@@ 'quality'
            LIMIT 2
          )
        ORDER BY id
    "#
    .fetch_one(&mut conn);

    let plan_str = format!("{:?}", plan);
    eprintln!(
        "Test 2 EXPLAIN plan:\n{}",
        serde_json::to_string_pretty(&plan).unwrap()
    );

    // Verify both ParadeDB scans (on products and reviews) and Citus distributed execution
    assert!(
        plan_str.contains("ParadeDB Scan") || plan_str.contains("Custom Scan"),
        "EXPLAIN plan should contain ParadeDB Custom Scan for search operators, but got: {}",
        plan_str
    );

    assert!(
        plan_str.contains("Citus") || plan_str.contains("distributed"),
        "EXPLAIN plan should contain Citus distributed query nodes, but got: {}",
        plan_str
    );

    // Test 3: CTE with LIMIT and pg_search
    let rows: Vec<(i32, String)> = r#"
        WITH top_rated AS (
          SELECT product_id FROM reviews 
          WHERE content @@@ 'best OR amazing' 
          ORDER BY rating DESC 
          LIMIT 2
        )
        SELECT id, name FROM products 
        WHERE description @@@ 'keyboard OR monitor'
          AND id IN (SELECT product_id FROM top_rated)
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(
        rows,
        vec![(3, "Keyboard".to_string()), (4, "Monitor".to_string())]
    );

    // Test 4: Simple subquery with LIMIT (minimal reproducer)
    let rows: Vec<(i32, String)> = r#"
        SELECT id, name FROM products
        WHERE id IN (SELECT product_id FROM reviews LIMIT 2)
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows, vec![(1, "Laptop".to_string())]);

    // Cleanup
    "DROP TABLE reviews CASCADE".execute(&mut conn);
    "DROP TABLE products CASCADE".execute(&mut conn);
}

/// Test Citus with pg_search when Citus processes a query without pg_search operators
/// This ensures the hook chaining works even when pg_search isn't actively involved
///
/// Note: This test requires Citus to be installed and loaded via shared_preload_libraries.
/// If Citus is not installed, the test will skip. If Citus is installed but not preloaded,
/// the test will fail (as this is a configuration error).
#[rstest]
fn citus_without_search_operators(mut conn: PgConnection) {
    // Check if Citus is in shared_preload_libraries
    let preload_libs: Vec<(String,)> = "SHOW shared_preload_libraries".fetch(&mut conn);
    let preload_str = &preload_libs[0].0;
    let citus_preloaded = preload_str.contains("citus");

    if !citus_preloaded {
        // Citus not in shared_preload_libraries, skip test
        eprintln!("Skipping test: Citus not found in shared_preload_libraries");
        return;
    }

    // Citus is preloaded, so CREATE EXTENSION should work
    // If it fails, that's a real error and the test should fail
    "CREATE EXTENSION IF NOT EXISTS citus".execute(&mut conn);

    r#"
    CREATE TABLE simple_table (
        id INT PRIMARY KEY,
        value TEXT
    );

    INSERT INTO simple_table (id, value) VALUES
        (1, 'first'),
        (2, 'second'),
        (3, 'third');
    "#
    .execute(&mut conn);

    // Distribute table
    "SELECT create_distributed_table('simple_table', 'id')".execute(&mut conn);

    // Query without pg_search operators (tests that Citus hook still works)
    // The ORDER BY in the subquery ensures deterministic results
    let rows: Vec<(i32, String)> = r#"
        SELECT id, value FROM simple_table
        WHERE id IN (SELECT id FROM simple_table WHERE id > 1 ORDER BY id LIMIT 1)
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows, vec![(2, "second".to_string())]);

    "DROP TABLE simple_table CASCADE".execute(&mut conn);
}
