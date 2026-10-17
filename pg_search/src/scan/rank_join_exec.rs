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

//! Pull-based symmetric hash rank join (`RankJoinExec`) combining Sorted Access (`SA`)
//! and lazy Random Access (`RA`) point lookups (`CA` / `MPro` / `HRJN*` / `NRJN`).
//!
//! # Algorithmic Overview
//!
//! `RankJoinExec` synthesizes relational rank join (`HRJN*` / `NRJN`, Ilyas et al. VLDB 2003 /
//! SIGMOD 2004; `PBRJ`, Schnaitter & Polyzotis PODS 2008) with lazy random-access probing
//! (`CA`, Fagin et al. PODS 2001; `MPro` / `Upper`, Chang & Hwang SIGMOD 2002, Bruno et al.
//! ICDE 2002) over a monotonic scoring function `f(s_l, s_r)`:
//!
//! - **Three-Quadrant Upper Bounds & Two-Tier `RA` Probing**:
//!   At any point, unseen join pairs belong to one of three quadrants bounded by:
//!   1. `Seen Unresolved Left x Unseen Right`: `u_ra_l = max(f(s_l_unres_pos, s_r_bottom), f(s_l_unres_zero, 0))`
//!   2. `Unseen Left x Seen Unresolved Right`: `u_ra_r = max(f(s_l_bottom, s_r_unres_pos), f(0, s_r_unres_zero))`
//!   3. `Unseen Left x Unseen Right`: `u_sa = f(s_l_bottom, s_r_bottom)`
//!
//!   The stopping threshold is `T = max(u_ra_l, u_ra_r, u_sa)`.
//!   Without `RA`, `s_unres` remains pinned at `s_top` (the classic `HRJN*`
//!   corner bound), so a single high-scoring orphan row on one side stalls emission until the
//!   opposite stream is scanned deeply. By lazily probing only the top unresolved keys whose
//!   bound exceeds `u_sa` via batched [`RankLookupPhysicalExpr`] (`TermSetQuery`) —
//!   split into Tier-1 (`score > 0.0` via `PruningScorer`) and Tier-2 (`score = 0.0` for
//!   disjunctive partners that did not match the opposite table's search query) — `T` collapses
//!   to `u_sa = f(s_l_bottom, s_r_bottom)` without scanning the
//!   tail of either input or scanning `0.0`-score rows during `SA`.
//!
//! - **Single-Sided Ordered Joins (`NRJN` Special Case)**:
//!   When only one child contributes to the sort expression (`combiner.uses_left() !=
//!   combiner.uses_right()`), the unscored child disables `SA` (`initial_window = 0.0`,
//!   `sa_exhausted = true`, `s_top = s_bottom = 0.0`) and executes purely
//!   via batched `RA` point lookups driven by each `SA` window of the ordered child.
//!
//! - **Composable `N >= 3` Multi-Way Joins (`parent_lookup_filters`)**:
//!   Because `RankJoinExec`'s output is itself a lazily-sorted stream that accepts incoming
//!   [`RankLookupPhysicalExpr`] requests via `handle_child_pushdown_result`, nested
//!   `RankJoinExec` nodes cascade `RA` lookups down to the child that owns the requested join
//!   key, resolve any newly discovered partner keys against their sibling child, and track
//!   emitted pairs in `emitted_pairs` so no pair is emitted twice across `RA` and `SA`.
//!
//! - **In-Band `DynamicFilterPhysicalExpr` Signaling Protocol**:
//!   - Parent `->` Child: publishes `RankLookupPhysicalExpr` into the child's
//!     `DynamicFilterPhysicalExpr` before polling to request an `RA` point-lookup batch; the
//!     child resets the filter to `lit(true)` upon servicing.
//!   - Child `->` Parent: when the child's `SA` stream is exhausted, the child sets
//!     its direct parent's `DynamicFilterPhysicalExpr` to `lit(false)` if staying alive to
//!     service future `RA` lookups, or to `Literal(ScalarValue::Boolean(None))` when neither
//!     `SA` nor `RA` remains.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt;
use std::sync::Arc;

use crate::api::{HashMap, HashSet};
use crate::scan::pre_filter::RankLookupPhysicalExpr;
use arrow_array::{Array, ArrayRef, BooleanArray, Float64Array, RecordBatch, RecordBatchOptions};
use arrow_schema::{DataType, SchemaRef};
use datafusion::arrow::compute::{cast, interleave};
use datafusion::arrow::row::{OwnedRow, RowConverter, SortField};
use datafusion::common::config::ConfigOptions;
use datafusion::common::tree_node::{Transformed, TreeNode, TreeNodeRecursion};
use datafusion::common::{
    DataFusionError, JoinSide, NullEquality, Result, ScalarValue, internal_err, project_schema,
};
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::logical_expr::JoinType;
use datafusion::physical_expr::equivalence::{ProjectionMapping, join_equivalence_properties};
use datafusion::physical_expr::expressions::{Column, DynamicFilterPhysicalExpr, Literal, lit};
use datafusion::physical_expr::{
    Distribution, LexOrdering, OrderingRequirements, Partitioning, PhysicalExpr, PhysicalSortExpr,
};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::filter_pushdown::{
    ChildPushdownResult, FilterDescription, FilterPushdownPhase, FilterPushdownPropagation,
    PushedDown,
};
use datafusion::physical_plan::joins::PartitionMode;
use datafusion::physical_plan::joins::utils::{
    ColumnIndex, JoinFilter, JoinOn, JoinOnRef, adjust_right_output_partitioning,
    build_join_schema, check_join_is_valid,
};
use datafusion::physical_plan::metrics::{
    BaselineMetrics, ExecutionPlanMetricsSet, MetricsSet, RecordOutput, Time,
};
use datafusion::physical_plan::sorts::sort::sort_batch;
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, ExecutionPlanProperties,
    InputDistributionRequirements, PlanProperties,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

/// Initial number of unresolved keys batched into an on-demand Random Access (`RA`) lookup.
/// Scales by `topk_retry_scale_factor` on subsequent `RA` pulls up to `max_topk_chunk_size`.
const RA_INITIAL_BATCH_SIZE: usize = 64;

/// Pair of `(left, right)` input scores evaluated by [`ScoreCombiner`].
#[derive(Debug, Clone, Copy)]
pub struct InputScorePair {
    pub left: f64,
    pub right: f64,
}

impl InputScorePair {
    fn for_side(side: AdvanceSide, side_score: f64, opp_score: f64) -> Self {
        match side {
            AdvanceSide::Left => Self {
                left: side_score,
                right: opp_score,
            },
            AdvanceSide::Right => Self {
                left: opp_score,
                right: side_score,
            },
        }
    }
}

/// Monotonic scoring function `f(s_l, s_r)` combining left and right input scores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScoreCombiner {
    LeftScore,
    RightScore,
    Constant(f64),
    Add(Box<ScoreCombiner>, Box<ScoreCombiner>),
    Mul(Box<ScoreCombiner>, Box<ScoreCombiner>),
}

impl ScoreCombiner {
    /// Evaluates `f(s_l, s_r)` for the given [`InputScorePair`].
    pub fn eval(&self, scores: InputScorePair) -> f64 {
        let raw = match self {
            ScoreCombiner::LeftScore => scores.left,
            ScoreCombiner::RightScore => scores.right,
            ScoreCombiner::Constant(c) => *c,
            ScoreCombiner::Add(a, b) => a.eval(scores) + b.eval(scores),
            ScoreCombiner::Mul(a, b) => a.eval(scores) * b.eval(scores),
        };
        if raw.is_nan() { f64::NEG_INFINITY } else { raw }
    }

    fn uses_side(&self, side: AdvanceSide) -> bool {
        match (self, side) {
            (ScoreCombiner::LeftScore, AdvanceSide::Left)
            | (ScoreCombiner::RightScore, AdvanceSide::Right) => true,
            (
                ScoreCombiner::LeftScore | ScoreCombiner::RightScore | ScoreCombiner::Constant(_),
                _,
            ) => false,
            (ScoreCombiner::Add(a, b) | ScoreCombiner::Mul(a, b), _) => {
                a.uses_side(side) || b.uses_side(side)
            }
        }
    }

    /// Returns whether the combined score depends on the left child's score (`LeftScore`).
    pub fn uses_left(&self) -> bool {
        self.uses_side(AdvanceSide::Left)
    }

    /// Returns whether the combined score depends on the right child's score (`RightScore`).
    pub fn uses_right(&self) -> bool {
        self.uses_side(AdvanceSide::Right)
    }

    /// Given a required strict combined score floor `f(s_L, s_R) > min_combined` and an upper
    /// bound `0 <= s_opp <= max_opp` on the opposite side, returns a strict lower-bound threshold
    /// `t >= 0.0` on `side`'s score (`s_side > t`, suitable for `PruningScorer`) if one holds by
    /// monotonicity.
    fn threshold_for(&self, side: AdvanceSide, min_combined: f32, max_opp: f64) -> Option<f32> {
        let min_combined_f64 = min_combined as f64;
        if !self.uses_side(side) || !max_opp.is_finite() || max_opp < 0.0 {
            return None;
        }
        if self.eval(InputScorePair::for_side(side, 0.0, max_opp)) > min_combined_f64 {
            return None;
        }
        if let Some(raw) = self.raw_min_score_for(side, min_combined_f64, max_opp)
            && raw > 0.0
            && raw.is_finite()
        {
            let candidate = ((raw as f32) * (1.0 - 1e-5) - 1e-5).max(0.0);
            if self.eval(InputScorePair::for_side(side, candidate as f64, max_opp))
                <= min_combined_f64
            {
                return Some(candidate);
            }
        }
        Some(0.0)
    }

    fn raw_min_score_for(&self, side: AdvanceSide, min_combined: f64, max_opp: f64) -> Option<f64> {
        let zero_pair = InputScorePair::for_side(side, 0.0, max_opp);
        match (self, side) {
            (ScoreCombiner::LeftScore, AdvanceSide::Left)
            | (ScoreCombiner::RightScore, AdvanceSide::Right) => Some(min_combined),
            (
                ScoreCombiner::LeftScore | ScoreCombiner::RightScore | ScoreCombiner::Constant(_),
                _,
            ) => None,
            (ScoreCombiner::Add(a, b), _) => {
                if !b.uses_side(side) {
                    a.raw_min_score_for(side, min_combined - b.eval(zero_pair), max_opp)
                } else if !a.uses_side(side) {
                    b.raw_min_score_for(side, min_combined - a.eval(zero_pair), max_opp)
                } else {
                    None
                }
            }
            (ScoreCombiner::Mul(a, b), _) => {
                if !b.uses_side(side) {
                    let b_max = b.eval(zero_pair);
                    (b_max > 0.0)
                        .then(|| a.raw_min_score_for(side, min_combined / b_max, max_opp))?
                } else if !a.uses_side(side) {
                    let a_max = a.eval(zero_pair);
                    (a_max > 0.0)
                        .then(|| b.raw_min_score_for(side, min_combined / a_max, max_opp))?
                } else {
                    None
                }
            }
        }
    }
}

/// Input plans for the left and right sides of [`RankJoinExec`].
#[derive(Debug, Clone)]
pub struct RankJoinInputs {
    pub left: Arc<dyn ExecutionPlan>,
    pub right: Arc<dyn ExecutionPlan>,
}

/// Sort expressions for the left input, right input, and combined join output.
#[derive(Debug, Clone)]
pub struct RankJoinSortExprs {
    /// Score ordering required from the left child.
    pub left: PhysicalSortExpr,
    /// Score ordering required from the right child.
    pub right: PhysicalSortExpr,
    /// Output ordering produced by `RankJoinExec` (whose first element is the
    /// combined score sort expression, followed by any tie-breaker expressions).
    pub output: LexOrdering,
}

/// Symmetric hash rank join physical execution plan node (`JoinType::Inner`).
#[derive(Debug, Clone)]
pub struct RankJoinExec {
    pub inputs: RankJoinInputs,
    pub on: JoinOn,
    pub filter: Option<JoinFilter>,
    pub projection: Option<Vec<usize>>,
    pub mode: PartitionMode,
    pub null_equality: NullEquality,
    pub sort_exprs: RankJoinSortExprs,
    pub combiner: ScoreCombiner,
    left_lookup_filter: Arc<dyn PhysicalExpr>,
    right_lookup_filter: Arc<dyn PhysicalExpr>,
    parent_lookup_filters: Vec<(JoinSide, Arc<dyn PhysicalExpr>)>,
    join_schema: SchemaRef,
    cache: Arc<PlanProperties>,
    metrics: ExecutionPlanMetricsSet,
}

