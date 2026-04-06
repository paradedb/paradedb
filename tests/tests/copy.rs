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
async fn test_copy_to_table(mut conn: PgConnection) {
    r#"
        DROP TABLE IF EXISTS test_copy_to_table;
        CREATE TABLE test_copy_to_table (id SERIAL PRIMARY KEY, name TEXT);
        CREATE INDEX idx_test_copy_to_table ON test_copy_to_table USING bm25(id, name) WITH (key_field = 'id');
    "#.execute(&mut conn);

    let mut copyin = conn
        .copy_in_raw("COPY test_copy_to_table(name) FROM STDIN")
        .await
        .unwrap();
    copyin.send("one\ntwo\nthree".as_bytes()).await.unwrap();
    copyin.finish().await.unwrap();

    let (count,) = "SELECT COUNT(*) FROM test_copy_to_table WHERE name @@@ 'one'"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}
