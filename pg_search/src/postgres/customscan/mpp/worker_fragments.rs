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
//! [`collect_stages`] walks the distributed physical plan and visits every
//! [`datafusion_distributed::NetworkBoundary`]; [`classify_stages`] turns each into one
//! [`StageEntry`] (`input_stage.num`, `task_count`, `routing`) from the unchanged physical plan.
//! The stage plans travel separately, serialized by the coordinator's dispatch; each worker later
//! expands a stage into one [`FragmentAssignment`] per `task_idx` it owns under `proc_for_task`.
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
use datafusion_distributed::{
    NetworkBoundaryExt, NetworkBroadcastExec, NetworkCoalesceExec, NetworkShuffleExec,
};

/// Proc 0 is the leader. Task 0 always belongs to the first producer proc, independent of how
/// many producers PostgreSQL actually attached.
const FIRST_WORKER_PROC: u32 = 1;

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
    /// Task index within the stage (0..task_count), validated for the shared-memory transport.
    pub task_idx: u32,
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
        /// Destination proc index. `0` for the leader (top-level gather), or
        /// [`FIRST_WORKER_PROC`] for a nested coalesce that lands on consumer task 0.
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
    },
}

/// One producer stage to dispatch: the routing metadata the blob carries. Each worker expands a
/// stage into one [`FragmentAssignment`] per `task_idx` it owns under `proc_for_task`.
pub struct StageEntry {
    /// `input_stage.num` of the boundary whose producer side this stage belongs to.
    pub stage_num: u32,
    /// Total task count for the stage (= `input_stage.tasks.len()`).
    pub task_count: usize,
    /// How to route each output partition to a destination proc.
    pub routing: FragmentRouting,
}

/// One producer stage: everything derivable from the plan alone, before any worker count exists.
pub(crate) struct DiscoveredStage {
    boundary: Arc<dyn ExecutionPlan>,
    stage_num: u32,
    task_count: usize,
    /// `false` for a boundary that emits into the leader, `true` for one nested under a parent
    /// stage.
    nested: bool,
    /// Stages with no local plan are neither counted nor dispatched, but their routing is still
    /// classified so unroutable shapes are rejected.
    dispatchable: bool,
}

/// Every producer stage, once per boundary. The launch walks before forking and hands the result
/// to both [`max_producer_task_count`] and [`classify_stages`], so the two cannot disagree about
/// which stages exist (#5667). [`classify_stages`] derives logical task routing from this list;
/// worker ownership remains deferred until each worker applies the actual attached width.
pub(crate) fn collect_stages(root: &Arc<dyn ExecutionPlan>) -> Vec<DiscoveredStage> {
    let mut out = Vec::new();
    walk_stages(root, /* nested = */ false, &mut out);
    out
}

fn walk_stages(plan: &Arc<dyn ExecutionPlan>, nested: bool, out: &mut Vec<DiscoveredStage>) {
    if let Some(nb) = plan.as_ref().as_network_boundary() {
        let stage = nb.input_stage();
        let local_plan = stage.local_plan();
        out.push(DiscoveredStage {
            boundary: Arc::clone(plan),
            stage_num: stage.num() as u32,
            task_count: stage.task_count(),
            nested,
            dispatchable: local_plan.is_some(),
        });
        if let Some(stage_plan) = local_plan {
            // Recurse into the stage's plan with `nested = true`. The boundary's `children()`
            // returns `[stage.plan]`, so descending through it would double-process every nested
            // stage. Return here to keep visit counts exact.
            walk_stages(stage_plan, true, out);
        }
        return;
    }
    // Non-boundary nodes recurse through plan children.
    for child in plan.children() {
        walk_stages(child, nested, out);
    }
}

/// Largest producer-stage task count, or 0 when the plan has no network boundaries (#5667). The
/// plan-first launch sizes the worker pool from this number before any DSM or process exists:
/// `producers = clamp(max_tasks, 2, cap)`.
pub(crate) fn max_producer_task_count(stages: &[DiscoveredStage]) -> usize {
    stages
        .iter()
        .filter(|stage| stage.dispatchable)
        .map(|stage| stage.task_count)
        .max()
        .unwrap_or(0)
}

