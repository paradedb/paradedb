// Copyright (c) 2023-2026 ParadeDB, Inc.
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
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn datetime_microsecond(mut conn: PgConnection) {
    r#"
    CREATE TABLE ts (id SERIAL, t TIMESTAMP);
    CREATE INDEX ts_idx on ts using bm25 (id, t) with (key_field = 'id');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14.079776Z');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14.079777Z');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14.079778Z');
    "#
    .execute(&mut conn);

    // Term queries
    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14.079777Z'::timestamp".fetch(&mut conn);
    let rows: Vec<(i32,)> = "SELECT id FROM ts WHERE id @@@ paradedb.term('t', '2025-01-28T18:19:14.079777Z'::timestamp)".fetch(&mut conn);
    assert_eq!(rows, expected);

    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14.079777Z'::timestamp".fetch(&mut conn);
    let rows: Vec<(i32,)> =
        r#"SELECT id FROM ts WHERE t @@@ '"2025-01-28T18:19:14.079777Z"'"#.fetch(&mut conn);
    assert_eq!(rows, expected);

    // Range queries
    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t > '2025-01-28T18:19:14.079777Z'::timestamp ORDER BY id"
            .fetch(&mut conn);
    let rows: Vec<(i32,)> = "SELECT id FROM ts WHERE id @@@ paradedb.range('t', tsrange('2025-01-28T18:19:14.079777Z'::timestamp, NULL, '(]')) ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, expected);
}

#[rstest]
fn datetime_term_millisecond(mut conn: PgConnection) {
    r#"
    CREATE TABLE ts (id SERIAL, t TIMESTAMP(3));
    CREATE INDEX ts_idx on ts using bm25 (id, t) with (key_field = 'id');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14.078Z');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14.079Z');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14.08Z');
    "#
    .execute(&mut conn);

    // Term queries
    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14.079Z'::timestamp".fetch(&mut conn);
    let rows: Vec<(i32,)> =
        "SELECT id FROM ts WHERE id @@@ paradedb.term('t', '2025-01-28T18:19:14.079Z'::timestamp)"
            .fetch(&mut conn);
    assert_eq!(rows, expected);

    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14.079Z'::timestamp".fetch(&mut conn);
    let rows: Vec<(i32,)> =
        r#"SELECT id FROM ts WHERE t @@@ '"2025-01-28T18:19:14.079Z"'"#.fetch(&mut conn);
    assert_eq!(rows, expected);

    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14Z'::timestamp".fetch(&mut conn);
    let rows: Vec<(i32,)> =
        r#"SELECT id FROM ts WHERE t @@@ '"2025-01-28T18:19:14Z"'"#.fetch(&mut conn);
    assert_eq!(rows, expected);

    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14.078001Z'::timestamp".fetch(&mut conn);
    let rows: Vec<(i32,)> =
        r#"SELECT id FROM ts WHERE t @@@ '"2025-01-28T18:19:14.078001Z"'"#.fetch(&mut conn);
    assert_eq!(rows, expected);

    // Range queries
    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t > '2025-01-28T18:19:14.079Z'::timestamp ORDER BY id"
            .fetch(&mut conn);
    let rows: Vec<(i32,)> = "SELECT id FROM ts WHERE id @@@ paradedb.range('t', tsrange('2025-01-28T18:19:14.079Z'::timestamp, NULL, '(]')) ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, expected);

    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t > '2025-01-28T18:19:14.07Z'::timestamp ORDER BY id"
            .fetch(&mut conn);
    let rows: Vec<(i32,)> = "SELECT id FROM ts WHERE id @@@ paradedb.range('t', tsrange('2025-01-28T18:19:14.07Z'::timestamp, NULL, '(]')) ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, expected);
}

#[rstest]
fn datetime_term_second(mut conn: PgConnection) {
    r#"
    CREATE TABLE ts (id SERIAL, t TIMESTAMP(0));
    CREATE INDEX ts_idx on ts using bm25 (id, t) with (key_field = 'id');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14Z');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14.1Z');
    INSERT INTO ts (t) values ('2025-01-28T18:19:15Z');
    "#
    .execute(&mut conn);

    // Term queries
    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14Z'::timestamp ORDER BY id"
            .fetch(&mut conn);
    let rows: Vec<(i32,)> = "SELECT id FROM ts WHERE id @@@ paradedb.term('t', '2025-01-28T18:19:14Z'::timestamp) ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, expected);

    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14.1Z'::timestamp".fetch(&mut conn);
    let rows: Vec<(i32,)> =
        r#"SELECT id FROM ts WHERE t @@@ '"2025-01-28T18:19:14.1Z"'"#.fetch(&mut conn);
    assert_eq!(rows, expected);

    // Range queries
    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t > '2025-01-28T18:19:14Z'::timestamp ORDER BY id"
            .fetch(&mut conn);
    let rows: Vec<(i32,)> = "SELECT id FROM ts WHERE id @@@ paradedb.range('t', tsrange('2025-01-28T18:19:14Z'::timestamp, NULL, '(]')) ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, expected);

    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t > '2025-01-28T18:19:14.001Z'::timestamp ORDER BY id"
            .fetch(&mut conn);
    let rows: Vec<(i32,)> = "SELECT id FROM ts WHERE id @@@ paradedb.range('t', tsrange('2025-01-28T18:19:14.001Z'::timestamp, NULL, '(]')) ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, expected);
}

