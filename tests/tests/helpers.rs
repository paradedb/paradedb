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
use serde_json::json;
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

use serde_json::Value;

#[rstest]
fn test_field_basic(mut conn: PgConnection) {
    let row = r#"
        SELECT paradedb.field(
            'title', 
            indexed => true, 
            stored => true, 
            fast => false, 
            fieldnorms => true, 
            record => 'single', 
            expand_dots => false, 
            tokenizer => '{"type":"default"}'::jsonb, 
            normalizer => 'lowercase'
        )::text;
    "#
    .fetch_one::<(String,)>(&mut conn);

    let result_json: Value = serde_json::from_str(&row.0).unwrap();
    let expected_json = json!({
        "title": {
            "indexed": true,
            "stored": true,
            "fast": false,
            "fieldnorms": true,
            "record": "single",
            "expand_dots": false,
            "tokenizer": {"type": "default"},
            "normalizer": "lowercase"
        }
    });

    assert_eq!(result_json, expected_json);
}

#[rstest]
fn test_field_optional_values(mut conn: PgConnection) {
    let row = r#"
        SELECT paradedb.field('description')::text;
    "#
    .fetch_one::<(String,)>(&mut conn);

    let result_json: Value = serde_json::from_str(&row.0).unwrap();
    let expected_json = json!({
        "description": {}
    });

    assert_eq!(result_json, expected_json);
}

#[rstest]
fn test_format_create_index_basic(mut conn: PgConnection) {
    let row = r#"
        SELECT paradedb.format_create_index(
            'my_index', 
            'my_table', 
            'id', 
            'public', 
            '{"title": true}'::jsonb, 
            '{"price": true}'::jsonb, 
            '{"is_available": true}'::jsonb, 
            '{"details": true}'::jsonb, 
            '{"price_range": true}'::jsonb, 
            '{"published_date": true}'::jsonb, 
            'price > 0'
        );
    "#
    .fetch_one::<(String,)>(&mut conn);

    let expected = r#"CREATE INDEX my_index ON public.my_table USING bm25 (id, details, is_available, price, price_range, published_date, title) WITH (key_field='id', text_fields='{"title":true}', numeric_fields='{"price":true}', boolean_fields='{"is_available":true}', json_fields='{"details":true}', range_fields='{"price_range":true}', datetime_fields='{"published_date":true}') WHERE price > 0;"#;

    assert_eq!(row.0, expected);
}

#[rstest]
fn test_format_create_index_no_predicate(mut conn: PgConnection) {
    let row = r#"
        SELECT paradedb.format_create_index(
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

    let expected = r#"CREATE INDEX another_index ON inventory.products USING bm25 (product_id, name) WITH (key_field='product_id', text_fields='{"name":true}', numeric_fields='{}', boolean_fields='{}', json_fields='{}', range_fields='{}', datetime_fields='{}') ;"#;

    assert_eq!(row.0, expected);
}
