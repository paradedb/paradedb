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

use async_std::task::block_on;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

const FAKE_OID: i64 = 99999;
const FAKE_SEGMENT: &str = "00000000-0000-0000-0000-000000000001";
const FAKE_SEGMENT_2: &str = "00000000-0000-0000-0000-000000000002";

fn clear_cache(conn: &mut PgConnection) {
    "SELECT paradedb.dsm_cache_test_clear_all()".execute(conn);
}


#[rstest]
fn dsm_cache_insert_and_lookup(mut conn: PgConnection) {
    clear_cache(&mut conn);

    // Insert a test entry
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 256)"
    )
    .execute(&mut conn);

    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn);

    assert_eq!(entries, 1);
    assert_eq!(total_bytes, 256);
}

#[rstest]
fn dsm_cache_entries_have_correct_fields(mut conn: PgConnection) {
    clear_cache(&mut conn);

    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 42, 512)"
    )
    .execute(&mut conn);

    let rows: Vec<(String, String, i32, i64)> = r#"
        SELECT segment_id, tag, sub_key, size_bytes
        FROM paradedb.dsm_cache_entries()
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 1);
    let (segment_id, tag, sub_key, size_bytes) = &rows[0];
    assert_eq!(segment_id, FAKE_SEGMENT);
    assert_eq!(tag, "Test");
    assert_eq!(*sub_key, 42);
    assert_eq!(*size_bytes, 512);
}

#[rstest]
fn dsm_cache_multiple_entries(mut conn: PgConnection) {
    clear_cache(&mut conn);

    // Two segments, same index
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 128)"
    )
    .execute(&mut conn);
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT_2}', 0, 256)"
    )
    .execute(&mut conn);

    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn);

    assert_eq!(entries, 2);
    assert_eq!(total_bytes, 128 + 256);
}

#[rstest]
fn dsm_cache_multiple_sub_keys(mut conn: PgConnection) {
    clear_cache(&mut conn);

    // Same segment, different sub_keys
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 100)"
    )
    .execute(&mut conn);
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 1, 200)"
    )
    .execute(&mut conn);

    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn);

    assert_eq!(entries, 2);
    assert_eq!(total_bytes, 300);
}

#[rstest]
fn dsm_cache_duplicate_insert_is_idempotent(mut conn: PgConnection) {
    clear_cache(&mut conn);

    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 256)"
   )
    .execute(&mut conn);
    // Insert same key again
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 256)"
    )
    .execute(&mut conn);

    let (entries, _, _): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn);

    assert_eq!(entries, 1, "duplicate insert should not create a second entry");
}

#[rstest]
fn dsm_cache_invalidate_segment(mut conn: PgConnection) {
    clear_cache(&mut conn);

    // Insert entries for two segments
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 128)"
    )
    .execute(&mut conn);
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT_2}', 0, 256)"
    )
    .execute(&mut conn);

    // Invalidate just the first segment
    format!(
        "SELECT paradedb.dsm_cache_test_invalidate_segment('{FAKE_OID}'::oid, '{FAKE_SEGMENT}')"
    )
    .execute(&mut conn);

    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn);

    assert_eq!(entries, 1, "only one segment should remain");
    assert_eq!(total_bytes, 256, "only the second segment's bytes should remain");
}

#[rstest]
fn dsm_cache_invalidate_index(mut conn: PgConnection) {
    clear_cache(&mut conn);

    // Insert entries for two segments under the same index
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 128)"
    )
    .execute(&mut conn);
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT_2}', 0, 256)"
    )
    .execute(&mut conn);

    // Invalidate the entire index
    format!(
        "SELECT paradedb.dsm_cache_test_invalidate_index('{FAKE_OID}'::oid)"
    )
    .execute(&mut conn);

    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn);

    assert_eq!(entries, 0, "all entries should be removed");
    assert_eq!(total_bytes, 0, "total bytes should be zero");
}

#[rstest]
fn dsm_cache_invalidate_segment_with_sub_keys(mut conn: PgConnection) {
    clear_cache(&mut conn);

    // Insert multiple sub_keys for the same segment
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 100)"
    )
    .execute(&mut conn);
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 1, 200)"
    )
    .execute(&mut conn);
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 2, 300)"
    )
    .execute(&mut conn);

    // Invalidate the segment — all sub_keys should be removed
    format!(
        "SELECT paradedb.dsm_cache_test_invalidate_segment('{FAKE_OID}'::oid, '{FAKE_SEGMENT}')"
    )
    .execute(&mut conn);

    let (entries, _, _): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn);

    assert_eq!(entries, 0, "all sub_keys for the segment should be removed");
}

#[rstest]
fn dsm_cache_visible_across_connections(database: Db) {
    let mut conn_a = block_on(async { database.connection().await });
    let mut conn_b = block_on(async { database.connection().await });

    "CREATE EXTENSION IF NOT EXISTS pg_search".execute(&mut conn_a);
    "CREATE EXTENSION IF NOT EXISTS pg_search".execute(&mut conn_b);
    clear_cache(&mut conn_a);

    // Connection A inserts a cache entry
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 512)"
    )
    .execute(&mut conn_a);

    // Connection B should see it (shared memory is cross-backend)
    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn_b);

    assert_eq!(entries, 1, "connection B should see connection A's entry");
    assert_eq!(total_bytes, 512);
}

