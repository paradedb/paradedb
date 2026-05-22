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
//! The natural-shape MPP path is the same flow for every customscan that opts in: deserialize
//! the leader's logical plan from DSM, build a distributed physical plan with the same session
//! config the leader ran, walk it to find this proc's fragments, and dispatch them via
//! [`run_worker_fragment`] + `join_all`. The only customscan-specific pieces are the seed
//! `SessionContext` (different `SessionContextProfile`) and where the inputs come from in
//! per-scan state.
//!
//! This module isolates the shape-agnostic logic. Per-scan wrappers (`AggregateScan::exec_mpp_worker`,
//! `JoinScan::exec_mpp_worker`) extract their inputs into [`MppWorkerInputs`], build their seed
//! `SessionContext`, and call [`run_mpp_worker`].

use std::sync::Arc;

use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::execution::{SessionStateBuilder, TaskContext};
use datafusion::physical_plan::ExecutionPlanProperties;
use datafusion::prelude::SessionContext;
use datafusion_distributed::{
    DistributedExec, DistributedExt, DistributedTaskContext, MultiChannelFrameHeader,
    SessionStateBuilderExt,
};
use pgrx::pg_sys;
use tantivy::index::SegmentId;

use crate::api::HashSet;
use crate::postgres::customscan::datafusion::memory::create_memory_pool;
use crate::postgres::customscan::mpp::glue::producer_worker_count;
use crate::postgres::customscan::mpp::runtime::{proc_for_task, MppMesh, ShmMqWorkerTransport};
use crate::postgres::customscan::mpp::transport::{CooperativeDrainSet, MppSender};
use crate::postgres::customscan::mpp::worker::run_worker_fragment;
use crate::postgres::customscan::mpp::worker_fragments::{
    find_worker_assignments, FragmentRouting,
};
use crate::postgres::customscan::parallel::list_segment_ids;
use crate::postgres::ParallelScanState;
use crate::scan::codec::deserialize_logical_plan_with_runtime;

/// Bundle of inputs the worker dispatcher needs. Per-scan `exec_mpp_worker` wrappers populate
/// this from their typed state and hand it to [`run_mpp_worker`].
pub(crate) struct MppWorkerInputs {
    /// The leader's `ParallelScanState`, used to claim the partitioning source's segment slice.
    pub parallel_state: Option<*mut ParallelScanState>,
    /// Canonical segment ID sets for non-partitioning sources, snapshotted by the leader.
    pub non_partitioning_segments: Vec<HashSet<SegmentId>>,
    /// Index (in the codec's per-source layout) of the source the workers partition over.
    pub partitioning_source_idx: usize,
    /// Total number of sources in the plan. Used to size the codec's per-source segment-ID Vec.
    pub plan_sources_count: usize,
    /// Leader's serialized logical plan, copied out of DSM during `worker_setup`.
    pub plan_bytes: Vec<u8>,
    /// This worker's `MppMesh` handle.
    pub worker_mesh: Arc<MppMesh>,
    /// This worker's outbound senders, keyed by destination `proc_idx`. The dispatcher takes
    /// ownership; consumers see `Detached` once these drop.
    pub outbound_senders: Vec<Option<MppSender>>,
}

