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
fn datetime_wide_range_dates(mut conn: PgConnection) {
    r#"
    CREATE TABLE wide_dates (id SERIAL, d DATE);
    INSERT INTO wide_dates (d) VALUES ('2200-06-15');
    INSERT INTO wide_dates (d) VALUES ('1700-01-01');
    INSERT INTO wide_dates (d) VALUES ('1980-07-04');
    CREATE INDEX wide_dates_idx ON wide_dates USING bm25 (id, d) WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM wide_dates WHERE id @@@ paradedb.term('d', '2200-06-15'::date)"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(i32,)> =
        "SELECT id FROM wide_dates WHERE id @@@ paradedb.term('d', '1700-01-01'::date)"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let all_rows: Vec<(i32,)> = "SELECT id FROM wide_dates ORDER BY id".fetch(&mut conn);
    assert_eq!(all_rows.len(), 3);
}

#[rstest]
fn datetime_wide_range_timestamps(mut conn: PgConnection) {
    r#"
    CREATE TABLE wide_ts (id SERIAL, t TIMESTAMP);
    INSERT INTO wide_ts (t) VALUES ('2200-06-15 12:00:00');
    INSERT INTO wide_ts (t) VALUES ('1700-01-01 00:00:00');
    INSERT INTO wide_ts (t) VALUES ('1980-07-04 12:30:00');
    CREATE INDEX wide_ts_idx ON wide_ts USING bm25 (id, t) WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM wide_ts WHERE id @@@ paradedb.term('t', '2200-06-15 12:00:00'::timestamp)"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let all_rows: Vec<(i32,)> = "SELECT id FROM wide_ts ORDER BY id".fetch(&mut conn);
    assert_eq!(all_rows.len(), 3);
}

#[rstest]
fn datetime_overflow_date_reports_error(mut conn: PgConnection) {
    r#"
    CREATE TABLE overflow_dates (id SERIAL, d DATE);
    INSERT INTO overflow_dates (d) VALUES ('57439-03-01');
    "#
    .execute(&mut conn);

    let result = r#"
    CREATE INDEX overflow_dates_idx ON overflow_dates USING bm25 (id, d) WITH (key_field = 'id');
    "#
    .execute_result(&mut conn);
    assert!(
        result.is_err(),
        "expected error for date beyond Tantivy nanosecond range"
    );
}

#[rstest]
fn datetime_overflow_ancient_date_reports_error(mut conn: PgConnection) {
    r#"
    CREATE TABLE ancient_dates (id SERIAL, d DATE);
    INSERT INTO ancient_dates (d) VALUES ('0001-01-01');
    "#
    .execute(&mut conn);

    let result = r#"
    CREATE INDEX ancient_dates_idx ON ancient_dates USING bm25 (id, d) WITH (key_field = 'id');
    "#
    .execute_result(&mut conn);
    assert!(
        result.is_err(),
        "expected error for date beyond Tantivy nanosecond range"
    );
}

#[rstest]
fn datetime_overflow_timestamp_reports_error(mut conn: PgConnection) {
    r#"
    CREATE TABLE overflow_ts (id SERIAL, t TIMESTAMP);
    INSERT INTO overflow_ts (t) VALUES ('57439-03-01 00:00:00');
    "#
    .execute(&mut conn);

    let result = r#"
    CREATE INDEX overflow_ts_idx ON overflow_ts USING bm25 (id, t) WITH (key_field = 'id');
    "#
    .execute_result(&mut conn);
    assert!(
        result.is_err(),
        "expected error for timestamp beyond Tantivy nanosecond range"
    );
}

#[rstest]
fn datetime_overflow_ancient_timestamp_reports_error(mut conn: PgConnection) {
    r#"
    CREATE TABLE ancient_ts (id SERIAL, t TIMESTAMP);
    INSERT INTO ancient_ts (t) VALUES ('0001-01-01 00:00:00');
    "#
    .execute(&mut conn);

    let result = r#"
    CREATE INDEX ancient_ts_idx ON ancient_ts USING bm25 (id, t) WITH (key_field = 'id');
    "#
    .execute_result(&mut conn);
    assert!(
        result.is_err(),
        "expected error for timestamp beyond Tantivy nanosecond range"
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
