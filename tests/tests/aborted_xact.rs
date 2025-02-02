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
fn aborted_segments_not_visible(mut conn: PgConnection) {
    r#"
        SET paradedb.create_index_parallelism = 1;
        SET paradedb.statement_parallelism = 1;
        DROP TABLE IF EXISTS test_table;
        CREATE TABLE test_table (id SERIAL PRIMARY KEY, value TEXT NOT NULL);
        INSERT INTO test_table (value) VALUES ('committed');

        CREATE INDEX idxtest_table ON public.test_table
        USING bm25 (id, value)
        WITH (
            key_field = 'id',
            text_fields = '{
                "value": {}
            }'
        );
    "#
    .execute(&mut conn);

    // there's one segment, from CREATE INDEX
    let (pre_update_visible_segments,) =
        "SELECT count(*) FROM paradedb.index_info('idxtest_table')".fetch_one::<(i64,)>(&mut conn);

    assert_eq!(pre_update_visible_segments, 1);

    // this will do a merge_on_insert, creating a new segment, even tho its contents will not be
    // visible (because the xact aborted), the segment itself will be
    "BEGIN; UPDATE test_table SET value = 'aborted'; ABORT".execute(&mut conn);

    // so that means this will return two segments.  The original one made by CREATE INDEX and
    // the segment from above
    let (post_visible_segments,) =
        "SELECT count(*) FROM paradedb.index_info('idxtest_table', true)"
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(post_visible_segments, 2);

    // and even tho this will search both segments, it will not return the row from the aborted xact
    let (count,) =
        "SELECT count(*) FROM test_table WHERE value @@@ 'aborted'".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);

    // because it's supposed to only return rows from live segments
    let (count,) = "SELECT count(*) FROM test_table WHERE value @@@ 'committed'"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}
