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

#![allow(unused_variables, unused_imports)]
mod fixtures;

use std::path::PathBuf;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

fn fmt_err<T: std::error::Error>(err: T) -> String {
    format!("unexpected error, received: {}", err)
}

#[rstest]
fn invalid_create_index(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'public')"
        .execute(&mut conn);

    match r#"CREATE INDEX index_config_index ON index_config
        USING bm25 (id) "#
        .execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with no key_field"),
        Err(err) => assert_eq!(
            err.to_string(),
            "error returned from database: key_field WITH option should be configured"
        ),
    };
}

#[rstest]
fn prevent_duplicate(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description) WITH (key_field='id')"#
        .execute(&mut conn);

    match r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description) WITH (key_field='id')"#
        .execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with relation already exists"),
        Err(err) => assert!(
            err.to_string().contains("already exists"),
            "{}",
            fmt_err(err)
        ),
    };
}

#[rstest]
async fn drop_column(mut conn: PgConnection) {
    r#"
    CREATE TABLE f_table (
        id SERIAL PRIMARY KEY,
        category TEXT
    );

    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        fkey INTEGER REFERENCES f_table ON UPDATE CASCADE ON DELETE RESTRICT,
        fulltext TEXT
    );

    INSERT INTO f_table (category) VALUES ('cat_a'), ('cat_b'), ('cat_c');
    INSERT INTO test_table (fkey, fulltext) VALUES (1, 'abc'), (1, 'def'), (2, 'ghi'), (3, 'jkl');
    "#
    .execute(&mut conn);

    r#"CREATE INDEX test_index ON test_table
        USING bm25 (id, fulltext) WITH (key_field='id')"#
        .execute(&mut conn);

    r#"DROP INDEX test_index CASCADE;
    ALTER TABLE test_table DROP COLUMN fkey;

    CREATE INDEX test_index ON test_table
        USING bm25 (id, fulltext) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('test_index')".fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("fulltext".into(), "Str".into()));
    assert_eq!(rows[2], ("id".into(), "I64".into()));
}

#[rstest]
fn default_text_field(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("description".into(), "Str".into()));
    assert_eq!(rows[2], ("id".into(), "I64".into()));
}

#[rstest]
fn text_field_with_options(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description)
        WITH (key_field='id', text_fields='{"description": {"tokenizer": {"type": "en_stem", "normalizer": "raw"}, "record": "freq", "fast": true}}');
"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("description".into(), "Str".into()));
    assert_eq!(rows[2], ("id".into(), "I64".into()));
}

#[rstest]
fn multiple_text_fields(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config

        USING bm25 (id, description, category)
        WITH (
            key_field='id',
            text_fields='{"description": {"tokenizer": {"type": "en_stem", "normalizer": "raw"}, "record": "freq", "fast": true}}'
        );
        "#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("category".into(), "Str".into()));
    assert_eq!(rows[1], ("ctid".into(), "U64".into()));
    assert_eq!(rows[2], ("description".into(), "Str".into()));
    assert_eq!(rows[3], ("id".into(), "I64".into()));
}

#[rstest]
fn default_numeric_field(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, rating) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("rating".into(), "I64".into()));
}

#[rstest]
fn numeric_field_with_options(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, rating) WITH (key_field='id', numeric_fields='{"rating": {"fast": true}}')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("rating".into(), "I64".into()));
}

#[rstest]
fn default_boolean_field(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, in_stock) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("in_stock".into(), "Bool".into()));
}

#[rstest]
fn boolean_field_with_options(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, in_stock) WITH (key_field='id', boolean_fields='{"in_stock": {"fast": false}}')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("in_stock".into(), "Bool".into()));
}

#[rstest]
fn default_json_field(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, metadata) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("metadata".into(), "JsonObject".into()));
}

#[rstest]
fn json_field_with_options(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, metadata)
        WITH (
            key_field='id',
            json_fields='{"metadata": {"fast": true, "expand_dots": false, "tokenizer": {"type": "raw", "normalizer": "raw"}}}'
        )"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("metadata".into(), "JsonObject".into()));
}

