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

#![allow(dead_code)]
//! MPP walker: rewrites a single-participant physical plan into a leader
//! (consumer) plan + a worker (producer) plan for the coordinator/worker
//! architecture.
//!
//! # Inputs and outputs
//!
//! Input is the physical plan DataFusion produced for the single-participant
//! version of the query — same plan the AggregateScan/JoinScan customscan
//! would execute today on the leader. The walker pattern-matches the plan's
//! shape (see [`super::shape::MppPlanShape`]), then rewrites it into a pair:
//!
//! - **Worker plan** — rooted at [`super::shm_mq_producer::ShmMqProducerExec`].
//!   Drives upstream, hash-routes via a [`super::partitioner::RowPartitioner`],
//!   pushes per-partition sub-batches into the leader's shm_mq queues,
//!   emits zero rows. Customscan runs this on parallel workers (and on the
//!   leader as worker 0 via a Tokio task — see follow-up notes).
//! - **Leader plan** — rooted at the same operator the original plan had
//!   (e.g. `AggregateExec(FinalPartitioned)`), with the cross-worker shuffle
//!   replaced by a [`datafusion_distributed::NetworkShuffleExec`] that the
//!   fork's [`datafusion_distributed::WorkerTransport`] seam hooks into our
//!   [`super::shm_mq_transport::ShmMqWorkerTransport`].
//!
//! # Status
//!
//! Only [`MppPlanShape::ScalarAggOnBinaryJoin`] is wired through to a worker-
//! plan rewrite right now; the leader-plan construction and the other shapes
//! return [`DataFusionError::NotImplemented`]. The customscan integration that
//! invokes this walker, the DSM mesh sizing for the new N×K topology, and the
//! end-to-end activation are follow-up work tracked in the chain summary.

use std::sync::Arc;

use datafusion::arrow::datatypes::SchemaRef;
use datafusion::common::{DataFusionError, Result as DfResult};
use datafusion::physical_plan::aggregates::{AggregateExec, AggregateMode, PhysicalGroupBy};
use datafusion::physical_plan::empty::EmptyExec;
use datafusion::physical_plan::repartition::RepartitionExec;
use datafusion::physical_plan::{ExecutionPlan, Partitioning};
use datafusion_distributed::NetworkShuffleExec;
use uuid::Uuid;

use crate::postgres::customscan::mpp::partitioner::{FixedTargetPartitioner, RowPartitioner};
use crate::postgres::customscan::mpp::shape::MppPlanShape;
use crate::postgres::customscan::mpp::shm_mq_producer::ShmMqProducerExec;

/// Produces a `(leader_plan, worker_plan)` pair for one MPP query.
#[derive(Debug)]
pub struct MppPlanPair {
    /// Plan the leader's customscan executes on the main thread, emitting rows
    /// back to PG. Reads from the cross-worker shm_mq mesh via
    /// [`datafusion_distributed::NetworkShuffleExec`] +
    /// [`super::shm_mq_transport::ShmMqWorkerTransport`].
    pub leader_plan: Arc<dyn ExecutionPlan>,
    /// Plan that every parallel worker executes (and that the leader also
    /// runs as worker 0 on a Tokio task in its own process). Drives upstream,
    /// hash-routes, and pushes batches into the leader's shm_mq queues.
    /// Emits zero rows back to its caller.
    pub worker_plan: Arc<dyn ExecutionPlan>,
}

/// Top-level entry point: dispatch on `shape` and produce the
/// [`MppPlanPair`].
///
/// `n_workers` is the number of producer-side participants (== leader-as-
/// worker-0 + parallel workers). Used to size partitioners and the
/// downstream `NetworkShuffleExec`'s `input_task_count`.
pub fn distribute_plan(
    shape: MppPlanShape,
    physical_plan: Arc<dyn ExecutionPlan>,
    n_workers: u32,
) -> DfResult<MppPlanPair> {
    match shape {
        MppPlanShape::ScalarAggOnBinaryJoin => distribute_scalar_agg(physical_plan, n_workers),
        MppPlanShape::GroupByAggOnBinaryJoin
        | MppPlanShape::TopKGroupByAggOnBinaryJoin
        | MppPlanShape::GroupByAggSingleTable
        | MppPlanShape::JoinOnly => Err(DataFusionError::NotImplemented(format!(
            "mpp: walker dispatch for shape {shape:?} is follow-up work; \
             only ScalarAggOnBinaryJoin lands in this PR"
        ))),
        MppPlanShape::Ineligible => Err(DataFusionError::Plan(
            "mpp: distribute_plan called with Ineligible shape; caller should \
             have routed to the serial path"
                .into(),
        )),
    }
}

