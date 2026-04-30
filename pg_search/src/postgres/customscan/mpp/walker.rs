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
//! Generic cut walker (`distribute_plan`).
//!
//! Ported in spirit from datafusion-contrib/datafusion-distributed's
//! `src/distributed_planner/distribute_plan.rs`. Walks a DataFusion physical
//! plan once, identifies the cut points where the plan needs to cross a
//! network boundary, and rewrites those cuts to emit
//! [`ShuffleExec`](crate::postgres::customscan::mpp::shuffle::ShuffleExec) +
//! [`DrainGatherExec`](crate::postgres::customscan::mpp::shuffle::DrainGatherExec)
//! pairs whose [`MppNetworkBoundary::input_stage`] is stamped by the walker.
//!
//! # What this module does
//!
//! [`distribute_plan`] is the single entry point callers invoke to turn a
//! standard DataFusion physical plan into its MPP-partitioned equivalent.
//! Cut detection is derived from plan structure — the walker looks for a
//! `HashJoinExec` and any `AggregateExec(Partial|Single)` above it, and
//! picks the right topology (scalar-agg, group-by-agg, or bare-join)
//! from that in situ rather than trusting an out-of-band shape enum.
//! The caller's [`MppPlanShape`] is kept around as a sanity cross-check
//! against the derivation, and its only other role is sizing the DSM
//! region at plan time via [`cut_count_for_shape`].
//!
//! # Visibility correctness (invariant enforced here)
//!
//! `VisibilityFilterExec` resolves per-segment packed DocAddress keys to
//! heap TIDs using segment-local Tantivy state plus a ctid-resolver table
//! keyed by `(plan_position, seg_ord)` that lists segments **local to this
//! participant**. Every [`ShuffleExec`] cut must therefore sit **inside** the subtree
//! of every [`VisibilityFilterExec`] the plan contains — i.e. no
//! `VisibilityFilterExec` may be a descendant of a `ShuffleExec`. Inverting
//! that placement means a row from participant A would reach participant B's resolver with
//! a `seg_ord` that addresses A's segment catalog; the lookup would return
//! the wrong heap TID (or panic in `heap_fetch` if the slot is absent).
//! [`assert_visibility_invariant`] walks the finished MPP plan and rejects it
//! with `DataFusionError::Plan` if that invariant is violated.
//!
//! The same constraint applies to other segment-local scan adornments
//! (`SegmentedTopKExec` pre-merge, `TantivyLookupExec`, etc.): they must
//! remain on the scan side of every cut. Today only
//! `VisibilityFilterExec` is asserted because it's the one that causes
//! `heap_fetch` panics; extending the assert to the other node types is
//! mechanical but out of scope until one of them is re-introduced as a
//! cross-shuffle risk.
//!
//! # Mesh allocation timing
//!
//! The walker exposes [`cut_count_for_shape`] so the DSM-estimate hook can
//! size the region at **plan time** without needing to run the full walker
//! inside the hook. Overestimating is cheap (one unused `shm_mq` region per
//! edge, dropped on tear-down) — the hook is free to round up via
//! [`worst_case_cut_count`] when the shape is not yet classified.
//!
//! # Leader/worker mesh-index contract
//!
//! Every topology consumes a contiguous slice of `MppExecutionState::meshes`
//! in a fixed order shared by every participant:
//!
//! - `meshes[0]` is wired into the **left** join input's shuffle.
//! - `meshes[1]` is wired into the **right** join input's shuffle.
//! - `meshes[2]` (when present) is the post-join mesh — either
//!   **final-gather-to-leader** (scalar-agg, `FixedTargetPartitioner(0)`)
//!   or **hash-partition on group keys** (group-by-agg, `HashPartitioner`).
//!
//! `JoinOnly` uses only meshes 0 and 1. Mesh ordering must be identical on
//! every participant — the meshes themselves are symmetric, but a mismatch would
//! route left rows to the right drain and break correctness.

#![allow(deprecated)] // `CoalesceBatchesExec` is deprecated in favor of
                      // arrow-rs's streaming `BatchCoalescer`, but DataFusion
                      // 52 still emits it as a plan node and we must recognize
                      // + reuse it.

#[cfg(test)]
use datafusion_distributed::require_one_child;
use datafusion_distributed::{
    distribute_annotated_plan, AnnotatedPlan, BoundaryFactory, DistributedConfig,
    PlanOrNetworkBoundary, TaskCountAnnotation,
};
use std::collections::VecDeque;
use std::mem;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[cfg(test)]
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::common::config::ConfigOptions;
use datafusion::common::{DataFusionError, Result as DfResult};
use datafusion::physical_expr::expressions::Column;
use datafusion::physical_expr::{Partitioning, PhysicalExpr};
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::aggregates::{AggregateExec, AggregateMode, PhysicalGroupBy};
use datafusion::physical_plan::coalesce_batches::CoalesceBatchesExec;
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::filter::FilterExec;
use datafusion::physical_plan::joins::{HashJoinExec, PartitionMode};
use datafusion::physical_plan::limit::GlobalLimitExec;
use datafusion::physical_plan::repartition::RepartitionExec;
use datafusion::physical_plan::sorts::sort::SortExec;
use datafusion::physical_plan::sorts::sort_preserving_merge::SortPreservingMergeExec;
use datafusion::physical_plan::ExecutionPlan;

use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;
use crate::scan::visibility_ctid_resolver_rule::VisibilityCtidResolverRule;

use super::customscan_glue::MppExecutionState;
use super::plan_build::{wrap_with_mpp_shuffle, MppShuffleInputs};
use super::shape::MppPlanShape;
use super::shuffle::{
    FixedTargetPartitioner, HashPartitioner, MppShuffleExec, RowPartitioner, ShuffleWiring,
};
use super::stage::{MppStage, MppTaskKey};
use super::transport::{DrainBuffer, DrainHandle, MppReceiver, MppSender};
use super::worker::{LeaderMesh, WorkerMesh};
use crate::scan::execution_plan::PgSearchScanPlan;

/// How many shuffle cuts the walker will insert for `shape`. Called by the
/// DSM-estimate hooks in `aggregatescan/mod.rs` and `joinscan/mod.rs` at
/// plan time to size the shared-memory region before the walker has a
/// concrete plan to inspect.
pub fn cut_count_for_shape(shape: MppPlanShape) -> u32 {
    match shape {
        MppPlanShape::ScalarAggOnBinaryJoin => 3,
        MppPlanShape::GroupByAggOnBinaryJoin => 3,
        // TopK groupby reuses scalar-style routing (FixedTargetPartitioner
        // postagg → leader gathers all groups), so 3 cuts: left, right,
        // postagg-to-leader.
        MppPlanShape::TopKGroupByAggOnBinaryJoin => 3,
        MppPlanShape::JoinOnly => 2,
        MppPlanShape::GroupByAggSingleTable => 1,
        MppPlanShape::Ineligible => 0,
    }
}

/// Upper bound on the number of cuts the walker can produce for any
/// supported shape. Used by the DSM estimate hook when the walker hasn't
/// been run yet (or when we don't want to risk running DataFusion physical
/// planning from inside the hook).
///
/// Safe to overestimate: an unused mesh wiring costs one `shm_mq` region
/// per edge and is dropped during `attach_cooperative_drain` / `take_meshes`.
#[allow(dead_code)]
pub fn worst_case_cut_count() -> u32 {
    cut_count_for_shape(MppPlanShape::ScalarAggOnBinaryJoin)
        .max(cut_count_for_shape(MppPlanShape::GroupByAggOnBinaryJoin))
        .max(cut_count_for_shape(MppPlanShape::GroupByAggSingleTable))
        .max(cut_count_for_shape(MppPlanShape::JoinOnly))
}

/// Walk `standard` and rewrite it into an MPP-partitioned plan by
/// identifying cut points and inserting `ShuffleExec` / `DrainGatherExec`
/// pairs stamped with an [`MppStage`].
///
/// Topology is derived from plan structure — the walker locates
/// `HashJoinExec` and any `AggregateExec(Partial|Single)` above it, and
/// picks the right assembler from there. `shape` is kept as an explicit
/// parameter so the pre-classified shape from `aggregatescan` / `joinscan`
/// can be cross-checked against the derivation; a disagreement aborts the
/// query via `DataFusionError::Plan` rather than silently executing the
/// wrong topology.
///
/// Before returning, the built plan is validated against the visibility
/// invariant (see module doc). A violation aborts the query with
/// `DataFusionError::Plan` rather than risk a `heap_fetch` panic at
/// execution.
pub fn distribute_plan(
    standard: Arc<dyn ExecutionPlan>,
    mpp_state: &mut MppExecutionState,
    shape: MppPlanShape,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let total = mpp_state.participant_config().total_participants;

    crate::mpp_log!(
        "mpp: distribute_plan walker dispatching shape={:?} total={} cuts={}",
        shape,
        total,
        cut_count_for_shape(shape)
    );

    // Derive topology from plan structure. `find_partial_agg` walks through
    // `AggregateExec(Final*)` / `CoalescePartitionsExec` / `CoalesceBatchesExec`
    // layers to reach a `Partial|Single` aggregate; `find_hash_join` does the
    // same to reach a `HashJoinExec`. Combined presence of the two + whether
    // the partial agg has group-by keys picks one of three assemblers.
    let partial_agg_opt = find_partial_agg(standard.as_ref());
    let hash_join_opt = find_hash_join(standard.as_ref());

    // Detect TopK structure before dispatch. Cheap walk; only relevant
    // when there's a Partial+HashJoin pair below an outer Sort/Limit.
    let topk_spec = if partial_agg_opt.is_some() && hash_join_opt.is_some() {
        extract_topk_spec(standard.as_ref())
    } else {
        None
    };

    let built = match (partial_agg_opt, hash_join_opt) {
        // Partial agg with empty group keys on top of a HashJoin — scalar agg.
        (Some(agg), Some(_)) if agg.group_expr().expr().is_empty() => {
            validate_shape_matches(shape, MppPlanShape::ScalarAggOnBinaryJoin)?;
            distribute_plan_generic(
                MppPlanShape::ScalarAggOnBinaryJoin,
                standard,
                mpp_state,
                None,
            )?
        }
        // Partial agg with group-by keys on top of a HashJoin AND an
        // outer SortExec[fetch=k] — TopK group-by agg. The classifier
        // (which only saw the logical plan) called this
        // GroupByAggOnBinaryJoin; the structural derivation upgrades.
        (Some(_), Some(_)) if topk_spec.is_some() => {
            validate_shape_matches_topk_upgrade(shape)?;
            distribute_plan_generic(
                MppPlanShape::TopKGroupByAggOnBinaryJoin,
                standard,
                mpp_state,
                topk_spec,
            )?
        }
        // Partial agg with group-by keys on top of a HashJoin — group-by agg.
        (Some(_), Some(_)) => {
            validate_shape_matches(shape, MppPlanShape::GroupByAggOnBinaryJoin)?;
            distribute_plan_generic(
                MppPlanShape::GroupByAggOnBinaryJoin,
                standard,
                mpp_state,
                None,
            )?
        }
        // HashJoin without a Partial agg above — bare join.
        //
        // Step 2d: all three supported shapes route through the DF-D-aligned
        // generic pipeline (`prepare_for_mpp` → `insert_mpp_cuts` →
        // `annotate_plan` → `_distribute_plan` → `finalize_for_mpp`). The
        // legacy topology assemblers below are retained `#[allow(dead_code)]`
        // until Step 2e deletes them.
        (None, Some(_)) => {
            validate_shape_matches(shape, MppPlanShape::JoinOnly)?;
            distribute_plan_generic(MppPlanShape::JoinOnly, standard, mpp_state, None)?
        }
        // Partial agg without HashJoin — e.g. GroupByAggSingleTable, not yet
        // wired through the assembler code.
        (Some(_), None) => {
            return Err(DataFusionError::Plan(
                "mpp: distribute_plan: aggregate-on-single-table shape not yet implemented \
                 (no HashJoinExec in plan)"
                    .into(),
            ));
        }
        // Neither — the caller's classifier should have routed to serial.
        (None, None) => {
            return Err(DataFusionError::Plan(
                "mpp: distribute_plan invoked on an ineligible plan — no HashJoinExec \
                 found; caller should have routed to the non-MPP serial path"
                    .into(),
            ));
        }
    };

    assert_visibility_invariant(&built)?;
    Ok(built)
}

/// Cross-check the pre-classified shape (from the planner) against the
/// walker's structural derivation. Disagreement means either the planner
/// mis-classified the query or the standard physical plan diverged from
/// what the classifier saw — either way, executing anyway would produce
/// wrong results for the shape we actually have.
fn validate_shape_matches(classified: MppPlanShape, derived: MppPlanShape) -> DfResult<()> {
    if classified == derived {
        Ok(())
    } else {
        Err(DataFusionError::Plan(format!(
            "mpp: walker shape derivation mismatch — classifier said {classified:?} \
             but plan structure implies {derived:?}"
        )))
    }
}

/// Accept the classifier's `GroupByAggOnBinaryJoin` as a valid match
/// when the structural derivation upgrades to
/// `TopKGroupByAggOnBinaryJoin`. The classifier only sees the logical
/// plan and can't predict whether DataFusion's physical planner will
/// fuse `Sort[fetch=k]` over the agg; the dispatcher refines based on
/// the physical plan it actually got.
fn validate_shape_matches_topk_upgrade(classified: MppPlanShape) -> DfResult<()> {
    if matches!(classified, MppPlanShape::GroupByAggOnBinaryJoin) {
        Ok(())
    } else {
        Err(DataFusionError::Plan(format!(
            "mpp: walker shape derivation mismatch — classifier said {classified:?} \
             but plan structure implies TopKGroupByAggOnBinaryJoin (only \
             GroupByAggOnBinaryJoin upgrades to TopK)"
        )))
    }
}