/// Classify each discovered stage's routing from the physical-plan boundary type. The routing blob
/// contains task IDs, not worker ownership, so it remains valid when PostgreSQL attaches fewer
/// workers than requested. Not filtered by proc; each worker later selects its own `(stage, task)`
/// slots with the actual attached-worker count.
pub(crate) fn classify_stages(
    stages: &[DiscoveredStage],
) -> Result<Vec<StageEntry>, DataFusionError> {
    let mut out = Vec::new();
    for stage in stages {
        // Classified before the `dispatchable` check so an unroutable shape is rejected whether or
        // not it carries a local plan.
        let routing = classify_routing(stage)?;
        if stage.dispatchable {
            out.push(StageEntry {
                stage_num: stage.stage_num,
                task_count: stage.task_count,
                routing,
            });
        }
    }
    Ok(out)
}

fn classify_routing(discovered: &DiscoveredStage) -> Result<FragmentRouting, DataFusionError> {
    let plan = &discovered.boundary;
    let stage_id = discovered.stage_num;
    let nested = discovered.nested;
    let Some(nb) = plan.as_ref().as_network_boundary() else {
        // `collect_stages` only records nodes that matched `as_network_boundary`.
        crate::postgres::customscan::mpp::fail_loud(format!(
            "mpp worker_fragments: {} is not a network boundary (stage_id={stage_id}).",
            plan.name()
        ))
    };
    {
        let stage = nb.input_stage();
        // Only the `mpp_log!` trace reads `p_c` (routing reads the crate's `route_partition`),
        // so it's gated to non-test builds to avoid the unused-variable warning.
        #[cfg(not(test))]
        let p_c = nb.properties().partitioning.partition_count();
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
        // receive-side math; Coalesce collapses to one consumer task;
        // top-level (`nested == false`) routes to the leader.
        let routing = if plan.is::<NetworkCoalesceExec>() {
            if nested {
                // Nested NetworkCoalesceExec: consumer is task 0 of the parent stage.
                FragmentRouting::Coalesce {
                    dest_proc: FIRST_WORKER_PROC,
                }
            } else {
                // Top-level NetworkCoalesceExec (gather to leader): consumer is leader proc 0.
                FragmentRouting::Coalesce { dest_proc: 0 }
            }
        } else if plan.is::<NetworkShuffleExec>() || plan.is::<NetworkBroadcastExec>() {
            if nested {
                // Nested NetworkShuffleExec or NetworkBroadcastExec: hash-partitioned mesh.
                // Each output partition q maps to the consumer task `route_partition(q)` selects.
                FragmentRouting::Hashed {
                    consumer_task: route_consumer_tasks()?,
                }
            } else {
                // Top-level NetworkShuffleExec / NetworkBroadcastExec means the entire consumer
                // pipeline collapsed to a single partition and remained on the leader instead of
                // becoming a nested consumer stage. In this case, the single leader task is the
                // only consumer, so it behaves identically to a Coalesce.
                FragmentRouting::Coalesce { dest_proc: 0 }
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
                "mpp worker_fragments::classify_routing boundary={} stage_id={stage_id} \
                 p_c={p_c} nested={nested}",
                plan.name()
            );
        }

        Ok(routing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::physical_plan::empty::EmptyExec;
    use datafusion_distributed::shm::proc_for_task;

    #[test]
    fn boundary_free_plan_yields_no_stages() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        let plan: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(schema));
        let out = classify_stages(&collect_stages(&plan)).unwrap();
        assert!(out.is_empty());
    }

    /// #5667: sizing and dispatch read the same enumeration, so a boundary-free plan has nothing
    /// to distribute and the launch spawns no workers at all.
    #[test]
    fn boundary_free_plan_has_zero_max_tasks() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        let plan: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(schema));
        assert_eq!(max_producer_task_count(&collect_stages(&plan)), 0);
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
        };
        match r {
            FragmentRouting::Hashed { consumer_task } => {
                assert_eq!(consumer_task, vec![0, 0, 1]);
            }
            _ => panic!("expected Hashed"),
        }
    }

    #[test]
    fn nested_coalesce_destination_is_width_independent() {
        // This invariant is why the dispatch payload can be built before worker launch and reused
        // unchanged after a viable short launch.
        for worker_count in 1..=8 {
            assert_eq!(proc_for_task(worker_count, 0), FIRST_WORKER_PROC);
        }
    }
}