impl RankJoinExec {
    /// Creates a new [`RankJoinExec`].
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        inputs: RankJoinInputs,
        on: JoinOn,
        filter: Option<JoinFilter>,
        projection: Option<Vec<usize>>,
        mode: PartitionMode,
        null_equality: NullEquality,
        sort_exprs: RankJoinSortExprs,
        combiner: ScoreCombiner,
    ) -> Result<Self> {
        let left_keys: Vec<Arc<dyn PhysicalExpr>> = on.iter().map(|(l, _)| Arc::clone(l)).collect();
        let right_keys: Vec<Arc<dyn PhysicalExpr>> =
            on.iter().map(|(_, r)| Arc::clone(r)).collect();
        let left_lookup_filter =
            Arc::new(DynamicFilterPhysicalExpr::new(left_keys, lit(true))) as Arc<dyn PhysicalExpr>;
        let right_lookup_filter = Arc::new(DynamicFilterPhysicalExpr::new(right_keys, lit(true)))
            as Arc<dyn PhysicalExpr>;
        Self::try_new_with_lookup_filters(
            inputs,
            on,
            filter,
            projection,
            mode,
            null_equality,
            sort_exprs,
            combiner,
            left_lookup_filter,
            right_lookup_filter,
            Vec::new(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn try_new_with_lookup_filters(
        inputs: RankJoinInputs,
        on: JoinOn,
        filter: Option<JoinFilter>,
        projection: Option<Vec<usize>>,
        mode: PartitionMode,
        null_equality: NullEquality,
        sort_exprs: RankJoinSortExprs,
        combiner: ScoreCombiner,
        left_lookup_filter: Arc<dyn PhysicalExpr>,
        right_lookup_filter: Arc<dyn PhysicalExpr>,
        parent_lookup_filters: Vec<(JoinSide, Arc<dyn PhysicalExpr>)>,
    ) -> Result<Self> {
        let left_schema = inputs.left.schema();
        let right_schema = inputs.right.schema();
        check_join_is_valid(&left_schema, &right_schema, &on)?;

        let join_schema =
            Arc::new(build_join_schema(&left_schema, &right_schema, &JoinType::Inner).0);
        let cache = Arc::new(Self::compute_properties(
            &inputs,
            &join_schema,
            &on,
            mode,
            projection.as_deref(),
            &sort_exprs.output,
        )?);

        Ok(Self {
            inputs,
            on,
            filter,
            projection,
            mode,
            null_equality,
            sort_exprs,
            combiner,
            left_lookup_filter,
            right_lookup_filter,
            parent_lookup_filters,
            join_schema,
            cache,
            metrics: ExecutionPlanMetricsSet::new(),
        })
    }

    pub fn left(&self) -> &Arc<dyn ExecutionPlan> {
        &self.inputs.left
    }

    pub fn right(&self) -> &Arc<dyn ExecutionPlan> {
        &self.inputs.right
    }

    pub fn on(&self) -> JoinOnRef<'_> {
        &self.on
    }

    pub fn projection(&self) -> Option<&[usize]> {
        self.projection.as_deref()
    }

    /// Attempts to remap a filter expression from `self.schema()` coordinates into the schema
    /// of `side` (`JoinSide::Left` or `JoinSide::Right`). Returns `None` if the expression
    /// references columns outside `side`.
    fn remap_filter_to_child_side(
        &self,
        expr: &Arc<dyn PhysicalExpr>,
        side: JoinSide,
    ) -> Result<Option<Arc<dyn PhysicalExpr>>> {
        let left_schema = self.inputs.left.schema();
        let right_schema = self.inputs.right.schema();
        let left_len = left_schema.fields().len();
        let total_len = self.join_schema.fields().len();

        let mut all_on_side = true;
        let mut saw_col = false;
        let remapped = Arc::clone(expr)
            .transform_down(|node| {
                if let Some(col) = node.downcast_ref::<Column>() {
                    saw_col = true;
                    let out_idx = col.index();
                    let join_idx = match &self.projection {
                        Some(proj) => proj.get(out_idx).copied(),
                        None => Some(out_idx),
                    };
                    let replacement: Option<Arc<dyn PhysicalExpr>> = match (side, join_idx) {
                        (JoinSide::Left, Some(idx)) if idx < left_len => {
                            let field = left_schema.field(idx);
                            Some(Arc::new(Column::new(field.name(), idx)))
                        }
                        (JoinSide::Right, Some(idx)) if idx >= left_len && idx < total_len => {
                            let child_idx = idx - left_len;
                            let field = right_schema.field(child_idx);
                            Some(Arc::new(Column::new(field.name(), child_idx)))
                        }
                        _ => None,
                    };
                    let Some(new_col) = replacement else {
                        all_on_side = false;
                        return Ok(Transformed::new(node, false, TreeNodeRecursion::Stop));
                    };
                    return Ok(Transformed::new(new_col, true, TreeNodeRecursion::Jump));
                }
                Ok(Transformed::no(node))
            })?
            .data;
        Ok((all_on_side && saw_col).then_some(remapped))
    }

    fn compute_properties(
        inputs: &RankJoinInputs,
        join_schema: &SchemaRef,
        on: JoinOnRef,
        mode: PartitionMode,
        projection: Option<&[usize]>,
        output_ordering: &LexOrdering,
    ) -> Result<PlanProperties> {
        let mut eq_properties = join_equivalence_properties(
            inputs.left.equivalence_properties().clone(),
            inputs.right.equivalence_properties().clone(),
            &JoinType::Inner,
            Arc::clone(join_schema),
            &[false, false],
            None,
            on,
        )?;

        let mut output_partitioning = match mode {
            PartitionMode::Partitioned => adjust_right_output_partitioning(
                inputs.right.output_partitioning(),
                inputs.left.schema().fields().len(),
            )?,
            PartitionMode::CollectLeft | PartitionMode::Auto => {
                Partitioning::UnknownPartitioning(1)
            }
        };

        if let Some(projection) = projection {
            let projection_mapping = ProjectionMapping::from_indices(projection, join_schema)?;
            let out_schema = project_schema(join_schema, Some(&projection))?;
            output_partitioning = output_partitioning.project(&projection_mapping, &eq_properties);
            eq_properties = eq_properties.project(&projection_mapping, out_schema);
        }

        eq_properties.add_ordering(output_ordering.clone());

        Ok(PlanProperties::new(
            eq_properties,
            output_partitioning,
            EmissionType::Incremental,
            Boundedness::Bounded,
        ))
    }

    pub(crate) fn encode_for_dispatch(
        &self,
        proto_converter: &dyn datafusion_proto::physical_plan::PhysicalProtoConverterExtension,
    ) -> Result<Vec<u8>> {
        let codec = datafusion_proto::physical_plan::DefaultPhysicalExtensionCodec {};
        let proto_conv = datafusion_proto::physical_plan::DefaultPhysicalProtoConverter {};

        let mut on_bytes = Vec::with_capacity(self.on.len());
        for (l, r) in &self.on {
            let l_node = proto_converter.physical_expr_to_proto(l, &codec)?;
            let r_node = proto_converter.physical_expr_to_proto(r, &codec)?;
            on_bytes.push((
                prost::Message::encode_to_vec(&l_node),
                prost::Message::encode_to_vec(&r_node),
            ));
        }

        let filter_bytes = if let Some(filter) = &self.filter {
            let expr_node = proto_converter.physical_expr_to_proto(filter.expression(), &codec)?;
            let indices: Vec<(bool, usize)> = filter
                .column_indices()
                .iter()
                .map(|ci| (matches!(ci.side, JoinSide::Left), ci.index))
                .collect();
            let schema_proto: datafusion_proto::protobuf::Schema =
                filter.schema().as_ref().try_into().map_err(|e| {
                    DataFusionError::Internal(format!(
                        "RankJoinExec dispatch: filter schema encode: {e}"
                    ))
                })?;
            Some((
                prost::Message::encode_to_vec(&expr_node),
                indices,
                prost::Message::encode_to_vec(&schema_proto),
            ))
        } else {
            None
        };

        let mode_tag: u8 = match self.mode {
            PartitionMode::Partitioned => 0,
            PartitionMode::CollectLeft => 1,
            PartitionMode::Auto => 2,
        };
        let nulls_equal = self.null_equality == NullEquality::NullEqualsNull;

        let left_sort_proto =
            datafusion_proto::physical_plan::to_proto::serialize_physical_sort_expr(
                self.sort_exprs.left.clone(),
                &codec,
                &proto_conv,
            )?;
        let right_sort_proto =
            datafusion_proto::physical_plan::to_proto::serialize_physical_sort_expr(
                self.sort_exprs.right.clone(),
                &codec,
                &proto_conv,
            )?;
        let output_sort_proto =
            datafusion_proto::physical_plan::to_proto::serialize_physical_sort_exprs(
                self.sort_exprs.output.iter().cloned(),
                &codec,
                &proto_conv,
            )?;

        let left_sort_bytes = prost::Message::encode_to_vec(&left_sort_proto);
        let right_sort_bytes = prost::Message::encode_to_vec(&right_sort_proto);
        let output_sort_bytes: Vec<Vec<u8>> = output_sort_proto
            .iter()
            .map(prost::Message::encode_to_vec)
            .collect();

        let left_lookup_node =
            proto_converter.physical_expr_to_proto(&self.left_lookup_filter, &codec)?;
        let right_lookup_node =
            proto_converter.physical_expr_to_proto(&self.right_lookup_filter, &codec)?;
        let left_lookup_bytes = prost::Message::encode_to_vec(&left_lookup_node);
        let right_lookup_bytes = prost::Message::encode_to_vec(&right_lookup_node);

        let mut parent_lookup_bytes = Vec::with_capacity(self.parent_lookup_filters.len());
        for (side, df) in &self.parent_lookup_filters {
            let node = proto_converter.physical_expr_to_proto(df, &codec)?;
            parent_lookup_bytes.push((
                matches!(side, JoinSide::Left),
                prost::Message::encode_to_vec(&node),
            ));
        }

        let payload = (
            on_bytes,
            filter_bytes,
            self.projection.clone(),
            mode_tag,
            nulls_equal,
            left_sort_bytes,
            right_sort_bytes,
            output_sort_bytes,
            self.combiner.clone(),
            left_lookup_bytes,
            right_lookup_bytes,
            parent_lookup_bytes,
        );
        serde_json::to_vec(&payload).map_err(|e| {
            DataFusionError::Internal(format!("RankJoinExec dispatch: serialize: {e}"))
        })
    }

    pub(crate) fn decode_for_dispatch(
        buf: &[u8],
        raw_inputs: &[Arc<dyn ExecutionPlan>],
        ctx: &TaskContext,
        proto_converter: &dyn datafusion_proto::physical_plan::PhysicalProtoConverterExtension,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let [left, right] = raw_inputs else {
            return Err(DataFusionError::Internal(format!(
                "RankJoinExec dispatch: expected 2 inputs, got {}",
                raw_inputs.len()
            )));
        };
        let inputs = RankJoinInputs {
            left: Arc::clone(left),
            right: Arc::clone(right),
        };

        type FilterPayload = (Vec<u8>, Vec<(bool, usize)>, Vec<u8>);
        type DispatchPayload = (
            Vec<(Vec<u8>, Vec<u8>)>,
            Option<FilterPayload>,
            Option<Vec<usize>>,
            u8,
            bool,
            Vec<u8>,
            Vec<u8>,
            Vec<Vec<u8>>,
            ScoreCombiner,
            Vec<u8>,
            Vec<u8>,
            Vec<(bool, Vec<u8>)>,
        );

        let (
            on_bytes,
            filter_bytes,
            projection,
            mode_tag,
            nulls_equal,
            left_sort_bytes,
            right_sort_bytes,
            output_sort_bytes,
            combiner,
            left_lookup_bytes,
            right_lookup_bytes,
            parent_lookup_bytes,
        ): DispatchPayload = serde_json::from_slice(buf).map_err(|e| {
            DataFusionError::Internal(format!("RankJoinExec dispatch: deserialize: {e}"))
        })?;

        let codec = datafusion_proto::physical_plan::DefaultPhysicalExtensionCodec {};
        let proto_conv = datafusion_proto::physical_plan::DefaultPhysicalProtoConverter {};
        let decode_ctx =
            datafusion_proto::physical_plan::PhysicalPlanDecodeContext::new(ctx, &codec);

        let left_schema = inputs.left.schema();
        let right_schema = inputs.right.schema();

        let mut on = Vec::with_capacity(on_bytes.len());
        for (l_buf, r_buf) in on_bytes {
            let l_node = <datafusion_proto::protobuf::PhysicalExprNode as prost::Message>::decode(
                l_buf.as_slice(),
            )
            .map_err(|e| {
                DataFusionError::Internal(format!("RankJoinExec dispatch: left key decode: {e}"))
            })?;
            let r_node = <datafusion_proto::protobuf::PhysicalExprNode as prost::Message>::decode(
                r_buf.as_slice(),
            )
            .map_err(|e| {
                DataFusionError::Internal(format!("RankJoinExec dispatch: right key decode: {e}"))
            })?;
            let l_expr = proto_converter.proto_to_physical_expr(
                &l_node,
                left_schema.as_ref(),
                &decode_ctx,
            )?;
            let r_expr = proto_converter.proto_to_physical_expr(
                &r_node,
                right_schema.as_ref(),
                &decode_ctx,
            )?;
            on.push((l_expr, r_expr));
        }

        let filter = if let Some((expr_buf, indices, schema_buf)) = filter_bytes {
            let schema_proto = <datafusion_proto::protobuf::Schema as prost::Message>::decode(
                schema_buf.as_slice(),
            )
            .map_err(|e| {
                DataFusionError::Internal(format!(
                    "RankJoinExec dispatch: filter schema decode: {e}"
                ))
            })?;
            let filter_schema: SchemaRef = Arc::new((&schema_proto).try_into().map_err(|e| {
                DataFusionError::Internal(format!(
                    "RankJoinExec dispatch: filter schema parse: {e}"
                ))
            })?);
            let expr_node =
                <datafusion_proto::protobuf::PhysicalExprNode as prost::Message>::decode(
                    expr_buf.as_slice(),
                )
                .map_err(|e| {
                    DataFusionError::Internal(format!("RankJoinExec dispatch: filter decode: {e}"))
                })?;
            let expr = proto_converter.proto_to_physical_expr(
                &expr_node,
                filter_schema.as_ref(),
                &decode_ctx,
            )?;
            let col_indices: Vec<ColumnIndex> = indices
                .into_iter()
                .map(|(is_left, index)| ColumnIndex {
                    index,
                    side: if is_left {
                        JoinSide::Left
                    } else {
                        JoinSide::Right
                    },
                })
                .collect();
            Some(JoinFilter::new(expr, col_indices, filter_schema))
        } else {
            None
        };

        let mode = match mode_tag {
            0 => PartitionMode::Partitioned,
            1 => PartitionMode::CollectLeft,
            _ => PartitionMode::Auto,
        };
        let null_equality = if nulls_equal {
            NullEquality::NullEqualsNull
        } else {
            NullEquality::NullEqualsNothing
        };

        let left_sort_node =
            <datafusion_proto::protobuf::PhysicalSortExprNode as prost::Message>::decode(
                left_sort_bytes.as_slice(),
            )
            .map_err(|e| {
                DataFusionError::Internal(format!("RankJoinExec dispatch: left sort decode: {e}"))
            })?;
        let right_sort_node =
            <datafusion_proto::protobuf::PhysicalSortExprNode as prost::Message>::decode(
                right_sort_bytes.as_slice(),
            )
            .map_err(|e| {
                DataFusionError::Internal(format!("RankJoinExec dispatch: right sort decode: {e}"))
            })?;
        let output_sort_nodes = output_sort_bytes
            .iter()
            .map(|b| {
                <datafusion_proto::protobuf::PhysicalSortExprNode as prost::Message>::decode(
                    b.as_slice(),
                )
            })
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| {
                DataFusionError::Internal(format!("RankJoinExec dispatch: output sort decode: {e}"))
            })?;

        let left_sort = datafusion_proto::physical_plan::from_proto::parse_physical_sort_expr(
            &left_sort_node,
            &decode_ctx,
            left_schema.as_ref(),
            &proto_conv,
        )?;
        let right_sort = datafusion_proto::physical_plan::from_proto::parse_physical_sort_expr(
            &right_sort_node,
            &decode_ctx,
            right_schema.as_ref(),
            &proto_conv,
        )?;

        let join_schema =
            Arc::new(build_join_schema(&left_schema, &right_schema, &JoinType::Inner).0);
        let output_schema = project_schema(&join_schema, projection.as_ref())?;
        let output_exprs = datafusion_proto::physical_plan::from_proto::parse_physical_sort_exprs(
            &output_sort_nodes,
            &decode_ctx,
            output_schema.as_ref(),
            &proto_conv,
        )?;
        let output_ordering = LexOrdering::new(output_exprs).ok_or_else(|| {
            DataFusionError::Internal("RankJoinExec dispatch: empty output sort order".into())
        })?;

        let left_lookup_node =
            <datafusion_proto::protobuf::PhysicalExprNode as prost::Message>::decode(
                left_lookup_bytes.as_slice(),
            )
            .map_err(|e| {
                DataFusionError::Internal(format!(
                    "RankJoinExec dispatch: left lookup filter decode: {e}"
                ))
            })?;
        let right_lookup_node =
            <datafusion_proto::protobuf::PhysicalExprNode as prost::Message>::decode(
                right_lookup_bytes.as_slice(),
            )
            .map_err(|e| {
                DataFusionError::Internal(format!(
                    "RankJoinExec dispatch: right lookup filter decode: {e}"
                ))
            })?;
        let left_lookup_filter = proto_converter.proto_to_physical_expr(
            &left_lookup_node,
            left_schema.as_ref(),
            &decode_ctx,
        )?;
        let right_lookup_filter = proto_converter.proto_to_physical_expr(
            &right_lookup_node,
            right_schema.as_ref(),
            &decode_ctx,
        )?;

        let mut parent_lookup_filters = Vec::with_capacity(parent_lookup_bytes.len());
        for (is_left, buf) in parent_lookup_bytes {
            let node = <datafusion_proto::protobuf::PhysicalExprNode as prost::Message>::decode(
                buf.as_slice(),
            )
            .map_err(|e| {
                DataFusionError::Internal(format!(
                    "RankJoinExec dispatch: parent lookup filter decode: {e}"
                ))
            })?;
            let (side, target_schema) = if is_left {
                (JoinSide::Left, left_schema.as_ref())
            } else {
                (JoinSide::Right, right_schema.as_ref())
            };
            let expr = proto_converter.proto_to_physical_expr(&node, target_schema, &decode_ctx)?;
            parent_lookup_filters.push((side, expr));
        }

        Ok(Arc::new(Self::try_new_with_lookup_filters(
            inputs,
            on,
            filter,
            projection,
            mode,
            null_equality,
            RankJoinSortExprs {
                left: left_sort,
                right: right_sort,
                output: output_ordering,
            },
            combiner,
            left_lookup_filter,
            right_lookup_filter,
            parent_lookup_filters,
        )?))
    }
}

