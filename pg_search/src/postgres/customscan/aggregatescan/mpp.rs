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

//! AggregateScan MPP worker exec path.
//!
//! Holds [`AggregateScan::exec_mpp_worker`], the body of `exec_custom_scan` when a parallel
//! worker is running an MPP fragment. The leader's path stays in `mod.rs`; this file isolates
//! the worker dispatcher's logical-plan deserialization, distributed-physical-plan build,
//! fragment discovery, and `join_all`-driven async dispatch so `mod.rs` keeps its focus on the
//! leader-side trait impl and the serial Tantivy paths.

use std::sync::Arc;

use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::execution::TaskContext;
use datafusion::physical_plan::ExecutionPlanProperties;
use datafusion_distributed::{DistributedExec, DistributedTaskContext};
use pgrx::pg_sys;

use crate::api::HashSet;
use crate::postgres::customscan::aggregatescan::datafusion_exec::create_aggregate_session_context;
use crate::postgres::customscan::aggregatescan::scan_state;
use crate::postgres::customscan::aggregatescan::AggregateScan;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::datafusion::memory::create_memory_pool;
use crate::postgres::customscan::mpp::runtime::proc_for_task;
use crate::postgres::customscan::mpp::transport::{CooperativeDrainSet, MppFrameHeader, MppSender};
use crate::postgres::customscan::mpp::worker::run_worker_fragment;
use crate::postgres::customscan::mpp::worker_fragments::{
    find_worker_assignments, FragmentRouting,
};
use crate::postgres::customscan::parallel::list_segment_ids;
use crate::scan::codec::deserialize_logical_plan_with_runtime;

impl AggregateScan {
    /// MPP worker exec: deserialize the logical plan from DSM, build the
    /// distributed physical plan (matching what the leader produced), find
    /// the worker fragment (the `input_stage.plan` of the bottom
    /// `NetworkShuffleExec`), and run it via
    /// [`mpp::worker::run_worker_fragment`] which pushes every output batch
    /// to the leader's shm_mq queues. Workers emit zero rows back to PG;
    /// returning `null_mut()` signals end-of-stream.
    pub(super) fn exec_mpp_worker(
        state: &mut CustomScanStateWrapper<Self>,
    ) -> *mut pg_sys::TupleTableSlot {
        // Pull worker-thread inputs from the outer state before we borrow
        // df_state mutably. parallel_state and non_partitioning_segments
        // are required to pin each worker's PgSearchTableProvider to the
        // right segment slice (the partitioning source) and the leader's
        // canonical replica (the non-partitioning sources). Without them,
        // every worker re-scans the full data and the leader-side hash
        // partitions get the same rows from every worker.
        let parallel_state = state.custom_state().parallel_state;
        let non_partitioning_segments = state.custom_state().non_partitioning_segments.clone();
        let partitioning_source_idx = state
            .custom_state()
            .mpp_partitioning_source_idx
            .unwrap_or(0);
        let plan_sources_count = state
            .custom_state()
            .source_manifests
            .len()
            .max(non_partitioning_segments.len() + 1);

        let df_state = state
            .custom_state_mut()
            .datafusion_state
            .as_mut()
            .expect("DataFusion state must be initialized");

        if df_state.runtime.is_some() {
            // Already drained on a prior call; just signal EOF.
            return std::ptr::null_mut();
        }
        let scan_state::MppExecState::Worker(worker) = df_state.mpp.as_ref().expect("checked")
        else {
            unreachable!("exec_mpp_worker called outside Worker state");
        };
        let plan_bytes = worker.plan_bytes.clone();
        // Worker mesh + total proc count for the dispatcher. The mesh's `ShmMqWorkerTransport`
        // gets wired into the session below. n_workers is rederived from `total_procs - 1`
        // at the dispatcher, matching what `worker_setup` constructs from its `header.n_procs`.
        let worker_mesh = Arc::clone(&worker.mesh);
        let this_proc = worker.mesh.this_proc;
        let total_procs = worker.mesh.n_procs;
        let outbound_senders: Vec<Option<MppSender>> = match df_state.mpp.as_mut() {
            Some(scan_state::MppExecState::Worker(w)) => std::mem::take(&mut w.outbound_senders),
            _ => unreachable!(),
        };

        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => pgrx::error!("mpp worker: tokio runtime build failed: {e}"),
        };
        df_state.runtime = Some(runtime);
        let runtime = df_state.runtime.as_ref().unwrap();
        // Use a bare SessionContext for plan deserialization; the workers
        // need the same `with_distributed_planner` config the leader had so
        // the resulting physical plan exposes the matching NetworkShuffleExec.
        let ctx = create_aggregate_session_context();

