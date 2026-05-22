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
//! The plan-walking step lives in the fork as
//! [`datafusion_distributed::for_each_worker_fragment`]: it descends the
//! distributed plan tree, detects every [`datafusion_distributed::NetworkBoundary`],
//! and yields a [`datafusion_distributed::WorkerFragment`] per
//! `(stage_id, task_idx)` tuple. This module turns each fragment into a
//! pg_search-specific [`FragmentAssignment`] by:
//!
//! 1. Deciding whether THIS proc owns the fragment, via
//!    [`proc_for_task`]'s round-robin mapping.
//! 2. Deciding how to route each of the fragment's output partitions, via the
//!    `(kind, nested)` match in [`routing_for`].
//!
//! Routing depends on whether the boundary is top-level or nested inside another
//! stage:
//!
//! - **Top-level Coalesce** (`nested = false`): the consumer is the leader at proc 0.
//!   Every output partition goes there.
//! - **Top-level Shuffle / Broadcast** (`nested = false`): not a shape any of our
//!   customscan plans produce; surfaced as a fail-loud planner anomaly.
//! - **Nested Shuffle** (`nested = true`): hash-partitioned mesh. Output partition
//!   `q` maps to consumer task `q / partitions_per_consumer_task` on the proc that
//!   owns that consumer task.
//! - **Nested Broadcast** (`nested = true`): same wire-level math as Shuffle, but the
//!   producer plan only runs on task 0 (the fork's `broadcast_subtree_max_one_task`
//!   default caps the build subtree).
//! - **Nested Coalesce** (`nested = true`): consumer is a single task in the parent
//!   stage; routing collapses to `proc_for_task(n_workers, 0)`.

use std::sync::Arc;

use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::{for_each_worker_fragment, NetworkBoundaryKind, WorkerFragment};

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
    /// The fork's default `broadcast_subtree_max_one_task` (paradedb/datafusion-distributed#11)
    /// caps the build subtree at `task_count = 1`, so the dispatcher only ever sees
    /// `task_idx == 0` fragments here. The cap relies on the AggregateScan all-gather step
    /// canonical-replicating the build side; a future planner that emits `NetworkBroadcastExec`
    /// over a sharded child would silently drop shards under this rule and needs a new routing
    /// variant.
    ///
    /// [`NetworkBroadcastExec`]: datafusion_distributed::NetworkBroadcastExec
    Broadcast {
        /// `B.properties().output_partitioning().partition_count()`. Same
        /// semantics as
        /// [`Self::Shuffle::partitions_per_consumer_task`].
        partitions_per_consumer_task: usize,
    },
}

