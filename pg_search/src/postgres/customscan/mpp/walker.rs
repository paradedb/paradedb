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

//! Generic cut walker (`annotate_plan`).
//!
//! Ported in spirit from datafusion-contrib/datafusion-distributed's
//! `src/distributed_planner/distribute_plan.rs`. Walks a DataFusion physical
//! plan once, identifies the cut points where the plan needs to cross a
//! network boundary, and rewrites those cuts to emit
//! [`ShuffleExec`](crate::postgres::customscan::mpp::shuffle::ShuffleExec) +
//! [`DrainGatherExec`](crate::postgres::customscan::mpp::shuffle::DrainGatherExec)
//! pairs whose [`MppNetworkBoundary::input_stage`] is stamped by the walker.
//!
//! # Why a generic walker at all
//!
//! The three shape-specific bridges
//! (`build_scalar_agg_on_binary_join_bridge`,
//! `build_groupby_agg_on_binary_join_bridge`, `build_join_only_bridge`) each
//! re-implement 90 % of the same logic (find the cut, strip DF repartition
//! layers, attach cooperative drain, build shuffle wiring, wrap with a
//! `FilterExec` re-applying the dynamic filter). Adding a fourth shape
//! (`GroupByAggSingleTable`, multi-table join) means a fourth copy. A single
//! walker that dispatches on plan structure collapses that to one path.
//!
//! # Visibility correctness (invariant enforced here)
//!
//! `VisibilityFilterExec` resolves per-segment packed DocAddress keys to
//! heap TIDs using segment-local Tantivy state plus a ctid-resolver table
//! keyed by `(plan_position, seg_ord)` that lists segments **local to this
//! seat**. Every [`ShuffleExec`] cut must therefore sit **inside** the subtree
//! of every [`VisibilityFilterExec`] the plan contains — i.e. no
//! `VisibilityFilterExec` may be a descendant of a `ShuffleExec`. Inverting
//! that placement means a row from seat A would reach seat B's resolver with
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
//! # Status
//!
//! Entry point [`annotate_plan`] is the canonical production path (P4) —
//! `exec_bridge::build_mpp_physical_plan` now just delegates here, and the
//! old `paradedb.mpp_use_generic_walker` GUC has been removed. The current
//! implementation is a shape-dispatching wrapper: each match arm calls a
//! private bridge function that owns the cut pattern for that shape.
//! Cut-point discovery is surfaced via [`expected_cuts`] for logging and
//! self-consistency checks; cut-insertion continues to live inside the
//! bridges until a follow-up landing carves it out without perturbing
//! byte-for-byte plan output.

use std::sync::Arc;

use datafusion::common::{DataFusionError, Result as DfResult};
use datafusion::physical_plan::ExecutionPlan;

use super::customscan_glue::MppExecutionState;
use super::shape::MppPlanShape;
use super::stage::MppStage;

/// Abstract description of one shuffle cut the walker plans to insert for a
/// given shape. Structured so that callers (e.g. tests, `mpp_log!` tracing)
/// can reason about cuts without dereferencing the opaque `ShuffleWiring`
/// built inside the bridges.
///
/// `stage_id` matches the mesh-index convention in [`super::exec_bridge`]:
/// left = 0, right = 1, final-or-postagg = 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CutDescriptor {
    /// Diagnostic label — also used as the `ShuffleExec` / `DrainGatherExec`
    /// `tag` so logs line up between the descriptor and the built nodes.
    pub tag: &'static str,
    /// Stage descriptor stamped on the boundary (via
    /// [`crate::postgres::customscan::mpp::stage::MppNetworkBoundary::with_input_stage`]).
    pub stage: MppStage,
    /// What kind of partitioner the cut uses. Abstract — concrete
    /// `RowPartitioner` construction happens inside the bridge that consumes
    /// this descriptor.
    pub partitioner_kind: PartitionerKind,
}

/// Partitioner flavor used by a cut. Mirrors the concrete `RowPartitioner`
/// impls in [`super::shuffle`] but without their transport-specific state, so
/// a descriptor is cheap to build and inspect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartitionerKind {
    /// `hash(row[keys]) % task_count` — the default join-key shuffle.
    /// `keys` holds column indices; they are populated by the bridge from
    /// the standard plan's join `on` list or the group-by expression list.
    /// The descriptor reports `Hash { keys: vec![] }` when the key list is
    /// shape-defined at build time (e.g. group-by columns) and filled in
    /// by the bridge, not statically known to the walker.
    Hash { keys: Vec<usize> },
    /// Every row routes to `target_seat` regardless of hash — used by the
    /// scalar-agg final-gather shuffle where only seat 0 finalizes.
    FixedTarget { target_seat: u32 },
}

