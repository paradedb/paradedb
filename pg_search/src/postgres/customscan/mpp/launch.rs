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

//! Leader-driven MPP worker launch.
//!
//! The leader spawns its producer workers itself through `parallel_worker::builder`
//! (`CreateParallelContext` + `LaunchParallelWorkers`), the same path index builds and the
//! parallel aggregate use. The leader picks the worker count, so a short launch becomes a clean
//! serial fallback instead of the hang it used to be when PG's Gather decided the count.
//!
//! The MPP DSM region rides as builder `ParallelState` entries instead of a hand-laid coordinate:
//! a reserve-only region for the ring mesh (`shm::dsm_region_bytes`), a zeroed byte blob for the
//! `ParallelScanState`, plus the partitioning-source index and a go flag. The leader initializes
//! the mesh and populates the scan state in place between `build()` and worker attach; workers
//! reconstruct their `MppWorkerInputs` from the same entries, with no PG plan node in reach.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use datafusion::prelude::SessionContext;
use pgrx::{check_for_interrupts, pg_sys};

use datafusion_distributed::shm::region_total;

use crate::parallel_worker::builder::ParallelProcessBuilder;
use crate::parallel_worker::{
    generic_parallel_worker_entry_point, ParallelProcess, ParallelState, ParallelStateManager,
    WorkerStyle,
};
use crate::postgres::customscan::aggregatescan::datafusion_exec::create_aggregate_session_context;
use crate::postgres::customscan::joinscan::scan_state::{
    create_datafusion_session_context, SessionContextProfile,
};
use crate::postgres::customscan::mpp::dispatch::{build_dispatch_payload, dispatch_plan_capacity};
use crate::postgres::customscan::mpp::exec_worker::{run_mpp_worker, MppWorkerInputs};
use crate::postgres::customscan::mpp::glue::{
    estimate_dsm_size, leader_setup, producer_worker_count, worker_setup, MppLeaderState,
};
use crate::postgres::{ParallelScanArgs, ParallelScanState};

/// `state_values()` order. Each index maps to a `ParallelState` TOC entry the workers look up.
const MESH_IDX: usize = 0;
const SCAN_IDX: usize = 1;
const GO_IDX: usize = 2;
const PART_IDX: usize = 3;

/// Go-flag states. The leader sets `RUN` once the full producer set launched and the rings are
/// initialized, or `ABORT` on a short launch so the spare workers exit before touching the mesh.
const GO_WAIT: u32 = 0;
const GO_RUN: u32 = 1;
const GO_ABORT: u32 = 2;

/// Per-worker completion queue size. MPP carries data over the ring mesh, not this queue; it only
/// serves as the leader's detach barrier in `wait_for_finish`. Matches the index-build size so
/// `shm_mq_create` has room for its header.
const MPP_MQ_SIZE: usize = 1024;

/// The builder process carrying the MPP DSM entries. The mesh region is reserve-only: at the
/// default queue size it is hundreds of megabytes, and `shm::leader_setup` writes every header
/// and ring slot it reads, so materializing (zeroing, copying) a host-side buffer of that size
/// per query would buy nothing. The scan state is a small zeroed blob populated in place.
struct MppParallelProcess {
    mesh_region: crate::parallel_worker::UninitializedBytesParallelState,
    scan_state: Vec<u8>,
    go: u32,
    partitioning_source_idx: u64,
}

impl ParallelProcess for MppParallelProcess {
    fn state_values(&self) -> Vec<&dyn ParallelState> {
        vec![
            &self.mesh_region,
            &self.scan_state,
            &self.go,
            &self.partitioning_source_idx,
        ]
    }
}

/// View the go-flag entry as an atomic. Leader and workers only ever touch this slot through this
/// helper, so the shared `u32` is never accessed non-atomically.
unsafe fn go_flag(sm: &ParallelStateManager) -> &'static AtomicU32 {
    let ptr = match sm.object::<u32>(GO_IDX) {
        Ok(Some(r)) => r as *mut u32,
        _ => pgrx::error!("mpp: go flag entry missing from parallel state"),
    };
    &*(ptr as *const AtomicU32)
}

