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

use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::{DistributedExec, NetworkBoundaryExt};
use pgrx::pg_sys;

use crate::parallel_worker::builder::ParallelProcessFinish;
use crate::postgres::customscan::mpp::dsm_stream::{
    DsmLeaderShutdownHandle, DsmWorkerDetachHandle, MIN_SPSC_RING_CAPACITY, ServerProc,
    build_leader_streams, dsm_region_bytes, init_dsm_mesh,
};
use crate::postgres::customscan::mpp::interrupt::{PgTokioRuntime, cancel_pending};
use crate::postgres::customscan::mpp::wakeup::{WakeupDispatcher, self_wakeup_slot};
use crate::postgres::customscan::mpp::worker::DsmGrpcChannelResolver;

/// Minimum total procs for MPP: leader (consumer-only, proc 0) plus at least 2 producers.
/// Below that, `build_mpp_session_context` still clamps `target_partitions` to 2 and the mesh
/// wouldn't have a worker for the second partition. Single source of truth so [`mpp_is_active`]
/// and the launch's clamp floor don't drift on the threshold.
pub const MIN_TOTAL_WORKER_COUNT: u32 = 3;

/// True iff PostgreSQL's parallelism budget allows at least `MIN_TOTAL_WORKER_COUNT - 1`
/// producer workers. There is no MPP-specific worker GUC (#5667):
/// `max_parallel_workers_per_gather` caps the per-query width (the same knob that caps
/// ordinary parallel scans, matching core PG's `compute_parallel_worker` convention) and
/// `max_parallel_workers` is the cluster-wide pool. Setting
/// `max_parallel_workers_per_gather` below 2 (or 0, PG's own parallelism-disable convention)
/// disables MPP. Customscan path-builders gate `parallel_workers` on this.
pub fn mpp_is_active() -> bool {
    producer_worker_cap() >= MIN_TOTAL_WORKER_COUNT - 1
}

/// Whether PostgreSQL judged the entire statement safe to execute while parallel mode is active.
///
/// This must be captured from [`pg_sys::PlannerGlobal::parallelModeOK`], rather than inferred
/// from the local query level: a SELECT subplan below `ModifyTable` still has `CMD_SELECT`, while
/// its enclosing INSERT makes the complete statement unsafe for parallel mode. Entering parallel
/// mode for such a subplan prevents PostgreSQL from assigning the transaction ID needed by the
/// heap write.
pub fn query_allows_parallel_mode(root: &pg_sys::PlannerInfo) -> bool {
    !root.glob.is_null() && unsafe { (*root.glob).parallelModeOK }
}

// The shared-memory transport requires at least 8-byte alignment (its ring headers hold `u64`
// atomics, aligned to 64-byte cache lines inside the region). The builder's `shm_toc_allocate`
// hands out MAXALIGN-aligned blobs for the mesh region, so the two must agree or the headers
// would be misaligned, which is UB.
const _: () = assert!(pg_sys::MAXIMUM_ALIGNOF == 8);

/// Per-SPSC-ring byte capacity derived from `paradedb.mpp_queue_size` (default `8 MiB` -> `1 MiB`
/// per ring, giving `18 MiB` total at `n_procs = 4`).
fn spsc_ring_capacity() -> usize {
    let raw = (crate::gucs::mpp_queue_size() / 8).max(MIN_SPSC_RING_CAPACITY);
    1usize << raw.ilog2()
}

/// Total DSM bytes the leader will need for the DSM SPSC ring mesh (`2 * W^2` rings).
/// `n_procs` is the total proc count (leader + launched producers) the plan-first launch computed
/// from the physical plan: the mesh grows as `W^2`, so this is sized from the plan's needs, not
/// the GUC ceiling (#5667). A short launch may initialize a narrower logical mesh in this
/// allocation, but never a wider one.
pub fn estimate_dsm_size(n_procs: u32) -> Result<usize, String> {
    dsm_region_bytes(n_procs, spsc_ring_capacity())
}

/// Maximum number of producer workers a launch may spawn:
/// `min(max_parallel_workers_per_gather, max_parallel_workers)` (#5667). Also the planner's
/// `target_partitions` ceiling. The plan-first launch spawns
/// `clamp(max_stage_task_count, MIN_TOTAL_WORKER_COUNT - 1, this)` producers, so the cap only
/// bounds: the plan's task fragments decide the actual width. When [`mpp_is_active`] holds
/// this is `>= MIN_TOTAL_WORKER_COUNT - 1`.
pub fn producer_worker_cap() -> u32 {
    let max_per_gather = unsafe { pg_sys::max_parallel_workers_per_gather };
    let max_workers = unsafe { pg_sys::max_parallel_workers };
    max_per_gather.min(max_workers).max(0) as u32
}

/// Leader-observable per-phase timing of an MPP launch, in microseconds. Populated so
/// `EXPLAIN ANALYZE` can show where the launch floor goes. Zero means unmeasured. The
/// worker-side cost the leader can't see directly is captured indirectly: `attach_us` is the
/// spawn-plus-fork wait, and `first_frame_us` folds in the workers' plan decode and first scan.
#[derive(Default, Clone, Copy, Debug)]
pub struct MppLaunchTiming {
    /// DSM alloc and scan-state populate.
    pub prepare_us: u64,
    /// Leader physical planning.
    pub plan_us: u64,
    /// Wait for every worker to fork and attach to the DSM segment.
    pub attach_us: u64,
    /// Leader ring-mesh init (`leader_setup`).
    pub leader_setup_us: u64,
    /// `plan.execute` stream construction.
    pub exec_us: u64,
    /// First batch out (worker decode, first scan, and network hop to the leader).
    pub first_frame_us: u64,
    /// Producer workers that attached.
    pub workers: u32,
}

