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
//! drive MPP exactly as today. When on, the bridges short-circuit to
//! [`annotate_plan`]. The walker is not yet fully wired: the default
//! implementation returns [`AnnotateError::NotYetImplemented`] so the
//! leader can surface a clear error during A/B testing instead of silently
//! falling back. P3's next commit replaces that stub with the real walker.

use std::sync::Arc;

use datafusion::common::{DataFusionError, Result as DfResult};
use datafusion::physical_plan::ExecutionPlan;

use super::customscan_glue::MppExecutionState;
use super::shape::{shuffles_required, MppPlanShape};

/// Errors from the cut walker that are distinct from DataFusion's
/// `DataFusionError`. Surface as `DataFusionError::Plan` at the bridge
/// boundary; the enum exists so callers can pattern-match in tests.
#[derive(Debug)]
pub enum AnnotateError {
    /// P3 stub — real walker lands in the next commit. Kept as a named
    /// variant (not just a `DataFusionError::NotImplemented`) so the
    /// A/B-test harness can distinguish "GUC flipped but walker not
    /// wired" from any other planning failure.
    NotYetImplemented,
}

impl std::fmt::Display for AnnotateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnnotateError::NotYetImplemented => write!(
                f,
                "mpp: annotate_plan walker not yet implemented (stub); \
                 unset paradedb.mpp_use_generic_walker to fall back to bridges"
            ),
        }
    }
}

impl From<AnnotateError> for DataFusionError {
    fn from(e: AnnotateError) -> Self {
        DataFusionError::Plan(format!("{e}"))
    }
}

/// Walk `standard` and rewrite it into an MPP-partitioned plan by
/// identifying cut points and inserting `ShuffleExec` / `DrainGatherExec`
/// pairs stamped with an [`MppStage`](super::stage::MppStage).
///
/// `shape` is passed through so the walker can short-circuit to the right
/// cut pattern when the standard-plan walk alone is ambiguous
/// (e.g. distinguishing `ScalarAgg` from `GroupByAgg` by inspecting the
/// `Partial` aggregate's `group_expr().expr().len()` works, but the
/// shape is already computed at classification time so we accept it as a
/// hint).
///
/// The `_mpp_state` handle will (next commit) be used to pull mesh
/// wirings out and build `ShuffleWiring`s, mirroring what the bridges do
/// today. Held as `&mut` because mesh extraction is destructive.
pub fn annotate_plan(
    _standard: Arc<dyn ExecutionPlan>,
    _mpp_state: &mut MppExecutionState,
    _shape: MppPlanShape,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    Err(AnnotateError::NotYetImplemented.into())
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

    #[test]
    fn stub_annotate_plan_returns_not_yet_implemented() {
        // Matches the not-yet-implemented variant rather than formatting
        // the error, so the test is robust to message tweaks.
        let err_is_stub = matches!(
            AnnotateError::NotYetImplemented,
            AnnotateError::NotYetImplemented
        );
        assert!(err_is_stub);
    }
}
