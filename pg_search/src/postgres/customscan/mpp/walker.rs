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
//! [`distribute_plan`] is the single entry point: turns a standard DataFusion
//! physical plan into its MPP equivalent. Cut detection derives the topology
//! from plan structure (locate `HashJoinExec` and any `AggregateExec(Partial|Single)`
//! above it). The caller's [`MppPlanShape`] is a cross-check, plus DSM
//! region sizing at plan time via [`cut_count_for_shape`].
//!
//! # Visibility correctness (invariant enforced here)
//!
//! `VisibilityFilterExec` resolves per-segment DocAddresses to heap TIDs
//! using segment-local Tantivy state and a ctid-resolver keyed by segments
//! **local to this participant**. Every `ShuffleExec` cut must sit inside
//! the subtree of every `VisibilityFilterExec`, never below one — otherwise
//! a row from participant A would reach B's resolver and look up the wrong
//! heap TID (or panic in `heap_fetch`). [`assert_visibility_invariant`]
//! walks the finished plan and aborts with `DataFusionError::Plan` if
//! violated. (Same constraint applies in principle to other segment-local
//! adornments — `SegmentedTopKExec`, `TantivyLookupExec`; only
//! `VisibilityFilterExec` is asserted today because it's the one that
//! crashes `heap_fetch`.)

// `CoalesceBatchesExec` is deprecated in favor of arrow-rs's streaming
// `BatchCoalescer`, but DataFusion 52 still emits it; we recognise + reuse it.
#![allow(deprecated)]

#[cfg(test)]
use datafusion_distributed::require_one_child;
use datafusion_distributed::{
    annotate_plan_sync, distribute_annotated_plan, register_network_boundary_extractor,
    BoundaryKind, DistributedConfig, NetworkBoundary, PooledBoundaryFactory,
};
#[cfg(test)]
use datafusion_distributed::{AnnotatedPlan, PlanOrNetworkBoundary};
use std::mem;
use std::sync::{Arc, OnceLock};
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
use datafusion::physical_plan::ExecutionPlan;

use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;
use crate::scan::visibility_ctid_resolver_rule::VisibilityCtidResolverRule;

use super::customscan_glue::MppExecutionState;
use super::plan_build::{wrap_with_mpp_shuffle, MppShuffleInputs};
use super::shape::MppPlanShape;
use super::shuffle::{
    FixedTargetPartitioner, HashPartitioner, MppShuffleExec, RowPartitioner, ShuffleWiring,
};
use super::transport::{DrainBuffer, DrainHandle, MppReceiver, MppSender};
use super::worker::{LeaderMesh, WorkerMesh};
use crate::scan::execution_plan::PgSearchScanPlan;
use datafusion_distributed::Stage;

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

/// Walk `standard`, insert `ShuffleExec` / `DrainGatherExec` pairs at the
/// cut points, and stamp each with an input [`Stage`]. `shape` is
/// cross-checked against the structural derivation; mismatch aborts with
/// `DataFusionError::Plan`. Returns after [`assert_visibility_invariant`]
/// passes — see module doc for the invariant.
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
        (None, Some(_)) => {
            validate_shape_matches(shape, MppPlanShape::JoinOnly)?;
            distribute_plan_generic(MppPlanShape::JoinOnly, standard, mpp_state, None)?
        }
        // Partial agg without HashJoin — e.g. GroupByAggSingleTable, not yet
        // wired.
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
// Generic cut walker. All three live shapes (`JoinOnly`,
// `ScalarAggOnBinaryJoin`, `GroupByAggOnBinaryJoin`) flow through
// `distribute_plan_generic` (`prepare_for_mpp` → `insert_mpp_cuts` →
// `annotate_plan_sync` → `distribute_annotated_plan` → `finalize_for_mpp`).
// `Ineligible` and `GroupByAggSingleTable` error out at the dispatcher.
//
// `annotate_plan_sync`, `AnnotatedPlan`, `PlanOrNetworkBoundary`,
// `BoundaryFactory`, and `distribute_annotated_plan` are imported from the
// `datafusion-distributed` fork. We use the fork's sync annotator (no
// `DistributedConfig`, no `TaskEstimator`, no `Broadcast` arm) because
// ParadeDB's participant count is fixed at plan time and we never emit
// broadcast annotations — [`emit_mpp_boundary`] errors loudly if the
// fork's walker ever asks for one.
// ============================================================================

