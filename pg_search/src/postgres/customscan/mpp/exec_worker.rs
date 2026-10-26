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

//! Shape-agnostic MPP worker exec dispatcher.
//!
//! The natural-shape MPP path is the same flow for every customscan that opts in: read the
//! leader's dispatch blob from DSM, decode this proc's per-stage physical subplans (the leader
//! built and sliced the plan once, so workers don't re-plan), and run each fragment via
//! [`datafusion_distributed::shm::run_worker_fragment`] + `FuturesUnordered`. The only
//! customscan-specific pieces are the seed `SessionContext` and where the inputs come from in per-scan state.
//!
//! This module isolates the shape-agnostic logic. Per-scan
//! `crate::postgres::customscan::mpp::host::MppWorkerHost` impls (in
//! `aggregatescan::mpp` and `joinscan::mpp`) extract their inputs into [`MppWorkerInputs`],
//! build their seed `SessionContext`, and are driven by
//! `crate::postgres::customscan::mpp::host::exec_mpp_worker`, which calls
//! [`run_mpp_worker`].

use std::sync::Arc;

use datafusion::execution::{SessionStateBuilder, TaskContext};
use datafusion::prelude::SessionContext;
use datafusion_distributed::{
    DistributedConfig, DistributedExt, DistributedTaskContext, SessionStateBuilderExt,
};
use futures::{FutureExt, StreamExt};
use pgrx::pg_sys;

use crate::index::mvcc::SegmentView;
use crate::postgres::customscan::datafusion::memory::{build_runtime_env, create_memory_pool};
use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::shm::{
    CooperativeDrainSet, InProcessWorkerResolver, MppMesh, MppSender, ShmChannelResolver,
    WorkerSession, collect_task_metrics, run_execute_task_loop, run_worker_fragment,
};
use datafusion_proto::physical_plan::DeduplicatingProtoConverter;
use tokio_util::sync::CancellationToken;

use crate::postgres::ParallelScanState;
use crate::postgres::customscan::mpp::dispatch::fragments_for_worker;
use crate::postgres::customscan::mpp::glue::{MppLeaderState, producer_worker_cap};
use crate::postgres::customscan::mpp::interrupt::{
    HeldInterrupts, cancel_pending, check_for_interrupts,
};
use crate::postgres::customscan::mpp::worker_fragments::FragmentAssignment;
use crate::postgres::utils::ExprContextGuard;
use crate::scan::execution_plan::{
    pg_search_scan_desired_task_count, pg_search_scan_scale_up_leaf_node,
};
use crate::scan::physical_codec::{
    PgSearchPhysicalExtensionCodec, deserialize_physical_plan_with_runtime,
};
use datafusion_distributed::shm::SetPlanFrame;

/// Bundle of inputs the worker dispatcher needs. Per-scan
/// `crate::postgres::customscan::mpp::host::MppWorkerHost` impls populate this from their
/// typed state and hand it to [`run_mpp_worker`].
pub(crate) struct MppWorkerInputs {
    /// The leader's `ParallelScanState`, used to claim the partitioning source's segment slice.
    /// Not optional: a dispatched fragment scanning without it would search every segment and
    /// duplicate rows across the mesh, so the entrypoint refuses to start without one. Valid
    /// for this worker's whole lifetime: PostgreSQL doesn't destroy a `ParallelContext`'s DSM
    /// until `WaitForParallelWorkersToFinish` returns, i.e. after every worker has exited.
    pub parallel_state: *mut ParallelScanState,
    /// Total number of sources in the plan. Used to size the codec's per-source segment-ID Vec.
    pub plan_sources_count: usize,
    /// Active worker session handle holding mesh, plan bytes, and outbound senders.
    pub session: WorkerSession,
}

