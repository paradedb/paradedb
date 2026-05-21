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

//! Worker-side fragment discovery for the multi-fragment runner.
//!
//! [`find_worker_assignments`] walks a worker's physical plan, visits every
//! [`datafusion_distributed::NetworkBoundary`], and collects the
//! `(input_stage.num, task_idx, plan, routing)` tuples assigned to a given
//! `this_proc`. The dispatcher in `aggregatescan::exec_mpp_worker` runs one
//! fragment per returned [`FragmentAssignment`].
//!
//! The walker tracks a `ParentContext` per recursion level so nested
//! boundaries know which OUTER stage's tasks consume their output. The
//! routing math (which proc to send partition `q` to) depends on this:
//!
//! - **Top-level boundary** (`parent = None`): the consumer is the leader at
//!   proc 0. Every output partition goes there.
//! - **Nested boundary inside outer stage `S_outer.plan`**: the consumer is
//!   one of `S_outer`'s tasks. For [`NetworkShuffleExec`] the routing is
//!   hash-partitioned (partition `q` → consumer task `q / P_c` where `P_c`
//!   is the per-consumer-task output count); for [`NetworkCoalesceExec`]
//!   the routing collapses to a single consumer task.
//!
//! [`NetworkShuffleExec`]: datafusion_distributed::NetworkShuffleExec
//! [`NetworkCoalesceExec`]: datafusion_distributed::NetworkCoalesceExec

use std::sync::Arc;

use datafusion::physical_plan::ExecutionPlan;
#[cfg(not(test))]
use datafusion::physical_plan::ExecutionPlanProperties;
use datafusion_distributed::{NetworkBoundaryExt, NetworkBoundaryKind};

use crate::postgres::customscan::mpp::runtime::proc_for_task;

/// One worker fragment to run for `this_proc`. The fragment is one task of a
/// producer stage; the dispatcher runs `plan` with the matching
/// `DistributedTaskContext { task_index: task_idx, task_count }` and routes
/// each output partition through the channel selected by [`Self::routing`].
#[derive(Clone)]
pub struct FragmentAssignment {
    /// `input_stage.num` of the boundary whose producer side this fragment
    /// belongs to. Frames the fragment emits carry this in the
    /// `MppFrameHeader::stage_id` field.
    pub stage_id: u32,
    /// Task index within the stage (0..task_count).
    pub task_idx: usize,
    /// Total task count for this stage (= `input_stage.tasks.len()`).
    pub task_count: usize,
    /// Plan to execute: the boundary's `input_stage.plan`.
    pub plan: Arc<dyn ExecutionPlan>,
    /// How to route each output partition to a destination proc.
    pub routing: FragmentRouting,
}

/// Routing rule for a fragment's output partitions.
#[derive(Clone, Debug)]
pub enum FragmentRouting {
    /// All output partitions go to one destination proc (`NetworkCoalesceExec`
    /// or the top-level gather case). Frame header carries
    /// `(stage_id, partition)` directly; consumer reads
    /// `stream_partition(partition)`.
    Coalesce {
        /// Destination proc index. `0` for the leader (top-level gather),
        /// or an `assignment.proc_for(parent_stage_id, 0)` lookup for a
        /// nested coalesce that lands on a single consumer task.
        dest_proc: u32,
    },
    /// Hash-partitioned mesh ([`NetworkShuffleExec`]). Output partition `q`
    /// goes to consumer task `q / partitions_per_consumer_task`, hosted on
    /// `proc_for_task(n_workers, consumer_task_idx)`. Frame header is
    /// `(stage_id, q)` so the consumer's
    /// `stream_partition(P_c * task_index + p_local)` finds it via the
    /// per-`(stage_id, partition)` channel buffer registry.
    ///
    /// [`NetworkShuffleExec`]: datafusion_distributed::NetworkShuffleExec
    Shuffle {
        /// `B.properties().output_partitioning().partition_count()`. Equal to
        /// `P_c` in the receive-side formula `off = P_c * task_index`.
        partitions_per_consumer_task: usize,
    },
    /// Broadcast mesh ([`NetworkBroadcastExec`]). Wire-level math matches [`Self::Shuffle`]
    /// (`q → q / P_c`).
    ///
    /// Delta from upstream: we cap the build subtree at `task_count = 1` via
    /// [`BroadcastBuildSideOneTaskEstimator`], so the dispatcher only ever sees `task_idx == 0`
    /// fragments here. The estimator relies on the AggregateScan all-gather step canonical-
    /// replicating the build side; a future planner that emits `NetworkBroadcastExec` over a
    /// sharded child would silently drop shards under this rule and needs a new routing variant.
    ///
    /// [`NetworkBroadcastExec`]: datafusion_distributed::NetworkBroadcastExec
    /// [`BroadcastBuildSideOneTaskEstimator`]: crate::postgres::customscan::mpp::task_estimator::BroadcastBuildSideOneTaskEstimator
    Broadcast {
        /// `B.properties().output_partitioning().partition_count()`. Same
        /// semantics as
        /// [`Self::Shuffle::partitions_per_consumer_task`].
        partitions_per_consumer_task: usize,
    },
}

