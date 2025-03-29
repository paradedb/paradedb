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
        CREATE TABLE test_ranges (
            id SERIAL PRIMARY KEY,
            int_range int8range,
            num_range numrange,
            date_range daterange,
            ts_range tsrange
        );
    "#;
    sql.execute(&mut conn);

    let sql = r#"
        CREATE INDEX idx_test_ranges ON test_ranges USING bm25 (id, int_range, num_range, date_range, ts_range)
        WITH (
            key_field='id',
            range_fields='{
                "int_range": {"fast": true},
                "num_range": {"fast": true},
                "date_range": {"fast": true},
                "ts_range": {"fast": true}
            }'
        );
    "#;
    sql.execute(&mut conn);

    r#"
        INSERT INTO test_ranges (id, int_range, num_range, date_range, ts_range) VALUES
        (1, '[10, 20)', '[10.5, 20.5)', '[2023-01-01, 2023-01-31)', '[2023-01-15 09:00, 2023-01-15 17:00)'),
        (2, '[5, 15)', '[5.5, 15.5)', '[2023-02-01, 2023-02-28)', '[2023-02-01 10:00, 2023-02-01 11:30)'),
        (3, '[25, 30)', '[25.5, 30.5)', '[2023-03-01, 2023-03-31)', '[2023-03-10 14:00, 2023-03-10 15:30)');
    "#
    .execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);
    "SET enable_bitmapscan TO off;".execute(&mut conn);
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    conn
}

mod fast_order_intrange {
    use super::*;

    #[rstest]
    fn verify_custom_scan(#[from(setup_test_table)] mut conn: PgConnection) {
        let (plan,) = r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            select * from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(int_range);
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
            select * from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(int_range);
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
            vec![Value::String("(lower(test_ranges.int_range))".to_string())]
        );
    }

    #[rstest]
    fn with_lower_range_asc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(int_range), upper(int_range) from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(int_range);
        "#
        .fetch::<(i32, i64, i64)>(&mut conn);
        assert_eq!(sql, vec![(2, 5, 15), (1, 10, 20), (3, 25, 30)]);
    }

    #[rstest]
    fn with_lower_range_desc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(int_range), upper(int_range) from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(int_range) desc;
        "#
        .fetch::<(i32, i64, i64)>(&mut conn);
        assert_eq!(sql, vec![(3, 25, 30), (1, 10, 20), (2, 5, 15)]);
    }

    #[rstest]
    fn with_upper_range_asc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(int_range), upper(int_range) from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by upper(int_range);
        "#
        .fetch::<(i32, i64, i64)>(&mut conn);
        assert_eq!(sql, vec![(2, 5, 15), (1, 10, 20), (3, 25, 30)]);
    }

    #[rstest]
    fn with_upper_range_desc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(int_range), upper(int_range) from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by upper(int_range) desc;
        "#
        .fetch::<(i32, i64, i64)>(&mut conn);
        assert_eq!(sql, vec![(3, 25, 30), (1, 10, 20), (2, 5, 15)]);
    }
}

mod fast_order_numrange {
    use super::*;

    #[rstest]
    fn verify_custom_scan(#[from(setup_test_table)] mut conn: PgConnection) {
        let (plan,) = r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            select * from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(num_range);
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
            select * from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(num_range);
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
            vec![Value::String("(lower(test_ranges.num_range))".to_string())]
        );
    }

    #[rstest]
    fn with_lower_range_asc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(num_range)::DOUBLE PRECISION, upper(num_range)::DOUBLE PRECISION from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(num_range);
        "#
        .fetch::<(i32, f64, f64)>(&mut conn);
        assert_eq!(sql, vec![(2, 5.5, 15.5), (1, 10.5, 20.5), (3, 25.5, 30.5)]);
    }

    #[rstest]
    fn with_lower_range_desc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(num_range)::DOUBLE PRECISION, upper(num_range)::DOUBLE PRECISION from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(num_range) desc;
        "#
        .fetch::<(i32, f64, f64)>(&mut conn);
        assert_eq!(sql, vec![(3, 25.5, 30.5), (1, 10.5, 20.5), (2, 5.5, 15.5)]);
    }

    #[rstest]
    fn with_upper_range_asc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(num_range)::DOUBLE PRECISION, upper(num_range)::DOUBLE PRECISION from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by upper(num_range);
        "#
        .fetch::<(i32, f64, f64)>(&mut conn);
        assert_eq!(sql, vec![(2, 5.5, 15.5), (1, 10.5, 20.5), (3, 25.5, 30.5)]);
    }

    #[rstest]
    fn with_upper_range_desc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(num_range)::DOUBLE PRECISION, upper(num_range)::DOUBLE PRECISION from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by upper(num_range) desc;
        "#
        .fetch::<(i32, f64, f64)>(&mut conn);
        assert_eq!(sql, vec![(3, 25.5, 30.5), (1, 10.5, 20.5), (2, 5.5, 15.5)]);
    }
}