// ============================================================================
// Generic cut walker (dead-code scaffolding ported from
// datafusion-contrib/datafusion-distributed).
//
// Port aims to match DF-D's `src/distributed_planner/plan_annotator.rs` and
// `src/common/children_helpers.rs` as closely as ParadeDB allows. The three
// deviations are all driven by ParadeDB constraints, not aesthetics:
//
//   * DF-D's `PlanOrNetworkBoundary::Broadcast` variant is dropped because
//     ParadeDB doesn't do broadcast joins.
//   * DF-D's `AnnotatedPlan::task_count` field is dropped because ParadeDB's
//     participant count is fixed by `MppParticipantConfig::total_participants`
//     at plan time — no task estimators, no cardinality scale factors, no
//     propagation passes needed.
//   * DF-D's `annotate_plan` / `_annotate_plan` are `async` to accommodate
//     task-estimator I/O; ParadeDB's are sync because there's nothing to
//     await.
//
// All three live shapes (`JoinOnly`, `ScalarAggOnBinaryJoin`,
// `GroupByAggOnBinaryJoin`) flow through `distribute_plan_generic`
// (`prepare_for_mpp` → `insert_mpp_cuts` → `annotate_plan` →
// `_distribute_plan` → `finalize_for_mpp`). The legacy topology assemblers
// are gone; this generic pipeline is the only path.
// `Ineligible` and `GroupByAggSingleTable` error out at the dispatcher.
// ============================================================================

// `PlanOrNetworkBoundary` and `AnnotatedPlan` are now imported from the
// `datafusion-distributed` fork (the `paradedb/boundary-factory` branch).
// The fork's variant of `PlanOrNetworkBoundary` has a `Broadcast` arm that
// ParadeDB never emits — `_annotate_plan` below produces only `Plan`,
// `Shuffle`, and `Coalesce` annotations, and `MppBoundaryFactory::broadcast`
// errors loudly if the fork's walker ever asks for one.

/// Annotates recursively an [`ExecutionPlan`] and its children with
/// information about whether a network boundary is needed below it. Ported
/// from DF-D's `annotate_plan` minus `async`, `DistributedConfig`,
/// `TaskEstimator`, `children_isolator_unions`, `propagate_task_count`, and
/// the `Broadcast` cut trigger (none of which apply to ParadeDB).
///
/// The two cut triggers preserved verbatim are:
///
/// * `RepartitionExec(Hash)` → annotate with `Shuffle`.
/// * Any non-leaf plan whose parent is `CoalescePartitionsExec` or
///   `SortPreservingMergeExec` → annotate with `Coalesce`.
///
/// Running this over ParadeDB's serial standard plan (as produced by
/// `exec_datafusion_aggregate`) yields zero network-boundary annotations
/// because the serial plan emits neither trigger. Step 2b adds
/// `insert_mpp_cuts`, a pre-pass that synthesizes the expected markers per
/// [`MppPlanShape`] before this walker runs.
pub(super) fn annotate_plan(
    plan: Arc<dyn ExecutionPlan>,
    total_participants: u32,
) -> DfResult<AnnotatedPlan> {
    let task_count = TaskCountAnnotation::Desired(total_participants as usize);
    _annotate_plan(plan, None, task_count)
}

fn _annotate_plan(
    plan: Arc<dyn ExecutionPlan>,
    parent: Option<&Arc<dyn ExecutionPlan>>,
    task_count: TaskCountAnnotation,
) -> DfResult<AnnotatedPlan> {
    let annotated_children: Vec<AnnotatedPlan> = plan
        .children()
        .into_iter()
        .map(|child| _annotate_plan(Arc::clone(child), Some(&plan), task_count.clone()))
        .collect::<DfResult<Vec<_>>>()?;

    // Wrap the node with a boundary node if the parent marks it.
    let mut annotation = AnnotatedPlan {
        plan_or_nb: PlanOrNetworkBoundary::Plan(Arc::clone(&plan)),
        children: annotated_children,
        task_count: task_count.clone(),
    };

    // Upon reaching a hash repartition, we need to introduce a shuffle right above it.
    if let Some(r_exec) = plan.as_any().downcast_ref::<RepartitionExec>() {
        if matches!(r_exec.partitioning(), Partitioning::Hash(_, _)) {
            annotation = AnnotatedPlan {
                plan_or_nb: PlanOrNetworkBoundary::Shuffle,
                children: vec![annotation],
                task_count: task_count.clone(),
            };
        }
    } else if let Some(parent) = parent {
        // DF-D expresses this as a single `else if let` + `&&` chain; Rust
        // 2021 doesn't allow let-chains, so split into nested ifs. Comments
        // preserved verbatim.
        //
        // If this node is a leaf node, putting a network boundary above is a bit wasteful, so
        // we don't want to do it.
        // If the parent is trying to coalesce all partitions into one, we need to introduce
        // a network coalesce right below it (or in other words, above the current node)
        if !plan.children().is_empty()
            && (parent.as_any().is::<CoalescePartitionsExec>()
                || parent.as_any().is::<SortPreservingMergeExec>())
        {
            annotation = AnnotatedPlan {
                plan_or_nb: PlanOrNetworkBoundary::Coalesce,
                children: vec![annotation],
                task_count,
            };
        }
    }

    Ok(annotation)
}

/// Pre-pass: rewrite a ParadeDB "standard" physical plan so it carries the
/// `RepartitionExec(Hash)` / `CoalescePartitionsExec` markers the DF-D
/// generic walker treats as cut triggers.
///
/// ParadeDB's standard plan (as produced by `exec_datafusion_aggregate`) is
/// single-partition and emits neither marker: the `HashJoinExec` is
/// `CollectLeft` with serial left/right scans, there's no upstream
/// `RepartitionExec`, and the aggregate chain has no `CoalescePartitionsExec`
/// above the `Partial` because there's only one input partition to begin
/// with. Running [`annotate_plan`] directly over that plan yields zero
/// boundary annotations.
///
/// This function closes that gap by shape:
///
/// * [`MppPlanShape::JoinOnly`] — wrap `HashJoinExec.left()` and `.right()`
///   in `RepartitionExec(Hash(key))`. Two Shuffle triggers result.
///
/// * [`MppPlanShape::ScalarAggOnBinaryJoin`] — the two pre-join
///   `RepartitionExec(Hash(key))` above, plus a `CoalescePartitionsExec`
///   wrapping the `AggregateExec(Partial)`. The two pre-join wrappers give
///   Shuffle triggers; the coalesce wrapper gives a Coalesce trigger above
///   the Partial whose Step-2c rewrite emits a `FixedTargetPartitioner(0)`
///   (final-gather to leader).
///
/// * [`MppPlanShape::GroupByAggOnBinaryJoin`] — the two pre-join
///   `RepartitionExec(Hash(key))` above, plus a
///   `RepartitionExec(Hash(group_keys))` wrapping the `AggregateExec(Partial)`.
///   Three Shuffle triggers; the post-Partial one drives the postagg shuffle
///   with a `HashPartitioner` in Step 2c.
///
/// The rewrite walks bottom-up so rewrites of descendants compose cleanly
/// with rewrites of ancestors (e.g. the scalar-agg Partial wrap must see
/// the already-shuffled join as its grandchild).
pub(super) fn insert_mpp_cuts(
    plan: Arc<dyn ExecutionPlan>,
    shape: MppPlanShape,
    total_participants: u32,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let kind = match shape {
        MppPlanShape::ScalarAggOnBinaryJoin => CutKind::ScalarAgg,
        MppPlanShape::GroupByAggOnBinaryJoin => CutKind::GroupByAgg,
        // TopK uses scalar-style postagg cut: wrap Partial in
        // `CoalescePartitionsExec` so the walker emits a
        // `FixedTargetPartitioner(0)` shuffle that ships every
        // participant's groups to the leader. The leader's finalize
        // wraps with `FinalPartitioned + SortExec[fetch=k]`.
        MppPlanShape::TopKGroupByAggOnBinaryJoin => CutKind::TopKGroupByAgg,
        MppPlanShape::JoinOnly => CutKind::JoinOnly,
        MppPlanShape::GroupByAggSingleTable => {
            return Err(DataFusionError::Plan(
                "mpp: insert_mpp_cuts: GroupByAggSingleTable not yet supported".into(),
            ));
        }
        MppPlanShape::Ineligible => {
            return Err(DataFusionError::Plan(
                "mpp: insert_mpp_cuts invoked on Ineligible shape — caller should have \
                 routed to the non-MPP serial path"
                    .into(),
            ));
        }
    };
    rewrite_with_cuts(plan, kind, total_participants)
}

#[derive(Debug, Clone, Copy)]
enum CutKind {
    ScalarAgg,
    GroupByAgg,
    /// Like `ScalarAgg` topology-wise (CoalescePartitionsExec wrapping
    /// → `FixedTargetPartitioner(0)` postagg), but the Partial may have
    /// group keys; the leader resolves the global Top-K after gathering.
    TopKGroupByAgg,
    JoinOnly,
}

fn rewrite_with_cuts(
    plan: Arc<dyn ExecutionPlan>,
    kind: CutKind,
    n: u32,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    // Recurse into children first so rewrites compose bottom-up.
    let original_children: Vec<Arc<dyn ExecutionPlan>> =
        plan.children().iter().map(|c| Arc::clone(c)).collect();
    let new_children: Vec<Arc<dyn ExecutionPlan>> = original_children
        .iter()
        .map(|c| rewrite_with_cuts(Arc::clone(c), kind, n))
        .collect::<DfResult<Vec<_>>>()?;

    let any_child_changed = original_children
        .iter()
        .zip(new_children.iter())
        .any(|(a, b)| !Arc::ptr_eq(a, b));
    let plan = if any_child_changed {
        Arc::clone(&plan).with_new_children(new_children)?
    } else {
        plan
    };

    // HashJoinExec — wrap its left / right inputs with RepartitionExec(Hash)
    // so the DF-D walker sees Shuffle triggers there. Does *not* touch the
    // join's own node type, mode, or keys — the join itself still runs
    // locally on each participant, operating on shuffle-gathered inputs.
    if let Some(hj) = plan.as_any().downcast_ref::<HashJoinExec>() {
        let join_on = hj.on().to_vec();
        let left_keys: Vec<Arc<dyn PhysicalExpr>> =
            join_on.iter().map(|(l, _)| Arc::clone(l)).collect();
        let right_keys: Vec<Arc<dyn PhysicalExpr>> =
            join_on.iter().map(|(_, r)| Arc::clone(r)).collect();

        let new_left: Arc<dyn ExecutionPlan> = Arc::new(RepartitionExec::try_new(
            Arc::clone(hj.left()),
            Partitioning::Hash(left_keys, n as usize),
        )?);
        let new_right: Arc<dyn ExecutionPlan> = Arc::new(RepartitionExec::try_new(
            Arc::clone(hj.right()),
            Partitioning::Hash(right_keys, n as usize),
        )?);
        return Arc::clone(&plan).with_new_children(vec![new_left, new_right]);
    }

    // AggregateExec(Partial) — per-shape wrapping. Final/FinalPartitioned
    // don't get wrapped; they're the stage above the cut.
    if let Some(agg) = plan.as_any().downcast_ref::<AggregateExec>() {
        if matches!(agg.mode(), AggregateMode::Partial) {
            let group_exprs = agg.group_expr().expr();
            match (kind, group_exprs.is_empty()) {
                (CutKind::ScalarAgg, true) => {
                    // Scalar agg: Coalesce trigger above the Partial.
                    return Ok(Arc::new(CoalescePartitionsExec::new(plan)));
                }
                (CutKind::GroupByAgg, false) => {
                    // Group-by agg: Shuffle trigger above the Partial,
                    // hashed on the group-by keys.
                    let group_keys: Vec<Arc<dyn PhysicalExpr>> =
                        group_exprs.iter().map(|(e, _)| Arc::clone(e)).collect();
                    return Ok(Arc::new(RepartitionExec::try_new(
                        plan,
                        Partitioning::Hash(group_keys, n as usize),
                    )?));
                }
                // TopK group-by agg: same Coalesce trigger as ScalarAgg
                // regardless of whether the Partial carries group keys.
                // The leader will gather every participant's groups and
                // run the global FinalPartitioned + SortExec[fetch=k].
                (CutKind::TopKGroupByAgg, _) => {
                    return Ok(Arc::new(CoalescePartitionsExec::new(plan)));
                }
                _ => {}
            }
        }
    }

    Ok(plan)
}

/// Per-query state the [`MppBoundaryFactory`] captures behind a
/// [`Mutex`] so the fork's `_distribute_plan` walker can call our
/// [`BoundaryFactory`] methods through a `&self` reference.
struct MppBoundaryFactoryState {
    /// Pool of pre-allocated meshes; each [`MppBoundaryFactory::shuffle`] /
    /// [`MppBoundaryFactory::coalesce`] call pops one off the front.
    meshes: VecDeque<MeshHalves>,
}

/// [`BoundaryFactory`] implementation that emits ParadeDB's `MppShuffleExec`
/// (instead of the fork's `NetworkShuffleExec` / `NetworkCoalesceExec` /
/// `NetworkBroadcastExec`) for each annotated boundary the walker encounters.
///
/// The factory wraps the per-query mesh pool, query id, participant index,
/// and shape classification — the same scalars the legacy `CutEmitCtx`
/// carried — so the fork's `distribute_annotated_plan` walker can run
/// unchanged and dispatch through the trait. The Mutex<>-wrapped state is
/// the price of `&self` in the trait signature; the walker is single-
/// threaded and the lock is uncontended.
struct MppBoundaryFactory {
    shape: MppPlanShape,
    state: Mutex<MppBoundaryFactoryState>,
    query_id: u64,
    participant_index: u32,
    total_participants: u32,
}

