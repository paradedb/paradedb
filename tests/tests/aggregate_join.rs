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

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

/// Set up two tables with BM25 indexes for join aggregate tests.
fn setup_join_tables(conn: &mut PgConnection) {
    r#"
    CREATE TABLE products (
        id SERIAL PRIMARY KEY,
        description TEXT,
        category TEXT,
        price FLOAT,
        rating INTEGER
    );
    CREATE TABLE tags (
        id SERIAL PRIMARY KEY,
        product_id INTEGER,
        tag_name TEXT
    );
    INSERT INTO products (description, category, price, rating) VALUES
        ('Laptop with fast processor', 'Electronics', 999.99, 5),
        ('Gaming laptop with RGB', 'Electronics', 1299.99, 5),
        ('Running shoes for athletes', 'Sports', 89.99, 4),
        ('Winter jacket warm', 'Clothing', 129.99, 3),
        ('Toy laptop for kids', 'Toys', 499.99, 2);
    INSERT INTO tags (product_id, tag_name) VALUES
        (1, 'tech'), (1, 'computer'),
        (2, 'tech'), (2, 'gaming'),
        (3, 'fitness'), (3, 'running'),
        (4, 'outdoor'),
        (5, 'tech'), (5, 'kids');
    CREATE INDEX products_idx ON products
    USING bm25 (id, description, category, price, rating)
    WITH (
        key_field='id',
        text_fields='{"description": {}, "category": {"fast": true}}',
        numeric_fields='{"price": {"fast": true}, "rating": {"fast": true}}'
    );
    CREATE INDEX tags_idx ON tags
    USING bm25 (id, product_id, tag_name)
    WITH (
        key_field='id',
        numeric_fields='{"product_id": {"fast": true}}',
        text_fields='{"tag_name": {"fast": true}}'
    );
    SET paradedb.enable_aggregate_custom_scan TO on;
    "#
    .execute(conn);
}

#[rstest]
fn test_join_aggregate_count(mut conn: PgConnection) {
    setup_join_tables(&mut conn);

    // 3 laptops (ids 1,2,5) with tags: 1→2tags, 2→2tags, 5→2tags = 6 joined rows
    let (count,) = r#"
        SELECT COUNT(*)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(count, 6, "3 laptops × 2 tags each = 6 joined rows");
}

#[rstest]
fn test_join_aggregate_sum_avg(mut conn: PgConnection) {
    setup_join_tables(&mut conn);

    // Use AVG on a FLOAT column (price) to avoid NUMERIC type issues
    let (count, sum, avg_price) = r#"
        SELECT COUNT(*), SUM(p.price), AVG(p.price)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(i64, f64, f64)>(&mut conn);

    assert_eq!(count, 6);
    // SUM: (999.99*2 + 1299.99*2 + 499.99*2) = 5599.94
    assert!(
        (sum - 5599.94).abs() < 0.01,
        "SUM(price) should be ~5599.94, got {sum}"
    );
    // AVG(price) = 5599.94 / 6 ≈ 933.32
    assert!(
        (avg_price - 933.32).abs() < 0.1,
        "AVG(price) should be ~933.32, got {avg_price}"
    );
}

#[rstest]
fn test_join_aggregate_min_max(mut conn: PgConnection) {
    setup_join_tables(&mut conn);

    let (min_price, max_price) = r#"
        SELECT MIN(p.price), MAX(p.price)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(f64, f64)>(&mut conn);

    assert!(
        (min_price - 499.99).abs() < 0.01,
        "MIN should be 499.99, got {min_price}"
    );
    assert!(
        (max_price - 1299.99).abs() < 0.01,
        "MAX should be 1299.99, got {max_price}"
    );
}

#[rstest]
fn test_join_aggregate_empty_result(mut conn: PgConnection) {
    setup_join_tables(&mut conn);

    // COUNT on empty result should return 0
    let (count,) = r#"
        SELECT COUNT(*)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'nonexistent_xyz'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(count, 0, "COUNT on empty result should be 0");

    // SUM/AVG on empty result should return NULL
    let result = r#"
        SELECT SUM(p.price), AVG(p.price)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'nonexistent_xyz'
    "#
    .fetch_one::<(Option<f64>, Option<f64>)>(&mut conn);

    assert_eq!(result.0, None, "SUM on empty result should be NULL");
    assert_eq!(result.1, None, "AVG on empty result should be NULL");
}

