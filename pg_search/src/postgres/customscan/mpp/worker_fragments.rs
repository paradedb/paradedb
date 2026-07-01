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

//! Leader-side producer-stage discovery for dispatch. "Fragment" here means a plan fragment
//! (one task of a producer stage), not the frame fragmentation the ring does for oversized
//! messages.
//!
//! [`collect_dispatched_stages`] walks the distributed physical plan, visits every
//! [`datafusion_distributed::NetworkBoundary`], and collects one [`StageEntry`]
//! (`input_stage.num`, `task_count`, `routing`, `plan`) per boundary. The leader serializes each
//! stage's plan and ships it; each worker later expands a stage into one [`FragmentAssignment`]
//! per `task_idx` it owns under `proc_for_task`.
//!
//! The fork's coordinator has no equivalent of this walk: it dispatches one boundary at a time,
//! when the consumer's `execute` opens connections, so routing is implicit in who pulls. These
//! workers launch exactly once and the mesh is push-driven, so the leader enumerates every
//! producer stage and precomputes destinations before any worker exists.
//!
//! Routing classification (which proc an output partition `q` is sent to) depends on the
//! boundary's position:
//!
//! - **Top-level boundary** (`nested = false`): the consumer is the leader at
//!   proc 0. Every output partition goes there.
//! - **Nested boundary inside an outer stage**: the consumer is one of that stage's
//!   tasks. For [`NetworkShuffleExec`] the routing is hash-partitioned (partition `q` →
//!   the consumer task `route_partition(q)` picks); for [`NetworkCoalesceExec`] the
//!   routing collapses to a single consumer task.
//!
//! [`NetworkShuffleExec`]: datafusion_distributed::NetworkShuffleExec
//! [`NetworkCoalesceExec`]: datafusion_distributed::NetworkCoalesceExec

use std::sync::Arc;

use datafusion::common::DataFusionError;
use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::shm::proc_for_task;
use datafusion_distributed::{
    NetworkBoundaryExt, NetworkBroadcastExec, NetworkCoalesceExec, NetworkShuffleExec,
};

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
    /// How to route each output partition to a destination proc.
    pub routing: FragmentRouting,
}

/// Routing rule for a fragment's output partitions.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum FragmentRouting {
    /// All output partitions go to one destination proc (`NetworkCoalesceExec`
    /// or the top-level gather case). Coalesce routes by producer task, not by
    /// output partition, so the crate's `route_partition` does not describe it;
    /// the dispatcher sends every partition to `dest_proc`.
    Coalesce {
        /// Destination proc index. `0` for the leader (top-level gather), or a
        /// `proc_for_task(n_workers, 0)` lookup for a nested coalesce that lands
        /// on a single consumer task.
        dest_proc: u32,
    },
    /// Hash-partitioned mesh ([`NetworkShuffleExec`] / [`NetworkBroadcastExec`]). Output
    /// partition `q` goes to the consumer task the crate's `route_partition(q)` selects,
    /// hosted on `proc_for_task(n_workers, consumer_task)`. Precomputed per output partition:
    /// the crate owns the receive-side formula, so the producer side reads it from
    /// `route_partition` rather than re-deriving `q / P_c`.
    ///
    /// [`NetworkShuffleExec`]: datafusion_distributed::NetworkShuffleExec
    /// [`NetworkBroadcastExec`]: datafusion_distributed::NetworkBroadcastExec
    Hashed {
        /// `route_partition(q).consumer_task` for each producer output partition `q`.
        consumer_task: Vec<u32>,
        /// `true` for broadcast. The build subtree is capped at `task_count = 1` via
        /// [`BroadcastBuildSideOneTaskEstimator`], so the dispatcher only ever sees
        /// `task_idx == 0` broadcast fragments; the cap is asserted at dispatch.
        ///
        /// [`BroadcastBuildSideOneTaskEstimator`]: crate::postgres::customscan::mpp::task_estimator::BroadcastBuildSideOneTaskEstimator
        broadcast: bool,
    },
}

