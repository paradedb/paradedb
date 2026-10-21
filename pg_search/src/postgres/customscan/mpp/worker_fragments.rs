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
//! Destinations are the consumer stage's own (outer) task numbers, the ones `proc_for_task`
//! places. A boundary only knows the numbering of the plan fragment it sits in, which under a
//! [`ChildrenIsolatorUnionExec`] is the child-local numbering the union hands each child,
//! restarting at zero per child. [`ConsumerPlacement`] carries the inner-to-outer translation
//! down the consumer plan so a partition is addressed to the outer task that will read it.
//!
//! - **Top-level boundary**: the consumer is the leader at proc 0. Every output partition goes
//!   there.
//! - **Nested [`NetworkShuffleExec`] / [`NetworkBroadcastExec`]**: consumers read partition slices
//!   (`route_partition(q)` names the inner consumer task), identical across producer tasks, so
//!   the destination depends on the output partition only.
//! - **Nested [`NetworkCoalesceExec`]**: consumers read whole producer tasks in contiguous
//!   groups, so the destination depends on the producer task only.
//!
//! [`NetworkShuffleExec`]: datafusion_distributed::NetworkShuffleExec
//! [`NetworkBroadcastExec`]: datafusion_distributed::NetworkBroadcastExec
//! [`NetworkCoalesceExec`]: datafusion_distributed::NetworkCoalesceExec

use std::sync::Arc;

use datafusion::common::{DataFusionError, Result};
use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::{
    ChildrenIsolatorUnionExec, NetworkBoundaryExt, NetworkBroadcastExec, NetworkCoalesceExec,
    NetworkShuffleExec, coalesce_consumer_task,
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
    /// Task index within the stage (0..task_count), validated for the shared-memory transport.
    pub task_idx: u32,
    /// Total task count for this stage (= `input_stage.tasks.len()`).
    pub task_count: usize,
    /// How to route each output partition to a destination proc.
    pub routing: FragmentRouting,
}

/// Routing rule for a fragment's output partitions. Destinations are outer task indexes of the
/// consumer stage; the worker maps them to procs with `proc_for_task`, so the rule stays valid
/// when PostgreSQL attaches fewer workers than requested.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FragmentRouting {
    /// Top-level boundary: every output partition goes to the leader at proc 0. Also the shape
    /// of a top-level shuffle or broadcast, whose consumer pipeline collapsed onto the leader.
    Leader,
    /// Nested [`NetworkCoalesceExec`]: `outer_task[t]` is the consumer task that reads every
    /// output partition of producer task `t`.
    ///
    /// [`NetworkCoalesceExec`]: datafusion_distributed::NetworkCoalesceExec
    PerTask { outer_task: Vec<u32> },
    /// Nested [`NetworkShuffleExec`] / [`NetworkBroadcastExec`]: `outer_task[q]` is the consumer
    /// task that reads output partition `q`, from every producer task.
    ///
    /// [`NetworkShuffleExec`]: datafusion_distributed::NetworkShuffleExec
    /// [`NetworkBroadcastExec`]: datafusion_distributed::NetworkBroadcastExec
    PerPartition { outer_task: Vec<u32> },
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

/// Who reads a boundary: the leader, or a plan fragment of an enclosing stage.
#[derive(Clone)]
enum Consumer {
    /// The boundary emits into the leader's plan (`DistributedExec` and above), one task at
    /// proc 0. A union in the leader's plan runs every child in that single task, so the
    /// placement never changes below it.
    Leader,
    /// The boundary sits in a stage's plan; its readers are tasks of that stage.
    Stage(ConsumerPlacement),
}

/// The task numbering in force at one point of a consumer stage's plan, and how it maps back to
/// the stage's own tasks: identity at the stage root, composed through each
/// [`ChildrenIsolatorUnionExec`] on the way down.
#[derive(Clone)]
struct ConsumerPlacement {
    /// Inner task index at this point of the plan to outer task of the stage. Its length is the
    /// inner task count.
    inner_to_outer: Arc<[u32]>,
}

impl ConsumerPlacement {
    fn identity(task_count: usize) -> Self {
        Self {
            inner_to_outer: (0..task_count as u32).collect(),
        }
    }

    fn task_count(&self) -> usize {
        self.inner_to_outer.len()
    }

