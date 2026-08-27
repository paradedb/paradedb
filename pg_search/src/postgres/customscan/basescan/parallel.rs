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

//! Parallel-aware BaseScan: DSM attach hooks and the process-local handle to
//! shared work assignment / telemetry IPC.

use std::os::raw::c_void;
use std::ptr::NonNull;

use crate::postgres::customscan::basescan::telemetry::ScanTelemetry;
use crate::postgres::customscan::basescan::BaseScan;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::postgres::ParallelScanState;

use pgrx::pg_sys::{shm_toc, Size};

/// Role of this backend in a parallel-aware scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelRole {
    Leader,
    Worker,
}

/// Handle to parallel-aware DSM: work assignment (segment checkout, thresholds,
/// agg rendezvous) plus telemetry publish/collect for EXPLAIN.
#[derive(Debug, Clone, Copy)]
pub struct ParallelScanHandle {
    dsm: NonNull<ParallelScanState>,
    role: ParallelRole,
}

impl ParallelScanHandle {
    /// # Safety
    /// `dsm` must point at a live [`ParallelScanState`] in DSM for the duration
    /// of this handle's use.
    pub unsafe fn new(dsm: *mut ParallelScanState, role: ParallelRole) -> Self {
        Self {
            dsm: NonNull::new(dsm).expect("ParallelScanState pointer must be non-null"),
            role,
        }
    }

    pub fn role(&self) -> ParallelRole {
        self.role
    }

    pub fn is_leader(&self) -> bool {
        self.role == ParallelRole::Leader
    }

    pub fn dsm_ptr(&self) -> *mut ParallelScanState {
        self.dsm.as_ptr()
    }

    /// Worker End: flush local telemetry into DSM.
    pub fn publish_telemetry(&self, local: &ScanTelemetry) {
        debug_assert_eq!(self.role, ParallelRole::Worker);
        unsafe {
            let dsm = self.dsm.as_ptr();
            (*dsm).set_query_count(local.query_count());
            (*dsm).publish_segment_info(local.segment_info());
        }
    }

    /// Leader Shutdown: write this process's query count into DSM, snapshot
    /// explain metadata + worker segment_info, merge segment_info into local.
    pub fn finalize_explain(&self, local: &mut ScanTelemetry) {
        debug_assert_eq!(self.role, ParallelRole::Leader);
        unsafe {
            let dsm = self.dsm.as_ptr();
            (*dsm).set_query_count(local.query_count());
            let explain_data = (*dsm).explain_data();
            local.accumulate_segment_info((*dsm).take_segment_info());
            local.set_parallel_explain(explain_data);
        }
    }

    /// Leader-only: reset the shared work queue for a rescan.
    pub fn reset_work_queue(&self) {
        debug_assert_eq!(self.role, ParallelRole::Leader);
        unsafe {
            let dsm = self.dsm.as_ptr();
            let _mutex = (*dsm).acquire_mutex();
            ParallelScanState::reset(&mut *dsm);
        }
    }
}

impl ParallelQueryCapable for BaseScan {
    fn estimate_dsm_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> Size {
        if state.custom_state().search_reader.is_none() {
            BaseScan::init_search_reader(state);
        }

        let args = state.custom_state().parallel_scan_args();
        ParallelScanState::size_of(
            &args.all_nsegments(),
            &args.query,
            args.with_aggregates,
            args.with_segment_info,
        )
    }

    fn initialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        coordinate: *mut c_void,
    ) {
        let args = state.custom_state().parallel_scan_args();

        unsafe {
            let pscan_state = coordinate.cast::<ParallelScanState>();
            assert!(!pscan_state.is_null(), "coordinate is null");
            (*pscan_state).create_and_populate(args);
            state
                .custom_state_mut()
                .attach_parallel(pscan_state, ParallelRole::Leader);

            // The leader owns the bitmap build: build in this scan's own DSA area,
            // prepare per-(consumer, segment) iterator states, and publish the claim
            // table before any worker launches. Workers only wait and attach.
            leader_publish_bitmap(state, pscan_state);
        }
    }

    fn reinitialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        coordinate: *mut c_void,
    ) {
        let pscan_state = coordinate.cast::<ParallelScanState>();
        assert!(!pscan_state.is_null(), "coordinate is null");
        unsafe {
            (*pscan_state).bitmap_reset();
            (*pscan_state).reset();
            // Republish for the relaunched workers. The scan's rescan callback ran
            // first (freeing the previous table and build when params changed), so
            // this rebuilds with the new params or republishes the still-valid one.
            if let Some(bitmap_exec) = state.custom_state_mut().bitmap_exec.as_mut() {
                let handle = bitmap_exec.republish();
                (*pscan_state).publish_bitmap_handle(handle);
                if handle.is_some() {
                    if let Some(cell) = state.custom_state().bitmap_cell.clone() {
                        if let Some(source) =
                            state.custom_state().bitmap_exec.as_ref().unwrap().source()
                        {
                            cell.fill(source);
                        }
                    }
                }
            }
        }
    }

    fn initialize_worker_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        _toc: *mut shm_toc,
        coordinate: *mut c_void,
    ) {
        let pscan_state = coordinate.cast::<ParallelScanState>();
        assert!(!pscan_state.is_null(), "coordinate is null");

        unsafe {
            state
                .custom_state_mut()
                .attach_parallel(pscan_state, ParallelRole::Worker);
            match (*pscan_state)
                .query()
                .expect("should be able to deserialize the query from the ParallelScanState")
            {
                Some(query) => state.custom_state_mut().set_base_search_query_input(query),
                None => panic!("no query in ParallelScanState"),
            }
        }
    }
}

/// Leader-only: ensure the reader (and with it the executed query and cell) exists,
/// build the shared bitmap over its segments, publish, and swap the leader's cell
/// to the shared view.
unsafe fn leader_publish_bitmap(
    state: &mut CustomScanStateWrapper<BaseScan>,
    pscan_state: *mut ParallelScanState,
) {
    unsafe {
        if state.custom_state().bitmap_exec.is_none() {
            return;
        }
        if state.custom_state().search_reader.is_none() {
            BaseScan::init_search_reader(state);
        }
        let consumers = state
            .custom_state()
            .search_query_input()
            .bitmap_consumer_count();
        if consumers == 0 {
            (*pscan_state).publish_bitmap_handle(None);
            return;
        }
        let segments: Vec<tantivy::index::SegmentId> = state
            .custom_state()
            .search_reader
            .as_ref()
            .expect("search reader should be open")
            .searcher()
            .segment_readers()
            .iter()
            .map(|r| r.segment_id())
            .collect();
        let handle = state
            .custom_state_mut()
            .bitmap_exec
            .as_mut()
            .unwrap()
            .shared_source(consumers, &segments);
        (*pscan_state).publish_bitmap_handle(handle);
        if let Some(cell) = state.custom_state().bitmap_cell.clone() {
            if let Some(source) = state.custom_state().bitmap_exec.as_ref().unwrap().source() {
                cell.fill(source);
            }
        }
    }
}
