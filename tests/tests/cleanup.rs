// Copyright (c) 2023-2024 Retake, Inc.
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
fn vacuum_full(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    "DELETE FROM paradedb.bm25_search WHERE id IN (1, 2, 3, 4, 5)".execute(&mut conn);

    "VACUUM FULL".execute(&mut conn);
}

#[rstest]
fn create_and_drop_builtin_index(mut conn: PgConnection) {
    // Test to ensure that dropping non-search indexes works correctly, as our event
    // trigger will need to skip indexes we didn't create.

    "CREATE TABLE test_table (id SERIAL PRIMARY KEY, value TEXT NOT NULL)".execute(&mut conn);

    "CREATE INDEX test_table_value_idx ON test_table(value)".execute(&mut conn);

    "DROP INDEX test_table_value_idx CASCADE".execute(&mut conn);

    let index_count = "SELECT COUNT(*) FROM pg_indexes WHERE indexname = 'test_table_value_idx'"
        .fetch_one::<(i64,)>(&mut conn)
        .0;

    assert_eq!(
        index_count, 0,
        "Index should no longer exist after dropping with CASCADE"
    );

    "DROP TABLE IF EXISTS test_table CASCADE".execute(&mut conn);
}

/// Tests that CREATE INDEX and REINDEX and VACUUM merge down to the proper number of segments, based on our
/// [`NPlusOne`] merge policy
#[rstest]
fn segment_count_correct_after_merge(mut conn: PgConnection) {
    r#"
        DROP TABLE IF EXISTS test_table;
        CREATE TABLE test_table (id SERIAL PRIMARY KEY, value TEXT NOT NULL);
        INSERT INTO test_table (value) SELECT md5(random()::text) FROM generate_series(1, 10000);

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
    let nsegments = "SELECT COUNT(*) FROM paradedb.index_info('idxtest_table');"
        .fetch_one::<(i64,)>(&mut conn)
        .0 as usize;
    assert_eq!(nsegments, 8); // '8' is our default value for `paradedb.create_index_parallelism` GUC

    // we now want to target just 2 segments
    "ALTER INDEX idxtest_table SET (target_segment_count = 2);".execute(&mut conn);

    // reindexing gets us 2 segments b/c that's our target *and* its less than the default parallelism
    "REINDEX INDEX idxtest_table;".execute(&mut conn);
    let nsegments = "SELECT COUNT(*) FROM paradedb.index_info('idxtest_table');"
        .fetch_one::<(i64,)>(&mut conn)
        .0 as usize;
    assert_eq!(nsegments, 2);

    // same thing here.  do full table update and then VACUUM should get us back to 2 segments
    "UPDATE test_table SET value = value || ' ';".execute(&mut conn);
    "VACUUM test_table;".execute(&mut conn);
    let nsegments = "SELECT COUNT(*) FROM paradedb.index_info('idxtest_table');"
        .fetch_one::<(i64,)>(&mut conn)
        .0 as usize;
    assert!(nsegments <= 3);
}