    /// The placement of `child` under `union`, which itself sits at `self`.
    fn through_union(&self, union: &ChildrenIsolatorUnionExec, child: usize) -> Result<Self> {
        let pairs: Vec<_> = union.outer_tasks_for_child(child).collect();
        let Some((_, first)) = pairs.first() else {
            return Err(DataFusionError::Internal(format!(
                "mpp worker_fragments: union child {child} is not executed by any task"
            )));
        };
        let inner_count = first.task_count;
        let mut inner_to_outer = vec![None; inner_count];
        for (outer, inner) in pairs {
            if inner.task_count != inner_count {
                return Err(DataFusionError::Internal(format!(
                    "mpp worker_fragments: union child {child} runs with inner task counts \
                     {inner_count} and {}",
                    inner.task_count
                )));
            }
            let outer = self.outer_for_inner(outer, "union outer task")?;
            let slot = inner_to_outer.get_mut(inner.task_index).ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "mpp worker_fragments: union child {child} inner task {} >= inner task \
                     count {inner_count}",
                    inner.task_index
                ))
            })?;
            if slot.replace(outer).is_some() {
                return Err(DataFusionError::Internal(format!(
                    "mpp worker_fragments: union child {child} inner task {} is executed by \
                     more than one outer task",
                    inner.task_index
                )));
            }
        }
        let inner_to_outer = inner_to_outer
            .into_iter()
            .enumerate()
            .map(|(inner, outer)| {
                outer.ok_or_else(|| {
                    DataFusionError::Internal(format!(
                        "mpp worker_fragments: union child {child} inner task {inner} is \
                         executed by no outer task"
                    ))
                })
            })
            .collect::<Result<Arc<[u32]>>>()?;
        Ok(Self { inner_to_outer })
    }

    /// The outer task that executes inner task `inner` of this placement.
    fn outer_for_inner(&self, inner: usize, what: &str) -> Result<u32> {
        self.inner_to_outer.get(inner).copied().ok_or_else(|| {
            DataFusionError::Internal(format!(
                "mpp worker_fragments: {what} {inner} >= consumer task count {}",
                self.task_count()
            ))
        })
    }
}

impl Consumer {
    fn through_union(&self, union: &ChildrenIsolatorUnionExec, child: usize) -> Result<Self> {
        match self {
            Consumer::Leader => Ok(Consumer::Leader),
            Consumer::Stage(placement) => {
                Ok(Consumer::Stage(placement.through_union(union, child)?))
            }
        }
    }
}

/// One producer stage: everything derivable from the plan alone, before any worker count exists.
pub(crate) struct DiscoveredStage {
    boundary: Arc<dyn ExecutionPlan>,
    stage_num: u32,
    task_count: usize,
    /// Who reads this stage's output, in the consumer stage's own task numbering.
    consumer: Consumer,
    /// Stages with no local plan are neither counted nor dispatched, but their routing is still
    /// classified so unroutable shapes are rejected.
    dispatchable: bool,
}

/// Every producer stage, once per boundary. The launch walks before forking and hands the result
/// to both [`max_producer_task_count`] and [`classify_stages`], so the two cannot disagree about
/// which stages exist (#5667). [`classify_stages`] derives logical task routing from this list;
/// worker ownership remains deferred until each worker applies the actual attached width.
///
/// Fails when a union's task map cannot be composed, which is a planner invariant breach, not a
/// plan shape to fall back from.
pub(crate) fn collect_stages(root: &Arc<dyn ExecutionPlan>) -> Result<Vec<DiscoveredStage>> {
    let mut out = Vec::new();
    walk_stages(root, &Consumer::Leader, &mut out)?;
    Ok(out)
}

