// Copyright (c) 2023-2024 Retake, Inc.
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

use fixtures::utils::pg_search_index_directory_path;
use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

fn fmt_err<T: std::error::Error>(err: T) -> String {
    format!("unexpected error, received: {}", err)
}

#[rstest]
fn invalid_create_bm25(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'public')"
        .execute(&mut conn);

    match "CALL paradedb.create_bm25(
	    index_name => 'index_config_index',
	    table_name => 'index_config'
    )"
    .execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with no key_field"),
        Err(err) => assert!(err.to_string().contains("no key_field parameter")),
    };

    match "CALL paradedb.create_bm25(
	    index_name => 'index_config_index',
	    table_name => 'index_config',
	    key_field => 'id'
    )"
    .execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with no fields"),
        Err(err) => assert!(err.to_string().contains("specified"), "{}", fmt_err(err)),
    };

    match "CALL paradedb.create_bm25(
	    index_name => 'index_config_index',
	    table_name => 'index_config',
	    key_field => 'id',
	    invalid_field => '{}'		
    )"
    .execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with invalid field"),
        Err(err) => assert!(err.to_string().contains("not exist"), "{}", fmt_err(err)),
    };

    match "CALL paradedb.create_bm25(
	    index_name => 'index_config_index',
	    table_name => 'index_config',
	    key_field => 'id',
	    numeric_fields => paradedb.field('id')		
    )"
    .execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with invalid field"),
        Err(err) => assert_eq!(err.to_string(), "error returned from database: key_field id cannot be included in text_fields, numeric_fields, boolean_fields, json_fields, range_fields, datetime_fields, or fields")
    };
}

#[rstest]
fn prevent_duplicate(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    "CALL paradedb.create_bm25(
        index_name => 'index_config_index',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'id',
        text_fields => paradedb.field('description'))"
        .execute(&mut conn);

    match "CALL paradedb.create_bm25(
        index_name => 'index_config_index',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'id',
        text_fields => paradedb.field('description'))"
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

    r#"
    CALL paradedb.create_bm25(
        index_name => 'test_index',
        schema_name => 'public',
        table_name => 'test_table',
        key_field => 'id',
        text_fields => paradedb.field('fulltext')
    );

    DROP INDEX test_index CASCADE;
    ALTER TABLE test_table DROP COLUMN fkey;
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        index_name => 'test_index',
        schema_name => 'public',
        table_name => 'test_table',
        key_field => 'id',
        text_fields => paradedb.field('fulltext')
    );
    "#
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

    "CALL paradedb.create_bm25(
	    index_name => 'index_config_index',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    text_fields => paradedb.field('description'))"
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

    r#"
    CALL paradedb.create_bm25(
	    index_name => 'index_config_index',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    text_fields => paradedb.field('description', fast => true, record => 'freq', normalizer => 'raw', tokenizer => paradedb.tokenizer('en_stem'))
    )"#
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

    r#"
    CALL paradedb.create_bm25(
	index_name => 'index_config_index',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    text_fields => paradedb.field('description', fast => true, record => 'freq', normalizer => 'raw', tokenizer => paradedb.tokenizer('en_stem')) ||
                       paradedb.field('category')
    )"#
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

    "CALL paradedb.create_bm25(
	    index_name => 'index_config_index',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    numeric_fields => paradedb.field('rating')
    );"
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

    "CALL paradedb.create_bm25(
	    index_name => 'index_config_index',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    numeric_fields => paradedb.field('rating', fast => false)
    )"
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

    "CALL paradedb.create_bm25(
	    index_name => 'index_config_index',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    boolean_fields => paradedb.field('in_stock')
    )"
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

    "CALL paradedb.create_bm25(
	    index_name => 'index_config_index',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    boolean_fields => paradedb.field('in_stock', fast => false)
    )"
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

    "CALL paradedb.create_bm25(
	    index_name => 'index_config_index',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    json_fields => paradedb.field('metadata')
    )"
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

    r#"CALL paradedb.create_bm25(
	    index_name => 'index_config_index',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    json_fields => paradedb.field('metadata', fast => true, expand_dots => false, tokenizer => paradedb.tokenizer('raw'), normalizer => 'raw')
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

    "CALL paradedb.create_bm25(
        index_name => 'index_config_index',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'id',
        datetime_fields => paradedb.field('created_at') || paradedb.field('last_updated_date')
    )"
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

    r#"CALL paradedb.create_bm25(
        index_name => 'index_config_index',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'id',
        datetime_fields => paradedb.field('created_at', fast => true) || paradedb.field('last_updated_date', fast => false)
    )"#
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

    "CALL paradedb.create_bm25( index_name => 'index_config_index',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    text_fields => paradedb.field('description') || paradedb.field('category'),
	    numeric_fields => paradedb.field('rating'),
	    boolean_fields => paradedb.field('in_stock'),
	    json_fields => paradedb.field('metadata')
    )"
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

    "CALL paradedb.create_bm25( 
        index_name => 'index_config_index',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    text_fields => paradedb.field('description') || paradedb.field('category'),
	    numeric_fields => paradedb.field('rating'),
	    boolean_fields => paradedb.field('in_stock'),
	    json_fields => paradedb.field('metadata')
    )"
    .execute(&mut conn);

    let rows: Vec<(String, Option<String>, Option<i32>)> = "
        SELECT description, category, rating
        FROM paradedb.index_config WHERE index_config @@@ 'description:\"Null Item\"'
        ORDER BY id"
        .fetch(&mut conn);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], ("Null Item 1".into(), None, None));
    assert_eq!(rows[1], ("Null Item 2".into(), None, Some(2)));

    // If incorrectly handled, false booleans can be mistaken as NULL values and ignored during indexing
    // This tests that false booleans are correctly indexed as such
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

    match "CALL paradedb.create_bm25(
        index_name => 'index_config_index',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'id',
        text_fields => paradedb.field('description')
    )".execute_result(&mut conn)
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

    "CALL paradedb.create_bm25(
        index_name => 'index_config_index',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'id',
        text_fields => paradedb.field('description')
    )"
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

    "CALL paradedb.create_bm25(
        index_name => 'index_config_index',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'IdName',
        text_fields => paradedb.field('ColumnName')
    )"
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
    "CALL paradedb.create_bm25(
        index_name => 'index_config1_index',
        table_name => 'index_config1',
        schema_name => 'paradedb',
        key_field => 'id',
        text_fields => paradedb.field('description')
    )"
    .execute(&mut conn);
    "CALL paradedb.create_bm25(
        index_name => 'index_config2_index',
        table_name => 'index_config2',
        schema_name => 'paradedb',
        key_field => 'id',
        text_fields => paradedb.field('description')
    )"
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
fn partitioned_index(mut conn: PgConnection) {
    r#"
        CREATE TABLE sales (
            id SERIAL,
            sale_date DATE NOT NULL,
            amount NUMERIC NOT NULL, description TEXT,
            PRIMARY KEY (id, sale_date)
        ) PARTITION BY RANGE (sale_date);

        CREATE TABLE sales_2023_q1 PARTITION OF sales
        FOR VALUES FROM ('2023-01-01') TO ('2023-03-31');

        CREATE TABLE sales_2023_q2 PARTITION OF sales
        FOR VALUES FROM ('2023-04-01') TO ('2023-06-30');

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

    match r#"
        CALL paradedb.create_bm25(
            index_name => 'sales_index',
            table_name => 'sales',
            schema_name => 'public',
            key_field => 'id',
            text_fields => paradedb.field('description'),
            datetime_fields => paradedb.field('sale_date'),
            numeric_fields => paradedb.field('amount')
        )
    "#.execute_result(&mut conn) {
        Ok(_) => panic!("should fail with partitioned table"),
        Err(err) => assert_eq!(err.to_string(), "error returned from database: Creating BM25 indexes over partitioned tables is a ParadeDB enterprise feature. Contact support@paradedb.com for access."),
    };
}

