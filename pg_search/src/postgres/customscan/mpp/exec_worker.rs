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

//! Shape-agnostic MPP worker entry point.
//!
//! Pull-shape protocol body: deserialize the leader's logical plan from DSM, build a distributed
//! physical plan with the same session config the leader ran, walk it for stage→plan mappings,
//! install a [`ProducerTaskRegistry`] as the request handler on every inbound drain, and enter a
//! service loop that pumps frames + dispatches Requests until the leader detaches and no drivers
//! are in flight.
//!
//! Per-scan wrappers (`AggregateScan::exec_mpp_worker`, `JoinScan::exec_mpp_worker`) extract
//! their inputs into [`MppWorkerInputs`], build their seed `SessionContext`, and call
//! [`run_mpp_worker`].

use std::sync::Arc;

use datafusion::common::DataFusionError;
use datafusion::execution::SessionStateBuilder;
use datafusion::prelude::SessionContext;
use datafusion_distributed::{DistributedExt, SessionStateBuilderExt};
use pgrx::pg_sys;
use tantivy::index::SegmentId;

use crate::api::HashSet;
use crate::postgres::customscan::mpp::coalesce_rule::CoalesceBeforeNetworkShuffleRule;
use crate::postgres::customscan::mpp::producer_service::{ProducerTaskRegistry, StagePlans};
use crate::postgres::customscan::mpp::runtime::{
    proc_for_task, MppMesh, MppWorkerResolver, ShmMqWorkerTransport,
};
use crate::postgres::customscan::mpp::task_estimator::BroadcastBuildSideOneTaskEstimator;
use crate::postgres::customscan::parallel::list_segment_ids;
use crate::postgres::ParallelScanState;
use crate::scan::codec::deserialize_logical_plan_with_runtime;
use crate::scan::physical_codec::PgSearchPhysicalCodec;

/// Bundle of inputs the worker dispatcher needs. Per-scan `exec_mpp_worker` wrappers populate
/// this from their typed state and hand it to [`run_mpp_worker`].
pub(crate) struct MppWorkerInputs {
    /// Worker's view of the shared `ParallelScanState` (the DSM-attached state the leader
    /// populated), used to claim the partitioning source's segment slice and to rebuild
    /// PgSearchScan runtime state on decoded shipped subplans.
    pub parallel_state: Option<*mut ParallelScanState>,
    /// Canonical segment ID sets for non-partitioning sources, snapshotted by the leader.
    pub non_partitioning_segments: Vec<HashSet<SegmentId>>,
    /// Index (in the codec's per-source layout) of the source the workers partition over.
    pub partitioning_source_idx: usize,
    /// Total number of sources in the plan. Used to size the codec's per-source segment-ID Vec.
    pub plan_sources_count: usize,
    /// Leader's serialized logical plan, copied out of DSM during `worker_setup`.
    pub plan_bytes: Vec<u8>,
    /// This worker's `MppMesh` handle. Owns both halves of the queue grid (inbound drains + the
    /// outbound senders the producer service uses to respond to Requests).
    pub worker_mesh: Arc<MppMesh>,
}