impl MppBoundaryFactory {
    /// Construct a factory by draining the meshes the leader / worker
    /// allocated up front. `expected_cuts` must match
    /// [`cut_count_for_shape`] for the shape — [`take_meshes`] panics
    /// otherwise, surfacing the leader/worker contract mismatch loudly
    /// rather than deferring to a walker-side error.
    fn from_state(
        state: &mut MppExecutionState,
        shape: MppPlanShape,
        expected_cuts: usize,
    ) -> Self {
        let cfg = state.participant_config();
        let participant_index = cfg.participant_index;
        let total_participants = cfg.total_participants;
        let query_id = state.query_id();
        let meshes: VecDeque<MeshHalves> = take_meshes(state, expected_cuts).into();
        Self {
            shape,
            state: Mutex::new(MppBoundaryFactoryState { meshes }),
            query_id,
            participant_index,
            total_participants,
        }
    }

    /// Shape-aware emit body shared by the `shuffle` and `coalesce` arms.
    /// Pops a mesh, builds the `ShuffleWiring`, and hands off to
    /// [`wrap_with_mpp_shuffle`] which constructs the `MppShuffleExec`.
    fn emit_one_cut(
        &self,
        cut_index: u32,
        child: Arc<dyn ExecutionPlan>,
        partitioner: Arc<dyn RowPartitioner>,
        tag: &'static str,
        hash_keys: Option<Vec<Arc<dyn PhysicalExpr>>>,
        drive_partition: u32,
    ) -> DfResult<Arc<dyn ExecutionPlan>> {
        let mesh = self
            .state
            .lock()
            .unwrap()
            .meshes
            .pop_front()
            .ok_or_else(|| {
                DataFusionError::Plan(format!(
                    "mpp: MppBoundaryFactory: out of mesh slots at cut {cut_index} ({tag}); \
                     insert_mpp_cuts / cut_count_for_shape disagree"
                ))
            })?;
        let stage = MppStage::new(self.query_id, cut_index, self.total_participants);
        let drain = spawn_drain(mesh.inbound);
        let task_key = MppTaskKey {
            query_id: stage.query_id,
            stage_id: stage.stage_id,
            task_number: self.participant_index,
        };
        let outbound = attach_cooperative_drain(stamp_frame_ids(mesh.outbound, task_key), &drain);
        let wiring = ShuffleWiring {
            partitioner,
            outbound_senders: outbound,
            participant_index: self.participant_index,
            cooperative_drain: Some(Arc::clone(&drain)),
        };
        let wrapped_schema = child.schema();
        wrap_with_mpp_shuffle(MppShuffleInputs {
            child,
            wiring,
            drain_handle: drain,
            wrapped_schema,
            tag,
            stage: Some(stage),
            hash_keys,
            drive_partition,
        })
    }

    /// Postcondition: every mesh popped corresponds to a real cut emitted
    /// into the plan. The over-emit case is caught inside `emit_one_cut`
    /// when `pop_front` returns `None`. This catches the symmetric
    /// under-emit case where `insert_mpp_cuts` synthesized fewer cut
    /// markers than `cut_count_for_shape` promised — meshes left
    /// unconsumed here would otherwise be silently dropped along with
    /// the factory.
    fn assert_all_meshes_consumed(&self, observed_cuts: u32) -> DfResult<()> {
        let leftover = self.state.lock().unwrap().meshes.len();
        if leftover > 0 {
            return Err(DataFusionError::Plan(format!(
                "mpp: distribute_plan_generic: walker emitted {observed_cuts} cuts but \
                 shape {:?} pre-allocated {} meshes; {leftover} meshes left unconsumed",
                self.shape,
                observed_cuts as usize + leftover
            )));
        }
        Ok(())
    }
}

impl BoundaryFactory for MppBoundaryFactory {
    fn shuffle(
        &self,
        child: Arc<dyn ExecutionPlan>,
        _query_id: uuid::Uuid,
        stage_id: usize,
        _task_count: usize,
        _input_task_count: usize,
    ) -> DfResult<Arc<dyn ExecutionPlan>> {
        let cut_index = stage_id as u32;
        let tag = tag_for_cut(self.shape, cut_index);
        let (partitioner, underlying, hash_keys) =
            shuffle_keys_from_repartition(&child, self.total_participants)?;
        // Hash shuffle: drive_partition is this participant's index — the
        // partition that holds this participant's local slice of rows.
        self.emit_one_cut(
            cut_index,
            underlying,
            partitioner,
            tag,
            Some(hash_keys),
            self.participant_index,
        )
    }

    fn coalesce(
        &self,
        child: Arc<dyn ExecutionPlan>,
        _query_id: uuid::Uuid,
        stage_id: usize,
        _task_count: usize,
        _input_task_count: usize,
    ) -> DfResult<Arc<dyn ExecutionPlan>> {
        let cut_index = stage_id as u32;
        let tag = tag_for_cut(self.shape, cut_index);
        let partitioner: Arc<dyn RowPartitioner> =
            Arc::new(FixedTargetPartitioner::new(0, self.total_participants));
        // Fixed-target gather: drive_partition is the target (0) on every
        // participant. No hash keys to declare — output partitioning is
        // `UnknownPartitioning(1)` since only the target ever has rows.
        self.emit_one_cut(cut_index, child, partitioner, tag, None, 0)
    }

    fn broadcast(
        &self,
        _child: Arc<dyn ExecutionPlan>,
        _query_id: uuid::Uuid,
        _stage_id: usize,
        _task_count: usize,
        _input_task_count: usize,
    ) -> DfResult<Arc<dyn ExecutionPlan>> {
        // ParadeDB never emits a Broadcast annotation in `_annotate_plan`
        // and never inserts a `BroadcastExec` marker in `insert_mpp_cuts`,
        // so the fork's walker should not reach this arm. If it ever does,
        // surface a planner error rather than silently no-op.
        Err(DataFusionError::Internal(
            "mpp: MppBoundaryFactory: unexpected Broadcast emit — \
             ParadeDB does not produce broadcast annotations"
                .into(),
        ))
    }
}

/// Extract hash-key column indices from the `RepartitionExec(Hash)` marker
/// that [`insert_mpp_cuts`] sits directly below a `Shuffle` boundary, then
/// build a [`HashPartitioner`] and return the marker's underlying input.
/// The walker wraps that input directly, dropping the `RepartitionExec` —
/// the MPP `ShuffleExec` replaces what the `RepartitionExec` was doing.
///
/// Errors (all surface as `DataFusionError::Plan`) when the child is not a
/// `RepartitionExec(Hash)` or any hash key is not a plain
/// [`Column`](datafusion::physical_expr::expressions::Column). The second
/// case is the same constraint the legacy topology assemblers enforce via
/// [`extract_key_col_indices`] — milestone 1 only supports column-ref join
/// keys.
/// Tuple returned by [`shuffle_keys_from_repartition`]: the production
/// `RowPartitioner` (production `HashPartitioner`), the underlying
/// `ExecutionPlan` whose rows the shuffle consumes, and the original hash-key
/// `PhysicalExpr` list — the latter is forwarded into
/// [`MppPartitionAdapterExec`] so the wrapped output declares
/// `Partitioning::Hash(keys, N)` natively.
type ShuffleEmitInputs = (
    Arc<dyn RowPartitioner>,
    Arc<dyn ExecutionPlan>,
    Vec<Arc<dyn PhysicalExpr>>,
);

fn shuffle_keys_from_repartition(
    child: &Arc<dyn ExecutionPlan>,
    total_participants: u32,
) -> DfResult<ShuffleEmitInputs> {
    let r_exec = child
        .as_any()
        .downcast_ref::<RepartitionExec>()
        .ok_or_else(|| {
            DataFusionError::Plan(
                "mpp: _distribute_plan: Shuffle boundary expected RepartitionExec child \
                 synthesized by insert_mpp_cuts"
                    .into(),
            )
        })?;
    let Partitioning::Hash(exprs, _n) = r_exec.partitioning() else {
        return Err(DataFusionError::Plan(
            "mpp: _distribute_plan: Shuffle boundary expected Partitioning::Hash; \
             insert_mpp_cuts only synthesizes Hash repartitions"
                .into(),
        ));
    };
    let keys: Vec<usize> = exprs.iter().map(col_index).collect::<DfResult<Vec<_>>>()?;
    let partitioner: Arc<dyn RowPartitioner> =
        Arc::new(HashPartitioner::new(keys, total_participants));
    Ok((partitioner, Arc::clone(r_exec.input()), exprs.clone()))
}

/// Map a (shape, bottom-up cut index) pair to the byte-exact tag string the
/// existing topology assemblers pass to `wrap_with_mpp_shuffle`. Keeps
/// benchmark-log grep patterns and `mpp_trace` output unchanged across the
/// generic-walker migration.
///
/// The cut ordinals below follow the post-order (children-first) traversal
/// `_distribute_plan` performs against a plan produced by [`insert_mpp_cuts`]:
///
/// * `JoinOnly`: `HashJoinExec` has two children — left subtree first, then
///   right subtree. After `insert_mpp_cuts` each is wrapped in
///   `RepartitionExec(Hash)` that `annotate_plan` lifts to a `Shuffle`.
///   Bottom-up visit order therefore yields indices 0 = `join_left`,
///   1 = `join_right`.
///
/// * `ScalarAggOnBinaryJoin`: same two join-side cuts (0, 1) plus a top
///   `CoalescePartitionsExec` over the `AggregateExec(Partial)` that
///   `annotate_plan` lifts to a `Coalesce`. Bottom-up the join-side cuts
///   come first (descendants visited before parents), then the scalar-final
///   cut at index 2.
///
/// * `GroupByAggOnBinaryJoin`: same two join-side cuts (0, 1) plus a top
///   `RepartitionExec(Hash(group_keys))` that becomes a `Shuffle` at index 2.
///
/// Mis-indexed cuts fall through to `"mpp_unknown_cut"` rather than panic —
/// a panic would mask a real emit-arm regression.
fn tag_for_cut(shape: MppPlanShape, cut_index: u32) -> &'static str {
    match (shape, cut_index) {
        (MppPlanShape::JoinOnly, 0) => "join_left",
        (MppPlanShape::JoinOnly, 1) => "join_right",
        (MppPlanShape::ScalarAggOnBinaryJoin, 0) => "scalar_left",
        (MppPlanShape::ScalarAggOnBinaryJoin, 1) => "scalar_right",
        (MppPlanShape::ScalarAggOnBinaryJoin, 2) => "scalar_final",
        (MppPlanShape::GroupByAggOnBinaryJoin, 0) => "gb_left",
        (MppPlanShape::GroupByAggOnBinaryJoin, 1) => "gb_right",
        (MppPlanShape::GroupByAggOnBinaryJoin, 2) => "gb_postagg",
        (MppPlanShape::TopKGroupByAggOnBinaryJoin, 0) => "tk_left",
        (MppPlanShape::TopKGroupByAggOnBinaryJoin, 1) => "tk_right",
        (MppPlanShape::TopKGroupByAggOnBinaryJoin, 2) => "tk_final",
        _ => "mpp_unknown_cut",
    }
}

// ============================================================================
// Generic MPP pipeline (Step 2d). `distribute_plan_generic` composes the
// shape-agnostic walker (`insert_mpp_cuts` → `annotate_plan` →
// `_distribute_plan`) with explicit shape-specific pre-passes
// (`prepare_for_mpp`) and post-passes (`finalize_for_mpp`). ParadeDB-specific
// obligations that don't fit DF-D's generic emit model (probe-side dynamic
// filter strip, HashJoinExec rebuild with `PartitionMode::Partitioned`,
// CoalesceBatchesExec(65_536), leader-only FinalPartitioned,
// VisibilityCtidResolverRule re-run) live in the pre/post passes — the
// walker body stays verbatim-DF-D.
//
// The dispatcher above (`distribute_plan`) routes per shape: JoinOnly goes
// through this pipeline (Step 2d); the two aggregate shapes stay on the
// legacy topology assemblers below until their pre/post passes are wired
// (still on the critical path for Step 2d follow-up).
// ============================================================================

/// Sort + Limit info captured from the standard plan when the dispatcher
/// upgrades a `GroupByAggOnBinaryJoin` query to
/// `TopKGroupByAggOnBinaryJoin`. Re-applied on the leader by
/// [`finalize_topk_groupby_agg`] after the post-agg gather.
#[derive(Clone)]
struct TopKSpec {
    sort_expr: datafusion::physical_expr::LexOrdering,
    fetch: usize,
}

/// Look at the standard plan for an outer `SortExec[fetch=k]` (DataFusion's
/// fused TopK) or `GlobalLimitExec(SortExec)` above the
/// `AggregateExec(Final|FinalPartitioned|SinglePartitioned)`. Returns
/// `Some(TopKSpec)` so the dispatcher can route to the TopK shape.
///
/// Walks a small allow-list (`SortExec`, `GlobalLimitExec`,
/// `LocalLimitExec`); anything else stops the walk and returns `None`,
/// preserving the existing GroupByAgg dispatch for plans we don't
/// recognise as TopK.
fn extract_topk_spec(plan: &dyn ExecutionPlan) -> Option<TopKSpec> {
    use datafusion::physical_plan::limit::LocalLimitExec;
    let mut node = plan;
    let mut limit: Option<usize> = None;
    loop {
        if let Some(sort) = node.as_any().downcast_ref::<SortExec>() {
            // Prefer the SortExec's own fetch; fall back to a wrapping
            // limit if SortExec didn't fuse the limit.
            let fetch = sort.fetch().or(limit)?;
            return Some(TopKSpec {
                sort_expr: sort.expr().clone(),
                fetch,
            });
        }
        if let Some(g) = node.as_any().downcast_ref::<GlobalLimitExec>() {
            limit = limit.or_else(|| g.fetch().map(|f| f + g.skip()));
            node = g.input().as_ref();
            continue;
        }
        if let Some(l) = node.as_any().downcast_ref::<LocalLimitExec>() {
            limit = limit.or(Some(l.fetch()));
            node = l.input().as_ref();
            continue;
        }
        return None;
    }
}

