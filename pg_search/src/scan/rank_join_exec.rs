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
//! `RankJoinExec` consumes up to two score-ordered input streams (`left` and `right`),
//! maintains in-memory hash tables and unresolved-key frontiers on both sides, resolves
//! blocking unmatched keys on demand via batched `RankLookupPhysicalExpr` dynamic filter
//! point lookups (including cascading `RA` probes across nested `RankJoinExec` nodes),
//! and emits joined rows in combined score order as soon as their score clears the
//! upper bound $T$ on any unseen future match.

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
    BaselineMetrics, ExecutionPlanMetricsSet, MetricsSet, RecordOutput,
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

/// Monotonic scoring function $f(s_L, s_R)$ combining left and right input scores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScoreCombiner {
    LeftScore,
    RightScore,
    Constant(f64),
    Add(Box<ScoreCombiner>, Box<ScoreCombiner>),
    Sub(Box<ScoreCombiner>, Box<ScoreCombiner>),
    Mul(Box<ScoreCombiner>, Box<ScoreCombiner>),
    Div(Box<ScoreCombiner>, Box<ScoreCombiner>),
    Greatest(Vec<ScoreCombiner>),
    Least(Vec<ScoreCombiner>),
    Ln(Box<ScoreCombiner>),
    Sqrt(Box<ScoreCombiner>),
    Exp(Box<ScoreCombiner>),
}

impl ScoreCombiner {
    /// Evaluates $f(s_L, s_R)$ for the given [`InputScorePair`].
    pub fn eval(&self, scores: InputScorePair) -> f64 {
        let raw = match self {
            ScoreCombiner::LeftScore => scores.left,
            ScoreCombiner::RightScore => scores.right,
            ScoreCombiner::Constant(c) => *c,
            ScoreCombiner::Add(a, b) => a.eval(scores) + b.eval(scores),
            ScoreCombiner::Sub(a, b) => a.eval(scores) - b.eval(scores),
            ScoreCombiner::Mul(a, b) => a.eval(scores) * b.eval(scores),
            ScoreCombiner::Div(a, b) => a.eval(scores) / b.eval(scores),
            ScoreCombiner::Greatest(args) => args
                .iter()
                .map(|arg| arg.eval(scores))
                .fold(f64::NEG_INFINITY, f64::max),
            ScoreCombiner::Least(args) => args
                .iter()
                .map(|arg| arg.eval(scores))
                .fold(f64::INFINITY, f64::min),
            ScoreCombiner::Ln(a) => a.eval(scores).ln(),
            ScoreCombiner::Sqrt(a) => a.eval(scores).sqrt(),
            ScoreCombiner::Exp(a) => a.eval(scores).exp(),
        };
        if raw.is_nan() { f64::NEG_INFINITY } else { raw }
    }

    /// Returns whether the combined score depends on the left child's score (`LeftScore`).
    pub fn uses_left(&self) -> bool {
        match self {
            ScoreCombiner::LeftScore => true,
            ScoreCombiner::RightScore | ScoreCombiner::Constant(_) => false,
            ScoreCombiner::Add(a, b)
            | ScoreCombiner::Sub(a, b)
            | ScoreCombiner::Mul(a, b)
            | ScoreCombiner::Div(a, b) => a.uses_left() || b.uses_left(),
            ScoreCombiner::Greatest(args) | ScoreCombiner::Least(args) => {
                args.iter().any(Self::uses_left)
            }
            ScoreCombiner::Ln(a) | ScoreCombiner::Sqrt(a) | ScoreCombiner::Exp(a) => a.uses_left(),
        }
    }

    /// Returns whether the combined score depends on the right child's score (`RightScore`).
    pub fn uses_right(&self) -> bool {
        match self {
            ScoreCombiner::RightScore => true,
            ScoreCombiner::LeftScore | ScoreCombiner::Constant(_) => false,
            ScoreCombiner::Add(a, b)
            | ScoreCombiner::Sub(a, b)
            | ScoreCombiner::Mul(a, b)
            | ScoreCombiner::Div(a, b) => a.uses_right() || b.uses_right(),
            ScoreCombiner::Greatest(args) | ScoreCombiner::Least(args) => {
                args.iter().any(Self::uses_right)
            }
            ScoreCombiner::Ln(a) | ScoreCombiner::Sqrt(a) | ScoreCombiner::Exp(a) => a.uses_right(),
        }
    }

    /// Given a required strict combined score floor `f(s_L, s_R) > min_combined` and an upper
    /// bound `0 <= s_R <= max_right`, returns a strict lower-bound threshold `t >= 0.0` on `s_L`
    /// (`s_L > t`, suitable for `PruningScorer`) if one holds by monotonicity.
    pub fn left_threshold_for(&self, min_combined: f32, max_right: f64) -> Option<f32> {
        let min_combined_f64 = min_combined as f64;
        if !self.uses_left() || !max_right.is_finite() || max_right < 0.0 {
            return None;
        }
        if self.eval(InputScorePair {
            left: 0.0,
            right: max_right,
        }) > min_combined_f64
        {
            return None;
        }
        if let Some(raw) = self.raw_min_left_score_for(min_combined_f64, max_right)
            && raw > 0.0
            && raw.is_finite()
        {
            let candidate = ((raw as f32) * (1.0 - 1e-5) - 1e-5).max(0.0);
            if self.eval(InputScorePair {
                left: candidate as f64,
                right: max_right,
            }) <= min_combined_f64
            {
                return Some(candidate);
            }
        }
        Some(0.0)
    }

    /// Given a required strict combined score floor `f(s_L, s_R) > min_combined` and an upper
    /// bound `0 <= s_L <= max_left`, returns a strict lower-bound threshold `t >= 0.0` on `s_R`
    /// (`s_R > t`, suitable for `PruningScorer`) if one holds by monotonicity.
    pub fn right_threshold_for(&self, min_combined: f32, max_left: f64) -> Option<f32> {
        let min_combined_f64 = min_combined as f64;
        if !self.uses_right() || !max_left.is_finite() || max_left < 0.0 {
            return None;
        }
        if self.eval(InputScorePair {
            left: max_left,
            right: 0.0,
        }) > min_combined_f64
        {
            return None;
        }
        if let Some(raw) = self.raw_min_right_score_for(min_combined_f64, max_left)
            && raw > 0.0
            && raw.is_finite()
        {
            let candidate = ((raw as f32) * (1.0 - 1e-5) - 1e-5).max(0.0);
            if self.eval(InputScorePair {
                left: max_left,
                right: candidate as f64,
            }) <= min_combined_f64
            {
                return Some(candidate);
            }
        }
        Some(0.0)
    }

