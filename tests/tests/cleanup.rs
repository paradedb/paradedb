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

use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

#[rstest]
fn verify_index(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let (count,) =
        "SELECT count(*) FROM pdb.verify_index('paradedb.bm25_search_bm25_index') WHERE NOT passed"
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);
}

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

#[rstest]
fn bulk_insert_segments_behavior(mut conn: PgConnection) {
    let mutable_segment_rows = 10;
    format!(
        r#"
        SET maintenance_work_mem = '1GB';
        SET work_mem = '1GB';
        SET paradedb.global_mutable_segment_rows = -1;
        DROP TABLE IF EXISTS test_table;
        CREATE TABLE test_table (id SERIAL PRIMARY KEY, value TEXT NOT NULL);

        CREATE INDEX idxtest_table ON public.test_table
        USING paradedb (id, value)
        WITH (
            key_field = 'id',
            mutable_segment_rows = {mutable_segment_rows},
            mutable_segment_bytes = '0'
        );
    "#
    )
    .execute(&mut conn);

    // Insert less than the mutable segments size, and confirm that we have 1 segment.
    format!(
        "INSERT INTO test_table (value) SELECT md5(random()::text) FROM generate_series(1, {})",
        1
    )
    .execute(&mut conn);
    let nsegments = "SELECT COUNT(*) FROM paradedb.index_info('idxtest_table');"
        .fetch_one::<(i64,)>(&mut conn)
        .0 as usize;
    assert_eq!(nsegments, 1);

    // Insert more than the mutable segments size, and confirm that it fills the first mutable
    // segment, and then produces one additional (immutable) segment.
    format!(
        "INSERT INTO test_table (value) SELECT md5(random()::text) FROM generate_series(1, {})",
        4 * mutable_segment_rows
    )
    .execute(&mut conn);
    let nsegments = "SELECT COUNT(*) FROM paradedb.index_info('idxtest_table');"
        .fetch_one::<(i64,)>(&mut conn)
        .0 as usize;
    assert_eq!(nsegments, 2);
}

/// Large-document updates must freeze the mutable segment by the byte cap (#5950), not only
/// after `mutable_segment_rows` rows accumulate.
#[rstest]
fn mutable_segment_bytes_freeze_large_docs(mut conn: PgConnection) {
    r#"
        SET paradedb.global_mutable_segment_rows = -1;
        SET paradedb.global_mutable_segment_bytes = -1;
        SET maintenance_work_mem = '1GB';
        DROP TABLE IF EXISTS large_docs;
        CREATE TABLE large_docs (id SERIAL PRIMARY KEY, body TEXT NOT NULL);
        CREATE INDEX large_docs_idx ON large_docs
        USING paradedb (id, body)
        WITH (
            key_field = 'id',
            mutable_segment_rows = 1000,
            mutable_segment_bytes = '200kB',
            background_layer_sizes = '0'
        );
        INSERT INTO large_docs (body)
        SELECT repeat(md5(g::text), 4000) FROM generate_series(1, 5) g;
    "#
    .execute(&mut conn);

    // Each UPDATE is its own statement (~120KB+ of indexed text). After a few of them the
    // byte cap must freeze/merge the mutable buffer so we are not left with a large unfrozen
    // mutable segment holding all churn.
    for i in 1..=8 {
        format!(
            "UPDATE large_docs SET body = repeat(md5('{i}' || clock_timestamp()::text), 4000) WHERE id = (({i} % 5) + 1);"
        )
        .execute(&mut conn);
    }

    let (max_mutable_docs,): (Option<i64>,) = r#"
        SELECT COALESCE(MAX(num_docs)::bigint, 0)
        FROM paradedb.index_info('large_docs_idx')
        WHERE mutable
    "#
    .fetch_one(&mut conn);

    // With a 200kB byte cap and ~125KB docs, an unfrozen (or just-frozen) mutable segment
    // must not accumulate all 8 large updates the way a rows-only cap of 1000 would.
    assert!(
        max_mutable_docs.unwrap_or(0) <= 3,
        "expected mutable segment to freeze under byte cap, but max mutable num_docs={max_mutable_docs:?}"
    );

    // Correctness: searches still work after byte-cap freezes.
    let (cnt,): (i64,) =
        "SELECT COUNT(*) FROM large_docs WHERE body @@@ 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'"
            .fetch_one(&mut conn);
    assert_eq!(cnt, 0);
}

