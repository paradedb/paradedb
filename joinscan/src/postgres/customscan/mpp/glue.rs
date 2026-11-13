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

//! High-level glue between PostgreSQL parallel-query callbacks and the
//! leader/worker MPP architecture over DSM `AsyncRead`/`AsyncWrite` SPSC rings.
//!
//! The leader-driven launch (`mpp::launch`) calls into this module; the shared-memory layout and
//! SPSC byte rings live in [`super::dsm_stream`] and are reached through these entry points:
//!
//! - [`mpp_is_active`]: gate for the customscan path-builder.
//! - [`estimate_dsm_size`]: mesh-region sizing for the plan-first launch.
//! - [`leader_setup`]: leader-side mesh init, called by `launch_mpp` after the workers spawn.
//!   Returns the leader's [`MppLeaderState`], which carries the [`DsmGrpcChannelResolver`] the
//!   customscan installs on its DataFusion `SessionContext`.
//! - [`worker_setup`]: worker-side mesh attach, called from `run_launched_worker`. Returns the
//!   [`WorkerSession`] carrying the DSM base pointer, owning `dsm_segment`, and `ServerProc`.

use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use arrow_array::RecordBatch;
use datafusion::physical_plan::{ExecutionPlan, SendableRecordBatchStream};
use datafusion_distributed::{DistributedExec, NetworkBoundaryExt};
use pgrx::pg_sys;

use crate::parallel_worker::builder::ParallelProcessFinish;
use crate::postgres::customscan::mpp::dsm_stream::{
    DsmLeaderShutdownHandle, DsmWorkerDetachHandle, ServerProc, assign_wakeup_slots,
    build_leader_streams, init_dsm_mesh,
};
use crate::postgres::customscan::mpp::interrupt::{PgTokioRuntime, cancel_pending};
use crate::postgres::customscan::mpp::launch::MppLifecycle;
use crate::postgres::customscan::mpp::wakeup::{WakeupDispatcher, WakeupSlotReservation};
use crate::postgres::customscan::mpp::worker::DsmGrpcChannelResolver;

pub use ::scan::postgres::customscan::mpp::glue::*;

/// Leader session handle holding the gRPC channel resolver, DSM shutdown handle, and wakeup slot
/// reservation.
pub struct LeaderSession {
    pub resolver: Arc<DsmGrpcChannelResolver>,
    pub shutdown_handle: DsmLeaderShutdownHandle,
    pub wakeup_reservation: Arc<WakeupSlotReservation>,
    shutdown_signaled: AtomicBool,
}

impl LeaderSession {
    /// Signal workers to shut down their Tonic servers and close all leader-side DSM streams.
    /// Idempotent across `shutdown_custom_scan` and `end_custom_scan`.
    pub fn shutdown(&self) {
        self.shutdown_signaled.store(true, Ordering::Release);
        self.shutdown_handle.signal_shutdown();
        self.resolver.close_all();
    }

