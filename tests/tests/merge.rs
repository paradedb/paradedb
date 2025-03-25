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
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn merge_with_no_positions(mut conn: PgConnection) {
    r#"
        CREATE TABLE test (
            id serial8,
            message text
        );
        CREATE INDEX idxtest ON test USING bm25 (id, message) WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // this will merge on the 12th insert
    for _ in 0..12 {
        "insert into test (message) select null from generate_series(1, 1000);".execute(&mut conn);
    }

    // and we should have 1 segment after it merges
    let (count,) =
        "select count(*) from paradedb.index_info('idxtest')".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}
