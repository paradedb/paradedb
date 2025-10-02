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

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn expression_paradedb_func(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb');

    CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, lower(description)) WITH (key_field='id');

    INSERT INTO paradedb.index_config (description) VALUES ('Test description');
    "#
    .execute(&mut conn);

    let (count,) =
        "SELECT count(*) FROM paradedb.index_config WHERE index_config @@@ paradedb.term('_pg_search_1', 'test')"
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) = "SELECT count(*) FROM paradedb.index_config WHERE lower(description) @@@ 'test'"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}

#[rstest]
fn expression_paradedb_op(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb');

    CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, (description || ' with cats')) WITH (key_field='id');

    INSERT INTO paradedb.index_config (description) VALUES ('Test description');
    "#
    .execute(&mut conn);

    // All entries in the index should match, since all of them now have cats.
    let (count,) =
        "SELECT count(*) FROM paradedb.index_config WHERE (description || ' with cats') @@@ 'cats'"
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 42);
    // Inserted test value still should too.
    let (count,) =
        "SELECT count(*) FROM paradedb.index_config WHERE (description || ' with cats') @@@ 'description'"
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}

#[rstest]
fn expression_conflicting_query_string(mut conn: PgConnection) {
    r#"
    CREATE TABLE expression_test (id SERIAL PRIMARY KEY, firstname TEXT, lastname TEXT);

    CREATE INDEX expression_test_idx ON expression_test
        USING bm25 (id, lower(firstname), lower(lastname)) WITH (key_field='id');

    INSERT INTO expression_test (firstname, lastname) VALUES ('John', 'Doe');
    "#
    .execute(&mut conn);

    let (count,) = "SELECT count(*) FROM expression_test WHERE lower(firstname) @@@ 'john'"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) = "SELECT count(*) FROM expression_test WHERE lower(lastname) @@@ 'doe'"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) =
        "SELECT count(*) FROM expression_test WHERE lower(firstname) @@@ 'john' AND lower(lastname) @@@ 'doe'"
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}