    pub fn is_shutdown_signaled(&self) -> bool {
        self.shutdown_signaled.load(Ordering::Acquire)
    }
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because `shutdown()` only flips atomic
// ring flags and detaches the `WakeupDispatcher` from the reusable `PgTokioRuntime` without
// calling into Postgres FFI, and must run when a subtransaction or query error drops the scan
// state without calling `shutdown_custom_scan` or `end_custom_scan`.
impl Drop for LeaderSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Worker session handle pointing to the initialized DSM mesh, its owning `dsm_segment`, and this
/// worker's `ServerProc`.
pub struct WorkerSession {
    pub dsm_base: *mut u8,
    pub dsm_seg: *mut pg_sys::dsm_segment,
    pub worker_proc: ServerProc,
}

/// Returned to the leader from [`leader_setup`]. The customscan stashes this on its execution
/// state and consults it during `exec_custom_scan`.
pub struct MppLeaderState {
    /// Active leader session handle holding the gRPC channel resolver and DSM shutdown handle.
    pub session: LeaderSession,
    /// The builder handle owning the launched producer workers. The leader controls the launch, so
    /// it owns the teardown too: `end_custom_scan` takes this and calls `wait_for_finish` to join
    /// the workers and destroy the parallel context. `None` until `launch` installs it on the
    /// success path.
    pub finish: Option<crate::parallel_worker::builder::ParallelProcessFinish>,
    /// Per-phase launch timing the leader fills in as it commits, for `EXPLAIN ANALYZE`.
    pub timing: MppLaunchTiming,
}

/// Initialize the leader's DSM SPSC ring mesh using a pre-reserved [`WakeupSlotReservation`] and
/// build its [`MppLeaderState`]. Called by [`crate::postgres::customscan::mpp::launch`] on a
/// builder-allocated region.
///
/// # Safety
/// - `coordinate` must point to a DSM byte region of at least `estimate_dsm_size(n_procs)` bytes.
/// - `n_procs` is the total mesh participant count including the leader: 1 (leader, proc 0)
///   plus the launched producer workers. It must not exceed the count passed to
///   [`estimate_dsm_size`] for this region. A short launch may use fewer processes than the
///   preallocated region was sized for.
pub unsafe fn leader_setup(
    coordinate: *mut c_void,
    n_procs: u32,
    wakeup_reservation: WakeupSlotReservation,
) -> Result<MppLeaderState, String> {
    // `mpp_trace` reads a pgrx GucSetting, which requires the backend thread. Safe here
    // because this runs synchronously on the leader backend from `launch_mpp`.
    let t_setup = crate::gucs::mpp_trace().then(std::time::Instant::now);
    let base = coordinate as *mut u8;
    let slots = wakeup_reservation.slots();
    let Some(&leader_slot) = slots.first() else {
        return Err("mpp: wakeup reservation is empty".into());
    };
    unsafe {
        init_dsm_mesh(base, n_procs, spsc_ring_capacity(n_procs)?)?;
        assign_wakeup_slots(base, &slots)?;
    }
    let dispatcher = WakeupDispatcher::new(leader_slot);
    let (streams, shutdown_handle) = unsafe { build_leader_streams(base, &dispatcher)? };
    let resolver = Arc::new(DsmGrpcChannelResolver::new(
        (n_procs - 1) as usize,
        dispatcher,
        streams,
    ));
    if let Some(t) = t_setup {
        pgrx::warning!(
            "mpp trace: leader_setup (ring create + self attach) took {:.3} ms",
            t.elapsed().as_secs_f64() * 1000.0
        );
    }
    Ok(MppLeaderState {
        session: LeaderSession {
            resolver,
            shutdown_handle,
            wakeup_reservation: Arc::new(wakeup_reservation),
            shutdown_signaled: AtomicBool::new(false),
        },
        finish: None,
        timing: MppLaunchTiming::default(),
    })
}

struct LeaderDsmDetachArg {
    dsm_detached: Arc<AtomicBool>,
    wakeup_reservation: Arc<WakeupSlotReservation>,
}

#[pgrx::pg_guard]
unsafe extern "C-unwind" fn mark_dsm_detached_on_dsm_detach(
    _seg: *mut pg_sys::dsm_segment,
    arg: pg_sys::Datum,
) {
    let detach_arg = unsafe { Box::from_raw(arg.cast_mut_ptr::<LeaderDsmDetachArg>()) };
    detach_arg.dsm_detached.store(true, Ordering::SeqCst);
    detach_arg.wakeup_reservation.release();
}

impl MppLeaderState {
    /// Register an `on_dsm_detach` callback that marks all leader DSM streams detached and
    /// releases the query's reserved wakeup slots before the DSM segment unmaps.
    ///
    /// # Safety
    /// `seg` must be the live DSM segment that owns the ring memory.
    pub unsafe fn register_streams_on_dsm_detach(&self, seg: *mut pg_sys::dsm_segment) {
        let detach_arg = Box::into_raw(Box::new(LeaderDsmDetachArg {
            dsm_detached: Arc::clone(self.session.shutdown_handle.detached_flag()),
            wakeup_reservation: Arc::clone(&self.session.wakeup_reservation),
        }));
        unsafe {
            pg_sys::on_dsm_detach(
                seg,
                Some(mark_dsm_detached_on_dsm_detach),
                pg_sys::Datum::from(detach_arg as *mut c_void),
            );
        }
    }
}

#[pgrx::pg_guard]
unsafe extern "C-unwind" fn detach_worker_streams_on_dsm_detach(
    _seg: *mut pg_sys::dsm_segment,
    arg: pg_sys::Datum,
) {
    let handle = unsafe { Box::from_raw(arg.cast_mut_ptr::<DsmWorkerDetachHandle>()) };
    handle.on_detach();
}

/// Register an `on_dsm_detach` callback on `seg` (when non-null) that closes all SPSC rings in the
/// mesh owned by this worker, wakes any sleeping peers, and marks the worker's DSM streams detached
/// immediately before the DSM segment unmaps.
///
/// # Safety
/// If `seg` is non-null, it must be the live DSM segment that owns the ring memory referenced by
/// `handle`.
pub unsafe fn register_worker_streams_on_dsm_detach(
    seg: *mut pg_sys::dsm_segment,
    handle: DsmWorkerDetachHandle,
) {
    if seg.is_null() {
        return;
    }
    let handle_ptr = Box::into_raw(Box::new(handle));
    unsafe {
        pg_sys::on_dsm_detach(
            seg,
            Some(detach_worker_streams_on_dsm_detach),
            pg_sys::Datum::from(handle_ptr as *mut c_void),
        );
    }
}

/// Body of worker attach in `launch::run_launched_worker`.
///
/// # Safety
/// `coordinate` must be the DSM region pointer PG supplied, and `dsm_seg` must be the owning
/// `dsm_segment` (or null in standalone tests).
pub unsafe fn worker_setup(
    coordinate: *mut c_void,
    dsm_seg: *mut pg_sys::dsm_segment,
    worker_number: i32,
) -> Result<WorkerSession, String> {
    if worker_number < 0 {
        return Err("mpp: worker_number < 0".into());
    }
    // Leader is `proc_idx = 0`, workers are `1..n_procs`. Worker N maps from PG's
    // `ParallelWorkerNumber = N` to `proc_idx = N + 1`.
    let proc_idx = (worker_number as u32) + 1;
    Ok(WorkerSession {
        dsm_base: coordinate as *mut u8,
        dsm_seg,
        worker_proc: ServerProc(proc_idx),
    })
}

/// One producer stage discovered in a distributed physical plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StageEntry {
    pub task_count: usize,
}

