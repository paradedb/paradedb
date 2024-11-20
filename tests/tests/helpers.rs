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

//! Tests for the paradedb.tokenize function

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn defult_tokenizer(mut conn: PgConnection) {
    let rows: Vec<(String, i32)> = r#"
    SELECT * FROM paradedb.tokenize(paradedb.tokenizer('default'), 'hello world');
    "#
    .fetch_collect(&mut conn);

    assert_eq!(rows, vec![("hello".into(), 0), ("world".into(), 1)]);

    let res = r#"
    SELECT * FROM paradedb.tokenize(paradedb.tokenizer('de'), 'hello world');
    "#
    .execute_result(&mut conn);

    assert!(res.is_err());
}

#[rstest]
fn tokenizer_filters(mut conn: PgConnection) {
    // Test en_stem tokenizer with default layers (lowercase => true, remove_long => 255).
    let rows: Vec<(String, i32)> = r#"
    SELECT * FROM paradedb.tokenize(
      paradedb.tokenizer('en_stem'), 
      'Hello, hello, ladiesandgentlemen!'
    );
    "#
    .fetch_collect(&mut conn);

    assert_eq!(
        rows,
        vec![
            ("hello".into(), 0),
            ("hello".into(), 1),
            ("ladiesandgentlemen".into(), 2)
        ]
    );

    // Test en_stem optimizer with explicit layers.
    let rows: Vec<(String, i32)> = r#"
    SELECT * FROM paradedb.tokenize(
      paradedb.tokenizer('en_stem', lowercase => false, remove_long => 15),
      'Hello, hello, ladiesandgentlemen!'
    );
    "#
    .fetch_collect(&mut conn);

    assert_eq!(
        rows,
        vec![
            ("Hello".into(), 0),
            ("hello".into(), 1),
            // ladiesandgentlemen is filtered out because it is too long
        ]
    );
}

#[rstest]
fn list_tokenizers(mut conn: PgConnection) {
    let rows: Vec<(String,)> = r#"
    SELECT * FROM paradedb.tokenizers();
    "#
    .fetch_collect(&mut conn);

    if cfg!(feature = "icu") {
        assert_eq!(
            rows,
            vec![
                ("default".into(),),
                ("raw".into(),),
                ("en_stem".into(),),
                ("stem".into(),),
                ("lowercase".into(),),
                ("white_space".into(),),
                ("regex_tokenizer".into(),),
                ("chinese_compatible".into(),),
                ("source_code".into(),),
                ("ngram".into(),),
                ("chinese_lindera".into(),),
                ("japanese_lindera".into(),),
                ("korean_lindera".into(),),
                ("icu".into(),)
            ]
        );
    } else {
        assert_eq!(
            rows,
            vec![
                ("default".into(),),
                ("raw".into(),),
                ("en_stem".into(),),
                ("stem".into(),),
                ("lowercase".into(),),
                ("white_space".into(),),
                ("regex_tokenizer".into(),),
                ("chinese_compatible".into(),),
                ("source_code".into(),),
                ("ngram".into(),),
                ("chinese_lindera".into(),),
                ("japanese_lindera".into(),),
                ("korean_lindera".into(),),
            ]
        );
    }
}

#[rstest]
fn test_format_create_bm25_basic(mut conn: PgConnection) {
    let row = r#"
        SELECT paradedb.format_create_bm25(
            'my_index'::text, 
            'my_table'::text, 
            'id'::text, 
            'public'::text, 
            '{"title": true}'::jsonb, 
            '{"price": true}'::jsonb, 
            '{"is_available": true}'::jsonb, 
            '{"details": true}'::jsonb, 
            '{"price_range": true}'::jsonb, 
            '{"published_date": true}'::jsonb, 
            'price > 0'::text
        );
    "#
    .fetch_one::<(String,)>(&mut conn);

    let expected = r##"CREATE INDEX my_index ON public.my_table USING bm25 (id, id, title, price, is_available, details, price_range, published_date) WITH (key_field='id', text_fields='{"title":true}', numeric_fields='{"price":true}', boolean_fields='{"is_available":true}', json_fields='{"details":true}', range_fields='{"price_range":true}', datetime_fields='{"published_date":true}') WHERE price > 0;"##;

    assert_eq!(row.0, expected);
}

#[rstest]
fn test_format_create_index_no_predicate(mut conn: PgConnection) {
    let row = r#"
        SELECT paradedb.format_create_bm25(
            'another_index', 
            'products', 
            'product_id', 
            'inventory', 
            '{"name": true}'::jsonb, 
            '{}'::jsonb, 
            '{}'::jsonb, 
            '{}'::jsonb, 
            '{}'::jsonb, 
            '{}'::jsonb, 
            ''
        );
    "#
    .fetch_one::<(String,)>(&mut conn);

    let expected = r##"CREATE INDEX another_index ON inventory.products USING bm25 (product_id, product_id, name) WITH (key_field='product_id', text_fields='{"name":true}', numeric_fields='{}', boolean_fields='{}', json_fields='{}', range_fields='{}', datetime_fields='{}') ;"##;

    assert_eq!(row.0, expected);
}