#[rstest]
fn test_join_aggregate_parity_with_postgres(mut conn: PgConnection) {
    setup_join_tables(&mut conn);

    // Get result with DataFusion backend (custom scan ON)
    let df_result = r#"
        SELECT COUNT(*), SUM(p.price), MIN(p.price), MAX(p.price)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(i64, f64, f64, f64)>(&mut conn);

    // Get result with Postgres default plan (custom scan OFF)
    "SET paradedb.enable_aggregate_custom_scan TO off".execute(&mut conn);

    let pg_result = r#"
        SELECT COUNT(*), SUM(p.price), MIN(p.price), MAX(p.price)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(i64, f64, f64, f64)>(&mut conn);

    assert_eq!(
        df_result.0, pg_result.0,
        "COUNT mismatch: DataFusion vs Postgres"
    );
    assert!(
        (df_result.1 - pg_result.1).abs() < 0.01,
        "SUM mismatch: DataFusion={} vs Postgres={}",
        df_result.1,
        pg_result.1
    );
    assert!(
        (df_result.2 - pg_result.2).abs() < 0.01,
        "MIN mismatch: DataFusion={} vs Postgres={}",
        df_result.2,
        pg_result.2
    );
    assert!(
        (df_result.3 - pg_result.3).abs() < 0.01,
        "MAX mismatch: DataFusion={} vs Postgres={}",
        df_result.3,
        pg_result.3
    );
}

#[rstest]
fn test_join_aggregate_after_insert(mut conn: PgConnection) {
    setup_join_tables(&mut conn);

    // Get initial count
    let (count_before,) = r#"
        SELECT COUNT(*)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(count_before, 6);

    // Insert a new laptop with 1 tag
    r#"
        INSERT INTO products (description, category, price, rating) VALUES
            ('Budget laptop cheap', 'Electronics', 299.99, 3);
        INSERT INTO tags (product_id, tag_name)
            SELECT MAX(id), 'budget' FROM products;
    "#
    .execute(&mut conn);

    // Count should increase by 1 (new laptop × 1 tag)
    let (count_after,) = r#"
        SELECT COUNT(*)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(
        count_after,
        count_before + 1,
        "After inserting 1 laptop with 1 tag, count should increase by 1"
    );
}

#[rstest]
fn test_join_aggregate_after_delete(mut conn: PgConnection) {
    setup_join_tables(&mut conn);

    let (count_before,) = r#"
        SELECT COUNT(*)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    // Delete the toy laptop (id=5) — has 2 tags
    "DELETE FROM products WHERE id = 5".execute(&mut conn);

    let (count_after,) = r#"
        SELECT COUNT(*)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(
        count_after,
        count_before - 2,
        "After deleting laptop with 2 tags, count should decrease by 2"
    );
}