/// Small documents must still buffer in mutable segments under the default 1MB byte cap.
#[rstest]
fn mutable_segment_small_docs_still_buffer(mut conn: PgConnection) {
    r#"
        SET paradedb.global_mutable_segment_rows = -1;
        SET paradedb.global_mutable_segment_bytes = -1;
        DROP TABLE IF EXISTS small_docs;
        CREATE TABLE small_docs (id SERIAL PRIMARY KEY, body TEXT NOT NULL);
        CREATE INDEX small_docs_idx ON small_docs
        USING paradedb (id, body)
        WITH (key_field = 'id', mutable_segment_rows = 1000);
        INSERT INTO small_docs (body)
        SELECT md5(g::text) FROM generate_series(1, 250) g;
    "#
    .execute(&mut conn);

    let (mutable_segs, mutable_docs, max_bytes): (i64, i64, Option<i64>) = r#"
        SELECT COUNT(*), COALESCE(SUM(num_docs), 0)::bigint, COALESCE(MAX(byte_size)::bigint, 0)
        FROM paradedb.index_info('small_docs_idx')
        WHERE mutable
    "#
    .fetch_one(&mut conn);

    assert_eq!(
        mutable_segs, 1,
        "expected one mutable segment for small docs"
    );
    assert_eq!(
        mutable_docs, 250,
        "expected all 250 small docs buffered, got {mutable_docs}"
    );
    assert!(
        max_bytes.unwrap_or(0) < 64 * 1024,
        "small-doc buffer should stay well under large-doc eager threshold, got {max_bytes:?}"
    );
}

/// Issue #5950: sustained large-document UPDATE churn must not linearly degrade
/// zero-match query latency. Byte-cap freezes trigger immediate foreground merge.
#[rstest]
fn mutable_segment_bytes_bounded_query_latency(mut conn: PgConnection) {
    use serde_json::Value;

    r#"
        SET paradedb.global_mutable_segment_rows = -1;
        SET paradedb.global_mutable_segment_bytes = -1;
        SET maintenance_work_mem = '1GB';
        SET work_mem = '1GB';
        DROP TABLE IF EXISTS churn5950;
        CREATE TABLE churn5950 (
            id bigint PRIMARY KEY,
            search_text text NOT NULL,
            resource_search_text text
        );
        INSERT INTO churn5950
        SELECT g,
               'onderwerp' || (g % 50000) || ' ' || md5(g::text),
               CASE WHEN g % 3 = 0 THEN 'verslag ' || md5((g * 7)::text) END
        FROM generate_series(1, 10000) g;

        CREATE INDEX churn5950_idx ON churn5950
        USING paradedb (id, search_text, resource_search_text)
        WITH (key_field = 'id'); -- default 1MB byte cap + 64kB large-doc eager freeze
    "#
    .execute(&mut conn);

    for i in 1..=50 {
        format!(
            "UPDATE churn5950 SET resource_search_text =
                (SELECT string_agg(md5(g::text || clock_timestamp()::text), ' ')
                 FROM generate_series(1, 3600) g)
             WHERE id = (({i} * 7919) % 10000) + 1;"
        )
        .execute(&mut conn);
    }

    let (mutable_count,): (i64,) = r#"
        SELECT COUNT(*)
        FROM paradedb.index_info('churn5950_idx')
        WHERE mutable
    "#
    .fetch_one(&mut conn);
    assert_eq!(
        mutable_count, 0,
        "large-doc updates must not leave a rematerializing mutable segment, got {mutable_count}"
    );

    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT id FROM churn5950
        WHERE (search_text @@@ 'xyzzyqplonk'
               OR resource_search_text @@@ 'xyzzyqplonk')
        LIMIT 21
    "#
    .fetch_one(&mut conn);

    let execution_ms = plan[0]["Execution Time"].as_f64().unwrap_or(f64::MAX);
    // Bound is intentionally loose for CI variance (PG16 arm64 can sit ~20–40ms under load).
    // Pre-fix behavior grew into hundreds of ms / seconds as mutable bytes accumulated.
    assert!(
        execution_ms < 100.0,
        "zero-match query too slow after large-doc churn: {execution_ms}ms (issue #5950)"
    );
}
