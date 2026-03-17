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

//! SQL-callable functions for the DSM cache: diagnostics and test helpers.

use super::{
    get_or_create, invalidate_index, invalidate_segment, load_shared, tag_name, CacheKey, CacheTag,
    DsmSlice,
};
use pgrx::pg_sys;
use pgrx::prelude::*;
use std::cell::RefCell;

// ---------------------------------------------------------------------------
// Test-only SQL functions
// ---------------------------------------------------------------------------

#[cfg(any(test, feature = "pg_test"))]
thread_local! {
    /// Stash for holding a DsmSlice across SQL calls, used to test refcounting.
    static HELD_SLICE: RefCell<Option<DsmSlice>> = RefCell::new(None);
}

/// Test-only: insert a cache entry with the Test tag and a payload of `size` zero-bytes.
#[cfg(any(test, feature = "pg_test"))]
#[pg_extern]
fn pg_test_dsm_cache_insert(index_oid: pg_sys::Oid, segment_id: String, sub_key: i32, size: i32) {
    let uuid = uuid::Uuid::parse_str(&segment_id).expect("invalid segment_id UUID");
    let key = CacheKey {
        index_oid,
        segment_id: *uuid.as_bytes(),
        tag: CacheTag::Test,
        sub_key: sub_key as u32,
    };
    get_or_create(&key, size as usize, |buf| buf.fill(0));
}

/// Test-only: invalidate all cache entries for a given segment.
#[cfg(any(test, feature = "pg_test"))]
#[pg_extern]
fn pg_test_dsm_cache_invalidate_segment(index_oid: pg_sys::Oid, segment_id: String) {
    let uuid = uuid::Uuid::parse_str(&segment_id).expect("invalid segment_id UUID");
    invalidate_segment(index_oid, uuid.as_bytes());
}

/// Test-only: invalidate all cache entries for a given index.
#[cfg(any(test, feature = "pg_test"))]
#[pg_extern]
fn pg_test_dsm_cache_invalidate_index(index_oid: pg_sys::Oid) {
    invalidate_index(index_oid);
}

/// Test-only: remove all entries from the cache.
#[cfg(any(test, feature = "pg_test"))]
#[pg_extern]
fn pg_test_dsm_cache_clear_all() {
    let Some(shared) = load_shared() else {
        return;
    };
    let handles = shared.remove_matching(|_| true);
    for handle in handles {
        unsafe {
            pg_sys::dsm_unpin_segment(handle);
        }
    }
    shared.bump_generation();
}

/// Test-only: create a cache entry and hold the DsmSlice in a thread-local stash.
/// Writes a known pattern (0xAB) so we can verify reads later.
/// Returns true if the entry was created successfully.
#[cfg(any(test, feature = "pg_test"))]
#[pg_extern]
fn pg_test_dsm_cache_hold(
    index_oid: pg_sys::Oid,
    segment_id: String,
    sub_key: i32,
    size: i32,
) -> bool {
    let uuid = uuid::Uuid::parse_str(&segment_id).expect("invalid segment_id UUID");
    let key = CacheKey {
        index_oid,
        segment_id: *uuid.as_bytes(),
        tag: CacheTag::Test,
        sub_key: sub_key as u32,
    };
    let slice = get_or_create(&key, size as usize, |buf| buf.fill(0xAB));
    let ok = slice.is_some();
    HELD_SLICE.with(|h| *h.borrow_mut() = slice);
    ok
}

/// Test-only: read from the held DsmSlice.
/// Returns true if the held slice is still readable and contains the expected pattern.
#[cfg(any(test, feature = "pg_test"))]
#[pg_extern]
fn pg_test_dsm_cache_read_held() -> bool {
    HELD_SLICE.with(|h| {
        let borrow = h.borrow();
        match borrow.as_ref() {
            Some(slice) => {
                let bytes: &[u8] = slice;
                !bytes.is_empty() && bytes.iter().all(|&b| b == 0xAB)
            }
            None => false,
        }
    })
}

/// Test-only: drop the held DsmSlice, releasing the Arc<MappingGuard>.
#[cfg(any(test, feature = "pg_test"))]
#[pg_extern]
fn pg_test_dsm_cache_release() {
    HELD_SLICE.with(|h| *h.borrow_mut() = None);
}

// ---------------------------------------------------------------------------
// SQL-callable diagnostics
// ---------------------------------------------------------------------------

#[pg_extern]
fn dsm_cache_entries() -> TableIterator<
    'static,
    (
        name!(index_oid, pg_sys::Oid),
        name!(segment_id, String),
        name!(tag, String),
        name!(sub_key, i32),
        name!(dsm_handle, i64),
        name!(size_bytes, i64),
    ),
> {
    let Some(shared) = load_shared() else {
        return TableIterator::new(Vec::new());
    };

    let mut rows = Vec::new();

    unsafe {
        let _guard = shared.lock.acquire_shared();
        for slot in shared.slot_slice() {
            if slot.key.is_empty() {
                break;
            }
            let seg_uuid = uuid::Uuid::from_bytes(slot.key.segment_id);
            rows.push((
                slot.key.index_oid,
                seg_uuid.to_string(),
                tag_name(slot.key.tag),
                slot.key.sub_key as i32,
                slot.handle as i64,
                slot.size as i64,
            ));
        }
    }

    TableIterator::new(rows)
}

#[pg_extern]
fn dsm_cache_stats() -> TableIterator<
    'static,
    (
        name!(entries, i64),
        name!(max_entries, i64),
        name!(total_bytes, i64),
    ),
> {
    let Some(shared) = load_shared() else {
        return TableIterator::once((0i64, 0i64, 0i64));
    };

    let mut count = 0i64;
    let mut total_bytes = 0i64;

    unsafe {
        let _guard = shared.lock.acquire_shared();
        for slot in shared.slot_slice() {
            if slot.key.is_empty() {
                break;
            }
            count += 1;
            total_bytes += slot.size as i64;
        }

        let max_entries = (*shared.header).max_entries as i64;

        TableIterator::once((count, max_entries, total_bytes))
    }
}