/// AggregateScan worker entry point. PG resolves this symbol by name (passed to
/// `ParallelProcessBuilder::build`), so the name must match the string in [`launch_mpp_aggregate`].
#[no_mangle]
#[pgrx::pg_guard]
pub unsafe extern "C-unwind" fn mpp_launched_worker_agg(
    seg: *mut pg_sys::dsm_segment,
    toc: *mut pg_sys::shm_toc,
) {
    let (state_manager, _mq_sender) = generic_parallel_worker_entry_point(seg, toc, MPP_MQ_SIZE);
    run_launched_worker(state_manager, create_aggregate_session_context);
    // `_mq_sender` drops here, detaching the completion queue so the leader's `wait_for_finish`
    // recv loop terminates.
}

/// JoinScan worker entry point. PG resolves this symbol by name; it must match the string in
/// [`launch_mpp_join`].
#[no_mangle]
#[pgrx::pg_guard]
pub unsafe extern "C-unwind" fn mpp_launched_worker_join(
    seg: *mut pg_sys::dsm_segment,
    toc: *mut pg_sys::shm_toc,
) {
    let (state_manager, _mq_sender) = generic_parallel_worker_entry_point(seg, toc, MPP_MQ_SIZE);
    run_launched_worker(state_manager, || {
        create_datafusion_session_context(SessionContextProfile::Join)
    });
}

/// Shared worker body: wait for the leader's go signal, attach to the ring mesh, reconstruct the
/// `MppWorkerInputs` from the DSM entries, and run the producer fragments. `seed_ctx` is the
/// per-shape serial session context used only for plan deserialization.
fn run_launched_worker(state_manager: ParallelStateManager, seed_ctx: fn() -> SessionContext) {
    let go = unsafe { go_flag(&state_manager) };
    loop {
        check_for_interrupts!();
        match go.load(Ordering::Acquire) {
            GO_RUN => break,
            // Short launch: the leader ran the query serially. Exit before touching the mesh.
            GO_ABORT => return,
            _ => std::thread::yield_now(),
        }
    }

    let partitioning_source_idx = match state_manager.object::<u64>(PART_IDX) {
        Ok(Some(r)) => *r as usize,
        _ => pgrx::error!("mpp worker: partitioning source index missing from parallel state"),
    };

    // Attach to the leader's initialized rings.
    let region_ptr = match state_manager.slice_mut::<u8>(MESH_IDX) {
        Ok(Some(s)) => s.as_mut_ptr() as *mut c_void,
        _ => pgrx::error!("mpp worker: mesh region missing from parallel state"),
    };
    let region_bytes = unsafe { region_total(region_ptr) };
    let worker_number = unsafe { pg_sys::ParallelWorkerNumber };
    let worker = match unsafe { worker_setup(region_ptr, region_bytes, worker_number) } {
        Ok(w) => w,
        Err(e) => pgrx::error!("mpp worker: worker_setup failed: {e}"),
    };

    // The leader populated the ParallelScanState before launch; read the canonical
    // non-partitioning segment sets from it.
    let scan_ptr = match state_manager.slice_mut::<u8>(SCAN_IDX) {
        Ok(Some(s)) => s.as_mut_ptr() as *mut ParallelScanState,
        _ => pgrx::error!("mpp worker: parallel scan state missing from parallel state"),
    };
    let non_partitioning_segments = unsafe { (*scan_ptr).non_partitioning_segment_ids() };
    let plan_sources_count = non_partitioning_segments.len() + 1;

    let inputs = MppWorkerInputs {
        parallel_state: Some(scan_ptr),
        non_partitioning_segments,
        partitioning_source_idx,
        plan_sources_count,
        plan_bytes: worker.plan_bytes,
        worker_mesh: worker.mesh,
        outbound_senders: worker.outbound_senders,
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => pgrx::error!("mpp worker: tokio runtime build failed: {e}"),
    };
    run_mpp_worker(inputs, seed_ctx(), &runtime);
}