/// Enumerate every producer stage in `root`.
pub(crate) fn collect_stages(root: &Arc<dyn ExecutionPlan>) -> Vec<StageEntry> {
    let mut out = Vec::new();
    walk_stages(root, &mut out);
    out
}

fn walk_stages(plan: &Arc<dyn ExecutionPlan>, out: &mut Vec<StageEntry>) {
    if let Some(nb) = plan.as_ref().as_network_boundary() {
        let stage = nb.input_stage();
        if let Some(stage_plan) = stage.local_plan() {
            out.push(StageEntry {
                task_count: stage.task_count(),
            });
            walk_stages(stage_plan, out);
        }
        return;
    }
    for child in plan.children() {
        walk_stages(child, out);
    }
}

/// Largest producer-stage task count, or 0 when the plan has no network boundaries (#5667).
pub(crate) fn max_producer_task_count(stages: &[StageEntry]) -> usize {
    stages
        .iter()
        .map(|stage| stage.task_count)
        .max()
        .unwrap_or(0)
}

/// True when discovered stages have enough producer tasks (`>= 2`) for MPP to launch (#5784).
pub(crate) fn stages_have_data_parallelism(stages: &[StageEntry]) -> bool {
    max_producer_task_count(stages) >= 2
}

/// Convenience wrapper over [`stages_have_data_parallelism`] for a finished physical plan.
pub(crate) fn mpp_plan_has_data_parallelism(plan: &Arc<dyn ExecutionPlan>) -> bool {
    stages_have_data_parallelism(&collect_stages(plan))
}

/// Wait briefly for worker `TaskMetrics` to arrive over `coordinator_channel` (after the leader
/// stream has been dropped, triggering `EndStreamNotifier`), then signal all workers to shut down
/// their Tonic servers and close all leader DSM streams.
pub fn finish_leader_execution(
    runtime: Option<&PgTokioRuntime>,
    plan: Option<&Arc<dyn ExecutionPlan>>,
    leader: &MppLeaderState,
) {
    if !leader.session.is_shutdown_signaled()
        && let (Some(rt), Some(plan)) = (runtime, plan)
        && let Some(dist) = plan.downcast_ref::<DistributedExec>()
        && !cancel_pending()
        && rt.is_running()
    {
        let _ = rt.block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_millis(250),
                dist.wait_for_metrics(),
            )
            .await
        });
    }
    leader.session.shutdown();
    if let Some(rt) = runtime {
        if cancel_pending() {
            rt.shutdown_workers();
        } else {
            rt.quiesce_tasks();
            if leader.session.shutdown_handle.active_streams() > 0 {
                let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
                while leader.session.shutdown_handle.active_streams() > 0
                    && std::time::Instant::now() < deadline
                {
                    std::thread::sleep(std::time::Duration::from_micros(50));
                }
                if leader.session.shutdown_handle.active_streams() > 0 {
                    rt.shutdown_workers();
                }
            }
        }
    }
    debug_assert_eq!(
        leader.session.shutdown_handle.active_streams(),
        0,
        "all leader DsmAsyncStreams must be dropped before DSM detach"
    );
}