    fn raw_min_left_score_for(&self, min_combined: f64, max_right: f64) -> Option<f64> {
        match self {
            ScoreCombiner::LeftScore => Some(min_combined),
            ScoreCombiner::RightScore | ScoreCombiner::Constant(_) => None,
            ScoreCombiner::Add(a, b) => {
                if !b.uses_left() {
                    let b_max = b.eval(InputScorePair {
                        left: 0.0,
                        right: max_right,
                    });
                    a.raw_min_left_score_for(min_combined - b_max, max_right)
                } else if !a.uses_left() {
                    let a_max = a.eval(InputScorePair {
                        left: 0.0,
                        right: max_right,
                    });
                    b.raw_min_left_score_for(min_combined - a_max, max_right)
                } else {
                    None
                }
            }
            ScoreCombiner::Sub(a, b) => {
                if !b.uses_left() {
                    let b_min = b.eval(InputScorePair {
                        left: 0.0,
                        right: 0.0,
                    });
                    a.raw_min_left_score_for(min_combined + b_min, max_right)
                } else {
                    None
                }
            }
            ScoreCombiner::Mul(a, b) => {
                if !b.uses_left() {
                    let b_max = b.eval(InputScorePair {
                        left: 0.0,
                        right: max_right,
                    });
                    (b_max > 0.0)
                        .then(|| a.raw_min_left_score_for(min_combined / b_max, max_right))?
                } else if !a.uses_left() {
                    let a_max = a.eval(InputScorePair {
                        left: 0.0,
                        right: max_right,
                    });
                    (a_max > 0.0)
                        .then(|| b.raw_min_left_score_for(min_combined / a_max, max_right))?
                } else {
                    None
                }
            }
            ScoreCombiner::Div(a, b) => {
                if !b.uses_left() {
                    let b_max = b.eval(InputScorePair {
                        left: 0.0,
                        right: max_right,
                    });
                    (b_max > 0.0)
                        .then(|| a.raw_min_left_score_for(min_combined * b_max, max_right))?
                } else {
                    None
                }
            }
            ScoreCombiner::Ln(a) => a.raw_min_left_score_for(min_combined.exp(), max_right),
            ScoreCombiner::Sqrt(a) => (min_combined > 0.0)
                .then(|| a.raw_min_left_score_for(min_combined * min_combined, max_right))?,
            ScoreCombiner::Exp(a) => (min_combined > 0.0)
                .then(|| a.raw_min_left_score_for(min_combined.ln(), max_right))?,
            ScoreCombiner::Least(args) => args
                .iter()
                .filter_map(|arg| arg.raw_min_left_score_for(min_combined, max_right))
                .reduce(f64::max),
            ScoreCombiner::Greatest(_) => None,
        }
    }