/// Build the distributed session context. The leader runs this (mesh = None) to build and slice
/// the plan for dispatch, and again at exec (mesh = Some) for its consumer side; workers run it
/// (mesh = Some) to host the decoded fragments. Both procs must agree on stage shape, task
/// estimator chain, and target_partitions so the dispatched stage numbers line up with the
/// leader's consumer plan.
///
/// `seed` is the customscan's serial session context (`create_datafusion_session_context()`).
/// The function copies its config and layers the distributed-planner knobs on top.
pub(crate) fn build_mpp_session_context(
    seed: SessionContext,
    mesh: Option<Arc<MppMesh>>,
) -> SessionContext {
    // Workers are procs 1..n_procs; leader is proc 0. Producer count = n_procs - 1.
    // n_procs >= 3 always holds: for mesh = Some, the launch clamps the spawned width to
    // `MIN_TOTAL_WORKER_COUNT - 1` (2) producers; for mesh = None, callers gate on `mpp_is_active()`
    // which requires a cap of >= 2 producers.
    //
    // `mesh = None` is the EXPLAIN-time / plan-time path: the planner only needs `n_workers`
    // for stage sizing and target_partitions, so it plans against the cap
    // (`producer_worker_cap()`, from PG's parallelism GUCs). EXPLAIN never opens a
    // `WorkerConnection`, so we skip the transport install and the fork's default sits
    // unused. Callers that render plain EXPLAIN must still apply the launch gate
    // (`mpp_plan_has_data_parallelism`) and replan serially when the finished plan has
    // fewer than 2 producer tasks (#5784), matching execution. mesh = Some reads the
    // launched width from the mesh header, which the plan-first launch sized from the
    // plan's task counts (#5667).
    let local_partitions = crate::gucs::mpp_local_partitions();
    let n_workers = if local_partitions > 0 {
        local_partitions
    } else {
        match mesh.as_ref() {
            Some(m) => m.n_workers() as usize,
            None => producer_worker_cap() as usize,
        }
    };
    assert!(
        n_workers >= 2 || local_partitions > 0,
        "MPP session contexts require at least two producer workers"
    );
    // Three knobs that have to be set for the planner to actually emit `NetworkShuffleExec`:
    //   1. target_partitions(N): without it, EnforceDistribution skips every
    //      RepartitionExec so the annotator never sees a Shuffle.
    //   2. desired_task_count(N): without it, leaves default to Maximum(1) and
    //      `_distribute_plan` elides every shuffle.
    //   3. distributed_broadcast_joins(true): otherwise CollectLeft HashJoins cap their
    //      stage at Maximum(1) and propagate the cap upward, eliding shuffles above the join.
    let cfg = seed.copied_config().with_target_partitions(n_workers);

    // Start from the seed's existing state so the customscan's query planner
    // (`PgSearchQueryPlanner`), optimizer rules, and registered extensions all carry over.
    // JoinScan needs this for `VisibilityFilterNode` -> `VisibilityFilterExec` translation;
    // AggregateScan's plan doesn't use custom logical nodes but inheriting the planner is
    // still the right default. We then override `with_config` (bumps target_partitions)
    // and layer the distributed-planner knobs on top.
    //
    // Both seeds ship without a `DistributedConfig` extension. The bootstrap below would
    // clobber one if a future change started adding `with_distributed_*` calls on the
    // seed, so guard explicitly. Debug-only; release builds silently let the bootstrap
    // win, which would surface as missing distributed knobs at execute time and trip the
    // regress suite.
    debug_assert!(
        seed.state()
            .config()
            .options()
            .extensions
            .get::<DistributedConfig>()
            .is_none(),
        "build_mpp_session_context: seed already carries a DistributedConfig; the bootstrap \
         below would overwrite it"
    );
    let mut state_builder = SessionStateBuilder::new_from_existing(seed.state())
        .with_config(cfg)
        // Explicit `DistributedConfig` bootstrap so the downstream `with_distributed_*`
        // setters have something to mutate regardless of order; the transport setter is
        // optional (EXPLAIN passes mesh = None), so nothing else is guaranteed to run first.
        .with_distributed_option_extension(DistributedConfig::default())
        // Placeholder resolver. Our "workers" are PG parallel workers in the same backend tree,
        // not URL-addressed nodes, so the shm_mq transport routes by task index and never dials a
        // URL; the planner only needs `n_workers` of them to size stages.
        .with_distributed_worker_resolver(InProcessWorkerResolver::new(n_workers));
    // Install the shm_mq transport only for actual execution (mesh = Some). mesh = None is the
    // structure-only path: EXPLAIN, and the leader serializing the plan for dispatch. Neither
    // opens a WorkerConnection, so the fork's default transport sits unused.
    if let Some(mesh) = mesh {
        state_builder =
            state_builder.with_distributed_channel_resolver(ShmChannelResolver::new(mesh));
    }
    let state_builder = state_builder
        .with_distributed_user_codec(PgSearchPhysicalExtensionCodec::default())
        .with_distributed_desired_task_count_handler(pg_search_scan_desired_task_count)
        .with_distributed_scale_up_leaf_node_handler(pg_search_scan_scale_up_leaf_node)
        .with_distributed_desired_task_count_handler(n_workers)
        .with_distributed_broadcast_joins(true)
        .expect("with_distributed_broadcast_joins")
        // Disable the use of distributed dynamic filters until we fix
        // https://github.com/paradedb/datafusion-distributed/issues/107
        .with_distributed_remote_dynamic_filters(false)
        .expect("with_distributed_remote_dynamic_filters")
        .with_distributed_planner();
    SessionContext::new_with_state(state_builder.build())
}