/// Pre-pass: synthesize the `RepartitionExec(Hash)` / `CoalescePartitionsExec`
/// markers `annotate_plan_sync` treats as cut triggers. ParadeDB's standard
/// plan is single-partition and emits neither — so we add them per shape.
/// Bottom-up traversal so descendant rewrites compose with ancestor ones.
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

/// `PooledBoundaryFactory` emit body: wires a pre-allocated mesh into an
/// `MppShuffleExec` tagged for the shape + cut index. ParadeDB's `u64`
/// `query_id` round-trips as the low 64 bits of the fork's UUID via
/// `Uuid::from_u128(query_id as u128)`.
#[allow(clippy::too_many_arguments)]
fn emit_mpp_boundary(
    kind: BoundaryKind,
    mesh: MeshHalves,
    child: Arc<dyn ExecutionPlan>,
    stage_id: usize,
    shape: MppPlanShape,
    participant_index: u32,
    total_participants: u32,
    query_id: u64,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let cut_index = stage_id as u32;
    let tag = tag_for_cut(shape, cut_index);
    let stage = Stage::new_unaddressed(
        Uuid::from_u128(query_id as u128),
        cut_index as usize,
        total_participants as usize,
    );
    let drain = spawn_drain(mesh.inbound);
    let outbound = attach_cooperative_drain(mesh.outbound, &drain);

    match kind {
        BoundaryKind::Shuffle => {
            let (partitioner, underlying, hash_keys) =
                shuffle_keys_from_repartition(&child, total_participants)?;
            let wiring = ShuffleWiring {
                partitioner,
                outbound_senders: outbound,
                participant_index,
                cooperative_drain: Some(Arc::clone(&drain)),
            };
            let wrapped_schema = underlying.schema();
            wrap_with_mpp_shuffle(MppShuffleInputs {
                child: underlying,
                wiring,
                drain_handle: drain,
                wrapped_schema,
                tag,
                stage: Some(stage),
                hash_keys: Some(hash_keys),
                drive_partition: participant_index,
            })
        }
        BoundaryKind::Coalesce => {
            let partitioner: Arc<dyn RowPartitioner> =
                Arc::new(FixedTargetPartitioner::new(0, total_participants));
            let wiring = ShuffleWiring {
                partitioner,
                outbound_senders: outbound,
                participant_index,
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
                hash_keys: None,
                drive_partition: 0,
            })
        }
        BoundaryKind::Broadcast => Err(DataFusionError::Internal(
            "mpp: emit_mpp_boundary: unexpected Broadcast emit — \
             ParadeDB does not produce broadcast annotations"
                .into(),
        )),
    }
}

/// `(HashPartitioner, underlying child, hash-key exprs)` returned by
/// [`shuffle_keys_from_repartition`]. The exprs are forwarded into
/// `MppShuffleExec` so the wrapped output declares `Partitioning::Hash(keys, N)`
/// natively.
type ShuffleEmitInputs = (
    Arc<dyn RowPartitioner>,
    Arc<dyn ExecutionPlan>,
    Vec<Arc<dyn PhysicalExpr>>,
);

/// Extract `Hash(keys)` from the `RepartitionExec` marker `insert_mpp_cuts`
/// placed below the `Shuffle` boundary, build a `HashPartitioner`, and
/// return the underlying input (the `RepartitionExec` is dropped — the
/// `ShuffleExec` replaces it). Errors as `DataFusionError::Plan` if the
/// child isn't `RepartitionExec(Hash)` or any key isn't a plain `Column`.
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

/// Map `(shape, post-order cut index)` to the diagnostic tag string used by
/// `mpp_trace` and benchmark-log grep. Mis-indexed cuts fall through to
/// `"mpp_unknown_cut"` (a panic would mask emit-arm regressions).
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
// Generic MPP pipeline. `distribute_plan_generic` composes the shape-agnostic
// walker (`insert_mpp_cuts` → `annotate_plan_sync` → `distribute_annotated_plan`)
// with shape-specific pre-passes (`prepare_for_mpp`) and post-passes
// (`finalize_for_mpp`). ParadeDB-specific obligations (dynamic-filter strip,
// `PartitionMode::Partitioned` rebuild, `CoalesceBatchesExec(65_536)`,
// leader-only `FinalPartitioned`, `VisibilityCtidResolverRule` re-run) live
// in the pre/post passes; the walker body itself is shape-agnostic.
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

