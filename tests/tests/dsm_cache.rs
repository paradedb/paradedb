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
use serial_test::serial;
use sqlx::PgConnection;

const FAKE_OID: i64 = 99999;
const FAKE_SEGMENT: &str = "00000000-0000-0000-0000-000000000001";
const FAKE_SEGMENT_2: &str = "00000000-0000-0000-0000-000000000002";

fn new_conn() -> (Db, PgConnection) {
    let db = block_on(async { Db::new().await });
    let mut conn = block_on(async { db.connection().await });
    "CREATE EXTENSION IF NOT EXISTS pg_search".execute(&mut conn);
    (db, conn)
}

fn clear_cache(conn: &mut PgConnection) {
    "SELECT paradedb.pg_test_dsm_cache_clear_all()".execute(conn);
}

#[test]
#[serial]
fn dsm_cache_insert_and_lookup() {
    let (_db, ref mut conn) = new_conn();
    clear_cache(conn);

    // Insert a test entry
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 256)"
    )
    .execute(conn);

    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(conn);

    assert_eq!(entries, 1);
    assert_eq!(total_bytes, 256);
}

#[test]
#[serial]
fn dsm_cache_entries_have_correct_fields() {
    let (_db, ref mut conn) = new_conn();
    clear_cache(conn);

    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 42, 512)"
    )
    .execute(conn);

    let rows: Vec<(String, String, i32, i64)> = r#"
        SELECT segment_id, tag, sub_key, size_bytes
        FROM paradedb.dsm_cache_entries()
    "#
    .fetch(conn);

    assert_eq!(rows.len(), 1);
    let (segment_id, tag, sub_key, size_bytes) = &rows[0];
    assert_eq!(segment_id, FAKE_SEGMENT);
    assert_eq!(tag, "Test");
    assert_eq!(*sub_key, 42);
    assert_eq!(*size_bytes, 512);
}

#[test]
#[serial]
fn dsm_cache_multiple_entries() {
    let (_db, ref mut conn) = new_conn();
    clear_cache(conn);

    // Two segments, same index
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 128)"
    )
    .execute(conn);
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT_2}', 0, 256)"
    )
    .execute(conn);

    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(conn);

    assert_eq!(entries, 2);
    assert_eq!(total_bytes, 128 + 256);
}

#[test]
#[serial]
fn dsm_cache_multiple_sub_keys() {
    let (_db, ref mut conn) = new_conn();
    clear_cache(conn);

    // Same segment, different sub_keys
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 100)"
    )
    .execute(conn);
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 1, 200)"
    )
    .execute(conn);

    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(conn);

    assert_eq!(entries, 2);
    assert_eq!(total_bytes, 300);
}

#[test]
#[serial]
fn dsm_cache_duplicate_insert_is_idempotent() {
    let (_db, ref mut conn) = new_conn();
    clear_cache(conn);

    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 256)"
    )
    .execute(conn);
    // Insert same key again
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 256)"
    )
    .execute(conn);

    let (entries, _, _): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(conn);

    assert_eq!(
        entries, 1,
        "duplicate insert should not create a second entry"
    );
}

#[test]
#[serial]
fn dsm_cache_invalidate_segment() {
    let (_db, ref mut conn) = new_conn();
    clear_cache(conn);

    // Insert entries for two segments
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 128)"
    )
    .execute(conn);
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT_2}', 0, 256)"
    )
    .execute(conn);

    // Invalidate just the first segment
    format!(
        "SELECT paradedb.pg_test_dsm_cache_invalidate_segment('{FAKE_OID}'::oid, '{FAKE_SEGMENT}')"
    )
    .execute(conn);

    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(conn);

    assert_eq!(entries, 1, "only one segment should remain");
    assert_eq!(
        total_bytes, 256,
        "only the second segment's bytes should remain"
    );
}

#[test]
#[serial]
fn dsm_cache_invalidate_index() {
    let (_db, ref mut conn) = new_conn();
    clear_cache(conn);

    // Insert entries for two segments under the same index
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 128)"
    )
    .execute(conn);
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT_2}', 0, 256)"
    )
    .execute(conn);

    // Invalidate the entire index
    format!("SELECT paradedb.pg_test_dsm_cache_invalidate_index('{FAKE_OID}'::oid)").execute(conn);

    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(conn);

    assert_eq!(entries, 0, "all entries should be removed");
    assert_eq!(total_bytes, 0, "total bytes should be zero");
}

#[test]
#[serial]
fn dsm_cache_invalidate_segment_with_sub_keys() {
    let (_db, ref mut conn) = new_conn();
    clear_cache(conn);

    // Insert multiple sub_keys for the same segment
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 100)"
    )
    .execute(conn);
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 1, 200)"
    )
    .execute(conn);
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 2, 300)"
    )
    .execute(conn);

    // Invalidate the segment — all sub_keys should be removed
    format!(
        "SELECT paradedb.pg_test_dsm_cache_invalidate_segment('{FAKE_OID}'::oid, '{FAKE_SEGMENT}')"
    )
    .execute(conn);

    let (entries, _, _): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(conn);

    assert_eq!(entries, 0, "all sub_keys for the segment should be removed");
}

#[test]
#[serial]
fn dsm_cache_visible_across_connections() {
    let (db, ref mut conn_a) = new_conn();
    let mut conn_b = block_on(async { db.connection().await });
    "CREATE EXTENSION IF NOT EXISTS pg_search".execute(&mut conn_b);
    clear_cache(conn_a);

    // Connection A inserts a cache entry
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 512)"
    )
    .execute(conn_a);

    // Connection B should see it (shared memory is cross-backend)
    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn_b);

    assert_eq!(entries, 1, "connection B should see connection A's entry");
    assert_eq!(total_bytes, 512);
}