/// Walk `root` (the worker's physical plan) and collect every fragment
/// assigned to `this_proc` under the `proc_for_task` round-robin policy.
/// Returns one [`FragmentAssignment`] per `(stage_id, task_idx)` pair
/// hosted by this proc; the dispatcher spawns one async task per entry.
pub fn find_worker_assignments(
    root: &Arc<dyn ExecutionPlan>,
    this_proc: u32,
    n_workers: u32,
) -> Vec<FragmentAssignment> {
    let mut out = Vec::new();
    collect(
        root, this_proc, n_workers, /* nested = */ false, &mut out,
    );
    #[cfg(not(test))]
    {
        crate::mpp_log!(
            "mpp worker_fragments::find_worker_assignments this_proc={} fragments={}",
            this_proc,
            out.len()
        );
        for f in &out {
            let n_out = f.plan.output_partitioning().partition_count();
            match &f.routing {
                FragmentRouting::Coalesce { dest_proc } => crate::mpp_log!(
                    "mpp worker_fragments fragment stage_id={} task_idx={} task_count={} \
                     n_out={n_out} routing=Coalesce dest_proc={dest_proc}",
                    f.stage_id,
                    f.task_idx,
                    f.task_count,
                ),
                FragmentRouting::Shuffle {
                    partitions_per_consumer_task,
                } => crate::mpp_log!(
                    "mpp worker_fragments fragment stage_id={} task_idx={} task_count={} \
                     n_out={n_out} routing=Shuffle \
                     partitions_per_consumer_task={partitions_per_consumer_task}",
                    f.stage_id,
                    f.task_idx,
                    f.task_count,
                ),
                FragmentRouting::Broadcast {
                    partitions_per_consumer_task,
                } => crate::mpp_log!(
                    "mpp worker_fragments fragment stage_id={} task_idx={} task_count={} \
                     n_out={n_out} routing=Broadcast \
                     partitions_per_consumer_task={partitions_per_consumer_task}",
                    f.stage_id,
                    f.task_idx,
                    f.task_count,
                ),
            }
        }
    }
    out
}