/// Take one fragment's `SetPlan` frame off the mesh, draining this proc's inbox while waiting:
/// nothing else drains during the plan-wait phase, and the frame can't route itself.
async fn take_set_plan_draining(
    mesh: &Arc<MppMesh>,
    stage_id: u32,
    task: u32,
) -> Result<SetPlanFrame, datafusion::common::DataFusionError> {
    let take = mesh.take_set_plan(stage_id, task);
    futures::pin_mut!(take);
    loop {
        if let std::task::Poll::Ready(result) = futures::poll!(take.as_mut()) {
            return result;
        }
        // Nothing else polls the cancel/die flags during the plan wait.
        mesh.check_interrupt()?;
        mesh.try_drain_pass()?;
        tokio::task::yield_now().await;
    }
}

/// All fragment futures share this type. The alias keeps `Vec<FragmentFuture>` legible and silences
/// clippy::type_complexity.
type FragmentFuture<'a> = std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<(), datafusion::common::DataFusionError>>
            + Send
            + 'a,
    >,
>;

/// Extract an error message string from a `pgrx`-raised panic payload (`ErrorReportWithLevel` or
/// `CaughtError`), returning `None` if the payload is not a known `pgrx` type.
fn downcast_pgrx_panic_payload(payload: &(dyn std::any::Any + Send)) -> Option<String> {
    use pgrx::pg_sys::panic::{CaughtError, ErrorReportWithLevel};

    if let Some(report) = payload.downcast_ref::<ErrorReportWithLevel>() {
        return Some(report.message().to_string());
    }
    if let Some(caught) = payload.downcast_ref::<CaughtError>() {
        return Some(match caught {
            CaughtError::RustPanic { ereport, .. } => ereport.message().to_string(),
            CaughtError::PostgresError(report) | CaughtError::ErrorReport(report) => {
                report.message().to_string()
            }
        });
    }
    None
}

/// Spawns the execution task loop for a worker fragment over `run_execute_task_loop`.
fn spawn_fragment_execution_task<'a>(
    mesh: Arc<MppMesh>,
    outbound_senders: &'a [Option<MppSender>],
    fragment: &FragmentAssignment,
    plan: Arc<dyn ExecutionPlan>,
    task_ctx: Arc<TaskContext>,
    token: CancellationToken,
) -> FragmentFuture<'a> {
    let stage_id = fragment.stage_id;
    let task_idx = fragment.task_idx;
    let n_partitions = plan.properties().output_partitioning().partition_count();
    Box::pin(async move {
        run_execute_task_loop(
            &mesh,
            outbound_senders,
            stage_id,
            task_idx,
            n_partitions,
            token,
            move |_request, _headers, range, task_sinks| {
                let plan = Arc::clone(&plan);
                let task_ctx = Arc::clone(&task_ctx);
                async move {
                    let fut = std::panic::AssertUnwindSafe(run_worker_fragment(
                        plan, task_sinks, task_ctx, range,
                    ));
                    // `df-d`'s panic handler downcasts `&str` and `String` panics, but doesn't
                    // know about `pgrx` error types. Intercept `pgrx` panic types here to format
                    // them to a `DataFusionError::Execution`, and resume unwinding for everything else.
                    match fut.catch_unwind().await {
                        Ok(res) => res,
                        Err(payload) => {
                            if let Some(msg) = downcast_pgrx_panic_payload(&*payload) {
                                HeldInterrupts::reassert();
                                Err(datafusion::common::DataFusionError::Execution(msg))
                            } else {
                                std::panic::resume_unwind(payload);
                            }
                        }
                    }
                }
            },
        )
        .await
    })
}
/// Carries `*mut ParallelScanState` across `Send + Sync` futures on a single-threaded backend
/// runtime.
///
/// SAFETY: Postgres backends are single-threaded, and the DSM allocation backing `ParallelScanState`
/// remains mapped for the entire lifetime of the leader and worker fragment tasks (until
/// `WaitForParallelWorkersToFinish` in `finish_mpp_execution`).
#[derive(Clone, Copy)]
struct ParallelScanStatePtr(*mut ParallelScanState);