mod fast_order_daterange {
    use super::*;
    use time::Date;

    #[rstest]
    fn verify_custom_scan(#[from(setup_test_table)] mut conn: PgConnection) {
        let (plan,) = r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            select * from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(date_range);
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
            select * from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(date_range);
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
            vec![Value::String("(lower(test_ranges.date_range))".to_string())]
        );
    }

    #[rstest]
    fn with_lower_range_asc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(date_range), upper(date_range) from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(date_range);
        "#
        .fetch::<(i32, Date, Date)>(&mut conn);
        assert_eq!(
            sql,
            vec![
                (
                    1,
                    Date::from_calendar_date(2023, time::Month::January, 1).unwrap(),
                    Date::from_calendar_date(2023, time::Month::January, 31).unwrap()
                ),
                (
                    2,
                    Date::from_calendar_date(2023, time::Month::February, 1).unwrap(),
                    Date::from_calendar_date(2023, time::Month::February, 28).unwrap()
                ),
                (
                    3,
                    Date::from_calendar_date(2023, time::Month::March, 1).unwrap(),
                    Date::from_calendar_date(2023, time::Month::March, 31).unwrap()
                )
            ]
        );
    }

    #[rstest]
    fn with_lower_range_desc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(date_range), upper(date_range) from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(date_range) desc;
        "#
        .fetch::<(i32, Date, Date)>(&mut conn);
        assert_eq!(
            sql,
            vec![
                (
                    3,
                    Date::from_calendar_date(2023, time::Month::March, 1).unwrap(),
                    Date::from_calendar_date(2023, time::Month::March, 31).unwrap()
                ),
                (
                    2,
                    Date::from_calendar_date(2023, time::Month::February, 1).unwrap(),
                    Date::from_calendar_date(2023, time::Month::February, 28).unwrap()
                ),
                (
                    1,
                    Date::from_calendar_date(2023, time::Month::January, 1).unwrap(),
                    Date::from_calendar_date(2023, time::Month::January, 31).unwrap()
                )
            ]
        );
    }

    #[rstest]
    fn with_upper_range_asc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(date_range), upper(date_range) from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by upper(date_range);
        "#
        .fetch::<(i32, Date, Date)>(&mut conn);
        assert_eq!(
            sql,
            vec![
                (
                    1,
                    Date::from_calendar_date(2023, time::Month::January, 1).unwrap(),
                    Date::from_calendar_date(2023, time::Month::January, 31).unwrap()
                ),
                (
                    2,
                    Date::from_calendar_date(2023, time::Month::February, 1).unwrap(),
                    Date::from_calendar_date(2023, time::Month::February, 28).unwrap()
                ),
                (
                    3,
                    Date::from_calendar_date(2023, time::Month::March, 1).unwrap(),
                    Date::from_calendar_date(2023, time::Month::March, 31).unwrap()
                )
            ]
        );
    }

    #[rstest]
    fn with_upper_range_desc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(date_range), upper(date_range) from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by upper(date_range) desc;
        "#
        .fetch::<(i32, Date, Date)>(&mut conn);
        assert_eq!(
            sql,
            vec![
                (
                    3,
                    Date::from_calendar_date(2023, time::Month::March, 1).unwrap(),
                    Date::from_calendar_date(2023, time::Month::March, 31).unwrap()
                ),
                (
                    2,
                    Date::from_calendar_date(2023, time::Month::February, 1).unwrap(),
                    Date::from_calendar_date(2023, time::Month::February, 28).unwrap()
                ),
                (
                    1,
                    Date::from_calendar_date(2023, time::Month::January, 1).unwrap(),
                    Date::from_calendar_date(2023, time::Month::January, 31).unwrap()
                )
            ]
        );
    }
}