/// Enumerate the cuts [`annotate_plan`] expects to produce for `shape`. Used
/// by tests and by `mpp_log!` tracing so an observer can cross-reference the
/// descriptor list against the nodes the bridge materialized.
///
/// Key columns are reported as empty vectors when they're not knowable from
/// `shape` alone (e.g. group-by columns are read from the Partial aggregate
/// node inside the bridge). The bridge populates the real indices when it
/// builds the `ShuffleWiring`. An empty `keys` in a `Hash` descriptor
/// therefore means "filled in later", not "no keys".
pub fn expected_cuts(
    shape: MppPlanShape,
    query_id: u64,
    total_participants: u32,
) -> Vec<CutDescriptor> {
    match shape {
        MppPlanShape::ScalarAggOnBinaryJoin => vec![
            CutDescriptor {
                tag: "scalar_left",
                stage: MppStage::new(query_id, 0, total_participants),
                partitioner_kind: PartitionerKind::Hash { keys: vec![] },
            },
            CutDescriptor {
                tag: "scalar_right",
                stage: MppStage::new(query_id, 1, total_participants),
                partitioner_kind: PartitionerKind::Hash { keys: vec![] },
            },
            CutDescriptor {
                tag: "scalar_final",
                stage: MppStage::new(query_id, 2, total_participants),
                partitioner_kind: PartitionerKind::FixedTarget { target_seat: 0 },
            },
        ],
        MppPlanShape::GroupByAggOnBinaryJoin => vec![
            CutDescriptor {
                tag: "gb_left",
                stage: MppStage::new(query_id, 0, total_participants),
                partitioner_kind: PartitionerKind::Hash { keys: vec![] },
            },
            CutDescriptor {
                tag: "gb_right",
                stage: MppStage::new(query_id, 1, total_participants),
                partitioner_kind: PartitionerKind::Hash { keys: vec![] },
            },
            CutDescriptor {
                tag: "gb_postagg",
                stage: MppStage::new(query_id, 2, total_participants),
                partitioner_kind: PartitionerKind::Hash { keys: vec![] },
            },
        ],
        MppPlanShape::JoinOnly => vec![
            CutDescriptor {
                tag: "join_left",
                stage: MppStage::new(query_id, 0, total_participants),
                partitioner_kind: PartitionerKind::Hash { keys: vec![] },
            },
            CutDescriptor {
                tag: "join_right",
                stage: MppStage::new(query_id, 1, total_participants),
                partitioner_kind: PartitionerKind::Hash { keys: vec![] },
            },
        ],
        MppPlanShape::GroupByAggSingleTable => vec![CutDescriptor {
            tag: "gb_single_postagg",
            stage: MppStage::new(query_id, 0, total_participants),
            partitioner_kind: PartitionerKind::Hash { keys: vec![] },
        }],
        MppPlanShape::Ineligible => vec![],
    }
}

/// How many shuffle cuts the walker will insert for `shape`. Replaces the
/// old `shape::shuffles_required` — derived from [`expected_cuts`] so the
/// two are guaranteed consistent. Called by the DSM-estimate hooks in
/// `aggregatescan/mod.rs` and `joinscan/mod.rs` to size the shared-memory
/// region at plan time.
pub fn cut_count_for_shape(shape: MppPlanShape) -> u32 {
    expected_cuts(shape, 0, 1).len() as u32
}

/// Walk `standard` and rewrite it into an MPP-partitioned plan by
/// identifying cut points and inserting `ShuffleExec` / `DrainGatherExec`
/// pairs stamped with an [`MppStage`].
///
/// `shape` is the classifier output from plan time. The walker uses it to
/// dispatch to the right cut pattern; a future truly shape-agnostic
/// implementation would re-derive the shape from the standard plan
/// structure, but keeping `shape` as an explicit parameter lets us validate
/// the classifier against the walker's derivation during the dark-launch.
///
/// Before returning, the built plan is validated against the visibility
/// invariant (see module doc). A violation aborts the query with
/// `DataFusionError::Plan` rather than risk a `heap_fetch` panic at
/// execution.
pub fn annotate_plan(
    standard: Arc<dyn ExecutionPlan>,
    mpp_state: &mut MppExecutionState,
    shape: MppPlanShape,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    use super::exec_bridge;

    let query_id = mpp_state.query_id();
    let total = mpp_state.participant_config().total_participants;
    let cuts = expected_cuts(shape, query_id, total);

    crate::mpp_log!(
        "mpp: annotate_plan walker dispatching shape={:?} total={} cuts={}",
        shape,
        total,
        cuts.len()
    );

    let built = match shape {
        MppPlanShape::ScalarAggOnBinaryJoin => {
            exec_bridge::build_scalar_agg_on_binary_join_bridge(standard, mpp_state)?
        }
        MppPlanShape::GroupByAggOnBinaryJoin => {
            exec_bridge::build_groupby_agg_on_binary_join_bridge(standard, mpp_state)?
        }
        MppPlanShape::JoinOnly => exec_bridge::build_join_only_bridge(standard, mpp_state)?,
        MppPlanShape::GroupByAggSingleTable => {
            return Err(DataFusionError::Plan(
                "mpp: annotate_plan: GroupByAggSingleTable shape not yet implemented \
                 (bridges do not support it either)"
                    .into(),
            ));
        }
        MppPlanShape::Ineligible => {
            return Err(DataFusionError::Plan(
                "mpp: annotate_plan invoked with Ineligible shape — caller should have \
                 routed to the non-MPP serial path"
                    .into(),
            ));
        }
    };

    assert_visibility_invariant(&built)?;
    Ok(built)
}