/// Detect the TopK shape: outer `SortExec[fetch=k]` (DataFusion's fused
/// TopK) or `GlobalLimitExec(SortExec)` above the Final aggregate. Walks
/// only `SortExec` / `GlobalLimitExec` / `LocalLimitExec`; anything else
/// stops the walk so the GroupByAgg dispatch handles it.
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

/// Run the generic pipeline: `prepare_for_mpp` → `insert_mpp_cuts` →
/// `annotate_plan_sync` → `distribute_annotated_plan` (driven by
/// [`PooledBoundaryFactory`]) → `finalize_for_mpp`.
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
    let participant_cfg = mpp_state.participant_config();
    let participant_index = participant_cfg.participant_index;
    let total_participants = participant_cfg.total_participants;
    let query_id = mpp_state.query_id();
    let expected_cuts = cut_count_for_shape(shape) as usize;

    let prepared = prepare_for_mpp(shape, standard)?;
    let with_cuts = insert_mpp_cuts(prepared, shape, total_participants)?;
    let annotated = annotate_plan_sync(with_cuts, total_participants as usize)?;

    // The walker's `stage_id` advances bottom-up once per emitted boundary,
    // matching the post-order indexing `tag_for_cut` and `Stage::new_unaddressed`
    // consume.
    let meshes: Vec<MeshHalves> = take_meshes(mpp_state, expected_cuts);
    let factory = PooledBoundaryFactory::new(
        meshes,
        move |kind, mesh, child, _qid, stage_id, _tc, _itc| {
            emit_mpp_boundary(
                kind,
                mesh,
                child,
                stage_id,
                shape,
                participant_index,
                total_participants,
                query_id,
            )
        },
    );

    // The walker requires `DistributedConfig` registered on `ConfigOptions`;
    // ParadeDB doesn't tune any of its knobs so defaults are correct.
    let mut cfg = ConfigOptions::default();
    cfg.extensions.insert(DistributedConfig::default());
    let walker_query_id = Uuid::nil();
    let mut stage_id_after: usize = 0;
    let emitted = distribute_annotated_plan(
        annotated,
        &cfg,
        walker_query_id,
        &mut stage_id_after,
        &factory,
    )?;
    factory.assert_drained()?;
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

/// JoinOnly pre-pass. Strips DF-inserted `RepartitionExec` /
/// `CoalesceBatchesExec` (they'd stack with `insert_mpp_cuts`'s own
/// markers) and the probe-side dynamic filter (`FilterPushdown`'s pushdown
/// would drop peer-bound rows before the shuffle, since the build side
/// hasn't filled the filter on this participant yet). Outer wrappers refresh
/// via `with_new_children`. Errors `DataFusionError::Plan` if no
/// `HashJoinExec` is found.
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

/// Rebuild the first `HashJoinExec` with `PartitionMode::Partitioned` and
/// graft it back via [`replace_first_hash_join`] so outer wrappers refresh.
/// `dynamic_filter` is intentionally dropped: under MPP, the local filter
/// only knows this participant's build keys, so a probe-side filter would
/// drop rows other participants' builds would have matched. The aggregate
/// paths re-apply it as a `FilterExec` above the shuffle (see
/// [`build_partitioned_hj_with_probe_filter`]); JoinOnly drops it outright.
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

/// Re-wire `VisibilityFilterExec.ctid_resolvers` after `with_new_children`
/// resets them.
fn run_visibility_ctid_resolver_rule(
    plan: Arc<dyn ExecutionPlan>,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let config = ConfigOptions::default();
    VisibilityCtidResolverRule.optimize(plan, &config)
}