/// Glue together the generic walker (`insert_mpp_cuts` → `annotate_plan` →
/// `_distribute_plan`) with the shape-specific pre/post passes. Called by
/// [`distribute_plan`] for shapes whose post-passes have been ported.
fn distribute_plan_generic(
    shape: MppPlanShape,
    standard: Arc<dyn ExecutionPlan>,
    mpp_state: &mut MppExecutionState,
    topk: Option<TopKSpec>,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    debug_assert!(
        topk.is_some() == matches!(shape, MppPlanShape::TopKGroupByAggOnBinaryJoin),
        "topk spec must be Some iff shape is TopKGroupByAggOnBinaryJoin"
    );
    let n = mpp_state.participant_config().total_participants;
    let expected_cuts = cut_count_for_shape(shape) as usize;

    let prepared = prepare_for_mpp(shape, standard)?;
    let with_cuts = insert_mpp_cuts(prepared, shape, n)?;
    let annotated = annotate_plan(with_cuts, n)?;

    // Drive the fork's walker through our `MppBoundaryFactory`. The fork's
    // `_distribute_plan` increments `stage_id` once per emitted boundary
    // bottom-up, which is exactly the cut index our `tag_for_cut` and
    // `MppStage::new` consume. Starting from 0 keeps the tag mapping
    // identical to the legacy walker.
    let factory = MppBoundaryFactory::from_state(mpp_state, shape, expected_cuts);
    // Fork's `_distribute_plan` calls `DistributedConfig::from_config_options`
    // which expects the extension to be registered. We don't tune any
    // distributed-specific knobs (broadcast_joins, partial_reduce, etc.) so
    // the default values are correct for ParadeDB's pipeline.
    let mut cfg = ConfigOptions::default();
    cfg.extensions.insert(DistributedConfig::default());
    let query_id = Uuid::nil();
    let mut stage_id_after: usize = 0;
    let emitted =
        distribute_annotated_plan(annotated, &cfg, query_id, &mut stage_id_after, &factory)?;
    factory.assert_all_meshes_consumed(stage_id_after as u32)?;
    finalize_for_mpp(shape, emitted, mpp_state, topk)
}

/// Shape-specific pre-pass. Runs before [`insert_mpp_cuts`] synthesizes
/// the `RepartitionExec(Hash)` / `CoalescePartitionsExec` cut markers.
///
/// All three live shapes are wired through their own pre-pass arm:
/// [`prepare_join_only`] for `JoinOnly`, and
/// [`prepare_agg_on_binary_join`] for both aggregate shapes (toggling
/// `expect_group_by` on the GROUP BY case).
fn prepare_for_mpp(
    shape: MppPlanShape,
    plan: Arc<dyn ExecutionPlan>,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    match shape {
        MppPlanShape::JoinOnly => prepare_join_only(plan),
        MppPlanShape::ScalarAggOnBinaryJoin => prepare_agg_on_binary_join(plan, false),
        MppPlanShape::GroupByAggOnBinaryJoin => prepare_agg_on_binary_join(plan, true),
        // TopK uses the same pre-pass as GroupByAgg: returns the
        // Partial-rooted subtree. The outer SortExec[fetch=k] wrapper
        // is dropped here and re-applied by `finalize_topk_groupby_agg`
        // on the leader after the post-agg gather.
        MppPlanShape::TopKGroupByAggOnBinaryJoin => prepare_agg_on_binary_join(plan, true),
        MppPlanShape::GroupByAggSingleTable | MppPlanShape::Ineligible => Ok(plan),
    }
}

/// JoinOnly pre-pass. Rewrites the first `HashJoinExec` in `plan` so
/// that [`insert_mpp_cuts`] sees a clean subtree below each side.
///
///  * `strip_repartition_layers` peels off any DataFusion-inserted
///    `RepartitionExec` / `CoalesceBatchesExec` layers. `insert_mpp_cuts`
///    will add its own `RepartitionExec(Hash)` marker; leaving the old
///    layers in place would just stack redundant partitioners.
///  * `strip_dynamic_filters_in_subtree` removes the probe-side dynamic
///    filter the `FilterPushdown` physical optimizer pushed into the
///    `PgSearchScanPlan`. With a dynamic filter on the probe, rows
///    destined for peer participants get dropped before they hit the shuffle
///    (the build side hasn't filled the filter yet on this participant), and
///    the row count drops to ~0 across the mesh.
///
/// Outer wrappers (`VisibilityFilterExec`, `SegmentedTopKExec`, ...) are
/// rebuilt by `with_new_children` so their subtree identity refreshes.
/// Errors (via `DataFusionError::Plan`) when no `HashJoinExec` is found
/// — the caller already validated the shape, so the absence would
/// indicate a planner bug.
fn prepare_join_only(plan: Arc<dyn ExecutionPlan>) -> DfResult<Arc<dyn ExecutionPlan>> {
    fn walk(node: Arc<dyn ExecutionPlan>) -> DfResult<(Arc<dyn ExecutionPlan>, bool)> {
        if let Some(hj) = node.as_any().downcast_ref::<HashJoinExec>() {
            let new_left = strip_repartition_layers(Arc::clone(hj.left()));
            let new_right =
                strip_dynamic_filters_in_subtree(strip_repartition_layers(Arc::clone(hj.right())))?;
            return Ok((node.with_new_children(vec![new_left, new_right])?, true));
        }
        let children = node.children();
        if children.is_empty() {
            return Ok((node, false));
        }
        let mut rebuilt = Vec::with_capacity(children.len());
        let mut any_changed = false;
        for child in children {
            let (new, changed) = walk(Arc::clone(child))?;
            if changed {
                any_changed = true;
            }
            rebuilt.push(new);
        }
        if any_changed {
            Ok((node.with_new_children(rebuilt)?, true))
        } else {
            Ok((node, false))
        }
    }
    let (new_root, found) = walk(plan)?;
    if !found {
        return Err(DataFusionError::Plan(
            "mpp: prepare_join_only: no HashJoinExec found in plan".into(),
        ));
    }
    Ok(new_root)
}

/// Shape-specific post-pass. Runs after [`_distribute_plan`] emits the
/// `ShuffleExec` / `DrainGatherExec` pairs.
///
/// All four live shapes are wired: [`finalize_join_only`],
/// [`finalize_scalar_agg`], [`finalize_groupby_agg`], and
/// [`finalize_topk_groupby_agg`]. `topk` is `Some` only for the TopK
/// shape (asserted in the caller).
fn finalize_for_mpp(
    shape: MppPlanShape,
    plan: Arc<dyn ExecutionPlan>,
    mpp_state: &MppExecutionState,
    topk: Option<TopKSpec>,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    match shape {
        MppPlanShape::JoinOnly => finalize_join_only(plan, mpp_state),
        MppPlanShape::ScalarAggOnBinaryJoin => finalize_scalar_agg(plan, mpp_state),
        MppPlanShape::GroupByAggOnBinaryJoin => finalize_groupby_agg(plan, mpp_state),
        MppPlanShape::TopKGroupByAggOnBinaryJoin => {
            let spec = topk.expect("TopKGroupByAgg shape ⇒ TopKSpec must be Some");
            finalize_topk_groupby_agg(plan, mpp_state, spec)
        }
        MppPlanShape::GroupByAggSingleTable | MppPlanShape::Ineligible => Ok(plan),
    }
}

/// JoinOnly post-pass. The generic walker's `Plan` arm called
/// `with_new_children` on the `HashJoinExec` to stitch in the emitted
/// shuffles. That preserves the original's `on` / `filter` /
/// `projection` / `join_type`, but two MPP-specific fixups are still
/// needed:
///
///  1. **Rebuild with `PartitionMode::Partitioned`.** The standard plan's
///     HashJoin typically carries `PartitionMode::Auto` or `CollectLeft`.
///     Under MPP the probe + build have already been partitioned by the
///     shuffles, so the join must run per-partition against those
///     outputs. `with_new_children` doesn't update the partition mode.
///  2. **Re-run `VisibilityCtidResolverRule`.** `with_new_children`
///     rebuilt any `VisibilityFilterExec` above the join via
///     `VisibilityFilterExec::new`, which resets `ctid_resolvers` to
///     empty. The resolver rule re-populates them against the new
///     subtree.
fn finalize_join_only(
    plan: Arc<dyn ExecutionPlan>,
    _mpp_state: &MppExecutionState,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let rebuilt = rebuild_hash_join_as_partitioned(plan)?;
    run_visibility_ctid_resolver_rule(rebuilt)
}

/// Find the first `HashJoinExec` in `plan`, rebuild it via
/// `HashJoinExec::try_new` with `PartitionMode::Partitioned`, and graft
/// back into the tree via [`replace_first_hash_join`] so outer wrappers
/// (`VisibilityFilterExec`, `SegmentedTopKExec`, `TantivyLookupExec`,
/// ...) refresh their subtree identity.
///
/// Most fields are preserved (`on`, `filter`, `join_type`, `projection`,
/// `null_equality`, `null_aware`). One field is **intentionally
/// dropped**: `dynamic_filter`. `prepare_join_only` already stripped the
/// probe-side dynamic filter via `strip_dynamic_filters_in_subtree`; we
/// don't re-apply it here because under MPP the build side is split
/// across participants and the local `Arc<DynamicFilterPhysicalExpr>`
/// only knows this participant's keys — a probe-side filter against
/// that partial key set would drop rows that other participants' builds
/// would have matched. The aggregate paths take the same approach with
/// a small twist (`build_partitioned_hj_with_probe_filter` re-applies
/// the filter as a `FilterExec` *above* the post-shuffle output, where
/// only the local partition flows through anyway). For JoinOnly, the
/// per-participant `FilterExec` would still be safe but the optimizer
/// rebuild for visibility-ctid resolution makes the savings small;
/// dropping is the simpler choice and is correct.
fn rebuild_hash_join_as_partitioned(
    plan: Arc<dyn ExecutionPlan>,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let hj = find_hash_join(plan.as_ref()).ok_or_else(|| {
        DataFusionError::Internal(
            "mpp: finalize_join_only: HashJoinExec missing after _distribute_plan".into(),
        )
    })?;
    let left = Arc::clone(hj.left());
    let right = Arc::clone(hj.right());
    let on = hj.on().to_vec();
    let filter = hj.filter().cloned();
    let join_type = *hj.join_type();
    let projection = hj.projection.as_deref().map(|s| s.to_vec());
    let null_equality = hj.null_equality();
    let null_aware = hj.null_aware;
    let replacement: Arc<dyn ExecutionPlan> = Arc::new(HashJoinExec::try_new(
        left,
        right,
        on,
        filter,
        &join_type,
        projection,
        PartitionMode::Partitioned,
        null_equality,
        null_aware,
    )?);
    replace_first_hash_join(plan, replacement)?.ok_or_else(|| {
        DataFusionError::Internal(
            "mpp: finalize_join_only: replace_first_hash_join could not find target".into(),
        )
    })
}

/// Re-run `VisibilityCtidResolverRule` on `plan` so any
/// `VisibilityFilterExec` rebuilt by `with_new_children` (which resets
/// its `ctid_resolvers` table) gets wired back to the scans in its fresh
/// subtree.
fn run_visibility_ctid_resolver_rule(
    plan: Arc<dyn ExecutionPlan>,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let config = ConfigOptions::default();
    VisibilityCtidResolverRule.optimize(plan, &config)
}

/// Shared pre-pass for the two aggregate-on-binary-join shapes. Locates
/// the topmost `AggregateExec(Partial|Single)` whose transitive child is a
/// `HashJoinExec`, rebuilds the HJ subtree so
/// [`insert_mpp_cuts`] sees a clean cut site, normalizes the aggregate
/// mode to `Partial` (so [`rewrite_with_cuts`]'s Partial-only match
/// fires), and returns just that Partial-rooted subtree. Outer wrappers
/// in the standard plan (e.g. `AggregateExec(Final)` /
/// `AggregateExec(FinalPartitioned)` / `CoalescePartitionsExec`) are
/// dropped — the post-pass (`finalize_scalar_agg` /
/// `finalize_groupby_agg`) rebuilds the correct `FinalPartitioned` wrap
/// against the MPP-shuffled plan.
///
///  * `strip_repartition_layers` peels off `RepartitionExec` /
///    `CoalesceBatchesExec` layers DataFusion inserted for single-
///    process hash partitioning — `insert_mpp_cuts` will add its own
///    `RepartitionExec(Hash)` cut markers; leaving the old layers in
///    place would stack redundant partitioners.
///  * `strip_dynamic_filters_in_subtree` removes the probe-side dynamic
///    filter the `FilterPushdown` physical optimizer pushed into the
///    `PgSearchScanPlan`. With a dynamic filter on the probe, rows
///    destined for peer participants get dropped before they hit the shuffle
///    (the build side hasn't populated the filter yet on this participant), and
///    the row count drops to ~0 across the mesh. The post-pass re-applies
///    the same `Arc<DynamicFilterPhysicalExpr>` as a `FilterExec` above
///    the post-shuffle probe output where it's safe.
///  * The HJ itself is rebuilt via
///    `HashJoinExecBuilder::with_new_children` so `dynamic_filter`
///    survives — `HashJoinExec::try_new` drops it.
///
/// `expect_group_by` — `false` for `ScalarAggOnBinaryJoin`, `true` for
/// `GroupByAggOnBinaryJoin`. Disagreement with the aggregate's actual
/// group_expr aborts with `DataFusionError::Plan`.
fn prepare_agg_on_binary_join(
    plan: Arc<dyn ExecutionPlan>,
    expect_group_by: bool,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let partial = find_partial_agg(plan.as_ref()).ok_or_else(|| {
        DataFusionError::Plan(
            "mpp: prepare_agg_on_binary_join: could not locate AggregateExec(Partial|Single) \
             in standard physical plan"
                .into(),
        )
    })?;
    let hash_join = find_hash_join(partial.input().as_ref()).ok_or_else(|| {
        DataFusionError::Plan(
            "mpp: prepare_agg_on_binary_join: AggregateExec child is not a HashJoinExec \
             (through coalesce layers) — plan shape unexpected for binary-join aggregate"
                .into(),
        )
    })?;

    let has_group_by = !partial.group_expr().expr().is_empty();
    if has_group_by != expect_group_by {
        return Err(DataFusionError::Plan(format!(
            "mpp: prepare_agg_on_binary_join: expected has_group_by={expect_group_by} but \
             aggregate has {} group keys",
            partial.group_expr().expr().len()
        )));
    }

    // HJ subtree cleanup.
    let new_left = strip_repartition_layers(Arc::clone(hash_join.left()));
    let new_right =
        strip_dynamic_filters_in_subtree(strip_repartition_layers(Arc::clone(hash_join.right())))?;
    let new_hj: Arc<dyn ExecutionPlan> = hash_join
        .builder()
        .with_new_children(vec![new_left, new_right])?
        .build_exec()?;

    // Force AggregateMode::Partial — the standard plan may have `Single`
    // (one-partition input), which `rewrite_with_cuts` doesn't match.
    let group_by = partial.group_expr().clone();
    let aggr_expr = partial.aggr_expr().to_vec();
    let filter_expr = partial.filter_expr().to_vec();
    let join_schema = new_hj.schema();
    let new_partial: Arc<dyn ExecutionPlan> = Arc::new(AggregateExec::try_new(
        AggregateMode::Partial,
        group_by,
        aggr_expr,
        filter_expr,
        new_hj,
        join_schema,
    )?);

    Ok(new_partial)
}