/// Whether any backend (leader or a worker) spilled a DataFusion operator to disk this query.
/// Reads the `ParallelScanState` slot the leader populated at launch.
fn mpp_did_spill(finish: &ParallelProcessFinish) -> bool {
    match finish
        .state_manager()
        .slice_mut::<u8>(crate::postgres::customscan::mpp::launch::SCAN_IDX)
    {
        Ok(Some(s)) => unsafe {
            (*(s.as_mut_ptr() as *mut crate::postgres::ParallelScanState)).did_spill()
        },
        _ => {
            // Structurally shouldn't happen: both launch-time readers of this same slot
            // (`launch.rs`) treat a missing/wrong-typed region as fatal. Debug-only here
            // because this runs at shutdown, after the query's results already exist;
            // erroring out of teardown for an advisory warning is worse than just not
            // warning. Release builds fall through to `false`: a genuine DSM bug would
            // surface elsewhere (or in debug/regress builds here) rather than only here.
            debug_assert!(false, "mpp: parallel scan state missing at shutdown");
            false
        }
    }
}

/// Folds the workers' DSM spill flag into the leader's own flag. Call it before the
/// leader finishes (and so destroys) a launch whose flag `warn_if_spilled` will not see,
/// such as a relaunch on rescan.
fn record_worker_spill(spilled: &AtomicBool, finish: &ParallelProcessFinish) {
    if mpp_did_spill(finish) {
        spilled.store(true, Ordering::Relaxed);
    }
}

/// Emits the spill warning, once per scan node per `ExecutorRun`. `spilled_locally` is
/// the leader's own flag, sticky across rescans; the worker flag lives in the DSM behind
/// `finish`, so this must run after the workers are joined. Before the join a worker can
/// still be executing (an early-terminated `LIMIT` query, for one) and spill after the
/// load, and the join is also what orders a worker's relaxed store before this load.
fn warn_if_spilled(spilled_locally: &AtomicBool, finish: Option<&ParallelProcessFinish>) {
    if spilled_locally.load(Ordering::Relaxed) || finish.is_some_and(mpp_did_spill) {
        pgrx::warning!(
            "query exceeded work_mem and spilled to disk; consider raising work_mem \
             for better performance (paradedb.spill_to_disk allowed it to complete \
             instead of erroring)"
        );
    }
}

/// Shared `ShutdownCustomScan` step for `JoinScan` and `AggregateScan`:
/// drops `stream` (triggering `EndStreamNotifier` on `DistributedExec` so `c2w_stream` closes and
/// workers flush their final `TaskMetrics`), shuts down the leader session or quiesces serial
/// runtime tasks, joins the producer workers so their metrics land before `EXPLAIN ANALYZE`
/// renders, and emits the spill warning if any operator spilled.
pub fn shutdown_mpp_scan(
    stream: &mut Option<SendableRecordBatchStream>,
    runtime: Option<&PgTokioRuntime>,
    physical_plan: Option<&Arc<dyn ExecutionPlan>>,
    mpp: &mut MppLifecycle,
    spilled: &AtomicBool,
) {
    *stream = None;
    if let Some(leader) = mpp.leader() {
        finish_leader_execution(runtime, physical_plan, leader);
    } else if let Some(rt) = runtime {
        rt.quiesce_tasks();
    }
    if let Some(leader) = mpp.leader_mut()
        && let Some(finish) = leader.finish.as_mut()
    {
        let _ = finish.recv();
    }
    warn_if_spilled(
        spilled,
        mpp.leader().and_then(|leader| leader.finish.as_ref()),
    );
}

/// Shared `EndCustomScan` and `ReScanCustomScan` teardown for `JoinScan` and `AggregateScan`:
/// drops `stream`, shuts down the leader session (or quiesces serial tasks), takes the leader out
/// of `mpp`, clears `current_batch`, `physical_plan`, and `runtime` before the DSM is destroyed,
/// and joins and destroys the parallel worker context while folding any worker spill flag into
/// `spilled`.
pub fn teardown_mpp_scan(
    stream: &mut Option<SendableRecordBatchStream>,
    current_batch: &mut Option<RecordBatch>,
    physical_plan: &mut Option<Arc<dyn ExecutionPlan>>,
    runtime: &mut Option<PgTokioRuntime>,
    mpp: &mut MppLifecycle,
    spilled: &AtomicBool,
) {
    *stream = None;
    if let Some(leader) = mpp.leader() {
        finish_leader_execution(runtime.as_ref(), physical_plan.as_ref(), leader);
    } else if let Some(rt) = runtime.as_ref() {
        rt.quiesce_tasks();
    }
    let finish = mpp
        .take_leader()
        .and_then(|mut leader| leader.finish.take());
    // Drop order: the physical plan and session hold DSM stream references, so both must go before
    // the DSM is destroyed below, and the stream/plan must go before the runtime.
    *current_batch = None;
    *physical_plan = None;
    *runtime = None;
    if let Some(mut finish) = finish {
        // Join first: a producer can still spill while it winds down, and the join is what orders
        // its store before this load. `recv` is idempotent, so the one inside `wait_for_finish`
        // returns at once.
        let _ = finish.recv();
        record_worker_spill(spilled, &finish);
        finish.wait_for_finish();
    }
}