unsafe impl Send for ParallelScanStatePtr {}
unsafe impl Sync for ParallelScanStatePtr {}

impl ParallelScanStatePtr {
    fn as_ptr(self) -> *mut ParallelScanState {
        self.0
    }

    fn mark_spilled(self) {
        unsafe { (*self.0).mark_spilled() }
    }

    fn collect_metrics(self) -> bool {
        unsafe { (*self.0).collect_metrics() }
    }

    fn mark_query_done(self) {
        if !self.0.is_null() {
            unsafe { (*self.0).mark_query_done() }
        }
    }

    fn is_query_done(self) -> bool {
        !self.0.is_null() && unsafe { (*self.0).is_query_done() }
    }
}

struct PreparedMppFragments {
    worker_session: WorkerSession,
    fragments: Vec<FragmentAssignment>,
    session: SessionContext,
    parallel_state: ParallelScanStatePtr,
    plan_sources_count: usize,
    index_segment_views: Vec<SegmentView>,
    expr_context_guard: ExprContextGuard,
    work_mem_bytes: usize,
    hash_mem_multiplier: f64,
    mpp_debug: bool,
    token: CancellationToken,
}

fn prepare_mpp_fragments(
    worker_session: WorkerSession,
    seed_ctx: SessionContext,
    parallel_state: *mut ParallelScanState,
    plan_sources_count: usize,
    token: CancellationToken,
) -> Result<PreparedMppFragments, datafusion::common::DataFusionError> {
    let this_proc = worker_session.mesh.this_proc;
    let n_workers = worker_session.mesh.n_workers();

    let mut index_segment_views: Vec<SegmentView> =
        vec![SegmentView::default(); plan_sources_count];
    for (i, slot) in index_segment_views.iter_mut().enumerate() {
        *slot = unsafe { (*parallel_state).segment_view_for_source_unlocked(i) };
    }

    let expr_context_guard = ExprContextGuard::new();
    let (fragments, session) = fragments_for_worker(
        &worker_session.plan_bytes,
        seed_ctx,
        Arc::clone(&worker_session.mesh),
        this_proc,
        n_workers,
    )?;
    if fragments.is_empty() {
        // The launch spawns `producer_count <= max_tasks` producers, so the widest stage's
        // round-robin assigns every proc at least one fragment.
        return Err(datafusion::common::DataFusionError::Internal(format!(
            "no fragments assigned (proc={this_proc})"
        )));
    }

    let work_mem_bytes = unsafe { pg_sys::work_mem as usize * 1024 };
    let hash_mem_multiplier = unsafe { pg_sys::hash_mem_multiplier };
    let mpp_debug = crate::gucs::mpp_debug();

    Ok(PreparedMppFragments {
        worker_session,
        fragments,
        session,
        parallel_state: ParallelScanStatePtr(parallel_state),
        plan_sources_count,
        index_segment_views,
        expr_context_guard,
        work_mem_bytes,
        hash_mem_multiplier,
        mpp_debug,
        token,
    })
}

async fn run_mpp_fragments_async(
    prepared: PreparedMppFragments,
) -> Result<(), datafusion::common::DataFusionError> {
    let this_proc = prepared.worker_session.mesh.this_proc;
    let mesh_for_fail = Arc::clone(&prepared.worker_session.mesh);
    let res = run_mpp_fragments_inner(prepared).await;
    if let Err(ref e) = res
        && this_proc == 0
    {
        mesh_for_fail.fail_local_scope(&e.to_string());
    }
    res
}

