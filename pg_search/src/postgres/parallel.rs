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

use crate::index::reader::index::SearchIndexReader;
use crate::postgres::ParallelScanState;
use pgrx::{pg_guard, pg_sys};
use tantivy::index::SegmentId;

#[pg_guard]
pub unsafe extern "C-unwind" fn aminitparallelscan(target: *mut ::core::ffi::c_void) {
    let state = target.cast::<ParallelScanState>();
    (*state).create();
}

#[pg_guard]
pub unsafe extern "C-unwind" fn amparallelrescan(scan: pg_sys::IndexScanDesc) {
    // PostgreSQL calls this before a rescan to reset the parallel scan state.
    // Mark as uninitialized so workers wait for leader to re-populate.
    if let Some(state) = get_bm25_scan_state(&mut (scan as *mut _)) {
        let _mutex = state.acquire_mutex();
        state.mark_uninitialized();
    }
}

#[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
#[pg_guard]
pub unsafe extern "C-unwind" fn amestimateparallelscan() -> pg_sys::Size {
    ParallelScanState::size_of(u16::MAX as usize, &[], false)
}

#[cfg(feature = "pg17")]
#[pg_guard]
pub unsafe extern "C-unwind" fn amestimateparallelscan(
    _nkeys: i32,
    _norderbys: i32,
) -> pg_sys::Size {
    // NB:  in this function, we have no idea how many segments we have.  We don't even know which
    // index we're querying.  So we choose a, hopefully, large enough value at 65536, or u16::MAX
    // TODO: This will result in a ~1MB allocation.
    ParallelScanState::size_of(u16::MAX as usize, &[], false)
}

#[cfg(feature = "pg18")]
#[pg_guard]
pub unsafe extern "C-unwind" fn amestimateparallelscan(
    rel: *mut pg_sys::RelationData,
    _nkeys: i32,
    _norderbys: i32,
) -> pg_sys::Size {
    // In PG18, we have access to the relation, so we can get a better estimate
    // using target_segment_count() instead of the worst-case u16::MAX
    let nsegments = if rel.is_null() {
        u16::MAX as usize
    } else {
        crate::postgres::options::BM25IndexOptions::from_relation(rel).target_segment_count()
    };
    ParallelScanState::size_of(nsegments, &[], false)
}

unsafe fn bm25_shared_state(
    scan: &mut pg_sys::IndexScanDescData,
) -> Option<&mut ParallelScanState> {
    if scan.parallel_scan.is_null() {
        None
    } else {
        scan.parallel_scan
            .cast::<std::ffi::c_void>()
            .add({
                #[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16", feature = "pg17"))]
                {
                    (*scan.parallel_scan).ps_offset
                }
                #[cfg(feature = "pg18")]
                {
                    (*scan.parallel_scan).ps_offset_am
                }
            })
            .cast::<ParallelScanState>()
            .as_mut()
    }
}

/// Initialize parallel scan state if not already done.
/// The first participant to acquire the mutex and see uninitialized state
/// will populate the segment pool. Segments are NOT claimed here - they're
/// claimed lazily in amgettuple/amgetbitmap via maybe_claim_segment.
pub unsafe fn maybe_init_parallel_scan(
    mut scan: pg_sys::IndexScanDesc,
    searcher: &SearchIndexReader,
) -> Option<i32> {
    if unsafe { (*scan).parallel_scan.is_null() } {
        // not a parallel scan, so there's nothing to initialize
        return None;
    }

    let state = get_bm25_scan_state(&mut scan)?;

    let _mutex = state.acquire_mutex();

    if !state.is_initialized() {
        state.populate(searcher.segment_readers(), &[], false);
    }
    Some(unsafe { pg_sys::ParallelWorkerNumber })
}

/// Claim (steal) a segment from the shared pool.
/// Both leader and workers use this to get work.
/// All participants wait for initialization before attempting to claim.
pub unsafe fn maybe_claim_segment(mut scan: pg_sys::IndexScanDesc) -> Option<SegmentId> {
    get_bm25_scan_state(&mut scan)?.checkout_segment()
}

fn get_bm25_scan_state(scan: &mut pg_sys::IndexScanDesc) -> Option<&mut ParallelScanState> {
    unsafe {
        assert!(!scan.is_null());
        let scan = scan.as_mut().unwrap_unchecked();
        bm25_shared_state(scan)
    }
}
