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
        CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', layer_sizes = '100kb, 1mb, 100mb');
    "#
    .execute_result(&mut conn).expect("creating table/index should not fail");

    // one might think 100 individual inserts of 1022 bytes each would get us right at 100k of
    // segment data, and while it does, LayeredMergePolicy has a fudge factor of 33% built in
    // so we actually need more to get to the point of actually merging
    for _ in 0..165 {
        // creates a segment of 1022 bytes
        "insert into layer_sizes select x from generate_series(1, 33) x;".execute(&mut conn);
    }

    // assert we actually have 132 segments and that a merge didn't happen yet
    let (nsegments,) = "select count(*) from paradedb.index_info('idxlayer_sizes');"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(nsegments, 165);

    // creates another segment of 1022 bytes, and will cause a merge based on our default layer sizes
    // leaving behind 1 segment.  that's a merge of all the segments we created above plus the segment
    // created by this INSERT
    "insert into layer_sizes select x from generate_series(1, 33) x;".execute(&mut conn);
    let (nsegments,) = "select count(*) from paradedb.index_info('idxlayer_sizes');"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(nsegments, 1);
}

#[rstest]
fn force_merge(mut conn: PgConnection) {
    r#"
        CREATE TABLE force_merge (id bigint);
        CREATE INDEX idxforce_merge ON force_merge USING bm25(id) WITH (key_field='id', layer_sizes = '100kb, 1mb, 100mb');
    "#
    .execute_result(&mut conn).expect("creating table/index should not fail");

    // creates a segment of 481 bytes
    for i in 0..10 {
        // creates a segment of 1022 bytes
        format!("insert into force_merge (id) values ({i});").execute(&mut conn);
    }

    // assert we actually have 10 segments and that a merge didn't happen yet
    let (nsegments,) = "select count(*) from paradedb.index_info('idxforce_merge');"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(nsegments, 10);

    // force merge into a layer of 800 bytes
    let (nsegments, nmerged) = "select * from paradedb.force_merge('idxforce_merge', 800);"
        .fetch_one::<(i64, i64)>(&mut conn);
    assert_eq!(nsegments, 3);
    assert_eq!(nmerged, 9);

    // which leaves us with 4 segments
    let (nsegments,) = "select count(*) from paradedb.index_info('idxforce_merge');"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(nsegments, 4);
}

#[rstest]
fn dont_merge_create_index_segments(mut conn: PgConnection) {
    // Test that a segment created by CREATE INDEX cannot get merged away even if less than layer size
    r#"
        CREATE TABLE dont_merge_create_index_segments (id bigint);
        INSERT INTO dont_merge_create_index_segments (id) SELECT x FROM generate_series(1, 1000000) x;
        CREATE INDEX idxdont_merge_create_index_segments ON dont_merge_create_index_segments USING bm25(id) WITH (key_field='id', layer_sizes = '500kb, 2mb, 5mb, 10mb');
    "#
    .execute_result(&mut conn).expect("creating table/index should not fail");

    let (nsegments_before,) =
        "select count(*) from paradedb.index_info('idxdont_merge_create_index_segments');"
            .fetch_one::<(i64,)>(&mut conn);

    "INSERT INTO dont_merge_create_index_segments (id) VALUES (1)".execute(&mut conn);

    let (nsegments_after,) =
        "select count(*) from paradedb.index_info('idxdont_merge_create_index_segments');"
            .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(nsegments_after, nsegments_before + 1);

    // Test that deleted segments created by CREATE INDEX can get merged away
    "DELETE FROM dont_merge_create_index_segments WHERE id > 10".execute(&mut conn);
    "VACUUM dont_merge_create_index_segments".execute(&mut conn);

    let (num_deleted_before,) = "SELECT sum(num_deleted)::int8 FROM paradedb.index_info('idxdont_merge_create_index_segments');"
        .fetch_one::<(i64,)>(&mut conn);
    "INSERT INTO dont_merge_create_index_segments (id) VALUES (1)".execute(&mut conn);
    let (num_deleted_after,) = "SELECT sum(num_deleted)::int8 FROM paradedb.index_info('idxdont_merge_create_index_segments');"
        .fetch_one::<(i64,)>(&mut conn);

    assert!(num_deleted_after < num_deleted_before);
}

#[rstest]
fn force_merge_create_index_segments(mut conn: PgConnection) {
    r#"
        CREATE TABLE force_merge (id bigint);
        INSERT INTO force_merge (id) SELECT x FROM generate_series(1, 1000000) x;
        CREATE INDEX idxforce_merge ON force_merge USING bm25(id) WITH (key_field='id');
    "#
    .execute_result(&mut conn)
    .expect("creating table/index should not fail");

    let (nsegments_before,) = "select count(*) from paradedb.index_info('idxforce_merge');"
        .fetch_one::<(i64,)>(&mut conn);

    "SELECT paradedb.force_merge('idxforce_merge', '5MB');".execute(&mut conn);

    let (nsegments_after,) = "select count(*) from paradedb.index_info('idxforce_merge');"
        .fetch_one::<(i64,)>(&mut conn);

    assert!(nsegments_after < nsegments_before);
}
