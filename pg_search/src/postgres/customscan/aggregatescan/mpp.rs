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

//! AggregateScan MPP worker exec path.
//!
//! Thin wrapper over [`mpp::exec_worker::run_mpp_worker`]: pulls AggregateScan-specific inputs
//! (parallel_state, source_manifests, partitioning source index, the worker's plan bytes and
//! mesh) out of the typed state, builds the aggregate-profile seed `SessionContext`, hands the
//! whole thing to the shape-agnostic dispatcher.

use std::sync::Arc;

use pgrx::pg_sys;

use crate::postgres::customscan::aggregatescan::datafusion_exec::create_aggregate_session_context;
use crate::postgres::customscan::aggregatescan::scan_state;
use crate::postgres::customscan::aggregatescan::AggregateScan;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::mpp::exec_worker::{run_mpp_worker, MppWorkerInputs};

/// MPP worker exec: extract inputs from AggregateScan's state, build the aggregate-profile seed
/// context, hand off to the shape-agnostic dispatcher. Workers emit zero rows back to PG;
/// returning `null_mut()` signals end-of-stream.
pub(super) fn exec_mpp_worker(
    state: &mut CustomScanStateWrapper<AggregateScan>,
) -> *mut pg_sys::TupleTableSlot {
    // Pull worker-thread inputs from the outer state before we borrow df_state mutably.
    // parallel_state and non_partitioning_segments are required to pin each worker's
    // PgSearchTableProvider to the right segment slice (the partitioning source) and the
    // leader's canonical replica (the non-partitioning sources). Without them, every worker
    // re-scans the full data and the leader-side hash partitions get the same rows from
    // every worker.
    let parallel_state = state.custom_state().parallel_state;
    let non_partitioning_segments = state.custom_state().non_partitioning_segments.clone();
    let partitioning_source_idx = state
        .custom_state()
        .mpp_partitioning_source_idx
        .unwrap_or(0);
    let plan_sources_count = state
        .custom_state()
        .source_manifests
        .len()
        .max(non_partitioning_segments.len() + 1);

    let df_state = state
        .custom_state_mut()
        .datafusion_state
        .as_mut()
        .expect("DataFusion state must be initialized");

    if df_state.runtime.is_some() {
        // Already drained on a prior call; just signal EOF.
        return std::ptr::null_mut();
    }
    let scan_state::MppExecState::Worker(worker) = df_state.mpp.as_ref().expect("checked") else {
        unreachable!("exec_mpp_worker called outside Worker state");
    };
    let plan_bytes = worker.plan_bytes.clone();
    let worker_mesh = Arc::clone(&worker.mesh);

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => pgrx::error!("mpp worker: tokio runtime build failed: {e}"),
    };
    df_state.runtime = Some(runtime);
    let runtime = df_state.runtime.as_ref().unwrap();

    // Use a bare aggregate-profile context for plan deserialization. The distributed planner
    // config (worker resolver, transport, estimators, codec) is layered on top inside
    // `run_mpp_worker` via `build_mpp_session_context`. Both procs have to agree on stage shape;
    // this is how.
    let seed_ctx = create_aggregate_session_context();

    run_mpp_worker(
        MppWorkerInputs {
            parallel_state,
            non_partitioning_segments,
            partitioning_source_idx,
            plan_sources_count,
            plan_bytes,
            worker_mesh,
        },
        seed_ctx,
        runtime,
    );
    std::ptr::null_mut()
}
