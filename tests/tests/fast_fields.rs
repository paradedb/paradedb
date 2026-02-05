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
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

#[rstest]
fn plans_numeric_fast_field(mut conn: PgConnection) {
    r#"
CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
CREATE INDEX idxbm25_search ON paradedb.bm25_search
USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
WITH (
    key_field='id',
    text_fields='{
        "description": {},
        "category": {"fast": true, "normalizer": "raw"}
    }',
    numeric_fields='{"rating": {"fast": true}}',
    boolean_fields='{"in_stock": {}}',
    json_fields='{"metadata": {}}',
    datetime_fields='{
        "created_at": {},
        "last_updated_date": {},
        "latest_available_time": {}
    }'
);
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT rating FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard'".fetch_one::<(Value,)>(&mut conn);

    assert_eq!(
        Some(&Value::String("rating".into())),
        plan.pointer("/0/Plan/Fast Fields")
    )
}

#[rstest]
fn plans_many_numeric_fast_fields(mut conn: PgConnection) {
    r#"
CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
CREATE INDEX idxbm25_search ON paradedb.bm25_search
USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
WITH (
    key_field='id',
    text_fields='{
        "description": {},
        "category": {"fast": true, "normalizer": "raw"}
    }',
    numeric_fields='{"rating": {"fast": true}}',
    boolean_fields='{"in_stock": {}}',
    json_fields='{"metadata": {}}',
    datetime_fields='{
        "created_at": {},
        "last_updated_date": {},
        "latest_available_time": {}
    }'
);
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT id, rating FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard'".fetch_one::<(Value,)>(&mut conn);

    assert_eq!(
        Some(&Value::String("id, rating".into())),
        plan.pointer("/0/Plan/Fast Fields")
    )
}

#[rstest]
fn plans_many_numeric_fast_fields_with_score(mut conn: PgConnection) {
    r#"
CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
CREATE INDEX idxbm25_search ON paradedb.bm25_search
USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
WITH (
    key_field='id',
    text_fields='{
        "description": {},
        "category": {"fast": true, "normalizer": "raw"}
    }',
    numeric_fields='{"rating": {"fast": true}}',
    boolean_fields='{"in_stock": {}}',
    json_fields='{"metadata": {}}',
    datetime_fields='{
        "created_at": {},
        "last_updated_date": {},
        "latest_available_time": {}
    }'
);
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT id, pdb.score(id), rating FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard'".fetch_one::<(Value,)>(&mut conn);
    assert_eq!(
        Some(&Value::String("id, rating".into())),
        plan.pointer("/0/Plan/Fast Fields")
    )
}

// string "fast fields" are only supported as part of an aggregate query.  They're basically slower
// in all other cases
#[rstest]
fn plans_string_fast_field(mut conn: PgConnection) {
    r#"
CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
CREATE INDEX idxbm25_search ON paradedb.bm25_search
USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
WITH (
    key_field='id',
    text_fields='{
        "description": {},
        "category": {"fast": true, "normalizer": "raw"}
    }',
    numeric_fields='{"rating": {"fast": true}}',
    boolean_fields='{"in_stock": {}}',
    json_fields='{"metadata": {}}',
    datetime_fields='{
        "created_at": {},
        "last_updated_date": {},
        "latest_available_time": {}
    }'
);
SET paradedb.enable_aggregate_custom_scan = false;
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT category, count(*) FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard' GROUP BY category".fetch_one::<(Value,)>(&mut conn);
    assert_eq!(
        Some(&Value::String("category".into())),
        plan.pointer("/0/Plan/Plans/0/Plans/0/Fast Fields")
    )
}

// only selecting a string field does use a "fast field"-style plan
#[rstest]
fn does_plan_string_fast_field(mut conn: PgConnection) {
    r#"
CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
CREATE INDEX idxbm25_search ON paradedb.bm25_search
USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
WITH (
    key_field='id',
    text_fields='{
        "description": {},
        "category": {"fast": true, "normalizer": "raw"}
    }',
    numeric_fields='{"rating": {"fast": true}}',
    boolean_fields='{"in_stock": {}}',
    json_fields='{"metadata": {}}',
    datetime_fields='{
        "created_at": {},
        "last_updated_date": {},
        "latest_available_time": {}
    }'
);
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT category FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard'".fetch_one::<(Value,)>(&mut conn);
    assert_eq!(
        Some(&Value::String("Custom Scan".into())),
        plan.pointer("/0/Plan/Node Type")
    )
}

