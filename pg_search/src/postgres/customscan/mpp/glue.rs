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

use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::{DistributedExec, NetworkBoundaryExt, TaskKey};
use datafusion_proto::physical_plan::DefaultPhysicalProtoConverter;
use pgrx::pg_sys;

use crate::parallel_worker::builder::ParallelProcessFinish;
use crate::postgres::customscan::mpp::dsm_stream::{
    DsmLeaderShutdownHandle, MIN_SPSC_RING_CAPACITY, ServerProc, build_leader_streams,
    dsm_region_bytes, init_dsm_mesh,
};
use crate::postgres::customscan::mpp::interrupt::{PgTokioRuntime, cancel_pending};
use crate::postgres::customscan::mpp::wakeup::{WakeupDispatcher, self_wakeup_slot};
use crate::postgres::customscan::mpp::worker::DsmGrpcChannelResolver;

/// Minimum total procs for MPP: leader (consumer-only, proc 0) plus at least 2 producers.
pub const MIN_TOTAL_WORKER_COUNT: u32 = 3;

/// True iff PostgreSQL's parallelism budget allows at least `MIN_TOTAL_WORKER_COUNT - 1`
/// producer workers.
pub fn mpp_is_active() -> bool {
    producer_worker_cap() >= MIN_TOTAL_WORKER_COUNT - 1
}

/// Whether PostgreSQL judged the entire statement safe to execute while parallel mode is active.
pub fn query_allows_parallel_mode(root: &pg_sys::PlannerInfo) -> bool {
    !root.glob.is_null() && unsafe { (*root.glob).parallelModeOK }
}

const _: () = assert!(pg_sys::MAXIMUM_ALIGNOF == 8);

/// Per-SPSC-ring byte capacity derived from `paradedb.mpp_queue_size` (default `8 MiB` -> `1 MiB`
/// per ring, giving `24 MiB` total at `n_procs = 4`).
fn spsc_ring_capacity() -> usize {
    let raw = (crate::gucs::mpp_queue_size() / 8).max(MIN_SPSC_RING_CAPACITY);
    1usize << raw.ilog2()
}

/// Total DSM bytes the leader will need for the DSM SPSC ring mesh (`2 * W^2` rings).
pub fn estimate_dsm_size(n_procs: u32) -> Result<usize, String> {
    dsm_region_bytes(n_procs, spsc_ring_capacity())
}

/// Maximum number of producer workers a launch may spawn:
/// `min(max_parallel_workers_per_gather, max_parallel_workers)` (#5667).
pub fn producer_worker_cap() -> u32 {
    let max_per_gather = unsafe { pg_sys::max_parallel_workers_per_gather };
    let max_workers = unsafe { pg_sys::max_parallel_workers };
    max_per_gather.min(max_workers).max(0) as u32
}

/// Leader-observable per-phase timing of an MPP launch, in microseconds.
#[derive(Default, Clone, Copy, Debug)]
pub struct MppLaunchTiming {
    pub prepare_us: u64,
    pub plan_us: u64,
    pub payload_us: u64,
    pub attach_us: u64,
    pub leader_setup_us: u64,
    pub exec_us: u64,
    pub first_frame_us: u64,
    pub workers: u32,
}