impl DisplayAs for RankJoinExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut fmt::Formatter) -> fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                let display_filter = self.filter.as_ref().map_or_else(
                    || "".to_string(),
                    |filter| format!(", filter={}", filter.expression()),
                );
                let display_projections = if let Some(proj) = &self.projection {
                    format!(
                        ", projection=[{}]",
                        proj.iter()
                            .map(|index| format!(
                                "{}@{}",
                                self.join_schema.fields().get(*index).unwrap().name(),
                                index
                            ))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                } else {
                    "".to_string()
                };
                let on_str = self
                    .on
                    .iter()
                    .map(|(c1, c2)| format!("({c1}, {c2})"))
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(
                    f,
                    "RankJoinExec: mode={:?}, on=[{}], order=[{}]{}{}",
                    self.mode, on_str, self.sort_exprs.output, display_filter, display_projections
                )
            }
            DisplayFormatType::TreeRender => {
                writeln!(f, "mode={:?}", self.mode)?;
                Ok(())
            }
        }
    }
}

impl ExecutionPlan for RankJoinExec {
    fn name(&self) -> &'static str {
        "RankJoinExec"
    }

    fn properties(&self) -> &Arc<PlanProperties> {
        &self.cache
    }

    fn required_input_distribution(&self) -> Vec<Distribution> {
        self.input_distribution_requirements().into_per_child()
    }

    fn input_distribution_requirements(&self) -> InputDistributionRequirements {
        match self.mode {
            PartitionMode::Partitioned => {
                let (left_expr, right_expr) = self
                    .on
                    .iter()
                    .map(|(l, r)| (Arc::clone(l), Arc::clone(r)))
                    .unzip();
                InputDistributionRequirements::co_partitioned(vec![
                    Distribution::KeyPartitioned(left_expr),
                    Distribution::KeyPartitioned(right_expr),
                ])
            }
            PartitionMode::CollectLeft | PartitionMode::Auto => {
                InputDistributionRequirements::new(vec![
                    Distribution::SinglePartition,
                    Distribution::SinglePartition,
                ])
            }
        }
    }

    fn benefits_from_input_partitioning(&self) -> Vec<bool> {
        vec![false, false]
    }

    fn required_input_ordering(&self) -> Vec<Option<OrderingRequirements>> {
        let left_req = if self.combiner.uses_left() {
            LexOrdering::new(vec![self.sort_exprs.left.clone()]).map(OrderingRequirements::from)
        } else {
            None
        };
        let right_req = if self.combiner.uses_right() {
            LexOrdering::new(vec![self.sort_exprs.right.clone()]).map(OrderingRequirements::from)
        } else {
            None
        };
        vec![left_req, right_req]
    }

    fn maintains_input_order(&self) -> Vec<bool> {
        vec![false, false]
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.inputs.left, &self.inputs.right]
    }

    fn apply_expressions(
        &self,
        f: &mut dyn FnMut(&Arc<dyn PhysicalExpr>) -> Result<TreeNodeRecursion>,
    ) -> Result<TreeNodeRecursion> {
        for (l, r) in &self.on {
            if f(l)? == TreeNodeRecursion::Stop {
                return Ok(TreeNodeRecursion::Stop);
            }
            if f(r)? == TreeNodeRecursion::Stop {
                return Ok(TreeNodeRecursion::Stop);
            }
        }
        if let Some(filter) = &self.filter
            && f(filter.expression())? == TreeNodeRecursion::Stop
        {
            return Ok(TreeNodeRecursion::Stop);
        }
        if f(&self.sort_exprs.left.expr)? == TreeNodeRecursion::Stop {
            return Ok(TreeNodeRecursion::Stop);
        }
        if f(&self.sort_exprs.right.expr)? == TreeNodeRecursion::Stop {
            return Ok(TreeNodeRecursion::Stop);
        }
        for sort_expr in self.sort_exprs.output.iter() {
            if f(&sort_expr.expr)? == TreeNodeRecursion::Stop {
                return Ok(TreeNodeRecursion::Stop);
            }
        }
        if f(&self.left_lookup_filter)? == TreeNodeRecursion::Stop {
            return Ok(TreeNodeRecursion::Stop);
        }
        if f(&self.right_lookup_filter)? == TreeNodeRecursion::Stop {
            return Ok(TreeNodeRecursion::Stop);
        }
        for (_, df) in &self.parent_lookup_filters {
            if f(df)? == TreeNodeRecursion::Stop {
                return Ok(TreeNodeRecursion::Stop);
            }
        }
        Ok(TreeNodeRecursion::Continue)
    }

    fn with_new_children(
        self: Arc<Self>,
        mut children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        if children.len() != 2 {
            return internal_err!("RankJoinExec wrong number of children: {}", children.len());
        }
        let right = children.pop().unwrap();
        let left = children.pop().unwrap();
        Ok(Arc::new(Self::try_new_with_lookup_filters(
            RankJoinInputs { left, right },
            self.on.clone(),
            self.filter.clone(),
            self.projection.clone(),
            self.mode,
            self.null_equality,
            self.sort_exprs.clone(),
            self.combiner.clone(),
            Arc::clone(&self.left_lookup_filter),
            Arc::clone(&self.right_lookup_filter),
            self.parent_lookup_filters.clone(),
        )?))
    }

    fn gather_filters_for_pushdown(
        &self,
        phase: FilterPushdownPhase,
        parent_filters: Vec<Arc<dyn PhysicalExpr>>,
        _config: &ConfigOptions,
    ) -> Result<FilterDescription> {
        if !matches!(phase, FilterPushdownPhase::Post) {
            return Ok(FilterDescription::all_unsupported(
                &parent_filters,
                &self.children(),
            ));
        }

        let unsupported_sentinel: Arc<dyn PhysicalExpr> =
            Arc::new(Column::new("__rank_join_unsupported__", usize::MAX));
        let mut left_parent_filters = Vec::with_capacity(parent_filters.len());
        let mut right_parent_filters = Vec::with_capacity(parent_filters.len());

        for pf in &parent_filters {
            if let Some(remapped) = self.remap_filter_to_child_side(pf, JoinSide::Left)? {
                left_parent_filters.push(remapped);
            } else {
                left_parent_filters.push(Arc::clone(&unsupported_sentinel));
            }
            if let Some(remapped) = self.remap_filter_to_child_side(pf, JoinSide::Right)? {
                right_parent_filters.push(remapped);
            } else {
                right_parent_filters.push(Arc::clone(&unsupported_sentinel));
            }
        }

        let mut left_desc =
            crate::scan::filter_pushdown::schema_preserving_child_filter_description(
                &left_parent_filters,
                &self.inputs.left.schema(),
                None,
            )?;
        if self.combiner.uses_right() || !parent_filters.is_empty() {
            left_desc = left_desc.with_self_filter(Arc::clone(&self.left_lookup_filter));
        }
        let mut right_desc =
            crate::scan::filter_pushdown::schema_preserving_child_filter_description(
                &right_parent_filters,
                &self.inputs.right.schema(),
                None,
            )?;
        if self.combiner.uses_left() || !parent_filters.is_empty() {
            right_desc = right_desc.with_self_filter(Arc::clone(&self.right_lookup_filter));
        }

        Ok(FilterDescription::new()
            .with_child(left_desc)
            .with_child(right_desc))
    }

    fn handle_child_pushdown_result(
        &self,
        phase: FilterPushdownPhase,
        child_pushdown_result: ChildPushdownResult,
        _config: &ConfigOptions,
    ) -> Result<FilterPushdownPropagation<Arc<dyn ExecutionPlan>>> {
        if !matches!(phase, FilterPushdownPhase::Post) {
            return Ok(FilterPushdownPropagation::if_any(child_pushdown_result));
        }

        let mut parent_lookup_filters = self.parent_lookup_filters.clone();
        let mut changed = false;

        for filter_result in &child_pushdown_result.parent_filters {
            if !filter_result.filter.is::<DynamicFilterPhysicalExpr>() {
                continue;
            }
            let accepted_side =
                if matches!(filter_result.child_results.first(), Some(PushedDown::Yes)) {
                    Some(JoinSide::Left)
                } else if matches!(filter_result.child_results.get(1), Some(PushedDown::Yes)) {
                    Some(JoinSide::Right)
                } else {
                    None
                };
            let Some(side) = accepted_side else {
                continue;
            };
            let Some(remapped) = self.remap_filter_to_child_side(&filter_result.filter, side)?
            else {
                continue;
            };
            let id = remapped.expression_id();
            match parent_lookup_filters
                .iter_mut()
                .find(|(_, df)| df.expression_id() == id)
            {
                Some((existing_side, slot))
                    if *existing_side != side || !Arc::ptr_eq(&remapped, slot) =>
                {
                    *existing_side = side;
                    *slot = remapped;
                    changed = true;
                }
                Some(_) => {}
                None => {
                    parent_lookup_filters.push((side, remapped));
                    changed = true;
                }
            }
        }

        let propagation = FilterPushdownPropagation::if_any(child_pushdown_result);
        if changed {
            let updated = Self::try_new_with_lookup_filters(
                self.inputs.clone(),
                self.on.clone(),
                self.filter.clone(),
                self.projection.clone(),
                self.mode,
                self.null_equality,
                self.sort_exprs.clone(),
                self.combiner.clone(),
                Arc::clone(&self.left_lookup_filter),
                Arc::clone(&self.right_lookup_filter),
                parent_lookup_filters,
            )?;
            Ok(propagation.with_updated_node(Arc::new(updated) as Arc<dyn ExecutionPlan>))
        } else {
            Ok(propagation)
        }
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        if matches!(self.mode, PartitionMode::CollectLeft | PartitionMode::Auto) && partition != 0 {
            return internal_err!(
                "RankJoinExec in {:?} mode only supports single-partition execution (got partition {})",
                self.mode,
                partition
            );
        }
        let left_stream = self.inputs.left.execute(partition, Arc::clone(&context))?;
        let right_stream = self.inputs.right.execute(partition, Arc::clone(&context))?;

        let left_schema = self.inputs.left.schema();
        let sort_fields: Vec<SortField> = self
            .on
            .iter()
            .map(|(l, _)| Ok(SortField::new(l.data_type(&left_schema)?)))
            .collect::<Result<_>>()?;
        let key_converter = Arc::new(RowConverter::new(sort_fields)?);

        let (left_keys, right_keys): (Vec<_>, Vec<_>) = self
            .on
            .iter()
            .map(|(l, r)| (Arc::clone(l), Arc::clone(r)))
            .unzip();

        let left_ra_capable =
            plan_contains_filter_id(&self.inputs.left, self.left_lookup_filter.expression_id());
        let right_ra_capable =
            plan_contains_filter_id(&self.inputs.right, self.right_lookup_filter.expression_id());

        let batch_size = context.session_config().batch_size().max(1);
        let baseline_metrics = BaselineMetrics::new(&self.metrics, partition);
        let elapsed_compute = baseline_metrics.elapsed_compute().clone();

        let left_cursor = RankedInputCursor::new(
            left_stream,
            self.sort_exprs.left.clone(),
            left_keys,
            Arc::clone(&key_converter),
            self.null_equality,
            Arc::clone(&self.left_lookup_filter),
            left_ra_capable,
            !self.combiner.uses_left(),
            elapsed_compute.clone(),
        );
        let right_cursor = RankedInputCursor::new(
            right_stream,
            self.sort_exprs.right.clone(),
            right_keys,
            key_converter,
            self.null_equality,
            Arc::clone(&self.right_lookup_filter),
            right_ra_capable,
            !self.combiner.uses_right(),
            elapsed_compute.clone(),
        );

        let schema = self.schema();
        let primary_sort_data_type = self.sort_exprs.output[0].expr.data_type(&schema).ok();
        let primary_score_is_f32 = matches!(primary_sort_data_type, Some(DataType::Float32));
        let primary_sort_needs_exact_tie_break = matches!(
            primary_sort_data_type,
            Some(
                DataType::Int64
                    | DataType::UInt64
                    | DataType::Timestamp(_, _)
                    | DataType::Date64
                    | DataType::Time64(_)
            )
        );
        let parent_target_probed =
            vec![ParentTargetProbedKeys::default(); self.parent_lookup_filters.len()];
        let mut state = RankJoinStream {
            schema: Arc::clone(&schema),
            left_num_cols: left_schema.fields().len(),
            right_num_cols: self.inputs.right.schema().fields().len(),
            filter: self.filter.clone(),
            projection: self.projection.clone(),
            output_ordering: self.sort_exprs.output.clone(),
            primary_score_is_f32,
            primary_sort_needs_exact_tie_break,
            combiner: self.combiner.clone(),
            parent_lookup_filters: self.parent_lookup_filters.clone(),
            parent_target_probed,
            left_cursor,
            right_cursor,
            candidate_heap: BinaryHeap::new(),
            emitted_pairs: HashSet::default(),
            next_sequence: 0,
            initialized: false,
            batch_size,
            elapsed_compute,
        };

        let stream_gen = async_stream::try_stream! {
            // Phase 1: Sorted Access (`SA`) + interleaved Random Access (`RA`) threshold loop.
            'sa_loop: loop {
                // Service any cascading RA lookups requested by a parent `RankJoinExec` before
                // advancing SA.
                while let Some((filter_idx, target_side, lookup)) =
                    state.take_parent_lookup_request()
                {
                    let batch = state
                        .execute_parent_lookup(filter_idx, target_side, lookup)
                        .await?;
                    yield batch.record_output(&baseline_metrics);
                }

                // Lazily pull the first SA batch from each non-exhausted input so `top_score`
                // is known before computing upper bounds.
                if !state.initialized {
                    state.initialize_sa_cursors().await?;
                }

                // If either input is completely empty and cannot produce rows via SA or RA,
                // no join match is possible.
                if state.has_empty_exhausted_input() {
                    break 'sa_loop;
                }

                // Rule 1: Advance already-buffered in-memory SA rows (pausing a side once it
                // accumulates `ra_batch_size` unresolved keys). Advancing the buffered window in
                // memory before checking `has_ready_candidates()` lowers `bottom_score` as far as
                // the current buffer allows so all unlocked candidates drain in a single batch
                // rather than trickling out one row at a time.
                {
                    let _timer = baseline_metrics.elapsed_compute().timer();
                    while let Some(side) = state.next_buffered_sa_side() {
                        state.advance_one_row(side);
                    }
                }

                // Drain and emit any candidate batches whose rank beats the stopping threshold.
                while state.has_ready_candidates() {
                    if let Some(batch) = state.drain_ready_batch()? {
                        yield batch.record_output(&baseline_metrics);
                        continue 'sa_loop;
                    }
                }

                // Rule 2: Compare upper bounds (U_RA_L, U_RA_R, U_SA) to either execute a batched
                // RA lookup or pull the next SA batch from a child stream.
                let step = {
                    let _timer = baseline_metrics.elapsed_compute().timer();
                    state.next_io_step()
                };
                match step {
                    NextIoStep::RaLookup {
                        target_side,
                        keys,
                        score_threshold,
                    } => {
                        state
                            .execute_ra_lookup(target_side, &keys, score_threshold)
                            .await?;
                    }
                    NextIoStep::SaPull(pull_side) => {
                        if state.ensure_side_buffered(pull_side).await? {
                            let _timer = baseline_metrics.elapsed_compute().timer();
                            state.advance_one_row(pull_side);
                        }
                    }
                    NextIoStep::Exhausted => break 'sa_loop,
                }
            }

            // Phase 2: Post-SA cascading RA server loop (when under a parent `RankJoinExec`).
            if state.signal_sa_exhausted_to_parent() {
                yield RecordBatch::new_empty(Arc::clone(&state.schema))
                    .record_output(&baseline_metrics);

                while let Some((filter_idx, target_side, lookup)) =
                    state.take_parent_lookup_request()
                {
                    let batch = state
                        .execute_parent_lookup(filter_idx, target_side, lookup)
                        .await?;
                    yield batch.record_output(&baseline_metrics);
                }
            }
        };
        Ok(Box::pin(RecordBatchStreamAdapter::new(schema, stream_gen)))
    }

    fn metrics(&self) -> Option<MetricsSet> {
        Some(self.metrics.clone_inner())
    }
}