fn walk_stages(
    plan: &Arc<dyn ExecutionPlan>,
    consumer: &Consumer,
    out: &mut Vec<DiscoveredStage>,
) -> Result<()> {
    if let Some(nb) = plan.as_ref().as_network_boundary() {
        let stage = nb.input_stage();
        let local_plan = stage.local_plan();
        out.push(DiscoveredStage {
            boundary: Arc::clone(plan),
            stage_num: stage.num() as u32,
            task_count: stage.task_count(),
            consumer: consumer.clone(),
            dispatchable: local_plan.is_some(),
        });
        if let Some(stage_plan) = local_plan {
            // Recurse into the stage's plan: its boundaries are read by this stage's tasks. The
            // boundary's `children()` returns `[stage.plan]`, so descending through it would
            // double-process every nested stage. Return here to keep visit counts exact.
            let inner = Consumer::Stage(ConsumerPlacement::identity(stage.task_count()));
            walk_stages(stage_plan, &inner, out)?;
        }
        return Ok(());
    }
    if let Some(union) = plan.downcast_ref::<ChildrenIsolatorUnionExec>() {
        for (child_idx, child) in plan.children().into_iter().enumerate() {
            let placement = consumer.through_union(union, child_idx)?;
            walk_stages(child, &placement, out)?;
        }
        return Ok(());
    }
    // Non-boundary nodes recurse through plan children.
    for child in plan.children() {
        walk_stages(child, consumer, out)?;
    }
    Ok(())
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

/// True when discovered stages have enough producer tasks for MPP to launch (#5784).
/// Same `>= 2` gate as the plan-first path in [`crate::postgres::customscan::mpp::launch`]:
/// below that there is no data parallelism, so execution runs serially and plain EXPLAIN
/// must render the serial shape rather than a cap-sized distributed plan.
pub(crate) fn stages_have_data_parallelism(stages: &[DiscoveredStage]) -> bool {
    max_producer_task_count(stages) >= 2
}

/// Convenience wrapper over [`stages_have_data_parallelism`] for a finished physical plan.
pub(crate) fn mpp_plan_has_data_parallelism(plan: &Arc<dyn ExecutionPlan>) -> bool {
    match collect_stages(plan) {
        Ok(stages) => stages_have_data_parallelism(&stages),
        Err(e) => crate::postgres::customscan::mpp::fail_loud(format!(
            "mpp worker_fragments: stage discovery failed: {e}"
        )),
    }
}

/// Classify each discovered stage's routing from the physical-plan boundary type. The routing blob
/// contains task IDs, not worker ownership, so it remains valid when PostgreSQL attaches fewer
/// workers than requested. Not filtered by proc; each worker later selects its own `(stage, task)`
/// slots with the actual attached-worker count.
pub(crate) fn classify_stages(stages: &[DiscoveredStage]) -> Result<Vec<StageEntry>> {
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

fn classify_routing(discovered: &DiscoveredStage) -> Result<FragmentRouting> {
    let plan = &discovered.boundary;
    let stage_id = discovered.stage_num;
    let Some(nb) = plan.as_ref().as_network_boundary() else {
        // `collect_stages` only records nodes that matched `as_network_boundary`.
        crate::postgres::customscan::mpp::fail_loud(format!(
            "mpp worker_fragments: {} is not a network boundary (stage_id={stage_id}).",
            plan.name()
        ))
    };
    let stage = nb.input_stage();
    let is_coalesce = plan.is::<NetworkCoalesceExec>();
    if !is_coalesce && !plan.is::<NetworkShuffleExec>() && !plan.is::<NetworkBroadcastExec>() {
        // `as_network_boundary()` matched, but the node isn't one of the three concrete
        // boundary types we route. Fail loudly rather than guess; a default destination would
        // silently produce wrong answers under a shape we haven't seen.
        crate::postgres::customscan::mpp::fail_loud(format!(
            "mpp worker_fragments: unrecognized network boundary {} (stage_id={stage_id}). \
             Add a routing arm before bumping the fork rev.",
            plan.name()
        ))
    }
    // The fork's gRPC path keys dispatch on resolver URLs and never has to decide this; our
    // shm_mq peers are push-driven without URLs, so the dispatcher has to.
    let routing = match &discovered.consumer {
        // Top-level: the consumer is the leader at proc 0, whatever the boundary type. A
        // top-level shuffle or broadcast means the consumer pipeline collapsed to a single
        // partition on the leader, so it behaves identically to a coalesce.
        Consumer::Leader => FragmentRouting::Leader,
        Consumer::Stage(placement) if is_coalesce => {
            let outer_task = (0..discovered.task_count)
                .map(|t| {
                    let inner =
                        coalesce_consumer_task(discovered.task_count, placement.task_count(), t)
                            .ok_or_else(|| {
                                DataFusionError::Internal(format!(
                                    "mpp worker_fragments: coalesce producer task {t} of {} has no \
                             consumer among {} tasks (stage_id={stage_id})",
                                    discovered.task_count,
                                    placement.task_count()
                                ))
                            })?;
                    placement.outer_for_inner(inner, "coalesce consumer task")
                })
                .collect::<Result<Vec<u32>>>()?;
            FragmentRouting::PerTask { outer_task }
        }
        // The crate owns the receive-side slice layout: read it from `route_partition` rather
        // than re-deriving it.
        Consumer::Stage(placement) => {
            let n_out = stage
                .local_plan()
                .map_or(0, |p| p.properties().partitioning.partition_count());
            let outer_task = (0..n_out)
                .map(|q| {
                    let inner = nb.route_partition(q)?.consumer_task;
                    placement.outer_for_inner(inner, "route_partition consumer task")
                })
                .collect::<Result<Vec<u32>>>()?;
            FragmentRouting::PerPartition { outer_task }
        }
    };
    crate::mpp_log!(
        "mpp worker_fragments::classify_routing boundary={} stage_id={stage_id} routing={routing:?}",
        plan.name()
    );
    Ok(routing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::array::{Int32Array, StringArray};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::arrow::record_batch::RecordBatch;
    use datafusion::common::ScalarValue;
    use datafusion::datasource::MemTable;
    use datafusion::datasource::provider_as_source;
    use datafusion::execution::SessionStateBuilder;
    use datafusion::functions_aggregate::count::count_all;
    use datafusion::logical_expr::{LogicalPlan, LogicalPlanBuilder, col, lit};
    use datafusion::physical_plan::empty::EmptyExec;
    use datafusion::prelude::SessionConfig;
    use datafusion_distributed::shm::{InProcessWorkerResolver, proc_for_task};
    use datafusion_distributed::{DistributedConfig, DistributedExt, SessionStateBuilderExt};

    #[test]
    fn boundary_free_plan_yields_no_stages() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        let plan: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(schema));
        let out = classify_stages(&collect_stages(&plan).unwrap()).unwrap();
        assert!(out.is_empty());
    }

    /// #5667: sizing and dispatch read the same enumeration, so a boundary-free plan has nothing
    /// to distribute and the launch spawns no workers at all.
    #[test]
    fn boundary_free_plan_has_zero_max_tasks() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        let plan: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(schema));
        assert_eq!(max_producer_task_count(&collect_stages(&plan).unwrap()), 0);
        assert!(!mpp_plan_has_data_parallelism(&plan));
    }

    /// Nested coalesce destinations are independent of the attached width: the consumer's task 0
    /// lands on the first producer proc however many workers attached. This invariant is why the
    /// dispatch payload can be built before worker launch and reused unchanged after a viable
    /// short launch.
    #[test]
    fn nested_coalesce_destination_is_width_independent() {
        for worker_count in 1..=8 {
            assert_eq!(proc_for_task(worker_count, 0), 1);
        }
    }

    fn mem_table(name: &str, partitions: usize) -> Arc<MemTable> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("k", DataType::Utf8, false),
            Field::new("v", DataType::Int32, false),
        ]));
        let batches: Vec<Vec<RecordBatch>> = (0..partitions)
            .map(|p| {
                vec![
                    RecordBatch::try_new(
                        Arc::clone(&schema),
                        vec![
                            Arc::new(StringArray::from(vec![format!("{name}{p}")])),
                            Arc::new(Int32Array::from(vec![p as i32])),
                        ],
                    )
                    .unwrap(),
                ]
            })
            .collect();
        Arc::new(MemTable::try_new(schema, batches).unwrap())
    }

    /// Plan `a UNION ALL b`, each side a GROUP BY whose final aggregate reads a hash shuffle,
    /// through the fork's distributed planner with three workers: the shape the aggregate scan
    /// builds for a scalar facet beside an array facet. The sides group by different columns,
    /// as the facets do, so the union stays a union instead of folding into an interleave of
    /// co-partitioned inputs. The union isolator splits the top stage's three tasks between the
    /// two children, so the second child's shuffle is read by an outer task that is not its
    /// inner task 0.
    fn plan_isolated_union() -> Arc<dyn ExecutionPlan> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime.block_on(async {
            let state = SessionStateBuilder::new()
                .with_default_features()
                .with_config(SessionConfig::new().with_target_partitions(3))
                .with_distributed_option_extension(DistributedConfig::default())
                .with_distributed_worker_resolver(InProcessWorkerResolver::new(3))
                .with_distributed_desired_task_count_handler(3usize)
                .with_distributed_planner()
                .build();
            let side = |name: &str, by_k: bool| -> LogicalPlan {
                let table = mem_table(name, 3);
                let (group, projection) = if by_k {
                    (
                        col("k"),
                        vec![
                            col("k"),
                            lit(ScalarValue::Int32(None)).alias("v"),
                            col("count(*)"),
                        ],
                    )
                } else {
                    (
                        col("v"),
                        vec![
                            lit(ScalarValue::Utf8(None)).alias("k"),
                            col("v"),
                            col("count(*)"),
                        ],
                    )
                };
                LogicalPlanBuilder::scan(name, provider_as_source(table), None)
                    .unwrap()
                    .aggregate(vec![group], vec![count_all()])
                    .unwrap()
                    .project(projection)
                    .unwrap()
                    .build()
                    .unwrap()
            };
            let logical = LogicalPlanBuilder::from(side("a", true))
                .union(side("b", false))
                .unwrap()
                .build()
                .unwrap();
            state.create_physical_plan(&logical).await.unwrap()
        })
    }

    fn contains_union(plan: &Arc<dyn ExecutionPlan>) -> bool {
        plan.downcast_ref::<ChildrenIsolatorUnionExec>().is_some()
            || plan.children().into_iter().any(contains_union)
            || plan
                .as_ref()
                .as_network_boundary()
                .and_then(|nb| nb.input_stage().local_plan().cloned())
                .is_some_and(|p| contains_union(&p))
    }

    /// Every nested boundary under a union isolator routes to the outer tasks that execute its
    /// child, not to the child-local task numbers the boundary itself reports. With two equal
    /// children and three tasks the isolator assigns `t0:[c0(0/2)] t1:[c0(1/2)] t2:[c1]`, so
    /// the second child's partitions must all go to outer task 2 although `route_partition`
    /// names task 0 for each of them.
    #[test]
    fn union_children_route_to_their_outer_tasks() {
        let physical = plan_isolated_union();
        let display = datafusion_distributed::display_plan_ascii(physical.as_ref(), false);
        assert!(
            contains_union(&physical),
            "planner did not isolate the union:\n{display}"
        );
        let stages = collect_stages(&physical).unwrap();
        let nested: Vec<&DiscoveredStage> = stages
            .iter()
            .filter(|s| matches!(s.consumer, Consumer::Stage(_)))
            .collect();
        assert_eq!(
            nested.len(),
            2,
            "expected one nested shuffle per union child:\n{display}"
        );

        let mut per_child: Vec<(Vec<u32>, Vec<u32>)> = Vec::new();
        for stage in &nested {
            let nb = stage.boundary.as_ref().as_network_boundary().unwrap();
            let n_out = nb
                .input_stage()
                .local_plan()
                .unwrap()
                .properties()
                .partitioning
                .partition_count();
            let child_local: Vec<u32> = (0..n_out)
                .map(|q| nb.route_partition(q).unwrap().consumer_task as u32)
                .collect();
            let FragmentRouting::PerPartition { outer_task } = classify_routing(stage).unwrap()
            else {
                panic!("nested shuffle must route per partition");
            };
            assert_eq!(outer_task.len(), n_out);
            per_child.push((child_local, outer_task));
        }

        // The single-task child is the misroute shape: every partition says inner 0, but the
        // outer task the isolator gave that child is not 0.
        let (two_task, one_task): (Vec<_>, Vec<_>) = per_child
            .into_iter()
            .partition(|(local, _)| local.iter().any(|&t| t != 0));
        assert_eq!(two_task.len(), 1);
        assert_eq!(one_task.len(), 1);
        let (local, outer) = &one_task[0];
        assert!(local.iter().all(|&t| t == 0));
        assert!(
            outer.iter().all(|&t| t == outer[0]) && outer[0] != 0,
            "single-task child must route to one non-zero outer task, got {outer:?}"
        );
        let (local, outer) = &two_task[0];
        let mut outer_set = outer.clone();
        outer_set.sort_unstable();
        outer_set.dedup();
        assert_eq!(outer_set.len(), 2);
        assert!(!outer_set.contains(&one_task[0].1[0]));
        for (q, &inner) in local.iter().enumerate() {
            assert_eq!(
                outer[q],
                outer[local.iter().position(|&t| t == inner).unwrap()]
            );
        }
        let top: Vec<_> = stages
            .iter()
            .filter(|s| matches!(s.consumer, Consumer::Leader))
            .collect();
        assert_eq!(top.len(), 1);
        assert_eq!(classify_routing(top[0]).unwrap(), FragmentRouting::Leader);
    }
}
