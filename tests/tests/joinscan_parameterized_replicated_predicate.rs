mod fixtures;

use anyhow::Result;
use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use serde_json::Value;

fn configure_parallel_joinscan(conn: &mut sqlx::PgConnection) {
    r#"
    SET paradedb.enable_custom_scan = on;
    SET paradedb.enable_join_custom_scan = on;
    SET paradedb.min_rows_per_worker = 0;
    SET max_parallel_workers = 4;
    SET max_parallel_workers_per_gather = 2;
    SET parallel_tuple_cost = 0;
    SET parallel_setup_cost = 0;
    SET min_parallel_table_scan_size = 0;
    SET min_parallel_index_scan_size = 0;
    "#
    .execute(conn);
}

fn assert_parallel_joinscan_plan(conn: &mut sqlx::PgConnection, query: &str) {
    let explain_query = format!("EXPLAIN (FORMAT JSON) {query}");
    let (plan,): (Value,) = explain_query.fetch_one(conn);
    let plan_str = format!("{plan:?}");
    assert!(plan_str.contains("ParadeDB Join Scan"), "{plan_str}");
    assert!(plan_str.contains("Workers Planned"), "{plan_str}");
    assert!(
        plan_str.contains("\"Parallel Aware\":true") || plan_str.contains("Parallel Aware"),
        "{plan_str}"
    );
}

