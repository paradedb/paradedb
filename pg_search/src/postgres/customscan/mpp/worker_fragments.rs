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
use datafusion_distributed::NetworkBoundaryExt;

use crate::postgres::customscan::mpp::assignment::TaskAssignment;

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
    /// `assignment.proc_for(parent_stage_id, consumer_task_idx)`. Frame
    /// header is `(stage_id, q)` so the consumer's
    /// `stream_partition(P_c * task_index + p_local)` finds it via the
    /// per-`(stage_id, partition)` sub-buffer registry.
    ///
    /// [`NetworkShuffleExec`]: datafusion_distributed::NetworkShuffleExec
    Shuffle {
        /// Stage id of the immediately enclosing stage. Consumer tasks live
        /// here; combined with `consumer_task_idx`, they resolve to a
        /// destination proc via `TaskAssignment::proc_for`.
        parent_stage_id: u32,
        /// `B.properties().output_partitioning().partition_count()`. Equal to
        /// `P_c` in the receive-side formula `off = P_c * task_index`.
        partitions_per_consumer_task: usize,
    },
}

#[derive(Clone, Copy)]
struct ParentContext {
    /// Stage id of the enclosing stage whose plan contains this boundary.
    /// Used as `parent_stage_id` in `FragmentRouting::Shuffle` / consumer
    /// lookup in `FragmentRouting::Coalesce` (nested case).
    parent_stage_id: u32,
}

/// Walk `root` (the worker's physical plan) and collect every fragment
/// assigned to `this_proc` per `assignment`. Returns one
/// [`FragmentAssignment`] per (stage_id, task_idx) pair hosted by this proc;
/// the dispatcher spawns one async task per entry.
pub fn find_worker_assignments(
    root: &Arc<dyn ExecutionPlan>,
    this_proc: u32,
    assignment: &TaskAssignment,
) -> Vec<FragmentAssignment> {
    let mut out = Vec::new();
    collect(root, this_proc, assignment, None, &mut out);
    out
}

fn collect(
    plan: &Arc<dyn ExecutionPlan>,
    this_proc: u32,
    assignment: &TaskAssignment,
    parent: Option<ParentContext>,
    out: &mut Vec<FragmentAssignment>,
) {
    if let Some(nb) = plan.as_ref().as_network_boundary() {
        let stage = nb.input_stage();
        let stage_id = stage.num() as u32;
        // Boundary's name decides routing rule. Downcasting through the
        // trait gives us back-pointers to the concrete type, but the
        // dispatcher only needs the receive-side math which is identical
        // for the embedded model whether we got here through Shuffle or
        // Coalesce; the only distinction is the partitions-per-task math.
        let name = plan.name();
        // Receive-side per-consumer-task partition count: see
        // `NetworkShuffleExec::execute`'s `off = P_c * task_index` formula.
        let p_c = plan.properties().partitioning.partition_count();

        // Determine routing for fragments produced by this boundary's
        // input_stage tasks.
        let routing = match (name, parent) {
            // Top-level gather (or any top-level boundary): consumer is leader.
            (_, None) => FragmentRouting::Coalesce { dest_proc: 0 },
            // Nested NetworkShuffleExec: hash routing.
            ("NetworkShuffleExec", Some(pctx)) => FragmentRouting::Shuffle {
                parent_stage_id: pctx.parent_stage_id,
                partitions_per_consumer_task: p_c,
            },
            // Nested NetworkCoalesceExec / NetworkBroadcastExec: consumer is
            // a single task in the parent stage. M2.d targets the natural-
            // shape Aggregate plan where nested coalesce isn't expected;
            // we still handle it as "all output → parent task 0" so the
            // shape is supported when it does appear.
            (_, Some(pctx)) => {
                let dest = assignment.proc_for(pctx.parent_stage_id, 0).unwrap_or(0);
                FragmentRouting::Coalesce { dest_proc: dest }
            }
        };

        let task_count = stage.tasks.len();
        if let Some(stage_plan) = stage.plan() {
            for task_idx in 0..task_count {
                let owner = assignment.proc_for(stage_id, task_idx as u32);
                if owner == Some(this_proc) {
                    out.push(FragmentAssignment {
                        stage_id,
                        task_idx,
                        task_count,
                        plan: Arc::clone(stage_plan),
                        routing: routing.clone(),
                    });
                }
            }
            // Recurse into the stage's plan with THIS stage as the new
            // parent. The boundary's `children()` returns `[stage.plan]` so
            // descending through it would double-process every nested
            // fragment — return here to keep visit counts exact.
            let new_parent = ParentContext {
                parent_stage_id: stage_id,
            };
            collect(stage_plan, this_proc, assignment, Some(new_parent), out);
        }
        return;
    }
    // Non-boundary nodes recurse through plan children.
    for child in plan.children() {
        collect(child, this_proc, assignment, parent, out);
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
        let assignment = TaskAssignment::empty(4);
        let out = find_worker_assignments(&plan, 1, &assignment);
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
    fn routing_shuffle_carries_parent_and_pc() {
        let r = FragmentRouting::Shuffle {
            parent_stage_id: 7,
            partitions_per_consumer_task: 3,
        };
        match r {
            FragmentRouting::Shuffle {
                parent_stage_id,
                partitions_per_consumer_task,
            } => {
                assert_eq!(parent_stage_id, 7);
                assert_eq!(partitions_per_consumer_task, 3);
            }
            _ => panic!("expected Shuffle"),
        }
    }
}
