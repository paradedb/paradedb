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
fn merges_to_1_100k_segment(mut conn: PgConnection) {
    r#"
        CREATE TABLE layer_sizes (id bigint);
        CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', layer_sizes = '1kb, 10kb, 100kb, 1mb',target_segment_count = 8);
    "#
    .execute_result(&mut conn).expect("creating table/index should not fail");

    for _ in 0..9 {
        "insert into layer_sizes select x from generate_series(1, 33) x;".execute(&mut conn);
    }

    // assert that a merge hasn't happened yet
    let (nsegments,) = "select count(*) from paradedb.index_info('idxlayer_sizes');"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(nsegments, 9);

    // creates another segment which will cause a merge
    "insert into layer_sizes select x from generate_series(1, 33) x;".execute(&mut conn);
    let (nsegments,) = "select count(*) from paradedb.index_info('idxlayer_sizes');"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(nsegments, 8);
}

#[rstest]
fn merge_with_no_positions(mut conn: PgConnection) {
    r#"
        CREATE TABLE test (
            id serial8,
            message text
        );
        CREATE INDEX idxtest ON test USING bm25 (id, message) WITH (key_field = 'id', target_segment_count = 8, layer_sizes = '10kb, 100kb, 1mb');
    "#
    .execute(&mut conn);

    // this will merge on the 9th insert insert
    for _ in 0..8 {
        "insert into test (message) select null from generate_series(1, 1000);".execute(&mut conn);
    }

    // and we should have 1 segment after it merges
    let (count,) =
        "select count(*) from paradedb.index_info('idxtest')".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 8);
}
