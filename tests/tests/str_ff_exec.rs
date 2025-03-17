#![allow(dead_code)]
// Copyright (c) 2023-2025 Retake, Inc.
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

mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[fixture]
fn setup_test_table(mut conn: PgConnection) -> PgConnection {
    let sql = r#"
        CREATE TABLE test (
            id SERIAL8 NOT NULL PRIMARY KEY,
            col_boolean boolean DEFAULT false,
            col_text text,
            col_int8 int8
        );
    "#;
    sql.execute(&mut conn);

    let sql = r#"
        CREATE INDEX idxtest ON test USING bm25 (id, col_boolean, col_text, col_int8)
        WITH (key_field='id', text_fields = '{"col_text": {"fast": true, "tokenizer": {"type":"raw"}}}');
    "#;
    sql.execute(&mut conn);

    "INSERT INTO test (id, col_text) VALUES (1, NULL);".execute(&mut conn);
    "INSERT INTO test (id, col_text) VALUES (2, 'foo');".execute(&mut conn);
    "INSERT INTO test (id, col_text, col_int8) VALUES (3, 'bar', 333);".execute(&mut conn);
    "INSERT INTO test (id, col_int8) VALUES (4, 444);".execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);
    "SET enable_bitmapscan TO off;".execute(&mut conn);
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    conn
}

mod string_fast_field_exec {
    use super::*;

    #[rstest]
    fn with_range(#[from(setup_test_table)] mut conn: PgConnection) {
        let res = r#"
            SELECT * FROM test
            WHERE id @@@ paradedb.range(field => 'id', range => int8range(1, 5, '[]'))
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(
            res,
            vec![
                (1, false, None, None),
                (2, false, Some(String::from("foo")), None),
                (3, false, Some(String::from("bar")), Some(333)),
                (4, false, None, Some(444))
            ]
        );
    }

    #[rstest]
    fn with_filter(#[from(setup_test_table)] mut conn: PgConnection) {
        let res = r#"
            SELECT * FROM test
            WHERE col_text IS NULL and id @@@ '>2'
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![(4, false, None, Some(444))]);
    }
}
