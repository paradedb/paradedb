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
fn use_ivm(mut conn: PgConnection) {
    if "CREATE EXTENSION IF NOT EXISTS pg_ivm;"
        .execute_result(&mut conn)
        .is_err()
    {
        // Test requires `pg_ivm`.
        return;
    }

    r#"
    DROP TABLE IF EXISTS test CASCADE;
    CREATE TABLE test (
        id int,
        content TEXT
    );

    DROP TABLE IF EXISTS test_view CASCADE;
    SELECT pgivm.create_immv('test_view', 'SELECT test.*, test.id + 1 as derived FROM test;');

    CREATE INDEX test_search_idx ON test_view
    USING bm25 (id, content)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Validate the DML works with/without the custom scan.
    r#"
    SET paradedb.enable_custom_scan = false;
    INSERT INTO test VALUES (1, 'pineapple sauce');
    UPDATE test SET id = id;
    "#
    .execute(&mut conn);

    r#"
    SET paradedb.enable_custom_scan = true;
    INSERT INTO test VALUES (2, 'mango sauce');
    UPDATE test SET id = id;
    "#
    .execute(&mut conn);

    // Confirm that the indexed view is queryable.
    let res: Vec<(i32, f32)> = r#"
    SELECT id, paradedb.score(id)
    FROM test_view
    WHERE test_view.content @@@ 'pineapple';
    "#
    .fetch(&mut conn);
    assert_eq!(res, vec![(1, 0.5389965)]);
}
