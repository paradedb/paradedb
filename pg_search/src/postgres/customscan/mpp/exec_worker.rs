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
//! [`run_worker_fragment`] + `join_all`. The only customscan-specific pieces are the seed
//! `SessionContext` (different `SessionContextProfile`) and where the inputs come from in
//! per-scan state.
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
use pgrx::pg_sys;
use tantivy::index::SegmentId;

use crate::api::HashSet;
use crate::postgres::customscan::datafusion::memory::{build_runtime_env, create_memory_pool};
use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::shm::{
    collect_task_metrics, proc_for_task, run_worker_fragment, CooperativeDrainSet,
    InProcessWorkerResolver, MppFrameHeader, MppMesh, MppPartitionSink, MppSender,
    ShmChannelResolver,
};
use datafusion_distributed::PartitionSink;

use crate::postgres::customscan::mpp::dispatch::fragments_for_worker;
use crate::postgres::customscan::mpp::glue::producer_worker_count;
use crate::postgres::customscan::mpp::interrupt::{check_for_interrupts, HeldInterrupts};
use crate::postgres::customscan::mpp::task_estimator::PgSearchScanTaskEstimator;
use crate::postgres::customscan::mpp::worker_fragments::FragmentRouting;
use crate::postgres::utils::ExprContextGuard;
use crate::postgres::ParallelScanState;
use crate::scan::physical_codec::deserialize_physical_plan_with_runtime;
use datafusion_distributed::shm::SetPlanFrame;

/// Bundle of inputs the worker dispatcher needs. Per-scan
/// `crate::postgres::customscan::mpp::host::MppWorkerHost` impls populate this from their
/// typed state and hand it to [`run_mpp_worker`].
pub(crate) struct MppWorkerInputs {
    /// The leader's `ParallelScanState`, used to claim the partitioning source's segment slice.
    pub parallel_state: Option<*mut ParallelScanState>,
    /// Total number of sources in the plan. Used to size the codec's per-source segment-ID Vec.
    pub plan_sources_count: usize,
    /// Leader's dispatch payload (framed per-stage physical subplans), copied out of DSM during
    /// `worker_setup`.
    pub plan_bytes: Vec<u8>,
    /// This worker's `MppMesh` handle.
    pub worker_mesh: Arc<MppMesh>,
    /// This worker's outbound senders, keyed by destination `proc_idx`. The dispatcher takes
    /// ownership; consumers see `Detached` once these drop.
    pub outbound_senders: Vec<Option<MppSender>>,
}

/// Build the distributed session context. The leader runs this (mesh = None) to build and slice
/// the plan for dispatch, and again at exec (mesh = Some) for its consumer side; workers run it
/// (mesh = Some) to host the decoded fragments. Both procs must agree on stage shape, task
/// estimator chain, and target_partitions so the dispatched stage numbers line up with the
/// leader's consumer plan.
///
/// `seed` is the customscan's serial session context (`create_aggregate_session_context()` for
/// AggregateScan, `create_datafusion_session_context(SessionContextProfile::Join)` for JoinScan).
/// The function copies its config and layers the distributed-planner knobs on top.
pub(crate) fn build_mpp_session_context(
    seed: SessionContext,
    mesh: Option<Arc<MppMesh>>,
) -> SessionContext {
    // Workers are procs 1..n_procs; leader is proc 0. Producer count = n_procs - 1.
    // `mpp_is_active()` already guarantees n_procs >= 3 (callers gate first).
    //
    // `mesh = None` is the EXPLAIN-time path: the planner only needs `n_workers` for stage
    // sizing and target_partitions. EXPLAIN never opens a `WorkerConnection`, so we skip
    // the transport install and the fork's default sits unused. Both branches resolve to
    // `mpp_worker_count() - 1` at call time, so EXPLAIN reflects the GUC at the time it
    // ran; a later `SET paradedb.mpp_worker_count` shifts the next execute path.
    let n_workers = match mesh.as_ref() {
        Some(m) => m.n_workers() as usize,
        None => producer_worker_count() as usize,
    };
    // Three knobs that have to be set for the planner to actually emit `NetworkShuffleExec`:
    //   1. target_partitions(N): without it, EnforceDistribution skips every
    //      RepartitionExec so the annotator never sees a Shuffle.
    //   2. distributed_task_estimator(N): without it, leaves default to Maximum(1) and
    //      `_distribute_plan` elides every shuffle.
    //   3. distributed_broadcast_joins(true): otherwise CollectLeft HashJoins cap their
    //      stage at Maximum(1) and propagate the cap upward, eliding shuffles above the join.
    let cfg = seed
        .copied_config()
        .with_target_partitions(n_workers.max(2));

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
        .with_distributed_task_estimator(PgSearchScanTaskEstimator)
        .with_distributed_task_estimator(n_workers)
        .with_distributed_broadcast_joins(true)
        .expect("with_distributed_broadcast_joins")
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
        mesh.try_drain_pass()?;
        tokio::task::yield_now().await;
    }
}