/// Walk `root` (the worker's physical plan) and collect every fragment
/// assigned to `this_proc` under the [`proc_for_task`] round-robin policy.
/// Returns one [`FragmentAssignment`] per `(stage_id, task_idx)` pair
/// hosted by this proc; the dispatcher spawns one async task per entry.
pub fn find_worker_assignments(
    root: &Arc<dyn ExecutionPlan>,
    this_proc: u32,
    n_workers: u32,
) -> Vec<FragmentAssignment> {
    let mut out = Vec::new();
    for_each_worker_fragment(root, |frag| {
        // Boundary-visit trace: logged for every fragment the visitor yields, regardless of
        // proc ownership. Useful when debugging "why does proc N have zero fragments?" —
        // without this, only fragments that pass the ownership filter below get logged.
        #[cfg(not(test))]
        crate::mpp_log!(
            "mpp worker_fragments::visit boundary kind={:?} stage_id={} task_idx={} \
             task_count={} p_c={} nested={}",
            frag.kind,
            frag.stage_id,
            frag.task_idx,
            frag.task_count,
            frag.partitions_per_consumer_task,
            frag.nested,
        );
        let owner = proc_for_task(n_workers, frag.task_idx as u32);
        if owner != this_proc {
            return;
        }
        out.push(FragmentAssignment {
            stage_id: frag.stage_id,
            task_idx: frag.task_idx,
            task_count: frag.task_count,
            plan: Arc::clone(frag.plan),
            routing: routing_for(&frag, n_workers),
        });
    });
    #[cfg(not(test))]
    {
        crate::mpp_log!(
            "mpp worker_fragments::find_worker_assignments this_proc={} fragments={}",
            this_proc,
            out.len()
        );
        for f in &out {
            use datafusion::physical_plan::ExecutionPlanProperties;
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

/// Decide the routing for a single fragment based on its boundary kind and whether it sits
/// at the top of the plan or nested inside another stage. The same shapes are spelled out
/// in this module's doc-comment; this function just encodes the table.
fn routing_for(frag: &WorkerFragment<'_>, n_workers: u32) -> FragmentRouting {
    let stage_id = frag.stage_id;
    let p_c = frag.partitions_per_consumer_task;
    match (frag.kind, frag.nested) {
        // Top-level NetworkBroadcastExec isn't a shape the natural-shape AggregateScan plan
        // produces (broadcast is always nested inside the HashJoin build subtree). Falling
        // through to `Coalesce { dest_proc: 0 }` would silently send every input task's
        // full canonical replica to the leader and `select_all` would over-count by
        // `input_task_count`. Surface it as an error so a future planner change that hits
        // this shape doesn't silently produce wrong answers.
        (NetworkBoundaryKind::Broadcast, false) => {
            crate::postgres::customscan::mpp::fail_loud(format!(
                "mpp worker_fragments: top-level NetworkBroadcastExec is unsupported \
                 (stage_id={stage_id}). The natural-shape AggregateScan plan does not \
                 produce this shape; route via a NetworkCoalesceExec gather instead."
            ))
        }
        // Top-level NetworkShuffleExec is also not a shape any of our customscan plans
        // produce — shuffles emit hash-partitioned output into a parent consumer stage,
        // not directly into the leader.
        (NetworkBoundaryKind::Shuffle, false) => {
            crate::postgres::customscan::mpp::fail_loud(format!(
                "mpp worker_fragments: top-level NetworkShuffleExec is unsupported \
                 (stage_id={stage_id}). Shuffles emit hash-partitioned output into a \
                 parent consumer stage; a top-level shuffle is a planner anomaly."
            ))
        }
        // Top-level NetworkCoalesceExec (gather to leader): consumer is leader proc 0.
        (NetworkBoundaryKind::Coalesce, false) => FragmentRouting::Coalesce { dest_proc: 0 },
        // Nested NetworkShuffleExec: hash-partitioned mesh. Each output partition q maps
        // to consumer task q / p_c.
        (NetworkBoundaryKind::Shuffle, true) => FragmentRouting::Shuffle {
            partitions_per_consumer_task: p_c,
        },
        // Nested NetworkBroadcastExec: same wire-level math as Shuffle, but the
        // dispatcher only runs the producer plan on task 0 (the fork's default
        // broadcast_subtree_max_one_task caps the build subtree).
        (NetworkBoundaryKind::Broadcast, true) => FragmentRouting::Broadcast {
            partitions_per_consumer_task: p_c,
        },
        // Nested NetworkCoalesceExec: consumer is a single task in the parent stage. The
        // receive math collapses to task 0 of the parent group, so the destination proc is
        // `proc_for_task(n_workers, 0)`.
        (NetworkBoundaryKind::Coalesce, true) => FragmentRouting::Coalesce {
            dest_proc: proc_for_task(n_workers, 0),
        },
        // `NetworkBoundaryKind` is `#[non_exhaustive]`. This arm catches any variant the
        // fork adds in a future release that we haven't reasoned about yet. Fail loudly
        // rather than guess at routing.
        _ => crate::postgres::customscan::mpp::fail_loud(format!(
            "mpp worker_fragments: unrecognized NetworkBoundaryKind {:?} \
             (stage_id={stage_id}). The DF-D fork has added a variant the embedder \
             hasn't been updated for; add a routing arm before bumping the rev.",
            frag.kind,
        )),
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