/// Regression test for cross-table NOT predicate being silently dropped.
///
/// `NOT (a.name @@@ 'bob' AND b.name @@@ 'bob')` spans both tables and
/// cannot be pushed to individual table scans. The DataFusion aggregate path
/// must apply it as a post-join filter; without that, the count is too high.
///
/// Uses enough rows (~10 per table, matching the CI `generated_joins_small`
/// setup) so that the Aggregate Scan naturally wins the cost competition
/// with default GUCs.
#[rstest]
fn test_join_aggregate_cross_table_not_predicate(mut conn: PgConnection) {
    r#"
    CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name TEXT
    );
    CREATE TABLE items (
        id SERIAL PRIMARY KEY,
        name TEXT
    );
    -- First row is a deterministic 'bob'/'bob' pair; the rest are random.
    INSERT INTO users (name) VALUES ('bob');
    INSERT INTO users (name)
        SELECT (ARRAY['alice','charlie','dave','eve','frank','grace','heidi','ivan','judy'])
               [floor(random()*9+1)::int]
        FROM generate_series(1, 10);
    INSERT INTO items (name) VALUES ('bob');
    INSERT INTO items (name)
        SELECT (ARRAY['apple','banana','cherry','date','elderberry','fig','grape','honeydew','kiwi'])
               [floor(random()*9+1)::int]
        FROM generate_series(1, 10);
    CREATE INDEX users_idx ON users USING bm25 (id, name)
    WITH (key_field='id', text_fields='{"name": {"tokenizer": {"type": "keyword"}, "fast": true}}');
    CREATE INDEX items_idx ON items USING bm25 (id, name)
    WITH (key_field='id', text_fields='{"name": {"tokenizer": {"type": "keyword"}, "fast": true}}');
    "#
    .execute(&mut conn);

    // Get the correct count from Postgres with all custom scans off.
    "SET paradedb.enable_custom_scan TO off".execute(&mut conn);
    "SET paradedb.enable_aggregate_custom_scan TO off".execute(&mut conn);

    let (pg_count,) = r#"
        SELECT COUNT(*)
        FROM users u JOIN items i ON u.id = i.id
        WHERE NOT ((u.name = 'bob') AND (i.name = 'bob'))
    "#
    .fetch_one::<(i64,)>(&mut conn);

    // Enable aggregate scan (the only custom scan active).
    "SET paradedb.enable_aggregate_custom_scan TO on".execute(&mut conn);

    // Verify the Aggregate Scan is chosen.
    let explain_lines: Vec<(String,)> = r#"
        EXPLAIN SELECT COUNT(*)
        FROM users u JOIN items i ON u.id = i.id
        WHERE NOT ((u.name @@@ 'bob') AND (i.name @@@ 'bob'))
    "#
    .fetch::<(String,)>(&mut conn);
    let explain = explain_lines
        .iter()
        .map(|(s,)| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        explain.contains("ParadeDB Aggregate Scan"),
        "Expected DataFusion Aggregate Scan to be chosen.\nEXPLAIN:\n{explain}"
    );

    // Verify the result matches Postgres.
    let (bm25_count,) = r#"
        SELECT COUNT(*)
        FROM users u JOIN items i ON u.id = i.id
        WHERE NOT ((u.name @@@ 'bob') AND (i.name @@@ 'bob'))
    "#
    .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(
        pg_count, bm25_count,
        "Cross-table NOT predicate must not be dropped. \
         Postgres={pg_count}, DataFusion aggregate={bm25_count}"
    );
}

// =====================================================================
// HAVING clause tests
// =====================================================================

#[rstest]
fn test_join_aggregate_having_count(mut conn: PgConnection) {
    setup_join_tables(&mut conn);

    // With DataFusion (ON): HAVING COUNT(*) > 2 should filter groups
    let df_rows: Vec<(String, i64)> = r#"
        SELECT p.category, COUNT(*)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop OR shoes OR jacket OR toy'
        GROUP BY p.category
        HAVING COUNT(*) > 2
        ORDER BY p.category
    "#
    .fetch(&mut conn);

    // With Postgres native (OFF)
    "SET paradedb.enable_aggregate_custom_scan TO off".execute(&mut conn);
    let pg_rows: Vec<(String, i64)> = r#"
        SELECT p.category, COUNT(*)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop OR shoes OR jacket OR toy'
        GROUP BY p.category
        HAVING COUNT(*) > 2
        ORDER BY p.category
    "#
    .fetch(&mut conn);

    assert_eq!(
        df_rows.len(),
        pg_rows.len(),
        "HAVING COUNT(*) > 2: row count mismatch: DataFusion={} vs Postgres={}",
        df_rows.len(),
        pg_rows.len()
    );
    for (df, pg) in df_rows.iter().zip(pg_rows.iter()) {
        assert_eq!(df.0, pg.0, "HAVING: category mismatch");
        assert_eq!(df.1, pg.1, "HAVING: count mismatch for {}", df.0);
    }
}