/// Shared pre-pass for both aggregate-on-binary-join shapes. Locates the
/// topmost `AggregateExec(Partial|Single)` whose descendant is a HashJoin,
/// rebuilds the HJ subtree (preserving `dynamic_filter` via the builder),
/// normalizes the aggregate mode to `Partial`, and returns the Partial-
/// rooted subtree (post-pass rebuilds the right `FinalPartitioned` wrap).
/// Same dynamic-filter strip as `prepare_join_only`; the post-pass
/// re-applies it as a `FilterExec` above the post-shuffle probe.
///
/// `expect_group_by` is the shape's expected group-by-ness; mismatch
/// against the aggregate's actual `group_expr` aborts.
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
    // [`MppShuffleExec`] stamps both children with
    // `Partitioning::Hash(keys, N)`, so CollectLeft's `left_partitions == 1`
    // assertion would fail; Partitioned is the matching join mode and aligns
    // with the [`MppPlanShape::JoinOnly`] finalizer.
    hj.builder()
        .with_new_children(vec![left_shuffled, right_probe])?
        .with_partition_mode(PartitionMode::Partitioned)
        .build_exec()
}

/// Post-pass for [`MppPlanShape::ScalarAggOnBinaryJoin`]. The walker hands
/// us the shape:
///
/// ```text
/// CoalescePartitionsExec
///   MppShuffleExec[scalar_final(FixedTargetPartitioner(0))]
///     AggregateExec(Partial, empty group)
///       HashJoinExec(original mode, dynamic_filter intact)
///         MppShuffleExec[scalar_left(HashPartitioner(left_keys))]
///         MppShuffleExec[scalar_right(HashPartitioner(right_keys))]
/// ```
///
/// We rebuild the HJ with its right probe wrapped in `FilterExec(dynamic_filter)`,
/// graft it back, then on the leader only wrap the root with
/// `AggregateExec(FinalPartitioned, empty group)`. Workers' shuffles are
/// `FixedTargetPartitioner(0)` (everything ships to the leader), so their
/// own `DrainGatherExec` reads nothing and they emit zero rows — PG's
/// Gather sees exactly one row per query.
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

/// Post-pass for [`MppPlanShape::GroupByAggOnBinaryJoin`]. Walker hands us:
///
/// ```text
/// MppShuffleExec[gb_postagg(HashPartitioner(group_keys))]
///   AggregateExec(Partial, group_by)
///     HashJoinExec(original mode, dynamic_filter intact)
///       MppShuffleExec[gb_left(HashPartitioner(left_keys))]
///       MppShuffleExec[gb_right(HashPartitioner(right_keys))]
/// ```
///
/// We rebuild the HJ with `FilterExec(dynamic_filter)` on the right probe,
/// insert `CoalesceBatchesExec(65_536)` between the Partial and the
/// post-agg shuffle (collapses ~191 batches → ~24 on the 25M benchmark,
/// keeping the shuffle payload inside the shm_mq capacity so
/// `FinalPartitioned` runs in parallel without backpressure), wrap with
/// `AggregateExec(FinalPartitioned, group_by)` on every participant — no
/// leader/worker asymmetry because the group-key hash routes each group to
/// exactly one participant — then collapse the N-partition output back to
/// one stream via `CoalescePartitionsExec` so PG's CustomScan
/// (`execute(0)` only) sees every group.
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

    let replacement_hj = build_partitioned_hj_with_probe_filter(hj)?;
    let grafted = replace_first_hash_join(plan, replacement_hj)?.ok_or_else(|| {
        DataFusionError::Internal(
            "mpp: finalize_groupby_agg: replace_first_hash_join could not find target".into(),
        )
    })?;

    let new_chain = wrap_first_shuffle_child_in_coalesce_batches(grafted)?;

    let final_agg = AggregateExec::try_new(
        AggregateMode::FinalPartitioned,
        group_by,
        aggr_expr,
        filter_expr,
        new_chain,
        partial_schema,
    )?;

    Ok(Arc::new(CoalescePartitionsExec::new(Arc::new(final_agg))))
}

/// Walk down to the first `MppShuffleExec` and wrap its child in
/// `CoalesceBatchesExec(65_536)`. Ancestor chain rebuilds via
/// `with_new_children`; non-shuffle siblings preserved by-reference.
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