    fn raw_min_right_score_for(&self, min_combined: f64, max_left: f64) -> Option<f64> {
        match self {
            ScoreCombiner::RightScore => Some(min_combined),
            ScoreCombiner::LeftScore | ScoreCombiner::Constant(_) => None,
            ScoreCombiner::Add(a, b) => {
                if !a.uses_right() {
                    let a_max = a.eval(InputScorePair {
                        left: max_left,
                        right: 0.0,
                    });
                    b.raw_min_right_score_for(min_combined - a_max, max_left)
                } else if !b.uses_right() {
                    let b_max = b.eval(InputScorePair {
                        left: max_left,
                        right: 0.0,
                    });
                    a.raw_min_right_score_for(min_combined - b_max, max_left)
                } else {
                    None
                }
            }
            ScoreCombiner::Sub(a, b) => {
                if !b.uses_right() {
                    let b_min = b.eval(InputScorePair {
                        left: 0.0,
                        right: 0.0,
                    });
                    a.raw_min_right_score_for(min_combined + b_min, max_left)
                } else {
                    None
                }
            }
            ScoreCombiner::Mul(a, b) => {
                if !a.uses_right() {
                    let a_max = a.eval(InputScorePair {
                        left: max_left,
                        right: 0.0,
                    });
                    (a_max > 0.0)
                        .then(|| b.raw_min_right_score_for(min_combined / a_max, max_left))?
                } else if !b.uses_right() {
                    let b_max = b.eval(InputScorePair {
                        left: max_left,
                        right: 0.0,
                    });
                    (b_max > 0.0)
                        .then(|| a.raw_min_right_score_for(min_combined / b_max, max_left))?
                } else {
                    None
                }
            }
            ScoreCombiner::Div(a, b) => {
                if !b.uses_right() {
                    let b_max = b.eval(InputScorePair {
                        left: max_left,
                        right: 0.0,
                    });
                    (b_max > 0.0)
                        .then(|| a.raw_min_right_score_for(min_combined * b_max, max_left))?
                } else {
                    None
                }
            }
            ScoreCombiner::Ln(a) => a.raw_min_right_score_for(min_combined.exp(), max_left),
            ScoreCombiner::Sqrt(a) => (min_combined > 0.0)
                .then(|| a.raw_min_right_score_for(min_combined * min_combined, max_left))?,
            ScoreCombiner::Exp(a) => (min_combined > 0.0)
                .then(|| a.raw_min_right_score_for(min_combined.ln(), max_left))?,
            ScoreCombiner::Least(args) => args
                .iter()
                .filter_map(|arg| arg.raw_min_right_score_for(min_combined, max_left))
                .reduce(f64::max),
            ScoreCombiner::Greatest(_) => None,
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
        let _ = expr.apply(|node| {
            if let Some(col) = node.downcast_ref::<Column>() {
                saw_col = true;
                let out_idx = col.index();
                let join_idx = match &self.projection {
                    Some(proj) => proj.get(out_idx).copied(),
                    None => Some(out_idx),
                };
                let valid = match (side, join_idx) {
                    (JoinSide::Left, Some(idx)) => idx < left_len,
                    (JoinSide::Right, Some(idx)) => idx >= left_len && idx < total_len,
                    _ => false,
                };
                if !valid {
                    all_on_side = false;
                    return Ok(TreeNodeRecursion::Stop);
                }
            }
            Ok(TreeNodeRecursion::Continue)
        });
        if !all_on_side || !saw_col {
            return Ok(None);
        }

        let remapped = Arc::clone(expr)
            .transform_down(|node| {
                if let Some(col) = node.downcast_ref::<Column>() {
                    let out_idx = col.index();
                    let join_idx = match &self.projection {
                        Some(proj) => proj[out_idx],
                        None => out_idx,
                    };
                    match side {
                        JoinSide::Left => {
                            let field = left_schema.field(join_idx);
                            return Ok(Transformed::yes(Arc::new(Column::new(
                                field.name(),
                                join_idx,
                            ))));
                        }
                        JoinSide::Right => {
                            let child_idx = join_idx - left_len;
                            let field = right_schema.field(child_idx);
                            return Ok(Transformed::yes(Arc::new(Column::new(
                                field.name(),
                                child_idx,
                            ))));
                        }
                        JoinSide::None => unreachable!(),
                    }
                }
                Ok(Transformed::no(node))
            })?
            .data;
        Ok(Some(remapped))
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

        let left_desc = crate::scan::filter_pushdown::schema_preserving_child_filter_description(
            &left_parent_filters,
            &self.inputs.left.schema(),
            None,
        )?
        .with_self_filter(Arc::clone(&self.left_lookup_filter));
        let right_desc = crate::scan::filter_pushdown::schema_preserving_child_filter_description(
            &right_parent_filters,
            &self.inputs.right.schema(),
            None,
        )?
        .with_self_filter(Arc::clone(&self.right_lookup_filter));

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

        let left_cursor = RankedInputCursor::new(
            left_stream,
            Arc::clone(&self.sort_exprs.left.expr),
            left_keys,
            Arc::clone(&key_converter),
            self.null_equality,
            Arc::clone(&self.left_lookup_filter),
            left_ra_capable,
            !self.combiner.uses_left(),
        );
        let right_cursor = RankedInputCursor::new(
            right_stream,
            Arc::clone(&self.sort_exprs.right.expr),
            right_keys,
            key_converter,
            self.null_equality,
            Arc::clone(&self.right_lookup_filter),
            right_ra_capable,
            !self.combiner.uses_right(),
        );

        let batch_size = context.session_config().batch_size().max(1);
        let baseline_metrics = BaselineMetrics::new(&self.metrics, partition);

        let schema = self.schema();
        let mut state = RankJoinStream {
            schema: Arc::clone(&schema),
            left_num_cols: left_schema.fields().len(),
            right_num_cols: self.inputs.right.schema().fields().len(),
            filter: self.filter.clone(),
            projection: self.projection.clone(),
            output_ordering: self.sort_exprs.output.clone(),
            combiner: self.combiner.clone(),
            parent_lookup_filters: self.parent_lookup_filters.clone(),
            left_cursor,
            right_cursor,
            left_hash_table: HashMap::default(),
            right_hash_table: HashMap::default(),
            candidate_heap: BinaryHeap::new(),
            emitted_pairs: HashSet::default(),
            next_sequence: 0,
            initialized: false,
            batch_size,
        };

        let stream_gen = async_stream::try_stream! {
            // Phase 1: Sorted Access (`SA`) + interleaved Random Access (`RA`) threshold loop.
            'sa_loop: loop {
                // Service any cascading RA lookups requested by a parent `RankJoinExec` before
                // advancing SA.
                while let Some((target_side, lookup)) = state.take_parent_lookup_request() {
                    let batch = state.execute_parent_lookup(target_side, lookup).await?;
                    yield batch.record_output(&baseline_metrics);
                }

                // Lazily pull the first SA batch from each non-exhausted input so `top_score`
                // is known before computing upper bounds.
                if !state.initialized {
                    state.initialize_sa_cursors().await?;
                }

                // Drain and emit any candidate batches whose rank beats the stopping threshold.
                while state.has_ready_candidates() {
                    if let Some(batch) = state.drain_ready_batch()? {
                        yield batch.record_output(&baseline_metrics);
                        continue 'sa_loop;
                    }
                }

                // If either input is completely empty and cannot produce rows via SA or RA,
                // no join match is possible.
                if state.has_empty_exhausted_input() {
                    break 'sa_loop;
                }

                // Rule 1: Consume buffered in-memory SA rows (pausing a side once it accumulates
                // `ra_batch_size` unresolved keys, or once `U_SA` already clears the top candidate)
                // until a candidate becomes ready or both sides need async I/O.
                while !state.has_ready_candidates()
                    && let Some(side) = state.next_buffered_sa_side()
                {
                    state.advance_one_row(side);
                }
                if state.has_ready_candidates() {
                    continue 'sa_loop;
                }

                // Rule 2: Compare upper bounds (U_RA_L, U_RA_R, U_SA) to either execute a batched
                // RA lookup or pull the next SA batch from a child stream.
                match state.next_io_step() {
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

                while let Some((target_side, lookup)) = state.take_parent_lookup_request() {
                    let batch = state.execute_parent_lookup(target_side, lookup).await?;
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

/// Entry stored in $H_L$ or $H_R$ for a previously visited input row.
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

/// A joined candidate pair waiting in the max-heap until its score clears threshold $T$.
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

/// Returned when a [`RankedInputCursor`] advances by one row.
struct CursorRow {
    row_ref: BufferedRowRef,
    score: f64,
    key: Option<OwnedRow>,
}

/// Wraps one child stream of [`RankJoinExec`], retaining pulled batches so matched rows
/// can be assembled by `(BatchIndex, RowIndex)` when their candidate clears threshold $T$,
/// and coordinating both Sorted Access (`SA`) window pulls and Random Access (`RA`) point lookups.
struct RankedInputCursor {
    stream: SendableRecordBatchStream,
    score_expr: Arc<dyn PhysicalExpr>,
    key_exprs: Vec<Arc<dyn PhysicalExpr>>,
    key_converter: Arc<RowConverter>,
    null_equality: NullEquality,
    lookup_filter: Arc<dyn PhysicalExpr>,
    ra_capable: bool,
    batches: Vec<RecordBatch>,
    /// Per-row `f64` scores parallel to `batches`, indexed by `[batch_idx][row_idx]`.
    batch_scores: Vec<Vec<f64>>,
    sa_batch_idx: Option<usize>,
    current_batch_scores: Vec<f64>,
    current_batch_keys: Vec<Option<OwnedRow>>,
    current_row_idx: usize,
    top_score: Option<f64>,
    bottom_score: Option<f64>,
    sa_exhausted: bool,
    /// Total number of `SA` rows advanced from this cursor so far.
    sa_rows_advanced: usize,
    /// Distinct keys from `SA` batches in rank order `(key, max_sa_score)` for tracking the
    /// highest-scoring unresolved key ($s^{\text{unres}}$) for Tier-1 (`score > 0.0`) and
    /// Tier-2 (`score >= 0.0`) `RA` probes on the opposite side.
    sa_keys_in_order: Vec<(OwnedRow, f64)>,
    sa_queued_keys: HashSet<OwnedRow>,
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
}

impl RankedInputCursor {
    #[expect(clippy::too_many_arguments)]
    fn new(
        stream: SendableRecordBatchStream,
        score_expr: Arc<dyn PhysicalExpr>,
        key_exprs: Vec<Arc<dyn PhysicalExpr>>,
        key_converter: Arc<RowConverter>,
        null_equality: NullEquality,
        lookup_filter: Arc<dyn PhysicalExpr>,
        ra_capable: bool,
        start_sa_exhausted: bool,
    ) -> Self {
        Self {
            stream,
            score_expr,
            key_exprs,
            key_converter,
            null_equality,
            lookup_filter,
            ra_capable,
            batches: Vec::new(),
            batch_scores: Vec::new(),
            sa_batch_idx: None,
            current_batch_scores: Vec::new(),
            current_batch_keys: Vec::new(),
            current_row_idx: 0,
            top_score: start_sa_exhausted.then_some(0.0),
            bottom_score: start_sa_exhausted.then_some(0.0),
            sa_exhausted: start_sa_exhausted,
            sa_rows_advanced: 0,
            sa_keys_in_order: Vec::new(),
            sa_queued_keys: HashSet::default(),
            next_unresolved_idx: 0,
            next_zero_unresolved_idx: 0,
            ra_batch_size: RA_INITIAL_BATCH_SIZE,
            ra_positive_probed_keys: HashSet::default(),
            ra_probed_keys: HashSet::default(),
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
            let retry_scale = (crate::gucs::topk_retry_scale_factor() as usize).max(2);
            let max_chunk =
                (crate::gucs::max_topk_chunk_size() as usize).max(RA_INITIAL_BATCH_SIZE);
            self.ra_batch_size = (self.ra_batch_size * retry_scale).min(max_chunk);
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
        if out.len() >= max_keys {
            let retry_scale = (crate::gucs::topk_retry_scale_factor() as usize).max(2);
            let max_chunk =
                (crate::gucs::max_topk_chunk_size() as usize).max(RA_INITIAL_BATCH_SIZE);
            self.ra_batch_size = (self.ra_batch_size * retry_scale).min(max_chunk);
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
        let score_arr = self.score_expr.evaluate(batch)?.into_array(num_rows)?;
        let score_f64_arr = if score_arr.data_type() == &DataType::Float64 {
            score_arr
        } else {
            cast(&score_arr, &DataType::Float64)?
        };
        let f64_col = score_f64_arr
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| {
                DataFusionError::Internal("RankJoinStream: score cast to Float64 failed".into())
            })?;
        let scores: Vec<f64> = (0..num_rows)
            .map(|i| {
                if f64_col.is_null(i) {
                    f64::NEG_INFINITY
                } else {
                    let v = f64_col.value(i);
                    if v.is_nan() { f64::NEG_INFINITY } else { v }
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

    /// Consumes and returns the next buffered `SA` row. Caller must ensure `has_buffered_rows()`.
    fn next_buffered_row(
        &mut self,
        record_for_ra: bool,
        opposite_probed: &HashSet<OwnedRow>,
    ) -> CursorRow {
        let batch_idx = self.sa_batch_idx.expect("SA batch must be present");
        let row_idx = self.current_row_idx;
        let score = self.current_batch_scores[row_idx];
        let key = self.current_batch_keys[row_idx].take();
        self.current_row_idx += 1;
        self.sa_rows_advanced += 1;
        self.bottom_score = Some(score);
        if record_for_ra
            && let Some(ref k) = key
            && !opposite_probed.contains(k)
            && self.sa_queued_keys.insert(k.clone())
        {
            self.sa_keys_in_order.push((k.clone(), score));
        }
        CursorRow {
            row_ref: BufferedRowRef {
                batch: BatchIndex(batch_idx),
                row: RowIndex(row_idx),
            },
            score,
            key,
        }
    }
}

/// Which input side [`RankJoinStream`] chooses to advance or probe next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdvanceSide {
    Left,
    Right,
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

/// Streaming state machine for [`RankJoinExec`].
struct RankJoinStream {
    schema: SchemaRef,
    left_num_cols: usize,
    right_num_cols: usize,
    filter: Option<JoinFilter>,
    projection: Option<Vec<usize>>,
    output_ordering: LexOrdering,
    combiner: ScoreCombiner,
    parent_lookup_filters: Vec<(JoinSide, Arc<dyn PhysicalExpr>)>,
    left_cursor: RankedInputCursor,
    right_cursor: RankedInputCursor,
    left_hash_table: HashMap<OwnedRow, Vec<HashTableEntry>>,
    right_hash_table: HashMap<OwnedRow, Vec<HashTableEntry>>,
    candidate_heap: BinaryHeap<CandidateMatch>,
    emitted_pairs: HashSet<(BufferedRowRef, BufferedRowRef)>,
    next_sequence: u64,
    initialized: bool,
    batch_size: usize,
}

impl RankJoinStream {
    /// Normalizes a combined score into a max-heap rank value (`score` for `DESC`, `-score` for `ASC`).
    fn to_rank_value(&self, combined_score: f64) -> f64 {
        if self.output_ordering[0].options.descending {
            combined_score
        } else {
            -combined_score
        }
    }

    /// Computes detailed upper bounds on the `rank_value` of any unseen pair:
    /// - `u_ra_l_pos`: `Left` unprobed row + `Right` unseen positive-score row (`score > 0.0`),
    /// - `u_ra_l_zero`: `Left` zero-unprobed row + `Right` unseen `0.0`-score row,
    /// - `u_ra_r_pos`: `Left` unseen positive-score row (`score > 0.0`) + `Right` unprobed row,
    /// - `u_ra_r_zero`: `Left` unseen `0.0`-score row + `Right` zero-unprobed row,
    /// - `u_sa`: `Left` unseen row + `Right` unseen row.
    fn compute_detailed_bounds(&self) -> DetailedBounds {
        let left_bound = if !self.left_cursor.sa_fully_consumed() {
            self.left_cursor.bottom_score
        } else if self.left_cursor.ra_capable {
            Some(0.0)
        } else {
            None
        };

        let right_bound = if !self.right_cursor.sa_fully_consumed() {
            self.right_cursor.bottom_score
        } else if self.right_cursor.ra_capable {
            Some(0.0)
        } else {
            None
        };

        let (u_ra_l_pos, u_ra_l_zero) =
            if self.right_cursor.sa_fully_consumed() && !self.right_cursor.ra_capable {
                (f64::NEG_INFINITY, f64::NEG_INFINITY)
            } else if !self.right_cursor.ra_capable {
                let u = match (self.left_cursor.top_score, right_bound) {
                    (Some(l), Some(r)) => {
                        self.to_rank_value(self.combiner.eval(InputScorePair { left: l, right: r }))
                    }
                    _ => f64::NEG_INFINITY,
                };
                (u, f64::NEG_INFINITY)
            } else {
                let s_l_pos = self.left_cursor.top_unresolved_score(
                    &self.right_cursor.ra_positive_probed_keys,
                    &self.right_cursor.ra_probed_keys,
                );
                let s_l_zero = self
                    .left_cursor
                    .top_zero_unresolved_score(&self.right_cursor.ra_probed_keys);
                let u_pos = match (s_l_pos, right_bound) {
                    (Some(l), Some(r)) if r > 0.0 => {
                        self.to_rank_value(self.combiner.eval(InputScorePair { left: l, right: r }))
                    }
                    _ => f64::NEG_INFINITY,
                };
                let u_zero = match (s_l_zero, right_bound) {
                    (Some(l), Some(_)) => self.to_rank_value(self.combiner.eval(InputScorePair {
                        left: l,
                        right: 0.0,
                    })),
                    _ => f64::NEG_INFINITY,
                };
                (u_pos, u_zero)
            };

        let (u_ra_r_pos, u_ra_r_zero) =
            if self.left_cursor.sa_fully_consumed() && !self.left_cursor.ra_capable {
                (f64::NEG_INFINITY, f64::NEG_INFINITY)
            } else if !self.left_cursor.ra_capable {
                let u = match (left_bound, self.right_cursor.top_score) {
                    (Some(l), Some(r)) => {
                        self.to_rank_value(self.combiner.eval(InputScorePair { left: l, right: r }))
                    }
                    _ => f64::NEG_INFINITY,
                };
                (u, f64::NEG_INFINITY)
            } else {
                let s_r_pos = self.right_cursor.top_unresolved_score(
                    &self.left_cursor.ra_positive_probed_keys,
                    &self.left_cursor.ra_probed_keys,
                );
                let s_r_zero = self
                    .right_cursor
                    .top_zero_unresolved_score(&self.left_cursor.ra_probed_keys);
                let u_pos = match (left_bound, s_r_pos) {
                    (Some(l), Some(r)) if l > 0.0 => {
                        self.to_rank_value(self.combiner.eval(InputScorePair { left: l, right: r }))
                    }
                    _ => f64::NEG_INFINITY,
                };
                let u_zero = match (left_bound, s_r_zero) {
                    (Some(_), Some(r)) => self.to_rank_value(self.combiner.eval(InputScorePair {
                        left: 0.0,
                        right: r,
                    })),
                    _ => f64::NEG_INFINITY,
                };
                (u_pos, u_zero)
            };

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

    /// Computes $(U_{\text{RA}, L}, U_{\text{RA}, R}, U_{\text{SA}})$, the upper bounds on the
    /// `rank_value` of any unseen pair from:
    /// 1. `Left` seen row + `Right` unseen row ($U_{\text{RA}, L}$),
    /// 2. `Left` unseen row + `Right` seen row ($U_{\text{RA}, R}$),
    /// 3. `Left` unseen row + `Right` unseen row ($U_{\text{SA}}$).
    fn compute_bounds(&self) -> (f64, f64, f64) {
        let b = self.compute_detailed_bounds();
        (
            b.u_ra_l_pos.max(b.u_ra_l_zero),
            b.u_ra_r_pos.max(b.u_ra_r_zero),
            b.u_sa,
        )
    }

    /// Global stopping threshold $T = \max(U_{\text{RA}, L}, U_{\text{RA}, R}, U_{\text{SA}})$.
    fn stopping_threshold(&self) -> f64 {
        let (u_ra_l, u_ra_r, u_sa) = self.compute_bounds();
        u_ra_l.max(u_ra_r).max(u_sa)
    }

    fn has_tie_breakers(&self) -> bool {
        self.output_ordering.len() > 1
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

    /// Advances the specified side by one buffered `SA` row, inserting into its hash table
    /// and probing the opposite hash table.
    fn advance_one_row(&mut self, side: AdvanceSide) {
        match side {
            AdvanceSide::Left => {
                let row = self.left_cursor.next_buffered_row(
                    self.right_cursor.ra_capable,
                    &self.right_cursor.ra_probed_keys,
                );
                let Some(key) = row.key else {
                    return;
                };
                if let Some(matches) = self.right_hash_table.get(&key) {
                    for right_entry in matches {
                        let combined = self.combiner.eval(InputScorePair {
                            left: row.score,
                            right: right_entry.score,
                        });
                        let rank_value = self.to_rank_value(combined);
                        let seq = self.next_sequence;
                        self.next_sequence += 1;
                        self.candidate_heap.push(CandidateMatch {
                            rank_value,
                            sequence: seq,
                            left_row: row.row_ref,
                            right_row: right_entry.row_ref,
                        });
                    }
                }
                self.left_hash_table
                    .entry(key)
                    .or_default()
                    .push(HashTableEntry {
                        row_ref: row.row_ref,
                        score: row.score,
                        from_sa: true,
                    });
            }
            AdvanceSide::Right => {
                let row = self.right_cursor.next_buffered_row(
                    self.left_cursor.ra_capable,
                    &self.left_cursor.ra_probed_keys,
                );
                let Some(key) = row.key else {
                    return;
                };
                if let Some(matches) = self.left_hash_table.get(&key) {
                    for left_entry in matches {
                        let combined = self.combiner.eval(InputScorePair {
                            left: left_entry.score,
                            right: row.score,
                        });
                        let rank_value = self.to_rank_value(combined);
                        let seq = self.next_sequence;
                        self.next_sequence += 1;
                        self.candidate_heap.push(CandidateMatch {
                            rank_value,
                            sequence: seq,
                            left_row: left_entry.row_ref,
                            right_row: row.row_ref,
                        });
                    }
                }
                self.right_hash_table
                    .entry(key)
                    .or_default()
                    .push(HashTableEntry {
                        row_ref: row.row_ref,
                        score: row.score,
                        from_sa: true,
                    });
            }
        }
    }

    /// Ingests a `RecordBatch` returned from a Random Access (`RA`) point lookup on `side`
    /// without modifying `side`'s `SA` cursor state or `bottom_score`.
    ///
    /// Only pairs `RA`-fetched rows with opposite-side hash table entries that arrived via
    /// `SA` (`entry.from_sa == true`), preventing `(RA_only, RA_only)` pairs that satisfy
    /// neither side's disjunctive search predicate when join keys are non-unique.
    fn ingest_ra_batch(&mut self, side: AdvanceSide, batch: RecordBatch) -> Result<()> {
        if batch.num_rows() == 0 {
            return Ok(());
        }
        match side {
            AdvanceSide::Left => {
                let (scores, keys) = self.left_cursor.evaluate_batch(&batch)?;
                let batch_idx = self.left_cursor.batches.len();
                self.left_cursor.batches.push(batch);
                self.left_cursor.batch_scores.push(scores.clone());
                for (row_idx, (score, key_opt)) in scores.into_iter().zip(keys).enumerate() {
                    let Some(key) = key_opt else {
                        continue;
                    };
                    let row_ref = BufferedRowRef {
                        batch: BatchIndex(batch_idx),
                        row: RowIndex(row_idx),
                    };
                    if let Some(matches) = self.right_hash_table.get(&key) {
                        for right_entry in matches.iter().filter(|e| e.from_sa) {
                            let combined = self.combiner.eval(InputScorePair {
                                left: score,
                                right: right_entry.score,
                            });
                            let rank_value = self.to_rank_value(combined);
                            let seq = self.next_sequence;
                            self.next_sequence += 1;
                            self.candidate_heap.push(CandidateMatch {
                                rank_value,
                                sequence: seq,
                                left_row: row_ref,
                                right_row: right_entry.row_ref,
                            });
                        }
                    }
                    self.left_hash_table
                        .entry(key)
                        .or_default()
                        .push(HashTableEntry {
                            row_ref,
                            score,
                            from_sa: false,
                        });
                }
            }
            AdvanceSide::Right => {
                let (scores, keys) = self.right_cursor.evaluate_batch(&batch)?;
                let batch_idx = self.right_cursor.batches.len();
                self.right_cursor.batches.push(batch);
                self.right_cursor.batch_scores.push(scores.clone());
                for (row_idx, (score, key_opt)) in scores.into_iter().zip(keys).enumerate() {
                    let Some(key) = key_opt else {
                        continue;
                    };
                    let row_ref = BufferedRowRef {
                        batch: BatchIndex(batch_idx),
                        row: RowIndex(row_idx),
                    };
                    if let Some(matches) = self.left_hash_table.get(&key) {
                        for left_entry in matches.iter().filter(|e| e.from_sa) {
                            let combined = self.combiner.eval(InputScorePair {
                                left: left_entry.score,
                                right: score,
                            });
                            let rank_value = self.to_rank_value(combined);
                            let seq = self.next_sequence;
                            self.next_sequence += 1;
                            self.candidate_heap.push(CandidateMatch {
                                rank_value,
                                sequence: seq,
                                left_row: left_entry.row_ref,
                                right_row: row_ref,
                            });
                        }
                    }
                    self.right_hash_table
                        .entry(key)
                        .or_default()
                        .push(HashTableEntry {
                            row_ref,
                            score,
                            from_sa: false,
                        });
                }
            }
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
        let cursor = match target_side {
            AdvanceSide::Left => &self.left_cursor,
            AdvanceSide::Right => &self.right_cursor,
        };
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
    fn take_parent_lookup_request(&self) -> Option<(AdvanceSide, RankLookupPhysicalExpr)> {
        for (side, pf) in &self.parent_lookup_filters {
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
                // filter here; otherwise the target leaf scan will consume and reset it.
                let target_ra_capable = match target_side {
                    AdvanceSide::Left => self.left_cursor.ra_capable,
                    AdvanceSide::Right => self.right_cursor.ra_capable,
                };
                if !target_ra_capable {
                    let _ = update_lookup_filter(pf, lit(true));
                }
                return Some((target_side, lookup_cloned));
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
        let (key_exprs, lookup_filter) = match side {
            AdvanceSide::Left => (
                self.left_cursor.key_exprs.clone(),
                Arc::clone(&self.left_cursor.lookup_filter),
            ),
            AdvanceSide::Right => (
                self.right_cursor.key_exprs.clone(),
                Arc::clone(&self.right_cursor.lookup_filter),
            ),
        };
        let lookup_expr = Arc::new(
            RankLookupPhysicalExpr::new(key_exprs, scalars).with_score_threshold(score_threshold),
        );
        update_lookup_filter(&lookup_filter, lookup_expr)?;

        let next_res = match side {
            AdvanceSide::Left => self.left_cursor.stream.next().await,
            AdvanceSide::Right => self.right_cursor.stream.next().await,
        };
        let _ = update_lookup_filter(&lookup_filter, lit(true));

        match next_res {
            Some(Err(e)) => Err(e),
            None => {
                let target_cursor = match side {
                    AdvanceSide::Left => &mut self.left_cursor,
                    AdvanceSide::Right => &mut self.right_cursor,
                };
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
        target_side: AdvanceSide,
        lookup: RankLookupPhysicalExpr,
    ) -> Result<RecordBatch> {
        let parent_thresh = lookup.score_threshold();
        let target_thresh = parent_thresh.and_then(|pt| {
            if !self.initialized {
                return None;
            }
            match target_side {
                AdvanceSide::Left => {
                    let max_right = self.right_cursor.top_score.unwrap_or(0.0);
                    self.combiner.left_threshold_for(pt, max_right)
                }
                AdvanceSide::Right => {
                    let max_left = self.left_cursor.top_score.unwrap_or(0.0);
                    self.combiner.right_threshold_for(pt, max_left)
                }
            }
        });

        let target_ra_capable = match target_side {
            AdvanceSide::Left => self.left_cursor.ra_capable,
            AdvanceSide::Right => self.right_cursor.ra_capable,
        };
        if target_ra_capable {
            if target_thresh != parent_thresh {
                let target_lookup: Arc<dyn PhysicalExpr> =
                    Arc::new(lookup.clone().with_score_threshold(target_thresh));
                let target_join_side = match target_side {
                    AdvanceSide::Left => JoinSide::Left,
                    AdvanceSide::Right => JoinSide::Right,
                };
                for (side, pf) in &self.parent_lookup_filters {
                    if *side == target_join_side {
                        let _ = update_lookup_filter(pf, Arc::clone(&target_lookup));
                    }
                }
            }
            let next_res = match target_side {
                AdvanceSide::Left => self.left_cursor.stream.next().await,
                AdvanceSide::Right => self.right_cursor.stream.next().await,
            };
            for (_, pf) in &self.parent_lookup_filters {
                let _ = update_lookup_filter(pf, lit(true));
            }
            match next_res {
                Some(Err(e)) => return Err(e),
                None => {
                    let target_cursor = match target_side {
                        AdvanceSide::Left => &mut self.left_cursor,
                        AdvanceSide::Right => &mut self.right_cursor,
                    };
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

        let matching_target_rows =
            self.find_matching_buffered_rows(target_side, &lookup, target_thresh)?;
        let opposite_side = match target_side {
            AdvanceSide::Left => AdvanceSide::Right,
            AdvanceSide::Right => AdvanceSide::Left,
        };
        let opposite_ra_capable = match opposite_side {
            AdvanceSide::Left => self.left_cursor.ra_capable,
            AdvanceSide::Right => self.right_cursor.ra_capable,
        };

        if opposite_ra_capable && !matching_target_rows.is_empty() {
            let mut unfiltered_keys = Vec::new();
            let mut threshold_keys = Vec::new();
            let mut threshold_seen: HashSet<OwnedRow> = HashSet::default();
            let mut min_opp_thresh: Option<f32> = None;

            for (row_ref, key) in &matching_target_rows {
                let target_score = match target_side {
                    AdvanceSide::Left => self.left_cursor.row_score(*row_ref),
                    AdvanceSide::Right => self.right_cursor.row_score(*row_ref),
                };
                let row_opp_thresh = parent_thresh.and_then(|pt| match target_side {
                    AdvanceSide::Left => self.combiner.right_threshold_for(pt, target_score),
                    AdvanceSide::Right => self.combiner.left_threshold_for(pt, target_score),
                });
                match row_opp_thresh {
                    None => {
                        let opposite_cursor = match opposite_side {
                            AdvanceSide::Left => &mut self.left_cursor,
                            AdvanceSide::Right => &mut self.right_cursor,
                        };
                        opposite_cursor.ra_positive_probed_keys.insert(key.clone());
                        if opposite_cursor.ra_probed_keys.insert(key.clone()) {
                            unfiltered_keys.push(key.clone());
                        }
                    }
                    Some(t) => {
                        let already_probed = {
                            let opposite_cursor = match opposite_side {
                                AdvanceSide::Left => &self.left_cursor,
                                AdvanceSide::Right => &self.right_cursor,
                            };
                            opposite_cursor.ra_probed_keys.contains(key)
                                || (t >= 0.0
                                    && opposite_cursor.ra_positive_probed_keys.contains(key))
                        };
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
                    let opposite_cursor = match opposite_side {
                        AdvanceSide::Left => &mut self.left_cursor,
                        AdvanceSide::Right => &mut self.right_cursor,
                    };
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
        let mut candidates = Vec::new();
        match target_side {
            AdvanceSide::Left => {
                let mut unadvanced_right: HashMap<&OwnedRow, Vec<BufferedRowRef>> =
                    HashMap::default();
                if self.right_cursor.has_buffered_rows()
                    && let Some(batch_idx) = self.right_cursor.sa_batch_idx
                {
                    for row_idx in self.right_cursor.current_row_idx
                        ..self.right_cursor.current_batch_keys.len()
                    {
                        if let Some(ref k) = self.right_cursor.current_batch_keys[row_idx] {
                            unadvanced_right.entry(k).or_default().push(BufferedRowRef {
                                batch: BatchIndex(batch_idx),
                                row: RowIndex(row_idx),
                            });
                        }
                    }
                }
                for (target_row_ref, key) in matching_target_rows {
                    let left_score = self.left_cursor.row_score(*target_row_ref);
                    if let Some(matches) = self.right_hash_table.get(key) {
                        for right_entry in matches {
                            if let Some(pt) = parent_thresh {
                                let combined = self.combiner.eval(InputScorePair {
                                    left: left_score,
                                    right: right_entry.score,
                                });
                                if (combined as f32) <= pt {
                                    continue;
                                }
                            }
                            if self
                                .emitted_pairs
                                .insert((*target_row_ref, right_entry.row_ref))
                            {
                                candidates.push(CandidateMatch {
                                    rank_value: 0.0,
                                    sequence: 0,
                                    left_row: *target_row_ref,
                                    right_row: right_entry.row_ref,
                                });
                            }
                        }
                    }
                    if let Some(unadv_matches) = unadvanced_right.get(key) {
                        for &right_row_ref in unadv_matches {
                            if let Some(pt) = parent_thresh {
                                let right_score = self.right_cursor.row_score(right_row_ref);
                                let combined = self.combiner.eval(InputScorePair {
                                    left: left_score,
                                    right: right_score,
                                });
                                if (combined as f32) <= pt {
                                    continue;
                                }
                            }
                            if self.emitted_pairs.insert((*target_row_ref, right_row_ref)) {
                                candidates.push(CandidateMatch {
                                    rank_value: 0.0,
                                    sequence: 0,
                                    left_row: *target_row_ref,
                                    right_row: right_row_ref,
                                });
                            }
                        }
                    }
                }
            }
            AdvanceSide::Right => {
                let mut unadvanced_left: HashMap<&OwnedRow, Vec<BufferedRowRef>> =
                    HashMap::default();
                if self.left_cursor.has_buffered_rows()
                    && let Some(batch_idx) = self.left_cursor.sa_batch_idx
                {
                    for row_idx in
                        self.left_cursor.current_row_idx..self.left_cursor.current_batch_keys.len()
                    {
                        if let Some(ref k) = self.left_cursor.current_batch_keys[row_idx] {
                            unadvanced_left.entry(k).or_default().push(BufferedRowRef {
                                batch: BatchIndex(batch_idx),
                                row: RowIndex(row_idx),
                            });
                        }
                    }
                }
                for (target_row_ref, key) in matching_target_rows {
                    let right_score = self.right_cursor.row_score(*target_row_ref);
                    if let Some(matches) = self.left_hash_table.get(key) {
                        for left_entry in matches {
                            if let Some(pt) = parent_thresh {
                                let combined = self.combiner.eval(InputScorePair {
                                    left: left_entry.score,
                                    right: right_score,
                                });
                                if (combined as f32) <= pt {
                                    continue;
                                }
                            }
                            if self
                                .emitted_pairs
                                .insert((left_entry.row_ref, *target_row_ref))
                            {
                                candidates.push(CandidateMatch {
                                    rank_value: 0.0,
                                    sequence: 0,
                                    left_row: left_entry.row_ref,
                                    right_row: *target_row_ref,
                                });
                            }
                        }
                    }
                    if let Some(unadv_matches) = unadvanced_left.get(key) {
                        for &left_row_ref in unadv_matches {
                            if let Some(pt) = parent_thresh {
                                let left_score = self.left_cursor.row_score(left_row_ref);
                                let combined = self.combiner.eval(InputScorePair {
                                    left: left_score,
                                    right: right_score,
                                });
                                if (combined as f32) <= pt {
                                    continue;
                                }
                            }
                            if self.emitted_pairs.insert((left_row_ref, *target_row_ref)) {
                                candidates.push(CandidateMatch {
                                    rank_value: 0.0,
                                    sequence: 0,
                                    left_row: left_row_ref,
                                    right_row: *target_row_ref,
                                });
                            }
                        }
                    }
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
            self.left_cursor.ensure_buffered().await?;
        }
        if !self.right_cursor.sa_exhausted {
            self.right_cursor.ensure_buffered().await?;
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

    /// Pulls from `side`'s `SA` stream until at least one buffered row is available or `SA` is exhausted.
    async fn ensure_side_buffered(&mut self, side: AdvanceSide) -> Result<bool> {
        match side {
            AdvanceSide::Left => self.left_cursor.ensure_buffered().await,
            AdvanceSide::Right => self.right_cursor.ensure_buffered().await,
        }
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
        (self.left_cursor.sa_fully_consumed()
            && !self.left_cursor.ra_capable
            && self.left_hash_table.is_empty())
            || (self.right_cursor.sa_fully_consumed()
                && !self.right_cursor.ra_capable
                && self.right_hash_table.is_empty())
    }

    /// Chooses between `Left` and `Right` when both sides have available `SA` rows.
    ///
    /// When both sides support `RA` point lookups, all seen `SA` rows are resolved via `RA` and the
    /// remaining `SA` frontier is $U_{\text{SA}} = g(\underline{s}_L, \underline{s}_R)$, so advancing
    /// both cursors at balanced `SA` row depth ($N_L = N_R$) is 2-competitive with the omniscient
    /// offline optimal $(N_L^*, N_R^*)$ regardless of score skew. When `RA` is unavailable on either
    /// side, falls back to the corner-bound priority $P_L = g(\underline{s}_L, \bar{s}_R)$ vs.
    /// $P_R = g(\bar{s}_L, \underline{s}_R)$.
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

    /// Rule 2: Compares $(U_{\text{RA}, L}, U_{\text{RA}, R}, U_{\text{SA}})$ to choose the next
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
            if b.u_ra_l_pos >= b.u_ra_l_zero && b.u_ra_l_pos > f64::NEG_INFINITY {
                let keys = self.left_cursor.pop_positive_unresolved_keys(
                    &mut self.right_cursor.ra_positive_probed_keys,
                    &self.right_cursor.ra_probed_keys,
                );
                return NextIoStep::RaLookup {
                    target_side: AdvanceSide::Right,
                    keys,
                    score_threshold: Some(0.0),
                };
            } else {
                let mut cutoff = b.u_ra_l_pos.max(eff_ra_r).max(eff_sa);
                if let Some(top) = self.candidate_heap.peek() {
                    cutoff = cutoff.max(top.rank_value);
                }
                let combiner = self.combiner.clone();
                let descending = self.output_ordering[0].options.descending;
                let keys = self.left_cursor.pop_all_unresolved_keys(
                    &mut self.right_cursor.ra_positive_probed_keys,
                    &mut self.right_cursor.ra_probed_keys,
                    |left_score| {
                        let combined = combiner.eval(InputScorePair {
                            left: left_score,
                            right: 0.0,
                        });
                        let rank_val = if descending { combined } else { -combined };
                        rank_val >= cutoff
                    },
                );
                return NextIoStep::RaLookup {
                    target_side: AdvanceSide::Right,
                    keys,
                    score_threshold: None,
                };
            }
        }

        if eff_ra_r > f64::NEG_INFINITY && eff_ra_r >= eff_sa {
            if b.u_ra_r_pos >= b.u_ra_r_zero && b.u_ra_r_pos > f64::NEG_INFINITY {
                let keys = self.right_cursor.pop_positive_unresolved_keys(
                    &mut self.left_cursor.ra_positive_probed_keys,
                    &self.left_cursor.ra_probed_keys,
                );
                return NextIoStep::RaLookup {
                    target_side: AdvanceSide::Left,
                    keys,
                    score_threshold: Some(0.0),
                };
            } else {
                let mut cutoff = b.u_ra_r_pos.max(eff_ra_l).max(eff_sa);
                if let Some(top) = self.candidate_heap.peek() {
                    cutoff = cutoff.max(top.rank_value);
                }
                let combiner = self.combiner.clone();
                let descending = self.output_ordering[0].options.descending;
                let keys = self.right_cursor.pop_all_unresolved_keys(
                    &mut self.left_cursor.ra_positive_probed_keys,
                    &mut self.left_cursor.ra_probed_keys,
                    |right_score| {
                        let combined = combiner.eval(InputScorePair {
                            left: 0.0,
                            right: right_score,
                        });
                        let rank_val = if descending { combined } else { -combined };
                        rank_val >= cutoff
                    },
                );
                return NextIoStep::RaLookup {
                    target_side: AdvanceSide::Left,
                    keys,
                    score_threshold: None,
                };
            }
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
    use arrow_array::{Float32Array, Int32Array};
    use arrow_schema::{Field, Schema, SortOptions};
    use datafusion::logical_expr::Operator;
    use datafusion::physical_expr::expressions::{BinaryExpr, Column};
    use datafusion::physical_plan::common::collect;

    /// Stream wrapper that counts how many batches were pulled from a child plan.
    struct CountingPlan {
        batches: Vec<RecordBatch>,
        schema: SchemaRef,
        ordering: LexOrdering,
        pulled_batches: Arc<std::sync::atomic::AtomicUsize>,
        cache: Arc<PlanProperties>,
    }

    impl fmt::Debug for CountingPlan {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("CountingPlan").finish()
        }
    }

    impl CountingPlan {
        fn new(
            batches: Vec<RecordBatch>,
            schema: SchemaRef,
            ordering: LexOrdering,
            pulled_batches: Arc<std::sync::atomic::AtomicUsize>,
        ) -> Self {
            let mut eq = datafusion::physical_expr::EquivalenceProperties::new(Arc::clone(&schema));
            eq.add_ordering(ordering.clone());
            let cache = Arc::new(PlanProperties::new(
                eq,
                Partitioning::UnknownPartitioning(1),
                EmissionType::Incremental,
                Boundedness::Bounded,
            ));
            Self {
                batches,
                schema,
                ordering,
                pulled_batches,
                cache,
            }
        }
    }

    impl DisplayAs for CountingPlan {
        fn fmt_as(&self, _t: DisplayFormatType, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "CountingPlan")
        }
    }

    impl ExecutionPlan for CountingPlan {
        fn name(&self) -> &str {
            "CountingPlan"
        }

        fn properties(&self) -> &Arc<PlanProperties> {
            &self.cache
        }

        fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
            vec![]
        }

        fn apply_expressions(
            &self,
            _f: &mut dyn FnMut(&Arc<dyn PhysicalExpr>) -> Result<TreeNodeRecursion>,
        ) -> Result<TreeNodeRecursion> {
            let _ = &self.ordering;
            Ok(TreeNodeRecursion::Continue)
        }

        fn with_new_children(
            self: Arc<Self>,
            _children: Vec<Arc<dyn ExecutionPlan>>,
        ) -> Result<Arc<dyn ExecutionPlan>> {
            Ok(self)
        }

        fn execute(
            &self,
            _partition: usize,
            _context: Arc<TaskContext>,
        ) -> Result<SendableRecordBatchStream> {
            let batches = self.batches.clone();
            let counter = Arc::clone(&self.pulled_batches);
            let s = futures::stream::iter(batches.into_iter().map(move |b| {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(b)
            }));
            Ok(Box::pin(RecordBatchStreamAdapter::new(
                Arc::clone(&self.schema),
                s,
            )))
        }
    }

    fn make_batch(schema: &SchemaRef, ids: &[i32], keys: &[i32], scores: &[f32]) -> RecordBatch {
        RecordBatch::try_new(
            Arc::clone(schema),
            vec![
                Arc::new(Int32Array::from(ids.to_vec())),
                Arc::new(Int32Array::from(keys.to_vec())),
                Arc::new(Float32Array::from(scores.to_vec())),
            ],
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_rank_join_early_stopping_and_order() -> Result<()> {
        let left_schema = Arc::new(Schema::new(vec![
            Field::new("l_id", DataType::Int32, false),
            Field::new("l_key", DataType::Int32, false),
            Field::new("l_score", DataType::Float32, false),
        ]));
        let right_schema = Arc::new(Schema::new(vec![
            Field::new("r_id", DataType::Int32, false),
            Field::new("r_key", DataType::Int32, false),
            Field::new("r_score", DataType::Float32, false),
        ]));

        let left_sort = PhysicalSortExpr {
            expr: Arc::new(Column::new("l_score", 2)),
            options: SortOptions {
                descending: true,
                nulls_first: false,
            },
        };
        let right_sort = PhysicalSortExpr {
            expr: Arc::new(Column::new("r_score", 2)),
            options: SortOptions {
                descending: true,
                nulls_first: false,
            },
        };

        // Batch 1 on both sides contains a high-scoring match (key=1, 10.0 + 10.0 = 20.0)
        // and lower-scoring rows (5.0 on both sides) so threshold drops to 15.0 after Batch 1,
        // allowing `RankJoinExec` to emit the 20.0 match without ever pulling Batch 2 or Batch 3!
        let left_batches = vec![
            make_batch(&left_schema, &[1, 2], &[1, 99], &[10.0, 5.0]),
            make_batch(&left_schema, &[3, 4], &[2, 3], &[4.0, 3.0]),
            make_batch(&left_schema, &[5, 6], &[4, 5], &[2.0, 1.0]),
        ];
        let right_batches = vec![
            make_batch(&right_schema, &[10, 20], &[1, 98], &[10.0, 5.0]),
            make_batch(&right_schema, &[30, 40], &[2, 3], &[4.0, 3.0]),
            make_batch(&right_schema, &[50, 60], &[4, 5], &[2.0, 1.0]),
        ];

        let left_pulled = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let right_pulled = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let left_plan = Arc::new(CountingPlan::new(
            left_batches,
            Arc::clone(&left_schema),
            LexOrdering::new(vec![left_sort.clone()]).unwrap(),
            Arc::clone(&left_pulled),
        ));
        let right_plan = Arc::new(CountingPlan::new(
            right_batches,
            Arc::clone(&right_schema),
            LexOrdering::new(vec![right_sort.clone()]).unwrap(),
            Arc::clone(&right_pulled),
        ));

        let output_score_expr: Arc<dyn PhysicalExpr> = Arc::new(BinaryExpr::new(
            Arc::new(Column::new("l_score", 2)),
            Operator::Plus,
            Arc::new(Column::new("r_score", 5)),
        ));
        let output_sort = PhysicalSortExpr {
            expr: output_score_expr,
            options: SortOptions {
                descending: true,
                nulls_first: false,
            },
        };

        let rank_join = Arc::new(RankJoinExec::try_new(
            RankJoinInputs {
                left: left_plan,
                right: right_plan,
            },
            vec![(
                Arc::new(Column::new("l_key", 1)),
                Arc::new(Column::new("r_key", 1)),
            )],
            None,
            None,
            PartitionMode::Partitioned,
            NullEquality::NullEqualsNothing,
            RankJoinSortExprs {
                left: left_sort,
                right: right_sort,
                output: LexOrdering::new(vec![output_sort]).unwrap(),
            },
            ScoreCombiner::Add(
                Box::new(ScoreCombiner::LeftScore),
                Box::new(ScoreCombiner::RightScore),
            ),
        )?);

        let ctx = Arc::new(TaskContext::default());
        let mut stream = rank_join.execute(0, Arc::clone(&ctx))?;

        // Pull only the first batch (top-1 result).
        let first_batch = stream.next().await.transpose()?.unwrap();
        assert_eq!(first_batch.num_rows(), 1);
        let l_ids = first_batch
            .column(0)
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        let r_ids = first_batch
            .column(3)
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        assert_eq!(l_ids.value(0), 1);
        assert_eq!(r_ids.value(0), 10);

        // Verify early stopping: only 1 batch was pulled from each side!
        assert_eq!(left_pulled.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(right_pulled.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Now collect the remaining batches and verify all 5 matches arrive in descending score order.
        let mut all_l_ids = vec![l_ids.value(0)];
        while let Some(batch) = stream.next().await.transpose()? {
            let col = batch
                .column(0)
                .as_any()
                .downcast_ref::<Int32Array>()
                .unwrap();
            for i in 0..batch.num_rows() {
                all_l_ids.push(col.value(i));
            }
        }
        assert_eq!(all_l_ids, vec![1, 3, 4, 5, 6]);
        Ok(())
    }

    #[tokio::test]
    async fn test_rank_join_tie_breaker_and_filter() -> Result<()> {
        let left_schema = Arc::new(Schema::new(vec![
            Field::new("l_id", DataType::Int32, false),
            Field::new("l_key", DataType::Int32, false),
            Field::new("l_score", DataType::Float32, false),
        ]));
        let right_schema = Arc::new(Schema::new(vec![
            Field::new("r_id", DataType::Int32, false),
            Field::new("r_key", DataType::Int32, false),
            Field::new("r_score", DataType::Float32, false),
        ]));

        let left_sort = PhysicalSortExpr {
            expr: Arc::new(Column::new("l_score", 2)),
            options: SortOptions {
                descending: true,
                nulls_first: false,
            },
        };
        let right_sort = PhysicalSortExpr {
            expr: Arc::new(Column::new("r_score", 2)),
            options: SortOptions {
                descending: true,
                nulls_first: false,
            },
        };

        // Rows with identical combined score 10.0 + 10.0 = 20.0, but arrived in reverse `l_id` order.
        let left_batches = vec![make_batch(
            &left_schema,
            &[30, 10, 20],
            &[1, 1, 1],
            &[10.0, 10.0, 10.0],
        )];
        let right_batches = vec![make_batch(&right_schema, &[100], &[1], &[10.0])];

        let left_plan = Arc::new(CountingPlan::new(
            left_batches,
            Arc::clone(&left_schema),
            LexOrdering::new(vec![left_sort.clone()]).unwrap(),
            Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        ));
        let right_plan = Arc::new(CountingPlan::new(
            right_batches,
            Arc::clone(&right_schema),
            LexOrdering::new(vec![right_sort.clone()]).unwrap(),
            Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        ));

        let output_score_expr: Arc<dyn PhysicalExpr> = Arc::new(BinaryExpr::new(
            Arc::new(Column::new("l_score", 2)),
            Operator::Plus,
            Arc::new(Column::new("r_score", 5)),
        ));
        let output_ordering = LexOrdering::new(vec![
            PhysicalSortExpr {
                expr: output_score_expr,
                options: SortOptions {
                    descending: true,
                    nulls_first: false,
                },
            },
            PhysicalSortExpr {
                expr: Arc::new(Column::new("l_id", 0)),
                options: SortOptions {
                    descending: false,
                    nulls_first: false,
                },
            },
        ])
        .unwrap();

        // Residual JoinFilter: l_id != 20
        let filter_schema = Arc::new(Schema::new(vec![Field::new(
            "l_id",
            DataType::Int32,
            false,
        )]));
        let filter_expr: Arc<dyn PhysicalExpr> = Arc::new(BinaryExpr::new(
            Arc::new(Column::new("l_id", 0)),
            Operator::NotEq,
            Arc::new(datafusion::physical_expr::expressions::Literal::new(
                datafusion::common::ScalarValue::Int32(Some(20)),
            )),
        ));
        let join_filter = JoinFilter::new(
            filter_expr,
            vec![ColumnIndex {
                index: 0,
                side: JoinSide::Left,
            }],
            filter_schema,
        );

        let rank_join = Arc::new(RankJoinExec::try_new(
            RankJoinInputs {
                left: left_plan,
                right: right_plan,
            },
            vec![(
                Arc::new(Column::new("l_key", 1)),
                Arc::new(Column::new("r_key", 1)),
            )],
            Some(join_filter),
            None,
            PartitionMode::Partitioned,
            NullEquality::NullEqualsNothing,
            RankJoinSortExprs {
                left: left_sort,
                right: right_sort,
                output: output_ordering,
            },
            ScoreCombiner::Add(
                Box::new(ScoreCombiner::LeftScore),
                Box::new(ScoreCombiner::RightScore),
            ),
        )?);

        let ctx = Arc::new(TaskContext::default());
        let batches = collect(rank_join.execute(0, ctx)?).await?;
        assert_eq!(batches.len(), 1);
        let l_ids = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        // `l_id = 20` is filtered out by `JoinFilter`, and `[30, 10]` are sorted by `l_id ASC` -> `[10, 30]`.
        assert_eq!(l_ids.values(), &[10, 30]);
        Ok(())
    }

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
        let left_plan: Arc<dyn ExecutionPlan> = Arc::new(CountingPlan::new(
            vec![],
            Arc::clone(&left_schema),
            LexOrdering::new(vec![left_sort.clone()]).unwrap(),
            Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        ));
        let right_plan: Arc<dyn ExecutionPlan> = Arc::new(CountingPlan::new(
            vec![],
            Arc::clone(&right_schema),
            LexOrdering::new(vec![right_sort.clone()]).unwrap(),
            Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        ));
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