#[rstest]
fn test_join_aggregate_having_sum(mut conn: PgConnection) {
    setup_join_tables(&mut conn);

    // HAVING SUM(price) > 500 with DataFusion
    let df_rows: Vec<(String, i64, f64)> = r#"
        SELECT p.category, COUNT(*), SUM(p.price)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop OR shoes OR jacket OR toy'
        GROUP BY p.category
        HAVING SUM(p.price) > 500
        ORDER BY p.category
    "#
    .fetch(&mut conn);

    // With Postgres native
    "SET paradedb.enable_aggregate_custom_scan TO off".execute(&mut conn);
    let pg_rows: Vec<(String, i64, f64)> = r#"
        SELECT p.category, COUNT(*), SUM(p.price)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop OR shoes OR jacket OR toy'
        GROUP BY p.category
        HAVING SUM(p.price) > 500
        ORDER BY p.category
    "#
    .fetch(&mut conn);

    assert_eq!(
        df_rows.len(),
        pg_rows.len(),
        "HAVING SUM > 500: row count mismatch"
    );
    for (df, pg) in df_rows.iter().zip(pg_rows.iter()) {
        assert_eq!(df.0, pg.0, "category mismatch");
        assert_eq!(df.1, pg.1, "count mismatch for {}", df.0);
        assert!(
            (df.2 - pg.2).abs() < 0.01,
            "SUM mismatch for {}: df={} pg={}",
            df.0,
            df.2,
            pg.2
        );
    }
}

// =====================================================================
// Negative / fallback tests — verify graceful fallback to Postgres native
// =====================================================================

/// Helper to set up a third table (reviews) for 3-table join tests.
fn setup_reviews_table(conn: &mut PgConnection) {
    r#"
    CREATE TABLE reviews (
        id SERIAL PRIMARY KEY,
        product_id INTEGER,
        score INTEGER
    );
    INSERT INTO reviews (product_id, score) VALUES (1, 5), (1, 4), (2, 3), (3, 4), (4, 3);
    CREATE INDEX reviews_idx ON reviews
    USING bm25 (id, product_id, score)
    WITH (
        key_field='id',
        numeric_fields='{"product_id": {"fast": true}, "score": {"fast": true}}'
    );
    "#
    .execute(conn);
}

#[rstest]
fn test_join_aggregate_3table_count(mut conn: PgConnection) {
    setup_join_tables(&mut conn);
    setup_reviews_table(&mut conn);

    // 3-table join COUNT(*) via DataFusion
    let (df_count,) = r#"
        SELECT COUNT(*)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        JOIN reviews r ON p.id = r.product_id
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO off".execute(&mut conn);
    let (pg_count,) = r#"
        SELECT COUNT(*)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        JOIN reviews r ON p.id = r.product_id
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(
        df_count, pg_count,
        "3-table join COUNT(*): DataFusion={df_count} vs Postgres={pg_count}"
    );
}