/// Shape-agnostic body of `exec_mpp_worker`. Runs to completion on the caller's tokio runtime,
/// pgrx::error!s on fatal failures, returns normally on EOF (the customscan's
/// `exec_custom_scan` then returns `null_mut()` to signal end-of-stream to PG).
///
/// `seed_ctx` is a bare serial `SessionContext` used only for plan deserialization
/// (`ctx.task_ctx()`). The distributed planner config is built separately via
/// [`build_mpp_session_context`] over the same seed.
pub(crate) fn run_mpp_worker(
    inputs: MppWorkerInputs,
    seed_ctx: SessionContext,
    runtime: &tokio::runtime::Runtime,
) {
    let MppWorkerInputs {
        parallel_state,
        plan_sources_count,
        plan_bytes,
        worker_mesh,
        outbound_senders,
    } = inputs;

    let this_proc = worker_mesh.this_proc;

    // Build per-source canonical segment ID sets from the populated ParallelScanState.
    // Workers will then claim individual segments via `checkout_segment_for_source` inside their
    // `PgSearchTableProvider`.
    let mut index_segment_ids: Vec<HashSet<SegmentId>> =
        vec![HashSet::default(); plan_sources_count];
    if let Some(ps) = parallel_state {
        for (i, slot) in index_segment_ids.iter_mut().enumerate() {
            *slot = unsafe { (*ps).segment_ids_for_source_unlocked(i) };
        }
    }

    // Build this worker's fragment assignments by decoding the leader's dispatched per-stage
    // subplans from the DSM payload (no re-planning), then run them on the dispatcher loop below.
    // `worker_mesh.n_procs >= 3` is guaranteed by `mpp_is_active()` (callers gate before reaching
    // this), so `n_workers() = n_procs - 1` is safe.
    let n_workers = worker_mesh.n_workers();
    let (fragments, session) = match fragments_for_worker(
        &plan_bytes,
        seed_ctx,
        Arc::clone(&worker_mesh),
        this_proc,
        n_workers,
    ) {
        Ok(v) => v,
        Err(e) => pgrx::error!("mpp worker: build fragment assignments failed: {e}"),
    };
    if fragments.is_empty() {
        pgrx::warning!(
            "mpp worker (proc={this_proc}): no fragments assigned; skipping (worker emits zero rows)"
        );
        return;
    }

    // Each fragment's plan arrives as a `SetPlan` frame on this proc's inbox, the same
    // `SetPlanRequest` Flight ships. Collect the frames first (the take drains the inbox while
    // it waits), then decode synchronously: decode injects `parallel_state`, a raw pointer that
    // must stay off the produce futures.
    let frames: Vec<SetPlanFrame> = {
        let collected = runtime.block_on(async {
            let mut frames = Vec::with_capacity(fragments.len());
            for fragment in &fragments {
                frames.push(
                    take_set_plan_draining(
                        &worker_mesh,
                        fragment.stage_id,
                        fragment.task_idx as u32,
                    )
                    .await?,
                );
            }
            Ok::<_, datafusion::common::DataFusionError>(frames)
        });
        match collected {
            Ok(frames) => frames,
            Err(e) => pgrx::error!("mpp worker: plan frames did not arrive: {e}"),
        }
    };
    let decode_ctx = session.task_ctx();
    let mut plans = Vec::with_capacity(frames.len());
    let expr_context_guard = ExprContextGuard::new();

    // Deserialize under the decode ctx, not the run ctx. The run ctx limits
    // allocations aggressively; decode builds the plan graph and can spike memory.
    for (fragment, frame) in fragments.iter().zip(frames) {
        let Some(set_plan) = frame.set_plan else {
            pgrx::error!(
                "mpp worker: SetPlan frame without a request (stage_id={}, task_idx={})",
                fragment.stage_id,
                fragment.task_idx
            );
        };
        match deserialize_physical_plan_with_runtime(
            &set_plan.plan_proto,
            &decode_ctx,
            parallel_state,
            index_segment_ids.to_vec(),
            Some(expr_context_guard.as_ptr()),
        ) {
            Ok(plan) => plans.push(plan),
            Err(e) => pgrx::error!(
                "mpp worker: decode dispatched plan failed (stage_id={}, task_idx={}): {e}",
                fragment.stage_id,
                fragment.task_idx
            ),
        }
    }

    let work_mem_bytes = unsafe { pg_sys::work_mem as usize * 1024 };
    let hash_mem_multiplier = unsafe { pg_sys::hash_mem_multiplier };
    let session_arc = Arc::new(session);

    // All fragment futures share this vector. The alias keeps the `Vec<_>` declaration legible and silences
    // clippy::type_complexity.
    type FragmentFuture = std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(), datafusion::common::DataFusionError>>
                + Send,
        >,
    >;
    // Hold cancel/die off for the duration so neither our drain/send loops nor a subroutine
    // (the scanner's own `CHECK_FOR_INTERRUPTS`, a buffer wait) can `proc_exit` out of the
    // live runtime. The loops poll cooperatively to bail promptly; see `mpp::interrupt`.
    let held = HeldInterrupts::hold();
    let result = runtime.block_on(async move {
        let mut futures: Vec<FragmentFuture> = Vec::with_capacity(fragments.len());
        let mut executed_fragments: Vec<(u32, usize, usize, Arc<dyn ExecutionPlan>)> =
            Vec::with_capacity(fragments.len());
        for (fragment, frag_plan) in fragments.iter().zip(&plans) {
            let n_out = frag_plan
                .properties()
                .output_partitioning()
                .partition_count();
            // Build a `PartitionSink` per output partition. For each partition `q` emitted by this
            // fragment, look up the destination proc via `fragment.routing`, clone the right
            // outbound sender, and wrap it as a sink the produce loop pushes through.
            let mut per_partition_sinks: Vec<Box<dyn PartitionSink>> = Vec::with_capacity(n_out);
            for q in 0..n_out {
                let dest_proc = match &fragment.routing {
                    FragmentRouting::Coalesce { dest_proc } => *dest_proc,
                    // A partition outside the table means the decoded plan's partitioning
                    // drifted from the blob the leader routed; name it instead of panicking
                    // on the index.
                    FragmentRouting::Hashed { consumer_task, .. } => {
                        let Some(&task) = consumer_task.get(q) else {
                            return Err(datafusion::common::DataFusionError::Internal(format!(
                                "mpp worker dispatch: partition {q} outside routing table of len {} \
                                 (stage_id={} task_idx={})",
                                consumer_task.len(),
                                fragment.stage_id,
                                fragment.task_idx,
                            )));
                        };
                        proc_for_task(n_workers, task)
                    }
                };
                let base = match outbound_senders
                    .get(dest_proc as usize)
                    .and_then(|s| s.as_ref())
                {
                    Some(s) => s,
                    None => {
                        return Err(datafusion::common::DataFusionError::Internal(format!(
                            "mpp worker dispatch: outbound_senders[{dest_proc}] is None \
                             (self-loop or unattached); fragment stage_id={} task_idx={}",
                            fragment.stage_id, fragment.task_idx,
                        )));
                    }
                };
                let q_u32 = u32::try_from(q).unwrap_or(u32::MAX);
                crate::mpp_log!(
                    "mpp worker dispatch this_proc={this_proc} fragment(stage_id={}, \
                     task_idx={}) partition={q} → dest_proc={dest_proc}",
                    fragment.stage_id,
                    fragment.task_idx,
                );
                // Attach the worker mesh as the cooperative drain so a full outbound
                // ring doesn't block the backend thread. The spin pulls every inbound
                // drain while retrying the send, breaking the symmetric-send stall
                // pattern where every peer is blocked sending to a full peer.
                per_partition_sinks.push(Box::new(MppPartitionSink::new(
                    base.clone_with_header(MppFrameHeader::batch(
                        fragment.stage_id,
                        q_u32,
                        worker_mesh.this_proc,
                    ))
                    .with_cooperative_drain(
                        Arc::clone(&worker_mesh) as Arc<dyn CooperativeDrainSet>
                    ),
                )));
            }

            // Build a TaskContext seeded with the right `DistributedTaskContext` so the boundary
            // nodes inside the fragment's plan know their `(task_index, task_count)`.
            let cfg = session_arc
                .state()
                .config()
                .clone()
                .with_extension(Arc::new(DistributedTaskContext {
                    task_index: fragment.task_idx,
                    task_count: fragment.task_count,
                }));
            let memory_pool =
                create_memory_pool(frag_plan, work_mem_bytes, hash_mem_multiplier);
            let task_ctx = Arc::new(
                TaskContext::default()
                    .with_session_config(cfg)
                    .with_runtime(build_runtime_env(memory_pool)),
            );

            // The fragment arrives ready-to-run: the leader serialized it with nested stages
            // already `Remote`, so its boundary leaves read the mesh through the session's
            // `ShmChannelResolver`. Nothing here converts or dispatches.
            let plan = Arc::clone(frag_plan);
            // Kept for the post-run metrics frame: the executed nodes (and their metrics)
            // live in this plan.
            executed_fragments.push((
                fragment.stage_id,
                fragment.task_idx,
                fragment.task_count,
                Arc::clone(&plan),
            ));
            futures.push(Box::pin(run_worker_fragment(
                plan,
                per_partition_sinks,
                task_ctx,
            )));
        }
        // The metrics frames go to the leader after the fragments finish; the clone keeps one
        // sender on the leader's inbox alive past the drop below, which only delays that ring's
        // detach observation, never a per-channel EOF.
        let metrics_sender_base = outbound_senders
            .first()
            .and_then(|s| s.as_ref())
            .map(|s| s.clone_with_header(MppFrameHeader::task_metrics(0, 0, this_proc)));
        // Drop the original outbound_senders so the only remaining Arcs to each shm_mq queue /
        // in-proc channel are the per-partition clones owned by the spawned fragments. Without
        // this, the originals would outlive the futures, the consumer-side drains would never
        // observe `Detached`, and `execute`'s pull loop would spin forever.
        drop(outbound_senders);
        crate::mpp_log!(
            "mpp worker dispatch this_proc={this_proc} starting join_all on {} fragments",
            fragments.len()
        );
        // `join_all`, not `try_join_all`. Fail-fast cancellation would drop sibling fragments
        // mid-`await`, and a cancelled `run_worker_fragment` cancels its inner partition
        // futures, leaving their `(stage_id, partition)` sub-buffers stuck at
        // `sources_done == 0` on the consumer side. Per-channel EOF is load-bearing on the
        // substrate alone (matching reasoning in `run_worker_fragment`).
        //
        // Deadlock detector. Under `paradedb.mpp_debug`, if any fragment hasn't completed
        // within 30 s the dispatcher surfaces an error instead of letting the backend spin
        // forever.
        let join_fut = futures::future::join_all(futures);
        let outcome: Result<(), datafusion::common::DataFusionError> = if crate::gucs::mpp_debug() {
            match tokio::time::timeout(std::time::Duration::from_secs(30), join_fut).await {
                Ok(results) => results
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()
                    .map(|_| ()),
                Err(_) => {
                    crate::mpp_log!(
                        "mpp worker dispatch this_proc={this_proc} HANG: join_all exceeded 30s"
                    );
                    Err(datafusion::common::DataFusionError::Internal(format!(
                        "mpp worker dispatch (proc={this_proc}): join_all exceeded 30s; \
                         deadlock detector triggered"
                    )))
                }
            }
        } else {
            join_fut
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .map(|_| ())
        };

        // Report each fragment's metrics to the leader, even after a fragment error: partial
        // metrics still tell the user where the time went. Best-effort like every transport's
        // metrics path; the bounded send drops the frame if the leader already went away.
        if let Some(base) = metrics_sender_base {
            for (stage_id, task_idx, task_count, plan) in &executed_fragments {
                let frame = collect_task_metrics(plan, *task_idx, *task_count);
                let sender = base.clone_with_header(MppFrameHeader::task_metrics(
                    *stage_id,
                    *task_idx as u32,
                    this_proc,
                ));
                let _ = sender.send_task_metrics_best_effort(&frame).await;
            }
        }
        outcome
    });
    // `block_on` has returned, so the runtime is idle and every fragment future (with its
    // DSM senders) has dropped. Resume interrupts, then service any cancel/die the loops
    // deferred, now on a stack with no live runtime; for a die this `proc_exit`s here instead
    // of mid-`block_on`.
    drop(held);
    check_for_interrupts();
    if let Err(e) = result {
        pgrx::error!("mpp worker: fragment dispatch failed: {e}");
    }
}
