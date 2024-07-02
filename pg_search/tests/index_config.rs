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

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

fn fmt_err<T: std::error::Error>(err: T) -> String {
    format!("unexpected error, received: {}", err)
}

#[rstest]
fn invalid_create_bm25(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    match "CALL paradedb.create_bm25(
	    index_name => 'index_config',
	    table_name => 'index_config'
    )"
    .execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with no key_field"),
        Err(err) => assert!(err.to_string().contains("no key_field parameter")),
    };

    match "CALL paradedb.create_bm25(
	    index_name => 'index_config',
	    table_name => 'index_config',
	    key_field => 'id'
    )"
    .execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with no fields"),
        Err(err) => assert!(err.to_string().contains("specified"), "{}", fmt_err(err)),
    };

    match "CALL paradedb.create_bm25(
	    index_name => 'index_config',
	    table_name => 'index_config',
	    key_field => 'id',
	    invalid_field => '{}'		
    )"
    .execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with invalid field"),
        Err(err) => assert!(err.to_string().contains("not exist"), "{}", fmt_err(err)),
    };
}

#[rstest]
fn prevent_duplicate(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    "CALL paradedb.create_bm25(
        index_name => 'index_config',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'id',
        text_fields => '{description: {}}')"
        .execute(&mut conn);

    match "CALL paradedb.create_bm25(
        index_name => 'index_config',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'id',
        text_fields => '{description: {}}')"
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
        text_fields => '{fulltext: {}}'
    );

    CALL paradedb.drop_bm25('test_index');
    ALTER TABLE test_table DROP COLUMN fkey;
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        index_name => 'test_index',
        schema_name => 'public',
        table_name => 'test_table',
        key_field => 'id',
        text_fields => '{fulltext: {}}'
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM test_index.schema()".fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("fulltext".into(), "Str".into()));
    assert_eq!(rows[2], ("id".into(), "I64".into()));
}

#[rstest]
fn default_text_field(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    "CALL paradedb.create_bm25(
	    index_name => 'index_config',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    text_fields => '{description: {}}')"
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM index_config.schema()".fetch(&mut conn);

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
	    index_name => 'index_config',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    text_fields => '{description: {
	        fast: true, "tokenizer": { type: "en_stem" },
	        record: "freq", normalizer: "raw"
	     }}'
    )"#
    .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM index_config.schema()".fetch(&mut conn);

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
	index_name => 'index_config',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    text_fields => '{
	        "description": {fast: true, tokenizer: { type: "en_stem" }, record: "freq", normalizer: "raw"},
	        category: {}}'
    )"#
    .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM index_config.schema()".fetch(&mut conn);

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
	    index_name => 'index_config',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    numeric_fields => '{rating: {}}'
    );"
    .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM index_config.schema()".fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("rating".into(), "I64".into()));
}

#[rstest]
fn numeric_field_with_options(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    "CALL paradedb.create_bm25(
	    index_name => 'index_config',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    numeric_fields => '{rating: {fast: false}}'
    )"
    .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM index_config.schema()".fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("rating".into(), "I64".into()));
}

#[rstest]
fn default_boolean_field(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    "CALL paradedb.create_bm25(
	    index_name => 'index_config',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    boolean_fields => '{in_stock: {}}'
    )"
    .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM index_config.schema()".fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("in_stock".into(), "Bool".into()));
}

#[rstest]
fn boolean_field_with_options(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    "CALL paradedb.create_bm25(
	    index_name => 'index_config',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    boolean_fields => '{in_stock: {fast: false}}'
    )"
    .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM index_config.schema()".fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("in_stock".into(), "Bool".into()));
}

#[rstest]
fn default_json_field(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    "CALL paradedb.create_bm25(
	    index_name => 'index_config',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    json_fields => '{metadata: {}}'
    )"
    .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM index_config.schema()".fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("metadata".into(), "JsonObject".into()));
}

#[rstest]
fn json_field_with_options(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CALL paradedb.create_bm25(
	    index_name => 'index_config',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    json_fields => '{
	        metadata: {fast: true, expand_dots: false, tokenizer: { type: "raw" }, normalizer: "raw"}
	    }'
    )"#
    .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM index_config.schema()".fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("metadata".into(), "JsonObject".into()));
}