/// Build the worker/leader distributed session context. Same builder both procs run so they
/// agree on stage shape, task estimator chain, target_partitions, and codec. Without that,
/// `find_worker_assignments` returns no fragments because the worker's plan numbers stages
/// differently from the leader's.
///
/// `seed` is the customscan's serial session context (`create_aggregate_session_context()` for
/// AggregateScan, `create_datafusion_session_context(SessionContextProfile::Join)` for JoinScan).
/// The function copies its config and layers the distributed-planner knobs on top.
pub(crate) fn build_mpp_session_context(
    seed: SessionContext,
    mesh: Option<Arc<MppMesh>>,
) -> SessionContext {
    // Workers are procs 1..n_procs; leader is proc 0. The producer count is `n_procs - 1`.
    // `n_procs >= 3` is guaranteed by `mpp_is_active()` (callers gate before reaching this).
    //
    // `mesh = None` is the EXPLAIN-time variant: the planner only needs `n_workers` for stage
    // sizing and target_partitions; the WorkerTransport is never consulted because EXPLAIN
    // doesn't execute. The fork's default (FlightWorkerTransport) sits unused. `mesh = Some(_)`
    // is required for actual execution.
    //
    // Both branches resolve to `mpp_worker_count() - 1` at call time — the leader's `MppMesh`
    // is itself constructed from `mpp_worker_count()` in `leader_setup`. EXPLAIN reflects the
    // GUC at EXPLAIN time; a subsequent `SET paradedb.mpp_worker_count` shifts the next
    // execute path's stage shape away from what EXPLAIN rendered.
    let n_workers = match mesh.as_ref() {
        Some(m) => m.n_workers() as usize,
        None => producer_worker_count() as usize,
    };

    // The fork's `with_distributed_in_process_defaults(n_workers)`
    // (paradedb/datafusion-distributed#12) bundles the four-knob "make in-process MPP actually
    // emit shuffles" recipe: `target_partitions(max(n_workers,2))` + leaf
    // `task_estimator(n_workers)` + `broadcast_joins(true)` + `in_process_mode(true)`. Without
    // those four, `_distribute_plan` either skips `EnforceDistribution`, caps the build-side
    // at `Maximum(1)`, elides every shuffle, or hits the gRPC plan-send path we don't have.
    //
    // No `with_distributed_worker_resolver(...)` line: fork PR #10 makes the resolver optional
    // under `in_process_mode`. The fork substitutes a placeholder URL internally.
    //
    // No `with_distributed_user_codec(...)` line: fork PR #8 short-circuits codec usage under
    // `in_process_mode`. Workers re-plan from the logical plan we ship via DSM and never
    // decode a physical subplan over the wire. (If a remote-worker mode is ever exercised,
    // restore the codec here for our custom execs: `PgSearchScan`, `VisibilityFilterExec`,
    // `SegmentedTopKExec`, `TantivyLookupExec`, `FilterPassthroughExec`.)
    //
    // The broadcast-subtree cap (`broadcast_subtree_max_one_task = true`) is already a default
    // in `DistributedConfig` after fork PR #11.
    let mut state_builder = SessionStateBuilder::new_from_existing(seed.state())
        .with_distributed_in_process_defaults(n_workers)
        .expect("with_distributed_in_process_defaults");

    // Install the shm_mq transport only when running for actual execution (mesh = Some).
    // EXPLAIN-time plan rendering passes `mesh = None` and lets the fork's default
    // (FlightWorkerTransport) sit unused — the planner never calls `WorkerConnection::open()`
    // outside of streaming, so the default is fine for read-only plan introspection.
    if let Some(mesh) = mesh {
        state_builder =
            state_builder.with_distributed_worker_transport(ShmMqWorkerTransport::new(mesh));
    }

    SessionContext::new_with_state(state_builder.with_distributed_planner().build())
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
        non_partitioning_segments,
        partitioning_source_idx,
        plan_sources_count,
        plan_bytes,
        worker_mesh,
        outbound_senders,
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
        None, // expr_context: bm25 search predicates don't need runtime params
        None, // planstate: same
        non_partitioning_segments,
        index_segment_ids,
    ) {
        Ok(lp) => lp,
        Err(e) => pgrx::error!("mpp worker: deserialize_logical_plan failed: {e}"),
    };

    let session = build_mpp_session_context(seed_ctx, Some(Arc::clone(&worker_mesh)));

    let physical_plan =
        runtime.block_on(async { session.state().create_physical_plan(&logical).await });
    let physical_plan = match physical_plan {
        Ok(p) => p,
        Err(e) => pgrx::error!("mpp worker: create_physical_plan failed: {e}"),
    };

    // Walk the plan and collect every `(stage_id, task_idx)` slot owned by this proc under
    // the `proc_for_task` round-robin policy. The dispatcher spawns one async task per
    // fragment; together they form the worker's complete contribution to the distributed
    // plan. `worker_mesh.n_procs >= 3` is guaranteed by `mpp_is_active()` (callers gate
    // before reaching this), so `n_workers() = n_procs - 1` is safe.
    let n_workers = worker_mesh.n_workers();
    let fragments = find_worker_assignments(&physical_plan, this_proc, n_workers);
    if fragments.is_empty() {
        pgrx::warning!(
            "mpp worker (proc={this_proc}): no fragments assigned; skipping (worker emits zero rows)"
        );
        return;
    }

    let work_mem_bytes = unsafe { pg_sys::work_mem as usize * 1024 };
    let hash_mem_multiplier = unsafe { pg_sys::hash_mem_multiplier };
    let session_arc = Arc::new(session);

    // Two `Future` shapes share this vector: real producer-fragment futures and broadcast
    // short-circuit EOF-only stubs. The alias keeps the `Vec<_>` declaration legible and silences
    // clippy::type_complexity.
    type FragmentFuture = std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(), datafusion::common::DataFusionError>>
                + Send,
        >,
    >;
    let result = runtime.block_on(async move {
        let mut futures: Vec<FragmentFuture> = Vec::with_capacity(fragments.len());
        for fragment in &fragments {
            let n_out = fragment.plan.output_partitioning().partition_count();
            // Build per-output-partition senders. For each partition `q` emitted by this
            // fragment, look up the destination proc via `fragment.routing` and clone the right
            // outbound sender.
            let mut per_partition_senders: Vec<MppSender> = Vec::with_capacity(n_out);
            for q in 0..n_out {
                let dest_proc = match &fragment.routing {
                    FragmentRouting::Coalesce { dest_proc } => *dest_proc,
                    FragmentRouting::Shuffle {
                        partitions_per_consumer_task,
                    }
                    | FragmentRouting::Broadcast {
                        partitions_per_consumer_task,
                    } => {
                        let t_c = (q / partitions_per_consumer_task) as u32;
                        proc_for_task(n_workers, t_c)
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
                // Attach the worker mesh as the cooperative drain so a full outbound shm_mq
                // queue doesn't block the backend thread. The spin pulls every inbound drain
                // while retrying the send, breaking N×N symmetric stalls.
                per_partition_senders.push(
                    base.clone_with_header(MultiChannelFrameHeader::batch(
                        fragment.stage_id,
                        q_u32,
                    ))
                    .with_cooperative_drain(
                        Arc::clone(&worker_mesh) as Arc<dyn CooperativeDrainSet>
                    ),
                );
            }

            // Broadcast invariant: fail-loud cap check.
            //
            // The natural-shape plan canonical-replicates the build subtree via the `mpp build
            // all-gather` step. Every producer task would scan the full canonical data, and the
            // consumer's `select_all` would over-count by `input_task_count`. The fork's default
            // `broadcast_subtree_max_one_task` caps the build subtree at task_count=1, so a
            // correct plan produces exactly one Broadcast fragment with task_idx == 0.
            //
            // A non-zero `task_idx` here means the cap silently failed: maybe a future planner
            // pass re-expanded the build subtree, or someone turned the default-on flag off.
            // Surface as a hard error rather than silently EOF-only-ing the fragment.
            if matches!(fragment.routing, FragmentRouting::Broadcast { .. }) {
                debug_assert!(
                    fragment.task_idx == 0,
                    "mpp dispatcher: Broadcast fragment with task_idx={} but the fork's \
                     broadcast_subtree_max_one_task caps the producer subtree, so \
                     `NetworkBroadcastExec::input_task_count` should be 1 here; plan-walk drift?",
                    fragment.task_idx,
                );
                if fragment.task_idx != 0 {
                    return Err(datafusion::common::DataFusionError::Internal(format!(
                        "mpp worker dispatch (proc={this_proc}): Broadcast fragment \
                         (stage_id={}, task_idx={}) with task_idx > 0. The fork's default \
                         broadcast_subtree_max_one_task caps the producer subtree, so \
                         `NetworkBroadcastExec::input_task_count` is expected to be 1. A \
                         non-zero task_idx here indicates plan-walk drift or that the cap \
                         was disabled on this session.",
                        fragment.stage_id, fragment.task_idx,
                    )));
                }
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
                create_memory_pool(&fragment.plan, work_mem_bytes, hash_mem_multiplier);
            let task_ctx = Arc::new(
                TaskContext::default()
                    .with_session_config(cfg)
                    .with_runtime(Arc::new(
                        RuntimeEnvBuilder::new()
                            .with_memory_pool(memory_pool)
                            .build()
                            .expect("Failed to create RuntimeEnv"),
                    )),
            );

            // Wrap fragment.plan in a fresh `DistributedExec` and run `prepare_in_process_plan`
            // to convert any nested boundaries' input stages from `Stage::Local` to
            // `Stage::Remote`. Without this, a nested `NetworkShuffleExec` /
            // `NetworkBroadcastExec` hitting `LocalStage::execute` errors when its task count
            // exceeds 1; with the conversion, those boundaries dispatch through
            // `ShmMqWorkerTransport` exactly like outer boundaries.
            let plan = {
                let dist = Arc::new(DistributedExec::new(Arc::clone(&fragment.plan)));
                match dist.prepare_in_process_plan(&task_ctx) {
                    Ok(p) => p,
                    Err(e) => {
                        return Err(datafusion::common::DataFusionError::Internal(format!(
                            "mpp worker: prepare_in_process_plan failed for fragment \
                             (stage_id={}, task_idx={}): {e}",
                            fragment.stage_id, fragment.task_idx
                        )));
                    }
                }
            };
            futures.push(Box::pin(run_worker_fragment(
                plan,
                per_partition_senders,
                task_ctx,
            )));
        }
        // Drop the original outbound_senders so the only remaining Arcs to each shm_mq queue /
        // in-proc channel are the per-partition clones owned by the spawned fragments. Without
        // this, the originals would outlive the futures, the consumer-side drains would never
        // observe `Detached`, and `stream_partition`'s pull loop would spin forever.
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
        outcome
    });
    if let Err(e) = result {
        pgrx::error!("mpp worker: fragment dispatch failed: {e}");
    }
}