/// Build the worker/leader distributed session context. Same builder both procs run so they
/// agree on stage shape, task estimator chain, target_partitions, and codec. Without that, a
/// worker's plan numbers stages differently from the leader's and the per-stage plan registry
/// disagrees with the consumer-side `target_task` addressing.
///
/// `seed` is the customscan's serial session context (`create_aggregate_session_context()` for
/// AggregateScan, `create_datafusion_session_context(SessionContextProfile::Join)` for JoinScan).
/// The function copies its config and layers the distributed-planner knobs on top.
pub(crate) fn build_mpp_session_context(
    seed: SessionContext,
    mesh: Arc<MppMesh>,
) -> SessionContext {
    let n_workers = mesh.n_procs.saturating_sub(1).max(1) as usize;
    // Four-knob unlock for actually inserting NetworkShuffleExec/etc.:
    //   1. target_partitions(N) — without this, EnforceDistribution skips every
    //      RepartitionExec, so the annotator never sees a Shuffle.
    //   2. distributed_task_estimator(N) — without this, leaves default to Maximum(1) and
    //      `_distribute_plan` elides every shuffle.
    //   3. distributed_broadcast_joins(true) — CollectLeft HashJoins otherwise cap their
    //      stage's task_count to Maximum(1) and propagate that cap upward, eliding shuffles
    //      above the join.
    //   4. distributed_user_codec — DF-D's `DistributedCodec::new_combined_with_user` reads
    //      the user codec back off the config when shipping subplans (leader's
    //      `ship_subplans_to_workers`) and when decoding them on the worker (`on_subplan`).
    //      The real `PgSearchPhysicalCodec` round-trips our four custom execs
    //      (`VisibilityFilterExec`, `TantivyLookupExec`, `SegmentedTopKExec`, `PgSearchScan`)
    //      plus the five built-in aggregate UDAFs (`min`/`max`/`count`/`sum`/`avg`) that
    //      shipped `AggregateExec` plans depend on.
    let cfg = seed
        .copied_config()
        .with_target_partitions(n_workers.max(2));

    let state_builder = SessionStateBuilder::new_from_existing(seed.state())
        .with_config(cfg)
        .with_distributed_worker_resolver(MppWorkerResolver::new(n_workers))
        .with_distributed_worker_transport(ShmMqWorkerTransport::new(mesh))
        .with_distributed_in_process_mode(true)
        .expect("with_distributed_in_process_mode")
        // BroadcastBuildSideOneTaskEstimator must come first in the chain. The DF-D fork tries
        // each estimator in registration order until one returns Some. Without the broadcast cap,
        // the default leaf estimator returns `Desired(n_workers)` for the memory-leaf canonical
        // all-gather, the consumer's `select_all` over-counts by `n_workers`, and pull-mode would
        // additionally dispatch one task per task_idx for the broadcast subtree — same dup
        // problem as push-mode. The estimator caps `BroadcastExec` at task_count = 1 so only
        // `task_idx = 0` requests are valid for broadcast stages.
        .with_distributed_task_estimator(BroadcastBuildSideOneTaskEstimator)
        .with_distributed_task_estimator(n_workers)
        .with_distributed_broadcast_joins(true)
        .expect("with_distributed_broadcast_joins")
        .with_distributed_user_codec(PgSearchPhysicalCodec)
        .with_distributed_planner()
        // Insert a `CoalesceBatchesExec` in front of every `NetworkShuffleExec` so partial-agg
        // output (often many small batches at high group cardinality) gets bundled into ~5 MB
        // Arrow IPC frames before hitting shm_mq. Without this the 20M `aggregate_join_groupby`
        // bench takes 2× longer than the pre-multi-stage path — see [`CoalesceBeforeNetworkShuffleRule`]
        // for the cost analysis.
        .with_physical_optimizer_rule(Arc::new(CoalesceBeforeNetworkShuffleRule));
    SessionContext::new_with_state(state_builder.build())
}