/// `ScalarAggOnBinaryJoin` rewrite.
///
/// The standard physical plan looks like (omitting outer wrappers):
///
/// ```text
///     AggregateExec(Final[*])
///       AggregateExec(Partial)
///         HashJoinExec
///           Scan left
///           Scan right
/// ```
///
/// Worker plan: pump the existing tree from `Partial` downward into a
/// [`ShmMqProducerExec`] with a [`FixedTargetPartitioner(0)`] so every partial
/// row routes to the leader's single consumer partition. The Final aggregate
/// is dropped — workers don't need it; the leader runs it after gathering.
///
/// Leader plan: TODO. The leader's plan replaces the original `Partial`-rooted
/// subtree with a `NetworkShuffleExec` that streams partial batches in from
/// the workers, then re-applies the original `Final*` aggregate on top. The
/// construction needs the partial's `aggr_expr` + `filter_expr` + output
/// schema, and a fork-side `Stage::new_unaddressed` to populate the
/// `NetworkShuffleExec` input stage. This is follow-up work; for now we
/// return an error so the customscan falls back to the serial path on the
/// leader's side and the worker plan stays dead at runtime.
fn distribute_scalar_agg(
    physical_plan: Arc<dyn ExecutionPlan>,
    n_workers: u32,
) -> DfResult<MppPlanPair> {
    if n_workers == 0 {
        return Err(DataFusionError::Plan(
            "mpp: distribute_scalar_agg requires n_workers >= 1".into(),
        ));
    }

    let partial_idx = find_partial_aggregate_path(physical_plan.as_ref()).ok_or_else(|| {
        DataFusionError::Plan(
            "mpp: distribute_scalar_agg: no AggregateExec(Partial) found in plan; \
             classifier said ScalarAggOnBinaryJoin but the physical plan does not \
             expose a Partial node — likely a planner-mismatch upstream"
                .into(),
        )
    })?;
    let partial_node = node_at_path(&physical_plan, &partial_idx)
        .expect("path returned by find_partial_aggregate_path is reachable");

    // Worker plan: replace the Partial subtree with a producer wrap.
    let producer_subtree = wrap_with_producer(Arc::clone(&partial_node), n_workers)?;
    let worker_plan = replace_at_path(Arc::clone(&physical_plan), &partial_idx, producer_subtree)?;

    // Leader plan: replace the Partial subtree with NetworkShuffleExec(reads
    // from N workers' single consumer partition) + AggregateExec(FinalPartitioned).
    let partial_agg = partial_node
        .as_any()
        .downcast_ref::<AggregateExec>()
        .expect("partial_node located via find_partial_aggregate_path");
    let leader_subtree = build_leader_finalize(partial_agg, n_workers)?;
    let leader_plan = replace_at_path(physical_plan, &partial_idx, leader_subtree)?;

    Ok(MppPlanPair {
        leader_plan,
        worker_plan,
    })
}

/// Build the leader-side replacement for the `AggregateExec(Partial)`-rooted
/// subtree:
///
/// ```text
///   AggregateExec(FinalPartitioned, group=empty, agg=<original>)
///     NetworkShuffleExec(input_task_count=N, output=Hash([], 1))
///       RepartitionExec(Hash([], 1), EmptyExec(partial_output_schema))
/// ```
///
/// The `EmptyExec` + `RepartitionExec` is a structural placeholder for the
/// fork's `NetworkShuffleExec::try_new` (which validates the input is hash-
/// partitioned). At execute time the fork bypasses the input entirely and
/// pulls record batches through the registered `WorkerTransport` (our
/// `ShmMqWorkerTransport`), so the placeholder is never run.
fn build_leader_finalize(
    partial: &AggregateExec,
    n_workers: u32,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let partial_output_schema: SchemaRef = partial.schema();
    let stub: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(Arc::clone(&partial_output_schema)));
    let hash_partitioned: Arc<dyn ExecutionPlan> = Arc::new(RepartitionExec::try_new(
        stub,
        Partitioning::Hash(vec![], 1),
    )?);
    let network_shuffle =
        NetworkShuffleExec::try_new(hash_partitioned, Uuid::nil(), 0, 1, n_workers as usize)?;
    let network_shuffle: Arc<dyn ExecutionPlan> = Arc::new(network_shuffle);

    // Re-apply the partial's aggregate spec on top, in FinalPartitioned mode.
    // We reuse `partial.group_expr()`, `partial.aggr_expr()`, and
    // `partial.filter_expr()`; the input schema for the Final pass is
    // `partial_output_schema` (= partial's output, which the fork-shuffle
    // streams to us).
    let final_agg = AggregateExec::try_new(
        AggregateMode::FinalPartitioned,
        PhysicalGroupBy::new_single(vec![]),
        partial.aggr_expr().to_vec(),
        partial.filter_expr().to_vec(),
        network_shuffle,
        partial_output_schema,
    )?;
    Ok(Arc::new(final_agg))
}