fn plan_contains_filter_id(plan: &Arc<dyn ExecutionPlan>, target_id: Option<u64>) -> bool {
    let Some(target_id) = target_id else {
        return false;
    };
    let mut found = false;
    let _ = plan.apply_expressions(&mut |expr| {
        let _ = expr.apply(|node| {
            if node.is::<DynamicFilterPhysicalExpr>() && node.expression_id() == Some(target_id) {
                found = true;
                return Ok(TreeNodeRecursion::Stop);
            }
            Ok(TreeNodeRecursion::Continue)
        });
        if found {
            Ok(TreeNodeRecursion::Stop)
        } else {
            Ok(TreeNodeRecursion::Continue)
        }
    });
    if found {
        return true;
    }
    plan.children()
        .iter()
        .any(|c| plan_contains_filter_id(c, Some(target_id)))
}

fn update_lookup_filter(
    filter: &Arc<dyn PhysicalExpr>,
    new_expr: Arc<dyn PhysicalExpr>,
) -> Result<()> {
    if let Some(dyn_filter) = filter.downcast_ref::<DynamicFilterPhysicalExpr>() {
        dyn_filter.update(new_expr)?;
    }
    Ok(())
}

fn current_lookup_filter(filter: &Arc<dyn PhysicalExpr>) -> Option<Arc<dyn PhysicalExpr>> {
    filter
        .downcast_ref::<DynamicFilterPhysicalExpr>()
        .and_then(|df| df.current().ok())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BatchIndex(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RowIndex(usize);

/// Identifies a buffered row inside a [`RankedInputCursor`]'s retained batches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BufferedRowRef {
    batch: BatchIndex,
    row: RowIndex,
}

/// Entry stored in [`RankedInputCursor::hash_table`] for a previously visited input row.
#[derive(Debug, Clone, Copy)]
struct HashTableEntry {
    row_ref: BufferedRowRef,
    score: f64,
    /// `true` when this row arrived via Sorted Access (`SA`), `false` when fetched via
    /// Random Access (`RA`). In a tagless disjunctive join, `RA`-fetched rows only satisfy
    /// their table's base filter (not its disjunctive search predicate), so `ingest_ra_batch`
    /// only pairs `RA` rows with opposite-side entries that have `from_sa == true`.
    from_sa: bool,
}

/// A joined candidate pair waiting in the max-heap until its score clears threshold `T`.
#[derive(Debug, Clone)]
struct CandidateMatch {
    /// Normalized rank value (`score` for `DESC`, `-score` for `ASC`) so the max-heap
    /// always pops the best candidate first.
    rank_value: f64,
    /// Monotonically increasing insertion counter for deterministic FIFO tie-breaking.
    sequence: u64,
    left_row: BufferedRowRef,
    right_row: BufferedRowRef,
}

impl CandidateMatch {
    fn for_side(
        side: AdvanceSide,
        side_row: BufferedRowRef,
        opp_row: BufferedRowRef,
        rank_value: f64,
        sequence: u64,
    ) -> Self {
        let (left_row, right_row) = match side {
            AdvanceSide::Left => (side_row, opp_row),
            AdvanceSide::Right => (opp_row, side_row),
        };
        Self {
            rank_value,
            sequence,
            left_row,
            right_row,
        }
    }
}

impl PartialEq for CandidateMatch {
    fn eq(&self, other: &Self) -> bool {
        self.rank_value.to_bits() == other.rank_value.to_bits() && self.sequence == other.sequence
    }
}

impl Eq for CandidateMatch {}

impl PartialOrd for CandidateMatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CandidateMatch {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank_value
            .total_cmp(&other.rank_value)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

/// Wraps one child stream of [`RankJoinExec`], retaining pulled batches so matched rows
/// can be assembled by `(BatchIndex, RowIndex)` when their candidate clears threshold `T`,
/// and coordinating both Sorted Access (`SA`) window pulls and Random Access (`RA`) point lookups.
struct RankedInputCursor {
    stream: SendableRecordBatchStream,
    sort_expr: PhysicalSortExpr,
    key_exprs: Vec<Arc<dyn PhysicalExpr>>,
    key_converter: Arc<RowConverter>,
    null_equality: NullEquality,
    lookup_filter: Arc<dyn PhysicalExpr>,
    ra_capable: bool,
    batches: Vec<RecordBatch>,
    /// Per-row `f64` scores parallel to `batches`, indexed by `[batch_idx][row_idx]`.
    batch_scores: Vec<Vec<f64>>,
    /// Hash table indexing all previously buffered rows of this input by join key.
    hash_table: HashMap<OwnedRow, Vec<HashTableEntry>>,
    sa_batch_idx: Option<usize>,
    current_batch_scores: Vec<f64>,
    current_batch_keys: Vec<Option<OwnedRow>>,
    current_row_idx: usize,
    top_score: Option<f64>,
    bottom_score: Option<f64>,
    sa_exhausted: bool,
    /// Total number of `SA` rows advanced from this cursor so far.
    sa_rows_advanced: usize,
    /// Distinct keys from `SA` batches in rank order `(key, best_sa_score)` for tracking the
    /// highest-ranking unresolved key (`s_unres`) for Tier-1 (`score > 0.0`) and
    /// Tier-2 (`score >= 0.0`) `RA` probes on the opposite side.
    sa_keys_in_order: Vec<(OwnedRow, f64)>,
    /// Maps each queued key in `sa_keys_in_order` to its best recorded score so duplicate keys
    /// with worse or equal scores can be skipped in `O(1)` time without scanning `sa_keys_in_order`.
    sa_queued_keys: HashMap<OwnedRow, f64>,
    /// Index of the first key in `sa_keys_in_order` not yet probed at all (Tier 1: `score > 0.0`).
    next_unresolved_idx: usize,
    /// Index of the first key in `sa_keys_in_order` not yet probed for `0.0`-score rows (Tier 2).
    next_zero_unresolved_idx: usize,
    /// Current batch size for `RA` point lookups triggered from this cursor's unresolved `SA` keys.
    /// Starts at [`RA_INITIAL_BATCH_SIZE`] and scales by `topk_retry_scale_factor` after each batch.
    ra_batch_size: usize,
    /// Keys that have been probed with `score_threshold = Some(0.0)` (`score > 0.0`) against this cursor's child stream.
    ra_positive_probed_keys: HashSet<OwnedRow>,
    /// Keys that have been fully probed (`score_threshold = None`, including `0.0`-score rows) against this cursor's child stream.
    ra_probed_keys: HashSet<OwnedRow>,
    elapsed_compute: Time,
}

impl RankedInputCursor {
    #[expect(clippy::too_many_arguments)]
    fn new(
        stream: SendableRecordBatchStream,
        sort_expr: PhysicalSortExpr,
        key_exprs: Vec<Arc<dyn PhysicalExpr>>,
        key_converter: Arc<RowConverter>,
        null_equality: NullEquality,
        lookup_filter: Arc<dyn PhysicalExpr>,
        ra_capable: bool,
        start_sa_exhausted: bool,
        elapsed_compute: Time,
    ) -> Self {
        Self {
            stream,
            sort_expr,
            key_exprs,
            key_converter,
            null_equality,
            lookup_filter,
            ra_capable,
            batches: Vec::new(),
            batch_scores: Vec::new(),
            hash_table: HashMap::default(),
            sa_batch_idx: None,
            current_batch_scores: Vec::new(),
            current_batch_keys: Vec::new(),
            current_row_idx: 0,
            top_score: start_sa_exhausted.then_some(0.0),
            bottom_score: start_sa_exhausted.then_some(0.0),
            sa_exhausted: start_sa_exhausted,
            sa_rows_advanced: 0,
            sa_keys_in_order: Vec::new(),
            sa_queued_keys: HashMap::default(),
            next_unresolved_idx: 0,
            next_zero_unresolved_idx: 0,
            ra_batch_size: RA_INITIAL_BATCH_SIZE,
            ra_positive_probed_keys: HashSet::default(),
            ra_probed_keys: HashSet::default(),
            elapsed_compute,
        }
    }

    fn row_score(&self, row_ref: BufferedRowRef) -> f64 {
        self.batch_scores[row_ref.batch.0][row_ref.row.0]
    }

    fn has_buffered_rows(&self) -> bool {
        self.current_row_idx < self.current_batch_scores.len()
    }

    fn sa_fully_consumed(&self) -> bool {
        self.sa_exhausted && !self.has_buffered_rows()
    }

    /// Number of un-probed `SA` key entries currently queued for Tier-1 `RA` lookup on the opposite side.
    fn unresolved_key_count(&self) -> usize {
        self.sa_keys_in_order
            .len()
            .saturating_sub(self.next_unresolved_idx)
    }

    /// Advances `next_unresolved_idx` past keys already in `opposite_positive_probed` or `opposite_probed`,
    /// and advances `next_zero_unresolved_idx` past keys already in `opposite_probed`.
    fn skip_probed_keys(
        &mut self,
        opposite_positive_probed: &HashSet<OwnedRow>,
        opposite_probed: &HashSet<OwnedRow>,
    ) {
        while self.next_unresolved_idx < self.sa_keys_in_order.len() {
            let (key, _) = &self.sa_keys_in_order[self.next_unresolved_idx];
            if opposite_positive_probed.contains(key) || opposite_probed.contains(key) {
                self.next_unresolved_idx += 1;
            } else {
                break;
            }
        }
        while self.next_zero_unresolved_idx < self.sa_keys_in_order.len() {
            let (key, _) = &self.sa_keys_in_order[self.next_zero_unresolved_idx];
            if opposite_probed.contains(key) {
                self.next_zero_unresolved_idx += 1;
            } else {
                break;
            }
        }
    }

    /// Returns the score of the highest-ranked `SA` row whose join key has not yet been
    /// probed at all via `RA` (neither Tier-1 nor Tier-2) on the opposite side.
    fn top_unresolved_score(
        &self,
        opposite_positive_probed: &HashSet<OwnedRow>,
        opposite_probed: &HashSet<OwnedRow>,
    ) -> Option<f64> {
        self.sa_keys_in_order[self.next_unresolved_idx..]
            .iter()
            .find(|(k, _)| !opposite_positive_probed.contains(k) && !opposite_probed.contains(k))
            .map(|(_, s)| *s)
    }

    /// Returns the score of the highest-ranked `SA` row whose join key has not yet been
    /// probed for `0.0`-score rows (Tier-2) on the opposite side.
    fn top_zero_unresolved_score(&self, opposite_probed: &HashSet<OwnedRow>) -> Option<f64> {
        self.sa_keys_in_order[self.next_zero_unresolved_idx..]
            .iter()
            .find(|(k, _)| !opposite_probed.contains(k))
            .map(|(_, s)| *s)
    }

    fn scale_ra_batch_size(&mut self) {
        let retry_scale = (crate::gucs::topk_retry_scale_factor() as usize).max(2);
        let max_chunk = (crate::gucs::max_topk_chunk_size() as usize).max(RA_INITIAL_BATCH_SIZE);
        self.ra_batch_size = (self.ra_batch_size * retry_scale).min(max_chunk);
    }

    /// Pops up to `self.ra_batch_size` distinct unresolved `SA` keys for a Tier-1 (`score > 0.0`)
    /// `RA` probe, marking them in `opposite_positive_probed`.
    fn pop_positive_unresolved_keys(
        &mut self,
        opposite_positive_probed: &mut HashSet<OwnedRow>,
        opposite_probed: &HashSet<OwnedRow>,
    ) -> Vec<OwnedRow> {
        let max_keys = self.ra_batch_size;
        let mut out = Vec::new();
        while self.next_unresolved_idx < self.sa_keys_in_order.len() && out.len() < max_keys {
            let (key, _) = &self.sa_keys_in_order[self.next_unresolved_idx];
            self.next_unresolved_idx += 1;
            if !opposite_probed.contains(key) && opposite_positive_probed.insert(key.clone()) {
                out.push(key.clone());
            }
        }
        self.skip_probed_keys(opposite_positive_probed, opposite_probed);
        if !out.is_empty() {
            self.scale_ra_batch_size();
        }
        out
    }

    /// Pops up to `self.ra_batch_size` distinct unresolved `SA` keys whose Tier-2 (`score = 0.0`
    /// on the opposite side) rank bound is `>= cutoff`, marking them in both `opposite_probed`
    /// and `opposite_positive_probed`.
    fn pop_all_unresolved_keys(
        &mut self,
        opposite_positive_probed: &mut HashSet<OwnedRow>,
        opposite_probed: &mut HashSet<OwnedRow>,
        mut include_score: impl FnMut(f64) -> bool,
    ) -> Vec<OwnedRow> {
        let max_keys = self.ra_batch_size;
        let mut out = Vec::new();
        while self.next_zero_unresolved_idx < self.sa_keys_in_order.len() && out.len() < max_keys {
            let (key, score) = &self.sa_keys_in_order[self.next_zero_unresolved_idx];
            if !out.is_empty() && !include_score(*score) {
                break;
            }
            self.next_zero_unresolved_idx += 1;
            opposite_positive_probed.insert(key.clone());
            if opposite_probed.insert(key.clone()) {
                out.push(key.clone());
            }
        }
        if self.next_unresolved_idx < self.next_zero_unresolved_idx {
            self.next_unresolved_idx = self.next_zero_unresolved_idx;
        }
        self.skip_probed_keys(opposite_positive_probed, opposite_probed);
        if !out.is_empty() {
            self.scale_ra_batch_size();
        }
        out
    }

    /// Pulls from the underlying stream in Sorted Access (`SA`) mode until either at least one
    /// buffered `SA` row is available or the `SA` stream is exhausted.
    async fn ensure_buffered(&mut self) -> Result<bool> {
        while !self.has_buffered_rows() {
            if self.sa_exhausted {
                return Ok(false);
            }
            match self.stream.next().await {
                None => {
                    self.sa_exhausted = true;
                    self.ra_capable = false;
                    return Ok(false);
                }
                Some(Err(e)) => return Err(e),
                Some(Ok(batch)) => {
                    // Check if the child signaled SA exhaustion (`lit(false)` when staying alive
                    // for RA, or `lit(Boolean(None))` when both SA and RA are exhausted).
                    if let Some(curr) = current_lookup_filter(&self.lookup_filter)
                        && let Some(lit_expr) = curr.downcast_ref::<Literal>()
                    {
                        match lit_expr.value() {
                            ScalarValue::Boolean(Some(false)) => {
                                let _ = update_lookup_filter(&self.lookup_filter, lit(true));
                                self.sa_exhausted = true;
                                if batch.num_rows() == 0 {
                                    return Ok(false);
                                }
                            }
                            ScalarValue::Boolean(None) => {
                                let _ = update_lookup_filter(&self.lookup_filter, lit(true));
                                self.sa_exhausted = true;
                                self.ra_capable = false;
                                if batch.num_rows() == 0 {
                                    return Ok(false);
                                }
                            }
                            _ => {}
                        }
                    }
                    if batch.num_rows() == 0 {
                        continue;
                    }
                    self.ingest_sa_batch(batch)?;
                }
            }
        }
        Ok(true)
    }

    fn evaluate_batch(&self, batch: &RecordBatch) -> Result<(Vec<f64>, Vec<Option<OwnedRow>>)> {
        let num_rows = batch.num_rows();
        let score_arr = self.sort_expr.expr.evaluate(batch)?.into_array(num_rows)?;
        let score_f64_arr = match score_arr.data_type() {
            DataType::Float64 => score_arr,
            DataType::Boolean => {
                let u8_arr = cast(&score_arr, &DataType::UInt8)?;
                cast(&u8_arr, &DataType::Float64)?
            }
            DataType::Timestamp(_, _) | DataType::Date64 | DataType::Time64(_) => {
                let i64_arr = cast(&score_arr, &DataType::Int64)?;
                cast(&i64_arr, &DataType::Float64)?
            }
            DataType::Date32 | DataType::Time32(_) => {
                let i32_arr = cast(&score_arr, &DataType::Int32)?;
                cast(&i32_arr, &DataType::Float64)?
            }
            _ => cast(&score_arr, &DataType::Float64)?,
        };
        let f64_col = score_f64_arr
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| {
                DataFusionError::Internal("RankJoinStream: score cast to Float64 failed".into())
            })?;
        // Map `NULL` to a finite extreme (`f64::MAX` or `f64::MIN`) according to `(descending, nulls_first)`
        // so that `to_rank_value(score)` stays strictly above `f64::NEG_INFINITY` (which is reserved
        // as the "no rows available" sentinel in `compute_detailed_bounds`).
        let null_score = if self.sort_expr.options.descending == self.sort_expr.options.nulls_first
        {
            f64::MAX
        } else {
            f64::MIN
        };
        let scores: Vec<f64> = (0..num_rows)
            .map(|i| {
                if f64_col.is_null(i) {
                    null_score
                } else {
                    let v = f64_col.value(i);
                    if v.is_nan() {
                        1.2e308_f64
                    } else if v == f64::INFINITY {
                        1.1e308_f64
                    } else if v == f64::NEG_INFINITY {
                        -1.1e308_f64
                    } else {
                        v.clamp(-1.0e308_f64, 1.0e308_f64)
                    }
                }
            })
            .collect();

        let key_arrays: Vec<ArrayRef> = self
            .key_exprs
            .iter()
            .map(|expr| expr.evaluate(batch)?.into_array(num_rows))
            .collect::<Result<_>>()?;
        let rows = self.key_converter.convert_columns(&key_arrays)?;

        let skip_nulls = self.null_equality == NullEquality::NullEqualsNothing;
        let mut keys = Vec::with_capacity(num_rows);
        for row_idx in 0..num_rows {
            if skip_nulls && key_arrays.iter().any(|arr| arr.is_null(row_idx)) {
                keys.push(None);
            } else {
                keys.push(Some(rows.row(row_idx).owned()));
            }
        }
        Ok((scores, keys))
    }

    fn ingest_sa_batch(&mut self, batch: RecordBatch) -> Result<()> {
        let _timer = self.elapsed_compute.timer();
        let (scores, keys) = self.evaluate_batch(&batch)?;
        if self.top_score.is_none()
            && let Some(&first_score) = scores.first()
        {
            self.top_score = Some(first_score);
            self.bottom_score = Some(first_score);
        }

        let batch_idx = self.batches.len();
        self.batches.push(batch);
        self.batch_scores.push(scores.clone());
        self.sa_batch_idx = Some(batch_idx);
        self.current_batch_scores = scores;
        self.current_batch_keys = keys;
        self.current_row_idx = 0;
        Ok(())
    }

    /// Records a key in `sa_keys_in_order` while maintaining this cursor's rank order
    /// (`descending` or `ascending`).
    fn record_positive_key(&mut self, key: OwnedRow, score: f64) {
        let descending = self.sort_expr.options.descending;
        let is_better_or_eq = |a: f64, b: f64| if descending { a >= b } else { a <= b };
        let is_strictly_better = |a: f64, b: f64| if descending { a > b } else { a < b };
        match self.sa_queued_keys.entry(key.clone()) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(score);
                let pos = if self
                    .sa_keys_in_order
                    .last()
                    .is_none_or(|(_, last_s)| is_better_or_eq(*last_s, score))
                {
                    self.sa_keys_in_order.len()
                } else {
                    self.sa_keys_in_order
                        .partition_point(|(_, s)| is_better_or_eq(*s, score))
                };
                self.sa_keys_in_order.insert(pos, (key, score));
                self.next_unresolved_idx = self.next_unresolved_idx.min(pos);
                self.next_zero_unresolved_idx = self.next_zero_unresolved_idx.min(pos);
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if is_strictly_better(score, *entry.get()) {
                    entry.insert(score);
                    if let Some(existing_idx) =
                        self.sa_keys_in_order.iter().position(|(k, _)| *k == key)
                    {
                        self.sa_keys_in_order.remove(existing_idx);
                    }
                    let pos = self
                        .sa_keys_in_order
                        .partition_point(|(_, s)| is_better_or_eq(*s, score));
                    self.sa_keys_in_order.insert(pos, (key, score));
                    self.next_unresolved_idx = self.next_unresolved_idx.min(pos);
                    self.next_zero_unresolved_idx = self.next_zero_unresolved_idx.min(pos);
                }
            }
        }
    }

    /// Advances the buffered `SA` cursor by one row, updating `bottom_score` and recording
    /// its key in `sa_keys_in_order` if the opposite side is `ra_capable`.
    fn advance_buffered_row(&mut self, record_for_ra: bool, opposite_probed: &HashSet<OwnedRow>) {
        let row_idx = self.current_row_idx;
        let score = self.current_batch_scores[row_idx];
        let key = self.current_batch_keys[row_idx].take();
        self.current_row_idx += 1;
        self.sa_rows_advanced += 1;
        self.bottom_score = Some(score);
        if record_for_ra
            && let Some(k) = key
            && !opposite_probed.contains(&k)
        {
            self.record_positive_key(k, score);
        }
    }
}