/// Rewrite the executed plan with the worker metrics collected in `DistributedExec.metrics_store`.
/// Mesh-free, so it is safe at EXPLAIN-render time, after the DSM is gone.
pub fn merge_worker_metrics(plan: &Arc<dyn ExecutionPlan>) -> Option<Arc<dyn ExecutionPlan>> {
    plan.downcast_ref::<DistributedExec>()?;
    let runtime = PgTokioRuntime::new().ok()?;
    runtime
        .block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_millis(250),
                datafusion_distributed::rewrite_distributed_plan_with_metrics(
                    Arc::clone(plan),
                    datafusion_distributed::DistributedMetricsFormat::PerTask,
                ),
            )
            .await
        })
        .ok()?
        .ok()
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::postgres::customscan::mpp::dsm_stream::dsm_region_bytes;
    use crate::postgres::customscan::mpp::wakeup::reserve_wakeup_slots;
    use pgrx::pg_test;

    unsafe fn leader_setup_for_test(
        coordinate: *mut c_void,
        n_procs: u32,
    ) -> Result<MppLeaderState, String> {
        let reservation = reserve_wakeup_slots(n_procs as usize).ok_or_else(|| {
            format!("mpp: failed to reserve {n_procs} wakeup slots from pre-allocated pool")
        })?;
        unsafe { leader_setup(coordinate, n_procs, reservation) }
    }

    #[pg_test]
    fn mesh_can_use_fewer_processes_than_its_reserved_region() {
        // `launch_mpp` must allocate DSM before PostgreSQL tells it how many workers actually
        // attached. Pin the transport contract that lets a short launch initialize its smaller
        // logical mesh in the region reserved for the requested width, including across
        // power-of-two rounding boundaries (`(4, 3)` and `(6, 5)`).
        for (requested_procs, launched_procs) in [(6, 4), (6, 5), (4, 3)] {
            let region_bytes = estimate_dsm_size(requested_procs).unwrap();
            assert!(
                region_bytes
                    >= dsm_region_bytes(
                        launched_procs,
                        spsc_ring_capacity(launched_procs).unwrap()
                    )
                    .unwrap()
            );
            let mut region = vec![0_u64; region_bytes.div_ceil(std::mem::size_of::<u64>())];

            let state = unsafe {
                leader_setup_for_test(region.as_mut_ptr().cast(), launched_procs).unwrap()
            };
            assert_eq!(
                state.session.resolver.n_workers(),
                (launched_procs - 1) as usize
            );
        }
    }

    #[pg_test]
    fn dsm_detach_marks_handle_detached() {
        let region_bytes = estimate_dsm_size(3).unwrap();
        let mut region = vec![0_u64; region_bytes.div_ceil(std::mem::size_of::<u64>())];
        let state = unsafe { leader_setup_for_test(region.as_mut_ptr().cast(), 3).unwrap() };
        let detach_arg = Box::into_raw(Box::new(LeaderDsmDetachArg {
            dsm_detached: Arc::clone(state.session.shutdown_handle.detached_flag()),
            wakeup_reservation: Arc::clone(&state.session.wakeup_reservation),
        }));
        unsafe {
            mark_dsm_detached_on_dsm_detach(
                std::ptr::null_mut(),
                pg_sys::Datum::from(detach_arg as *mut c_void),
            );
        }
        assert!(
            state
                .session
                .shutdown_handle
                .detached_flag()
                .load(Ordering::Acquire)
        );
        assert!(state.session.wakeup_reservation.slots().is_empty());
    }

    #[test]
    fn boundary_free_plan_yields_no_stages() {
        use datafusion::arrow::datatypes::{DataType, Field, Schema};
        use datafusion::physical_plan::empty::EmptyExec;

        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        let plan: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(schema));
        let out = collect_stages(&plan);
        assert!(out.is_empty());
        assert_eq!(max_producer_task_count(&out), 0);
        assert!(!mpp_plan_has_data_parallelism(&plan));
    }
}