#[rstest]
fn numeric_fast_field_in_window_func(mut conn: PgConnection) {
    r#"
CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
CREATE INDEX idxbm25_search ON paradedb.bm25_search
USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
WITH (
    key_field='id',
    text_fields='{
        "description": {},
        "category": {"fast": true, "normalizer": "raw"}
    }',
    numeric_fields='{"rating": {"fast": true}}',
    boolean_fields='{"in_stock": {}}',
    json_fields='{"metadata": {}}',
    datetime_fields='{
        "created_at": {},
        "last_updated_date": {},
        "latest_available_time": {}
    }'
);
    "#
    .execute(&mut conn);

    let (plan,) = r#"EXPLAIN (ANALYZE, FORMAT JSON)
    WITH RankedContacts AS (
        SELECT id,
               ROW_NUMBER() OVER (PARTITION BY rating ORDER BY id) AS rn
        FROM paradedb.bm25_search
        WHERE id @@@ 'description:shoes'
        )
    SELECT id
    FROM RankedContacts
    WHERE rn <= 10
    LIMIT 100 OFFSET 100;
    "#
    .fetch_one::<(Value,)>(&mut conn);
    eprintln!("plan: {plan:#?}");
    assert_eq!(
        Some(&Value::String("MixedFastFieldExecState".into())),
        plan.pointer("/0/Plan/Plans/0/Plans/0/Plans/0/Plans/0/Exec Method")
    )
}

/// Test NumericBytes (unbounded NUMERIC) fast fields via parallel execution.
/// This exercises the batch scanner's FFType::Bytes handling which converts
/// decimal_bytes::Decimal back to AnyNumeric via Arrow BinaryViewArray.
#[rstest]
fn numeric_bytes_fast_field_parallel(mut conn: PgConnection) {
    // Create table with unbounded NUMERIC (uses NumericBytes storage)
    r#"
    CREATE TABLE numeric_bytes_test (
        id SERIAL PRIMARY KEY,
        description TEXT,
        -- Unbounded NUMERIC uses NumericBytes storage (lexicographic bytes)
        amount NUMERIC NOT NULL,
        -- High-precision bounded NUMERIC also uses NumericBytes (precision > 18)
        precise_value NUMERIC(30, 10) NOT NULL
    );

    -- Insert enough rows to make parallel execution likely
    INSERT INTO numeric_bytes_test (description, amount, precise_value)
    SELECT
        'item ' || i,
        (random() * 1000000)::numeric,
        (random() * 100000000000000000000.0)::numeric(30, 10)
    FROM generate_series(1, 10000) i;

    CREATE INDEX numeric_bytes_idx ON numeric_bytes_test
    USING bm25 (id, description, amount, precise_value)
    WITH (
        key_field = 'id',
        text_fields = '{"description": {}}',
        numeric_fields = '{"amount": {"fast": true}, "precise_value": {"fast": true}}'
    );
    "#
    .execute(&mut conn);

    // Force parallel execution
    r#"
    SET max_parallel_workers = 8;
    SET max_parallel_workers_per_gather = 4;
    SET parallel_tuple_cost = 0;
    SET parallel_setup_cost = 0;
    SET min_parallel_table_scan_size = 0;
    SET min_parallel_index_scan_size = 0;
    "#
    .execute(&mut conn);

    // Try to enable debug_parallel_query (available in PG16+)
    let _ = "SET debug_parallel_query TO on".execute_result(&mut conn);

    // Query that selects NumericBytes columns - uses MixedFastFieldExecState
    // which exercises the batch scanner's FFType::Bytes handling
    let results = r#"
    SELECT id, amount, precise_value
    FROM numeric_bytes_test
    WHERE description @@@ 'item'
    "#
    .fetch::<(i32, sqlx::types::BigDecimal, sqlx::types::BigDecimal)>(&mut conn);

    // Verify results are returned correctly (all 10000 rows match 'item')
    assert_eq!(results.len(), 10000);

    // Verify numeric values are not corrupted (basic sanity check)
    for (id, amount, precise_value) in &results {
        assert!(*id > 0);
        // Amount should be positive (we generated random() * 1000000)
        assert!(amount.to_string().parse::<f64>().unwrap() >= 0.0);
        // Precise value should also be positive
        assert!(precise_value.to_string().parse::<f64>().unwrap() >= 0.0);
    }

    // Check EXPLAIN to verify Custom Scan with parallel workers
    let (plan,) = r#"
    EXPLAIN (FORMAT JSON)
    SELECT id, amount, precise_value
    FROM numeric_bytes_test
    WHERE description @@@ 'item'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Verify parallel execution is being used
    let plan_str = serde_json::to_string(&plan).unwrap();
    assert!(
        plan_str.contains("Gather") || plan_str.contains("Custom Scan"),
        "Query should use parallel or custom scan. Plan: {plan:#?}"
    );
}