#[rstest]
fn delete_index_deletes_tantivy_files(mut conn: PgConnection) {
    // Create the test table and BM25 index
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'public')"
        .execute(&mut conn);

    "CALL paradedb.create_bm25(
        index_name => 'index_config_index',
        table_name => 'index_config',
        schema_name => 'public',
        key_field => 'id',
        text_fields => paradedb.field('description')
    )"
    .execute(&mut conn);

    // Ensure the expected directory exists.
    let index_dir = pg_search_index_directory_path(&mut conn, "index_config_index");
    assert!(
        index_dir.exists(),
        "expected index directory to exist at: {:?}",
        index_dir
    );

    // Delete the index.
    "DROP INDEX index_config_index CASCADE".execute(&mut conn);

    // Ensure deletion has worked as expected.
    // Tantivy is a little stubborn about deletion. While the contents of the index
    // will indeed be cleaned up, lingering Readers cached in connections seem to re-create
    // certain files if they are found to be deleted. This makes it difficult to completely
    // clean up the folder, so we will just test if our configuration JSON has been deleted.
    assert!(
        !index_dir.join("search-index.json").exists(),
        "expected index directory to have been deleted at: {:?}",
        index_dir
    );
}

#[rstest]
fn delete_index_aborted_maintains_tantivy_files(mut conn: PgConnection) {
    // Create the test table and BM25 index
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'public')"
        .execute(&mut conn);

    "CALL paradedb.create_bm25(
        index_name => 'index_config_index',
        table_name => 'index_config',
        schema_name => 'public',
        key_field => 'id',
        text_fields => paradedb.field('description')
    )"
    .execute(&mut conn);

    // Ensure the expected directory exists.
    let index_dir = pg_search_index_directory_path(&mut conn, "index_config_index");
    assert!(
        index_dir.exists(),
        "expected index directory to exist at: {:?}",
        index_dir
    );

    // Delete the index.
    "DO $$ 
    BEGIN
        DROP INDEX index_config_index CASCADE;
        RAISE EXCEPTION 'Aborting the transaction intentionally';
    END $$;"
        .execute_result(&mut conn)
        .ok();

    // Ensure index files still exist.
    assert!(
        index_dir.join("search-index.json").exists(),
        "expected index directory to have been not been deleted at: {:?}",
        index_dir
    );
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
    CALL paradedb.create_bm25(
        index_name => 'index_config_index',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'id',
        text_fields => paradedb.field('description'),
        numeric_fields => paradedb.field('color')
    )
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
    CALL paradedb.create_bm25(
        index_name => 'index_config_index',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'id',
        text_fields => paradedb.field('description'),
        numeric_fields => paradedb.field('color')
    )
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> =
        "SELECT id, description FROM paradedb.index_config WHERE id @@@ paradedb.parse('color:1.0')".fetch(&mut conn);

    assert_eq!(rows, vec![(1, "Item 1".into())]);
}
