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

use crate::index::reader::index::SearchIndexReader;
use crate::postgres::ParallelScanState;
use pgrx::{pg_guard, pg_sys};
use std::ptr::addr_of_mut;
use tantivy::index::SegmentId;

#[derive(Debug)]
#[repr(transparent)]
pub struct Spinlock(pg_sys::slock_t);

impl Spinlock {
    #[inline(always)]
    pub fn init(&mut self) {
        unsafe {
            // SAFETY:  `unsafe` due to normal FFI
            pg_sys::SpinLockInit(addr_of_mut!(self.0));
        }
    }

    #[inline(always)]
    pub fn acquire(&mut self) -> impl Drop {
        AcquiredSpinLock::new(self)
    }
}

#[repr(transparent)]
struct AcquiredSpinLock(*mut pg_sys::slock_t);

impl AcquiredSpinLock {
    fn new(lock: &mut Spinlock) -> Self {
        unsafe {
            let addr = addr_of_mut!(lock.0);
            pg_sys::SpinLockAcquire(addr);
            Self(addr)
        }
    }
}

impl Drop for AcquiredSpinLock {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            pg_sys::SpinLockRelease(self.0);
        }
    }
}

#[pg_guard]
pub unsafe extern "C" fn aminitparallelscan(target: *mut ::core::ffi::c_void) {
    let state = target.cast::<ParallelScanState>();
    (*state).mutex.init();
}

#[pg_guard]
pub unsafe extern "C" fn amparallelrescan(_scan: pg_sys::IndexScanDesc) {}

#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
#[pg_guard]
pub unsafe extern "C" fn amestimateparallelscan() -> pg_sys::Size {
    ParallelScanState::size_of_with_segments(u16::MAX as usize)
}

#[cfg(feature = "pg17")]
#[pg_guard]
pub unsafe extern "C" fn amestimateparallelscan(_nkeys: i32, _norderbys: i32) -> pg_sys::Size {
    // NB:  in this function, we have no idea how many segments we have.  We don't even know which
    // index we're querying.  So we choose a, hopefully, large enough value at 65536, or u16::MAX
    ParallelScanState::size_of_with_segments(u16::MAX as usize)
}

unsafe fn bm25_shared_state(scan: &pg_sys::IndexScanDescData) -> Option<&mut ParallelScanState> {
    if scan.parallel_scan.is_null() {
        None
    } else {
        scan.parallel_scan
            .cast::<std::ffi::c_void>()
            .add((*scan.parallel_scan).ps_offset)
            .cast::<ParallelScanState>()
            .as_mut()
    }
}

pub unsafe fn maybe_init_parallel_scan(
    scan: pg_sys::IndexScanDesc,
    searcher: &SearchIndexReader,
) -> Option<i32> {
    if unsafe { (*scan).parallel_scan.is_null() } {
        // not a parallel scan, so there's nothing to initialize
        return None;
    }

    let state = get_bm25_scan_state(&scan)?;
    let worker_number = unsafe { pg_sys::ParallelWorkerNumber };
    let _mutex = state.mutex.acquire();
    if worker_number == -1 {
        // ParallelWorkerNumber -1 is the main backend, which is where we'll set up
        // our shared memory information
        state.assign_segment_ids(searcher);
    }
    Some(worker_number)
}

pub unsafe fn maybe_claim_segment(scan: pg_sys::IndexScanDesc) -> Option<SegmentId> {
    let state = get_bm25_scan_state(&scan)?;

    let _mutex = state.mutex.acquire();
    if state.remaining_segments == 0 {
        // no more to claim
        None
    } else {
        // claim the next one
        state.remaining_segments -= 1;
        Some(state.get_segment_id(state.remaining_segments as usize))
    }
}

fn get_bm25_scan_state(scan: &pg_sys::IndexScanDesc) -> Option<&mut ParallelScanState> {
    unsafe {
        assert!(!scan.is_null());
        let scan = scan.as_mut().unwrap_unchecked();
        bm25_shared_state(scan)
    }
}