/// One producer stage to dispatch. The leader serializes `plan` once per stage and ships it with
/// its `task_count` and `routing`; each worker expands a stage into one [`FragmentAssignment`] per
/// `task_idx` it owns under `proc_for_task`.
pub struct StageEntry {
    /// `input_stage.num` of the boundary whose producer side this stage belongs to.
    pub stage_num: u32,
    /// Total task count for the stage (= `input_stage.tasks.len()`).
    pub task_count: usize,
    /// How to route each output partition to a destination proc.
    pub routing: FragmentRouting,
    /// The stage's `input_stage.plan` (nested boundaries left `Local`; the worker converts them
    /// at run time via `prepare_in_process_plan`, same as before).
    pub plan: Arc<dyn ExecutionPlan>,
}

/// Walk the distributed physical plan and collect every producer stage, once per boundary. The
/// leader runs this (replacing the worker-side re-plan): it classifies routing from the boundary
/// type and captures each stage's `local_plan` for serialization. Not filtered by proc; the blob
/// is shared and each worker selects its own `(stage, task)` slots.
pub fn collect_dispatched_stages(
    root: &Arc<dyn ExecutionPlan>,
    n_workers: u32,
) -> Result<Vec<StageEntry>, DataFusionError> {
    let mut out = Vec::new();
    collect_stages(root, n_workers, /* nested = */ false, &mut out)?;
    Ok(out)
}