/// Find the first `HashJoinExec` in `plan` via a depth-first traversal
/// that descends into every child — unlike [`find_hash_join`] which only
/// walks single-child pass-through nodes. Needed in the aggregate
/// finalize paths because the walker output wraps the HJ inside
/// [`ChainExec`] (2 children: `ShuffleExec` + `DrainGatherExec`), which
/// the single-child walker would stop at.
///
/// Plans handled here contain exactly one `HashJoinExec`, so returning
/// the first hit is unambiguous.
fn find_hash_join_any(plan: &dyn ExecutionPlan) -> Option<&HashJoinExec> {
    if let Some(hj) = plan.as_any().downcast_ref::<HashJoinExec>() {
        return Some(hj);
    }
    for child in plan.children() {
        if let Some(found) = find_hash_join_any(child.as_ref()) {
            return Some(found);
        }
    }
    None
}

/// Find the first `AggregateExec(Partial)` in `plan` via a depth-first
/// traversal that descends into every child — the walker output wraps
/// the Partial inside `ChainExec` (2 children), so [`find_partial_agg`]'s
/// single-child rule doesn't reach it. Used by the aggregate finalize
/// paths to recover `aggr_expr` / `filter_expr` / output schema without
/// re-threading them through the pre-pass.
fn find_partial_agg_any(plan: &dyn ExecutionPlan) -> Option<&AggregateExec> {
    if let Some(agg) = plan.as_any().downcast_ref::<AggregateExec>() {
        if matches!(agg.mode(), AggregateMode::Partial) {
            return Some(agg);
        }
    }
    for child in plan.children() {
        if let Some(found) = find_partial_agg_any(child.as_ref()) {
            return Some(found);
        }
    }
    None
}

/// Build an MPP-shuffled replacement for the walker-output `HashJoinExec`,
/// re-applying the dynamic-filter predicate above the right probe as a
/// `FilterExec` (the original pushdown into `PgSearchScanPlan` was
/// stripped in the pre-pass). The rebuild goes through the builder so
/// `dynamic_filter` survives — `HashJoinExec::try_new` drops it. The
/// partition mode stays whatever the standard plan picked (typically
/// `Auto` / `CollectLeft`) because the walker-emitted `ChainExec` has a
/// single output partition, which DataFusion's execute-time assertions
/// accept under any mode.
fn build_partitioned_hj_with_probe_filter(hj: &HashJoinExec) -> DfResult<Arc<dyn ExecutionPlan>> {
    let hj_children = hj.children();
    if hj_children.len() != 2 {
        return Err(DataFusionError::Internal(format!(
            "mpp: build_partitioned_hj_with_probe_filter: HashJoinExec expected 2 children, got {}",
            hj_children.len()
        )));
    }
    let left_shuffled = Arc::clone(hj_children[0]);
    let right_shuffled = Arc::clone(hj_children[1]);

    let right_probe: Arc<dyn ExecutionPlan> = match hj.dynamic_filter_for_test().cloned() {
        Some(df) => Arc::new(FilterExec::try_new(df, right_shuffled)?),
        None => right_shuffled,
    };

    // Pin `PartitionMode::Partitioned` regardless of the inbound HJ's mode.
    // The native-partition pivot stamps both children with
    // `Partitioning::Hash(keys, N)` via [`MppPartitionAdapterExec`], so
    // CollectLeft's `left_partitions == 1` assertion would fail; Partitioned
    // is the matching join mode and aligns with the upstream
    // `MppPlanShape::JoinOnly` finalizer.
    hj.builder()
        .with_new_children(vec![left_shuffled, right_probe])?
        .with_partition_mode(PartitionMode::Partitioned)
        .build_exec()
}

/// Post-pass for [`MppPlanShape::ScalarAggOnBinaryJoin`]. The walker
/// produced a tree shaped like
///
/// ```text
/// CoalescePartitionsExec                             // preserved from
///                                                    // insert_mpp_cuts
///   ChainExec[scalar_final(FixedTargetPartitioner(0))]
///     ShuffleExec
///       AggregateExec(Partial, empty group)
///         HashJoinExec(original mode, dynamic_filter intact)
///           ChainExec[scalar_left(HashPartitioner(left_keys))]
///           ChainExec[scalar_right(HashPartitioner(right_keys))]
///     DrainGatherExec
/// ```
///
/// The outer `CoalescePartitionsExec` is preserved because [`annotate_plan`]
/// turned it into a `Plan` annotation while wrapping its child Partial in
/// the `Coalesce` boundary that [`emit_shuffle_cut`] then rewrites — so
/// the walker's `with_new_children` call on the CP node hands back the
/// rebuilt CP-over-Chain tree above.
///
/// This pass:
///  1. Emits the byte-exact `mpp: assembling ScalarAggOnBinaryJoin plan
///     (participant=, total=, aggr_count=, join_keys=)` warning line so
///     regress tests' `mpp_debug=on` output stays stable.
///  2. Rebuilds the HJ with its right probe wrapped in a
///     `FilterExec(dynamic_filter)` via
///     [`build_partitioned_hj_with_probe_filter`], then grafts the
///     replacement back through [`replace_first_hash_join`].
///  3. Leader-only: wraps the root with `AggregateExec(FinalPartitioned,
///     empty group)` using the Partial's `aggr_expr` / `filter_expr`.
///     Workers return the grafted tree as-is — their output stream's
///     `DrainGatherExec` reads nothing (every peer ships *to* the leader,
///     not to workers), so the worker emits zero rows and PG's Gather
///     sees exactly one row per query from the leader's final aggregate.
fn finalize_scalar_agg(
    plan: Arc<dyn ExecutionPlan>,
    mpp_state: &MppExecutionState,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let hj = find_hash_join_any(plan.as_ref()).ok_or_else(|| {
        DataFusionError::Internal(
            "mpp: finalize_scalar_agg: HashJoinExec missing after _distribute_plan".into(),
        )
    })?;
    let partial = find_partial_agg_any(plan.as_ref()).ok_or_else(|| {
        DataFusionError::Internal(
            "mpp: finalize_scalar_agg: AggregateExec(Partial) missing after _distribute_plan"
                .into(),
        )
    })?;

    let participant_index = mpp_state.participant_config().participant_index;
    let total_participants = mpp_state.participant_config().total_participants;
    let aggr_expr = partial.aggr_expr().to_vec();
    let filter_expr = partial.filter_expr().to_vec();
    let partial_schema = partial.schema();
    let join_on_len = hj.on().len();

    crate::mpp_log!(
        "mpp: assembling ScalarAggOnBinaryJoin plan (participant={}, total={}, \
         aggr_count={}, join_keys={})",
        participant_index,
        total_participants,
        aggr_expr.len(),
        join_on_len
    );

    let replacement_hj = build_partitioned_hj_with_probe_filter(hj)?;
    let grafted = replace_first_hash_join(plan, replacement_hj)?.ok_or_else(|| {
        DataFusionError::Internal(
            "mpp: finalize_scalar_agg: replace_first_hash_join could not find target".into(),
        )
    })?;

    if !mpp_state.is_leader() {
        return Ok(grafted);
    }

    let final_agg = AggregateExec::try_new(
        AggregateMode::FinalPartitioned,
        PhysicalGroupBy::new_single(vec![]),
        aggr_expr,
        filter_expr,
        grafted,
        partial_schema,
    )?;
    Ok(Arc::new(final_agg))
}

/// Post-pass for [`MppPlanShape::GroupByAggOnBinaryJoin`]. The walker
/// produced a tree shaped like
///
/// ```text
/// ChainExec[gb_postagg(HashPartitioner(group_keys))]
///   AggregateExec(Partial, group_by)
///     HashJoinExec(original mode, dynamic_filter intact)
///       ChainExec[gb_left(HashPartitioner(left_keys))]
///       ChainExec[gb_right(HashPartitioner(right_keys))]
/// ```
///
/// This pass:
///  1. Emits the byte-exact `mpp: assembling GroupByAggOnBinaryJoin plan
///     (participant=, total=, aggr_count=, group_keys=, join_keys=)`
///     warning line.
///  2. Rebuilds the HJ with its right probe wrapped in a
///     `FilterExec(dynamic_filter)` and grafts the replacement back via
///     [`replace_first_hash_join`].
///  3. Inserts `CoalesceBatchesExec(target = 64 Ki rows)` between the
///     Partial aggregate and the `gb_postagg` `ShuffleExec`. On the 25 M
///     row benchmark this collapses ~191 batches per participant to ~24, keeping
///     the post-agg shuffle payload under the 64 MiB shm_mq queue
///     capacity so backpressure stays near zero while `FinalPartitioned`
///     runs in parallel on every participant.
///  4. Wraps the root with `AggregateExec(FinalPartitioned, group_by)`
///     on every participant (no leader/worker asymmetry — each group lands on
///     exactly one participant via the group-key hash shuffle, so every participant's
///     Final emits a disjoint subset and PG's Gather concatenates
///     without double-counting).
fn finalize_groupby_agg(
    plan: Arc<dyn ExecutionPlan>,
    mpp_state: &MppExecutionState,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let hj = find_hash_join_any(plan.as_ref()).ok_or_else(|| {
        DataFusionError::Internal(
            "mpp: finalize_groupby_agg: HashJoinExec missing after _distribute_plan".into(),
        )
    })?;
    let partial = find_partial_agg_any(plan.as_ref()).ok_or_else(|| {
        DataFusionError::Internal(
            "mpp: finalize_groupby_agg: AggregateExec(Partial) missing after _distribute_plan"
                .into(),
        )
    })?;

    let participant_index = mpp_state.participant_config().participant_index;
    let total_participants = mpp_state.participant_config().total_participants;
    let group_by = partial.group_expr().clone();
    let num_group_keys = group_by.expr().len();
    let aggr_expr = partial.aggr_expr().to_vec();
    let filter_expr = partial.filter_expr().to_vec();
    let partial_schema = partial.schema();
    let join_on_len = hj.on().len();

    crate::mpp_log!(
        "mpp: assembling GroupByAggOnBinaryJoin plan (participant={}, total={}, \
         aggr_count={}, group_keys={}, join_keys={})",
        participant_index,
        total_participants,
        aggr_expr.len(),
        num_group_keys,
        join_on_len
    );

    // Phase 1: rebuild HJ with FilterExec on right probe, graft back.
    let replacement_hj = build_partitioned_hj_with_probe_filter(hj)?;
    let grafted = replace_first_hash_join(plan, replacement_hj)?.ok_or_else(|| {
        DataFusionError::Internal(
            "mpp: finalize_groupby_agg: replace_first_hash_join could not find target".into(),
        )
    })?;

    // Phase 2: insert `CoalesceBatchesExec(65_536)` between the Partial
    // aggregate and the `gb_postagg` ShuffleExec. Walks down to the
    // first ShuffleExec, wraps its single child in CoalesceBatches, and
    // rebuilds the ancestor chain via `with_new_children`. Shape-
    // agnostic with respect to the wrapper above ShuffleExec
    // (`wrap_with_mpp_shuffle` produces
    // `CoalescePartitionsExec(UnionExec(repartition, drain))`).
    let new_chain = wrap_first_shuffle_child_in_coalesce_batches(grafted)?;

    // Phase 3: wrap with `FinalPartitioned` on every participant.
    let final_agg = AggregateExec::try_new(
        AggregateMode::FinalPartitioned,
        group_by,
        aggr_expr,
        filter_expr,
        new_chain,
        partial_schema,
    )?;

    // Phase 4: collapse the FinalPartitioned's N output partitions back to a
    // single stream so PG's CustomScan (which only drives `execute(0)`) sees
    // the participant's complete output. The post-agg shuffle's
    // [`MppPartitionAdapterExec`] declares `Partitioning::Hash(group_keys, N)`
    // — without this gather, only partition 0's groups would surface to PG
    // and the other N-1 participants' contributions would be dropped.
    Ok(Arc::new(CoalescePartitionsExec::new(Arc::new(final_agg))))
}

