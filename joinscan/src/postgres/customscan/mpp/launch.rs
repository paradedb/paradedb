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
#![allow(unsafe_op_in_unsafe_fn)]

//! Leader-driven MPP worker launch.
//!
//! The leader spawns its background workers itself through `parallel_worker::builder`
//! (`CreateParallelContext` + `LaunchParallelWorkers`), the same path index builds and the
//! parallel aggregate use. The leader picks the requested background worker count based on
//! stage task counts and `parallel_leader_participation`; if PostgreSQL attaches fewer (but
//! still enough for at least two task-executing participants), tasks safely multiplex onto the
//! attached participants instead of falling back to serial execution.
//!
//! The MPP DSM region rides as builder `ParallelState` entries instead of a hand-laid coordinate:
//! a reserve-only region for the SPSC byte-ring mesh (`dsm_stream::dsm_region_bytes`), a zeroed
//! byte blob for the `ParallelScanState`, plus a go flag. Workers reconstruct their
//! `MppWorkerInputs` from the same entries, with no PG plan node in reach.
//!
//! The launch is plan-first (`#5667`), mirroring core PG's parallel-query order (plan ->
//! size-by-walking-the-plan -> create DSM -> bind pointers -> launch): the caller builds the
//! distributed physical plan with the scan's participant ceiling as the planner's `target_partitions`,
//! then [`launch_mpp`] walks the finished plan to learn the largest producer-stage task count,
//! size the mesh, spawn the needed background workers, and stamp the shared scan state into the
//! plan ([`crate::scan::execution_plan::stamp_parallel_state`]). A plan with no data parallelism
//! spawns nothing at all. One plan serves both on-demand stage serialization and the leader's own
//! execution; the go flag holds spawned workers off the mesh until the DSM SPSC rings are
//! initialized. If PostgreSQL attaches fewer workers, the plan does not change: the leader just
//! initializes a narrower mesh inside the region it already reserved, and the plan's tasks
//! multiplex onto the active participants.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use pgrx::{check_for_interrupts, pg_sys};

use crate::parallel_worker::builder::ParallelProcessBuilder;
use crate::parallel_worker::{
    ParallelProcess, ParallelState, ParallelStateManager, WorkerStyle,
    generic_parallel_worker_entry_point,
};
use crate::postgres::customscan::joinscan::build::RelNode;
use crate::postgres::customscan::mpp::glue::{
    BgWorkerCount, LeaderParticipation, MppBudget, MppLeaderState, bg_worker_cap, collect_stages,
    debug_parallel_query_is_active, estimate_dsm_size, leader_setup, max_producer_task_count,
    worker_setup,
};
use crate::postgres::customscan::mpp::interrupt::PgTokioRuntime;
use crate::postgres::customscan::mpp::wakeup::{has_available_wakeup_slots, reserve_wakeup_slots};
use crate::postgres::customscan::mpp::worker::{MppWorkerInputs, run_mpp_worker};
use crate::postgres::customscan::parallel::RelParallelWorkers;
use crate::postgres::{ParallelScanArgs, ParallelScanState};
use crate::scan::execution_plan::prepare_vector_routing;
use crate::scan::info::RowEstimate;

/// `state_values()` order. Each index maps to a `ParallelState` TOC entry the workers look up.
const MESH_IDX: usize = 0;
pub(super) const SCAN_IDX: usize = 1;
const GO_IDX: usize = 2;

/// Go-flag states. The leader sets `RUN` once the mesh is initialized, or `ABORT` if too few
/// workers attached for an MPP mesh.
const GO_WAIT: u32 = 0;
const GO_RUN: u32 = 1;
const GO_ABORT: u32 = 2;

/// Per-worker completion queue size. MPP carries data over the SPSC ring mesh, not this queue; it
/// only serves as the leader's detach barrier in `wait_for_finish`. Matches the index-build size
/// so `shm_mq_create` has room for its header.
const MPP_MQ_SIZE: usize = 1024;