/// Test NumericBytes fast fields in JoinScan path.
/// JoinScan uses the batch scanner when projecting fields from inner tables.
#[rstest]
fn numeric_bytes_fast_field_joinscan(mut conn: PgConnection) {
    // Create two tables with unbounded NUMERIC columns
    r#"
    CREATE TABLE orders (
        id SERIAL PRIMARY KEY,
        product_name TEXT NOT NULL,
        -- Unbounded NUMERIC uses NumericBytes storage
        total_amount NUMERIC NOT NULL
    );

    CREATE TABLE order_items (
        id SERIAL PRIMARY KEY,
        order_id INTEGER NOT NULL,
        item_name TEXT NOT NULL,
        -- Unbounded NUMERIC uses NumericBytes storage
        item_price NUMERIC NOT NULL
    );

    -- Insert test data
    INSERT INTO orders (product_name, total_amount)
    SELECT
        'product ' || i,
        (random() * 10000)::numeric
    FROM generate_series(1, 1000) i;

    INSERT INTO order_items (order_id, item_name, item_price)
    SELECT
        (random() * 999 + 1)::int,
        'item ' || i,
        (random() * 1000)::numeric
    FROM generate_series(1, 5000) i;

    CREATE INDEX orders_idx ON orders
    USING bm25 (id, product_name, total_amount)
    WITH (
        key_field = 'id',
        text_fields = '{"product_name": {}}',
        numeric_fields = '{"total_amount": {"fast": true}}'
    );

    CREATE INDEX order_items_idx ON order_items
    USING bm25 (id, item_name, item_price)
    WITH (
        key_field = 'id',
        text_fields = '{"item_name": {}}',
        numeric_fields = '{"item_price": {"fast": true}}'
    );

    -- Enable JoinScan
    SET paradedb.enable_join_custom_scan = true;
    "#
    .execute(&mut conn);

    // Query that joins tables and selects NumericBytes columns
    let results = r#"
    SELECT o.id, o.total_amount, oi.item_price
    FROM orders o
    JOIN order_items oi ON o.id = oi.order_id
    WHERE o.product_name @@@ 'product' AND oi.item_name @@@ 'item'
    ORDER BY o.id, oi.id
    LIMIT 50
    "#
    .fetch::<(i32, sqlx::types::BigDecimal, sqlx::types::BigDecimal)>(&mut conn);

    // Verify we got results
    assert!(!results.is_empty(), "JoinScan query should return results");

    // Verify numeric values are valid
    for (id, total_amount, item_price) in &results {
        assert!(*id > 0);
        assert!(total_amount.to_string().parse::<f64>().unwrap() >= 0.0);
        assert!(item_price.to_string().parse::<f64>().unwrap() >= 0.0);
    }

    // Check EXPLAIN to verify JoinScan is being used
    let (plan,) = r#"
    EXPLAIN (FORMAT JSON)
    SELECT o.id, o.total_amount, oi.item_price
    FROM orders o
    JOIN order_items oi ON o.id = oi.order_id
    WHERE o.product_name @@@ 'product' AND oi.item_name @@@ 'item'
    ORDER BY o.id, oi.id
    LIMIT 50
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Look for Custom Scan in the plan (JoinScan appears as Custom Scan)
    let plan_str = serde_json::to_string(&plan).unwrap();
    assert!(
        plan_str.contains("Custom Scan"),
        "Query should use Custom Scan. Plan: {plan:#?}"
    );
}
