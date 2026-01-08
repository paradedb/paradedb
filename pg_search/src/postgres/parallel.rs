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

use crate::api::HashSet;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::ParallelScanState;
use pgrx::{pg_guard, pg_sys};
use std::cell::RefCell;
use tantivy::index::SegmentId;

// Thread-local buffer for deferred logging - avoids synchronization during scan
thread_local! {
    static LOG_BUFFER: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

/// Buffer a log message (no synchronization)
pub fn log_buffered(msg: String) {
    LOG_BUFFER.with(|buf| buf.borrow_mut().push(msg));
}

/// Flush all buffered logs at end of scan
pub fn flush_logs() {
    LOG_BUFFER.with(|buf| {
        let mut logs = buf.borrow_mut();
        if !logs.is_empty() {
            // Print all at once
            pgrx::warning!("[parallel] {}", logs.join(" | "));
            logs.clear();
        }
    });
}

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

/// Mark that a new scan (or rescan) is starting. Called at the very start of amrescan.
///
/// For parallel scans, mark state as uninitialized so workers wait.
/// Only the leader calls this to signal that a new scan is starting.
pub unsafe fn mark_rescan_starting(mut scan: pg_sys::IndexScanDesc) {
    if (*scan).parallel_scan.is_null() {
        return;
    }

    let worker_number = pg_sys::ParallelWorkerNumber;

    // Only leader marks uninitialized
    if worker_number == -1 {
        if let Some(state) = get_bm25_scan_state(&mut scan) {
            let _mutex = state.acquire_mutex();
            state.mark_uninitialized();
        }
    }
    // Workers don't need to do anything here - they'll wait in checkout_segment
}

/// Initialize parallel scan state if not already done.
/// The first participant to acquire the mutex and see uninitialized state
/// will populate the segment pool. Segments are NOT claimed here - they're
/// claimed lazily in amgettuple via maybe_claim_segment.
pub unsafe fn maybe_init_parallel_scan(
    mut scan: pg_sys::IndexScanDesc,
    searcher: &SearchIndexReader,
) {
    if (*scan).parallel_scan.is_null() {
        return;
    }

    // Get these BEFORE mutable borrow of scan via get_bm25_scan_state
    let worker = pg_sys::ParallelWorkerNumber;
    let idx = (*(*scan).indexRelation).rd_id.to_u32();

    let state = match get_bm25_scan_state(&mut scan) {
        Some(s) => s,
        None => return,
    };

    let _mutex = state.acquire_mutex();

    if !state.is_initialized() {
        let num_segments = searcher.segment_readers().len();
        state.populate(searcher.segment_readers(), &[], false);
        log_buffered(format!(
            "INIT:W{}:idx={}:segs={}",
            worker, idx, num_segments
        ));
    } else {
        log_buffered(format!("NO_INIT:W{}:idx={}", worker, idx));
    }
}

/// Claim (steal) a segment from the shared pool.
/// Both leader and workers use this to get work.
/// Workers will wait for initialization before attempting to claim.
pub unsafe fn maybe_claim_segment(mut scan: pg_sys::IndexScanDesc) -> Option<SegmentId> {
    // Get these BEFORE mutable borrow of scan via get_bm25_scan_state
    let worker = pg_sys::ParallelWorkerNumber;
    let idx = (*(*scan).indexRelation).rd_id.to_u32();

    let state = get_bm25_scan_state(&mut scan)?;
    let nseg = state.nsegments_raw();
    let rem_before = state.remaining_raw();

    let claimed = state.checkout_segment();

    let rem_after = state.remaining_raw();
    log_buffered(format!(
        "CLAIM:W{}:idx={}:nseg={}:rem_before={}:rem_after={}:claimed={:?}",
        worker, idx, nseg, rem_before, rem_after, claimed
    ));

    claimed
}

pub unsafe fn list_segment_ids(mut scan: pg_sys::IndexScanDesc) -> Option<HashSet<SegmentId>> {
    // Workers wait for leader to initialize, then get segment IDs
    Some(
        get_bm25_scan_state(&mut scan)?
            .segments(0) // expected_scan_id is no longer used
            .keys()
            .cloned()
            .collect(),
    )
}

fn get_bm25_scan_state(scan: &mut pg_sys::IndexScanDesc) -> Option<&mut ParallelScanState> {
    unsafe {
        assert!(!scan.is_null());
        let scan = scan.as_mut().unwrap_unchecked();
        bm25_shared_state(scan)
    }
}