impl MppLaunchTiming {
    pub fn explain_text(&self) -> String {
        format!(
            "workers={} prepare={}us plan={}us payload={}us attach={}us \
             leader_setup={}us exec={}us first_frame={}us",
            self.workers,
            self.prepare_us,
            self.plan_us,
            self.payload_us,
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

/// Worker session handle pointing to the initialized DSM mesh and this worker's `ServerProc`.
pub struct WorkerSession {
    pub dsm_base: *mut u8,
    pub worker_proc: ServerProc,
}

/// Returned to the leader from [`leader_setup`].
pub struct MppLeaderState {
    pub session: LeaderSession,
    pub finish: Option<crate::parallel_worker::builder::ParallelProcessFinish>,
    pub timing: MppLaunchTiming,
}

/// Initialize the leader's DSM SPSC ring mesh and build its [`MppLeaderState`].
///
/// # Safety
/// - `coordinate` must point to a DSM byte region of at least `estimate_dsm_size(n_procs)` bytes.
/// - `n_procs` is `1 + launched_workers`.
pub unsafe fn leader_setup(
    coordinate: *mut c_void,
    n_procs: u32,
) -> Result<MppLeaderState, String> {
    let t_setup = crate::gucs::mpp_trace().then(std::time::Instant::now);
    let base = coordinate as *mut u8;
    unsafe {
        init_dsm_mesh(base, n_procs, spsc_ring_capacity())?;
    }
    let dispatcher = WakeupDispatcher::new(self_wakeup_slot());
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

/// Body of worker attach in `launch::run_launched_worker`.
///
/// # Safety
/// `coordinate` must be the DSM region pointer PG supplied.
pub unsafe fn worker_setup(
    coordinate: *mut c_void,
    worker_number: i32,
) -> Result<WorkerSession, String> {
    if worker_number < 0 {
        return Err("mpp: worker_number < 0".into());
    }
    let proc_idx = (worker_number as u32) + 1;
    Ok(WorkerSession {
        dsm_base: coordinate as *mut u8,
        worker_proc: ServerProc(proc_idx),
    })
}

/// Serializes each stage subplan the coordinator dispatches, with the pg codec
/// (`serialize_physical_plan`).
///
/// TODO: Delegate `try_encode_udf` / `try_decode_udf` / `try_encode_udaf` / `try_decode_udaf` in
/// `datafusion-distributed`'s `DistributedCodec` to the inner user `PhysicalExtensionCodec`, and
/// strip `FilterPassthroughExec` before `DistributedPhysicalOptimizerRule`, so `DispatchPlanSource`
/// and `StagePlanDispatchSource` can be removed.
#[derive(Default)]
pub struct StagePlanDispatchSource {
    cache: std::sync::Mutex<std::collections::HashMap<(usize, usize), Vec<u8>>>,
}

impl datafusion_distributed::DispatchPlanSource for StagePlanDispatchSource {
    fn dispatch_plan_proto(
        &self,
        task: &TaskKey,
        specialized: &Arc<dyn ExecutionPlan>,
    ) -> Option<datafusion::common::Result<Vec<u8>>> {
        let stage_id = task.stage_id;
        let task_number = task.task_number;
        let key = (stage_id, task_number);
        if let Some(bytes) = self.cache.lock().unwrap().get(&key) {
            return Some(Ok(bytes.clone()));
        }
        let bytes = match crate::scan::physical_codec::serialize_physical_plan(
            Arc::clone(specialized),
            &DefaultPhysicalProtoConverter {},
        ) {
            Ok(bytes) => bytes,
            Err(e) => return Some(Err(e)),
        };
        self.cache.lock().unwrap().insert(key, bytes.clone());
        Some(Ok(bytes))
    }
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
}

/// Whether any backend (leader or a worker) spilled a DataFusion operator to disk this query.
fn mpp_did_spill(finish: &ParallelProcessFinish) -> bool {
    match finish
        .state_manager()
        .slice_mut::<u8>(crate::postgres::customscan::mpp::launch::SCAN_IDX)
    {
        Ok(Some(s)) => unsafe {
            (*(s.as_mut_ptr() as *mut crate::postgres::ParallelScanState)).did_spill()
        },
        _ => {
            debug_assert!(false, "mpp: parallel scan state missing at shutdown");
            false
        }
    }
}

/// Folds the workers' DSM spill flag into the leader's own flag.
pub fn record_worker_spill(spilled: &AtomicBool, finish: &ParallelProcessFinish) {
    if mpp_did_spill(finish) {
        spilled.store(true, Ordering::Relaxed);
    }
}

/// Emits the spill warning, once per scan node per `ExecutorRun`.
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
