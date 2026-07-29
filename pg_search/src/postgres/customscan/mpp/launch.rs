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
//! `ParallelScanState`, plus a go flag. Workers reconstruct their `MppWorkerInputs` from the same
//! entries, with no PG plan node in reach.
//!
//! The launch is plan-first (#5667), mirroring core PG's parallel-query order (plan →
//! size-by-walking-the-plan → create DSM → bind pointers → launch): the caller builds the
//! distributed physical plan with `producer_worker_cap()` (PG's parallelism GUCs) as the
//! planner's ceiling, then [`launch_mpp_join`] / [`launch_mpp_aggregate`] walk the finished plan to learn
//! the largest producer-stage task count, size the mesh and spawn exactly
//! `clamp(max_tasks, 2, cap)` producers, stamp the shared scan state into the plan
//! ([`crate::scan::execution_plan::stamp_parallel_state`]), and dispatch. A plan with no
//! producer stages spawns nothing at all. One plan serves both dispatch and the leader's own
//! execution; the go flag holds spawned workers off the mesh until the rings and dispatch
//! payload are initialized, and aborts them on a short launch (#5061).

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
use crate::postgres::customscan::mpp::dispatch::{
    dispatch_payload_from_plan, dispatch_plan_capacity,
};
use crate::postgres::customscan::mpp::exec_worker::{run_mpp_worker, MppWorkerInputs};
use crate::postgres::customscan::mpp::glue::{
    estimate_dsm_size, leader_setup, producer_worker_cap, worker_setup, MppLeaderState,
    MIN_PRODUCER_COUNT,
};
use crate::postgres::customscan::mpp::worker_fragments::max_producer_task_count;
use crate::postgres::{ParallelScanArgs, ParallelScanState};

/// `state_values()` order. Each index maps to a `ParallelState` TOC entry the workers look up.
const MESH_IDX: usize = 0;
const SCAN_IDX: usize = 1;
const GO_IDX: usize = 2;

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
}

