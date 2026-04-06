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
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn select_everything(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value text
    );
    INSERT INTO test_table (value) VALUES ('beer'), ('wine'), ('cheese');
    CREATE INDEX test_index ON test_table USING bm25 (id, value) WITH (key_field='id');
    "#
    .execute(&mut conn);

    r#"set paradedb.enable_custom_scan to off; set max_parallel_workers_per_gather = 0;"#
        .execute(&mut conn);
    let (count,) = r#"SELECT count(*) FROM test_table WHERE id @@@ paradedb.all() OR id > 0"#
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 3);
}

#[rstest]
fn query_empty_table(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS test_table;
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value text[]
    );

    CREATE INDEX test_index ON test_table
    USING bm25 (id, value) WITH (key_field='id', text_fields='{"value": {}}');
    "#
    .execute(&mut conn);

    "SET max_parallel_workers = 0;".execute(&mut conn);
    let (count,) =
        "SELECT count(*) FROM test_table WHERE value @@@ 'beer';".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);

    "SET max_parallel_workers = 8;".execute(&mut conn);
    if pg_major_version(&mut conn) >= 16 {
        "SET debug_parallel_query TO on".execute(&mut conn);
    }
    let (count,) =
        "SELECT count(*) FROM test_table WHERE value @@@ 'beer';".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);
}

#[rstest]
fn unary_not_issue2141(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value text[]
    );

    INSERT INTO test_table (value) VALUES (ARRAY['beer', 'cheese']), (ARRAY['beer', 'wine']), (ARRAY['beer']), (ARRAY['beer']);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value) WITH (key_field='id', text_fields='{"value": {}}');
    "#
    .execute(&mut conn);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE value @@@ 'beer';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 4);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE NOT value @@@ 'beer';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE value @@@ 'wine';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE NOT value @@@ 'wine';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 3);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE value @@@ 'wine' AND NOT value @@@ 'cheese';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE NOT value @@@ 'wine' OR NOT value @@@ 'missing';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 4);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE NOT value @@@ 'wine' AND NOT value @@@ 'cheese';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 2);
}