        // Build per-source canonical segment ID sets. For the partitioning
        // source, pull the full list out of the populated ParallelScanState
        // (workers will then claim individual segments via `checkout_segment`
        // inside their `PgSearchTableProvider`). For non-partitioning sources,
        // use the segment IDs the leader snapshotted into shared memory.
        let mut index_segment_ids: Vec<HashSet<tantivy::index::SegmentId>> =
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
            &ctx.task_ctx(),
            parallel_state,
            None, // expr_context: bm25 search predicates don't need runtime params
            None, // planstate: same
            non_partitioning_segments,
            index_segment_ids,
        ) {
            Ok(lp) => lp,
            Err(e) => pgrx::error!("mpp worker: deserialize_logical_plan failed: {e}"),
        };

        // Build the worker's distributed session context. Reuses the same builder the leader
        // runs so both procs agree on stage shape, task estimator chain, target_partitions,
        // and codec — without that, `find_worker_assignments` returns no fragments for this
        // worker because the worker's plan numbers stages differently from the leader's.
        let session = Self::build_mpp_session_context(Arc::clone(&worker_mesh));

        let physical_plan =
            runtime.block_on(async { session.state().create_physical_plan(&logical).await });
        let physical_plan = match physical_plan {
            Ok(p) => p,
            Err(e) => pgrx::error!("mpp worker: create_physical_plan failed: {e}"),
        };
        // Walk the plan and collect every `(stage_id, task_idx)` slot owned
        // by this proc under the `proc_for_task` round-robin policy. The
        // dispatcher spawns one async task per fragment; together they form
        // the worker's complete contribution to the distributed plan.
        let n_workers = total_procs.saturating_sub(1).max(1);
        let fragments = find_worker_assignments(&physical_plan, this_proc, n_workers);
        if fragments.is_empty() {
            pgrx::warning!(
                "mpp worker (proc={this_proc}): no fragments assigned; skipping (worker emits zero rows)"
            );
            return std::ptr::null_mut();
        }

        let work_mem_bytes = unsafe { pg_sys::work_mem as usize * 1024 };
        let hash_mem_multiplier = unsafe { pg_sys::hash_mem_multiplier };
        let session_arc = Arc::new(session);

        // Two `Future` shapes share this vector: real producer-fragment
        // futures and broadcast short-circuit EOF-only stubs. The alias
        // keeps the `Vec<_>` declaration legible and silences
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
                // Build per-output-partition senders. For each partition `q`
                // emitted by this fragment, look up the destination proc via
                // `fragment.routing` and clone the right outbound sender.
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
                    // Attach the worker mesh as the cooperative drain so a
                    // full outbound shm_mq queue doesn't block the backend
                    // thread. The spin pulls every inbound drain while
                    // retrying the send, breaking N×N symmetric stalls.
                    per_partition_senders.push(
                        base.clone_with_header(MppFrameHeader::batch(fragment.stage_id, q_u32))
                            .with_cooperative_drain(
                                Arc::clone(&worker_mesh) as Arc<dyn CooperativeDrainSet>
                            ),
                    );
                }

                // Broadcast invariant: fail-loud cap check.
                //
                // pg_search's natural-shape AggregateScan plan canonical-replicates the build
                // subtree via the `mpp build all-gather` step. Every producer task would scan
                // the full canonical data, and the consumer's `select_all` would over-count by
                // `input_task_count`. The planner-level [`BroadcastBuildSideOneTaskEstimator`]
                // caps the build subtree at task_count=1, so a correct plan produces exactly one
                // Broadcast fragment with task_idx == 0.
                //
                // A non-zero `task_idx` here means the cap silently failed: maybe the estimator
                // wasn't installed, the chain order is wrong, or a future planner pass
                // re-expanded the build subtree. We surface this as a hard error rather than
                // silently EOF-only-ing the fragment. The EOF-only fallback is only correct
                // under the canonical-replica INVARIANT documented on
                // `FragmentRouting::Broadcast`, and we'd rather fail loudly than emit a stealth
                // correctness regression. Matches the top-level NetworkBroadcastExec treatment
                // in `worker_fragments::collect`.
                if matches!(fragment.routing, FragmentRouting::Broadcast { .. }) {
                    debug_assert!(
                        fragment.task_idx == 0,
                        "mpp dispatcher: Broadcast fragment with task_idx={} but \
                         BroadcastBuildSideOneTaskEstimator should have capped \
                         input_task_count at 1; plan-walk drift?",
                        fragment.task_idx,
                    );
                    if fragment.task_idx != 0 {
                        return Err(datafusion::common::DataFusionError::Internal(format!(
                            "mpp worker dispatch (proc={this_proc}): Broadcast fragment \
                             (stage_id={}, task_idx={}) with task_idx > 0. The planner-level \
                             BroadcastBuildSideOneTaskEstimator should cap input_task_count at \
                             1. A non-zero task_idx here indicates plan-walk drift or a missing \
                             estimator chain on this session; running the producer plan would \
                             duplicate the canonical replica, EOF-only-ing it would drop a \
                             real shard if the cap loss was due to a sharded build subtree. \
                             Surface as error so the divergence is visible.",
                            fragment.stage_id, fragment.task_idx,
                        )));
                    }
                }

                // Build a TaskContext seeded with the right `DistributedTaskContext` so the
                // boundary nodes inside the fragment's plan know their
                // `(task_index, task_count)`.
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

                // Wrap fragment.plan in a fresh `DistributedExec` and run
                // `prepare_in_process_plan` to convert any nested boundaries' input stages from
                // `Stage::Local` to `Stage::Remote`. Without this, a nested `NetworkShuffleExec`
                // / `NetworkBroadcastExec` hitting `LocalStage::execute` errors when its task
                // count exceeds 1; with the conversion, those boundaries dispatch through
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
            // Drop the original outbound_senders so the only remaining Arcs to each shm_mq queue
            // / in-proc channel are the per-partition clones owned by the spawned fragments.
            // Without this, the originals would outlive the futures, the consumer-side drains
            // would never observe `Detached`, and `stream_partition`'s pull loop would spin
            // forever.
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
            // forever. Per-drain state shows up in the inbound drains' own traces in
            // transport.rs / runtime.rs.
            let join_fut = futures::future::join_all(futures);
            let outcome: Result<(), datafusion::common::DataFusionError> =
                if crate::gucs::mpp_debug() {
                    match tokio::time::timeout(std::time::Duration::from_secs(30), join_fut).await {
                        Ok(results) => results
                            .into_iter()
                            .collect::<Result<Vec<_>, _>>()
                            .map(|_| ()),
                        Err(_) => {
                            crate::mpp_log!(
                                "mpp worker dispatch this_proc={this_proc} \
                             HANG: join_all exceeded 30s"
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
        std::ptr::null_mut()
    }
}
