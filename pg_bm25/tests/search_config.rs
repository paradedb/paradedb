#![allow(unused_variables, unused_imports)]
mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn basic_search_query(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let rows: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('category:electronics')".fetch_collect(&mut conn);

    assert_eq!(rows.id, vec![1, 2, 12, 22, 32])
}

#[rstest]
fn with_limit_and_offset(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let rows: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('category:electronics', limit_rows => 2)"
            .fetch_collect(&mut conn);

    assert_eq!(rows.id, vec![1, 2]);

    let rows: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('category:electronics', limit_rows => 2, offset_rows => 1)"
            .fetch_collect(&mut conn);

    assert_eq!(rows.id, vec![2, 12]);
}

#[rstest]
fn fuzzy_fields(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let rows: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('category:electornics', fuzzy_fields => 'category')"
            .fetch_collect(&mut conn);

    assert_eq!(rows.id, vec![1, 2, 12, 22, 32], "wrong results");

    let rows: Vec<SimpleProductsTable> =
        "SELECT * FROM bm25_search.search('category:electornics')".fetch(&mut conn);

    assert!(rows.is_empty(), "without fuzzy field should be empty");

    let rows: Vec<SimpleProductsTable> = "SELECT * FROM bm25_search.search(
            'description:keybaord',
            fuzzy_fields => 'description',
            transpose_cost_one => false,
            distance => 1
        )"
    .fetch_collect(&mut conn);

    assert!(rows.is_empty(), "transpose false should be empty");

    let rows: SimpleProductsTableVec = "SELECT * FROM bm25_search.search(
            'description:keybaord',
            fuzzy_fields => 'description',
            transpose_cost_one => true,
            distance => 1
        )"
    .fetch_collect(&mut conn);

    assert_eq!(rows.id, vec![1, 2], "incorrect transpose true");

    let rows: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('com', regex_fields => 'description')"
            .fetch_collect(&mut conn);

    assert_eq!(rows.id, vec![6, 23], "incorrect regex field");

    let rows: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('key', fuzzy_fields => 'description,category', distance => 2, transpose_cost_one => false, prefix => false, limit_rows => 5)"
            .fetch_collect(&mut conn);

    assert_eq!(rows.id, vec![8, 10, 30], "incorrect fuzzy prefix disabled");

    let rows: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('key', fuzzy_fields => 'description,category', distance => 2, transpose_cost_one => false, prefix => true, limit_rows => 5)"
            .fetch_collect(&mut conn);

    assert_eq!(
        rows.id,
        vec![1, 2, 10, 40, 12],
        "incorrect fuzzy prefix enabled"
    );
}

#[rstest]
fn default_tokenizer_config(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'tokenizer_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CALL paradedb.create_bm25(
    	index_name => 'tokenizer_config',
    	table_name => 'tokenizer_config',
    	schema_name => 'paradedb',
    	key_field => 'id',
    	text_fields => '{"description": {}}'
    )"#
    .execute(&mut conn);

    let rows: Vec<()> =
        "SELECT * FROM tokenizer_config.search('description:earbud')".fetch(&mut conn);

    assert!(rows.is_empty());
}

#[rstest]
fn en_stem_tokenizer_config(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'tokenizer_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CALL paradedb.create_bm25(
    	index_name => 'tokenizer_config',
    	table_name => 'tokenizer_config',
    	schema_name => 'paradedb',
    	key_field => 'id',
    	text_fields => '{"description": {"tokenizer": { "type": "en_stem" }}}'
    )"#
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM tokenizer_config.search('description:earbud')".fetch(&mut conn);

    assert_eq!(rows[0], (12,));
}

#[rstest]
fn ngram_tokenizer_config(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'tokenizer_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CALL paradedb.create_bm25(
    	index_name => 'tokenizer_config',
    	table_name => 'tokenizer_config',
    	schema_name => 'paradedb',
    	key_field => 'id',
	    text_fields => '{"description": {"tokenizer": {"type": "ngram", "min_gram": 3, "max_gram": 8, "prefix_only": false}}}'
    )"#
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM tokenizer_config.search('description:boa')".fetch(&mut conn);

    assert_eq!(rows[0], (2,));
    assert_eq!(rows[1], (20,));
    assert_eq!(rows[2], (1,));
}

#[rstest]
fn chinese_compatible_tokenizer_config(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'tokenizer_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CALL paradedb.create_bm25(
    	index_name => 'tokenizer_config',
    	table_name => 'tokenizer_config',
    	schema_name => 'paradedb',
    	key_field => 'id',
	    text_fields => '{"description": {"tokenizer": {"type": "chinese_compatible"}, "record": "position"}}'
    );
    INSERT INTO paradedb.tokenizer_config (description, rating, category) VALUES ('电脑', 4, 'Electronics');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM tokenizer_config.search('description:电脑')".fetch(&mut conn);

    assert_eq!(rows[0], (42,));
}