/// Which input side [`RankJoinStream`] chooses to advance or probe next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdvanceSide {
    Left,
    Right,
}

impl AdvanceSide {
    fn opposite(self) -> Self {
        match self {
            AdvanceSide::Left => AdvanceSide::Right,
            AdvanceSide::Right => AdvanceSide::Left,
        }
    }

    fn to_join_side(self) -> JoinSide {
        match self {
            AdvanceSide::Left => JoinSide::Left,
            AdvanceSide::Right => JoinSide::Right,
        }
    }
}

/// Which async child I/O operation [`RankJoinStream`] chooses in Rule 2 when in-memory `SA`
/// buffers are either empty or paused at `ra_batch_size` unresolved keys.
enum NextIoStep {
    /// Execute a batched Random Access (`RA`) lookup on `target_side` for `keys`.
    RaLookup {
        target_side: AdvanceSide,
        keys: Vec<OwnedRow>,
        score_threshold: Option<f32>,
    },
    /// Pull the next Sorted Access (`SA`) batch from `side`.
    SaPull(AdvanceSide),
    /// All `SA` streams and `RA` probes are exhausted.
    Exhausted,
}

/// Detailed upper bounds across Tier-1 (`score > 0.0`) and Tier-2 (`score = 0.0`) `RA` probes
/// and `SA` cursor advancement.
struct DetailedBounds {
    u_ra_l_pos: f64,
    u_ra_l_zero: f64,
    u_ra_r_pos: f64,
    u_ra_r_zero: f64,
    u_sa: f64,
}

/// Tracks cascading parent lookup keys already queried on a child side of a nested [`RankJoinExec`],
/// avoiding redundant leaf queries when a parent issues a Tier-1 (`score_threshold = Some(0.0)`)
/// lookup (which may already widen to `target_thresh = None` on `target_side`) followed by a
/// Tier-2 (`score_threshold = None`) lookup for the same keys.
#[derive(Default, Clone)]
struct ParentTargetProbedKeys {
    /// Keys already queried on this target side with `target_thresh = Some(0.0)`.
    positive_probed: HashSet<Vec<ScalarValue>>,
    /// Keys already queried on this target side with `target_thresh = None` (all rows fetched).
    all_probed: HashSet<Vec<ScalarValue>>,
}

impl ParentTargetProbedKeys {
    /// Filters `keys` to those not yet probed at `target_thresh`, recording the returned
    /// keys as probed at `target_thresh`.
    fn take_unprobed_keys(
        &mut self,
        keys: &[Vec<ScalarValue>],
        target_thresh: Option<f32>,
    ) -> Vec<Vec<ScalarValue>> {
        let mut out = Vec::new();
        for key in keys {
            if self.all_probed.contains(key) {
                continue;
            }
            match target_thresh {
                None => {
                    self.positive_probed.insert(key.clone());
                    if self.all_probed.insert(key.clone()) {
                        out.push(key.clone());
                    }
                }
                Some(t) if t <= 0.0 => {
                    if self.positive_probed.insert(key.clone()) {
                        out.push(key.clone());
                    }
                }
                Some(_) => {
                    if !self.positive_probed.contains(key) {
                        out.push(key.clone());
                    }
                }
            }
        }
        out
    }
}

/// Streaming state machine for [`RankJoinExec`].
struct RankJoinStream {
    schema: SchemaRef,
    left_num_cols: usize,
    right_num_cols: usize,
    filter: Option<JoinFilter>,
    projection: Option<Vec<usize>>,
    output_ordering: LexOrdering,
    primary_score_is_f32: bool,
    primary_sort_needs_exact_tie_break: bool,
    combiner: ScoreCombiner,
    parent_lookup_filters: Vec<(JoinSide, Arc<dyn PhysicalExpr>)>,
    parent_target_probed: Vec<ParentTargetProbedKeys>,
    left_cursor: RankedInputCursor,
    right_cursor: RankedInputCursor,
    candidate_heap: BinaryHeap<CandidateMatch>,
    emitted_pairs: HashSet<(BufferedRowRef, BufferedRowRef)>,
    next_sequence: u64,
    initialized: bool,
    batch_size: usize,
    elapsed_compute: Time,
}