impl ParallelProcess for MppParallelProcess {
    fn state_values(&self) -> Vec<&dyn ParallelState> {
        vec![&self.mesh_region, &self.scan_state, &self.go]
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
    // Under the plan-first launch the leader has already planned; this wait only spans the
    // leader's ring init (`leader_setup`) and short-launch decision. Still back off to
    // sleeping after a burst of yields rather than spin.
    let mut spins = 0u32;
    loop {
        check_for_interrupts!();
        match go.load(Ordering::Acquire) {
            GO_RUN => break,
            // Short launch: the leader ran the query serially. Exit before touching the mesh.
            GO_ABORT => return,
            _ if spins < 1000 => {
                spins += 1;
                std::thread::yield_now();
            }
            _ => unsafe { pg_sys::pg_usleep(100) },
        }
    }

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
    // segment sets from it.
    let scan_ptr = match state_manager.slice_mut::<u8>(SCAN_IDX) {
        Ok(Some(s)) => s.as_mut_ptr() as *mut ParallelScanState,
        _ => pgrx::error!("mpp worker: parallel scan state missing from parallel state"),
    };
    let plan_sources_count = unsafe { (*scan_ptr).source_count() };

    let inputs = MppWorkerInputs {
        parallel_state: Some(scan_ptr),
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

/// Where MPP sits in a scan's launch lifecycle. Every transition consumes the previous stage,
/// so a scan is in exactly one stage at a time; a single field keeps the impossible
/// combinations unrepresentable. Held only by the leader; builder-launched workers reconstruct
/// their state from DSM and never carry this.
#[derive(Default)]
pub enum MppLifecycle {
    /// Serial execution: the query didn't qualify, a fallback abandoned the launch, or
    /// teardown already reclaimed the leader state.
    #[default]
    Inactive,
    /// Serialized logical-plan bytes, stashed at begin time. The launch uses their length to
    /// size the DSM payload region.
    PlanBytes(Vec<u8>),
    /// The workers are running dispatched fragments; carries the leader's mesh and finish
    /// handles until teardown.
    Launched(MppLeaderState),
}

impl MppLifecycle {
    /// Consume the stashed plan bytes. Leaves `Inactive`, so a launch fallback reads as the
    /// serial path from then on.
    pub fn take_plan_bytes(&mut self) -> Option<Vec<u8>> {
        match std::mem::take(self) {
            MppLifecycle::PlanBytes(bytes) => Some(bytes),
            other => {
                *self = other;
                None
            }
        }
    }

    /// Consume the leader state at teardown, leaving `Inactive`.
    pub fn take_leader(&mut self) -> Option<MppLeaderState> {
        match std::mem::take(self) {
            MppLifecycle::Launched(leader) => Some(leader),
            other => {
                *self = other;
                None
            }
        }
    }

    pub fn leader(&self) -> Option<&MppLeaderState> {
        match self {
            MppLifecycle::Launched(leader) => Some(leader),
            _ => None,
        }
    }

    pub fn leader_mut(&mut self) -> Option<&mut MppLeaderState> {
        match self {
            MppLifecycle::Launched(leader) => Some(leader),
            _ => None,
        }
    }

    pub fn is_launched(&self) -> bool {
        matches!(self, MppLifecycle::Launched(_))
    }
}

/// Plan-first MPP launch (#5667): size the worker pool from the finished physical plan, build
/// the DSM, stamp the shared scan state into the plan, spawn exactly the needed producers, and
/// dispatch. `None` means run serially — the plan has nothing to distribute, the DSM couldn't
/// be built, or the machine couldn't give us the full producer set. Nothing is forked on the
/// nothing-to-distribute path. A `pgrx::error!` is reserved for setup that already committed a
/// launched worker to the mesh, where a silent serial fallback would hide a real bug.
fn launch_mpp(
    physical: &std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>,
    plan_bytes_len: usize,
    args: ParallelScanArgs,
    worker_entrypoint: &'static str,
) -> Option<MppLeaderState> {
    let mut timing = crate::postgres::customscan::mpp::glue::MppLaunchTiming::default();

    // Task counts were capped by `target_partitions = cap` at plan time and reflect segment
    // counts (#5657). The `MIN_PRODUCER_COUNT` floor keeps the mesh-width invariant:
    // `build_mpp_session_context` clamps `target_partitions` to 2, so the mesh needs a queue
    // for the second partition.
    let max_tasks = max_producer_task_count(physical);
    if max_tasks == 0 {
        return None;
    }
    let cap = producer_worker_cap();
    if cap < MIN_PRODUCER_COUNT {
        // Unreachable when callers gate on `mpp_is_active()`, but `clamp` panics when
        // min > max; a forgotten gate should mean a serial run, not a panic.
        return None;
    }
    let producer_count = (max_tasks as u32).clamp(MIN_PRODUCER_COUNT, cap);

    let t_prepare = std::time::Instant::now();
    let payload_capacity = dispatch_plan_capacity(plan_bytes_len);
    let region_bytes = match estimate_dsm_size(producer_count + 1, payload_capacity) {
        Ok(sz) => sz,
        Err(e) => {
            pgrx::warning!("mpp: estimate_dsm failed: {e}; running serially");
            return None;
        }
    };
    let scan_size = ParallelScanState::size_of(&args.all_nsegments(), &[], false);

    let process = MppParallelProcess {
        // SAFETY: workers can only read the region back as `u8`, and they hold on the go
        // flag until `leader_setup` has written every ring header, so nothing reads the
        // reserved bytes before their first writer.
        mesh_region: unsafe {
            crate::parallel_worker::UninitializedBytesParallelState::new(region_bytes)
        },
        scan_state: vec![0u8; scan_size],
        go: GO_WAIT,
    };

    let launcher = ParallelProcessBuilder::build(
        process,
        worker_entrypoint,
        WorkerStyle::Query,
        producer_count as usize,
        MPP_MQ_SIZE,
    )?;

    // Populate the ParallelScanState in place while the DSM is mapped and the leader still holds
    // the source manifests `args` borrows.
    let scan_ptr = match launcher.state_manager().slice_mut::<u8>(SCAN_IDX) {
        Ok(Some(s)) => s.as_mut_ptr() as *mut ParallelScanState,
        _ => pgrx::error!("mpp: parallel scan state region missing"),
    };
    unsafe { (*scan_ptr).create_and_populate(args) };

    timing.prepare_us = t_prepare.elapsed().as_micros() as u64;

    // Spawn the workers; the go flag keeps them off the mesh until the rings and the dispatch
    // payload are initialized below.
    let attach = launcher.launch()?;

    // Derive the per-stage subplans from the plan the leader itself will execute. A failure
    // here is a hard error: a serialization gap is a codec bug, and the parked workers die
    // with the transaction.
    let t_payload = std::time::Instant::now();
    let (payload, stage_count) =
        match dispatch_payload_from_plan(physical, producer_count, payload_capacity) {
            Ok(p) => p,
            Err(e) => pgrx::error!("mpp: dispatch payload build failed: {e}"),
        };
    timing.payload_us = t_payload.elapsed().as_micros() as u64;

    let t_attach = std::time::Instant::now();
    let finish = attach.wait_for_attach()?;
    timing.attach_us = t_attach.elapsed().as_micros() as u64;
    let launched = finish.launched_workers() as u32;
    timing.workers = launched;

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

    // `max_producer_task_count > 0` implies dispatch finds the same stages (both walks gate on
    // `local_plan()`), so this should be unreachable — but if the two walks ever drift, a
    // zero-stage dispatch would leave workers waiting on `SetPlan` frames that never come.
    // Abort them and run serially rather than hang.
    if stage_count == 0 {
        debug_assert!(
            false,
            "mpp: max_producer_task_count > 0 but dispatch found no stages"
        );
        go.store(GO_ABORT, Ordering::Release);
        finish.wait_for_finish();
        return None;
    }

    // Bind the shared scan state into the plan the leader will execute. Must stay below the
    // last serial-fallback `return None`: a stamped plan must never outlive its DSM, and the
    // fallback paths replan. Workers are unaffected — dispatch encodes are context-free and
    // each worker injects its own pointer at decode.
    crate::scan::execution_plan::stamp_parallel_state(physical, scan_ptr);

    // Initialize the leader's rings now that we're committed to the parallel path. After launch on
    // purpose: the serial fallbacks above never create the DSM-backed control senders.
    let mesh_ptr = match finish.state_manager().slice_mut::<u8>(MESH_IDX) {
        Ok(Some(s)) => s.as_mut_ptr() as *mut c_void,
        _ => pgrx::error!("mpp: mesh region missing"),
    };
    let t_setup = std::time::Instant::now();
    let mut leader = match unsafe { leader_setup(mesh_ptr, producer_count + 1, payload) } {
        Ok(l) => l,
        Err(e) => pgrx::error!("mpp: leader_setup failed: {e}"),
    };
    timing.leader_setup_us = t_setup.elapsed().as_micros() as u64;
    leader.timing = timing;

    // Registered here (not in `leader_setup`) because this is the first point with both the segment
    // (`finish`) and the senders (`leader`) in hand.
    unsafe { leader.register_control_senders_on_detach(finish.dsm_segment()) };

    // Release the workers into ring attach + plan wait.
    go.store(GO_RUN, Ordering::Release);

    leader.finish = Some(finish);
    Some(leader)
}

/// AggregateScan launch entry: aggregate worker symbol.
pub fn launch_mpp_aggregate(
    physical: &std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>,
    plan_bytes_len: usize,
    args: ParallelScanArgs,
) -> Option<MppLeaderState> {
    launch_mpp(physical, plan_bytes_len, args, "mpp_launched_worker_agg")
}

/// JoinScan launch entry: join worker symbol.
pub fn launch_mpp_join(
    physical: &std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>,
    plan_bytes_len: usize,
    args: ParallelScanArgs,
) -> Option<MppLeaderState> {
    launch_mpp(physical, plan_bytes_len, args, "mpp_launched_worker_join")
}