/// Shape-agnostic body of `exec_mpp_worker`. Runs to completion on the caller's tokio runtime,
/// `pgrx::error!`s on fatal failures, returns normally once the leader has detached and every
/// in-flight driver has emitted its `Eof` frame.
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
        non_partitioning_segments,
        partitioning_source_idx,
        plan_sources_count,
        plan_bytes,
        worker_mesh,
    } = inputs;

    let this_proc = worker_mesh.this_proc;

    // Build per-source canonical segment ID sets. For the partitioning source, pull the full
    // list out of the populated ParallelScanState (workers will then claim individual segments
    // via `checkout_segment` inside their `PgSearchTableProvider`). For non-partitioning sources,
    // use the segment IDs the leader snapshotted into shared memory.
    let mut index_segment_ids: Vec<HashSet<SegmentId>> =
        vec![HashSet::default(); plan_sources_count];
    if let Some(ps) = parallel_state {
        let mut np_counter = 0usize;
        for (i, slot) in index_segment_ids.iter_mut().enumerate() {
            if i == partitioning_source_idx {
                *slot = unsafe { list_segment_ids(ps) };
            } else if let Some(ids) = non_partitioning_segments.get(np_counter) {
                *slot = ids.clone();
                np_counter += 1;
            }
        }
    }

    let logical = match deserialize_logical_plan_with_runtime(
        &plan_bytes,
        &seed_ctx.task_ctx(),
        parallel_state,
        None,
        None,
        non_partitioning_segments,
        index_segment_ids.clone(),
    ) {
        Ok(lp) => lp,
        Err(e) => pgrx::error!("mpp worker: deserialize_logical_plan failed: {e}"),
    };

    let session = build_mpp_session_context(seed_ctx, Arc::clone(&worker_mesh));

    let physical_plan =
        runtime.block_on(async { session.state().create_physical_plan(&logical).await });
    let physical_plan = match physical_plan {
        Ok(p) => p,
        Err(e) => pgrx::error!("mpp worker: create_physical_plan failed: {e}"),
    };

    // Walk the distributed physical plan and capture every (stage_id, local_plan, task_count)
    // for the producer service. Empty means no NetworkBoundaryExt under the root — the natural
    // shape always emits at least one, so an empty walk means the planner gave us a degenerate
    // plan and the worker has nothing to do.
    let stage_plans = StagePlans::build(&physical_plan);
    if stage_plans.is_empty() {
        pgrx::warning!(
            "mpp worker (proc={this_proc}): physical plan has no NetworkBoundary; skipping \
             (worker emits zero rows)"
        );
        return;
    }
    crate::mpp_log!(
        "mpp worker (proc={this_proc}): {} stage(s) registered for pull-shape dispatch",
        stage_plans.len()
    );

    let work_mem_bytes = unsafe { pg_sys::work_mem as usize * 1024 };
    let hash_mem_multiplier = unsafe { pg_sys::hash_mem_multiplier };
    let session_arc = Arc::new(session);
    // Pass the per-source canonical segment-ID sets + the ParallelScanState pointer down so the
    // registry can rebuild PgSearchScan / FFHelper runtime state when decoding leader-shipped
    // subplans.
    let registry = Arc::new(ProducerTaskRegistry::new(
        stage_plans,
        session_arc,
        &worker_mesh,
        work_mem_bytes,
        hash_mem_multiplier,
        index_segment_ids,
        parallel_state,
    ));

    // Install the request handler on every inbound drain. The cooperative drain dispatches
    // each `Request` frame to `registry.on_request`, which spawns a driver future on the same
    // tokio runtime backing this `block_on`.
    worker_mesh.install_request_handler(Arc::clone(&registry)
        as Arc<dyn crate::postgres::customscan::mpp::transport::RequestHandler>);

    // Install the subplan handler so leader-shipped per-(stage, task) subplans land in
    // `registry.shipped_subplans`. `ProducerTaskRegistry::prepare_task` consults this map
    // first; on a hit the shipped subplan is used directly. Must be installed BEFORE the
    // initial drain pump below so any Subplan frames already buffered get routed.
    worker_mesh.install_subplan_handler(Arc::clone(&registry)
        as Arc<dyn crate::postgres::customscan::mpp::transport::SubplanHandler>);

    // Enumerate the `(stage_id, task_idx)` tuples this worker owns. Only owned tasks ever
    // receive Requests under the natural-shape estimator chain
    // (`proc_for_task(n_workers, t) == this_proc`), so those are the only ones the prewarm
    // barrier needs to settle before the service loop runs.
    //
    // The defensive `broadcast_zero` prewarm on non-owner procs that used to live here was
    // load-bearing only via the prep-cache race the new barrier closes. Two independent
    // invariants make it safe to drop today: (1) `BroadcastBuildSideOneTaskEstimator` caps
    // broadcast subtrees at `task_count == 1`, and (2) `proc_for_task(_, 0) == 1`. If either
    // invariant changes — a different estimator chain, or a `proc_for_task` offset shift —
    // non-owner procs could start serving `task_idx == 0` again, and the broadcast_zero
    // prewarm needs to come back alongside the change. See `task_estimator.rs` and
    // `runtime.rs::proc_for_task`.
    //
    // `owned_keys` is sorted so the barrier waits on keys in a deterministic order. The
    // underlying `StagePlans` map iterates in `HashMap` order, which is randomized per
    // process; without the sort, two runs of the same query show different per-worker
    // startup-latency traces depending on which key happens to be checked first.
    let n_workers = worker_mesh.n_workers();
    let mut owned_keys: Vec<(u32, u32)> = registry
        .iter_task_counts()
        .flat_map(|(stage_id, task_count)| {
            (0..task_count as u32)
                .filter(|task_idx| proc_for_task(n_workers, *task_idx) == this_proc)
                .map(move |task_idx| (stage_id, task_idx))
        })
        .collect();
    owned_keys.sort_unstable();

    // Prewarm barrier: drain → wait until `on_subplan` has *attempted* the owned key (shipped
    // OR decode-failed) → call `prewarm`. `prepare_task` then either consumes the shipped
    // subplan (preferred) or falls through to the local-prepare path (codec-gap fallback,
    // TODO(codec-coverage) tracked separately). The barrier replaces the old startup-eager
    // prewarm that raced with the leader's `ship_subplans_to_workers` and could cache a
    // locally-prepared entry that `on_subplan` would later have to invalidate.
    //
    // The barrier closes the prewarm-time race; a separate, smaller race remains during
    // prewarm itself if a Request arrives before its corresponding Subplan. The leader's
    // ship-before-execute ordering (`ship_subplans_to_workers` runs synchronously to
    // completion before `physical_plan.execute` issues any Request) keeps that race closed
    // under the natural shape; documented here because it's an unenforced invariant rather
    // than a structural one.
    //
    // Termination has three exits (in priority order):
    //   1. `is_subplan_attempted(stage, task)` flips → call prewarm, advance to next key.
    //   2. `registry.take_error()` returns `Some` → a spawned driver future failed during
    //      the barrier; surface it as `pgrx::error!` rather than wait forever.
    //   3. `worker_mesh.leader_inbound_detached()` flips before (1) → the leader gave up
    //      shipping (e.g. a leader-side `pgrx::error!` longjmped out of
    //      `ship_subplans_to_workers` partway through the task vector); error out with a
    //      clear diagnostic instead of looping forever.
    //
    // The barrier sits inside `runtime.block_on` so `tokio::task::yield_now().await` is the
    // right cooperative-yield primitive (lets `tokio::spawn`ed drivers from `on_request`
    // make forward progress) and so `tokio::spawn` itself has a current runtime — without
    // the runtime context, a `Request` frame already queued at startup brings the worker
    // down the moment its dispatch path calls `tokio::spawn`.
    let result: Result<(), DataFusionError> = runtime.block_on(async {
        for (stage_id, task_idx) in &owned_keys {
            let (stage_id, task_idx) = (*stage_id, *task_idx);
            loop {
                pgrx::check_for_interrupts!();
                worker_mesh.drain_all_inbound()?;
                if registry.is_subplan_attempted(stage_id, task_idx) {
                    break;
                }
                if let Some(e) = registry.take_error() {
                    return Err(e);
                }
                if worker_mesh.leader_inbound_detached() {
                    return Err(DataFusionError::Internal(format!(
                        "mpp worker (proc={this_proc}): leader detached before shipping \
                         subplan for stage_id={stage_id} task_idx={task_idx}; most likely \
                         the leader's `ship_subplans_to_workers` errored partway through \
                         the task vector, or the leader's and worker's `StagePlans` walks \
                         disagree on which stages exist"
                    )));
                }
                tokio::task::yield_now().await;
            }
            // After the barrier, `prewarm` runs either the shipped path (when `on_subplan`
            // decoded successfully) or the local-prepare fallback (when decode failed). An
            // error here therefore means either:
            //   - shipped path's `build_task_ctx` failed (memory pool builder, etc.), OR
            //   - local-prepare's `DistributedExec::prepare_in_process_plan` failed (DF
            //     planner refused the shape, etc.).
            // Either way, abort instead of sliding into the service loop with no prepared
            // plan for an owned task.
            registry.prewarm(stage_id, task_idx)?;
        }
        Ok(())
    });
    if let Err(e) = result {
        pgrx::error!("mpp worker (proc={this_proc}): prewarm barrier failed: {e}");
    }

    // Service loop. Three pieces interleave on the single tokio task scheduler:
    //   - This loop: pumps every inbound drain, dispatching Requests.
    //   - Spawned per-partition drivers (one per Request): run plan.execute(p, ctx), push
    //     batches + Eof through `mesh.outbound_sender(sender_proc).clone_with_header(...)`.
    //   - The cooperative-drain spin inside each `MppSender::send_*_traced`: pumps the same
    //     inbounds while waiting for outbound space.
    //
    // All on the backend thread, all under pgrx's `check_active_thread` invariant. `yield_now`
    // surrenders the runtime between iterations so spawned drivers make forward progress;
    // without it, this loop would starve everything else on the runtime.
    //
    // Termination: leader detached AND no active drivers. Leader detach means the leader's
    // outbound senders dropped — no more Requests can arrive from the consumer side. With zero
    // active drivers, no producer fragment is still streaming. Using the leader's inbound as the
    // detection signal (not "every inbound detached") avoids the symmetric-wait deadlock where
    // every worker would wait for every other worker to detach first; see
    // `MppMesh::leader_inbound_detached` for the reasoning.
    let result: Result<(), DataFusionError> = runtime.block_on(async {
        loop {
            pgrx::check_for_interrupts!();
            worker_mesh.drain_all_inbound()?;
            if let Some(e) = registry.take_error() {
                return Err(e);
            }
            if worker_mesh.leader_inbound_detached() && registry.active_drivers() == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
        Ok(())
    });

    // Break the registry ↔ mesh handler cycles so the registry's `Arc<dyn RequestHandler>` and
    // `Arc<dyn SubplanHandler>` refs (one each per drain) release. Without this, `registry` and
    // `worker_mesh` keep each other alive past the function return.
    worker_mesh.uninstall_request_handler();
    worker_mesh.uninstall_subplan_handler();
    drop(registry);

    if let Err(e) = result {
        pgrx::error!("mpp worker: service loop failed: {e}");
    }
}