impl RankJoinStream {
    fn cursor(&self, side: AdvanceSide) -> &RankedInputCursor {
        match side {
            AdvanceSide::Left => &self.left_cursor,
            AdvanceSide::Right => &self.right_cursor,
        }
    }

    fn cursor_mut(&mut self, side: AdvanceSide) -> &mut RankedInputCursor {
        match side {
            AdvanceSide::Left => &mut self.left_cursor,
            AdvanceSide::Right => &mut self.right_cursor,
        }
    }

    fn cursors(&self, side: AdvanceSide) -> (&RankedInputCursor, &RankedInputCursor) {
        match side {
            AdvanceSide::Left => (&self.left_cursor, &self.right_cursor),
            AdvanceSide::Right => (&self.right_cursor, &self.left_cursor),
        }
    }

    fn score_to_rank_value(
        combined_score: f64,
        primary_score_is_f32: bool,
        descending: bool,
    ) -> f64 {
        let score = if primary_score_is_f32 && combined_score.abs() <= (f32::MAX as f64) {
            (combined_score as f32) as f64
        } else {
            combined_score
        };
        let rank = if descending { score } else { -score };
        if rank == 0.0 { 0.0 } else { rank }
    }

    /// Normalizes a combined score into a max-heap rank value (`score` for `DESC`, `-score` for `ASC`),
    /// quantizing to `f32` precision when the primary sort expression has `DataType::Float32` so tie
    /// groups in `candidate_heap` match `sort_batch` / `SortExec` tie groups bit-for-bit.
    fn to_rank_value(&self, combined_score: f64) -> f64 {
        Self::score_to_rank_value(
            combined_score,
            self.primary_score_is_f32,
            self.output_ordering[0].options.descending,
        )
    }

    fn compute_side_ra_bounds(&self, side: AdvanceSide, opp_bound: Option<f64>) -> (f64, f64) {
        let (cursor, opp_cursor) = self.cursors(side);
        if opp_cursor.sa_fully_consumed() && !opp_cursor.ra_capable {
            (f64::NEG_INFINITY, f64::NEG_INFINITY)
        } else if !opp_cursor.ra_capable {
            let u = match (cursor.top_score, opp_bound) {
                (Some(s), Some(opp)) => {
                    self.to_rank_value(self.combiner.eval(InputScorePair::for_side(side, s, opp)))
                }
                _ => f64::NEG_INFINITY,
            };
            (u, f64::NEG_INFINITY)
        } else {
            let s_pos = cursor.top_unresolved_score(
                &opp_cursor.ra_positive_probed_keys,
                &opp_cursor.ra_probed_keys,
            );
            let s_zero = cursor.top_zero_unresolved_score(&opp_cursor.ra_probed_keys);
            let u_pos = match (s_pos, opp_bound) {
                (Some(s), Some(opp)) if opp > 0.0 => {
                    self.to_rank_value(self.combiner.eval(InputScorePair::for_side(side, s, opp)))
                }
                _ => f64::NEG_INFINITY,
            };
            let u_zero = match (s_zero, opp_bound) {
                (Some(s), Some(_)) => {
                    self.to_rank_value(self.combiner.eval(InputScorePair::for_side(side, s, 0.0)))
                }
                _ => f64::NEG_INFINITY,
            };
            (u_pos, u_zero)
        }
    }

    /// Computes detailed upper bounds on the `rank_value` of any unseen pair:
    /// - `u_ra_l_pos`: `Left` unprobed row + `Right` unseen positive-score row (`score > 0.0`),
    /// - `u_ra_l_zero`: `Left` zero-unprobed row + `Right` unseen `0.0`-score row,
    /// - `u_ra_r_pos`: `Left` unseen positive-score row (`score > 0.0`) + `Right` unprobed row,
    /// - `u_ra_r_zero`: `Left` unseen `0.0`-score row + `Right` zero-unprobed row,
    /// - `u_sa`: `Left` unseen row + `Right` unseen row.
    fn compute_detailed_bounds(&self) -> DetailedBounds {
        let cursor_bound = |cursor: &RankedInputCursor| {
            if !cursor.sa_fully_consumed() {
                cursor.bottom_score
            } else if cursor.ra_capable {
                Some(0.0)
            } else {
                None
            }
        };
        let left_bound = cursor_bound(&self.left_cursor);
        let right_bound = cursor_bound(&self.right_cursor);

        let (u_ra_l_pos, u_ra_l_zero) = self.compute_side_ra_bounds(AdvanceSide::Left, right_bound);
        let (u_ra_r_pos, u_ra_r_zero) = self.compute_side_ra_bounds(AdvanceSide::Right, left_bound);

        let u_sa = if self.left_cursor.sa_fully_consumed() && self.right_cursor.sa_fully_consumed()
        {
            f64::NEG_INFINITY
        } else {
            match (left_bound, right_bound) {
                (Some(l), Some(r)) => {
                    self.to_rank_value(self.combiner.eval(InputScorePair { left: l, right: r }))
                }
                _ => f64::NEG_INFINITY,
            }
        };

        DetailedBounds {
            u_ra_l_pos,
            u_ra_l_zero,
            u_ra_r_pos,
            u_ra_r_zero,
            u_sa,
        }
    }

    /// Computes `(u_ra_l, u_ra_r, u_sa)`, the upper bounds on the
    /// `rank_value` of any unseen pair from:
    /// 1. `Left` seen row + `Right` unseen row (`u_ra_l`),
    /// 2. `Left` unseen row + `Right` seen row (`u_ra_r`),
    /// 3. `Left` unseen row + `Right` unseen row (`u_sa`).
    fn compute_bounds(&self) -> (f64, f64, f64) {
        let b = self.compute_detailed_bounds();
        (
            b.u_ra_l_pos.max(b.u_ra_l_zero),
            b.u_ra_r_pos.max(b.u_ra_r_zero),
            b.u_sa,
        )
    }

    /// Global stopping threshold `T = max(u_ra_l, u_ra_r, u_sa)`.
    fn stopping_threshold(&self) -> f64 {
        let (u_ra_l, u_ra_r, u_sa) = self.compute_bounds();
        u_ra_l.max(u_ra_r).max(u_sa)
    }

    fn has_tie_breakers(&self) -> bool {
        self.output_ordering.len() > 1 || self.primary_sort_needs_exact_tie_break
    }

    fn candidate_is_ready(&self, rank_value: f64, threshold: f64) -> bool {
        if threshold == f64::NEG_INFINITY {
            true
        } else if self.has_tie_breakers() {
            rank_value > threshold
        } else {
            rank_value >= threshold
        }
    }