/// Launch the producer workers and return the leader's mesh state, or `None` to run serially.
///
/// `None` covers every fallback that must not deploy MPP: DSM too large, or the machine couldn't
/// give us the full producer set. A `pgrx::error!` is reserved for setup that already committed a
/// launched worker to the mesh, where a silent serial fallback would hide a real bug.
fn launch_mpp(
    plan_bytes: Vec<u8>,
    args: ParallelScanArgs,
    partitioning_source_idx: usize,
    seed_for_dispatch: SessionContext,
    worker_entrypoint: &'static str,
) -> Option<MppLeaderState> {
    let producer_count = producer_worker_count();

    let region_bytes = match estimate_dsm_size(dispatch_plan_capacity(plan_bytes.len())) {
        Ok(sz) => sz,
        Err(e) => {
            pgrx::warning!("mpp: estimate_dsm failed: {e}; running serially");
            return None;
        }
    };
    let scan_size =
        ParallelScanState::size_of(&args.all_nsegments(), partitioning_source_idx, &[], false);

    let process = MppParallelProcess {
        mesh_region: crate::parallel_worker::UninitializedBytesParallelState::new(region_bytes),
        scan_state: vec![0u8; scan_size],
        go: GO_WAIT,
        partitioning_source_idx: partitioning_source_idx as u64,
    };

    let launcher = ParallelProcessBuilder::build(
        process,
        worker_entrypoint,
        WorkerStyle::Query,
        producer_count as usize,
        MPP_MQ_SIZE,
    )?;

    // Populate the ParallelScanState in place while the DSM is mapped and the leader still holds
    // the source manifests `args` borrows. Done before launch so workers find it initialized.
    let scan_ptr = match launcher.state_manager().slice_mut::<u8>(SCAN_IDX) {
        Ok(Some(s)) => s.as_mut_ptr() as *mut ParallelScanState,
        _ => pgrx::error!("mpp: parallel scan state region missing"),
    };
    unsafe { (*scan_ptr).create_and_populate(args) };
    let non_partitioning_segments = unsafe { (*scan_ptr).non_partitioning_segment_ids() };

    // Build the per-stage subplans once. A failure here is a hard error, matching the pre-cutover
    // path: a serial fallback on a serialization gap would hide a codec bug behind a slow plan.
    let (payload, stage_plans) = match build_dispatch_payload(
        &plan_bytes,
        seed_for_dispatch,
        producer_count,
        &non_partitioning_segments,
    ) {
        Ok(p) => p,
        Err(e) => pgrx::error!("mpp: dispatch payload build failed: {e}"),
    };

    // Spawn the workers. They block on the go flag before attaching, so a short launch is aborted
    // without any worker reaching the mesh.
    let attach = launcher.launch()?;
    let finish = attach.wait_for_attach()?;
    let launched = finish.launched_workers() as u32;

    let go = unsafe { go_flag(finish.state_manager()) };

    if launched < producer_count {
        // #5061: the machine couldn't give us the full producer set. The launched workers are
        // still on the go flag with no rings attached; release them and run serially. No
        // `leader_setup` ran, so there are no DSM-backed senders to outlive the mapping.
        go.store(GO_ABORT, Ordering::Release);
        finish.wait_for_finish();
        pgrx::warning!(
            "mpp: launched {launched} of {producer_count} requested workers; running serially"
        );
        return None;
    }

    // Initialize the leader's rings now that we're committed to the parallel path. After launch on
    // purpose: the serial fallbacks above never create the DSM-backed control senders.
    let mesh_ptr = match finish.state_manager().slice_mut::<u8>(MESH_IDX) {
        Ok(Some(s)) => s.as_mut_ptr() as *mut c_void,
        _ => pgrx::error!("mpp: mesh region missing"),
    };
    let mut leader = match unsafe { leader_setup(mesh_ptr, payload, stage_plans) } {
        Ok(l) => l,
        Err(e) => pgrx::error!("mpp: leader_setup failed: {e}"),
    };

    // Release the workers into ring attach + plan wait.
    go.store(GO_RUN, Ordering::Release);

    leader.finish = Some(finish);
    leader.parallel_state = scan_ptr;
    Some(leader)
}

/// AggregateScan launch entry: aggregate seed context + aggregate worker symbol.
pub fn launch_mpp_aggregate(
    plan_bytes: Vec<u8>,
    args: ParallelScanArgs,
    partitioning_source_idx: usize,
) -> Option<MppLeaderState> {
    launch_mpp(
        plan_bytes,
        args,
        partitioning_source_idx,
        create_aggregate_session_context(),
        "mpp_launched_worker_agg",
    )
}

/// JoinScan launch entry: join seed context + join worker symbol.
pub fn launch_mpp_join(
    plan_bytes: Vec<u8>,
    args: ParallelScanArgs,
    partitioning_source_idx: usize,
) -> Option<MppLeaderState> {
    launch_mpp(
        plan_bytes,
        args,
        partitioning_source_idx,
        create_datafusion_session_context(SessionContextProfile::Join),
        "mpp_launched_worker_join",
    )
}
