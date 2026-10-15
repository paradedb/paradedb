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

<<<<<<< HEAD
use pgrx::pg_sys::{shm_toc, Size};
=======
use pgrx::pg_sys::{self, Size, dsa_area, shm_toc};
>>>>>>> 444ea42 (fix: parallel bitmap intersection crash when its Gather is rescanned (#6751))

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
            leader_publish_query(state, &mut *pscan_state);

            // The leader owns the bitmap build: build in this scan's own DSA area,
<<<<<<< HEAD
            // prepare per-(consumer, segment) iterator states, and publish the claim
            // table before any worker launches. Workers only wait and attach.
=======
            // mint one iteration state per participant slot, and publish the claim
            // table before any worker launches. Workers only wait and attach. The
            // context's worker count is the most that can attach, not how many launch.
            let planned_workers = (*pcxt).nworkers as u32;
            (*pscan_state).bitmap_slots = planned_workers + 1;
>>>>>>> 444ea42 (fix: parallel bitmap intersection crash when its Gather is rescanned (#6751))
            leader_publish_bitmap(state, pscan_state);
        }
    }

    fn reinitialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        coordinate: *mut c_void,
    ) {
        let pscan_state = coordinate.cast::<ParallelScanState>();
        assert!(!pscan_state.is_null(), "coordinate is null");
<<<<<<< HEAD
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
=======
        let pscan_state = unsafe { &mut *pscan_state };
        // Only the counters are reset, never the segment payload: the DSM was sized once at
        // estimate time, so it cannot hold a larger set. Every participant therefore keeps
        // replaying the first published view for the life of the query, which also keeps the
        // leader's pins on those segments held that long.
        pscan_state.bitmap_reset();
        pscan_state.reset();
        // The leader's own ReScan runs only after the relaunched workers launch (see above),
        // too late to change what they read, so this callback solves the query for the
        // round's parameters, publishes it, and rebuilds the bitmap. The leader's results
        // from the last round point into the solved query and the bitmap, so they drop
        // first; its reader stays, holding the segment pins, until that ReScan replaces it.
        state.custom_state_mut().reset_exec_results();
        BaseScan::prepare_query(state);
        unsafe { leader_publish_query(state, pscan_state) };
        unsafe { leader_publish_bitmap(state, pscan_state) };
>>>>>>> 444ea42 (fix: parallel bitmap intersection crash when its Gather is rescanned (#6751))
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
<<<<<<< HEAD
                .attach_parallel(pscan_state, ParallelRole::Worker);
            match (*pscan_state)
                .query()
                .expect("should be able to deserialize the query from the ParallelScanState")
            {
                Some(query) => state.custom_state_mut().set_base_search_query_input(query),
                None => panic!("no query in ParallelScanState"),
            }
        }
=======
                .attach_parallel(pscan_state, ParallelRole::Worker)
        };
        let query = unsafe { (*pscan_state).query(query_area(state)) }
            .expect("should be able to read the query from the ParallelScanState");
        state.custom_state_mut().set_base_search_query_input(query);
>>>>>>> 444ea42 (fix: parallel bitmap intersection crash when its Gather is rescanned (#6751))
    }
}

/// The parallel query's DSA area. Postgres installs it on the EState around the DSM
/// callbacks in the leader and for the whole run in a worker (`execParallel.c`).
fn query_area(state: &CustomScanStateWrapper<BaseScan>) -> *mut dsa_area {
    let area = unsafe { (*state.csstate.ss.ps.state).es_query_dsa };
    assert!(
        !area.is_null(),
        "es_query_dsa is only set inside the parallel DSM callbacks"
    );
    area
}

/// Leader-only: hand the coming round's workers the solved query.
unsafe fn leader_publish_query(
    state: &mut CustomScanStateWrapper<BaseScan>,
    pscan_state: &mut ParallelScanState,
) {
    let query = serde_json::to_vec(state.custom_state().search_query_input())
        .expect("should be able to serialize query");
    unsafe { pscan_state.publish_query(query_area(state), &query) };
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
<<<<<<< HEAD
            .shared_source(consumers, &segments);
        (*pscan_state).publish_bitmap_handle(handle);
        if let Some(cell) = state.custom_state().bitmap_cell.clone() {
            if let Some(source) = state.custom_state().bitmap_exec.as_ref().unwrap().source() {
                cell.fill(source);
            }
        }
=======
            .shared_source(consumers, &segments, pscan_state.bitmap_slots)
    };
    pscan_state.publish_bitmap_handle(handle);
    if let Some(cell) = state.custom_state().bitmap_cell.clone()
        && let Some(source) = state.custom_state().bitmap_exec.as_ref().unwrap().source()
    {
        cell.fill(source);
>>>>>>> 444ea42 (fix: parallel bitmap intersection crash when its Gather is rescanned (#6751))
    }
}