async fn run_mpp_fragments_inner(
    prepared: PreparedMppFragments,
) -> Result<(), datafusion::common::DataFusionError> {
    let PreparedMppFragments {
        worker_session,
        fragments,
        session,
        parallel_state,
        plan_sources_count,
        index_segment_views,
        expr_context_guard,
        work_mem_bytes,
        hash_mem_multiplier,
        mpp_debug,
        token,
    } = prepared;
    let mesh = &worker_session.mesh;
    let outbound_senders = worker_session.outbound_senders();
    let this_proc = mesh.this_proc;

    // Each fragment's plan arrives as a `SetPlan` frame on this proc's inbox, the same
    // `SetPlanRequest` Flight ships. Collect the frames first (the take drains the inbox while
    // it waits), then decode synchronously.
    let mut frames: Vec<SetPlanFrame> = Vec::with_capacity(fragments.len());
    for fragment in &fragments {
        let frame = tokio::select! {
            res = take_set_plan_draining(mesh, fragment.stage_id, fragment.task_idx) => res?,
            _ = token.cancelled() => return Ok(()),
        };
        frames.push(frame);
    }

    // Deserialize under the decode ctx, not the run ctx. The run ctx limits
    // allocations aggressively; decode builds the plan graph and can spike memory.
    let decode_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let decode_ctx = session.task_ctx();
        let mut plans = Vec::with_capacity(frames.len());
        for (fragment, frame) in fragments.iter().zip(frames) {
            let Some(set_plan) = frame.set_plan else {
                return Err(datafusion::common::DataFusionError::Internal(format!(
                    "SetPlan frame without a request (stage_id={}, task_idx={})",
                    fragment.stage_id, fragment.task_idx
                )));
            };
            // Deduplicating decode re-shares expressions by `expression_id`, so occurrences of one
            // dynamic filter come back as a single shared instance instead of per-site snapshots.
            let plan = deserialize_physical_plan_with_runtime(
                &set_plan.plan_proto,
                &decode_ctx,
                Some(parallel_state.as_ptr()),
                index_segment_views.to_vec(),
                Some(expr_context_guard.as_ptr()),
                &DeduplicatingProtoConverter {},
            )
            .map_err(|e| {
                datafusion::common::DataFusionError::Execution(format!(
                    "decode dispatched plan failed (stage_id={}, task_idx={}): {e}",
                    fragment.stage_id, fragment.task_idx
                ))
            })?;
            plans.push(plan);
        }
        // `checkout_segment_for_source` answers a pool index past the DSM's table with `None`,
        // which a scan reads as "no segments left". Refuse the plan instead of scanning nothing.
        for plan in &plans {
            let mut bad_idx = None;
            crate::scan::execution_plan::visit_scan_nodes(plan, &mut |scan| {
                if let Some(idx) = scan.source_idx()
                    && idx >= plan_sources_count
                {
                    bad_idx = Some(idx);
                }
            });
            if let Some(idx) = bad_idx {
                return Err(datafusion::common::DataFusionError::Internal(format!(
                    "scan claims from segment pool {idx} but the DSM holds \
                     {plan_sources_count} pools (proc={this_proc})"
                )));
            }
        }
        Ok::<Vec<Arc<dyn ExecutionPlan>>, datafusion::common::DataFusionError>(plans)
    }));
    let plans = match decode_res {
        Ok(res) => res?,
        Err(payload) => {
            if let Some(msg) = downcast_pgrx_panic_payload(&*payload) {
                HeldInterrupts::reassert();
                return Err(datafusion::common::DataFusionError::Execution(msg));
            } else {
                std::panic::resume_unwind(payload);
            }
        }
    };

    let collect_metrics = parallel_state.collect_metrics();
    let session_arc = Arc::new(session);
    let mut futures: Vec<FragmentFuture<'_>> = Vec::with_capacity(fragments.len());
    let mut executed_fragments: Vec<(u32, u32, usize, Arc<dyn ExecutionPlan>)> = if collect_metrics
    {
        Vec::with_capacity(fragments.len())
    } else {
        Vec::new()
    };
    for (fragment, frag_plan) in fragments.iter().zip(plans) {
        // Build a TaskContext seeded with the right `DistributedTaskContext` so the boundary
        // nodes inside the fragment's plan know their `(task_index, task_count)`.
        let cfg = session_arc
            .state()
            .config()
            .clone()
            .with_extension(Arc::new(DistributedTaskContext {
                task_index: fragment.task_idx as usize,
                task_count: fragment.task_count,
            }));
        let memory_pool = create_memory_pool(&frag_plan, work_mem_bytes, hash_mem_multiplier);
        let spill_state = parallel_state;
        let task_ctx = Arc::new(
            TaskContext::from(session_arc.as_ref())
                .with_session_config(cfg)
                .with_runtime(build_runtime_env(
                    memory_pool,
                    Arc::new(move || spill_state.mark_spilled()),
                )),
        );

        // The fragment arrives ready-to-run: the leader serialized it with nested stages
        // already `Remote`, so its boundary leaves read the mesh through the session's
        // `ShmChannelResolver`.
        if collect_metrics {
            executed_fragments.push((
                fragment.stage_id,
                fragment.task_idx,
                fragment.task_count,
                Arc::clone(&frag_plan),
            ));
        }
        futures.push(spawn_fragment_execution_task(
            Arc::clone(mesh),
            outbound_senders,
            fragment,
            frag_plan,
            task_ctx,
            token.clone(),
        ));
    }

    crate::mpp_log!(
        "mpp worker dispatch this_proc={this_proc} starting fragment futures on {} fragments",
        fragments.len()
    );
    // Fail-fast via FuturesUnordered. Under the pull model, an error in any fragment must
    // surface immediately so that worker_session drops outbound senders and triggers
    // pgrx::error! (causing peers to observe Detached status and fail pending buffers).
    // Waiting for all fragments could cause a hang if a fragment blocks waiting for an RPC
    // request that will never arrive due to a sibling failure.
    //
    // Deadlock detector. Under `paradedb.mpp_debug`, if any fragment hasn't completed
    // within 30 s the dispatcher surfaces an error instead of letting the backend spin
    // forever.
    let mut frag_stream: futures::stream::FuturesUnordered<_> = futures.into_iter().collect();
    // A cancel or die that lands while a fragment is idle in its request wait reaches no
    // send spin and no consumer pull, the only places that poll the flags (a drain pass
    // only moves frames). The leader is already waiting for this worker to exit and
    // releases nothing before that, so the wait would never end on its own. Cancelling
    // the token unwinds the request loops.
    let interrupt_watch = {
        let token = token.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                if cancel_pending() || parallel_state.is_query_done() {
                    token.cancel();
                    return;
                }
            }
        })
    };
    let join_fut = async {
        while let Some(res) = frag_stream.next().await {
            res?;
        }
        Ok::<(), datafusion::common::DataFusionError>(())
    };
    let outcome: Result<(), datafusion::common::DataFusionError> = if mpp_debug {
        match tokio::time::timeout(std::time::Duration::from_secs(30), join_fut).await {
            Ok(res) => res,
            Err(_) => {
                crate::mpp_log!(
                    "mpp worker dispatch this_proc={this_proc} HANG: fragment execution exceeded 30s"
                );
                Err(datafusion::common::DataFusionError::Internal(format!(
                    "mpp worker dispatch (proc={this_proc}): fragment execution exceeded 30s; \
                     deadlock detector triggered"
                )))
            }
        }
    } else {
        join_fut.await
    };
    interrupt_watch.abort();
    drop(frag_stream);

    // Report each fragment's metrics to the leader, even after a fragment error: partial
    // metrics still tell the user where the time went. Best-effort like every transport's
    // metrics path; the bounded send drops the frame if the leader already went away.
    for (stage_id, task_idx, task_count, plan) in &executed_fragments {
        let frame = collect_task_metrics(plan, *task_idx as usize, *task_count);
        mesh.send_task_metrics_best_effort(outbound_senders, *stage_id, *task_idx, &frame)
            .await;
    }
    drop(expr_context_guard);
    outcome
}

