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
//! parallel aggregate use. The leader picks the requested worker count; if PostgreSQL attaches
//! fewer (but still at least two), tasks safely multiplex onto the attached workers instead of
//! falling back to serial execution.
//!
//! The MPP DSM region rides as builder `ParallelState` entries instead of a hand-laid coordinate:
//! a reserve-only region for the SPSC byte-ring mesh (`dsm_stream::dsm_region_bytes`), a zeroed
//! byte blob for the `ParallelScanState`, plus a go flag. Workers reconstruct their
//! `MppWorkerInputs` from the same entries, with no PG plan node in reach.
//!
//! The launch is plan-first (`#5667`), mirroring core PG's parallel-query order (plan ->
//! size-by-walking-the-plan -> create DSM -> bind pointers -> launch): the caller builds the
//! distributed physical plan with `producer_worker_cap()` (PG's parallelism GUCs) as the
//! planner's ceiling, then [`launch_mpp_join`] / [`launch_mpp_aggregate`] walk the finished plan
//! to learn the largest producer-stage task count, size the mesh and spawn exactly
//! `clamp(max_tasks, 2, cap)` producers, and stamp the shared scan state into the plan
//! ([`crate::scan::execution_plan::stamp_parallel_state`]). A plan with no producer stages spawns
//! nothing at all. One plan serves both on-demand stage serialization
//! ([`crate::postgres::customscan::mpp::glue::StagePlanDispatchSource`]) and the leader's own
//! execution; the go flag holds spawned workers off the mesh until the DSM SPSC rings are
//! initialized. If PostgreSQL attaches fewer workers, the plan does not change: the leader just
//! initializes a narrower mesh inside the region it already reserved, and the plan's tasks
//! multiplex onto the attached producers.

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
    MIN_TOTAL_WORKER_COUNT, MppLeaderState, collect_stages, estimate_dsm_size, leader_setup,
    max_producer_task_count, mpp_is_active, producer_worker_cap, stages_have_data_parallelism,
    worker_setup,
};
use crate::postgres::customscan::mpp::interrupt::PgTokioRuntime;
use crate::postgres::customscan::mpp::worker::{MppWorkerInputs, run_mpp_worker};
use crate::postgres::{ParallelScanArgs, ParallelScanState};
use crate::scan::info::{RowEstimate, ScanInfo};

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
/// `2 * W^2` 64 KiB SPSC ring headers in place inside DSM (`~8 MiB` at `W = 8`), so materializing
/// (zeroing, copying) a host-side buffer of that size per query would buy nothing. The scan state
/// is a small zeroed blob populated in place.
struct MppParallelProcess {
    mesh_region: crate::parallel_worker::UninitializedBytesParallelState,
    scan_state: Vec<u8>,
    go: u32,
}

/// The only launch outcomes that preserve the MPP mesh invariant.
///
/// PostgreSQL can attach fewer workers than requested, but the mesh needs at least two
/// producers. More workers than requested would contradict the DSM allocation made for this
/// launch and must not be treated as a valid width.
#[derive(Debug, PartialEq, Eq)]
enum MppAttachOutcome {
    SerialFallback,
    Parallel,
}

