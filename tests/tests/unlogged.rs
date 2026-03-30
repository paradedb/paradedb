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
fn unlogged_table_create_index(mut conn: PgConnection) {
    r#"
    CREATE UNLOGGED TABLE test_unlogged (
        id SERIAL PRIMARY KEY,
        description TEXT
    );
    INSERT INTO test_unlogged (description) VALUES
        ('keyboard'), ('mouse'), ('monitor');

    CREATE INDEX ON test_unlogged
    USING bm25 (id, description)
    WITH (key_field='id');
    "#
    .execute(&mut conn);
}

#[rstest]
fn unlogged_table_search(mut conn: PgConnection) {
    r#"
    CREATE UNLOGGED TABLE test_unlogged (
        id SERIAL PRIMARY KEY,
        description TEXT
    );
    INSERT INTO test_unlogged (description) VALUES
        ('keyboard'), ('mouse'), ('monitor');

    CREATE INDEX ON test_unlogged
    USING bm25 (id, description)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM test_unlogged WHERE test_unlogged @@@ 'description:keyboard'"
            .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 1);
}

#[rstest]
fn unlogged_table_insert_after_index(mut conn: PgConnection) {
    r#"
    CREATE UNLOGGED TABLE test_unlogged (
        id SERIAL PRIMARY KEY,
        description TEXT
    );
    INSERT INTO test_unlogged (description) VALUES
        ('keyboard'), ('mouse'), ('monitor');

    CREATE INDEX ON test_unlogged
    USING bm25 (id, description)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    "INSERT INTO test_unlogged (description) VALUES ('headphones')".execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM test_unlogged WHERE test_unlogged @@@ 'description:headphones'"
            .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 1);
}

#[rstest]
fn unlogged_table_update(mut conn: PgConnection) {
    r#"
    CREATE UNLOGGED TABLE test_unlogged (
        id SERIAL PRIMARY KEY,
        description TEXT
    );
    INSERT INTO test_unlogged (description) VALUES
        ('keyboard'), ('mouse'), ('monitor');

    CREATE INDEX ON test_unlogged
    USING bm25 (id, description)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    "UPDATE test_unlogged SET description = 'trackpad' WHERE description = 'mouse'"
        .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM test_unlogged WHERE test_unlogged @@@ 'description:mouse'"
            .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 0);

    let rows: Vec<(i32,)> =
        "SELECT id FROM test_unlogged WHERE test_unlogged @@@ 'description:trackpad'"
            .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 1);
}

#[rstest]
fn unlogged_table_delete(mut conn: PgConnection) {
    r#"
    CREATE UNLOGGED TABLE test_unlogged (
        id SERIAL PRIMARY KEY,
        description TEXT
    );
    INSERT INTO test_unlogged (description) VALUES
        ('keyboard'), ('mouse'), ('monitor');

    CREATE INDEX ON test_unlogged
    USING bm25 (id, description)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    "DELETE FROM test_unlogged WHERE description = 'mouse'".execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM test_unlogged WHERE test_unlogged @@@ 'description:mouse'"
            .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 0);
}