#[rstest]
fn dsm_cache_invalidation_across_connections(database: Db) {
    let mut conn_a = block_on(async { database.connection().await });
    let mut conn_b = block_on(async { database.connection().await });

    "CREATE EXTENSION IF NOT EXISTS pg_search".execute(&mut conn_a);
    "CREATE EXTENSION IF NOT EXISTS pg_search".execute(&mut conn_b);
    clear_cache(&mut conn_a);

    // Connection A inserts entries
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 128)"
    )
    .execute(&mut conn_a);
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT_2}', 0, 256)"
    )
    .execute(&mut conn_a);

    // Connection B sees both
    let (entries, _, _): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn_b);
    assert_eq!(entries, 2);

    // Connection A invalidates one segment
    format!(
        "SELECT paradedb.dsm_cache_test_invalidate_segment('{FAKE_OID}'::oid, '{FAKE_SEGMENT}')"
    )
    .execute(&mut conn_a);

    // Connection B should see only one remaining
    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn_b);
    assert_eq!(entries, 1, "connection B should see the invalidation");
    assert_eq!(total_bytes, 256);
}

#[rstest]
fn dsm_cache_drop_index_clears_entries_across_connections(database: Db) {
    let mut conn_a = block_on(async { database.connection().await });
    let mut conn_b = block_on(async { database.connection().await });

    "CREATE EXTENSION IF NOT EXISTS pg_search".execute(&mut conn_a);
    "CREATE EXTENSION IF NOT EXISTS pg_search".execute(&mut conn_b);
    clear_cache(&mut conn_a);

    // Create a plain index so we have a real OID for the object_access_hook
    "CREATE TABLE test_dsm_drop (id SERIAL, body TEXT)".execute(&mut conn_a);
    "CREATE INDEX test_dsm_drop_idx ON test_dsm_drop (id)".execute(&mut conn_a);

    // Get the real index OID
    let (index_oid,): (i64,) =
        "SELECT oid::int8 FROM pg_class WHERE relname = 'test_dsm_drop_idx'"
            .fetch_one(&mut conn_a);

    // Insert fake cache entries under that real index OID
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{index_oid}'::oid, '{FAKE_SEGMENT}', 0, 128)"
    )
    .execute(&mut conn_a);
    format!(
        "SELECT paradedb.dsm_cache_test_insert('{index_oid}'::oid, '{FAKE_SEGMENT_2}', 0, 256)"
    )
    .execute(&mut conn_a);

    // Connection B sees the entries
    let (entries, _, _): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn_b);
    assert_eq!(entries, 2, "both entries should be visible before DROP");

    // Drop the index — object_access_hook should fire and invalidate
    "DROP INDEX test_dsm_drop_idx".execute(&mut conn_a);

    // Connection B should see the entries are gone
    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn_b);
    assert_eq!(entries, 0, "DROP INDEX should clear all cache entries");
    assert_eq!(total_bytes, 0);
}

#[rstest]
fn dsm_cache_refcount_keeps_mapping_alive_after_invalidation(database: Db) {
    let mut conn_a = block_on(async { database.connection().await });
    let mut conn_b = block_on(async { database.connection().await });

    "CREATE EXTENSION IF NOT EXISTS pg_search".execute(&mut conn_a);
    "CREATE EXTENSION IF NOT EXISTS pg_search".execute(&mut conn_b);
    clear_cache(&mut conn_a);

    // 1. Connection A creates an entry and HOLDS the DsmSlice
    let (held,): (bool,) = format!(
        "SELECT paradedb.dsm_cache_test_hold('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 1024)"
    )
    .fetch_one(&mut conn_a);
    assert!(held, "conn_a: should create and hold a cache entry");

    // 2. Connection B attaches to the SAME entry and holds its own DsmSlice
    let (held,): (bool,) = format!(
        "SELECT paradedb.dsm_cache_test_hold('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 1024)"
    )
    .fetch_one(&mut conn_b);
    assert!(held, "conn_b: should attach and hold the same cache entry");

    // Verify the entry is in the shared array
    let (entries, _, _): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn_a);
    assert_eq!(entries, 1);

    // 3. Invalidate the segment — removes from shared array and unpins globally
    format!(
        "SELECT paradedb.dsm_cache_test_invalidate_segment('{FAKE_OID}'::oid, '{FAKE_SEGMENT}')"
    )
    .execute(&mut conn_a);

    // Shared array should be empty now
    let (entries, _, _): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn_a);
    assert_eq!(entries, 0, "entry should be gone from shared array");

    // 4. Connection A releases its reference
    "SELECT paradedb.dsm_cache_test_release()".execute(&mut conn_a);
    let (readable,): (bool,) =
        "SELECT paradedb.dsm_cache_test_read_held()".fetch_one(&mut conn_a);
    assert!(!readable, "conn_a: should have nothing held after release");

    // 5. Connection B can STILL read — its Arc<MappingGuard> keeps the mapping alive
    let (readable,): (bool,) =
        "SELECT paradedb.dsm_cache_test_read_held()".fetch_one(&mut conn_b);
    assert!(readable, "conn_b: should still be readable after conn_a released");

    // 6. Connection B releases — last reference, dsm_detach fires
    "SELECT paradedb.dsm_cache_test_release()".execute(&mut conn_b);
    let (readable,): (bool,) =
        "SELECT paradedb.dsm_cache_test_read_held()".fetch_one(&mut conn_b);
    assert!(!readable, "conn_b: should have nothing held after release");
}
