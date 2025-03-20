// Copyright (c) 2023-2025 Retake, Inc.
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
        CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', layer_sizes = '100kb, 1mb, 100mb');
    "#
    .execute_result(&mut conn).expect("creating table/index should not fail");

    // one might think 100 individual inserts of 1022 bytes each would get us right at 100k of
    // segment data, and while it does, LayeredMergePolicy has a fudge factor of 33% built in
    // so we actually need more to get to the point of actually merging
    for _ in 0..132 {
        // creates a segment of 1022 bytes
        "insert into layer_sizes select x from generate_series(1, 33) x;".execute(&mut conn);
    }

    // assert we actually have 132 segments and that a merge didn't happen yet
    let (nsegments,) = "select count(*) from paradedb.index_info('idxlayer_sizes');"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(nsegments, 132);

    // creates another segment of 1022 bytes, and will cause a merge based on our default layer sizes
    "insert into layer_sizes select x from generate_series(1, 33) x;".execute(&mut conn);
    let (nsegments,) = "select count(*) from paradedb.index_info('idxlayer_sizes');"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(nsegments, 1);
}