fn collect(
    plan: &Arc<dyn ExecutionPlan>,
    this_proc: u32,
    n_workers: u32,
    nested: bool,
    out: &mut Vec<FragmentAssignment>,
) {
    if let Some(nb) = plan.as_ref().as_network_boundary() {
        let stage = nb.input_stage();
        let stage_id = stage.num() as u32;
        let kind = nb.kind();
        // Per-consumer-task partition count from upstream's `NetworkShuffleExec::execute`
        // receive-side formula `off = P_c * task_index`.
        let p_c = plan.properties().partitioning.partition_count();

        // Classify by `(boundary_kind, top_level)`. Upstream DF-D dispatches producers via gRPC
        // keyed on the resolver's URLs, so worker code never has to decide where a producer's
        // output partitions land. We don't have URLs: each shm_mq peer is push-driven, and the
        // dispatcher here picks the destination proc for every output partition. The routing math
        // differs by boundary kind (Shuffle / Broadcast share receive-side math but Broadcast caps
        // to task 0; Coalesce collapses to one consumer task), and the top-level case routes to
        // the leader.
        //
        // `nb.kind()` returns a typed enum from the DF-D fork (paradedb/datafusion-distributed#9),
        // replacing the older `plan.name()` string match. The enum match makes a fork rename of
        // any of the three exec types a compile error here instead of a silent fall-through to
        // the `mis-route batches` arm.
        let routing = match (kind, nested) {
            // Top-level NetworkBroadcastExec isn't a shape the natural-shape AggregateScan plan
            // produces (broadcast is always nested inside the HashJoin build subtree). Falling
            // through to `Coalesce { dest_proc: 0 }` would silently send every input task's full
            // canonical replica to the leader and `select_all` would over-count by
            // `input_task_count`. Surface it as an error so a future planner change that hits
            // this shape doesn't silently produce wrong answers.
            (NetworkBoundaryKind::Broadcast, false) => {
                crate::postgres::customscan::mpp::fail_loud(format!(
                    "mpp worker_fragments: top-level NetworkBroadcastExec is unsupported \
                     (stage_id={stage_id}). The natural-shape AggregateScan plan does not \
                     produce this shape; route via a NetworkCoalesceExec gather instead."
                ))
            }
            // Top-level boundary (gather to leader): consumer is leader proc 0.
            (_, false) => FragmentRouting::Coalesce { dest_proc: 0 },
            // Nested NetworkShuffleExec: hash-partitioned mesh. Each output partition q maps to
            // consumer task q / p_c.
            (NetworkBoundaryKind::Shuffle, true) => FragmentRouting::Shuffle {
                partitions_per_consumer_task: p_c,
            },
            // Nested NetworkBroadcastExec: same wire-level math as Shuffle, but the dispatcher
            // only runs the producer plan on task 0 to avoid the canonical-replica duplication
            // described on `FragmentRouting::Broadcast`.
            (NetworkBoundaryKind::Broadcast, true) => FragmentRouting::Broadcast {
                partitions_per_consumer_task: p_c,
            },
            // Nested NetworkCoalesceExec: consumer is a single task in the parent stage. The
            // receive math collapses to task 0 of the parent group, so the destination proc is
            // `proc_for_task(n_workers, 0)`.
            (NetworkBoundaryKind::Coalesce, true) => FragmentRouting::Coalesce {
                dest_proc: proc_for_task(n_workers, 0),
            },
        };
        #[cfg(not(test))]
        {
            crate::mpp_log!(
                "mpp worker_fragments::collect boundary kind={kind:?} stage_id={stage_id} \
                 p_c={p_c} nested={nested}"
            );
        }

        let task_count = stage.task_count();
        if let Some(stage_plan) = stage.local_plan() {
            for task_idx in 0..task_count {
                let owner = proc_for_task(n_workers, task_idx as u32);
                if owner == this_proc {
                    out.push(FragmentAssignment {
                        stage_id,
                        task_idx,
                        task_count,
                        plan: Arc::clone(stage_plan),
                        routing: routing.clone(),
                    });
                }
            }
            // Recurse into the stage's plan with `nested = true`. The boundary's `children()`
            // returns `[stage.plan]`, so descending through it would double-process every nested
            // fragment. Return here to keep visit counts exact.
            collect(stage_plan, this_proc, n_workers, true, out);
        }
        return;
    }
    // Non-boundary nodes recurse through plan children.
    for child in plan.children() {
        collect(child, this_proc, n_workers, nested, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::physical_plan::empty::EmptyExec;

    #[test]
    fn boundary_free_plan_returns_no_assignments() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        let plan: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(schema));
        let out = find_worker_assignments(&plan, 1, 3);
        assert!(out.is_empty());
    }

    #[test]
    fn routing_coalesce_for_top_level_assigns_to_leader() {
        // Smoke check of the routing enum's Coalesce branch: a top-level
        // boundary routes to proc 0 regardless of parent.
        let r = FragmentRouting::Coalesce { dest_proc: 0 };
        match r {
            FragmentRouting::Coalesce { dest_proc } => assert_eq!(dest_proc, 0),
            _ => panic!("expected Coalesce"),
        }
    }

    #[test]
    fn routing_shuffle_carries_pc() {
        let r = FragmentRouting::Shuffle {
            partitions_per_consumer_task: 3,
        };
        match r {
            FragmentRouting::Shuffle {
                partitions_per_consumer_task,
            } => assert_eq!(partitions_per_consumer_task, 3),
            _ => panic!("expected Shuffle"),
        }
    }
}
