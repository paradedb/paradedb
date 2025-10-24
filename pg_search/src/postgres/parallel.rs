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

use crate::api::HashSet;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::ParallelScanState;
use pgrx::{pg_guard, pg_sys};
use tantivy::index::SegmentId;

#[pg_guard]
pub unsafe extern "C-unwind" fn aminitparallelscan(target: *mut ::core::ffi::c_void) {
    let state = target.cast::<ParallelScanState>();
    (*state).init_mutex();
}

#[pg_guard]
pub unsafe extern "C-unwind" fn amparallelrescan(_scan: pg_sys::IndexScanDesc) {}

#[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
#[pg_guard]
pub unsafe extern "C-unwind" fn amestimateparallelscan() -> pg_sys::Size {
    ParallelScanState::size_of(u16::MAX as usize, &[])
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
    ParallelScanState::size_of(u16::MAX as usize, &[])
}

#[cfg(feature = "pg18")]
#[pg_guard]
pub unsafe extern "C-unwind" fn amestimateparallelscan(
    _rel: *mut pg_sys::RelationData,
    _nkeys: i32,
    _norderbys: i32,
) -> pg_sys::Size {
    // NB:  in this function, we have no idea how many segments we have.  We don't even know which
    // index we're querying.  So we choose a, hopefully, large enough value at 65536, or u16::MAX
    // TODO: This will result in a ~1MB allocation.
    ParallelScanState::size_of(u16::MAX as usize, &[])
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

pub unsafe fn maybe_init_parallel_scan(
    mut scan: pg_sys::IndexScanDesc,
    searcher: &SearchIndexReader,
) -> Option<i32> {
    if unsafe { (*scan).parallel_scan.is_null() } {
        // not a parallel scan, so there's nothing to initialize
        return None;
    }

    let state = get_bm25_scan_state(&mut scan)?;
    let worker_number = unsafe { pg_sys::ParallelWorkerNumber };
    let _mutex = state.acquire_mutex();
    if worker_number == -1 {
        // ParallelWorkerNumber -1 is the main backend, which is where we'll set up
        // our shared memory information.  The mutex was already initialized, directly, in
        // `aminitparallelscan()`
        state.init_without_mutex(searcher.segment_readers(), &[]);
    }
    Some(worker_number)
}

pub unsafe fn maybe_claim_segment(mut scan: pg_sys::IndexScanDesc) -> Option<SegmentId> {
    get_bm25_scan_state(&mut scan)?.checkout_segment()
}

pub unsafe fn list_segment_ids(mut scan: pg_sys::IndexScanDesc) -> Option<HashSet<SegmentId>> {
    Some(
        get_bm25_scan_state(&mut scan)?
            .segments()
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