/// Wrap `partial` in a [`ShmMqProducerExec`] using a
/// [`FixedTargetPartitioner(0)`] so every partial row lands in the leader's
/// single consumer partition. Scalar-agg final-gather is K=1 by definition.
fn wrap_with_producer(
    partial: Arc<dyn ExecutionPlan>,
    _n_workers: u32,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let partitioner: Arc<dyn RowPartitioner> = Arc::new(FixedTargetPartitioner::new(0, 1));
    let producer = ShmMqProducerExec::try_new(partial, partitioner, 1)?;
    Ok(Arc::new(producer))
}

/// Walk the plan top-down depth-first and return the path
/// (index-into-`children`) to the first `AggregateExec(Partial)` encountered.
/// Returns `None` if no Partial aggregate is reachable from the root.
fn find_partial_aggregate_path(plan: &dyn ExecutionPlan) -> Option<Vec<usize>> {
    if let Some(agg) = plan.as_any().downcast_ref::<AggregateExec>() {
        if matches!(agg.mode(), AggregateMode::Partial) {
            return Some(Vec::new());
        }
    }
    for (i, child) in plan.children().iter().enumerate() {
        if let Some(mut path) = find_partial_aggregate_path(child.as_ref()) {
            path.insert(0, i);
            return Some(path);
        }
    }
    None
}

/// Resolve a path from `find_partial_aggregate_path` into the actual node.
fn node_at_path(plan: &Arc<dyn ExecutionPlan>, path: &[usize]) -> Option<Arc<dyn ExecutionPlan>> {
    let mut cursor: Arc<dyn ExecutionPlan> = Arc::clone(plan);
    for &i in path {
        let child = Arc::clone(cursor.children().get(i)?);
        cursor = child;
    }
    Some(cursor)
}

/// Replace the node at `path` in `plan` with `replacement`, propagating the
/// rebuild back up via `with_new_children` so each ancestor's properties get
/// recomputed against the new subtree.
fn replace_at_path(
    plan: Arc<dyn ExecutionPlan>,
    path: &[usize],
    replacement: Arc<dyn ExecutionPlan>,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    if path.is_empty() {
        return Ok(replacement);
    }
    let head = path[0];
    let tail = &path[1..];
    let mut new_children: Vec<Arc<dyn ExecutionPlan>> =
        plan.children().into_iter().map(Arc::clone).collect();
    let target = new_children
        .get(head)
        .cloned()
        .ok_or_else(|| DataFusionError::Internal(format!("walker: child index {head} OOB")))?;
    new_children[head] = replace_at_path(target, tail, replacement)?;
    plan.with_new_children(new_children)
}