impl MppLeaderState {
    /// Spawn the leader's own producer fragments (`proc 0`) onto `runtime`.
    pub fn spawn_leader_fragments(
        &mut self,
        seed_ctx: SessionContext,
        runtime: &tokio::runtime::Runtime,
    ) {
        let Some(worker_session) = self.session.worker.take() else {
            return;
        };
        let prepared = match prepare_mpp_fragments(
            worker_session,
            seed_ctx,
            self.parallel_state,
            self.plan_sources_count,
            self.leader_token.clone(),
        ) {
            Ok(p) => p,
            Err(e) => pgrx::error!("mpp leader: {e}"),
        };
        self.leader_task = Some(runtime.spawn(run_mpp_fragments_async(prepared)));
    }

    /// Drive the leader's producer fragment task (`proc 0`) to completion so its `TaskMetrics`
    /// land before `drain_worker_metrics` and its DSM senders drop before `wait_for_finish`.
    pub fn finish_leader_fragments(&mut self, runtime: Option<&tokio::runtime::Runtime>) {
        let parallel_state = ParallelScanStatePtr(self.parallel_state);
        let Some(mut task) = self.leader_task.take() else {
            parallel_state.mark_query_done();
            return;
        };
        let Some(runtime) = runtime else {
            parallel_state.mark_query_done();
            self.leader_token.cancel();
            task.abort();
            return;
        };
        let token = self.leader_token.clone();
        let mesh = Arc::clone(&self.session.mesh);
        let outcome = {
            let _held = HeldInterrupts::hold();
            runtime.block_on(async {
                // `Stage 0` has already dropped its output stream when this runs. Give in-flight
                // `Cancel` and `Eof` frames a brief grace window to drain naturally so fragments
                // finish cleanly and report metrics; once all remote producers have detached or
                // the grace deadline expires, mark `query_done` and cancel `leader_token` so any
                // unrequested upstream stages unwind instead of waiting for `ExecuteTaskFrame`s.
                let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(50);
                let mut cancelled = false;
                loop {
                    let _ = mesh.try_drain_pass();
                    tokio::select! {
                        biased;
                        join_res = &mut task => break join_res,
                        _ = tokio::time::sleep_until(deadline), if !cancelled => {
                            cancelled = true;
                            parallel_state.mark_query_done();
                            token.cancel();
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_millis(1)) => {}
                    }
                }
            })
        };
        parallel_state.mark_query_done();
        check_for_interrupts();
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                pgrx::error!("mpp leader: fragment dispatch failed: {e}");
            }
            Err(join_err) if join_err.is_panic() => {
                std::panic::resume_unwind(join_err.into_panic());
            }
            Err(_) => {}
        }
    }
}

