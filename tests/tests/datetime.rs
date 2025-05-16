// Copyright (c) 2023-2025 ParadeDB, Inc.
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
