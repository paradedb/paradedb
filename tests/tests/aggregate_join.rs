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

/// Regression: when JoinScan (CustomPath) is cheapest_total_path, the aggregate
/// scan's `has_non_equi_join_quals` could not see `joinrestrictinfo` and silently
/// dropped cross-table OR predicates, returning wrong COUNT(*).
///
/// We force JoinScan to be preferred by disabling seqscan/indexscan, then verify
/// that a cross-table OR still produces the same count with aggregate scan ON
/// vs OFF.
#[rstest]
fn test_cross_table_or_with_joinscan_and_aggregate_scan(mut conn: PgConnection) {
    setup_join_tables(&mut conn);

    // Force JoinScan to be cheapest_total_path by making native scans expensive
    r#"
        SET enable_seqscan TO off;
        SET enable_indexscan TO off;
        SET paradedb.enable_join_custom_scan TO on;
        SET paradedb.enable_aggregate_custom_scan TO on;
    "#
    .execute(&mut conn);

    // Cross-table OR: description @@@ 'laptop' matches products 1,2,5 (6 joined rows)
    // tag_name @@@ 'fitness' matches tag 5 → product 3 (1 joined row)
    // Total expected: 7
    let (count_agg_on,) = r#"
        SELECT COUNT(*)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop' OR t.tag_name @@@ 'fitness'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    // Same query with aggregate scan OFF (baseline via native Postgres execution)
    "SET paradedb.enable_aggregate_custom_scan TO off".execute(&mut conn);
    let (count_agg_off,) = r#"
        SELECT COUNT(*)
        FROM products p
        JOIN tags t ON p.id = t.product_id
        WHERE p.description @@@ 'laptop' OR t.tag_name @@@ 'fitness'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(
        count_agg_on, count_agg_off,
        "Cross-table OR: aggregate scan ON ({count_agg_on}) must match OFF ({count_agg_off})"
    );
    assert_eq!(count_agg_on, 7, "Expected 7 rows (6 laptops + 1 fitness)");
}