/// Count operators in the plan, for the diagnostic message in
/// [`distribute_scalar_agg`]'s NotImplemented error.
fn count_ops(plan: &Arc<dyn ExecutionPlan>) -> usize {
    1 + plan.children().iter().map(|c| count_ops(c)).sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::physical_expr::expressions::Column;
    use datafusion::physical_expr::PhysicalExpr;
    use datafusion::physical_plan::aggregates::PhysicalGroupBy;
    use datafusion::physical_plan::empty::EmptyExec;
    use datafusion::physical_plan::projection::ProjectionExec;

    fn empty_plan() -> Arc<dyn ExecutionPlan> {
        let schema = Arc::new(Schema::new(vec![Field::new("c", DataType::Int64, false)]));
        Arc::new(EmptyExec::new(schema))
    }

    fn partial_agg(input: Arc<dyn ExecutionPlan>) -> Arc<AggregateExec> {
        // Empty group-by, no aggregate exprs. AggregateExec is happy to
        // construct in this degenerate case for tests.
        let group_by = PhysicalGroupBy::new_single(vec![]);
        let agg = AggregateExec::try_new(
            AggregateMode::Partial,
            group_by,
            vec![],
            vec![],
            input.clone(),
            input.schema(),
        )
        .expect("AggregateExec construction");
        Arc::new(agg)
    }

    #[test]
    fn find_partial_aggregate_path_returns_root_path_when_root_is_partial() {
        let p = partial_agg(empty_plan());
        let path = find_partial_aggregate_path(p.as_ref());
        assert_eq!(path, Some(vec![]));
    }

    #[test]
    fn find_partial_aggregate_path_descends_through_projection() {
        let inner: Arc<dyn ExecutionPlan> = partial_agg(empty_plan());
        let proj_schema = inner.schema();
        let exprs: Vec<(Arc<dyn PhysicalExpr>, String)> = (0..proj_schema.fields().len())
            .map(|i| {
                (
                    Arc::new(Column::new(proj_schema.field(i).name(), i)) as Arc<dyn PhysicalExpr>,
                    proj_schema.field(i).name().to_string(),
                )
            })
            .collect();
        let proj: Arc<dyn ExecutionPlan> =
            Arc::new(ProjectionExec::try_new(exprs, inner).expect("ProjectionExec"));
        let path = find_partial_aggregate_path(proj.as_ref());
        assert_eq!(path, Some(vec![0]));
    }

    #[test]
    fn find_partial_aggregate_path_returns_none_when_absent() {
        let plan = empty_plan();
        let path = find_partial_aggregate_path(plan.as_ref());
        assert_eq!(path, None);
    }

    #[test]
    fn replace_at_path_replaces_root_when_path_empty() {
        let original = empty_plan();
        let replacement = empty_plan();
        let out = replace_at_path(original.clone(), &[], replacement.clone()).unwrap();
        assert!(Arc::ptr_eq(&out, &replacement));
    }

    #[test]
    fn replace_at_path_rebuilds_ancestors_via_with_new_children() {
        let inner_partial = partial_agg(empty_plan());
        let proj_schema = inner_partial.schema();
        let exprs: Vec<(Arc<dyn PhysicalExpr>, String)> = (0..proj_schema.fields().len())
            .map(|i| {
                (
                    Arc::new(Column::new(proj_schema.field(i).name(), i)) as Arc<dyn PhysicalExpr>,
                    proj_schema.field(i).name().to_string(),
                )
            })
            .collect();
        let proj: Arc<dyn ExecutionPlan> =
            Arc::new(ProjectionExec::try_new(exprs, inner_partial).expect("ProjectionExec"));
        let replacement = empty_plan();
        let out = replace_at_path(proj, &[0], replacement.clone()).unwrap();
        // Root is still the ProjectionExec; child[0] now points at our replacement.
        let new_child = Arc::clone(out.children().first().expect("child 0"));
        assert!(Arc::ptr_eq(&new_child, &replacement));
    }

    #[test]
    fn count_ops_counts_every_node() {
        let p = empty_plan();
        assert_eq!(count_ops(&p), 1);
        let ag: Arc<dyn ExecutionPlan> = partial_agg(p);
        assert_eq!(count_ops(&ag), 2);
    }

    #[test]
    fn wrap_with_producer_returns_shm_mq_producer_exec_at_root() {
        let partial: Arc<dyn ExecutionPlan> = partial_agg(empty_plan());
        let wrapped = wrap_with_producer(partial, 4).expect("wrap");
        assert_eq!(wrapped.name(), "ShmMqProducerExec");
        // Producer exec has exactly one child (the partial it wraps).
        assert_eq!(wrapped.children().len(), 1);
    }

    #[test]
    fn distribute_plan_returns_not_implemented_for_unsupported_shapes() {
        let p = partial_agg(empty_plan());
        for shape in [
            MppPlanShape::GroupByAggOnBinaryJoin,
            MppPlanShape::TopKGroupByAggOnBinaryJoin,
            MppPlanShape::GroupByAggSingleTable,
            MppPlanShape::JoinOnly,
        ] {
            let err = distribute_plan(shape, p.clone(), 4).unwrap_err();
            assert!(matches!(err, DataFusionError::NotImplemented(_)));
        }
    }

    #[test]
    fn distribute_plan_ineligible_returns_plan_error() {
        let p = partial_agg(empty_plan());
        let err = distribute_plan(MppPlanShape::Ineligible, p, 4).unwrap_err();
        assert!(matches!(err, DataFusionError::Plan(_)));
    }

    #[test]
    fn distribute_scalar_agg_zero_workers_errors() {
        let p = partial_agg(empty_plan());
        let err = distribute_scalar_agg(p, 0).unwrap_err();
        assert!(matches!(err, DataFusionError::Plan(_)));
    }

    #[test]
    fn distribute_scalar_agg_builds_both_plans_for_root_partial() {
        // Plan = bare AggregateExec(Partial); walker produces leader + worker
        // plans without error. Worker plan top is ShmMqProducerExec (single
        // op above Partial). Leader plan top is AggregateExec(FinalPartitioned)
        // wrapping NetworkShuffleExec.
        let p: Arc<dyn ExecutionPlan> = partial_agg(empty_plan());
        let pair = distribute_scalar_agg(p, 4).unwrap();
        assert_eq!(pair.worker_plan.name(), "ShmMqProducerExec");
        assert_eq!(pair.leader_plan.name(), "AggregateExec");
        // Leader's child should be NetworkShuffleExec.
        let leader_child = pair.leader_plan.children()[0].clone();
        assert_eq!(leader_child.name(), "NetworkShuffleExec");
    }
}