mod fast_order_tsrange {
    use super::*;
    use chrono::NaiveDateTime;

    #[rstest]
    fn verify_custom_scan(#[from(setup_test_table)] mut conn: PgConnection) {
        let (plan,) = r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            select * from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(ts_range);
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
            select * from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(ts_range);
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
            vec![Value::String("(lower(test_ranges.ts_range))".to_string())]
        );
    }

    #[rstest]
    fn with_lower_range_asc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(ts_range)::TIMESTAMP, upper(ts_range)::TIMESTAMP from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(ts_range);
        "#
        .fetch::<(i32, NaiveDateTime, NaiveDateTime)>(&mut conn);
        assert_eq!(
            sql,
            vec![
                (
                    1,
                    NaiveDateTime::parse_from_str("2023-01-15 09:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                    NaiveDateTime::parse_from_str("2023-01-15 17:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap()
                ),
                (
                    2,
                    NaiveDateTime::parse_from_str("2023-02-01 10:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                    NaiveDateTime::parse_from_str("2023-02-01 11:30:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap()
                ),
                (
                    3,
                    NaiveDateTime::parse_from_str("2023-03-10 14:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                    NaiveDateTime::parse_from_str("2023-03-10 15:30:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap()
                )
            ]
        );
    }

    #[rstest]
    fn with_lower_range_desc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(ts_range)::TIMESTAMP, upper(ts_range)::TIMESTAMP from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by lower(ts_range) desc;
        "#
        .fetch::<(i32, NaiveDateTime, NaiveDateTime)>(&mut conn);
        assert_eq!(
            sql,
            vec![
                (
                    3,
                    NaiveDateTime::parse_from_str("2023-03-10 14:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                    NaiveDateTime::parse_from_str("2023-03-10 15:30:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap()
                ),
                (
                    2,
                    NaiveDateTime::parse_from_str("2023-02-01 10:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                    NaiveDateTime::parse_from_str("2023-02-01 11:30:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap()
                ),
                (
                    1,
                    NaiveDateTime::parse_from_str("2023-01-15 09:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                    NaiveDateTime::parse_from_str("2023-01-15 17:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap()
                )
            ]
        );
    }

    #[rstest]
    fn with_upper_range_asc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(ts_range)::TIMESTAMP, upper(ts_range)::TIMESTAMP from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by upper(ts_range);
        "#
        .fetch::<(i32, NaiveDateTime, NaiveDateTime)>(&mut conn);
        assert_eq!(
            sql,
            vec![
                (
                    1,
                    NaiveDateTime::parse_from_str("2023-01-15 09:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                    NaiveDateTime::parse_from_str("2023-01-15 17:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap()
                ),
                (
                    2,
                    NaiveDateTime::parse_from_str("2023-02-01 10:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                    NaiveDateTime::parse_from_str("2023-02-01 11:30:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap()
                ),
                (
                    3,
                    NaiveDateTime::parse_from_str("2023-03-10 14:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                    NaiveDateTime::parse_from_str("2023-03-10 15:30:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap()
                )
            ]
        );
    }

    #[rstest]
    fn with_upper_range_desc(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            select id, lower(ts_range)::TIMESTAMP, upper(ts_range)::TIMESTAMP from test_ranges
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            order by upper(ts_range) desc;
        "#
        .fetch::<(i32, NaiveDateTime, NaiveDateTime)>(&mut conn);
        assert_eq!(
            sql,
            vec![
                (
                    3,
                    NaiveDateTime::parse_from_str("2023-03-10 14:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                    NaiveDateTime::parse_from_str("2023-03-10 15:30:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap()
                ),
                (
                    2,
                    NaiveDateTime::parse_from_str("2023-02-01 10:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                    NaiveDateTime::parse_from_str("2023-02-01 11:30:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap()
                ),
                (
                    1,
                    NaiveDateTime::parse_from_str("2023-01-15 09:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                    NaiveDateTime::parse_from_str("2023-01-15 17:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap()
                )
            ]
        );
    }
}
