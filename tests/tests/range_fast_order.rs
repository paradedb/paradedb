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
use serde_json::Value;
use sqlx::PgConnection;

#[fixture]
fn setup_test_table(mut conn: PgConnection) -> PgConnection {
    let sql = r#"
        CREATE TABLE test_range (
            id SERIAL PRIMARY KEY,
            value numrange
        );
    "#;
    sql.execute(&mut conn);

    let sql = r#"
        CREATE INDEX idx_test_range ON test_range USING bm25 (id, value)
        WITH (
            key_field='id',
            range_fields='{"value": {"fast": true}}'
        );
    "#;
    sql.execute(&mut conn);

    r#"
        INSERT INTO test_range (id, value) VALUES
            (1, '[10, 20)'),
            (2, '[5, 15)'),
            (3, '[25, 30)');
    "#
    .execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);
    "SET enable_bitmapscan TO off;".execute(&mut conn);
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    conn
}

mod fast_order_numrange {
    use super::*;

    #[rstest]
    fn verify_custom_scan(#[from(setup_test_table)] mut conn: PgConnection) {
        let (plan,) = r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            select * from test_range
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(value);
            "#
        .fetch_one::<(Value,)>(&mut conn);

        let custom_scan = plan
            .pointer("/0/Plan/Plans/0")
            .unwrap()
            .as_object()
            .unwrap();
        pretty_assertions::assert_eq!(
            custom_scan.get("Node Type"),
            Some(&Value::String(String::from("Custom Scan")))
        );
    }

    #[rstest]
    fn verify_sort_key(#[from(setup_test_table)] mut conn: PgConnection) {
        let (plan,) = r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            select * from test_range
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(value);
            "#
        .fetch_one::<(Value,)>(&mut conn);

        let sort_key = plan
            .pointer("/0/Plan/Sort Key")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        pretty_assertions::assert_eq!(
            sort_key,
            vec![Value::String("(lower(test_range.value))".to_string())]
        );
    }

    #[rstest]
    fn with_lower_range_asc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(value)::BIGINT, upper(value)::BIGINT from test_range
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(value);
        "#
        .fetch::<(i32, i64, i64)>(&mut conn);
        assert_eq!(sql, vec![(2, 5, 15), (1, 10, 20), (3, 25, 30)]);
    }

    #[rstest]
    fn with_lower_range_desc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(value)::BIGINT, upper(value)::BIGINT from test_range
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(value) desc;
        "#
        .fetch::<(i32, i64, i64)>(&mut conn);
        assert_eq!(sql, vec![(3, 25, 30), (1, 10, 20), (2, 5, 15)]);
    }

    #[rstest]
    fn with_upper_range_asc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(value)::BIGINT, upper(value)::BIGINT from test_range
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by upper(value);
        "#
        .fetch::<(i32, i64, i64)>(&mut conn);
        assert_eq!(sql, vec![(2, 5, 15), (1, 10, 20), (3, 25, 30)]);
    }

    #[rstest]
    fn with_upper_range_desc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(value)::BIGINT, upper(value)::BIGINT from test_range
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by upper(value) desc;
        "#
        .fetch::<(i32, i64, i64)>(&mut conn);
        assert_eq!(sql, vec![(3, 25, 30), (1, 10, 20), (2, 5, 15)]);
    }
}