#[rstest]
#[case::date(
    "DATE",
    "'2200-06-15'",
    "'1700-01-01'",
    "'1980-07-04'",
    "'2200-06-15'::date"
)]
#[case::timestamp(
    "TIMESTAMP",
    "'2200-06-15 12:00:00'",
    "'1700-01-01 00:00:00'",
    "'1980-07-04 12:30:00'",
    "'2200-06-15 12:00:00'::timestamp"
)]
fn datetime_wide_range(
    mut conn: PgConnection,
    #[case] col_type: &str,
    #[case] val_future: &str,
    #[case] val_past: &str,
    #[case] val_mid: &str,
    #[case] search_val: &str,
) {
    format!(
        r#"
        CREATE TABLE wide_range (id SERIAL, v {col_type});
        INSERT INTO wide_range (v) VALUES ({val_future});
        INSERT INTO wide_range (v) VALUES ({val_past});
        INSERT INTO wide_range (v) VALUES ({val_mid});
        CREATE INDEX wide_range_idx ON wide_range USING bm25 (id, v) WITH (key_field = 'id');
        "#
    )
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        format!("SELECT id FROM wide_range WHERE id @@@ paradedb.term('v', {search_val})")
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let all_rows: Vec<(i32,)> = "SELECT id FROM wide_range ORDER BY id".fetch(&mut conn);
    assert_eq!(all_rows.len(), 3);
}

#[rstest]
#[case::future_date("DATE", "'57439-03-01'")]
#[case::ancient_date("DATE", "'0001-01-01'")]
#[case::future_timestamp("TIMESTAMP", "'57439-03-01 00:00:00'")]
#[case::ancient_timestamp("TIMESTAMP", "'0001-01-01 00:00:00'")]
fn datetime_overflow_reports_error(
    mut conn: PgConnection,
    #[case] col_type: &str,
    #[case] val: &str,
) {
    format!(
        r#"
        CREATE TABLE overflow_test (id SERIAL, v {col_type});
        INSERT INTO overflow_test (v) VALUES ({val});
        "#
    )
    .execute(&mut conn);

    let result = "CREATE INDEX overflow_test_idx ON overflow_test USING bm25 (id, v) WITH (key_field = 'id')".execute_result(&mut conn);
    assert!(
        result.is_err(),
        "expected error for {col_type} value {val} beyond Tantivy nanosecond range"
    );
}

#[rstest]
fn datetime_mutable_segment_validates_on_insert(mut conn: PgConnection) {
    r#"
    CREATE TABLE mutable_dates (id SERIAL, d DATE);
    CREATE INDEX mutable_dates_idx ON mutable_dates USING bm25 (id, d)
        WITH (key_field = 'id', mutable_segment_rows = 1000);
    "#
    .execute(&mut conn);

    let result = "INSERT INTO mutable_dates (d) VALUES ('57439-03-01')".execute_result(&mut conn);
    assert!(
        result.is_err(),
        "mutable segment insert should fail for date beyond Tantivy nanosecond range"
    );
}

#[rstest]
fn datetime_mutable_segment_accepts_valid_dates(mut conn: PgConnection) {
    r#"
    CREATE TABLE mutable_valid (id SERIAL, d DATE);
    CREATE INDEX mutable_valid_idx ON mutable_valid USING bm25 (id, d)
        WITH (key_field = 'id', mutable_segment_rows = 1000);
    INSERT INTO mutable_valid (d) VALUES ('2024-06-15');
    INSERT INTO mutable_valid (d) VALUES ('1980-01-01');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM mutable_valid WHERE id @@@ paradedb.term('d', '2024-06-15'::date)"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let all_rows: Vec<(i32,)> = "SELECT id FROM mutable_valid ORDER BY id".fetch(&mut conn);
    assert_eq!(all_rows.len(), 2);
}

#[rstest]
fn datetime_mutable_segment_validates_date_array(mut conn: PgConnection) {
    r#"
    CREATE TABLE mutable_date_arr (id SERIAL, dates DATE[]);
    CREATE INDEX mutable_date_arr_idx ON mutable_date_arr USING bm25 (id, dates)
        WITH (key_field = 'id', mutable_segment_rows = 1000);
    "#
    .execute(&mut conn);

    let result = "INSERT INTO mutable_date_arr (dates) VALUES (ARRAY['57439-03-01'::date])"
        .execute_result(&mut conn);
    assert!(
        result.is_err(),
        "mutable segment insert should fail for date array with out-of-range element"
    );

    "INSERT INTO mutable_date_arr (dates) VALUES (ARRAY['2024-06-15'::date, '1980-01-01'::date])"
        .execute(&mut conn);

    let all_rows: Vec<(i32,)> = "SELECT id FROM mutable_date_arr ORDER BY id".fetch(&mut conn);
    assert_eq!(all_rows.len(), 1);
}
