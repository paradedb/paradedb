mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn basic_search_query(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let rows: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('category:electronics', stable_sort => true)"
            .fetch_collect(&mut conn);

    assert_eq!(rows.id, vec![1, 2, 12, 22, 32])
}

#[rstest]
fn with_limit_and_offset(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let rows: SimpleProductsTableVec = "SELECT * FROM bm25_search.search(
            'category:electronics',
            limit_rows => 2,
            stable_sort => true
    )"
    .fetch_collect(&mut conn);

    assert_eq!(rows.id, vec![1, 2]);

    let rows: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('category:electronics', limit_rows => 2, offset_rows => 1, stable_sort => true)"
            .fetch_collect(&mut conn);

    assert_eq!(rows.id, vec![2, 12]);
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
        "SELECT * FROM tokenizer_config.search('description:earbud', stable_sort => true)"
            .fetch(&mut conn);

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
        "SELECT id FROM tokenizer_config.search('description:earbud', stable_sort => true)"
            .fetch(&mut conn);

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
        "SELECT id FROM tokenizer_config.search('description:boa', stable_sort => true)"
            .fetch(&mut conn);

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
        "SELECT id FROM tokenizer_config.search('description:电脑', stable_sort => true)"
            .fetch(&mut conn);

    assert_eq!(rows[0], (42,));
}