#[test]
#[serial]
fn dsm_cache_invalidation_across_connections() {
    let (db, ref mut conn_a) = new_conn();
    let mut conn_b = block_on(async { db.connection().await });
    "CREATE EXTENSION IF NOT EXISTS pg_search".execute(&mut conn_b);
    clear_cache(conn_a);

    // Connection A inserts entries
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 128)"
    )
    .execute(conn_a);
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{FAKE_OID}'::oid, '{FAKE_SEGMENT_2}', 0, 256)"
    )
    .execute(conn_a);

    // Connection B sees both
    let (entries, _, _): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn_b);
    assert_eq!(entries, 2);

    // Connection A invalidates one segment
    format!(
        "SELECT paradedb.pg_test_dsm_cache_invalidate_segment('{FAKE_OID}'::oid, '{FAKE_SEGMENT}')"
    )
    .execute(conn_a);

    // Connection B should see only one remaining
    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn_b);
    assert_eq!(entries, 1, "connection B should see the invalidation");
    assert_eq!(total_bytes, 256);
}

#[test]
#[serial]
fn dsm_cache_drop_index_clears_entries_across_connections() {
    let (db, ref mut conn_a) = new_conn();
    let mut conn_b = block_on(async { db.connection().await });
    "CREATE EXTENSION IF NOT EXISTS pg_search".execute(&mut conn_b);
    clear_cache(conn_a);

    // Create a plain index so we have a real OID for the object_access_hook
    "CREATE TABLE test_dsm_drop (id SERIAL, body TEXT)".execute(conn_a);
    "CREATE INDEX test_dsm_drop_idx ON test_dsm_drop (id)".execute(conn_a);

    // Get the real index OID
    let (index_oid,): (i64,) =
        "SELECT oid::int8 FROM pg_class WHERE relname = 'test_dsm_drop_idx'".fetch_one(conn_a);

    // Insert fake cache entries under that real index OID
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{index_oid}'::oid, '{FAKE_SEGMENT}', 0, 128)"
    )
    .execute(conn_a);
    format!(
        "SELECT paradedb.pg_test_dsm_cache_insert('{index_oid}'::oid, '{FAKE_SEGMENT_2}', 0, 256)"
    )
    .execute(conn_a);

    // Connection B sees the entries
    let (entries, _, _): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn_b);
    assert_eq!(entries, 2, "both entries should be visible before DROP");

    // Drop the index — object_access_hook should fire and invalidate
    "DROP INDEX test_dsm_drop_idx".execute(conn_a);

    // Connection B should see the entries are gone
    let (entries, _, total_bytes): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(&mut conn_b);
    assert_eq!(entries, 0, "DROP INDEX should clear all cache entries");
    assert_eq!(total_bytes, 0);
}

#[test]
#[serial]
fn dsm_cache_refcount_keeps_mapping_alive_after_invalidation() {
    let (db, ref mut conn_a) = new_conn();
    let mut conn_b = block_on(async { db.connection().await });
    "CREATE EXTENSION IF NOT EXISTS pg_search".execute(&mut conn_b);
    clear_cache(conn_a);

    // 1. Connection A creates an entry and HOLDS the DsmSlice
    let (held,): (bool,) = format!(
        "SELECT paradedb.pg_test_dsm_cache_hold('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 1024)"
    )
    .fetch_one(conn_a);
    assert!(held, "conn_a: should create and hold a cache entry");

    // 2. Connection B attaches to the SAME entry and holds its own DsmSlice
    let (held,): (bool,) = format!(
        "SELECT paradedb.pg_test_dsm_cache_hold('{FAKE_OID}'::oid, '{FAKE_SEGMENT}', 0, 1024)"
    )
    .fetch_one(&mut conn_b);
    assert!(held, "conn_b: should attach and hold the same cache entry");

    // Verify the entry is in the shared array
    let (entries, _, _): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(conn_a);
    assert_eq!(entries, 1);

    // 3. Invalidate the segment — removes from shared array and unpins globally
    format!(
        "SELECT paradedb.pg_test_dsm_cache_invalidate_segment('{FAKE_OID}'::oid, '{FAKE_SEGMENT}')"
    )
    .execute(conn_a);

    // Shared array should be empty now
    let (entries, _, _): (i64, i64, i64) =
        "SELECT * FROM paradedb.dsm_cache_stats()".fetch_one(conn_a);
    assert_eq!(entries, 0, "entry should be gone from shared array");

    // 4. Connection A releases its reference
    "SELECT paradedb.pg_test_dsm_cache_release()".execute(conn_a);
    let (readable,): (bool,) = "SELECT paradedb.pg_test_dsm_cache_read_held()".fetch_one(conn_a);
    assert!(!readable, "conn_a: should have nothing held after release");

    // 5. Connection B can STILL read — its Arc<MappingGuard> keeps the mapping alive
    let (readable,): (bool,) =
        "SELECT paradedb.pg_test_dsm_cache_read_held()".fetch_one(&mut conn_b);
    assert!(
        readable,
        "conn_b: should still be readable after conn_a released"
    );

    // 6. Connection B releases — last reference, dsm_detach fires
    "SELECT paradedb.pg_test_dsm_cache_release()".execute(&mut conn_b);
    let (readable,): (bool,) =
        "SELECT paradedb.pg_test_dsm_cache_read_held()".fetch_one(&mut conn_b);
    assert!(!readable, "conn_b: should have nothing held after release");
}
