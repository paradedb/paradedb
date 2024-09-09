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