/// Post-pass for [`MppPlanShape::TopKGroupByAggOnBinaryJoin`]. Scalar-style
/// gather (`FixedTargetPartitioner(0)`): leader runs `FinalPartitioned`
/// over the merged groups then `SortExec[fetch=k]`; workers return the
/// grafted plan unchanged (their shuffle pumps to leader, their drain
/// receives nothing). `TopKSpec` carries the original `Sort` expressions
/// and `fetch` from the standard plan.
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

    let replacement_hj = build_partitioned_hj_with_probe_filter(hj)?;
    let grafted = replace_first_hash_join(plan, replacement_hj)?.ok_or_else(|| {
        DataFusionError::Internal(
            "mpp: finalize_topk_groupby_agg: replace_first_hash_join could not find target".into(),
        )
    })?;

    if !mpp_state.is_leader() {
        return Ok(grafted);
    }

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
static STRIP_DYNAMIC_FILTERS_FN: OnceLock<StripDynamicFiltersFn> = OnceLock::new();

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
    static REGISTERED: OnceLock<()> = OnceLock::new();
    REGISTERED.get_or_init(|| {
        register_network_boundary_extractor(extract_mpp_shuffle_exec);
    });
}

fn extract_mpp_shuffle_exec(plan: &dyn ExecutionPlan) -> Option<&dyn NetworkBoundary> {
    plan.as_any()
        .downcast_ref::<MppShuffleExec>()
        .map(|e| e as &dyn NetworkBoundary)
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
    // DF-D generic cut walker tests.
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
    fn annotate_plan_sync_detects_hash_repartition_as_shuffle() {
        let schema: SchemaRef =
            Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let input = mem_source(&schema);
        let key: Arc<dyn PhysicalExpr> = Arc::new(Column::new("id", 0));
        let repart: Arc<dyn ExecutionPlan> =
            Arc::new(RepartitionExec::try_new(input, Partitioning::Hash(vec![key], 2)).unwrap());

        let annotated = annotate_plan_sync(repart, 2).unwrap();
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
    fn annotate_plan_sync_round_robin_repartition_does_not_trigger_shuffle() {
        let schema: SchemaRef =
            Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let input = mem_source(&schema);
        let repart: Arc<dyn ExecutionPlan> =
            Arc::new(RepartitionExec::try_new(input, Partitioning::RoundRobinBatch(2)).unwrap());

        let annotated = annotate_plan_sync(repart, 2).unwrap();
        // Non-hash repartitioning is not a DF-D cut trigger.
        assert!(matches!(
            annotated.plan_or_nb,
            PlanOrNetworkBoundary::Plan(_)
        ));
    }

    #[test]
    fn annotate_plan_sync_plain_plan_has_no_boundaries() {
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
        let annotated = annotate_plan_sync(partial, 2).unwrap();
        assert_no_boundary(&annotated);
    }

    #[test]
    fn annotate_plan_sync_coalesce_parent_triggers_coalesce_boundary() {
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
        let annotated = annotate_plan_sync(coalesced, 2).unwrap();

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
    fn annotate_plan_sync_leaf_under_coalesce_is_not_boundaried() {
        // A true leaf node (no children) underneath CoalescePartitionsExec
        // must not get a Coalesce boundary — DF-D's comment: "putting a
        // network boundary above [a leaf] is a bit wasteful".
        let schema: SchemaRef =
            Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let input = mem_source(&schema);
        let coalesced: Arc<dyn ExecutionPlan> = Arc::new(CoalescePartitionsExec::new(input));
        let annotated = annotate_plan_sync(coalesced, 2).unwrap();

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
    // `insert_mpp_cuts` tests.
    //
    // These exercise the aggregate-wrapping half of the pre-pass on
    // synthetic plans built from `MemorySourceConfig` +
    // `AggregateExec(Partial)`. The `HashJoinExec` half is exercised by
    // the regression suite — building a synthetic `HashJoinExec` here
    // would duplicate the fixture the regression tests already produce
    // from real SQL.
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
        let annotated = annotate_plan_sync(Arc::clone(&rewritten), 2).unwrap();
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
        let annotated = annotate_plan_sync(Arc::clone(&rewritten), 2).unwrap();
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
    // expected number of `ShuffleExec` nodes with stamped input `Stage` ids.
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
