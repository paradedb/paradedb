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

//! Pull-based symmetric hash rank join (`RankJoinExec`) for top-K scored joins.
//!
//! `RankJoinExec` consumes two score-ordered input streams (`left` and `right`),
//! maintains in-memory hash tables on both sides, tracks the upper bound $T$ on any
//! future unseen match using the top and current scores of both streams, and emits
//! joined rows in combined score order as soon as their score is $\ge T$ (or $> T$
//! when secondary tie-breaker sort expressions are present).

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::api::HashMap;
use arrow_array::{Array, ArrayRef, BooleanArray, Float64Array, RecordBatch, RecordBatchOptions};
use arrow_schema::{DataType, SchemaRef};
use datafusion::arrow::compute::{cast, interleave};
use datafusion::arrow::row::{OwnedRow, RowConverter, SortField};
use datafusion::common::tree_node::TreeNodeRecursion;
use datafusion::common::{
    DataFusionError, JoinSide, NullEquality, Result, internal_err, project_schema,
};
use datafusion::execution::{RecordBatchStream, SendableRecordBatchStream, TaskContext};
use datafusion::logical_expr::JoinType;
use datafusion::physical_expr::equivalence::{ProjectionMapping, join_equivalence_properties};
use datafusion::physical_expr::{
    Distribution, LexOrdering, OrderingRequirements, Partitioning, PhysicalExpr, PhysicalSortExpr,
};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::joins::PartitionMode;
use datafusion::physical_plan::joins::utils::{
    ColumnIndex, JoinFilter, JoinOn, JoinOnRef, adjust_right_output_partitioning,
    build_join_schema, check_join_is_valid,
};
use datafusion::physical_plan::metrics::{
    BaselineMetrics, ExecutionPlanMetricsSet, MetricsSet, RecordOutput,
};
use datafusion::physical_plan::sorts::sort::sort_batch;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, ExecutionPlanProperties,
    InputDistributionRequirements, PlanProperties,
};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

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

        Ok(Arc::new(Self::try_new(
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
        vec![
            LexOrdering::new(vec![self.sort_exprs.left.clone()]).map(OrderingRequirements::from),
            LexOrdering::new(vec![self.sort_exprs.right.clone()]).map(OrderingRequirements::from),
        ]
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
        Ok(Arc::new(Self::try_new(
            RankJoinInputs { left, right },
            self.on.clone(),
            self.filter.clone(),
            self.projection.clone(),
            self.mode,
            self.null_equality,
            self.sort_exprs.clone(),
            self.combiner.clone(),
        )?))
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

        let left_cursor = RankedInputCursor::new(
            left_stream,
            Arc::clone(&self.sort_exprs.left.expr),
            left_keys,
            Arc::clone(&key_converter),
            self.null_equality,
        );
        let right_cursor = RankedInputCursor::new(
            right_stream,
            Arc::clone(&self.sort_exprs.right.expr),
            right_keys,
            key_converter,
            self.null_equality,
        );

        let batch_size = context.session_config().batch_size().max(1);
        let baseline_metrics = BaselineMetrics::new(&self.metrics, partition);

        Ok(Box::pin(RankJoinStream {
            schema: self.schema(),
            left_num_cols: left_schema.fields().len(),
            right_num_cols: self.inputs.right.schema().fields().len(),
            filter: self.filter.clone(),
            projection: self.projection.clone(),
            output_ordering: self.sort_exprs.output.clone(),
            combiner: self.combiner.clone(),
            left_cursor,
            right_cursor,
            left_hash_table: HashMap::default(),
            right_hash_table: HashMap::default(),
            candidate_heap: BinaryHeap::new(),
            next_sequence: 0,
            initialized: false,
            done: false,
            batch_size,
            baseline_metrics,
        }))
    }

    fn metrics(&self) -> Option<MetricsSet> {
        Some(self.metrics.clone_inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BatchIndex(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RowIndex(usize);

/// Identifies a buffered row inside a [`RankedInputCursor`]'s retained batches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BufferedRowRef {
    batch: BatchIndex,
    row: RowIndex,
}

/// Entry stored in $H_L$ or $H_R$ for a previously visited input row.
#[derive(Debug, Clone, Copy)]
struct HashTableEntry {
    row_ref: BufferedRowRef,
    score: f64,
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
/// can be assembled by `(BatchIndex, RowIndex)` when their candidate clears threshold $T$.
struct RankedInputCursor {
    stream: SendableRecordBatchStream,
    score_expr: Arc<dyn PhysicalExpr>,
    key_exprs: Vec<Arc<dyn PhysicalExpr>>,
    key_converter: Arc<RowConverter>,
    null_equality: NullEquality,
    batches: Vec<RecordBatch>,
    current_batch_scores: Vec<f64>,
    current_batch_keys: Vec<Option<OwnedRow>>,
    current_row_idx: usize,
    top_score: Option<f64>,
    bottom_score: Option<f64>,
    exhausted: bool,
}

impl RankedInputCursor {
    fn new(
        stream: SendableRecordBatchStream,
        score_expr: Arc<dyn PhysicalExpr>,
        key_exprs: Vec<Arc<dyn PhysicalExpr>>,
        key_converter: Arc<RowConverter>,
        null_equality: NullEquality,
    ) -> Self {
        Self {
            stream,
            score_expr,
            key_exprs,
            key_converter,
            null_equality,
            batches: Vec::new(),
            current_batch_scores: Vec::new(),
            current_batch_keys: Vec::new(),
            current_row_idx: 0,
            top_score: None,
            bottom_score: None,
            exhausted: false,
        }
    }

    fn has_buffered_rows(&self) -> bool {
        self.current_row_idx < self.current_batch_scores.len()
    }

    /// Polls the underlying stream until either at least one buffered row is available
    /// or the stream is exhausted.
    fn poll_ensure_buffered(&mut self, cx: &mut Context<'_>) -> Poll<Result<bool>> {
        while !self.has_buffered_rows() {
            if self.exhausted {
                return Poll::Ready(Ok(false));
            }
            match self.stream.poll_next_unpin(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    self.exhausted = true;
                    return Poll::Ready(Ok(false));
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(e)),
                Poll::Ready(Some(Ok(batch))) => {
                    if batch.num_rows() == 0 {
                        continue;
                    }
                    self.ingest_batch(batch)?;
                }
            }
        }
        Poll::Ready(Ok(true))
    }

    fn ingest_batch(&mut self, batch: RecordBatch) -> Result<()> {
        let num_rows = batch.num_rows();
        let score_arr = self.score_expr.evaluate(&batch)?.into_array(num_rows)?;
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

        if self.top_score.is_none()
            && let Some(&first_score) = scores.first()
        {
            self.top_score = Some(first_score);
            self.bottom_score = Some(first_score);
        }

        let key_arrays: Vec<ArrayRef> = self
            .key_exprs
            .iter()
            .map(|expr| expr.evaluate(&batch)?.into_array(num_rows))
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

        self.batches.push(batch);
        self.current_batch_scores = scores;
        self.current_batch_keys = keys;
        self.current_row_idx = 0;
        Ok(())
    }

    /// Consumes and returns the next buffered row. Caller must ensure `has_buffered_rows()`.
    fn next_buffered_row(&mut self) -> CursorRow {
        let batch_idx = self.batches.len() - 1;
        let row_idx = self.current_row_idx;
        let score = self.current_batch_scores[row_idx];
        let key = self.current_batch_keys[row_idx].take();
        self.current_row_idx += 1;
        self.bottom_score = Some(score);
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

/// Which input side [`RankJoinStream`] chooses to advance next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdvanceSide {
    Left,
    Right,
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
    left_cursor: RankedInputCursor,
    right_cursor: RankedInputCursor,
    left_hash_table: HashMap<OwnedRow, Vec<HashTableEntry>>,
    right_hash_table: HashMap<OwnedRow, Vec<HashTableEntry>>,
    candidate_heap: BinaryHeap<CandidateMatch>,
    next_sequence: u64,
    initialized: bool,
    done: bool,
    batch_size: usize,
    baseline_metrics: BaselineMetrics,
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

    /// Computes $(U_L, U_R)$, the upper bounds on the `rank_value` of any unseen pair
    /// formed by advancing the left or right cursor respectively.
    fn upper_bounds(&self) -> (f64, f64) {
        let (Some(top_l), Some(top_r)) = (self.left_cursor.top_score, self.right_cursor.top_score)
        else {
            return (f64::NEG_INFINITY, f64::NEG_INFINITY);
        };

        let u_l = if self.left_cursor.exhausted && !self.left_cursor.has_buffered_rows() {
            f64::NEG_INFINITY
        } else if let Some(bot_l) = self.left_cursor.bottom_score {
            self.to_rank_value(self.combiner.eval(InputScorePair {
                left: bot_l,
                right: top_r,
            }))
        } else {
            f64::INFINITY
        };

        let u_r = if self.right_cursor.exhausted && !self.right_cursor.has_buffered_rows() {
            f64::NEG_INFINITY
        } else if let Some(bot_r) = self.right_cursor.bottom_score {
            self.to_rank_value(self.combiner.eval(InputScorePair {
                left: top_l,
                right: bot_r,
            }))
        } else {
            f64::INFINITY
        };

        (u_l, u_r)
    }

    /// Stopping threshold $T = \max(U_L, U_R)$.
    fn stopping_threshold(&self) -> f64 {
        let (u_l, u_r) = self.upper_bounds();
        u_l.max(u_r)
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

    /// Advances the specified side by one buffered row, inserting into its hash table
    /// and probing the opposite hash table.
    fn advance_one_row(&mut self, side: AdvanceSide) {
        match side {
            AdvanceSide::Left => {
                let row = self.left_cursor.next_buffered_row();
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
                    });
            }
            AdvanceSide::Right => {
                let row = self.right_cursor.next_buffered_row();
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
                    });
            }
        }
    }

    /// Pops ready candidates from `candidate_heap`, applies any residual `JoinFilter`,
    /// and materializes the output `RecordBatch`.
    fn drain_ready_batch(&mut self, threshold: f64) -> Result<Option<RecordBatch>> {
        let mut ready = Vec::new();
        while let Some(top) = self.candidate_heap.peek() {
            if !self.candidate_is_ready(top.rank_value, threshold) {
                break;
            }
            let popped = self.candidate_heap.pop().unwrap();
            let last_rank_value = popped.rank_value;
            ready.push(popped);

            if ready.len() >= self.batch_size {
                if self.has_tie_breakers() {
                    // Drain any remaining ties with the exact same `rank_value` so that
                    // tie-breaking across secondary sort columns never splits a tie group.
                    while let Some(next) = self.candidate_heap.peek() {
                        if next.rank_value.to_bits() == last_rank_value.to_bits() {
                            ready.push(self.candidate_heap.pop().unwrap());
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
}

impl Stream for RankJoinStream {
    type Item = Result<RecordBatch>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        if this.done {
            return Poll::Ready(None);
        }

        // Step 1: Ensure both inputs have pulled their initial batch so `top_score` is known.
        if !this.initialized {
            match this.left_cursor.poll_ensure_buffered(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(Ok(false)) => {
                    this.done = true;
                    return Poll::Ready(None);
                }
                Poll::Ready(Ok(true)) => {}
            }
            match this.right_cursor.poll_ensure_buffered(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(Ok(false)) => {
                    this.done = true;
                    return Poll::Ready(None);
                }
                Poll::Ready(Ok(true)) => {}
            }
            this.initialized = true;
            // Seed both hash tables with row 0 from each side so initial corner scores
            // immediately reflect row 0 matches.
            this.advance_one_row(AdvanceSide::Left);
            this.advance_one_row(AdvanceSide::Right);
        }

        loop {
            // Ensure that whichever cursor has moved past the end of its current batch
            // pulls its next batch (or marks itself exhausted) before we compute `upper_bounds()`.
            if !this.left_cursor.has_buffered_rows() && !this.left_cursor.exhausted {
                match this.left_cursor.poll_ensure_buffered(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(e))),
                    Poll::Ready(Ok(_)) => {}
                }
            }
            if !this.right_cursor.has_buffered_rows() && !this.right_cursor.exhausted {
                match this.right_cursor.poll_ensure_buffered(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(e))),
                    Poll::Ready(Ok(_)) => {}
                }
            }

            let threshold = this.stopping_threshold();
            let has_ready = this
                .candidate_heap
                .peek()
                .is_some_and(|top| this.candidate_is_ready(top.rank_value, threshold));

            if has_ready {
                match this.drain_ready_batch(threshold) {
                    Err(e) => return Poll::Ready(Some(Err(e))),
                    Ok(Some(batch)) => {
                        return Poll::Ready(Some(Ok(batch.record_output(&this.baseline_metrics))));
                    }
                    Ok(None) => {
                        // All ready candidates in this drain were rejected by `JoinFilter`;
                        // continue advancing inputs.
                    }
                }
            }

            let left_avail = this.left_cursor.has_buffered_rows();
            let right_avail = this.right_cursor.has_buffered_rows();

            if !left_avail && !right_avail {
                // Both inputs are completely exhausted and no ready candidates remain.
                this.done = true;
                return Poll::Ready(None);
            }

            let (u_l, u_r) = this.upper_bounds();
            let side = if left_avail && (!right_avail || u_l >= u_r) {
                AdvanceSide::Left
            } else {
                AdvanceSide::Right
            };
            this.advance_one_row(side);
        }
    }
}

impl RecordBatchStream for RankJoinStream {
    fn schema(&self) -> SchemaRef {
        Arc::clone(&self.schema)
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
    use datafusion::physical_plan::stream::RecordBatchStreamAdapter;

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