/// Walk down `plan`, find the first `ShuffleExec`, and wrap its single
/// child in `CoalesceBatchesExec(65_536)`. Used by
/// [`finalize_groupby_agg`]'s Phase 2 to coalesce the post-Partial
/// payload before the post-agg shuffle.
///
/// Shape-agnostic: works regardless of the operator that
/// [`wrap_with_mpp_shuffle`] put above `ShuffleExec` (today
/// `CoalescePartitionsExec(UnionExec(shuffle, drain))`). The
/// `with_new_children` chain rebuilds the ancestor nodes; siblings of
/// the ShuffleExec are preserved by-reference.
fn wrap_first_shuffle_child_in_coalesce_batches(
    plan: Arc<dyn ExecutionPlan>,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    if plan.as_any().is::<MppShuffleExec>() {
        let children = plan.children();
        if children.len() != 1 {
            return Err(DataFusionError::Internal(format!(
                "mpp: finalize_groupby_agg: expected ShuffleExec with 1 child, got {}",
                children.len()
            )));
        }
        let shuffle_child = Arc::clone(children[0]);
        let coalesced: Arc<dyn ExecutionPlan> =
            Arc::new(CoalesceBatchesExec::new(shuffle_child, 65_536));
        return plan.with_new_children(vec![coalesced]);
    }
    let mut new_children = Vec::with_capacity(plan.children().len());
    let mut found = false;
    for child in plan.children() {
        if !found {
            let rebuilt = wrap_first_shuffle_child_in_coalesce_batches(Arc::clone(child))?;
            if !Arc::ptr_eq(&rebuilt, child) {
                found = true;
            }
            new_children.push(rebuilt);
        } else {
            new_children.push(Arc::clone(child));
        }
    }
    if !found {
        return Err(DataFusionError::Internal(
            "mpp: finalize_groupby_agg: no ShuffleExec found in plan".into(),
        ));
    }
    plan.with_new_children(new_children)
}

/// Post-pass for [`MppPlanShape::TopKGroupByAggOnBinaryJoin`]. Topology
/// is scalar-style: every participant ships its `Partial(group_by)`
/// output to participant 0 via `FixedTargetPartitioner(0)`; the leader
/// gathers, runs `AggregateExec(FinalPartitioned, group_by)` over the
/// merged groups, then applies `SortExec[fetch=k]` to resolve the
/// global Top-K. Workers return the grafted plan as-is (their `Shuffle`
/// pumps groups to leader, their `DrainGather` receives nothing because
/// no peer ships *to* them).
///
/// The `TopKSpec` carries the original `Sort` expressions and `fetch`
/// extracted from the standard plan by `extract_topk_spec`.
fn finalize_topk_groupby_agg(
    plan: Arc<dyn ExecutionPlan>,
    mpp_state: &MppExecutionState,
    topk: TopKSpec,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let hj = find_hash_join_any(plan.as_ref()).ok_or_else(|| {
        DataFusionError::Internal(
            "mpp: finalize_topk_groupby_agg: HashJoinExec missing after _distribute_plan".into(),
        )
    })?;
    let partial = find_partial_agg_any(plan.as_ref()).ok_or_else(|| {
        DataFusionError::Internal(
            "mpp: finalize_topk_groupby_agg: AggregateExec(Partial) missing after \
             _distribute_plan"
                .into(),
        )
    })?;

    let participant_index = mpp_state.participant_config().participant_index;
    let total_participants = mpp_state.participant_config().total_participants;
    let group_by = partial.group_expr().clone();
    let num_group_keys = group_by.expr().len();
    let aggr_expr = partial.aggr_expr().to_vec();
    let filter_expr = partial.filter_expr().to_vec();
    let partial_schema = partial.schema();
    let join_on_len = hj.on().len();

    crate::mpp_log!(
        "mpp: assembling TopKGroupByAggOnBinaryJoin plan (participant={}, total={}, \
         aggr_count={}, group_keys={}, join_keys={}, fetch={})",
        participant_index,
        total_participants,
        aggr_expr.len(),
        num_group_keys,
        join_on_len,
        topk.fetch
    );

    // Phase 1: rebuild HJ with FilterExec on right probe, graft back.
    // Same as the other agg shapes — the post-shuffle right probe needs
    // its dynamic filter re-applied above the shuffle.
    let replacement_hj = build_partitioned_hj_with_probe_filter(hj)?;
    let grafted = replace_first_hash_join(plan, replacement_hj)?.ok_or_else(|| {
        DataFusionError::Internal(
            "mpp: finalize_topk_groupby_agg: replace_first_hash_join could not find target".into(),
        )
    })?;

    // Workers ship their Partial groups to leader; they emit no rows
    // upward. Returning the grafted tree as-is matches scalar_agg's
    // worker path.
    if !mpp_state.is_leader() {
        return Ok(grafted);
    }

    // Leader: run FinalPartitioned over the gathered groups, then
    // SortExec[fetch=k] to pick the global Top-K.
    let final_agg: Arc<dyn ExecutionPlan> = Arc::new(AggregateExec::try_new(
        AggregateMode::FinalPartitioned,
        group_by,
        aggr_expr,
        filter_expr,
        grafted,
        partial_schema,
    )?);
    let topk_sort = SortExec::new(topk.sort_expr, final_agg).with_fetch(Some(topk.fetch));
    Ok(Arc::new(topk_sort))
}

// ============================================================================
// Mesh / shuffle primitives. `emit_shuffle_cut` takes one mesh slot out of
// `MppExecutionState`, spins up a drain, and attaches the cooperative-drain
// + frame-id stamps before handing off to `wrap_with_mpp_shuffle`. Shared
// across every shape; fully private to this module.
// ============================================================================

/// Single directed mesh extracted from the per-scan MPP state. A
/// participant-agnostic adapter over `LeaderMesh` / `WorkerMesh`, whose
/// shapes are identical but whose type-level variants force otherwise-
/// generic code to branch for no reason.
struct MeshHalves {
    outbound: Vec<Option<MppSender>>,
    inbound: Vec<Option<MppReceiver>>,
}

impl From<LeaderMesh> for MeshHalves {
    fn from(m: LeaderMesh) -> Self {
        MeshHalves {
            outbound: m.outbound,
            inbound: m.inbound,
        }
    }
}

impl From<WorkerMesh> for MeshHalves {
    fn from(m: WorkerMesh) -> Self {
        MeshHalves {
            outbound: m.outbound,
            inbound: m.inbound,
        }
    }
}

/// Take the per-mesh wirings out of `mpp_state.meshes`, replacing them with
/// empty `Vec`s so the borrow checker is satisfied. Drops the participant's
/// side-specific variant on the floor — only the generic `MeshHalves` survive.
///
/// Panics (via `pgrx::error!`) if `meshes.len() != expected`. This is a
/// leader/worker contract mismatch and must surface loudly, not silently.
fn take_meshes(state: &mut MppExecutionState, expected: usize) -> Vec<MeshHalves> {
    match state {
        MppExecutionState::Leader(ctx) => {
            let taken = mem::take(&mut ctx.meshes);
            if taken.len() != expected {
                let actual = taken.len();
                // Drop `taken` before the `pgrx::error!` longjmp so the held
                // shm_mq attachments run their `Drop` impls. Otherwise the
                // longjmp through Rust frames would skip them and leak the
                // attachments until backend exit.
                drop(taken);
                pgrx::error!(
                    "mpp: leader meshes.len()={} but shape expected {}",
                    actual,
                    expected
                );
            }
            taken.into_iter().map(MeshHalves::from).collect()
        }
        MppExecutionState::Worker(ctx) => {
            let taken = mem::take(&mut ctx.meshes);
            if taken.len() != expected {
                let actual = taken.len();
                drop(taken);
                pgrx::error!(
                    "mpp: worker meshes.len()={} but shape expected {}",
                    actual,
                    expected
                );
            }
            taken.into_iter().map(MeshHalves::from).collect()
        }
    }
}

fn spawn_drain(inbound: Vec<Option<MppReceiver>>) -> Arc<DrainHandle> {
    // `inbound[participant_index]` is always `None`; flatten drops it. Every
    // other peer contributes exactly one receiver.
    let receivers: Vec<_> = inbound.into_iter().flatten().collect();
    let num_sources = receivers.len();
    // `DrainBuffer::new(0)` would flip to EOF immediately; give it at least 1.
    let buffer = DrainBuffer::new(num_sources.max(1) as u32);

    // Cooperative (not thread-backed) drain: pgrx's `check_active_thread`
    // panics any pg FFI call from non-backend threads, so spawning a
    // `std::thread` to read from `shm_mq` would die on its first
    // `shm_mq_receive`. The drain work runs inline from
    // `DrainGatherStream::poll_next` on the backend thread; the returned
    // `Arc` is also held by each same-mesh `MppSender` so `send_batch` can
    // cooperatively poll the inbound during would-block retries, breaking
    // the symmetric-send deadlock on a single-threaded runtime.
    Arc::new(DrainHandle::cooperative(receivers, buffer))
}

/// Inject `drain` into each outbound sender so its `send_batch` can
/// cooperatively poll our inbound during would-block retries — breaks
/// the symmetric-send deadlock on a single-threaded runtime.
fn attach_cooperative_drain(
    senders: Vec<Option<MppSender>>,
    drain: &Arc<DrainHandle>,
) -> Vec<Option<MppSender>> {
    senders
        .into_iter()
        .map(|opt| opt.map(|s| s.with_cooperative_drain(Arc::clone(drain))))
        .collect()
}

/// Stamp every outbound sender in `senders` with a `FrameId` computed from
/// the shared `task_key` (one per mesh — query_id + stage_id + our
/// participant_index as `task_number`) plus the sender's position in the
/// `Vec` as `partition`. Position == destination participant index today: the
/// outbound vec is already built that way by `take_meshes`. P5b will
/// decouple the two when multiple logical streams share one shm_mq.
fn stamp_frame_ids(
    senders: Vec<Option<MppSender>>,
    task_key: MppTaskKey,
) -> Vec<Option<MppSender>> {
    senders
        .into_iter()
        .enumerate()
        .map(|(partition, opt)| opt.map(|s| s.with_frame_id(task_key, partition as u32)))
        .collect()
}

// ============================================================================
// Plan-walking primitives. Find the Partial agg / HashJoin in a standard
// DataFusion plan, skipping the coalesce/final wrapper layers the planner
// may have inserted.
// ============================================================================

/// Walk the plan from the root, descending only through an explicit
/// allow-list of pass-through wrappers, to find an `AggregateExec` in
/// `Partial` or `Single` mode.
///
/// Allow-list:
///   * `AggregateExec(Final | FinalPartitioned | SinglePartitioned)` —
///     the outer aggregate stage emitted by DataFusion's planner.
///   * `CoalescePartitionsExec` — multi→single partition merger.
///   * `CoalesceBatchesExec` — batch-size normaliser.
///
/// Anything else (a `ProjectionExec`, `FilterExec`, etc. above the
/// Partial) returns `None` so `prepare_agg_on_binary_join`'s caller
/// errors out and the dispatcher falls back to the serial path. This
/// is deliberately strict: the post-pass rebuilds the outer wrapper
/// chain from scratch as `AggregateExec(FinalPartitioned, …)` and would
/// silently discard any non-allow-listed node above the Partial,
/// producing wrong-shape rows. A tight allow-list turns that silent
/// drop into a clean fallback.
///
/// DataFusion's planner picks `AggregateMode::Single` when the input
/// produces exactly one partition (our usual serial build), and
/// `Partial + FinalPartitioned` when it produces multiple partitions —
/// both are equally valid as the "aggregate atop the join" we want to
/// rebuild from. The assembler always emits `Partial` on top of the
/// shuffled join, regardless of which mode the serial plan used.
fn find_partial_agg(plan: &dyn ExecutionPlan) -> Option<&AggregateExec> {
    if let Some(agg) = plan.as_any().downcast_ref::<AggregateExec>() {
        if matches!(agg.mode(), AggregateMode::Partial | AggregateMode::Single) {
            return Some(agg);
        }
        // Final / FinalPartitioned / SinglePartitioned: descend into its child.
        return find_partial_agg(agg.input().as_ref());
    }
    if let Some(cp) = plan.as_any().downcast_ref::<CoalescePartitionsExec>() {
        return find_partial_agg(cp.input().as_ref());
    }
    if let Some(cb) = plan.as_any().downcast_ref::<CoalesceBatchesExec>() {
        return find_partial_agg(cb.input().as_ref());
    }
    // Anything not in the allow-list above ends the walk. Returning
    // `None` lets `prepare_agg_on_binary_join` produce a clean
    // `DataFusionError::Plan` so the dispatcher falls back to serial,
    // rather than silently discarding the unknown wrapper.
    None
}

/// Find a `HashJoinExec` underneath a Partial aggregate's input (or
/// under the `standard` root when the plan is a bare join), tolerating
/// `CoalescePartitionsExec` / `CoalesceBatchesExec` layers the planner
/// may insert between them.
fn find_hash_join(plan: &dyn ExecutionPlan) -> Option<&HashJoinExec> {
    if let Some(hj) = plan.as_any().downcast_ref::<HashJoinExec>() {
        return Some(hj);
    }
    if let Some(cp) = plan.as_any().downcast_ref::<CoalescePartitionsExec>() {
        return find_hash_join(cp.input().as_ref());
    }
    if let Some(cb) = plan.as_any().downcast_ref::<CoalesceBatchesExec>() {
        return find_hash_join(cb.input().as_ref());
    }
    let children = plan.children();
    if children.len() == 1 {
        return find_hash_join(children[0].as_ref());
    }
    None
}

/// Runtime-installed indirection over `PgSearchScanPlan::strip_dynamic_filters_from_dyn`.
/// A direct call plants `<PgSearchScanPlan as ExecutionPlan>`'s vtable in every
/// caller's static reachable graph; under `cargo llvm-cov` (instrumentation
/// disables DCE) that pulls `execute()` → pgrx FFI → `CurrentMemoryContext`
/// as an unresolved GLOB_DAT reloc, breaking the lib-test binary's ld.so load.
/// Same mitigation pattern as `aggregatescan/filterquery.rs::BUILD_FILTER_QUERY_FN`.
type StripDynamicFiltersFn = fn(Arc<dyn ExecutionPlan>) -> DfResult<Arc<dyn ExecutionPlan>>;
static STRIP_DYNAMIC_FILTERS_FN: std::sync::OnceLock<StripDynamicFiltersFn> =
    std::sync::OnceLock::new();