/// Post-build validation: no `VisibilityFilterExec` may be a descendant of a
/// `ShuffleExec` in the produced MPP plan.
///
/// Rationale (see module doc): `VisibilityFilterExec` resolves packed
/// DocAddress → heap TID via a ctid-resolver table populated with segments
/// local to this seat. If a shuffle sits above a visibility filter, the
/// filter operates on rows originating locally only (fine), but if a
/// visibility filter sits below a shuffle we run the filter on rows that
/// came in over shm_mq from a peer seat, whose `seg_ord` addresses the
/// peer's segment catalog rather than ours — `heap_fetch` then returns
/// garbage or panics.
///
/// The walk is cheap (O(nodes)) and runs once per query, so there is no
/// perf reason to gate this behind a GUC.
pub fn assert_visibility_invariant(plan: &Arc<dyn ExecutionPlan>) -> DfResult<()> {
    walk_checking_visibility(plan.as_ref(), /*inside_shuffle=*/ false)
}

fn walk_checking_visibility(node: &dyn ExecutionPlan, inside_shuffle: bool) -> DfResult<()> {
    use super::shuffle::ShuffleExec;
    use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;

    let is_shuffle = node.as_any().downcast_ref::<ShuffleExec>().is_some();
    let is_visibility = node
        .as_any()
        .downcast_ref::<VisibilityFilterExec>()
        .is_some();

    if inside_shuffle && is_visibility {
        return Err(DataFusionError::Plan(
            "mpp: visibility invariant violated — VisibilityFilterExec appears below a \
             ShuffleExec. Segment-local ctid resolution cannot run on rows that crossed \
             an shm_mq boundary from a peer seat (peer seg_ord addresses its own segment \
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

/// Upper bound on the number of cuts the walker can produce for any
/// supported shape. Used by the DSM estimate hook when the walker hasn't
/// been run yet (or when we don't want to risk running DataFusion physical
/// planning from inside the hook).
///
/// Safe to overestimate: an unused mesh wiring costs one `shm_mq` region
/// per edge and is dropped during `attach_cooperative_drain` / `take_meshes`.
/// The hook already rounds up via `MeshLayout`, so a small over-allocation
/// is cheap relative to benchmark-sized queue regions.
///
/// Currently unused: P4's DSM-estimate hook will call this when shape
/// classification hasn't yet run. Kept `pub` so P4 can wire it without
/// adjusting module visibility.
#[allow(dead_code)]
pub fn worst_case_cut_count() -> u32 {
    cut_count_for_shape(MppPlanShape::ScalarAggOnBinaryJoin)
        .max(cut_count_for_shape(MppPlanShape::GroupByAggOnBinaryJoin))
        .max(cut_count_for_shape(MppPlanShape::GroupByAggSingleTable))
        .max(cut_count_for_shape(MppPlanShape::JoinOnly))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cut_count_matches_expected_cuts_len() {
        for shape in [
            MppPlanShape::ScalarAggOnBinaryJoin,
            MppPlanShape::GroupByAggOnBinaryJoin,
            MppPlanShape::GroupByAggSingleTable,
            MppPlanShape::JoinOnly,
            MppPlanShape::Ineligible,
        ] {
            assert_eq!(
                cut_count_for_shape(shape) as usize,
                expected_cuts(shape, 0, 1).len(),
                "cut_count diverges from expected_cuts for {shape:?}"
            );
        }
    }

    #[test]
    fn worst_case_cut_count_matches_widest_shape() {
        // Scalar-agg-on-join and groupby-agg-on-join are tied at 3 cuts.
        // If someone adds a shape with more cuts, this assertion surfaces
        // the mismatch.
        assert_eq!(worst_case_cut_count(), 3);
    }

    #[test]
    fn expected_cuts_stamp_correct_stage_ids() {
        let cuts = expected_cuts(MppPlanShape::ScalarAggOnBinaryJoin, 42, 4);
        assert_eq!(cuts.len(), 3);
        assert_eq!(cuts[0].stage.stage_id, 0);
        assert_eq!(cuts[0].stage.query_id, 42);
        assert_eq!(cuts[0].stage.task_count, 4);
        assert_eq!(cuts[1].stage.stage_id, 1);
        assert_eq!(cuts[2].stage.stage_id, 2);
        assert!(matches!(
            cuts[2].partitioner_kind,
            PartitionerKind::FixedTarget { target_seat: 0 }
        ));
    }

    // The visibility-invariant walker is exercised by the regression tests
    // (mpp_join, mpp_exec) against real plans. A lightweight unit test on
    // synthetic `ExecutionPlan`s would require wiring up enough of
    // `VisibilityFilterExec` + `ShuffleExec` to make the downcast fire,
    // which duplicates the fixture the bridges already exercise. Skipped.
}