fn mpp_attach_outcome(
    requested_producers: u32,
    attached_producers: u32,
) -> Result<MppAttachOutcome, String> {
    if attached_producers > requested_producers {
        return Err(format!(
            "mpp: {attached_producers} workers attached after requesting {requested_producers}"
        ));
    }

    if attached_producers < MIN_TOTAL_WORKER_COUNT - 1 {
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

/// AggregateScan worker entry point. PG resolves this symbol by name (passed to
/// `ParallelProcessBuilder::build`), so the name must match the string in
/// [`launch_mpp_aggregate`].
#[unsafe(no_mangle)]
#[pgrx::pg_guard]
pub unsafe extern "C-unwind" fn mpp_launched_worker_agg(
    seg: *mut pg_sys::dsm_segment,
    toc: *mut pg_sys::shm_toc,
) {
    let (state_manager, _mq_sender) = generic_parallel_worker_entry_point(seg, toc, MPP_MQ_SIZE);
    run_launched_worker(state_manager);
    // `_mq_sender` drops here, detaching the completion queue so the leader's `wait_for_finish`
    // recv loop terminates.
}

/// JoinScan worker entry point. PG resolves this symbol by name; it must match the string in
/// [`launch_mpp_join`].
#[unsafe(no_mangle)]
#[pgrx::pg_guard]
pub unsafe extern "C-unwind" fn mpp_launched_worker_join(
    seg: *mut pg_sys::dsm_segment,
    toc: *mut pg_sys::shm_toc,
) {
    let (state_manager, _mq_sender) = generic_parallel_worker_entry_point(seg, toc, MPP_MQ_SIZE);
    run_launched_worker(state_manager);
}

/// Shared worker body: wait for the leader's go signal, attach to the SPSC ring mesh, reconstruct
/// `MppWorkerInputs` from the DSM entries, and run the Tonic gRPC worker server.
fn run_launched_worker(state_manager: ParallelStateManager) {
    let go = unsafe { go_flag(&state_manager) };
    // Under the plan-first launch the leader has already planned; this wait only spans the
    // leader's ring init (`leader_setup`) and short-launch decision. Still back off to
    // sleeping after a burst of yields rather than spin.
    let mut spins = 0u32;
    loop {
        check_for_interrupts!();
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
    let worker = match unsafe { worker_setup(region_ptr, worker_number) } {
        Ok(session) => session,
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
        parallel_state: scan_ptr,
        plan_sources_count,
        session: worker,
    };

    let runtime = match PgTokioRuntime::new() {
        Ok(rt) => rt,
        Err(e) => pgrx::error!("mpp worker: tokio runtime build failed: {e}"),
    };
    run_mpp_worker(inputs, &runtime);
}

/// Where MPP sits in a scan's launch lifecycle. Every transition consumes the previous stage,
/// so a scan is in exactly one stage at a time; a single field keeps the impossible
/// combinations unrepresentable. Held only by the leader; builder-launched workers reconstruct
/// their state from DSM and never carry this.
#[derive(Default)]
#[allow(clippy::large_enum_variant)]
pub enum MppLifecycle {
    /// Serial execution: the query didn't qualify, a fallback abandoned the launch, or
    /// teardown already reclaimed the leader state.
    #[default]
    Inactive,
    /// The scan qualified for one MPP planning and launch attempt. No plan is stored here: the
    /// finished physical stages produce the exact dispatch payload before DSM allocation.
    Pending,
    /// The workers are running dispatched fragments; carries the leader's mesh and finish
    /// handles until teardown.
    Launched(MppLeaderState),
}

impl MppLifecycle {
    /// Consume the pending launch marker. Leaves `Inactive`, so a launch fallback reads as the
    /// serial path from then on.
    pub fn take_pending(&mut self) -> bool {
        match std::mem::take(self) {
            MppLifecycle::Pending => true,
            other => {
                *self = other;
                false
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

/// True when the size gate keeps MPP off: the largest source's estimated match count sits
/// under `paradedb.mpp_min_rows`. Matches Postgres' refusal to parallelize small scans
/// (`min_parallel_table_scan_size`): below this size the launch cost (worker spawn, plan
/// dispatch, per-worker index opens) dominates whatever the split saves. The largest source
/// stands for the scan: it is the bulk of the work the split divides, and the smaller sides
/// ride along. A selective query over a large index does little scan work, so each source
/// counts estimated matches, not index documents; an estimate substituted from the index's
/// total document count (Postgres expressions, heap filters) is discounted by
/// `PARAMETERIZED_SELECTIVITY`, mirroring BaseScan.
///
/// Shared by the launch and by the plain-EXPLAIN plan rebuilds, the same split as
/// [`mpp_plan_has_data_parallelism`](crate::postgres::customscan::mpp::glue::mpp_plan_has_data_parallelism),
/// so the rendered plan agrees with the executed mode (#5784).
///
/// `RowEstimate::Unknown` does not gate. The join estimator always produces `Known` for real
/// sources, so `Unknown` marks a placeholder; silently serializing a big query would cost far
/// more than a wasted launch.
pub(crate) fn mpp_gated_by_min_rows<'a>(sources: impl IntoIterator<Item = &'a ScanInfo>) -> bool {
    gated_by_min_rows(
        crate::gucs::mpp_min_rows() as u64,
        sources
            .into_iter()
            .map(|info| (info.estimate, info.estimate_from_total_docs)),
    )
}

/// Whether a scan over `plan` may attempt an MPP launch: the statement allows parallel mode
/// (#6157), PG's worker budget admits producers, and the query clears the size gate (#5784).
/// Shared by launch preparation and the plain-EXPLAIN plan rebuild so both agree.
pub(crate) fn mpp_eligible(parallel_mode_ok: bool, plan: &RelNode) -> bool {
    parallel_mode_ok
        && mpp_is_active()
        && !mpp_gated_by_min_rows(plan.sources().into_iter().map(|s| &s.scan_info))
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
/// the DSM, stamp the shared scan state into the plan, spawn exactly the needed producers, and
/// dispatch. `None` means run serially — the plan has nothing to distribute, the DSM couldn't
/// be built, or the machine could not attach the minimum two producers needed for an MPP mesh.
/// Nothing is forked on the nothing-to-distribute path. `None` covers only environmental
/// shortfalls; a `pgrx::error!` means an invariant breach or a failure past mesh commitment,
/// where a silent serial fallback would hide a real bug.
fn launch_mpp(
    physical: &std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>,
    args: ParallelScanArgs,
    worker_entrypoint: &'static str,
) -> Option<MppLeaderState> {
    let mut timing = crate::postgres::customscan::mpp::glue::MppLaunchTiming::default();

    let stages = collect_stages(physical);

    // Task counts were capped by `target_partitions = cap` at plan time and reflect segment
    // counts (#5657). The producer floor keeps the mesh-width invariant; see
    // [`MIN_TOTAL_WORKER_COUNT`]. Same gate as plain EXPLAIN (#5784).
    if !stages_have_data_parallelism(&stages) {
        // 0: nothing to distribute. 1: no data parallelism — every 1-task stage lands on
        // proc 1 (`proc_for_task`), leaving the second (floor) producer idle. Run serially.
        // This also keeps `producer_count <= max_tasks`, so every launched proc owns at
        // least one fragment of the widest stage.
        return None;
    }
    let max_tasks = max_producer_task_count(&stages);
    let cap = producer_worker_cap();
    let producer_count = (max_tasks as u32).clamp(MIN_TOTAL_WORKER_COUNT - 1, cap);

    let t_prepare = std::time::Instant::now();
    let region_bytes = match estimate_dsm_size(producer_count + 1) {
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
        worker_entrypoint,
        WorkerStyle::Query,
        producer_count as usize,
        MPP_MQ_SIZE,
    )?;

    // Populate the ParallelScanState in place while the DSM is mapped.
    let scan_ptr = match launcher.state_manager().slice_mut::<u8>(SCAN_IDX) {
        Ok(Some(s)) => s.as_mut_ptr() as *mut ParallelScanState,
        _ => pgrx::error!("mpp: parallel scan state region missing"),
    };
    unsafe { (*scan_ptr).create_and_populate(args) };

    timing.prepare_us = t_prepare.elapsed().as_micros() as u64;

    // Spawn the workers; the go flag keeps them off the mesh until the rings and the dispatch
    // payload are initialized below. The trace doubles as the regress observable that a
    // launch was attempted at all (mpp_worker_sizing).
    crate::mpp_log!("launch: spawning {producer_count} producers");
    let attach = launcher.launch()?;

    let t_attach = std::time::Instant::now();
    let finish = attach.wait_for_attach()?;
    timing.attach_us = t_attach.elapsed().as_micros() as u64;
    let launched = finish.launched_workers() as u32;
    timing.workers = launched;

    let go = unsafe { go_flag(finish.state_manager()) };

    match mpp_attach_outcome(producer_count, launched) {
        Ok(MppAttachOutcome::SerialFallback) => {
            // The machine could not give us the minimum two producers required for the MPP mesh.
            // The launched workers are still on the go flag with no rings attached; release them
            // and run serially. No `leader_setup` ran, so there are no DSM-backed senders to
            // outlive the mapping.
            go.store(GO_ABORT, Ordering::Release);
            finish.wait_for_finish();
            pgrx::warning!(
                "mpp: launched {launched} of {producer_count} requested workers; running serially"
            );
            return None;
        }
        Ok(MppAttachOutcome::Parallel) => {}
        Err(e) => pgrx::error!("{e}"),
    }

    // PostgreSQL may attach fewer workers than requested. The plan and its task counts stand:
    // `MppWorkerResolver(launched)` and `RoundRobinRouteTaskHandler` assign tasks across the
    // attached workers, and same-worker stage reads short-circuit in memory via
    // `LocalWorkerContext`.
    if launched != producer_count {
        crate::mpp_log!(
            "launch: {launched} of {producer_count} producers attached; multiplexing original task plan"
        );
    }

    // Bind the shared scan state into the plan the leader will execute. Must stay below the
    // last serial-fallback `return None`: a stamped plan must never outlive its DSM, and the
    // fallback paths replan. Workers are unaffected: stage plan encodes are context-free and
    // each worker injects its own pointer at decode.
    crate::scan::execution_plan::stamp_parallel_state(physical, scan_ptr);

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
    let mut leader = match unsafe { leader_setup(mesh_ptr, launched + 1) } {
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

/// AggregateScan launch entry: aggregate worker symbol.
pub fn launch_mpp_aggregate(
    physical: &std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>,
    args: ParallelScanArgs,
) -> Option<MppLeaderState> {
    launch_mpp(physical, args, "mpp_launched_worker_agg")
}

/// JoinScan launch entry: join worker symbol.
pub fn launch_mpp_join(
    physical: &std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>,
    args: ParallelScanArgs,
) -> Option<MppLeaderState> {
    launch_mpp(physical, args, "mpp_launched_worker_join")
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::pg_test;

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

    #[pg_test]
    fn pending_launch_is_consumed_once() {
        let mut lifecycle = MppLifecycle::Pending;

        assert!(lifecycle.take_pending());
        assert!(!lifecycle.take_pending());
    }

    #[pg_test]
    fn short_launch_uses_the_attached_width_when_the_mesh_is_viable() {
        assert_eq!(
            mpp_attach_outcome(5, 0),
            Ok(MppAttachOutcome::SerialFallback)
        );
        assert_eq!(
            mpp_attach_outcome(5, 1),
            Ok(MppAttachOutcome::SerialFallback)
        );
        assert_eq!(mpp_attach_outcome(5, 2), Ok(MppAttachOutcome::Parallel));
        assert_eq!(mpp_attach_outcome(5, 3), Ok(MppAttachOutcome::Parallel));
        assert_eq!(mpp_attach_outcome(5, 5), Ok(MppAttachOutcome::Parallel));
        assert!(mpp_attach_outcome(5, 6).is_err());
    }

    #[pg_test]
    fn mpp_aggregate_executes_across_parallel_workers() {
        use std::sync::Arc;

        use arrow_array::Int64Array;
        use datafusion::physical_plan::collect;
        use datafusion::prelude::SessionContext;
        use datafusion_distributed::DistributedExt;
        use pgrx::Spi;

        use crate::index::fast_fields_helper::WhichFastField;
        use crate::index::mvcc::MvccSatisfies;
        use crate::index::reader::index::SearchIndexManifest;
        use crate::postgres::customscan::mpp::glue::{
            StagePlanDispatchSource, finish_leader_execution, merge_worker_metrics,
        };
        use crate::postgres::customscan::mpp::worker::build_mpp_session_context;
        use crate::postgres::rel::PgSearchRelation;
        use crate::scan::info::ScanInfo;
        use crate::scan::table_provider::PgSearchTableProvider;
        use crate::schema::SearchFieldType;

        Spi::run("SET paradedb.global_mutable_segment_rows = 0;").unwrap();
        Spi::run("CREATE TABLE mpp_pg_test (id INT, category INT, val INT);").unwrap();
        Spi::run("CREATE INDEX mpp_pg_test_idx ON mpp_pg_test USING paradedb (id, category, val);")
            .unwrap();
        Spi::run("INSERT INTO mpp_pg_test SELECT i, i % 3, i * 10 FROM generate_series(1, 30) i;")
            .unwrap();
        Spi::run("INSERT INTO mpp_pg_test SELECT i, i % 3, i * 10 FROM generate_series(31, 60) i;")
            .unwrap();
        Spi::run("RESET paradedb.global_mutable_segment_rows;").unwrap();
        Spi::run("SET max_parallel_workers = 8;").unwrap();
        Spi::run("SET max_parallel_workers_per_gather = 2;").unwrap();

        let heap_oid = Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_class WHERE relname = 'mpp_pg_test' AND relkind = 'r';",
        )
        .unwrap()
        .unwrap();
        let index_oid = Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_class WHERE relname = 'mpp_pg_test_idx' AND relkind = 'i';",
        )
        .unwrap()
        .unwrap();

        unsafe {
            pg_sys::CommandCounterIncrement();
            let snap = pg_sys::GetTransactionSnapshot();
            pg_sys::PushActiveSnapshot(snap);
        }

        let index_rel = PgSearchRelation::open(index_oid);
        let manifest = SearchIndexManifest::capture(&index_rel, MvccSatisfies::Snapshot).unwrap();
        assert!(
            manifest.segment_view().len() >= 2,
            "expected at least 2 segments for parallel MPP scan"
        );

        let fields = vec![
            WhichFastField::Ctid,
            WhichFastField::eager("id".to_string(), SearchFieldType::I64(pg_sys::INT4OID)),
            WhichFastField::eager(
                "category".to_string(),
                SearchFieldType::I64(pg_sys::INT4OID),
            ),
            WhichFastField::eager("val".to_string(), SearchFieldType::I64(pg_sys::INT4OID)),
        ];
        let mut scan_info = ScanInfo::new(1, heap_oid, index_oid, crate::scan::ScanMode::all());
        for (i, field) in fields.iter().enumerate() {
            scan_info.add_field(i as pg_sys::AttrNumber, field.clone());
        }
        let mut provider = PgSearchTableProvider::new(scan_info, fields, Some(0));
        provider.set_manifest(manifest.clone());

        let runtime = PgTokioRuntime::new().unwrap();

        let plan_ctx = build_mpp_session_context(SessionContext::new(), None);
        plan_ctx.register_table("t", Arc::new(provider)).unwrap();

        let physical_plan = runtime.block_on(async {
            let df = plan_ctx
                .sql("SELECT category, COUNT(*) AS cnt, SUM(val) AS total FROM t GROUP BY category ORDER BY category")
                .await
                .unwrap();
            df.create_physical_plan().await.unwrap()
        });

        // Concurrent `#[pg_test]` tests (e.g. `build_parallel`) share the cluster-wide
        // `max_worker_processes` pool and can temporarily starve `LaunchParallelWorkers`.
        let mut leader = (0..200)
            .find_map(|attempt| {
                if attempt > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                let args = ParallelScanArgs {
                    all_sources: vec![manifest.segment_view()],
                    with_aggregates: false,
                    with_segment_info: false,
                };
                launch_mpp_aggregate(&physical_plan, args)
            })
            .expect("expected launch_mpp_aggregate to spawn parallel workers");
        assert_eq!(leader.timing.workers, 2);

        let exec_ctx = build_mpp_session_context(
            SessionContext::new(),
            Some(Arc::clone(&leader.session.resolver)),
        )
        .with_distributed_dispatch_plan_source(StagePlanDispatchSource::default());

        let batches = runtime.block_on(async {
            collect(Arc::clone(&physical_plan), exec_ctx.task_ctx())
                .await
                .unwrap()
        });

        let mut rows: Vec<(i64, i64, i64)> = Vec::new();
        for batch in &batches {
            let cats = batch
                .column(0)
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap();
            let cnts = batch
                .column(1)
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap();
            let sums = batch
                .column(2)
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap();
            for i in 0..batch.num_rows() {
                rows.push((cats.value(i), cnts.value(i), sums.value(i)));
            }
        }
        assert_eq!(rows, vec![(0, 20, 6300), (1, 20, 5900), (2, 20, 6100)]);

        finish_leader_execution(Some(&runtime), Some(&physical_plan), &leader);
        let merged = merge_worker_metrics(&physical_plan);
        assert!(merged.is_some(), "expected worker metrics to be merged");

        if let Some(mut finish) = leader.finish.take() {
            let _ = finish.recv();
            finish.wait_for_finish();
        }
        drop(leader);
        drop(runtime);
        unsafe {
            pg_sys::PopActiveSnapshot();
        }
    }
}