/// Install the real implementation. Called from `_PG_init`; because `_PG_init`
/// is unreachable from `#[test]`, the function pointer stays out of the test
/// binary's static reachable graph.
pub fn init_mpp_strip_dynamic_filters() {
    STRIP_DYNAMIC_FILTERS_FN.get_or_init(|| PgSearchScanPlan::strip_dynamic_filters_from_dyn);
}

/// Register an extractor with the `datafusion-distributed` fork so its
/// [`NetworkBoundaryExt::as_network_boundary`] downcast chain recognizes
/// our [`MppShuffleExec`]. Without this, the fork's idempotency check
/// (`distribute_plan_with_factory`'s `original.exists(|p| p.is_network_boundary())`)
/// would mis-classify our plans as boundary-free and re-run distribution,
/// and the metrics rewriter would skip our boundaries when traversing.
///
/// Called from `_PG_init` alongside [`init_mpp_strip_dynamic_filters`];
/// the registration is idempotent at the consumer level (we wrap with
/// a `OnceLock` so repeated calls are cheap), and the fork's registry
/// itself is append-only — first-match-wins ensures no harm even if
/// the registration somehow fires twice.
pub fn init_mpp_network_boundary_extractor() {
    static REGISTERED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    REGISTERED.get_or_init(|| {
        datafusion_distributed::register_network_boundary_extractor(extract_mpp_shuffle_exec);
    });
}

fn extract_mpp_shuffle_exec(
    plan: &dyn datafusion::physical_plan::ExecutionPlan,
) -> Option<&dyn datafusion_distributed::NetworkBoundary> {
    plan.as_any()
        .downcast_ref::<crate::postgres::customscan::mpp::shuffle::MppShuffleExec>()
        .map(|e| e as &dyn datafusion_distributed::NetworkBoundary)
}

/// Walk a join-input subtree and replace every `PgSearchScanPlan` with a
/// copy whose `dynamic_filters` Vec is empty. The `FilterPushdown` physical
/// optimizer pushed the HashJoin's dynamic-filter Arc into the probe-side
/// scan at planning time; MPP rewires so the same Arc is applied by a
/// `FilterExec` above the post-shuffle output instead.
fn strip_dynamic_filters_in_subtree(
    node: Arc<dyn ExecutionPlan>,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    if node.as_any().downcast_ref::<PgSearchScanPlan>().is_some() {
        let f = STRIP_DYNAMIC_FILTERS_FN.get().ok_or_else(|| {
            DataFusionError::Internal(
                "mpp: init_mpp_strip_dynamic_filters() not called — should be wired from _PG_init"
                    .into(),
            )
        })?;
        return f(node);
    }

    let children = node.children();
    if children.is_empty() {
        return Ok(node);
    }

    let new_children = children
        .iter()
        .map(|c| strip_dynamic_filters_in_subtree(Arc::clone(c)))
        .collect::<Result<Vec<_>, _>>()?;
    node.with_new_children(new_children)
}

/// Strip `RepartitionExec` and `CoalesceBatchesExec` layers off the top of a
/// plan, returning the underlying child. DataFusion inserts these between a
/// `HashJoinExec(Partitioned)` and its inputs to hash-repartition; in the MPP
/// plan we replace them with our own shuffle.
fn strip_repartition_layers(plan: Arc<dyn ExecutionPlan>) -> Arc<dyn ExecutionPlan> {
    if let Some(rp) = plan.as_any().downcast_ref::<RepartitionExec>() {
        return strip_repartition_layers(Arc::clone(rp.input()));
    }
    if let Some(cb) = plan.as_any().downcast_ref::<CoalesceBatchesExec>() {
        return strip_repartition_layers(Arc::clone(cb.input()));
    }
    plan
}

fn col_index(expr: &Arc<dyn PhysicalExpr>) -> DfResult<usize> {
    expr.as_any()
        .downcast_ref::<Column>()
        .map(|c| c.index())
        .ok_or_else(|| {
            DataFusionError::Plan(format!(
                "mpp: join key expression {expr} is not a plain Column — MPP shuffle only \
                 supports column-ref keys in milestone 1"
            ))
        })
}

/// Walk `root` top-down, replacing the first `HashJoinExec` found with
/// `replacement`. Outer wrappers are rebuilt via `with_new_children` so their
/// identities (and per-node state) refresh, which is required for nodes like
/// `VisibilityFilterExec` whose resolver table must be re-wired against the
/// new subtree. Returns `Ok(None)` if no `HashJoinExec` is present.
fn replace_first_hash_join(
    root: Arc<dyn ExecutionPlan>,
    replacement: Arc<dyn ExecutionPlan>,
) -> DfResult<Option<Arc<dyn ExecutionPlan>>> {
    if root.as_any().downcast_ref::<HashJoinExec>().is_some() {
        return Ok(Some(replacement));
    }
    let children = root.children();
    if children.is_empty() {
        return Ok(None);
    }
    let mut new_children: Vec<Arc<dyn ExecutionPlan>> = Vec::with_capacity(children.len());
    let mut replaced = false;
    for child in children {
        if !replaced {
            if let Some(new_child) =
                replace_first_hash_join(Arc::clone(child), Arc::clone(&replacement))?
            {
                new_children.push(new_child);
                replaced = true;
                continue;
            }
        }
        new_children.push(Arc::clone(child));
    }
    if replaced {
        Ok(Some(root.with_new_children(new_children)?))
    } else {
        Ok(None)
    }
}

// ============================================================================
// Post-build validation
// ============================================================================

/// Post-build validation: no `VisibilityFilterExec` may be a descendant of a
/// `ShuffleExec` in the produced MPP plan.
///
/// Rationale (see module doc): `VisibilityFilterExec` resolves packed
/// DocAddress → heap TID via a ctid-resolver table populated with segments
/// local to this participant. If a shuffle sits above a visibility filter, the
/// filter operates on rows originating locally only (fine), but if a
/// visibility filter sits below a shuffle we run the filter on rows that
/// came in over shm_mq from a peer participant, whose `seg_ord` addresses the
/// peer's segment catalog rather than ours — `heap_fetch` then returns
/// garbage or panics.
///
/// The walk is cheap (O(nodes)) and runs once per query, so there is no
/// perf reason to gate this behind a GUC.
pub fn assert_visibility_invariant(plan: &Arc<dyn ExecutionPlan>) -> DfResult<()> {
    walk_checking_visibility(plan.as_ref(), /*inside_shuffle=*/ false)
}