#[rstest]
fn default_datetime_field(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, created_at, last_updated_date) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("created_at".into(), "Date".into()));
    assert_eq!(rows[1], ("ctid".into(), "U64".into()));
    assert_eq!(rows[2], ("id".into(), "I64".into()));
    assert_eq!(rows[3], ("last_updated_date".into(), "Date".into()));
}

#[rstest]
fn datetime_field_with_options(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, created_at, last_updated_date)
        WITH (key_field='id', datetime_fields='{"created_at": {"fast": true}, "last_updated_date": {"fast": false}}')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("created_at".into(), "Date".into()));
    assert_eq!(rows[1], ("ctid".into(), "U64".into()));
    assert_eq!(rows[2], ("id".into(), "I64".into()));
    assert_eq!(rows[3], ("last_updated_date".into(), "Date".into()));
}

#[rstest]
fn multiple_fields(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description, category, rating, in_stock, metadata) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("category".into(), "Str".into()));
    assert_eq!(rows[1], ("ctid".into(), "U64".into()));
    assert_eq!(rows[2], ("description".into(), "Str".into()));
    assert_eq!(rows[3], ("id".into(), "I64".into()));
    assert_eq!(rows[4], ("in_stock".into(), "Bool".into()));
    assert_eq!(rows[5], ("metadata".into(), "JsonObject".into()));
    assert_eq!(rows[6], ("rating".into(), "I64".into()));
}

#[rstest]
fn missing_schema_index(mut conn: PgConnection) {
    match "SELECT paradedb.schema('paradedb.missing_bm25_index')".fetch_result::<(i64,)>(&mut conn)
    {
        Err(err) => assert!(err
            .to_string()
            .contains(r#"relation "paradedb.missing_bm25_index" does not exist"#)),
        _ => panic!("non-existing index should throw an error"),
    }
}

#[rstest]
fn null_values(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    "INSERT INTO paradedb.index_config (description, category, rating) VALUES ('Null Item 1', NULL, NULL), ('Null Item 2', NULL, 2)"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description, category, rating, in_stock, metadata) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, Option<String>, Option<i32>)> = "
        SELECT description, category, rating
        FROM paradedb.index_config WHERE index_config @@@ 'description:\"Null Item\"'
        ORDER BY id"
        .fetch(&mut conn);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], ("Null Item 1".into(), None, None));
    assert_eq!(rows[1], ("Null Item 2".into(), None, Some(2)));

    let rows: Vec<(bool,)> =
        "SELECT in_stock FROM paradedb.index_config WHERE index_config @@@ 'in_stock:false'"
            .fetch(&mut conn);

    assert_eq!(rows.len(), 13);
}

#[rstest]
fn null_key_field_build(mut conn: PgConnection) {
    "CREATE TABLE paradedb.index_config(id INTEGER, description TEXT)".execute(&mut conn);
    "INSERT INTO paradedb.index_config VALUES (NULL, 'Null Item 1'), (2, 'Null Item 2')"
        .execute(&mut conn);

    match r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description) WITH (key_field='id')"#
        .execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with null key_field"),
        Err(err) => assert_eq!(
            err.to_string(),
            "error returned from database: error creating index entries for index 'index_config_index': key_field column 'id' cannot be NULL"
        ),
    };
}

#[rstest]
fn null_key_field_insert(mut conn: PgConnection) {
    "CREATE TABLE paradedb.index_config(id INTEGER, description TEXT)".execute(&mut conn);
    "INSERT INTO paradedb.index_config VALUES (1, 'Null Item 1'), (2, 'Null Item 2')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description) WITH (key_field='id')"#
        .execute(&mut conn);

    match "INSERT INTO paradedb.index_config VALUES (NULL, 'Null Item 3')".execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with null key_field"),
        Err(err) => assert_eq!(
            err.to_string(),
            "error returned from database: error creating index entries for index 'index_config_index': key_field column 'id' cannot be NULL"
        ),
    };
}