#[rstest]
fn test_join_aggregate_3table_group_by(mut conn: PgConnection) {
    setup_join_tables(&mut conn);
    setup_reviews_table(&mut conn);

    // 3-table join with GROUP BY + multiple aggregates
    // MAX returns the base column type (INT4), so use i32 for it
    let df_rows: Vec<(String, i64, i64, i32)> = r#"
        SELECT p.category, COUNT(*), SUM(r.score), MAX(r.score)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        JOIN reviews r ON p.id = r.product_id
        WHERE p.description @@@ 'laptop OR shoes OR jacket'
        GROUP BY p.category
        ORDER BY p.category
    "#
    .fetch(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO off".execute(&mut conn);
    let pg_rows: Vec<(String, i64, i64, i32)> = r#"
        SELECT p.category, COUNT(*), SUM(r.score), MAX(r.score)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        JOIN reviews r ON p.id = r.product_id
        WHERE p.description @@@ 'laptop OR shoes OR jacket'
        GROUP BY p.category
        ORDER BY p.category
    "#
    .fetch(&mut conn);

    assert_eq!(
        df_rows.len(),
        pg_rows.len(),
        "3-table GROUP BY: row count mismatch"
    );
    for (df, pg) in df_rows.iter().zip(pg_rows.iter()) {
        assert_eq!(df, pg, "3-table GROUP BY mismatch for category {}", df.0);
    }
}

#[rstest]
fn test_join_aggregate_3table_having(mut conn: PgConnection) {
    setup_join_tables(&mut conn);
    setup_reviews_table(&mut conn);

    // 3-table join with HAVING
    let df_rows: Vec<(String, i64)> = r#"
        SELECT p.category, COUNT(*)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        JOIN reviews r ON p.id = r.product_id
        WHERE p.description @@@ 'laptop OR shoes OR jacket'
        GROUP BY p.category
        HAVING COUNT(*) > 2
        ORDER BY p.category
    "#
    .fetch(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO off".execute(&mut conn);
    let pg_rows: Vec<(String, i64)> = r#"
        SELECT p.category, COUNT(*)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        JOIN reviews r ON p.id = r.product_id
        WHERE p.description @@@ 'laptop OR shoes OR jacket'
        GROUP BY p.category
        HAVING COUNT(*) > 2
        ORDER BY p.category
    "#
    .fetch(&mut conn);

    assert_eq!(
        df_rows.len(),
        pg_rows.len(),
        "3-table HAVING: row count mismatch"
    );
    for (df, pg) in df_rows.iter().zip(pg_rows.iter()) {
        assert_eq!(df, pg, "3-table HAVING mismatch for {}", df.0);
    }
}

#[rstest]
fn test_join_aggregate_4table(mut conn: PgConnection) {
    setup_join_tables(&mut conn);
    setup_reviews_table(&mut conn);

    // Add a 4th table (suppliers)
    r#"
    CREATE TABLE suppliers (
        id SERIAL PRIMARY KEY,
        product_id INTEGER,
        supplier_name TEXT
    );
    INSERT INTO suppliers (product_id, supplier_name) VALUES
        (1, 'TechCorp'), (2, 'GameInc'), (3, 'SportCo');
    CREATE INDEX suppliers_idx ON suppliers
    USING bm25 (id, product_id, supplier_name)
    WITH (
        key_field='id',
        numeric_fields='{"product_id": {"fast": true}}',
        text_fields='{"supplier_name": {"fast": true}}'
    );
    "#
    .execute(&mut conn);

    // 4-table join via DataFusion
    let df_rows: Vec<(String, i64, i64)> = r#"
        SELECT p.category, COUNT(*), SUM(r.score)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        JOIN reviews r ON p.id = r.product_id
        JOIN suppliers s ON p.id = s.product_id
        WHERE p.description @@@ 'laptop OR shoes'
        GROUP BY p.category
        ORDER BY p.category
    "#
    .fetch(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO off".execute(&mut conn);
    let pg_rows: Vec<(String, i64, i64)> = r#"
        SELECT p.category, COUNT(*), SUM(r.score)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        JOIN reviews r ON p.id = r.product_id
        JOIN suppliers s ON p.id = s.product_id
        WHERE p.description @@@ 'laptop OR shoes'
        GROUP BY p.category
        ORDER BY p.category
    "#
    .fetch(&mut conn);

    assert_eq!(
        df_rows.len(),
        pg_rows.len(),
        "4-table join: row count mismatch"
    );
    for (df, pg) in df_rows.iter().zip(pg_rows.iter()) {
        assert_eq!(df, pg, "4-table join mismatch for {}", df.0);
    }
}

#[rstest]
fn test_join_aggregate_sum_distinct(mut conn: PgConnection) {
    setup_join_tables(&mut conn);

    // SUM(DISTINCT) on join should use DataFusion and match Postgres
    let (df_sum,) = r#"
        SELECT SUM(DISTINCT t.product_id)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO off".execute(&mut conn);
    let (pg_sum,) = r#"
        SELECT SUM(DISTINCT t.product_id)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(
        df_sum, pg_sum,
        "SUM(DISTINCT): DataFusion={df_sum} vs Postgres={pg_sum}"
    );
}

#[rstest]
fn test_join_aggregate_cross_join_falls_back(mut conn: PgConnection) {
    setup_join_tables(&mut conn);

    // CROSS JOIN — should fall back to Postgres native
    let (df_count,) = r#"
        SELECT COUNT(*)
        FROM products p
        CROSS JOIN tags t
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO off".execute(&mut conn);
    let (pg_count,) = r#"
        SELECT COUNT(*)
        FROM products p
        CROSS JOIN tags t
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(
        df_count, pg_count,
        "CROSS JOIN should produce same count whether DataFusion is ON or OFF"
    );
}