fn walk_checking_visibility(node: &dyn ExecutionPlan, inside_shuffle: bool) -> DfResult<()> {
    let is_shuffle = node.as_any().downcast_ref::<MppShuffleExec>().is_some();
    let is_visibility = node
        .as_any()
        .downcast_ref::<VisibilityFilterExec>()
        .is_some();

    if inside_shuffle && is_visibility {
        return Err(DataFusionError::Plan(
            "mpp: visibility invariant violated — VisibilityFilterExec appears below a \
             ShuffleExec. Segment-local ctid resolution cannot run on rows that crossed \
             an shm_mq boundary from a peer participant (peer seg_ord addresses its own segment \
             catalog, not ours). Place every shuffle cut above any VisibilityFilterExec \
             in the standard plan."
                .into(),
        ));
    }

    let next_inside_shuffle = inside_shuffle || is_shuffle;
    for child in node.children() {
        walk_checking_visibility(child.as_ref(), next_inside_shuffle)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::customscan::mpp::transport::{in_proc_channel, MppSender};
    use datafusion::arrow::array::{Int32Array, RecordBatch};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::datasource::memory::MemorySourceConfig;
    use datafusion::logical_expr::Operator;
    use datafusion::physical_expr::expressions::{BinaryExpr, Literal};
    use datafusion::scalar::ScalarValue;

    #[test]
    fn cut_count_matches_topology() {
        assert_eq!(cut_count_for_shape(MppPlanShape::ScalarAggOnBinaryJoin), 3);
        assert_eq!(cut_count_for_shape(MppPlanShape::GroupByAggOnBinaryJoin), 3);
        assert_eq!(cut_count_for_shape(MppPlanShape::GroupByAggSingleTable), 1);
        assert_eq!(cut_count_for_shape(MppPlanShape::JoinOnly), 2);
        assert_eq!(cut_count_for_shape(MppPlanShape::Ineligible), 0);
    }

    #[test]
    fn worst_case_cut_count_matches_widest_shape() {
        // Scalar-agg-on-join and groupby-agg-on-join are tied at 3 cuts.
        // If someone adds a shape with more cuts, this assertion surfaces
        // the mismatch.
        assert_eq!(worst_case_cut_count(), 3);
    }

    #[test]
    fn stamp_frame_ids_assigns_destination_partition_per_slot() {
        // The vec slot index equals the destination participant today. If
        // `stamp_frame_ids` ever stops using the slot index as partition,
        // the wire format's routing guarantee breaks.
        let (tx1, _rx1) = in_proc_channel(1);
        let (tx3, _rx3) = in_proc_channel(1);
        let senders: Vec<Option<MppSender>> = vec![
            None, // participant 0 (self)
            Some(MppSender::new(Box::new(tx1))),
            None, // gap — simulates a partially-built mesh on a larger cluster
            Some(MppSender::new(Box::new(tx3))),
        ];

        let stage = MppStage::new(0xa5a5, 2, 4);
        let stamped = stamp_frame_ids(
            senders,
            MppTaskKey {
                query_id: stage.query_id,
                stage_id: stage.stage_id,
                task_number: 0, // pretend we're participant 0
            },
        );

        assert!(stamped[0].is_none());
        let f1 = stamped[1].as_ref().unwrap().frame_id().unwrap();
        assert_eq!(f1.partition, 1);
        assert_eq!(f1.task_key.stage_id, 2);
        assert_eq!(f1.task_key.task_number, 0);
        assert_eq!(f1.task_key.query_id, 0xa5a5);

        assert!(stamped[2].is_none());
        let f3 = stamped[3].as_ref().unwrap().frame_id().unwrap();
        assert_eq!(f3.partition, 3);
    }

    #[test]
    fn col_index_plain_column() {
        let col: Arc<dyn PhysicalExpr> = Arc::new(Column::new("id", 3));
        assert_eq!(col_index(&col).unwrap(), 3);
    }

    #[test]
    fn col_index_rejects_non_column() {
        let left: Arc<dyn PhysicalExpr> = Arc::new(Column::new("id", 0));
        let right: Arc<dyn PhysicalExpr> = Arc::new(Literal::new(ScalarValue::Int32(Some(0))));
        let expr: Arc<dyn PhysicalExpr> = Arc::new(BinaryExpr::new(left, Operator::Plus, right));
        let err = col_index(&expr).unwrap_err();
        assert!(
            format!("{err}").contains("not a plain Column"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn find_partial_agg_walks_through_coalesce_and_final() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(Int32Array::from(vec![1, 2, 3]))],
        )
        .unwrap();
        let input = MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch]).unwrap();

        // Build a Partial -> CoalescePartitions -> Final stack.
        let partial = Arc::new(
            AggregateExec::try_new(
                AggregateMode::Partial,
                PhysicalGroupBy::new_single(vec![]),
                vec![],
                vec![],
                input,
                schema.clone(),
            )
            .unwrap(),
        );
        let partial_schema = partial.schema();
        let coalesced: Arc<dyn ExecutionPlan> = Arc::new(CoalescePartitionsExec::new(partial));
        let final_agg = Arc::new(
            AggregateExec::try_new(
                AggregateMode::Final,
                PhysicalGroupBy::new_single(vec![]),
                vec![],
                vec![],
                coalesced,
                partial_schema,
            )
            .unwrap(),
        );

        let plan: Arc<dyn ExecutionPlan> = final_agg;
        let partial_ref = find_partial_agg(plan.as_ref()).expect("partial found");
        assert!(matches!(partial_ref.mode(), AggregateMode::Partial));
    }

    // The visibility-invariant walker is exercised by the regression tests
    // (mpp_join, mpp_exec) against real plans. A lightweight unit test on
    // synthetic `ExecutionPlan`s would require wiring up enough of
    // `VisibilityFilterExec` + `ShuffleExec` to make the downcast fire,
    // which duplicates the fixture the bridges already exercise. Skipped.

    // ========================================================================
    // DF-D generic cut walker tests (dead-path scaffolding — Step 2a).
    // ========================================================================

    fn mem_source(schema: &SchemaRef) -> Arc<dyn ExecutionPlan> {
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(Int32Array::from(vec![1, 2, 3]))],
        )
        .unwrap();
        MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch]).unwrap()
    }

    #[test]
    fn annotate_plan_detects_hash_repartition_as_shuffle() {
        let schema: SchemaRef =
            Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let input = mem_source(&schema);
        let key: Arc<dyn PhysicalExpr> = Arc::new(Column::new("id", 0));
        let repart: Arc<dyn ExecutionPlan> =
            Arc::new(RepartitionExec::try_new(input, Partitioning::Hash(vec![key], 2)).unwrap());

        let annotated = annotate_plan(repart, 2).unwrap();
        // The trigger sits *above* the RepartitionExec, so the top of the
        // annotated tree is the Shuffle boundary whose sole child is the
        // RepartitionExec-as-Plan.
        assert!(matches!(
            annotated.plan_or_nb,
            PlanOrNetworkBoundary::Shuffle
        ));
        assert_eq!(annotated.children.len(), 1);
        assert!(matches!(
            annotated.children[0].plan_or_nb,
            PlanOrNetworkBoundary::Plan(_)
        ));
    }

    #[test]
    fn annotate_plan_round_robin_repartition_does_not_trigger_shuffle() {
        let schema: SchemaRef =
            Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let input = mem_source(&schema);
        let repart: Arc<dyn ExecutionPlan> =
            Arc::new(RepartitionExec::try_new(input, Partitioning::RoundRobinBatch(2)).unwrap());

        let annotated = annotate_plan(repart, 2).unwrap();
        // Non-hash repartitioning is not a DF-D cut trigger.
        assert!(matches!(
            annotated.plan_or_nb,
            PlanOrNetworkBoundary::Plan(_)
        ));
    }

    #[test]
    fn annotate_plan_plain_plan_has_no_boundaries() {
        fn assert_no_boundary(a: &AnnotatedPlan) {
            assert!(
                !a.plan_or_nb.is_network_boundary(),
                "unexpected boundary: {:?}",
                a.plan_or_nb
            );
            for c in &a.children {
                assert_no_boundary(c);
            }
        }

        let schema: SchemaRef =
            Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let input = mem_source(&schema);
        let partial: Arc<dyn ExecutionPlan> = Arc::new(
            AggregateExec::try_new(
                AggregateMode::Partial,
                PhysicalGroupBy::new_single(vec![]),
                vec![],
                vec![],
                input,
                schema.clone(),
            )
            .unwrap(),
        );
        let annotated = annotate_plan(partial, 2).unwrap();
        assert_no_boundary(&annotated);
    }

    #[test]
    fn annotate_plan_coalesce_parent_triggers_coalesce_boundary() {
        let schema: SchemaRef =
            Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let input = mem_source(&schema);
        // Partial aggregation is non-leaf (children() non-empty) and its
        // parent is CoalescePartitionsExec — this is the DF-D Coalesce
        // trigger verbatim.
        let partial: Arc<dyn ExecutionPlan> = Arc::new(
            AggregateExec::try_new(
                AggregateMode::Partial,
                PhysicalGroupBy::new_single(vec![]),
                vec![],
                vec![],
                input,
                schema.clone(),
            )
            .unwrap(),
        );
        let coalesced: Arc<dyn ExecutionPlan> = Arc::new(CoalescePartitionsExec::new(partial));
        let annotated = annotate_plan(coalesced, 2).unwrap();

        // Root is the CoalescePartitionsExec itself (annotated as Plan; the
        // trigger only wraps the child that sits *under* the coalesce).
        assert!(matches!(
            annotated.plan_or_nb,
            PlanOrNetworkBoundary::Plan(_)
        ));
        assert_eq!(annotated.children.len(), 1);
        // The Partial aggregate has its parent as CoalescePartitionsExec, so
        // the walker wraps it with Coalesce.
        assert!(matches!(
            annotated.children[0].plan_or_nb,
            PlanOrNetworkBoundary::Coalesce
        ));
        // Inside the Coalesce annotation, the wrapped plan is the Partial
        // aggregate.
        assert_eq!(annotated.children[0].children.len(), 1);
        assert!(matches!(
            annotated.children[0].children[0].plan_or_nb,
            PlanOrNetworkBoundary::Plan(_)
        ));
    }

    #[test]
    fn annotate_plan_leaf_under_coalesce_is_not_boundaried() {
        // A true leaf node (no children) underneath CoalescePartitionsExec
        // must not get a Coalesce boundary — DF-D's comment: "putting a
        // network boundary above [a leaf] is a bit wasteful".
        let schema: SchemaRef =
            Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let input = mem_source(&schema);
        let coalesced: Arc<dyn ExecutionPlan> = Arc::new(CoalescePartitionsExec::new(input));
        let annotated = annotate_plan(coalesced, 2).unwrap();

        assert!(matches!(
            annotated.plan_or_nb,
            PlanOrNetworkBoundary::Plan(_)
        ));
        assert_eq!(annotated.children.len(), 1);
        assert!(matches!(
            annotated.children[0].plan_or_nb,
            PlanOrNetworkBoundary::Plan(_)
        ));
    }

    #[test]
    fn require_one_child_accepts_single_child() {
        let schema: SchemaRef =
            Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let input = mem_source(&schema);
        let got = require_one_child(vec![Arc::clone(&input)]).unwrap();
        assert!(Arc::ptr_eq(&got, &input));
    }

    #[test]
    fn require_one_child_rejects_zero_children() {
        let empty: Vec<Arc<dyn ExecutionPlan>> = vec![];
        let err = require_one_child(empty).unwrap_err();
        assert!(format!("{err}").contains("Expected exactly 1"));
    }

    #[test]
    fn require_one_child_rejects_many_children() {
        let schema: SchemaRef =
            Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let err = require_one_child(vec![mem_source(&schema), mem_source(&schema)]).unwrap_err();
        assert!(format!("{err}").contains("Expected exactly 1"));
    }

    // ========================================================================
    // `insert_mpp_cuts` tests (dead-path scaffolding — Step 2b).
    //
    // These exercise the aggregate-wrapping half of the pre-pass on
    // synthetic plans built from `MemorySourceConfig` +
    // `AggregateExec(Partial)`. The `HashJoinExec` half is exercised by
    // regression tests in Step 2d, when `insert_mpp_cuts` is wired into
    // `distribute_plan`'s live path — building a synthetic `HashJoinExec`
    // here would duplicate the fixture the regression tests already
    // produce from real SQL.
    // ========================================================================

    fn partial_agg_over_mem_source(group_cols: Vec<&str>) -> Arc<dyn ExecutionPlan> {
        let schema: SchemaRef =
            Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let input = mem_source(&schema);
        let group_by_exprs: Vec<(Arc<dyn PhysicalExpr>, String)> = group_cols
            .into_iter()
            .map(|c| {
                (
                    Arc::new(Column::new(c, 0)) as Arc<dyn PhysicalExpr>,
                    c.to_string(),
                )
            })
            .collect();
        Arc::new(
            AggregateExec::try_new(
                AggregateMode::Partial,
                PhysicalGroupBy::new_single(group_by_exprs),
                vec![],
                vec![],
                input,
                schema.clone(),
            )
            .unwrap(),
        )
    }

    #[test]
    fn insert_mpp_cuts_scalar_wraps_partial_with_coalesce() {
        let plan = partial_agg_over_mem_source(vec![]);
        let rewritten = insert_mpp_cuts(plan, MppPlanShape::ScalarAggOnBinaryJoin, 2).unwrap();

        // Root should be CoalescePartitionsExec; its sole child is the
        // original Partial aggregate.
        assert!(rewritten
            .as_any()
            .downcast_ref::<CoalescePartitionsExec>()
            .is_some());
        assert_eq!(rewritten.children().len(), 1);
        let partial = rewritten.children()[0]
            .as_any()
            .downcast_ref::<AggregateExec>()
            .expect("child is AggregateExec");
        assert_eq!(*partial.mode(), AggregateMode::Partial);

        // Cross-check: annotate_plan should see exactly one Coalesce boundary.
        let annotated = annotate_plan(Arc::clone(&rewritten), 2).unwrap();
        assert!(matches!(
            annotated.plan_or_nb,
            PlanOrNetworkBoundary::Plan(_)
        ));
        assert_eq!(annotated.children.len(), 1);
        assert!(matches!(
            annotated.children[0].plan_or_nb,
            PlanOrNetworkBoundary::Coalesce
        ));
    }

    #[test]
    fn insert_mpp_cuts_groupby_wraps_partial_with_hash_repartition() {
        let plan = partial_agg_over_mem_source(vec!["id"]);
        let rewritten = insert_mpp_cuts(plan, MppPlanShape::GroupByAggOnBinaryJoin, 4).unwrap();

        // Root should be RepartitionExec(Hash(id), 4); its sole child is
        // the original Partial aggregate.
        let repart = rewritten
            .as_any()
            .downcast_ref::<RepartitionExec>()
            .expect("root is RepartitionExec");
        match repart.partitioning() {
            Partitioning::Hash(keys, n) => {
                assert_eq!(keys.len(), 1);
                assert_eq!(*n, 4);
            }
            other => panic!("expected Hash partitioning, got {other:?}"),
        }
        assert_eq!(rewritten.children().len(), 1);
        let partial = rewritten.children()[0]
            .as_any()
            .downcast_ref::<AggregateExec>()
            .expect("child is AggregateExec");
        assert_eq!(*partial.mode(), AggregateMode::Partial);

        // Cross-check: annotate_plan should see a Shuffle boundary on the
        // RepartitionExec.
        let annotated = annotate_plan(Arc::clone(&rewritten), 2).unwrap();
        assert!(matches!(
            annotated.plan_or_nb,
            PlanOrNetworkBoundary::Shuffle
        ));
    }

    #[test]
    fn insert_mpp_cuts_scalar_does_not_wrap_groupby_partial() {
        // A Partial *with* group keys under ScalarAgg shape should not be
        // wrapped — rewrite_with_cuts matches on (kind, group_exprs.is_empty()).
        let plan = partial_agg_over_mem_source(vec!["id"]);
        let rewritten = insert_mpp_cuts(plan, MppPlanShape::ScalarAggOnBinaryJoin, 2).unwrap();

        // Root is still the Partial — no CoalescePartitionsExec, no RepartitionExec.
        assert!(rewritten.as_any().downcast_ref::<AggregateExec>().is_some());
        assert!(rewritten
            .as_any()
            .downcast_ref::<CoalescePartitionsExec>()
            .is_none());
    }

    #[test]
    fn insert_mpp_cuts_groupby_does_not_wrap_scalar_partial() {
        // A Partial *without* group keys under GroupByAgg shape should not
        // be wrapped. The caller's classifier is responsible for picking
        // the right shape — rewrite_with_cuts enforces the invariant by
        // no-op-ing on the mismatched pair.
        let plan = partial_agg_over_mem_source(vec![]);
        let rewritten = insert_mpp_cuts(plan, MppPlanShape::GroupByAggOnBinaryJoin, 2).unwrap();

        assert!(rewritten.as_any().downcast_ref::<AggregateExec>().is_some());
        assert!(rewritten
            .as_any()
            .downcast_ref::<RepartitionExec>()
            .is_none());
    }

    #[test]
    fn insert_mpp_cuts_join_only_does_not_wrap_partial() {
        // JoinOnly shape should leave any Partial aggregate alone — the
        // only rewrite is on HashJoinExec children (exercised in regression).
        let plan = partial_agg_over_mem_source(vec!["id"]);
        let rewritten = insert_mpp_cuts(plan, MppPlanShape::JoinOnly, 2).unwrap();

        assert!(rewritten.as_any().downcast_ref::<AggregateExec>().is_some());
    }

    #[test]
    fn insert_mpp_cuts_rejects_ineligible() {
        let plan = partial_agg_over_mem_source(vec![]);
        let err = insert_mpp_cuts(plan, MppPlanShape::Ineligible, 2).unwrap_err();
        assert!(format!("{err}").contains("Ineligible"));
    }

    #[test]
    fn insert_mpp_cuts_rejects_single_table_groupby() {
        let plan = partial_agg_over_mem_source(vec!["id"]);
        let err = insert_mpp_cuts(plan, MppPlanShape::GroupByAggSingleTable, 2).unwrap_err();
        assert!(format!("{err}").contains("GroupByAggSingleTable"));
    }

    // ========================================================================
    // `_distribute_plan` + `tag_for_cut` tests (dead-path scaffolding — Step
    // 2c-ii). The `Plan` arm exercises `with_new_children` recursion; the
    // `Shuffle` and `Coalesce` arms hit `wrap_with_mpp_shuffle` against a
    // synthetic in-process mesh and verify the resulting tree embeds the
    // expected number of `ShuffleExec` nodes with stamped `MppStage` ids.
    // ========================================================================

    #[test]
    fn tag_for_cut_covers_every_live_shape() {
        assert_eq!(tag_for_cut(MppPlanShape::JoinOnly, 0), "join_left");
        assert_eq!(tag_for_cut(MppPlanShape::JoinOnly, 1), "join_right");
        assert_eq!(
            tag_for_cut(MppPlanShape::ScalarAggOnBinaryJoin, 0),
            "scalar_left"
        );
        assert_eq!(
            tag_for_cut(MppPlanShape::ScalarAggOnBinaryJoin, 1),
            "scalar_right"
        );
        assert_eq!(
            tag_for_cut(MppPlanShape::ScalarAggOnBinaryJoin, 2),
            "scalar_final"
        );
        assert_eq!(
            tag_for_cut(MppPlanShape::GroupByAggOnBinaryJoin, 0),
            "gb_left"
        );
        assert_eq!(
            tag_for_cut(MppPlanShape::GroupByAggOnBinaryJoin, 1),
            "gb_right"
        );
        assert_eq!(
            tag_for_cut(MppPlanShape::GroupByAggOnBinaryJoin, 2),
            "gb_postagg"
        );
        assert_eq!(
            tag_for_cut(MppPlanShape::TopKGroupByAggOnBinaryJoin, 0),
            "tk_left"
        );
        assert_eq!(
            tag_for_cut(MppPlanShape::TopKGroupByAggOnBinaryJoin, 1),
            "tk_right"
        );
        assert_eq!(
            tag_for_cut(MppPlanShape::TopKGroupByAggOnBinaryJoin, 2),
            "tk_final"
        );
    }

    #[test]
    fn tag_for_cut_unknown_pair_falls_through_to_placeholder() {
        // Out-of-range indices and Ineligible / SingleTable shapes fall
        // through to the placeholder rather than panic — a panic in the
        // dead-code scaffold would mask a real emit-arm regression.
        assert_eq!(tag_for_cut(MppPlanShape::JoinOnly, 7), "mpp_unknown_cut");
        assert_eq!(tag_for_cut(MppPlanShape::Ineligible, 0), "mpp_unknown_cut");
        assert_eq!(
            tag_for_cut(MppPlanShape::GroupByAggSingleTable, 0),
            "mpp_unknown_cut"
        );
    }
}