fn collect_stages(
    plan: &Arc<dyn ExecutionPlan>,
    n_workers: u32,
    nested: bool,
    out: &mut Vec<StageEntry>,
) -> Result<(), DataFusionError> {
    if let Some(nb) = plan.as_ref().as_network_boundary() {
        let stage = nb.input_stage();
        let stage_id = stage.num() as u32;
        // Only the `mpp_log!` trace reads `p_c` (routing reads the crate's `route_partition`),
        // so it's gated to non-test builds to avoid the unused-variable warning.
        #[cfg(not(test))]
        let p_c = nb.partitions_per_consumer_task();
        // `route_partition(q).consumer_task` for every producer output partition. Used by the
        // hash-partitioned boundaries (Shuffle / Broadcast) only; Coalesce routes by task instead.
        let route_consumer_tasks = || -> Result<Vec<u32>, DataFusionError> {
            let n_out = stage
                .local_plan()
                .map_or(0, |p| p.properties().partitioning.partition_count());
            (0..n_out)
                .map(|q| Ok(nb.route_partition(q)?.consumer_task as u32))
                .collect()
        };
        // Classify the boundary by downcasting to its concrete `Network*Exec` type, then pick a
        // destination proc for every output partition from `(type, top_level)`. The fork's gRPC
        // path keys dispatch on resolver URLs and never has to decide this; our shm_mq peers are
        // push-driven without URLs, so the dispatcher has to. Shuffle and Broadcast share the
        // receive-side math but Broadcast caps to task 0; Coalesce collapses to one consumer task;
        // top-level (`nested == false`) routes to the leader.
        let routing = if plan.is::<NetworkCoalesceExec>() {
            if nested {
                // Nested NetworkCoalesceExec: consumer is a single task in the parent stage. The
                // receive math collapses to task 0 of the parent group, so the destination proc
                // is `proc_for_task(n_workers, 0)`.
                FragmentRouting::Coalesce {
                    dest_proc: proc_for_task(n_workers, 0),
                }
            } else {
                // Top-level NetworkCoalesceExec (gather to leader): consumer is leader proc 0.
                FragmentRouting::Coalesce { dest_proc: 0 }
            }
        } else if plan.is::<NetworkShuffleExec>() {
            if nested {
                // Nested NetworkShuffleExec: hash-partitioned mesh. Each output partition q maps
                // to the consumer task `route_partition(q)` selects.
                FragmentRouting::Hashed {
                    consumer_task: route_consumer_tasks()?,
                    broadcast: false,
                }
            } else {
                // Top-level NetworkShuffleExec isn't a shape our customscan plans produce.
                // Shuffles emit hash-partitioned output into a parent consumer stage, not directly
                // into the leader. Coalescing the partitions to proc 0 would technically work (each
                // batch reaches `select_all` exactly once) but it would mask a planner anomaly by
                // silently treating hash-partitioned output as one logical stream.
                crate::postgres::customscan::mpp::fail_loud(format!(
                    "mpp worker_fragments: top-level NetworkShuffleExec is unsupported \
                     (stage_id={stage_id}). Shuffles emit hash-partitioned output into a \
                     parent consumer stage; a top-level shuffle is a planner anomaly."
                ))
            }
        } else if plan.is::<NetworkBroadcastExec>() {
            if nested {
                // Nested NetworkBroadcastExec: same receive-side routing as Shuffle (via
                // `route_partition`), but the dispatcher only runs the producer plan on task 0 to
                // avoid the canonical-replica duplication described on `FragmentRouting::Hashed`.
                FragmentRouting::Hashed {
                    consumer_task: route_consumer_tasks()?,
                    broadcast: true,
                }
            } else {
                // Top-level NetworkBroadcastExec isn't a shape the natural-shape AggregateScan
                // plan produces; broadcast always sits nested inside the HashJoin build subtree.
                // Falling through to `Coalesce { dest_proc: 0 }` would send every input task's full
                // canonical replica to the leader and `select_all` would over-count by
                // `input_task_count`. Fail loudly so a future planner change that hits this shape
                // can't silently produce wrong answers.
                crate::postgres::customscan::mpp::fail_loud(format!(
                    "mpp worker_fragments: top-level NetworkBroadcastExec is unsupported \
                     (stage_id={stage_id}). The natural-shape AggregateScan plan does not \
                     produce this shape; route via a NetworkCoalesceExec gather instead."
                ))
            }
        } else {
            // `as_network_boundary()` matched, but the node isn't one of the three concrete
            // boundary types we route. Fail loudly rather than guess; a default destination would
            // silently produce wrong answers under a shape we haven't seen.
            crate::postgres::customscan::mpp::fail_loud(format!(
                "mpp worker_fragments: unrecognized network boundary {} (stage_id={stage_id}). \
                 Add a routing arm before bumping the fork rev.",
                plan.name()
            ))
        };
        #[cfg(not(test))]
        {
            crate::mpp_log!(
                "mpp worker_fragments::collect_stages boundary={} stage_id={stage_id} \
                 p_c={p_c} nested={nested}",
                plan.name()
            );
        }

        let task_count = stage.task_count();
        if let Some(stage_plan) = stage.local_plan() {
            out.push(StageEntry {
                stage_num: stage_id,
                task_count,
                routing,
                plan: Arc::clone(stage_plan),
            });
            // Recurse into the stage's plan with `nested = true`. The boundary's `children()`
            // returns `[stage.plan]`, so descending through it would double-process every nested
            // stage. Return here to keep visit counts exact.
            collect_stages(stage_plan, n_workers, true, out)?;
        }
        return Ok(());
    }
    // Non-boundary nodes recurse through plan children.
    for child in plan.children() {
        collect_stages(child, n_workers, nested, out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::physical_plan::empty::EmptyExec;

    #[test]
    fn boundary_free_plan_yields_no_stages() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        let plan: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(schema));
        let out = collect_dispatched_stages(&plan, 3).unwrap();
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
    fn routing_hashed_carries_consumer_tasks() {
        let r = FragmentRouting::Hashed {
            consumer_task: vec![0, 0, 1],
            broadcast: false,
        };
        match r {
            FragmentRouting::Hashed {
                consumer_task,
                broadcast,
            } => {
                assert_eq!(consumer_task, vec![0, 0, 1]);
                assert!(!broadcast);
            }
            _ => panic!("expected Hashed"),
        }
    }
}