#[rstest]
fn default_datetime_field(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    "CALL paradedb.create_bm25(
        index_name => 'index_config',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'id',
        datetime_fields => '{created_at: {}, last_updated_date: {}}'
    )"
    .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM index_config.schema()".fetch(&mut conn);

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
        index_name => 'index_config',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'id',
        datetime_fields => '{
            created_at: {fast: true},
            last_updated_date: {fast: false}
        }'
    )"#
    .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM index_config.schema()".fetch(&mut conn);

    assert_eq!(rows[0], ("created_at".into(), "Date".into()));
    assert_eq!(rows[1], ("ctid".into(), "U64".into()));
    assert_eq!(rows[2], ("id".into(), "I64".into()));
    assert_eq!(rows[3], ("last_updated_date".into(), "Date".into()));
}

#[rstest]
fn multiple_fields(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    "CALL paradedb.create_bm25( index_name => 'index_config',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    text_fields => '{description: {}, category: {}}',
	    numeric_fields => '{rating: {}}',
	    boolean_fields => '{in_stock: {}}',
	    json_fields => '{metadata: {}}'
    )"
    .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM index_config.schema()".fetch(&mut conn);

    assert_eq!(rows[0], ("category".into(), "Str".into()));
    assert_eq!(rows[1], ("ctid".into(), "U64".into()));
    assert_eq!(rows[2], ("description".into(), "Str".into()));
    assert_eq!(rows[3], ("id".into(), "I64".into()));
    assert_eq!(rows[4], ("in_stock".into(), "Bool".into()));
    assert_eq!(rows[5], ("metadata".into(), "JsonObject".into()));
    assert_eq!(rows[6], ("rating".into(), "I64".into()));
}

#[rstest]
fn null_values(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    "INSERT INTO paradedb.index_config (description, category, rating) VALUES ('Null Item 1', NULL, NULL), ('Null Item 2', NULL, 2)"
        .execute(&mut conn);

    "CALL paradedb.create_bm25( 
        index_name => 'index_config',
	    table_name => 'index_config',
	    schema_name => 'paradedb',
	    key_field => 'id',
	    text_fields => '{description: {}, category: {}}',
	    numeric_fields => '{rating: {}}',
	    boolean_fields => '{in_stock: {}}',
	    json_fields => '{metadata: {}}'
    )"
    .execute(&mut conn);

    let rows: Vec<(String, Option<String>, Option<i32>)> =
        "SELECT description, category, rating FROM index_config.search('description:\"Null Item\"', stable_sort => true)"
            .fetch(&mut conn);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], ("Null Item 1".into(), None, None));
    assert_eq!(rows[1], ("Null Item 2".into(), None, Some(2)));

    // If incorrectly handled, false booleans can be mistaken as NULL values and ignored during indexing
    // This tests that false booleans are correctly indexed as such
    let rows: Vec<(bool,)> =
        "SELECT in_stock FROM index_config.search('in_stock:false')".fetch(&mut conn);

    assert_eq!(rows.len(), 13);
}

#[rstest]
fn null_key_field_build(mut conn: PgConnection) {
    "CREATE TABLE paradedb.index_config(id INTEGER, description TEXT)".execute(&mut conn);
    "INSERT INTO paradedb.index_config VALUES (NULL, 'Null Item 1'), (2, 'Null Item 2')"
        .execute(&mut conn);

    match "CALL paradedb.create_bm25(
        index_name => 'index_config',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'id',
        text_fields => '{description: {}}'
    )".execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with null key_field"),
        Err(err) => assert_eq!(
            err.to_string(),
            "error returned from database: error creating index entries for index 'index_config_bm25_index': key_field column 'id' cannot be NULL"
        ),
    };
}

#[rstest]
fn null_key_field_insert(mut conn: PgConnection) {
    "CREATE TABLE paradedb.index_config(id INTEGER, description TEXT)".execute(&mut conn);
    "INSERT INTO paradedb.index_config VALUES (1, 'Null Item 1'), (2, 'Null Item 2')"
        .execute(&mut conn);

    "CALL paradedb.create_bm25(
        index_name => 'index_config',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'id',
        text_fields => '{description: {}}'
    )"
    .execute(&mut conn);

    match "INSERT INTO paradedb.index_config VALUES (NULL, 'Null Item 3')".execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with null key_field"),
        Err(err) => assert_eq!(
            err.to_string(),
            "error returned from database: error creating index entries for index 'index_config_bm25_index': key_field column 'id' cannot be NULL"
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
        index_name => 'index_config',
        table_name => 'index_config',
        schema_name => 'paradedb',
        key_field => 'IdName',
        text_fields => '{ColumnName: {}}'
    )"
    .execute(&mut conn);

    let rows: Vec<(i32, String)> =
        "SELECT * FROM index_config.search('ColumnName:keyboard')".fetch(&mut conn);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], (1, "Plastic Keyboard".into()));
}