/// Shape-agnostic body of `exec_mpp_worker`. Runs to completion on the caller's tokio runtime,
/// pgrx::error!s on fatal failures, returns normally on EOF (the customscan's
/// `exec_custom_scan` then returns `null_mut()` to signal end-of-stream to PG).
///
/// `seed_ctx` is a bare serial `SessionContext` used only for plan deserialization
/// (`ctx.task_ctx()`). The distributed planner config is built separately via
/// [`build_mpp_session_context`] over the same seed.
// TODO: Unwieldy. Once the dust settles on task multiplexing, should push more of this into `df-d`.
pub(crate) fn run_mpp_worker(
    inputs: MppWorkerInputs,
    seed_ctx: SessionContext,
    runtime: &tokio::runtime::Runtime,
) {
    let MppWorkerInputs {
        parallel_state,
        plan_sources_count,
        session: worker_session,
    } = inputs;

    let prepared = match prepare_mpp_fragments(
        worker_session,
        seed_ctx,
        parallel_state,
        plan_sources_count,
        CancellationToken::new(),
    ) {
        Ok(p) => p,
        Err(e) => pgrx::error!("mpp worker: {e}"),
    };

    // Hold cancel/die off for the duration so neither our drain/send loops nor a subroutine
    // (the scanner's own `CHECK_FOR_INTERRUPTS`, a buffer wait) can `proc_exit` out of the
    // live runtime. The loops poll cooperatively to bail promptly; see `mpp::interrupt`.
    let held = HeldInterrupts::hold();
    let outcome = runtime.block_on(run_mpp_fragments_async(prepared));
    // `block_on` has returned, so the runtime is idle and every fragment future (with its
    // DSM senders) has dropped. Resume interrupts, then service any cancel/die the loops
    // deferred, now on a stack with no live runtime; for a die this `proc_exit`s here instead
    // of mid-`block_on`.
    drop(held);
    check_for_interrupts();
    if let Err(e) = outcome {
        pgrx::error!("mpp worker: fragment dispatch failed: {e}");
    }
}