    /// Converts a slice of `OwnedRow` join keys back into `Vec<Vec<ScalarValue>>` for
    /// constructing a [`RankLookupPhysicalExpr`].
    fn owned_rows_to_scalars(&self, rows: &[OwnedRow]) -> Result<Vec<Vec<ScalarValue>>> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let _timer = self.elapsed_compute.timer();
        let parser = self.left_cursor.key_converter.parser();
        let raw_rows: Vec<_> = rows.iter().map(|r| parser.parse(r.as_ref())).collect();
        let arrays = self.left_cursor.key_converter.convert_rows(raw_rows)?;
        let num_cols = arrays.len();
        let mut out = Vec::with_capacity(rows.len());
        for row_idx in 0..rows.len() {
            let mut tuple = Vec::with_capacity(num_cols);
            for col_arr in &arrays {
                tuple.push(ScalarValue::try_from_array(col_arr, row_idx)?);
            }
            out.push(tuple);
        }
        Ok(out)
    }

    /// Advances the specified side's `SA` cursor by one buffered row, lowering `bottom_score`
    /// and recording its join key for `RA` probing on the opposite side.
    fn advance_one_row(&mut self, side: AdvanceSide) {
        let (cursor, opp_cursor) = match side {
            AdvanceSide::Left => (&mut self.left_cursor, &self.right_cursor),
            AdvanceSide::Right => (&mut self.right_cursor, &self.left_cursor),
        };
        cursor.advance_buffered_row(opp_cursor.ra_capable, &opp_cursor.ra_probed_keys);
    }

    /// Inserts all rows of the newly pulled `SA` batch on `side` into `side`'s hash table
    /// (`from_sa: true`) and pairs them with existing entries in the opposite hash table.
    fn insert_current_sa_batch_into_hash_table(&mut self, side: AdvanceSide) {
        let _timer = self.elapsed_compute.timer();
        let primary_score_is_f32 = self.primary_score_is_f32;
        let descending = self.output_ordering[0].options.descending;
        let (cursor, opp_cursor) = match side {
            AdvanceSide::Left => (&mut self.left_cursor, &self.right_cursor),
            AdvanceSide::Right => (&mut self.right_cursor, &self.left_cursor),
        };
        let Some(batch_idx) = cursor.sa_batch_idx else {
            return;
        };
        let num_rows = cursor.current_batch_scores.len();
        for row_idx in 0..num_rows {
            let Some(key) = cursor.current_batch_keys[row_idx].clone() else {
                continue;
            };
            let score = cursor.current_batch_scores[row_idx];
            let row_ref = BufferedRowRef {
                batch: BatchIndex(batch_idx),
                row: RowIndex(row_idx),
            };
            if let Some(matches) = opp_cursor.hash_table.get(&key) {
                for opp_entry in matches {
                    let combined =
                        self.combiner
                            .eval(InputScorePair::for_side(side, score, opp_entry.score));
                    let rank_value =
                        Self::score_to_rank_value(combined, primary_score_is_f32, descending);
                    let seq = self.next_sequence;
                    self.next_sequence += 1;
                    self.candidate_heap.push(CandidateMatch::for_side(
                        side,
                        row_ref,
                        opp_entry.row_ref,
                        rank_value,
                        seq,
                    ));
                }
            }
            cursor
                .hash_table
                .entry(key)
                .or_default()
                .push(HashTableEntry {
                    row_ref,
                    score,
                    from_sa: true,
                });
        }
    }

    /// Ingests a `RecordBatch` returned from a Random Access (`RA`) point lookup on `side`
    /// without modifying `side`'s `SA` cursor state or `bottom_score`.
    ///
    /// Zero-score `RA` rows (`score <= 0.0`) are only paired with opposite-side hash table entries
    /// that arrived via `SA` (`entry.from_sa == true`), preventing `(RA_only, RA_only)` pairs that
    /// satisfy neither side's disjunctive search predicate when join keys are non-unique.
    /// Positive-score `RA` rows (`score > 0.0`) matched their own table's search query (and were
    /// added to `emitted_docs`, so `SA` will not re-emit them), so they are treated as `from_sa`
    /// and recorded in `sa_keys_in_order` for Tier-2 probing on the opposite side.
    fn ingest_ra_batch(&mut self, side: AdvanceSide, batch: RecordBatch) -> Result<()> {
        if batch.num_rows() == 0 {
            return Ok(());
        }
        let _timer = self.elapsed_compute.timer();
        let primary_score_is_f32 = self.primary_score_is_f32;
        let descending = self.output_ordering[0].options.descending;
        let (cursor, opp_cursor) = match side {
            AdvanceSide::Left => (&mut self.left_cursor, &self.right_cursor),
            AdvanceSide::Right => (&mut self.right_cursor, &self.left_cursor),
        };
        let (scores, keys) = cursor.evaluate_batch(&batch)?;
        let batch_idx = cursor.batches.len();
        cursor.batches.push(batch);
        cursor.batch_scores.push(scores.clone());
        for (row_idx, (score, key_opt)) in scores.into_iter().zip(keys).enumerate() {
            let Some(key) = key_opt else {
                continue;
            };
            let row_ref = BufferedRowRef {
                batch: BatchIndex(batch_idx),
                row: RowIndex(row_idx),
            };
            let from_sa = score > 0.0;
            if let Some(matches) = opp_cursor.hash_table.get(&key) {
                for opp_entry in matches.iter().filter(|e| from_sa || e.from_sa) {
                    let combined =
                        self.combiner
                            .eval(InputScorePair::for_side(side, score, opp_entry.score));
                    let rank_value =
                        Self::score_to_rank_value(combined, primary_score_is_f32, descending);
                    let seq = self.next_sequence;
                    self.next_sequence += 1;
                    self.candidate_heap.push(CandidateMatch::for_side(
                        side,
                        row_ref,
                        opp_entry.row_ref,
                        rank_value,
                        seq,
                    ));
                }
            }
            if from_sa && opp_cursor.ra_capable && !opp_cursor.ra_probed_keys.contains(&key) {
                cursor.record_positive_key(key.clone(), score);
            }
            cursor
                .hash_table
                .entry(key)
                .or_default()
                .push(HashTableEntry {
                    row_ref,
                    score,
                    from_sa,
                });
        }
        Ok(())
    }

    /// Finds all buffered rows in `target_side` across all retained batches that satisfy `lookup`
    /// and (when `target_thresh` is `Some(t)`) have `score > t`, returning their
    /// `(BufferedRowRef, join_key)` pairs.
    fn find_matching_buffered_rows(
        &self,
        target_side: AdvanceSide,
        lookup: &RankLookupPhysicalExpr,
        target_thresh: Option<f32>,
    ) -> Result<Vec<(BufferedRowRef, OwnedRow)>> {
        let _timer = self.elapsed_compute.timer();
        let cursor = self.cursor(target_side);
        let skip_nulls = cursor.null_equality == NullEquality::NullEqualsNothing;
        let mut out = Vec::new();
        for (batch_idx, batch) in cursor.batches.iter().enumerate() {
            let num_rows = batch.num_rows();
            if num_rows == 0 {
                continue;
            }
            let mask_val = lookup.evaluate(batch)?.into_array(num_rows)?;
            let mask = mask_val
                .as_any()
                .downcast_ref::<BooleanArray>()
                .ok_or_else(|| {
                    DataFusionError::Internal(
                        "RankLookupPhysicalExpr did not evaluate to BooleanArray".into(),
                    )
                })?;
            if mask.true_count() == 0 {
                continue;
            }
            let key_arrays: Vec<ArrayRef> = cursor
                .key_exprs
                .iter()
                .map(|expr| expr.evaluate(batch)?.into_array(num_rows))
                .collect::<Result<_>>()?;
            let rows = cursor.key_converter.convert_columns(&key_arrays)?;
            let scores = &cursor.batch_scores[batch_idx];
            for (row_idx, &score) in scores.iter().enumerate().take(num_rows) {
                if !mask.is_null(row_idx) && mask.value(row_idx) {
                    if let Some(t) = target_thresh
                        && (score as f32) <= t
                    {
                        continue;
                    }
                    if skip_nulls && key_arrays.iter().any(|arr| arr.is_null(row_idx)) {
                        continue;
                    }
                    out.push((
                        BufferedRowRef {
                            batch: BatchIndex(batch_idx),
                            row: RowIndex(row_idx),
                        },
                        rows.row(row_idx).owned(),
                    ));
                }
            }
        }
        Ok(out)
    }

    /// Checks whether a parent [`RankJoinExec`] in a multi-way join has installed a cascading
    /// [`RankLookupPhysicalExpr`] on one of this join's sides.
    fn take_parent_lookup_request(&self) -> Option<(usize, AdvanceSide, RankLookupPhysicalExpr)> {
        for (filter_idx, (side, pf)) in self.parent_lookup_filters.iter().enumerate() {
            if let Some(current_expr) = current_lookup_filter(pf)
                && let Some(lookup) = current_expr.downcast_ref::<RankLookupPhysicalExpr>()
            {
                let target_side = match side {
                    JoinSide::Left => AdvanceSide::Left,
                    JoinSide::Right => AdvanceSide::Right,
                    JoinSide::None => continue,
                };
                let lookup_cloned = lookup.clone();
                // Note: if the target child is not `ra_capable` (already exhausted), reset the
                // filter here; otherwise `execute_parent_lookup` or the target leaf scan will
                // consume and reset it.
                if !self.cursor(target_side).ra_capable {
                    let _ = update_lookup_filter(pf, lit(true));
                }
                return Some((filter_idx, target_side, lookup_cloned));
            }
        }
        None
    }

    /// Performs a batched Random Access (`RA`) lookup on `side` for `keys` with optional
    /// strict `score_threshold` pushdown and ingests any returned rows into `side`'s hash
    /// table and `candidate_heap`.
    async fn execute_ra_lookup(
        &mut self,
        side: AdvanceSide,
        keys: &[OwnedRow],
        score_threshold: Option<f32>,
    ) -> Result<()> {
        if keys.is_empty() {
            return Ok(());
        }
        let scalars = self.owned_rows_to_scalars(keys)?;
        let (key_exprs, lookup_filter) = {
            let cursor = self.cursor(side);
            (cursor.key_exprs.clone(), Arc::clone(&cursor.lookup_filter))
        };
        let lookup_expr = Arc::new(
            RankLookupPhysicalExpr::new(key_exprs, scalars).with_score_threshold(score_threshold),
        );
        update_lookup_filter(&lookup_filter, lookup_expr)?;

        let next_res = self.cursor_mut(side).stream.next().await;
        let _ = update_lookup_filter(&lookup_filter, lit(true));

        match next_res {
            Some(Err(e)) => Err(e),
            None => {
                let target_cursor = self.cursor_mut(side);
                target_cursor.sa_exhausted = true;
                target_cursor.ra_capable = false;
                Ok(())
            }
            Some(Ok(batch)) => {
                if batch.num_rows() > 0 {
                    self.ingest_ra_batch(side, batch)?;
                }
                Ok(())
            }
        }
    }

    /// Executes a cascading `RA` lookup requested by a parent [`RankJoinExec`].
    async fn execute_parent_lookup(
        &mut self,
        filter_idx: usize,
        target_side: AdvanceSide,
        lookup: RankLookupPhysicalExpr,
    ) -> Result<RecordBatch> {
        let opposite_side = target_side.opposite();
        let parent_thresh = lookup.score_threshold();
        let target_thresh = parent_thresh.and_then(|pt| {
            if !self.initialized {
                return None;
            }
            let max_opp = self.cursor(opposite_side).top_score.unwrap_or(0.0);
            self.combiner.threshold_for(target_side, pt, max_opp)
        });

        if self.cursor(target_side).ra_capable {
            // Filter out keys that were already queried on `target_side` at an equal or looser
            // `target_thresh` (for example, when a parent's Tier-1 `Some(0.0)` lookup already
            // widened to `target_thresh = None` on `target_side` and fetched all matching rows).
            let unprobed_target_keys = self.parent_target_probed[filter_idx]
                .take_unprobed_keys(lookup.keys(), target_thresh);
            if unprobed_target_keys.is_empty() {
                for (_, pf) in &self.parent_lookup_filters {
                    let _ = update_lookup_filter(pf, lit(true));
                }
            } else {
                if target_thresh != parent_thresh
                    || unprobed_target_keys.len() != lookup.keys().len()
                {
                    let target_lookup: Arc<dyn PhysicalExpr> = Arc::new(
                        lookup.with_keys_and_threshold(unprobed_target_keys, target_thresh),
                    );
                    let target_join_side = target_side.to_join_side();
                    for (side, pf) in &self.parent_lookup_filters {
                        if *side == target_join_side {
                            let _ = update_lookup_filter(pf, Arc::clone(&target_lookup));
                        }
                    }
                }
                let next_res = self.cursor_mut(target_side).stream.next().await;
                for (_, pf) in &self.parent_lookup_filters {
                    let _ = update_lookup_filter(pf, lit(true));
                }
                match next_res {
                    Some(Err(e)) => return Err(e),
                    None => {
                        let target_cursor = self.cursor_mut(target_side);
                        target_cursor.sa_exhausted = true;
                        target_cursor.ra_capable = false;
                    }
                    Some(Ok(batch)) => {
                        if batch.num_rows() > 0 {
                            self.ingest_ra_batch(target_side, batch)?;
                        }
                    }
                }
            }
        }

        let matching_target_rows =
            self.find_matching_buffered_rows(target_side, &lookup, target_thresh)?;

        if self.cursor(opposite_side).ra_capable && !matching_target_rows.is_empty() {
            let mut unfiltered_keys = Vec::new();
            let mut threshold_keys = Vec::new();
            let mut threshold_seen: HashSet<OwnedRow> = HashSet::default();
            let mut min_opp_thresh: Option<f32> = None;

            for (row_ref, key) in &matching_target_rows {
                let target_score = self.cursor(target_side).row_score(*row_ref);
                let row_opp_thresh = parent_thresh
                    .and_then(|pt| self.combiner.threshold_for(opposite_side, pt, target_score));
                match row_opp_thresh {
                    None => {
                        let opposite_cursor = self.cursor_mut(opposite_side);
                        opposite_cursor.ra_positive_probed_keys.insert(key.clone());
                        if opposite_cursor.ra_probed_keys.insert(key.clone()) {
                            unfiltered_keys.push(key.clone());
                        }
                    }
                    Some(t) => {
                        let opposite_cursor = self.cursor(opposite_side);
                        let already_probed = opposite_cursor.ra_probed_keys.contains(key)
                            || (t >= 0.0 && opposite_cursor.ra_positive_probed_keys.contains(key));
                        if !already_probed && threshold_seen.insert(key.clone()) {
                            threshold_keys.push(key.clone());
                            min_opp_thresh = Some(min_opp_thresh.map_or(t, |m| m.min(t)));
                        }
                    }
                }
            }

            if !unfiltered_keys.is_empty() {
                self.execute_ra_lookup(opposite_side, &unfiltered_keys, None)
                    .await?;
            }
            if !threshold_keys.is_empty() {
                let opp_thresh = min_opp_thresh.unwrap_or(0.0);
                let filtered_threshold_keys: Vec<OwnedRow> = {
                    let opposite_cursor = self.cursor_mut(opposite_side);
                    threshold_keys
                        .into_iter()
                        .filter(|k| {
                            if opposite_cursor.ra_probed_keys.contains(k) {
                                return false;
                            }
                            if opp_thresh == 0.0 {
                                opposite_cursor.ra_positive_probed_keys.insert(k.clone())
                            } else {
                                true
                            }
                        })
                        .collect()
                };
                if !filtered_threshold_keys.is_empty() {
                    self.execute_ra_lookup(
                        opposite_side,
                        &filtered_threshold_keys,
                        Some(opp_thresh),
                    )
                    .await?;
                }
            }
        }

        self.materialize_parent_lookup(target_side, &matching_target_rows, parent_thresh)
    }

    fn materialize_parent_lookup(
        &mut self,
        target_side: AdvanceSide,
        matching_target_rows: &[(BufferedRowRef, OwnedRow)],
        parent_thresh: Option<f32>,
    ) -> Result<RecordBatch> {
        let _timer = self.elapsed_compute.timer();
        let (target_cursor, opp_cursor) = match target_side {
            AdvanceSide::Left => (&self.left_cursor, &self.right_cursor),
            AdvanceSide::Right => (&self.right_cursor, &self.left_cursor),
        };
        let mut unadvanced_opp: HashMap<&OwnedRow, Vec<BufferedRowRef>> = HashMap::default();
        if opp_cursor.has_buffered_rows()
            && let Some(batch_idx) = opp_cursor.sa_batch_idx
        {
            for row_idx in opp_cursor.current_row_idx..opp_cursor.current_batch_keys.len() {
                if let Some(ref k) = opp_cursor.current_batch_keys[row_idx] {
                    unadvanced_opp.entry(k).or_default().push(BufferedRowRef {
                        batch: BatchIndex(batch_idx),
                        row: RowIndex(row_idx),
                    });
                }
            }
        }
        let mut candidates = Vec::new();
        for (target_row_ref, key) in matching_target_rows {
            let target_score = target_cursor.row_score(*target_row_ref);
            let mut try_push = |opp_row_ref: BufferedRowRef, opp_score: f64| {
                if let Some(pt) = parent_thresh {
                    let combined = self.combiner.eval(InputScorePair::for_side(
                        target_side,
                        target_score,
                        opp_score,
                    ));
                    if (combined as f32) <= pt {
                        return;
                    }
                }
                let cand =
                    CandidateMatch::for_side(target_side, *target_row_ref, opp_row_ref, 0.0, 0);
                if self.emitted_pairs.insert((cand.left_row, cand.right_row)) {
                    candidates.push(cand);
                }
            };
            if let Some(matches) = opp_cursor.hash_table.get(key) {
                for opp_entry in matches {
                    try_push(opp_entry.row_ref, opp_entry.score);
                }
            }
            if let Some(unadv_matches) = unadvanced_opp.get(key) {
                for &opp_row_ref in unadv_matches {
                    try_push(opp_row_ref, opp_cursor.row_score(opp_row_ref));
                }
            }
        }

        if candidates.is_empty() {
            return Ok(RecordBatch::new_empty(Arc::clone(&self.schema)));
        }
        if let Some(filter) = &self.filter {
            candidates = self.apply_join_filter(filter, candidates)?;
            if candidates.is_empty() {
                return Ok(RecordBatch::new_empty(Arc::clone(&self.schema)));
            }
        }
        self.materialize_batch(&candidates)
    }

    /// Pops ready candidates from `candidate_heap`, applies any residual `JoinFilter`,
    /// and materializes the output `RecordBatch`.
    fn drain_ready_batch(&mut self) -> Result<Option<RecordBatch>> {
        let _timer = self.elapsed_compute.timer();
        let threshold = self.stopping_threshold();
        let mut ready = Vec::new();
        while let Some(top) = self.candidate_heap.peek() {
            if !self.candidate_is_ready(top.rank_value, threshold) {
                break;
            }
            let popped = self.candidate_heap.pop().unwrap();
            if !self
                .emitted_pairs
                .insert((popped.left_row, popped.right_row))
            {
                continue;
            }
            let last_rank_value = popped.rank_value;
            ready.push(popped);

            if ready.len() >= self.batch_size {
                if self.has_tie_breakers() {
                    // Drain any remaining ties with the exact same `rank_value` so that
                    // tie-breaking across secondary sort columns never splits a tie group.
                    while let Some(next) = self.candidate_heap.peek() {
                        if next.rank_value.to_bits() == last_rank_value.to_bits() {
                            let tie = self.candidate_heap.pop().unwrap();
                            if self.emitted_pairs.insert((tie.left_row, tie.right_row)) {
                                ready.push(tie);
                            }
                        } else {
                            break;
                        }
                    }
                }
                break;
            }
        }

        if ready.is_empty() {
            return Ok(None);
        }

        if let Some(filter) = &self.filter {
            ready = self.apply_join_filter(filter, ready)?;
            if ready.is_empty() {
                return Ok(None);
            }
        }

        let batch = self.materialize_batch(&ready)?;
        if self.has_tie_breakers() && batch.num_rows() > 1 {
            let sorted = sort_batch(&batch, &self.output_ordering, None)?;
            return Ok(Some(sorted));
        }
        Ok(Some(batch))
    }

    fn apply_join_filter(
        &self,
        filter: &JoinFilter,
        candidates: Vec<CandidateMatch>,
    ) -> Result<Vec<CandidateMatch>> {
        let left_indices: Vec<(usize, usize)> = candidates
            .iter()
            .map(|c| (c.left_row.batch.0, c.left_row.row.0))
            .collect();
        let right_indices: Vec<(usize, usize)> = candidates
            .iter()
            .map(|c| (c.right_row.batch.0, c.right_row.row.0))
            .collect();

        let mut filter_columns = Vec::with_capacity(filter.column_indices().len());
        for col_idx in filter.column_indices() {
            let array = match col_idx.side {
                JoinSide::Left => Self::interleave_column(
                    &self.left_cursor.batches,
                    col_idx.index,
                    &left_indices,
                )?,
                JoinSide::Right => Self::interleave_column(
                    &self.right_cursor.batches,
                    col_idx.index,
                    &right_indices,
                )?,
                JoinSide::None => {
                    return internal_err!(
                        "RankJoinStream: unexpected JoinSide::None in JoinFilter"
                    );
                }
            };
            filter_columns.push(array);
        }

        let filter_batch = RecordBatch::try_new_with_options(
            Arc::clone(filter.schema()),
            filter_columns,
            &RecordBatchOptions::new().with_row_count(Some(candidates.len())),
        )?;
        let filter_values = filter
            .expression()
            .evaluate(&filter_batch)?
            .into_array(candidates.len())?;
        let mask = filter_values
            .as_any()
            .downcast_ref::<BooleanArray>()
            .ok_or_else(|| {
                DataFusionError::Internal(
                    "RankJoinStream: JoinFilter did not evaluate to BooleanArray".into(),
                )
            })?;

        let filtered = candidates
            .into_iter()
            .enumerate()
            .filter_map(|(i, c)| (!mask.is_null(i) && mask.value(i)).then_some(c))
            .collect();
        Ok(filtered)
    }

    fn materialize_batch(&self, candidates: &[CandidateMatch]) -> Result<RecordBatch> {
        let left_indices: Vec<(usize, usize)> = candidates
            .iter()
            .map(|c| (c.left_row.batch.0, c.left_row.row.0))
            .collect();
        let right_indices: Vec<(usize, usize)> = candidates
            .iter()
            .map(|c| (c.right_row.batch.0, c.right_row.row.0))
            .collect();

        let total_unprojected = self.left_num_cols + self.right_num_cols;
        let output_col_indices: Vec<usize> = match &self.projection {
            Some(proj) => proj.clone(),
            None => (0..total_unprojected).collect(),
        };

        let mut columns = Vec::with_capacity(output_col_indices.len());
        for join_col_idx in output_col_indices {
            let array = if join_col_idx < self.left_num_cols {
                Self::interleave_column(&self.left_cursor.batches, join_col_idx, &left_indices)?
            } else {
                let right_col_idx = join_col_idx - self.left_num_cols;
                Self::interleave_column(&self.right_cursor.batches, right_col_idx, &right_indices)?
            };
            columns.push(array);
        }

        let batch = RecordBatch::try_new_with_options(
            Arc::clone(&self.schema),
            columns,
            &RecordBatchOptions::new().with_row_count(Some(candidates.len())),
        )?;
        Ok(batch)
    }

    fn interleave_column(
        batches: &[RecordBatch],
        col_idx: usize,
        indices: &[(usize, usize)],
    ) -> Result<ArrayRef> {
        let slices: Vec<&dyn Array> = batches.iter().map(|b| b.column(col_idx).as_ref()).collect();
        Ok(interleave(&slices, indices)?)
    }

    /// Pulls the initial `SA` batch from each non-exhausted input so `top_score` is known, and
    /// advances the first row from each available cursor.
    async fn initialize_sa_cursors(&mut self) -> Result<()> {
        if !self.left_cursor.sa_exhausted {
            self.ensure_side_buffered(AdvanceSide::Left).await?;
        }
        if !self.right_cursor.sa_exhausted {
            self.ensure_side_buffered(AdvanceSide::Right).await?;
        }
        self.initialized = true;
        if self.left_cursor.has_buffered_rows() {
            self.advance_one_row(AdvanceSide::Left);
        }
        if self.right_cursor.has_buffered_rows() {
            self.advance_one_row(AdvanceSide::Right);
        }
        Ok(())
    }

    /// Pulls from `side`'s `SA` stream until at least one buffered row is available or `SA` is exhausted,
    /// indexing any newly pulled `SA` batch into `side`'s hash table.
    async fn ensure_side_buffered(&mut self, side: AdvanceSide) -> Result<bool> {
        let had_buffered = self.cursor(side).has_buffered_rows();
        let has_buffered = self.cursor_mut(side).ensure_buffered().await?;
        if !had_buffered && has_buffered {
            self.insert_current_sa_batch_into_hash_table(side);
        }
        Ok(has_buffered)
    }

    /// Returns `true` if the top candidate in `candidate_heap` beats `stopping_threshold()`.
    fn has_ready_candidates(&self) -> bool {
        let threshold = self.stopping_threshold();
        self.candidate_heap
            .peek()
            .is_some_and(|top| self.candidate_is_ready(top.rank_value, threshold))
    }

    /// Returns `true` if either input is completely empty and cannot produce rows via `SA` or `RA`.
    fn has_empty_exhausted_input(&self) -> bool {
        [&self.left_cursor, &self.right_cursor]
            .into_iter()
            .any(|c| c.sa_fully_consumed() && !c.ra_capable && c.hash_table.is_empty())
    }

    /// Chooses between `Left` and `Right` when both sides have available `SA` rows.
    ///
    /// When both sides support `RA` point lookups, all seen `SA` rows are resolved via `RA` and the
    /// remaining `SA` frontier is `u_sa = g(bot_l, bot_r)`, so advancing
    /// both cursors at balanced `SA` row depth (`sa_rows_advanced`) is 2-competitive with the omniscient
    /// offline optimal `(n_l_opt, n_r_opt)` regardless of score skew. When `RA` is unavailable on either
    /// side, falls back to the corner-bound priority `p_l = g(bot_l, top_r)` vs.
    /// `p_r = g(top_l, bot_r)`.
    fn preferred_corner_side(&self, left_avail: bool, right_avail: bool) -> Option<AdvanceSide> {
        if left_avail && right_avail {
            if self.left_cursor.ra_capable && self.right_cursor.ra_capable {
                return Some(
                    if self.left_cursor.sa_rows_advanced <= self.right_cursor.sa_rows_advanced {
                        AdvanceSide::Left
                    } else {
                        AdvanceSide::Right
                    },
                );
            }
            let top_l = self.left_cursor.top_score.unwrap_or(0.0);
            let top_r = self.right_cursor.top_score.unwrap_or(0.0);
            let bot_l = self.left_cursor.bottom_score.unwrap_or(top_l);
            let bot_r = self.right_cursor.bottom_score.unwrap_or(top_r);
            let p_l = self.to_rank_value(self.combiner.eval(InputScorePair {
                left: bot_l,
                right: top_r,
            }));
            let p_r = self.to_rank_value(self.combiner.eval(InputScorePair {
                left: top_l,
                right: bot_r,
            }));
            Some(if p_l >= p_r {
                AdvanceSide::Left
            } else {
                AdvanceSide::Right
            })
        } else if left_avail {
            Some(AdvanceSide::Left)
        } else if right_avail {
            Some(AdvanceSide::Right)
        } else {
            None
        }
    }

    /// Rule 1: Selects which side's in-memory `SA` buffer to advance next, pausing a side once it
    /// has accumulated `ra_batch_size` unresolved keys so Rule 2 can interleave a batched `RA`
    /// lookup before draining the remainder of a large `SA` window.
    fn next_buffered_sa_side(&mut self) -> Option<AdvanceSide> {
        self.left_cursor.skip_probed_keys(
            &self.right_cursor.ra_positive_probed_keys,
            &self.right_cursor.ra_probed_keys,
        );
        self.right_cursor.skip_probed_keys(
            &self.left_cursor.ra_positive_probed_keys,
            &self.left_cursor.ra_probed_keys,
        );

        let left_can_advance = self.left_cursor.has_buffered_rows()
            && (!self.right_cursor.ra_capable
                || self.left_cursor.unresolved_key_count() < self.left_cursor.ra_batch_size);
        let right_can_advance = self.right_cursor.has_buffered_rows()
            && (!self.left_cursor.ra_capable
                || self.right_cursor.unresolved_key_count() < self.right_cursor.ra_batch_size);
        self.preferred_corner_side(left_can_advance, right_can_advance)
    }

    fn pop_ra_step(
        &mut self,
        source_side: AdvanceSide,
        pos_bound: f64,
        zero_bound: f64,
        cutoff: f64,
    ) -> NextIoStep {
        let target_side = source_side.opposite();
        let combiner = self.combiner.clone();
        let descending = self.output_ordering[0].options.descending;
        let primary_score_is_f32 = self.primary_score_is_f32;
        let (src_cursor, dst_cursor) = match source_side {
            AdvanceSide::Left => (&mut self.left_cursor, &mut self.right_cursor),
            AdvanceSide::Right => (&mut self.right_cursor, &mut self.left_cursor),
        };
        if pos_bound >= zero_bound && pos_bound > f64::NEG_INFINITY {
            let keys = src_cursor.pop_positive_unresolved_keys(
                &mut dst_cursor.ra_positive_probed_keys,
                &dst_cursor.ra_probed_keys,
            );
            NextIoStep::RaLookup {
                target_side,
                keys,
                score_threshold: Some(0.0),
            }
        } else {
            let keys = src_cursor.pop_all_unresolved_keys(
                &mut dst_cursor.ra_positive_probed_keys,
                &mut dst_cursor.ra_probed_keys,
                |src_score| {
                    let combined =
                        combiner.eval(InputScorePair::for_side(source_side, src_score, 0.0));
                    Self::score_to_rank_value(combined, primary_score_is_f32, descending) >= cutoff
                },
            );
            NextIoStep::RaLookup {
                target_side,
                keys,
                score_threshold: None,
            }
        }
    }

    /// Rule 2: Compares `(u_ra_l, u_ra_r, u_sa)` to choose the next
    /// async child I/O operation (Tier-1 `score > 0.0` `RA` lookup, Tier-2 `score >= 0.0` `RA`
    /// lookup, or pulling the next `SA` batch).
    fn next_io_step(&mut self) -> NextIoStep {
        let b = self.compute_detailed_bounds();
        let u_ra_l = b.u_ra_l_pos.max(b.u_ra_l_zero);
        let u_ra_r = b.u_ra_r_pos.max(b.u_ra_r_zero);
        let u_sa = b.u_sa;

        let can_ra_l = self.right_cursor.ra_capable && u_ra_l > f64::NEG_INFINITY;
        let can_ra_r = self.left_cursor.ra_capable && u_ra_r > f64::NEG_INFINITY;
        let can_sa = (!self.left_cursor.sa_fully_consumed()
            || !self.right_cursor.sa_fully_consumed())
            && u_sa > f64::NEG_INFINITY;

        let eff_ra_l = if can_ra_l { u_ra_l } else { f64::NEG_INFINITY };
        let eff_ra_r = if can_ra_r { u_ra_r } else { f64::NEG_INFINITY };
        let eff_sa = if can_sa { u_sa } else { f64::NEG_INFINITY };

        if eff_ra_l > f64::NEG_INFINITY && eff_ra_l >= eff_ra_r && eff_ra_l >= eff_sa {
            return self.pop_ra_step(AdvanceSide::Left, b.u_ra_l_pos, b.u_ra_l_zero, eff_sa);
        }

        if eff_ra_r > f64::NEG_INFINITY && eff_ra_r >= eff_sa {
            return self.pop_ra_step(AdvanceSide::Right, b.u_ra_r_pos, b.u_ra_r_zero, eff_sa);
        }

        let left_sa_avail = !self.left_cursor.sa_fully_consumed();
        let right_sa_avail = !self.right_cursor.sa_fully_consumed();
        if let Some(pull_side) = self.preferred_corner_side(left_sa_avail, right_sa_avail) {
            return NextIoStep::SaPull(pull_side);
        }

        NextIoStep::Exhausted
    }

    /// If this join is a child of a parent [`RankJoinExec`] and can still service cascading `RA`
    /// lookups after `SA` is exhausted, signals `lit(false)` to the direct parent and returns `true`.
    fn signal_sa_exhausted_to_parent(&self) -> bool {
        if !self.parent_lookup_filters.is_empty()
            && (self.left_cursor.ra_capable || self.right_cursor.ra_capable)
        {
            if let Some((_, pf)) = self.parent_lookup_filters.first() {
                let _ = update_lookup_filter(pf, lit(false));
            }
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_schema::{Field, Schema, SortOptions};
    use datafusion::logical_expr::Operator;
    use datafusion::physical_expr::expressions::{BinaryExpr, Column};
    use datafusion::physical_plan::empty::EmptyExec;

    #[test]
    fn test_rank_join_codec_round_trip_and_preserved_ordering() -> Result<()> {
        let left_schema = Arc::new(Schema::new(vec![
            Field::new("l_id", DataType::Int32, false),
            Field::new("l_score", DataType::Float32, false),
        ]));
        let right_schema = Arc::new(Schema::new(vec![
            Field::new("r_id", DataType::Int32, false),
            Field::new("r_score", DataType::Float32, false),
        ]));
        let left_sort = PhysicalSortExpr {
            expr: Arc::new(Column::new("l_score", 1)),
            options: SortOptions {
                descending: true,
                nulls_first: false,
            },
        };
        let right_sort = PhysicalSortExpr {
            expr: Arc::new(Column::new("r_score", 1)),
            options: SortOptions {
                descending: true,
                nulls_first: false,
            },
        };
        let left_plan: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(Arc::clone(&left_schema)));
        let right_plan: Arc<dyn ExecutionPlan> =
            Arc::new(EmptyExec::new(Arc::clone(&right_schema)));
        let sum_expr: Arc<dyn PhysicalExpr> = Arc::new(BinaryExpr::new(
            Arc::new(Column::new("l_score", 1)),
            Operator::Plus,
            Arc::new(Column::new("r_score", 3)),
        ));
        let output_ordering = LexOrdering::new(vec![PhysicalSortExpr {
            expr: sum_expr,
            options: SortOptions {
                descending: true,
                nulls_first: false,
            },
        }])
        .unwrap();

        let exec = RankJoinExec::try_new(
            RankJoinInputs {
                left: Arc::clone(&left_plan),
                right: Arc::clone(&right_plan),
            },
            vec![(
                Arc::new(Column::new("l_id", 0)),
                Arc::new(Column::new("r_id", 0)),
            )],
            None,
            Some(vec![0, 1, 2, 3]),
            PartitionMode::CollectLeft,
            NullEquality::NullEqualsNothing,
            RankJoinSortExprs {
                left: left_sort,
                right: right_sort,
                output: output_ordering.clone(),
            },
            ScoreCombiner::Add(
                Box::new(ScoreCombiner::LeftScore),
                Box::new(ScoreCombiner::RightScore),
            ),
        )?;

        let converter = datafusion_proto::physical_plan::DefaultPhysicalProtoConverter {};
        let bytes = exec.encode_for_dispatch(&converter)?;
        let ctx = TaskContext::default();
        let decoded =
            RankJoinExec::decode_for_dispatch(&bytes, &[left_plan, right_plan], &ctx, &converter)?;
        assert_eq!(
            decoded.properties().output_ordering(),
            Some(&output_ordering)
        );

        // Verify `preserved_ordering` preserves the composite `l_score + r_score` sort expression
        // when a downstream lookup/decode changes a non-score column's type (`l_id: Int32 -> Utf8`).
        let decoded_output_schema = Arc::new(Schema::new(vec![
            Field::new("l_id", DataType::Utf8, false),
            Field::new("l_score", DataType::Float32, false),
            Field::new("r_id", DataType::Int32, false),
            Field::new("r_score", DataType::Float32, false),
        ]));
        let eq = crate::scan::deferred_lookup::preserved_ordering(&decoded, decoded_output_schema);
        assert_eq!(eq.output_ordering(), Some(output_ordering));
        Ok(())
    }
}