#[rstest]
fn test_format_bm25_basic(mut conn: PgConnection) {
    let row = r#"
        SELECT paradedb.format_create_bm25(
            'my_search_idx',
            'articles',
            'id',
            'public',
            '{"title": true, "content": true}'::jsonb,
            '{"rating": true}'::jsonb,
            '{"published": true}'::jsonb,
            '{"metadata": true}'::jsonb,
            '{"price_range": true}'::jsonb,
            '{"created_at": true}'::jsonb,
            'rating > 3'
        );
    "#
    .fetch_one::<(String,)>(&mut conn);

    let expected = r##"CREATE INDEX my_search_idx ON public.articles USING bm25 (id, id, content, title, rating, published, metadata, price_range, created_at) WITH (key_field='id', text_fields='{"content":true,"title":true}', numeric_fields='{"rating":true}', boolean_fields='{"published":true}', json_fields='{"metadata":true}', range_fields='{"price_range":true}', datetime_fields='{"created_at":true}') WHERE rating > 3;"##;

    assert_eq!(row.0, expected);
}

#[rstest]
fn test_format_bm25_empty_fields(mut conn: PgConnection) {
    let row = r#"
        SELECT paradedb.format_create_bm25(
            'minimal_idx',
            'simple_table',
            'id',
            'public',
            '{"title": true}'::jsonb,
            '{}'::jsonb,
            '{}'::jsonb,
            '{}'::jsonb,
            '{}'::jsonb,
            '{}'::jsonb,
            ''
        );
    "#
    .fetch_one::<(String,)>(&mut conn);

    let expected = r#"CREATE INDEX minimal_idx ON public.simple_table USING bm25 (id, id, title) WITH (key_field='id', text_fields='{"title":true}', numeric_fields='{}', boolean_fields='{}', json_fields='{}', range_fields='{}', datetime_fields='{}') ;"#;

    assert_eq!(row.0, expected);
}

#[rstest]
fn test_format_bm25_invalid_json(mut conn: PgConnection) {
    let res = r#"
        SELECT paradedb.format_bm25(
            'bad_idx',
            'test_table',
            'id',
            'public',
            'invalid json',
            '{}'::jsonb,
            '{}'::jsonb,
            '{}'::jsonb,
            '{}'::jsonb,
            '{}'::jsonb,
            ''
        );
    "#
    .execute_result(&mut conn);

    assert!(res.is_err());
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .to_string()
        .contains("error returned from database"));
}

#[rstest]
fn test_index_fields(mut conn: PgConnection) {
    // First create a test table and index
    r#"
        CREATE TABLE test_fields (
            id INTEGER PRIMARY KEY,
            title TEXT,
            price NUMERIC,
            in_stock BOOLEAN,
            metadata JSONB,
            price_range INT8RANGE,
            created_at TIMESTAMP
        );
    "#
    .execute_result(&mut conn)
    .unwrap();

    r#"
        CREATE INDEX idx_test_fields ON test_fields USING bm25 (
            id, title, price, in_stock, metadata, price_range, created_at
        ) WITH (
            key_field='id',
            text_fields='{"title": {"fast": true}}',
            numeric_fields='{"price": {}}',
            boolean_fields='{"in_stock": {}}',
            json_fields='{"metadata": {}}',
            range_fields='{"price_range": {}}',
            datetime_fields='{"created_at": {}}'
        );
    "#
    .execute_result(&mut conn)
    .unwrap();

    // Get the index fields
    let row: (serde_json::Value,) = r#"
        SELECT paradedb.index_fields('idx_test_fields')::jsonb;
    "#
    .fetch_one(&mut conn);

    // Verify all fields are present with correct configurations
    let fields = row.0.as_object().unwrap();

    // Check key field (id)
    assert!(fields.contains_key("id"));
    let id_config = fields.get("id").unwrap().get("Numeric").unwrap();
    assert_eq!(id_config.get("indexed").unwrap(), true);
    assert_eq!(id_config.get("fast").unwrap(), true);
    assert_eq!(id_config.get("stored").unwrap(), true);

    // Check text field (title)
    assert!(fields.contains_key("title"));
    let title_config = fields
        .get("title")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Text")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        title_config.get("indexed").unwrap().as_bool().unwrap(),
        true
    );
    assert_eq!(title_config.get("stored").unwrap().as_bool().unwrap(), true);

    // Check numeric field (price)
    assert!(fields.contains_key("price"));
    let price_config = fields
        .get("price")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Numeric")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        price_config.get("indexed").unwrap().as_bool().unwrap(),
        true
    );
    assert_eq!(price_config.get("fast").unwrap().as_bool().unwrap(), true);

    // Check boolean field (in_stock)
    assert!(fields.contains_key("in_stock"));
    let stock_config = fields
        .get("in_stock")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Boolean")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        stock_config.get("indexed").unwrap().as_bool().unwrap(),
        true
    );
    assert_eq!(stock_config.get("stored").unwrap().as_bool().unwrap(), true);

    // Check JSON field (metadata)
    assert!(fields.contains_key("metadata"));
    let metadata_config = fields
        .get("metadata")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Json")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        metadata_config.get("indexed").unwrap().as_bool().unwrap(),
        true
    );
    assert_eq!(
        metadata_config.get("stored").unwrap().as_bool().unwrap(),
        true
    );

    // Check range field (price_range)
    assert!(fields.contains_key("price_range"));
    let range_config = fields
        .get("price_range")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Range")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(range_config.get("stored").unwrap().as_bool().unwrap(), true);

    // Check datetime field (created_at)
    assert!(fields.contains_key("created_at"));
    let date_config = fields
        .get("created_at")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Date")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(date_config.get("indexed").unwrap().as_bool().unwrap(), true);
    assert_eq!(date_config.get("stored").unwrap().as_bool().unwrap(), true);

    // Check ctid field is present
    assert!(fields.contains_key("ctid"));

    // Cleanup
    r#"DROP TABLE test_fields CASCADE;"#.execute_result(&mut conn).unwrap();
}
