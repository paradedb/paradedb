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
