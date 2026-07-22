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
//!
//! The launch is split around the leader's planning pass: [`launch_mpp_prepare`] builds the DSM
//! and spawns the workers parked on the go flag, so worker process startup overlaps the
//! leader's planning; [`launch_mpp_commit`] derives the dispatch payload from the leader's plan
//! and releases them. One plan serves both dispatch and the leader's own execution, and every
//! planning fallback aborts the parked workers before they touch the mesh.

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
    estimate_dsm_size, leader_setup, producer_worker_count, worker_setup, MppLeaderState,
};
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
/// `ParallelProcessBuilder::build`), so the name must match the string in [`prepare_mpp_aggregate`].
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
/// [`prepare_mpp_join`].
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
    // The wait spans the leader's planning pass, so back off to sleeping after a burst of
    // yields; spinning here would steal cores from the planner.
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

/// DSM prepared for an MPP launch, before the leader has planned. The workers are already
/// spawned but parked on the go flag: their process startup (library load, backend init)
/// overlaps the leader's planning pass, while the go flag keeps them off the mesh until the
/// payload exists. The leader plans against `scan_ptr`, then hands the physical plan to
/// [`launch_mpp_commit`].
pub struct MppLaunchPrep {
    attach: crate::parallel_worker::builder::ParallelProcessAttach,
    pub scan_ptr: *mut ParallelScanState,
    producer_count: u32,
    payload_capacity: usize,
}

/// Where MPP sits in a scan's launch lifecycle. Every transition consumes the previous stage,
/// so a scan is in exactly one stage at a time; a single field keeps the impossible
/// combinations (prepared and launched at once, say) unrepresentable. Held only by the leader;
/// builder-launched workers reconstruct their state from DSM and never carry this.
#[derive(Default)]
pub enum MppLifecycle {
    /// Serial execution: the query didn't qualify, a fallback abandoned the launch, or
    /// teardown already reclaimed the leader state.
    #[default]
    Inactive,
    /// Serialized logical-plan bytes, stashed at begin time. Prepare uses their length to size
    /// the DSM payload region before the physical plan exists.
    PlanBytes(Vec<u8>),
    /// The DSM is built and the producer workers are spawned, parked on the go flag while the
    /// leader plans.
    Prepared(MppLaunchPrep),
    /// The workers are running dispatched fragments; carries the leader's mesh and finish
    /// handles until teardown.
    Launched(MppLeaderState),
}

impl MppLifecycle {
    /// Consume the stashed plan bytes. Leaves `Inactive`, so a prepare fallback reads as the
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

    /// Consume the prepared launch. Leaves `Inactive`; [`launch_mpp_commit`] decides whether
    /// the scan moves to `Launched` or stays serial.
    pub fn take_prep(&mut self) -> Option<MppLaunchPrep> {
        match std::mem::take(self) {
            MppLifecycle::Prepared(prep) => Some(prep),
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

    pub fn is_prepared(&self) -> bool {
        matches!(self, MppLifecycle::Prepared(_))
    }

    pub fn is_launched(&self) -> bool {
        matches!(self, MppLifecycle::Launched(_))
    }
}

/// Build the MPP DSM (mesh region, `ParallelScanState`, go flag) and spawn the workers parked
/// on the go flag. `None` covers the fallbacks that must not deploy MPP: DSM too large, or no
/// parallel context available.
fn launch_mpp_prepare(
    plan_bytes_len: usize,
    args: ParallelScanArgs,
    worker_entrypoint: &'static str,
) -> Option<MppLaunchPrep> {
    let producer_count = producer_worker_count();
    let payload_capacity = dispatch_plan_capacity(plan_bytes_len);

    let region_bytes = match estimate_dsm_size(payload_capacity) {
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
    // the source manifests `args` borrows. Done before planning so the leader's plan (the same
    // one the dispatch payload is derived from) binds to the shared state the workers will use.
    let scan_ptr = match launcher.state_manager().slice_mut::<u8>(SCAN_IDX) {
        Ok(Some(s)) => s.as_mut_ptr() as *mut ParallelScanState,
        _ => pgrx::error!("mpp: parallel scan state region missing"),
    };
    unsafe { (*scan_ptr).create_and_populate(args) };

    // Spawn the workers before the leader plans: their process startup overlaps the planning
    // pass, and the go flag keeps them off the mesh until the payload is written.
    let attach = launcher.launch()?;

    Some(MppLaunchPrep {
        attach,
        scan_ptr,
        producer_count,
        payload_capacity,
    })
}

/// Serialize the leader's plan into the dispatch payload, release the workers, and return the
/// leader's mesh state, or `None` to run serially (the machine couldn't give us the full
/// producer set, or the plan has nothing to distribute). A `pgrx::error!` is reserved for setup
/// that already committed a launched worker to the mesh, where a silent serial fallback would
/// hide a real bug.
pub fn launch_mpp_commit(
    prep: MppLaunchPrep,
    physical: &std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>,
) -> Option<MppLeaderState> {
    let MppLaunchPrep {
        attach,
        scan_ptr,
        producer_count,
        payload_capacity,
    } = prep;

    let mut timing = crate::postgres::customscan::mpp::glue::MppLaunchTiming::default();

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

    // A plan with no producer stages has nothing to distribute; the workers would only exit
    // with no fragments while the leader runs a plan whose per-source scans aren't executable
    // without a worker's state. Release the workers and let the caller replan serially.
    if stage_count == 0 {
        go.store(GO_ABORT, Ordering::Release);
        finish.wait_for_finish();
        return None;
    }

    // Initialize the leader's rings now that we're committed to the parallel path. After launch on
    // purpose: the serial fallbacks above never create the DSM-backed control senders.
    let mesh_ptr = match finish.state_manager().slice_mut::<u8>(MESH_IDX) {
        Ok(Some(s)) => s.as_mut_ptr() as *mut c_void,
        _ => pgrx::error!("mpp: mesh region missing"),
    };
    let t_setup = std::time::Instant::now();
    let mut leader = match unsafe { leader_setup(mesh_ptr, payload) } {
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
    leader.parallel_state = scan_ptr;
    Some(leader)
}

/// AggregateScan prepare entry: aggregate worker symbol.
pub fn prepare_mpp_aggregate(
    plan_bytes_len: usize,
    args: ParallelScanArgs,
) -> Option<MppLaunchPrep> {
    launch_mpp_prepare(plan_bytes_len, args, "mpp_launched_worker_agg")
}

/// JoinScan prepare entry: join worker symbol.
pub fn prepare_mpp_join(plan_bytes_len: usize, args: ParallelScanArgs) -> Option<MppLaunchPrep> {
    launch_mpp_prepare(plan_bytes_len, args, "mpp_launched_worker_join")
}
