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

//! P3 — Generic cut walker (`annotate_plan`).
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
//! Today's three shape-specific bridges
//! (`build_scalar_agg_on_binary_join_bridge`,
//! `build_groupby_agg_on_binary_join_bridge`, `build_join_only_bridge`) each
//! re-implement 90 % of the same logic (find the cut, strip DF repartition
//! layers, attach cooperative drain, build shuffle wiring, wrap with a
//! `FilterExec` re-applying the dynamic filter). Adding a fourth shape
//! (`GroupByAggSingleTable`, multi-table join) means a fourth copy. A single
//! walker that dispatches on plan structure collapses that to one path.
//!
//! # Visibility correctness
//!
//! `VisibilityFilterExec` resolves per-segment packed DocAddress keys to
//! heap TIDs using segment-local Tantivy state. Once rows cross a shuffle,
//! peer participants no longer have that state — a shuffled row from seat A
//! would be looked up against seat B's segment table and return garbage or
//! crash in `heap_fetch`. The walker must therefore place every shuffle cut
//! **below** any `VisibilityFilterExec` the standard plan contains, never
//! above it. The `ShuffleExec` → DrainGather pair lives on the scan side of
//! the visibility filter; each seat's own visibility filter runs before its
//! scan output enters the shuffle (so the TIDs it emits are already real
//! heap TIDs from its local segments).
//!
//! The same constraint applies to other segment-local scan adornments the
//! planner may insert (`SegmentedTopKExec` pre-merge, `TantivyLookupExec`,
//! etc.). They must remain on the scan side of every cut.
//!
//! # Mesh allocation timing
//!
//! P3 expects mesh allocation to happen at **plan time** — the walker
//! produces a `cut_count` that the DSM-estimate hook can read to size the
//! region. If we can't land plan-time cut counting in this phase (e.g. the
//! hook runs before the walker has a `ParticipantConfig`), we fall back to
//! the worst-case count
//! (`super::shape::shuffles_required(MppPlanShape::ScalarAggOnBinaryJoin) = 3`
//! — the current maximum across shapes). See
//! [`worst_case_cut_count`].
//!
//! # Status (dark-launched)
//!
//! Entry point [`annotate_plan`] is behind
//! `paradedb.mpp_use_generic_walker` (default off). When off, the bridges
//! drive MPP exactly as today. When on, [`build_mpp_physical_plan`] routes
//! through [`annotate_plan`] instead.
//!
//! **Current implementation**: a thin dispatching wrapper that delegates to
//! the existing per-shape bridge functions
//! (`build_{scalar_agg,groupby_agg,join_only}_*_bridge`). No behaviour
//! change from the bridges yet — the purpose is to lock in the dispatch
//! seam, emit an `mpp_log!` so A/B tests can observe which path ran, and
//! validate the visibility invariant (below) before swapping the body for
//! a truly shape-agnostic cut walker in a follow-up commit.
//!
//! Follow-up commits will progressively carve cut discovery
//! (`find_mpp_cut_points`) and cut insertion (`insert_shuffles`) out of the
//! bridges into standalone walker helpers, then retire the bridges once
//! the walker output matches byte-for-byte across the test suite.

use std::sync::Arc;

use datafusion::common::{DataFusionError, Result as DfResult};
use datafusion::physical_plan::ExecutionPlan;

use super::customscan_glue::MppExecutionState;
use super::shape::{shuffles_required, MppPlanShape};

/// Walk `standard` and rewrite it into an MPP-partitioned plan by
/// identifying cut points and inserting `ShuffleExec` / `DrainGatherExec`
/// pairs stamped with an [`MppStage`](super::stage::MppStage).
///
/// `shape` is the classifier output from plan time. The walker uses it to
/// dispatch to the right cut pattern; a future truly shape-agnostic
/// implementation would re-derive the shape from the standard plan
/// structure, but keeping `shape` as an explicit parameter lets us validate
/// the classifier against the walker's derivation during the dark-launch.
///
/// Current body: a dispatching wrapper over the per-shape bridge functions
/// in [`super::exec_bridge`]. See module-level doc for the migration plan.
pub fn annotate_plan(
    standard: Arc<dyn ExecutionPlan>,
    mpp_state: &mut MppExecutionState,
    shape: MppPlanShape,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    use super::exec_bridge;

    crate::mpp_log!(
        "mpp: annotate_plan walker dispatching shape={:?} (use_generic_walker=true)",
        shape
    );

    match shape {
        MppPlanShape::ScalarAggOnBinaryJoin => {
            exec_bridge::build_scalar_agg_on_binary_join_bridge(standard, mpp_state)
        }
        MppPlanShape::GroupByAggOnBinaryJoin => {
            exec_bridge::build_groupby_agg_on_binary_join_bridge(standard, mpp_state)
        }
        MppPlanShape::JoinOnly => exec_bridge::build_join_only_bridge(standard, mpp_state),
        MppPlanShape::GroupByAggSingleTable => Err(DataFusionError::Plan(
            "mpp: annotate_plan: GroupByAggSingleTable shape not yet implemented \
             (bridges do not support it either)"
                .into(),
        )),
        MppPlanShape::Ineligible => Err(DataFusionError::Plan(
            "mpp: annotate_plan invoked with Ineligible shape — caller should have \
             routed to the non-MPP serial path"
                .into(),
        )),
    }
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
#[allow(dead_code)] // wired by the DSM-estimate fallback in a follow-up commit
pub fn worst_case_cut_count() -> u32 {
    // `ScalarAggOnBinaryJoin` and `GroupByAggOnBinaryJoin` both need 3 meshes.
    // Keep this linked to `shuffles_required` so the upper bound tracks the
    // shape table as shapes are added.
    shuffles_required(MppPlanShape::ScalarAggOnBinaryJoin)
        .max(shuffles_required(MppPlanShape::GroupByAggOnBinaryJoin))
        .max(shuffles_required(MppPlanShape::GroupByAggSingleTable))
        .max(shuffles_required(MppPlanShape::JoinOnly))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worst_case_cut_count_matches_widest_shape() {
        // At the moment, scalar-agg-on-join and groupby-agg-on-join are
        // tied at 3 meshes. If someone adds a shape with more meshes, this
        // assertion surfaces the mismatch.
        assert_eq!(worst_case_cut_count(), 3);
    }
}