#[rstest]
fn column_name_camelcase(mut conn: PgConnection) {
    "CREATE TABLE paradedb.index_config(\"IdName\" INTEGER, \"ColumnName\" TEXT)"
        .execute(&mut conn);
    "INSERT INTO paradedb.index_config VALUES (1, 'Plastic Keyboard'), (2, 'Bluetooth Headphones')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 ("IdName", "ColumnName") WITH (key_field='IdName')"#
        .execute(&mut conn);

    let rows: Vec<(i32, String)> =
        "SELECT * FROM paradedb.index_config WHERE index_config @@@ 'ColumnName:keyboard'"
            .fetch(&mut conn);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], (1, "Plastic Keyboard".into()));
}

#[rstest]
fn multi_index_insert_in_transaction(mut conn: PgConnection) {
    "CREATE TABLE paradedb.index_config1(id INTEGER, description TEXT)".execute(&mut conn);
    "CREATE TABLE paradedb.index_config2(id INTEGER, description TEXT)".execute(&mut conn);
    r#"CREATE INDEX index_config1_index ON paradedb.index_config1
        USING bm25 (id, description) WITH (key_field='id')"#
        .execute(&mut conn);
    r#"CREATE INDEX index_config2_index ON paradedb.index_config2
        USING bm25 (id, description) WITH (key_field='id')"#
        .execute(&mut conn);
    "BEGIN".execute(&mut conn);
    "INSERT INTO paradedb.index_config1 VALUES (1, 'Item 1'), (2, 'Item 2')".execute(&mut conn);
    "INSERT INTO paradedb.index_config2 VALUES (1, 'Item 1'), (2, 'Item 2')".execute(&mut conn);
    "COMMIT".execute(&mut conn);

    let rows: Vec<(i32, String)> =
        "SELECT * FROM paradedb.index_config1 WHERE index_config1 @@@ 'description:item'"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(i32, String)> =
        "SELECT * FROM paradedb.index_config2 WHERE index_config2 @@@ 'description:item'"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn partitioned_schema(mut conn: PgConnection) {
    PartitionedTable::setup().execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('sales_index') ORDER BY name"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("amount".into(), "F64".into()));
    assert_eq!(rows[1], ("ctid".into(), "U64".into()));
    assert_eq!(rows[2], ("description".into(), "Str".into()));
    assert_eq!(rows[3], ("id".into(), "I64".into()));
    assert_eq!(rows[4], ("sale_date".into(), "Date".into()));
}

#[rstest]
fn partitioned_info(mut conn: PgConnection) {
    PartitionedTable::setup().execute(&mut conn);

    // Insert rows into both partitions.
    r#"
        INSERT INTO sales (sale_date, amount, description) VALUES
        ('2023-01-10', 150.00, 'Ergonomic metal keyboard'),
        ('2023-04-01', 250.00, 'Modern wall clock');
    "#
    .execute(&mut conn);

    // And validate that we see at least one segment for each.
    let segments_per_partition: Vec<(String, i64)> = "
        SELECT index_name, COUNT(*) FROM paradedb.index_info('sales_index') GROUP BY index_name
    "
    .fetch(&mut conn);
    assert_eq!(segments_per_partition.len(), 2);
    for (index_name, segment_count) in segments_per_partition {
        assert!(
            segment_count > 0,
            "Got {segment_count} for index partition {index_name}"
        );
    }

    // Just cover `index_layer_info`.
    let segments_per_partition: Vec<(String, String, i64)> =
        "SELECT relname::text, layer_size, count FROM paradedb.index_layer_info".fetch(&mut conn);
    assert!(!segments_per_partition.is_empty());
}

#[rstest]
fn partitioned_all(mut conn: PgConnection) {
    PartitionedTable::setup().execute(&mut conn);

    let schema_rows: Vec<(String, String)> =
        "SELECT id from sales WHERE id @@@ paradedb.all()".fetch(&mut conn);
    assert_eq!(schema_rows.len(), 0);

    r#"
        INSERT INTO sales (sale_date, amount, description) VALUES
        ('2023-01-10', 150.00, 'Ergonomic metal keyboard'),
        ('2023-04-01', 250.00, 'Modern wall clock');
    "#
    .execute(&mut conn);

    let schema_rows: Vec<(i32,)> =
        "SELECT id from sales WHERE id @@@ paradedb.all()".fetch(&mut conn);
    assert_eq!(schema_rows.len(), 2);
}

#[rstest]
fn partitioned_query(mut conn: PgConnection) {
    // Set up the partitioned table with two partitions and a BM25 index.
    PartitionedTable::setup().execute(&mut conn);

    // Insert some data.
    r#"
        INSERT INTO sales (sale_date, amount, description) VALUES
        ('2023-01-10', 150.00, 'Ergonomic metal keyboard'),
        ('2023-01-15', 200.00, 'Plastic keyboard'),
        ('2023-02-05', 300.00, 'Sleek running shoes'),
        ('2023-03-12', 175.50, 'Bluetooth speaker'),
        ('2023-03-25', 225.75, 'Artistic ceramic vase');

        INSERT INTO sales (sale_date, amount, description) VALUES
        ('2023-04-01', 250.00, 'Modern wall clock'),
        ('2023-04-18', 180.00, 'Designer wall paintings'),
        ('2023-05-09', 320.00, 'Handcrafted wooden frame');
    "#
    .execute(&mut conn);

    // Test: Verify data is partitioned correctly by querying each partition
    let rows_q1: Vec<(i32, String, String)> = r#"
        SELECT id, description, sale_date::text FROM sales_2023_q1
    "#
    .fetch(&mut conn);
    assert_eq!(rows_q1.len(), 5, "Expected 5 rows in Q1 partition");

    let rows_q2: Vec<(i32, String, String)> = r#"
        SELECT id, description, sale_date::text FROM sales_2023_q2
    "#
    .fetch(&mut conn);
    assert_eq!(rows_q2.len(), 3, "Expected 3 rows in Q2 partition");

    // Test: Search using the bm25 index against both the parent and child tables.
    for table in ["sales", "sales_2023_q1"] {
        let search_results: Vec<(i32, String)> = format!(
            r#"
            SELECT id, description FROM {table} WHERE id @@@ 'description:keyboard'
            "#
        )
        .fetch(&mut conn);
        assert_eq!(search_results.len(), 2, "Expected 2 items with 'keyboard'");
    }

    // Test: Retrieve items by a numeric range (amount field) and verify bm25 compatibility
    for (table, expected) in [("sales", 5), ("sales_2023_q1", 3)] {
        let amount_results: Vec<(i32, String, f32)> = format!(
            r#"
            SELECT id, description, amount FROM {table}
            WHERE amount @@@ '[175 TO 250]'
            ORDER BY amount ASC
            "#
        )
        .fetch(&mut conn);
        assert_eq!(
            amount_results.len(),
            expected,
            "Expected {expected} items with amount in range 175-250"
        );
    }
}

#[rstest]
fn partitioned_uses_custom_scan(mut conn: PgConnection) {
    PartitionedTable::setup().execute(&mut conn);

    r#"
        INSERT INTO sales (sale_date, amount, description) VALUES
        ('2023-01-10', 150.00, 'Ergonomic metal keyboard'),
        ('2023-04-01', 250.00, 'Modern wall clock');
    "#
    .execute(&mut conn);

    "SET max_parallel_workers TO 0;".execute(&mut conn);

    // Without the partition key.
    let (plan,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT count(*)
        FROM sales
        WHERE id @@@ '1';
        "#
    .fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");

    let per_partition_plans = plan
        .pointer("/0/Plan/Plans/0/Plans")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(
        per_partition_plans.len(),
        2,
        "Expected 2 partitions to be scanned."
    );
    for per_partition_plan in per_partition_plans {
        pretty_assertions::assert_eq!(
            per_partition_plan.get("Node Type"),
            Some(&Value::String(String::from("Custom Scan")))
        );
    }

    // With the partition key: we expect the partition to be filtered, and for
    // us to apply pushdown.
    let (plan,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT count(*)
        FROM sales
        WHERE description @@@ 'keyboard' and sale_date = '2023-01-10';
        "#
    .fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");

    let per_partition_plans = plan.pointer("/0/Plan/Plans").unwrap().as_array().unwrap();
    assert_eq!(
        per_partition_plans.len(),
        1,
        "Expected 1 partition to be scanned."
    );
    for per_partition_plan in per_partition_plans {
        pretty_assertions::assert_eq!(
            per_partition_plan.get("Node Type"),
            Some(&Value::String(String::from("Custom Scan")))
        );
        let query = per_partition_plan
            .get("Human Readable Query")
            .unwrap()
            .to_string();
        assert!(
            query.to_string().contains("sale_date:2023-01-10"),
            "Expected sale_date to be pushed down into query: {query:?}",
        );
    }
}

#[rstest]
fn custom_enum_term(mut conn: PgConnection) {
    r#"
    CREATE TYPE color AS ENUM ('red', 'green', 'blue');
    CREATE TABLE paradedb.index_config(id INTEGER, description TEXT, color color);
    INSERT INTO paradedb.index_config VALUES (1, 'Item 1', 'red'), (2, 'Item 2', 'green');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX index_config_index ON paradedb.index_config
    USING bm25 (id, description, color)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> =
        "SELECT id, description FROM paradedb.index_config WHERE id @@@ paradedb.term('color', 'red'::color)".fetch(&mut conn);

    assert_eq!(rows, vec![(1, "Item 1".into())]);
}

#[rstest]
fn custom_enum_parse(mut conn: PgConnection) {
    r#"
    CREATE TYPE color AS ENUM ('red', 'green', 'blue');
    CREATE TABLE paradedb.index_config(id INTEGER, description TEXT, color color);
    INSERT INTO paradedb.index_config VALUES (1, 'Item 1', 'red'), (2, 'Item 2', 'green');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX index_config_index ON paradedb.index_config
    USING bm25 (id, description, color)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> =
        "SELECT id, description FROM paradedb.index_config WHERE id @@@ paradedb.parse('color:1.0')".fetch(&mut conn);

    assert_eq!(rows, vec![(1, "Item 1".into())]);
}

#[rstest]
fn long_text_key_field_issue2198(mut conn: PgConnection) {
    "CREATE TABLE issue2198 (id TEXT, value TEXT)".execute(&mut conn);

    "CREATE INDEX idxissue2198 ON issue2198 USING bm25 (id, value) WITH (key_field='id')"
        .execute(&mut conn);

    let long_string = "a".repeat(10000);

    format!("INSERT INTO issue2198(id) VALUES ('{long_string}')").execute(&mut conn);
    let (count,) = format!("SELECT count(*) FROM issue2198 WHERE id @@@ '{long_string}'")
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) =
        format!("SELECT count(*) FROM issue2198 WHERE id @@@ paradedb.term('id', '{long_string}')")
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}

#[rstest]
fn uuid_as_raw_issue2199(mut conn: PgConnection) {
    "CREATE TABLE issue2199 (id SERIAL8 NOT NULL PRIMARY KEY, value uuid);".execute(&mut conn);

    "CREATE INDEX idxissue2199 ON issue2199 USING bm25 (id, value) WITH (key_field='id');"
        .execute(&mut conn);

    let uuid = uuid::Uuid::new_v4();

    format!("INSERT INTO issue2199(value) VALUES ('{uuid}')").execute(&mut conn);
    let (count,) = format!("SELECT count(*) FROM issue2199 WHERE value @@@ '{uuid}'")
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) =
        format!("SELECT count(*) FROM issue2199 WHERE id @@@ paradedb.term('value', '{uuid}')")
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}