/// The builder process carrying the MPP DSM entries. The mesh region is reserve-only:
/// `glue::leader_setup` (`dsm_stream::init_dsm_mesh`) initializes the header, wakeup slots, and
/// `2 * N * (N - 1)` SPSC ring headers (`N = W + 1`, sized from `paradedb.mpp_queue_size` across
/// all rings) in place inside DSM, so materializing (zeroing, copying) a host-side buffer of that
/// size per query would buy nothing. The scan state is a small zeroed blob populated in place.
struct MppParallelProcess {
    mesh_region: crate::parallel_worker::UninitializedBytesParallelState,
    scan_state: Vec<u8>,
    go: u32,
}

/// The only launch outcomes that preserve the MPP mesh invariant.
///
/// PostgreSQL can attach fewer workers than requested, so long as at least one background worker
/// attached and the resulting participant set (`attached + leader_share`) has at least two
/// task-executing participants. More workers than requested would contradict the DSM allocation
/// made for this launch and must not be treated as a valid width.
#[derive(Debug, PartialEq, Eq)]
enum MppAttachOutcome {
    SerialFallback,
    Parallel,
}

fn mpp_attach_outcome(
    requested: BgWorkerCount,
    attached: BgWorkerCount,
    leader: LeaderParticipation,
) -> Result<MppAttachOutcome, String> {
    if attached > requested {
        return Err(format!(
            "mpp: {} workers attached after requesting {}",
            attached.get(),
            requested.get()
        ));
    }

    if attached < leader.min_mpp_bg_workers() {
        Ok(MppAttachOutcome::SerialFallback)
    } else {
        Ok(MppAttachOutcome::Parallel)
    }
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

/// MPP parallel worker entry point. PG resolves this symbol by name (passed to
/// `ParallelProcessBuilder::build`), so the name must match the string in [`launch_mpp`].
#[unsafe(no_mangle)]
#[pgrx::pg_guard]
pub unsafe extern "C-unwind" fn mpp_launched_worker(
    seg: *mut pg_sys::dsm_segment,
    toc: *mut pg_sys::shm_toc,
) {
    let (state_manager, _mq_sender) = generic_parallel_worker_entry_point(seg, toc, MPP_MQ_SIZE);
    run_launched_worker(seg, state_manager);
    // `_mq_sender` drops here, detaching the completion queue so the leader's `wait_for_finish`
    // recv loop terminates.
}

/// Shared worker body: wait for the leader's go signal, attach to the SPSC ring mesh, reconstruct
/// `MppWorkerInputs` from the DSM entries, and run the Tonic gRPC worker server.
fn run_launched_worker(seg: *mut pg_sys::dsm_segment, state_manager: ParallelStateManager) {
    let go = unsafe { go_flag(&state_manager) };
    // Under the plan-first launch the leader has already planned; this wait only spans the
    // leader's ring init (`leader_setup`) and short-launch decision. Still back off to
    // sleeping after a burst of yields rather than spin.
    let mut spins = 0u32;
    loop {
        check_for_interrupts();
        match go.load(Ordering::Acquire) {
            GO_RUN => break,
            // Too few workers attached for MPP: the leader runs serially. Exit before touching
            // the mesh. A short-but-viable launch instead reaches GO_RUN and multiplexes tasks.
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
    let worker_number = unsafe { pg_sys::ParallelWorkerNumber };
    let worker = match unsafe { worker_setup(region_ptr, seg, worker_number) } {
        Ok(session) => session,
        Err(e) => pgrx::error!("mpp worker: worker_setup failed: {e}"),
    };

    // The leader populated the ParallelScanState before launch; read the canonical
    // segment sets from it.
    let scan_ptr = match state_manager.slice_mut::<u8>(SCAN_IDX) {
        Ok(Some(s)) => s.as_mut_ptr() as *mut ParallelScanState,
        _ => pgrx::error!("mpp worker: parallel scan state missing from parallel state"),
    };

    let inputs = MppWorkerInputs {
        parallel_state: scan_ptr,
        session: worker,
    };

    let runtime = match PgTokioRuntime::new() {
        Ok(rt) => rt,
        Err(e) => pgrx::error!("mpp worker: tokio runtime build failed: {e}"),
    };
    run_mpp_worker(inputs, &runtime);
}

/// Compute the effective [`MppBudget`] when a scan over `plan` qualifies for MPP, or `None` when
/// MPP is ineligible or gated off.
///
/// Enforces three checks in a single pass over `plan.sources()`:
/// 1. `parallel_mode_ok`: the statement, absorbed sources, and pushed-down expressions allow
///    parallel mode (`#6157`).
/// 2. Participant budget: starts from PostgreSQL's global parallel worker GUCs ([`bg_worker_cap`]),
///    applies per-relation `ALTER TABLE ... SET (parallel_workers = N)` across BM25-indexed
///    sources (`Disabled` -> `0`, `Fixed(n)` -> `min(n, global)`, `Unset` -> `global`, taking the
///    `max` across sources like PostgreSQL's parallel join paths), and requires at least
///    [`LeaderParticipation::min_mpp_bg_workers`] (`>= 2` total participants).
/// 3. Row-estimate gate (`#5784`): the largest source's estimated match count (discounting
///    total-doc fallback estimates by `PARAMETERIZED_SELECTIVITY`, with `RowEstimate::Unknown`
///    left ungated) must meet `paradedb.mpp_min_rows`, unless `debug_parallel_query` is active or
///    an indexed source explicitly sets `parallel_workers = N` (`N > 0`), matching PostgreSQL's
///    `compute_parallel_worker`.
///
/// TODO: Evaluate aligning MPP's size and cost gates with PostgreSQL's
/// `min_parallel_table_scan_size`, `min_parallel_index_scan_size`, `parallel_setup_cost`, and
/// `parallel_tuple_cost` GUCs rather than gating solely on `paradedb.mpp_min_rows`.
///
/// Shared by plan-time mode selection, plain-`EXPLAIN` rebuilds, and execution-time launch so all
/// three agree.
pub(crate) fn mpp_budget(parallel_mode_ok: bool, plan: &RelNode) -> Option<MppBudget> {
    if !parallel_mode_ok {
        return None;
    }
    let leader = LeaderParticipation::current();
    let global = bg_worker_cap();
    let sources = plan.sources();

    let mut saw_indexed_source = false;
    let mut effective_bg = 0u32;
    let mut has_fixed_rel_workers = false;
    for source in &sources {
        let info = &source.scan_info;
        if info.indexrelid == pg_sys::InvalidOid {
            continue;
        }
        saw_indexed_source = true;
        let source_cap = match RelParallelWorkers::from_opt(info.rel_parallel_workers) {
            RelParallelWorkers::Disabled => 0,
            RelParallelWorkers::Fixed(n) => {
                has_fixed_rel_workers = true;
                (n.get() as u32).min(global.get())
            }
            RelParallelWorkers::Unset => global.get(),
        };
        effective_bg = effective_bg.max(source_cap);
    }

    let bg_cap = if saw_indexed_source {
        BgWorkerCount::new(effective_bg)
    } else {
        global
    };
    if bg_cap < leader.min_mpp_bg_workers() {
        return None;
    }

    if !debug_parallel_query_is_active()
        && !has_fixed_rel_workers
        && gated_by_min_rows(
            crate::gucs::mpp_min_rows() as u64,
            sources
                .iter()
                .map(|s| (s.scan_info.estimate, s.scan_info.estimate_from_total_docs)),
        )
    {
        return None;
    }

    Some(MppBudget::new(bg_cap, leader))
}

/// Decide on the first `exec_custom_scan` pull whether to attempt an MPP launch, returning the
/// scan's [`MppBudget`] when eligible.
///
/// PostgreSQL's `ExecutePlan` (`execMain.c`) sets `estate->es_use_parallel_mode` only at
/// `ExecutorRun` time: it stays `false` throughout `ExecutorStart` (`begin_custom_scan`) and is
/// left `false` on partial portal fetches (`Execute` with `max_rows > 0`), `SPI_execute(...,
/// tcount > 0)`, and PL/pgSQL `FOR` loops, matching `ExecGather` (`nodeGather.c`). Checking here
/// once on the first tuple pull avoids preparing MPP state during `begin_custom_scan` only to
/// revert it on partial fetches or unpulled (`LIMIT 0`) scans.
///
/// # Safety
/// `estate` must be either null or a valid `EState` pointer for the active executor run.
pub(crate) unsafe fn exec_mpp_budget(
    estate: *mut pg_sys::EState,
    parallel_mode_ok: bool,
    plan: &RelNode,
) -> Option<MppBudget> {
    if pg_sys::ParallelWorkerNumber != -1 {
        return None;
    }
    if !estate.is_null() && !(*estate).es_use_parallel_mode {
        return None;
    }
    mpp_budget(parallel_mode_ok, plan)
}

/// `(estimate, estimate_from_total_docs)` per source, not `&ScanInfo`: the plain unit tests
/// below must not construct (and drop) pgrx-typed values, whose drop glue references server
/// symbols a standalone test binary cannot link.
fn gated_by_min_rows(
    min_rows: u64,
    sources: impl IntoIterator<Item = (RowEstimate, bool)>,
) -> bool {
    if min_rows == 0 {
        return false;
    }
    let mut largest: u64 = 0;
    for (estimate, from_total_docs) in sources {
        let rows = match estimate {
            RowEstimate::Known(n) if from_total_docs => {
                (n as f64 * crate::PARAMETERIZED_SELECTIVITY) as u64
            }
            RowEstimate::Known(n) => n,
            RowEstimate::Unknown => return false,
        };
        largest = largest.max(rows);
    }
    largest < min_rows
}

/// Plan-first MPP launch (#5667): size the worker pool from the finished physical plan, build
/// the DSM, stamp the shared scan state into the plan, spawn the needed background workers, and
/// initialize the mesh. `None` means run serially: the plan has nothing to distribute, the DSM
/// couldn't be built, or the machine could not attach enough background workers for at least two
/// task-executing participants. Nothing is forked on the nothing-to-distribute path. `None`
/// covers only environmental shortfalls; a `pgrx::error!` means an invariant breach or a failure
/// past mesh commitment, where a silent serial fallback would hide a real bug.
///
/// # Safety
/// `estate` must be either null or point to a valid, mutable `EState` for the duration of this call.
pub unsafe fn launch_mpp(
    // `es_parallel_workers_to_launch` / `es_parallel_workers_launched` were added in PG18.
    #[cfg_attr(not(feature = "pg18"), allow(unused_variables))] estate: *mut pg_sys::EState,
    budget: MppBudget,
    physical: &std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>,
    args: ParallelScanArgs,
) -> Option<MppLeaderState> {
    let mut timing = crate::postgres::customscan::mpp::glue::MppLaunchTiming::default();

    let stages = collect_stages(physical);

    // Task counts were capped by `target_partitions = participants` at plan time and reflect
    // segment counts (#5657). Same `max_tasks >= 2` gate as plain EXPLAIN (#5784).
    let requested_bg = budget.for_stage_tasks(max_producer_task_count(&stages))?;

    let min_mesh_procs = budget.leader.min_mpp_bg_workers().mesh_procs() as usize;
    if !has_available_wakeup_slots(min_mesh_procs) {
        pgrx::warning!("mpp: wakeup slot pool exhausted; running serially");
        return None;
    }

    let t_prepare = std::time::Instant::now();
    let region_bytes = match estimate_dsm_size(requested_bg) {
        Ok(sz) => sz,
        Err(e) => {
            pgrx::warning!("mpp: estimate_dsm failed: {e}; running serially");
            return None;
        }
    };
    let scan_size = ParallelScanState::size_of(&args.all_nsegments(), false, false);

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
        "mpp_launched_worker",
        WorkerStyle::Query,
        requested_bg.get() as usize,
        MPP_MQ_SIZE,
    )?;

    // Populate the ParallelScanState in place while the DSM is mapped.
    let scan_ptr = match launcher.state_manager().slice_mut::<u8>(SCAN_IDX) {
        Ok(Some(s)) => s.as_mut_ptr() as *mut ParallelScanState,
        _ => pgrx::error!("mpp: parallel scan state region missing"),
    };
    unsafe { (*scan_ptr).create_and_populate(args) };

    timing.prepare_us = t_prepare.elapsed().as_micros() as u64;

    // Spawn the workers; the go flag keeps them off the mesh until the rings are initialized
    // below. The trace doubles as the integration-test observable (`tests/tests/mpp_worker_sizing.rs`)
    // that a launch was attempted at all.
    crate::mpp_log!(
        "launch: spawning {} producers (leader_participation={:?})",
        requested_bg.get(),
        budget.leader
    );
    #[cfg(feature = "pg18")]
    if !estate.is_null() {
        unsafe { (*estate).es_parallel_workers_to_launch += requested_bg.get() as i32 };
    }
    let attach = launcher.launch()?;
    prepare_vector_routing(physical);

    let t_attach = std::time::Instant::now();
    let finish = attach.wait_for_attach()?;
    timing.attach_us = t_attach.elapsed().as_micros() as u64;
    let launched = BgWorkerCount::new(finish.launched_workers() as u32);
    #[cfg(feature = "pg18")]
    if !estate.is_null() {
        unsafe { (*estate).es_parallel_workers_launched += launched.get() as i32 };
    }
    timing.workers = launched.get();

    let go = unsafe { go_flag(finish.state_manager()) };

    match mpp_attach_outcome(requested_bg, launched, budget.leader) {
        Ok(MppAttachOutcome::SerialFallback) => {
            // The machine could not attach enough background workers for an active MPP mesh.
            // Any launched worker is still on the go flag with no rings attached; release it
            // and run serially. No `leader_setup` ran, so there are no DSM-backed senders to
            // outlive the mapping.
            go.store(GO_ABORT, Ordering::Release);
            finish.wait_for_finish();
            pgrx::warning!(
                "mpp: launched {} of {} requested workers; running serially",
                launched.get(),
                requested_bg.get()
            );
            return None;
        }
        Ok(MppAttachOutcome::Parallel) => {}
        Err(e) => pgrx::error!("{e}"),
    }

    let mesh_procs = launched.mesh_procs() as usize;
    let Some(wakeup_reservation) = reserve_wakeup_slots(mesh_procs) else {
        go.store(GO_ABORT, Ordering::Release);
        finish.wait_for_finish();
        pgrx::warning!(
            "mpp: failed to reserve {} wakeup slots; running serially",
            mesh_procs
        );
        return None;
    };

    // PostgreSQL may attach fewer workers than requested. The plan and its task counts stand:
    // `MppWorkerResolver(launched, leader_participation)` assigns tasks across the active
    // participants, and same-worker stage reads short-circuit in memory via `LocalWorkerContext`.
    if launched != requested_bg {
        crate::mpp_log!(
            "launch: {} of {} producers attached; multiplexing original task plan",
            launched.get(),
            requested_bg.get()
        );
    }

    // Bind the shared scan state into the plan the leader will execute. Must stay below the
    // last serial-fallback `return None`: a stamped plan must never outlive its DSM, and the
    // fallback paths replan. Workers are unaffected: stage plan encodes are context-free and
    // each worker injects its own pointer at decode.
    unsafe { crate::scan::execution_plan::stamp_parallel_state(physical, scan_ptr) };

    // Initialize the leader's rings now that we're committed to the parallel path. After launch on
    // purpose: the serial fallbacks above never create the DSM-backed ring streams.
    let mesh_ptr = match finish.state_manager().slice_mut::<u8>(MESH_IDX) {
        Ok(Some(s)) => s.as_mut_ptr() as *mut c_void,
        _ => pgrx::error!("mpp: mesh region missing"),
    };
    // The narrow mesh depends on attached workers occupying `ParallelWorkerNumber`
    // 0..launched-1 with no holes: PostgreSQL stops registering after the first failure,
    // and `wait_for_attach` reports a worker that dies before attaching. Each worker maps
    // its number to `proc_idx = worker_number + 1`, so every attached worker is in this mesh.
    let t_setup = std::time::Instant::now();
    let mut leader =
        match unsafe { leader_setup(mesh_ptr, launched, budget.leader, wakeup_reservation) } {
            Ok(l) => l,
            Err(e) => pgrx::error!("mpp: leader_setup failed: {e}"),
        };
    timing.leader_setup_us = t_setup.elapsed().as_micros() as u64;
    leader.timing = timing;

    // Registered here (not in `leader_setup`) because this is the first point with both the segment
    // (`finish`) and the leader streams (`leader`) in hand.
    unsafe { leader.register_streams_on_dsm_detach(finish.dsm_segment()) };

    // Release the workers into ring attach + plan wait.
    go.store(GO_RUN, Ordering::Release);

    leader.finish = Some(finish);
    Some(leader)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_takes_the_largest_source() {
        let dim = (RowEstimate::Known(10_000), false);
        let fact = (RowEstimate::Known(2_000_000), false);
        assert!(!gated_by_min_rows(500_000, [dim, fact]));
        assert!(gated_by_min_rows(500_000, [dim]));
    }

    #[test]
    fn total_docs_estimates_are_discounted() {
        // A parameterized predicate stores the whole index's document count. At the BaseScan
        // discount, 600k docs stand for 60k matches: under the default threshold.
        assert!(gated_by_min_rows(
            500_000,
            [(RowEstimate::Known(600_000), true)]
        ));
        assert!(!gated_by_min_rows(
            500_000,
            [(RowEstimate::Known(600_000), false)]
        ));
    }

    #[test]
    fn unknown_estimates_do_not_gate() {
        let placeholder = (RowEstimate::Unknown, false);
        let small = (RowEstimate::Known(10), false);
        assert!(!gated_by_min_rows(500_000, [placeholder, small]));
    }

    #[test]
    fn zero_threshold_disables_the_gate() {
        assert!(!gated_by_min_rows(0, [(RowEstimate::Known(1), false)]));
    }

    #[test]
    fn short_launch_uses_the_attached_width_when_the_mesh_is_viable() {
        let req = BgWorkerCount::new(5);

        // Participating leader: 0 attached workers falls back to serial; 1+ runs in parallel
        // (leader + 1 worker = 2 participants).
        assert_eq!(
            mpp_attach_outcome(
                req,
                BgWorkerCount::new(0),
                LeaderParticipation::Participating
            ),
            Ok(MppAttachOutcome::SerialFallback)
        );
        for attached in [1, 2, 3, 5] {
            assert_eq!(
                mpp_attach_outcome(
                    req,
                    BgWorkerCount::new(attached),
                    LeaderParticipation::Participating
                ),
                Ok(MppAttachOutcome::Parallel)
            );
        }
        assert!(
            mpp_attach_outcome(
                req,
                BgWorkerCount::new(6),
                LeaderParticipation::Participating
            )
            .is_err()
        );

        // Coordinator-only leader: 0 or 1 attached workers falls back to serial; 2+ runs in
        // parallel.
        for attached in [0, 1] {
            assert_eq!(
                mpp_attach_outcome(
                    req,
                    BgWorkerCount::new(attached),
                    LeaderParticipation::CoordinatorOnly
                ),
                Ok(MppAttachOutcome::SerialFallback)
            );
        }
        for attached in [2, 3, 5] {
            assert_eq!(
                mpp_attach_outcome(
                    req,
                    BgWorkerCount::new(attached),
                    LeaderParticipation::CoordinatorOnly
                ),
                Ok(MppAttachOutcome::Parallel)
            );
        }
        assert!(
            mpp_attach_outcome(
                req,
                BgWorkerCount::new(6),
                LeaderParticipation::CoordinatorOnly
            )
            .is_err()
        );
    }
}