impl MppLaunchTiming {
    /// The `MPP Launch` line for `EXPLAIN ANALYZE`, shared by JoinScan and AggregateScan so
    /// the launched width is observable (and regress-assertable) on both paths.
    pub fn explain_text(&self) -> String {
        format!(
            "workers={} prepare={}us plan={}us attach={}us \
             leader_setup={}us exec={}us first_frame={}us",
            self.workers,
            self.prepare_us,
            self.plan_us,
            self.attach_us,
            self.leader_setup_us,
            self.exec_us,
            self.first_frame_us,
        )
    }
}

/// Leader session handle holding the gRPC channel resolver and DSM shutdown handle.
pub struct LeaderSession {
    pub resolver: Arc<DsmGrpcChannelResolver>,
    pub shutdown_handle: DsmLeaderShutdownHandle,
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

/// Initialize the leader's DSM SPSC ring mesh and build its [`MppLeaderState`]. Called by the
/// leader-driven [`crate::postgres::customscan::mpp::launch`] on a builder-allocated region.
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
) -> Result<MppLeaderState, String> {
    // `mpp_trace` reads a pgrx GucSetting, which requires the backend thread. Safe here
    // because this runs synchronously on the leader backend from `launch_mpp`.
    let t_setup = crate::gucs::mpp_trace().then(std::time::Instant::now);
    let base = coordinate as *mut u8;
    unsafe {
        init_dsm_mesh(base, n_procs, spsc_ring_capacity())?;
    }
    let slot = self_wakeup_slot().map_err(|e| format!("mpp: {e}"))?;
    let dispatcher = WakeupDispatcher::new(slot);
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
            shutdown_signaled: AtomicBool::new(false),
        },
        finish: None,
        timing: MppLaunchTiming::default(),
    })
}

#[pgrx::pg_guard]
unsafe extern "C-unwind" fn mark_dsm_detached_on_dsm_detach(
    _seg: *mut pg_sys::dsm_segment,
    arg: pg_sys::Datum,
) {
    let flag = unsafe { Arc::from_raw(arg.cast_mut_ptr::<AtomicBool>()) };
    flag.store(true, Ordering::SeqCst);
}

impl MppLeaderState {
    /// Register an `on_dsm_detach` callback that marks all leader DSM streams detached before
    /// the DSM segment unmaps, so any delayed stream drop cannot dereference unmapped memory.
    ///
    /// # Safety
    /// `seg` must be the live DSM segment that owns the ring memory.
    pub unsafe fn register_streams_on_dsm_detach(&self, seg: *mut pg_sys::dsm_segment) {
        let flag_ptr = Arc::into_raw(Arc::clone(self.session.shutdown_handle.detached_flag()));
        unsafe {
            pg_sys::on_dsm_detach(
                seg,
                Some(mark_dsm_detached_on_dsm_detach),
                pg_sys::Datum::from(flag_ptr as *mut c_void),
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
        }
    }
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
pub fn record_worker_spill(spilled: &AtomicBool, finish: &ParallelProcessFinish) {
    if mpp_did_spill(finish) {
        spilled.store(true, Ordering::Relaxed);
    }
}

/// Emits the spill warning, once per scan node per `ExecutorRun`. `spilled_locally` is
/// the leader's own flag, sticky across rescans; the worker flag lives in the DSM behind
/// `finish`, so this must run after the workers are joined. Before the join a worker can
/// still be executing (an early-terminated `LIMIT` query, for one) and spill after the
/// load, and the join is also what orders a worker's relaxed store before this load.
pub fn warn_if_spilled(spilled_locally: &AtomicBool, finish: Option<&ParallelProcessFinish>) {
    if spilled_locally.load(Ordering::Relaxed) || finish.is_some_and(mpp_did_spill) {
        pgrx::warning!(
            "query exceeded work_mem and spilled to disk; consider raising work_mem \
             for better performance (paradedb.spill_to_disk allowed it to complete \
             instead of erroring)"
        );
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
    use pgrx::pg_test;

    #[pg_test]
    fn mesh_can_use_fewer_processes_than_its_reserved_region() {
        // `launch_mpp` must allocate DSM before PostgreSQL tells it how many workers actually
        // attached. Pin the transport contract that lets a short launch initialize its smaller
        // logical mesh in the region reserved for the requested width.
        let requested_procs = 6;
        let launched_procs = 4;
        let region_bytes = estimate_dsm_size(requested_procs).unwrap();
        let mut region = vec![0_u64; region_bytes.div_ceil(std::mem::size_of::<u64>())];

        let state = unsafe { leader_setup(region.as_mut_ptr().cast(), launched_procs).unwrap() };
        assert_eq!(
            state.session.resolver.n_workers(),
            (launched_procs - 1) as usize
        );
    }

    #[pg_test]
    fn dsm_detach_marks_handle_detached() {
        let region_bytes = estimate_dsm_size(3).unwrap();
        let mut region = vec![0_u64; region_bytes.div_ceil(std::mem::size_of::<u64>())];
        let state = unsafe { leader_setup(region.as_mut_ptr().cast(), 3).unwrap() };
        let flag_ptr = Arc::into_raw(Arc::clone(state.session.shutdown_handle.detached_flag()));
        unsafe {
            mark_dsm_detached_on_dsm_detach(
                std::ptr::null_mut(),
                pg_sys::Datum::from(flag_ptr as *mut c_void),
            );
        }
        assert!(
            state
                .session
                .shutdown_handle
                .detached_flag()
                .load(Ordering::Acquire)
        );
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
