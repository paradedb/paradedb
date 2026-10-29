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

use crate::index::reader::io_stats::Trace;
use crate::index::stats::SegmentStats;
use crate::postgres::ParallelScanState;
use crate::postgres::customscan::basescan::BaseScan;
use crate::postgres::customscan::basescan::privdat::PrivateData;
use crate::postgres::customscan::basescan::telemetry::ScanTelemetry;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::vector::PgVector;

use pgrx::pg_sys::{self, Size, dsa_area, shm_toc};

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

    /// Worker Shutdown: flush local segment_info into DSM.
    pub fn publish_telemetry(&self, local: &ScanTelemetry, trace: Option<&Trace>) {
        debug_assert_eq!(self.role, ParallelRole::Worker);
        let dsm = unsafe { &mut *self.dsm.as_ptr() };
        dsm.publish_segment_info(local.segment_info());
        if let Some(trace) = trace {
            let hits = trace.shared_hits();
            let _lock = dsm.acquire_mutex();
            dsm.worker_buffer_hits.merge(hits);
        }
    }

    /// Leader Shutdown: snapshot explain metadata + worker segment_info, merge
    /// segment_info into local.
    pub fn finalize_explain(&self, local: &mut ScanTelemetry) {
        debug_assert_eq!(self.role, ParallelRole::Leader);
        let dsm = unsafe { &mut *self.dsm.as_ptr() };
        let explain_data = dsm.explain_data();
        local.accumulate_segment_info(dsm.take_segment_info());
        local.set_parallel_explain(explain_data);
    }

    pub(crate) fn finalize_buffer_hits(&self, trace: &Trace) {
        let dsm = unsafe { &mut *self.dsm.as_ptr() };
        trace.merge_hits(std::mem::take(&mut dsm.worker_buffer_hits));
    }
}

// LIMIT can shut down the leader scan before its workers; collect after Gather joins them.
#[pgrx::pg_guard]
unsafe extern "C-unwind" fn collect_buffer_hits_on_detach(
    _seg: *mut pg_sys::dsm_segment,
    arg: pg_sys::Datum,
) {
    let (parallel, trace) =
        *unsafe { Box::from_raw(arg.cast_mut_ptr::<(ParallelScanHandle, Trace)>()) };
    if unsafe { pg_sys::IsTransactionState() } {
        parallel.finalize_buffer_hits(&trace);
    }
}

unsafe fn leader_order_vector_segments(
    state: &mut CustomScanStateWrapper<BaseScan>,
    parallel: &mut ParallelScanState,
) {
    parallel.set_vector_order(None);
    let plan = state.csstate.ss.ps.plan.cast::<pg_sys::CustomScan>();
    let private = PrivateData::from((*plan).custom_private);
    let Some(mut order) = private
        .maybe_orderby_info()
        .as_ref()
        .and_then(|order| order.first())
        .cloned()
    else {
        return;
    };
    if !order.is_vector_distance() || !order.direction.is_asc() {
        return;
    }
    order.resolve_query_vector(state.csstate.ss.ps.state, state.planstate());
    let (name, query, metric) = order.as_vector_distance().unwrap();
    let query = PgVector(query.to_vec());
    let reader = state.custom_state().search_reader.as_ref().unwrap();
    let field = reader
        .schema()
        .tantivy_schema()
        .get_field(&name.root())
        .expect("vector field exists");
    let mut known = 0;
    let mut ranked = reader
        .searcher()
        .segment_readers()
        .iter()
        .enumerate()
        .map(|(ordinal, segment)| {
            let centroid = SegmentStats::of_reader(segment)
                .expect("segment stats should be readable")
                .and_then(|stats| {
                    stats
                        .vector_partition(field)
                        .expect("partition centroid should be readable")
                });
            let distance = centroid
                .filter(|c| c.len() == query.0.len() && c.iter().all(|v| v.is_finite()))
                .map(|c| {
                    known += 1;
                    query.partition_distance(metric, &c)
                })
                .unwrap_or(f32::NEG_INFINITY);
            (ordinal as u32, distance)
        })
        .collect::<Vec<_>>();
    if known == 0 {
        return;
    }
    ranked.sort_unstable_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    parallel.set_vector_order(Some(
        &ranked
            .into_iter()
            .map(|(ordinal, _)| ordinal)
            .collect::<Vec<_>>(),
    ));
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
        pcxt: *mut pg_sys::ParallelContext,
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
            leader_order_vector_segments(state, &mut *pscan_state);

            if !(*pcxt).seg.is_null()
                && let Some(trace) = state.custom_state().io_trace.clone()
            {
                let callback = Box::new((
                    ParallelScanHandle::new(pscan_state, ParallelRole::Leader),
                    trace,
                ));
                pg_sys::on_dsm_detach(
                    (*pcxt).seg,
                    Some(collect_buffer_hits_on_detach),
                    pg_sys::Datum::from(Box::into_raw(callback)),
                );
            }

            // The leader owns the bitmap build: build in this scan's own DSA area,
            // mint one iteration state per participant slot, and publish the claim
            // table before any worker launches. Workers only wait and attach. The
            // context's worker count is the most that can attach, not how many launch.
            let planned_workers = (*pcxt).nworkers as u32;
            (*pscan_state).bitmap_slots = planned_workers + 1;
            leader_publish_bitmap(state, pscan_state);
        }
    }

    fn reinitialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        coordinate: *mut c_void,
    ) {
        // The one place the shared segment work queue is reset for a rescan.
        // PostgreSQL guarantees this runs while no workers are attached (see
        // nodeGather.c: "ReInitializeDSM should reset only shared state, ReScan
        // should reset only local state"). Resetting the queue anywhere else —
        // in particular from the leader's ReScan callback, which runs after
        // LaunchParallelWorkers — races with workers' segment claims and makes
        // already-claimed segments get scanned twice (#5024).
        let pscan_state = coordinate.cast::<ParallelScanState>();
        assert!(!pscan_state.is_null(), "coordinate is null");
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
        unsafe { leader_order_vector_segments(state, pscan_state) };
        unsafe { leader_publish_bitmap(state, pscan_state) };
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
                .attach_parallel(pscan_state, ParallelRole::Worker)
        };
        let query = unsafe { (*pscan_state).query(query_area(state)) }
            .expect("should be able to read the query from the ParallelScanState");
        state.custom_state_mut().set_base_search_query_input(query);
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
    if state.custom_state().bitmap_exec.is_none() {
        return;
    }
    if state.custom_state().search_reader.is_none() {
        BaseScan::init_search_reader(state);
    }
    let pscan_state = unsafe { &mut *pscan_state };
    let consumers = state
        .custom_state()
        .search_query_input()
        .bitmap_consumer_count();
    if consumers == 0 {
        pscan_state.publish_bitmap_handle(None);
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
    let handle = unsafe {
        state
            .custom_state_mut()
            .bitmap_exec
            .as_mut()
            .unwrap()
            .shared_source(consumers, &segments, pscan_state.bitmap_slots)
    };
    pscan_state.publish_bitmap_handle(handle);
    if let Some(cell) = state.custom_state().bitmap_cell.clone()
        && let Some(source) = state.custom_state().bitmap_exec.as_ref().unwrap().source()
    {
        cell.fill(source);
    }
}