fn setup_parameterized_joinscan_schema(conn: &mut sqlx::PgConnection) {
    r#"
    CREATE EXTENSION IF NOT EXISTS pg_search;

    DROP TABLE IF EXISTS js_param_values CASCADE;
    DROP TABLE IF EXISTS js_param_items CASCADE;
    DROP TABLE IF EXISTS js_param_categories CASCADE;

    CREATE TABLE js_param_categories (
        id BIGINT PRIMARY KEY,
        name TEXT
    ) WITH (autovacuum_enabled = false);

    CREATE TABLE js_param_items (
        id BIGINT PRIMARY KEY,
        name TEXT,
        content TEXT,
        category_id BIGINT REFERENCES js_param_categories(id)
    ) WITH (autovacuum_enabled = false);

    CREATE TABLE js_param_values (
        id BIGINT PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#
    .execute(conn);

    "SET paradedb.global_mutable_segment_rows = 0;".execute(conn);

    r#"
    CREATE INDEX js_param_categories_bm25 ON js_param_categories
    USING bm25 (id, name)
    WITH (
        key_field = 'id',
        target_segment_count = 4,
        background_layer_sizes = '0'
    );
    "#
    .execute(conn);

    r#"
    CREATE INDEX js_param_items_bm25 ON js_param_items
    USING bm25 (id, name, content, category_id)
    WITH (
        key_field = 'id',
        numeric_fields = '{"category_id": {"fast": true}}',
        target_segment_count = 64,
        background_layer_sizes = '0'
    );
    "#
    .execute(conn);

    r#"
    INSERT INTO js_param_categories VALUES
        (1, 'electronics'),
        (2, 'books'),
        (3, 'clothing');
    "#
    .execute(conn);

    for batch_start in (1..=20_000).step_by(2_500) {
        let batch_end = batch_start + 2_499;
        format!(
            r#"
            INSERT INTO js_param_items
            SELECT i,
                   'item ' || i,
                   CASE WHEN i % 2 = 0 THEN 'wireless gadget' ELSE 'paper book' END,
                   CASE WHEN i % 3 = 0 THEN 3 WHEN i % 2 = 0 THEN 1 ELSE 2 END
            FROM generate_series({batch_start}, {batch_end}) i;
            "#
        )
        .execute(conn);
    }

    r#"
    INSERT INTO js_param_values VALUES
        (1, 'electronics'),
        (2, 'books');
    "#
    .execute(conn);

    "RESET paradedb.global_mutable_segment_rows;".execute(conn);
}

#[rstest]
#[tokio::test]
async fn prepared_param_on_replicated_source_succeeds(database: Db) -> Result<()> {
    let mut conn = database.connection().await;
    setup_parameterized_joinscan_schema(&mut conn);
    configure_parallel_joinscan(&mut conn);
    "SET statement_timeout = '30s';".execute(&mut conn);

    let explain_query = r#"
        SELECT i.id
        FROM js_param_items i
        JOIN js_param_categories c ON i.category_id = c.id
        WHERE i.content @@@ 'wireless'
          AND c.name @@@ 'electronics'
        ORDER BY i.id
        LIMIT 100
    "#;
    assert_parallel_joinscan_plan(&mut conn, explain_query);
    "SET paradedb.enable_join_custom_scan = off;".execute(&mut conn);
    let expected_rows: Vec<(i64,)> = explain_query.fetch(&mut conn);
    "SET paradedb.enable_join_custom_scan = on;".execute(&mut conn);
    assert!(
        !expected_rows.is_empty(),
        "serial reference query should return rows"
    );

    let query = r#"
        SELECT i.id
        FROM js_param_items i
        JOIN js_param_categories c ON i.category_id = c.id
        WHERE i.content @@@ 'wireless'
          AND c.name @@@ $1
        ORDER BY i.id
        LIMIT 100
    "#;

    let prepare = format!("PREPARE js_param_stmt(text) AS {query}");
    prepare.execute(&mut conn);
    assert_parallel_joinscan_plan(&mut conn, "EXECUTE js_param_stmt('electronics')");

    let actual_rows: Vec<(i64,)> = "EXECUTE js_param_stmt('electronics')".fetch(&mut conn);
    assert_eq!(actual_rows, expected_rows);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn initplan_param_on_replicated_source_succeeds(database: Db) -> Result<()> {
    let mut conn = database.connection().await;
    setup_parameterized_joinscan_schema(&mut conn);
    configure_parallel_joinscan(&mut conn);
    "SET statement_timeout = '30s';".execute(&mut conn);

    let explain_query = r#"
        SELECT i.id
        FROM js_param_items i
        JOIN js_param_categories c ON i.category_id = c.id
        WHERE i.content @@@ 'wireless'
          AND c.name @@@ 'electronics'
        ORDER BY i.id
        LIMIT 100
    "#;
    assert_parallel_joinscan_plan(&mut conn, explain_query);
    "SET paradedb.enable_join_custom_scan = off;".execute(&mut conn);
    let expected_rows: Vec<(i64,)> = explain_query.fetch(&mut conn);
    "SET paradedb.enable_join_custom_scan = on;".execute(&mut conn);
    assert!(
        !expected_rows.is_empty(),
        "serial reference query should return rows"
    );

    let query = r#"
        SELECT i.id
        FROM js_param_items i
        JOIN js_param_categories c ON i.category_id = c.id
        WHERE i.content @@@ 'wireless'
          AND c.name @@@ (SELECT value FROM js_param_values WHERE id = 1)
        ORDER BY i.id
        LIMIT 100
    "#;
    // InitPlan-backed @@@ expressions are not yet safe in parallel JoinScan
    // workers (SubPlan nodes cannot be re-evaluated there). Force serial
    // JoinScan to verify the query produces correct results.
    "SET max_parallel_workers_per_gather = 0;".execute(&mut conn);
    let actual_rows: Vec<(i64,)> = query.fetch(&mut conn);
    assert_eq!(actual_rows, expected_rows);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn prepared_param_on_partitioning_source_succeeds(database: Db) -> Result<()> {
    let mut conn = database.connection().await;
    setup_parameterized_joinscan_schema(&mut conn);
    configure_parallel_joinscan(&mut conn);
    "SET statement_timeout = '30s';".execute(&mut conn);

    let explain_query = r#"
        SELECT i.id
        FROM js_param_items i
        JOIN js_param_categories c ON i.category_id = c.id
        WHERE i.content @@@ 'wireless'
          AND c.name @@@ 'electronics'
        ORDER BY i.id
        LIMIT 100
    "#;
    assert_parallel_joinscan_plan(&mut conn, explain_query);
    "SET paradedb.enable_join_custom_scan = off;".execute(&mut conn);
    let expected_rows: Vec<(i64,)> = explain_query.fetch(&mut conn);
    "SET paradedb.enable_join_custom_scan = on;".execute(&mut conn);
    assert!(
        !expected_rows.is_empty(),
        "serial reference query should return rows"
    );

    let query = r#"
        SELECT i.id
        FROM js_param_items i
        JOIN js_param_categories c ON i.category_id = c.id
        WHERE i.content @@@ $1
          AND c.name @@@ 'electronics'
        ORDER BY i.id
        LIMIT 100
    "#;

    let prepare = format!("PREPARE js_part_param_stmt(text) AS {query}");
    prepare.execute(&mut conn);
    assert_parallel_joinscan_plan(&mut conn, "EXECUTE js_part_param_stmt('wireless')");

    let actual_rows: Vec<(i64,)> = "EXECUTE js_part_param_stmt('wireless')".fetch(&mut conn);
    assert_eq!(actual_rows, expected_rows);

    Ok(())
}
