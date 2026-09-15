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

//! Pre-materialization dynamic filter support.
//!
//! See the [JoinScan README](../../postgres/customscan/joinscan/README.md) for
//! how dynamic filters fit into the overall pruning pipeline.
//!
//! Dynamic filters allow parent operators (e.g. `SortExec(TopK)` or `HashJoinExec`)
//! to push filters down into scan nodes so that rows failing the filter are pruned
//! before column materialization.
//!
//! There are three distinct mechanisms for dynamic filter pushdown:
//!
//! 1. **Query-Time Pushdown (Inverted Index):** Filters that are known before the scan
//!    begins (such as `InList` predicates from a completed HashJoin build-side) are
//!    intercepted during the first `poll_next` of the scan stream. They are converted
//!    into native Tantivy queries (e.g., `TermSetQuery`) and `AND`ed into the main
//!    search query. This filters documents *while* executing the search, leveraging
//!    the inverted index for maximum performance.
//!
//! 2. **Segment-Statistics Pushdown:** At batch boundaries, published dynamic-filter generations
//!    are checked against the execution reader's immutable `.stats` snapshot. Proofs are rebuilt
//!    only after a source publishes a new generation. A newly impossible active segment is
//!    abandoned, and impossible deferred segment scorers are never opened. Unsupported
//!    expressions and missing statistics retain the segment.
//!
//! 3. **Pre-Filter Pushdown (Fast Fields):** Evolving thresholds (such as the rolling
//!    Top K threshold from `SortExec`) or filters that cannot be mapped to the inverted
//!    index are applied as `PreFilter`s *after* the search but *before* Arrow column
//!    materialization. These evaluate directly against Tantivy fast fields (using
//!    term-ordinal bounds for strings or direct numeric comparisons).
//!
//! # Data Flow
//!
//! ```text
//! HashJoinExec / SortExec(TopK)
//!   creates DynamicFilterPhysicalExpr (e.g. "col IN (...)" or "val < current_threshold")
//!        │
//!        │  FilterPushdown pass
//!        ▼
//! PgSearchScanPlan                   ← handle_child_pushdown_result stores
//!   .dynamic_filters                   the DynamicFilterPhysicalExpr.
//!        │
//!        │  at first poll_next
//!        ▼
//! try_dynamic_filter_pushdown()      ← Converts eligible static filters (like InList)
//!                                      into a Tantivy Query and modifies the SearchIndexReader.
//!                                      Rewrites the DataFusion expr to lit(true).
//!        │
//!        │  at poll_next after a generation changes
//!        ▼
//! DynamicSegmentPruner::refresh()   ← proves the latest ranges against segment `.stats`;
//!        │                              Scanner drops active/deferred impossible segments
//!        ▼
//! build_filters()                   ← calls DynamicFilterPhysicalExpr::current()
//!   → collect_filters()               to get the latest threshold for remaining filters,
//!   → Vec<PreFilter>                  decomposes them into PreFilter(s).
//!        │
//!        ▼
//! Scanner::next()                    ← applies PreFilters via apply_arrow()
//!   prunes doc IDs in-place            before materializing Arrow columns.
//! ```
//!
//! # Native DataFusion Evaluation
//!
//! `PreFilter`s do not execute custom matching logic. Instead, they leverage native DataFusion
//! `PhysicalExpr` evaluation over a mock `RecordBatch` containing only the fetched fast-field columns.
//! For string columns, to avoid expensive materialization, the `PreFilter` dynamically rewrites the
//! expression per segment: translating string literals into local `UInt64` ordinal bounds and evaluating
//! the bounds check directly against the fetched term ordinals. This allows complex expressions
//! (e.g. `IS NULL OR col < 'abc'`) to be seamlessly evaluated by Arrow's highly optimized compute kernels.
//!
//! # Observability
//!
//! `segments_pruned_dynamic_range`, `rows_pruned`, and `rows_scanned` in
//! `EXPLAIN (ANALYZE)` distinguish scorer avoidance from row-level filtering;
//! `dynamic_filters=N` in the non-ANALYZE plan shows how many filters were pushed down.

use std::any::Any;
use std::ops::Bound;
use std::sync::Arc;
use std::sync::atomic::AtomicU8;

use arrow_schema::SchemaRef;
use datafusion::arrow::array::UInt64Array;
use datafusion::arrow::array::{Array, ArrayRef, BooleanArray};
use datafusion::arrow::compute::cast;
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::ScalarValue;
use datafusion::common::tree_node::{Transformed, TransformedResult, TreeNode};
use datafusion::logical_expr::Operator;
use datafusion::physical_expr::PhysicalExpr;
use datafusion::physical_expr::expressions::{
    BinaryExpr, CastExpr, Column, DynamicFilterPhysicalExpr, IsNullExpr, Literal, NotExpr,
};
use datafusion::physical_plan::expressions::InListExpr;
use datafusion::physical_plan::joins::HashTableLookupExpr;
use tantivy::index::SegmentId;
use tantivy::{Score, SegmentOrdinal};

use crate::api::HashSet;
use crate::index::fast_fields_helper::{FFHelper, FFType, NULL_TERM_ORDINAL};
use crate::index::reader::index::{PushedDownInList, SearchIndexReader};
use crate::index::segment_pruning::SegmentStatsSnapshot;
use crate::index::segment_pruning::predicate::{
    SegmentTruth, SegmentTruthTable, table_for_exists, table_for_range, table_for_terms,
};
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::query::value_to_term;
use crate::scan::deferred_encode::is_deferred_field;
use crate::schema::SearchField;
use tantivy::Term;
use tantivy::query::{ConstScoreQuery, Query, TermSetQuery, TermSetStrategyConfig};

/// A pre-materialization filter applied inside `Scanner::next()`.
///
/// Wraps a DataFusion `PhysicalExpr` that has been validated to only contain
/// operations we can evaluate early (e.g. before fetching expensive string dictionaries).
pub struct PreFilter {
    /// The validated DataFusion physical expression.
    pub expr: Arc<dyn PhysicalExpr>,
    /// The indices of the fast fields this expression requires.
    pub required_columns: Vec<usize>,
}

/// A wrapper bundling a list of `PreFilter`s with the schema they apply to.
pub struct PreFilters<'a> {
    pub filters: &'a [PreFilter],
    pub schema: &'a SchemaRef,
}

impl PreFilter {
    /// Evaluate the pre-filter against a batch of memoized fast-field columns.
    /// Returns a boolean mask of rows that pass the filter.
    pub fn apply_arrow(
        &self,
        ffhelper: &FFHelper,
        segment_ord: SegmentOrdinal,
        memoized_columns: &[Option<ArrayRef>],
        schema: &SchemaRef,
        num_rows: usize,
    ) -> Result<BooleanArray, String> {
        // 1. Rewrite the expression for the current segment.
        // String literal comparisons are rewritten to ordinal comparisons.
        // NOTE: This runs two `transform()` passes on every batch. If this shows up in
        // profiling, the rewritten expression could be cached per-segment to reduce
        // allocation overhead for small batch sizes.
        let rewritten_string_expr = self
            .expr
            .clone()
            .transform_down(|node| {
                if let Some(dyn_filter) = node.downcast_ref::<DynamicFilterPhysicalExpr>() {
                    let current_expr = dyn_filter.current().map_err(|e| {
                        datafusion::error::DataFusionError::Execution(format!(
                            "DynamicFilter error: {}",
                            e
                        ))
                    })?;
                    return Ok(Transformed::yes(current_expr));
                } else if let Some(cast) = node.downcast_ref::<CastExpr>() {
                    if cast.cast_type() == &cast.expr().data_type(schema)? {
                        return Ok(Transformed::yes(Arc::clone(cast.expr())));
                    }
                    return Ok(Transformed::no(node));
                } else if let Some(binary) = node.downcast_ref::<BinaryExpr>() {
                    if let Some(rewritten) =
                        try_rewrite_binary(binary, ffhelper, segment_ord, schema)?
                    {
                        return Ok(Transformed::yes(rewritten));
                    }
                } else if let Some(in_list) = node.downcast_ref::<InListExpr>()
                    && let Some(rewritten) =
                        try_rewrite_in_list(in_list, ffhelper, segment_ord, schema)?
                {
                    return Ok(Transformed::yes(rewritten));
                }
                Ok(Transformed::no(node))
            })
            .data()
            .map_err(|e| format!("Failed to rewrite string expr: {}", e))?;

        let rewritten_expr = rewritten_string_expr
            .transform(|node| {
                if let Some(col) = node.downcast_ref::<Column>() {
                    let orig_idx = col.index();
                    if orig_idx < schema.fields().len()
                        && let Some(new_idx) = self
                            .required_columns
                            .iter()
                            .position(|&idx| idx == orig_idx)
                    {
                        let new_col = Column::new(col.name(), new_idx);
                        return Ok(Transformed::yes(Arc::new(new_col) as Arc<dyn PhysicalExpr>));
                    }
                }
                Ok(Transformed::no(node))
            })
            .data()
            .map_err(|e| format!("Failed to update col indices: {}", e))?;

        // 2. Build a RecordBatch from memoized_columns.
        // We only include the columns that were actually required and fetched.
        let mut fields = Vec::with_capacity(self.required_columns.len());
        let mut arrays = Vec::with_capacity(self.required_columns.len());
        for &ff_index in &self.required_columns {
            let col_name = schema.field(ff_index).name().clone();
            let mut array = memoized_columns[ff_index]
                .as_ref()
                .ok_or_else(|| format!("Column {} not fetched", ff_index))?
                .clone();

            let schema_field = schema.field(ff_index);
            let schema_type = schema_field.data_type();

            // Cast numeric fast fields to match the expected DataFusion schema type
            if !is_string_like_field(schema_field) && array.data_type() != schema_type {
                array = cast(&array, schema_type).map_err(|e| {
                    format!(
                        "Failed to cast columnar field from {:?} to DataFusion schema type {:?}: {}",
                        array.data_type(), schema_type, e
                    )
                })?;
            }
            // Note: The schema of the array might differ from the global schema
            // (e.g. UInt64 ordinals instead of Utf8). DataFusion `Column` exprs just extract by name/index,
            // so we must build the batch schema to match the *actual* array types we pass in.
            fields.push(Field::new(col_name, array.data_type().clone(), true));
            arrays.push(array);
        }

        let batch_schema = Arc::new(Schema::new(fields));
        let options = datafusion::arrow::record_batch::RecordBatchOptions::new()
            .with_row_count(Some(num_rows));
        let batch = RecordBatch::try_new_with_options(batch_schema.clone(), arrays, &options)
            .map_err(|e| format!("Failed to build RecordBatch: {}", e))?;

        // 3. Evaluate the rewritten expression natively via DataFusion.
        let columnar_value = rewritten_expr
            .evaluate(&batch)
            .map_err(|e| format!("Failed to evaluate expr: {}", e))?;

        let array = columnar_value
            .into_array(num_rows)
            .map_err(|e| format!("Failed to convert into array: {}", e))?;

        let bool_array = array
            .as_any()
            .downcast_ref::<BooleanArray>()
            .ok_or_else(|| "Result is not a BooleanArray".to_string())?
            .clone();

        Ok(bool_array)
    }
}

/// Recursively decomposes and validates a `PhysicalExpr` into `PreFilter`s.
///
/// Top-level `AND` operations are split into separate `PreFilter`s to allow early
/// short-circuiting in the scanner. Expressions containing unsupported nodes
/// (e.g. non-comparison operators, functions) are safely skipped.
pub fn collect_filters(
    expr: &Arc<dyn PhysicalExpr>,
    schema: &SchemaRef,
    out: &mut Vec<PreFilter>,
    score_col_schema_idx: Option<usize>,
    score_threshold: &mut Option<Score>,
) {
    // Split top-level ANDs to maximize early pruning
    if let Some(binary) = expr.downcast_ref::<BinaryExpr>()
        && matches!(binary.op(), Operator::And)
    {
        collect_filters(
            binary.left(),
            schema,
            out,
            score_col_schema_idx,
            score_threshold,
        );
        collect_filters(
            binary.right(),
            schema,
            out,
            score_col_schema_idx,
            score_threshold,
        );
        return;
    }

    let threshold = match (
        try_extract_score_threshold(expr, score_col_schema_idx),
        &score_threshold,
    ) {
        (Some(new), Some(existing)) => Some(new.min(*existing)),
        (Some(new), None) => Some(new),
        (None, Some(existing)) => Some(*existing),
        (None, None) => None,
    };
    *score_threshold = threshold;

    // Check if the expression is supported for pre-filtering
    let mut required_columns = Vec::new();
    if is_supported(expr, schema, &mut required_columns) {
        required_columns.sort_unstable();
        required_columns.dedup();
        out.push(PreFilter {
            expr: Arc::clone(expr),
            required_columns,
        });
    }
}

/// Prove which execution-visible segments cannot satisfy the *current* dynamic filters.
///
/// This is intentionally a proof-only lowering. Unsupported shapes return `None` and therefore
/// retain segments; DataFusion still evaluates the complete predicate above the scan. Top-level
/// dynamic filters are conjunctive, so a segment rejected by any one filter is safe to skip.
#[cfg(any(test, feature = "pg_test"))]
pub(crate) fn dynamically_rejected_segments(
    reader: &SearchIndexReader,
    filters: &[Arc<dyn PhysicalExpr>],
    schema: &SchemaRef,
) -> HashSet<SegmentId> {
    let mut pruner = DynamicSegmentPruner::new(filters);
    pruner.refresh(reader, schema).unwrap_or_default()
}

/// The identity a consumer caches a dynamic filter's published state under: the same expression
/// at the same `snapshot_generation`.
pub(crate) fn dynamic_filter_generation(dynamic: &DynamicFilterPhysicalExpr) -> (u64, u64) {
    (
        dynamic
            .expression_id()
            .expect("DynamicFilterPhysicalExpr has an expression id"),
        dynamic.snapshot_generation(),
    )
}

/// One DataFusion filter source whose updates are guaranteed to tighten during a single execution.
///
/// The producers currently admitted by `PgSearchScan` are Top-K thresholds, min/max aggregate
/// bounds, and hash-join filters published after their build information becomes authoritative.
/// DataFusion's generic `DynamicFilterPhysicalExpr` API does not encode this property, so all
/// downcasts are centralized here: adding another producer requires auditing this contract first.
struct MonotonicDynamicFilterSource(Arc<DynamicFilterPhysicalExpr>);

impl MonotonicDynamicFilterSource {
    fn from_physical_expr(source: &Arc<dyn PhysicalExpr>) -> Option<Self> {
        let source = Arc::clone(source) as Arc<dyn Any + Send + Sync>;
        Arc::downcast::<DynamicFilterPhysicalExpr>(source)
            .ok()
            .map(Self)
    }

    fn expression(&self) -> &DynamicFilterPhysicalExpr {
        &self.0
    }
}

/// Generation-aware execution cache for segment proofs derived from monotonic DataFusion dynamic
/// filters. A failed `current()` read leaves the evaluated generation unchanged, so a transient
/// remapping error cannot make a source appear to loosen: the caller keeps the rejection set it
/// last installed.
pub(crate) struct DynamicSegmentPruner {
    sources: Box<[MonotonicDynamicFilterSource]>,
    state: PrunerState,
}

enum PrunerState {
    NotEvaluated,
    Evaluated(Box<[(u64, u64)]>),
}

impl DynamicSegmentPruner {
    pub(crate) fn new(filters: &[Arc<dyn PhysicalExpr>]) -> Self {
        Self {
            sources: filters
                .iter()
                .filter_map(MonotonicDynamicFilterSource::from_physical_expr)
                .collect(),
            state: PrunerState::NotEvaluated,
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        !self.sources.is_empty()
    }

    /// Recompute only when a dynamic source publishes a new generation. `Some` means the caller
    /// must install the returned complete rejection set; `None` means its existing set remains
    /// authoritative.
    pub(crate) fn refresh(
        &mut self,
        reader: &SearchIndexReader,
        schema: &SchemaRef,
    ) -> Option<HashSet<SegmentId>> {
        if self.sources.is_empty() {
            return None;
        }
        let dynamic_filters = || {
            self.sources
                .iter()
                .map(MonotonicDynamicFilterSource::expression)
        };
        let unchanged = match &self.state {
            PrunerState::NotEvaluated => false,
            PrunerState::Evaluated(generations) => {
                generations.len() == self.sources.len()
                    && generations
                        .iter()
                        .zip(dynamic_filters())
                        .all(|(cached, dynamic)| *cached == dynamic_filter_generation(dynamic))
            }
        };
        if unchanged {
            return None;
        }

        // Capture the exact generation being evaluated before reading `current()`. If a producer
        // updates between these reads, the older generation is cached and the next refresh must
        // evaluate again; stale proof results are never labeled as a newer generation.
        let evaluated_generations = dynamic_filters()
            .map(dynamic_filter_generation)
            .collect::<Box<[_]>>();

        // Do not partially evaluate a generation: every top-level dynamic filter is conjunctive,
        // and retaining the prior complete set is the only safe response to one failed source.
        let current = dynamic_filters()
            .map(|dynamic| dynamic.current().ok())
            .collect::<Option<Vec<_>>>()?;
        let snapshot = reader.segment_stats_snapshot();
        let mut rejected = HashSet::default();
        for expr in current {
            if let Some(rejections) = dynamic_truth(reader, &expr, schema, &snapshot) {
                rejected.extend(rejections);
            }
        }
        self.state = PrunerState::Evaluated(evaluated_generations);
        Some(rejected)
    }
}

/// Resolve the Arrow column an expression reads (looking through casts) to its index field name.
fn column_field_name<'a>(expr: &Arc<dyn PhysicalExpr>, schema: &'a SchemaRef) -> Option<&'a str> {
    let column = physical_column(expr)?;
    schema
        .fields()
        .get(column.index())
        .map(|field| field.name().as_str())
}

fn exists_table(
    reader: &SearchIndexReader,
    snapshot: &Arc<SegmentStatsSnapshot>,
    field_name: &str,
) -> Option<Arc<SegmentTruthTable>> {
    let field = reader.schema().search_field(field_name)?;
    Some(table_for_exists(Arc::clone(snapshot), &field))
}

fn dynamic_truth(
    reader: &SearchIndexReader,
    expr: &Arc<dyn PhysicalExpr>,
    schema: &SchemaRef,
    snapshot: &Arc<SegmentStatsSnapshot>,
) -> Option<HashSet<SegmentId>> {
    if let Some(binary) = expr.downcast_ref::<BinaryExpr>() {
        return match binary.op() {
            Operator::And => match (
                dynamic_truth(reader, binary.left(), schema, snapshot),
                dynamic_truth(reader, binary.right(), schema, snapshot),
            ) {
                (Some(mut left), Some(right)) => {
                    left.extend(right);
                    Some(left)
                }
                // For a conjunction, one independently impossible arm is enough to reject a
                // segment. Composed values carry only rejection information, so retaining this
                // arm cannot accidentally claim that the unknown arm always matches.
                (Some(known), None) | (None, Some(known)) => Some(known),
                (None, None) => None,
            },
            Operator::Or => {
                let mut left = dynamic_truth(reader, binary.left(), schema, snapshot)?;
                let right = dynamic_truth(reader, binary.right(), schema, snapshot)?;
                left.retain(|id| right.contains(id));
                Some(left)
            }
            Operator::Eq
            | Operator::NotEq
            | Operator::Lt
            | Operator::LtEq
            | Operator::Gt
            | Operator::GtEq => comparison_truth(reader, binary, schema, snapshot),
            _ => None,
        };
    }

    // `IS NOT NULL` and general `NOT` are not lowered: statistics record whether a column has
    // nulls, never whether it has only nulls, so no segment can be rejected from them.
    if let Some(is_null) = expr.downcast_ref::<IsNullExpr>() {
        let field_name = column_field_name(is_null.arg(), schema)?;
        return exists_table(reader, snapshot, field_name).map(|table| table.rejected(true));
    }
    if let Some(in_list) = expr.downcast_ref::<InListExpr>() {
        return in_list_truth(reader, in_list, schema, snapshot);
    }
    if let Some(literal) = expr.downcast_ref::<Literal>() {
        let truth = match literal.value() {
            ScalarValue::Boolean(Some(true)) => SegmentTruth::Always,
            // In SQL filter position FALSE and NULL both reject every row.
            ScalarValue::Boolean(Some(false) | None) => SegmentTruth::Never,
            _ => return None,
        };
        return Some(SegmentTruthTable::uniform(Arc::clone(snapshot), truth).rejected(false));
    }
    None
}

fn physical_column(expr: &Arc<dyn PhysicalExpr>) -> Option<&Column> {
    if let Some(column) = expr.downcast_ref::<Column>() {
        Some(column)
    } else if let Some(cast) = expr.downcast_ref::<CastExpr>() {
        physical_column(cast.expr())
    } else {
        None
    }
}

/// Splits `Column op Literal` or `Literal op Column` into the column-first form. Casts are not
/// looked through: a cast may change values or comparison semantics.
fn column_op_literal(binary: &BinaryExpr) -> Option<(&Column, Operator, &Literal)> {
    if let (Some(column), Some(literal)) = (
        binary.left().downcast_ref::<Column>(),
        binary.right().downcast_ref::<Literal>(),
    ) {
        return Some((column, *binary.op(), literal));
    }
    let literal = binary.left().downcast_ref::<Literal>()?;
    let column = binary.right().downcast_ref::<Column>()?;
    Some((column, flip_operator(binary.op())?, literal))
}

fn search_field_for_column(
    reader: &SearchIndexReader,
    schema: &SchemaRef,
    column: &Column,
) -> Option<SearchField> {
    let field_name = schema.fields().get(column.index())?.name();
    reader.schema().search_field(field_name)
}

/// A hash join publishes a small build side as `column IN (...)`; every member is an execution
/// scalar in the column's Arrow encoding.
fn in_list_truth(
    reader: &SearchIndexReader,
    in_list: &InListExpr,
    schema: &SchemaRef,
    snapshot: &Arc<SegmentStatsSnapshot>,
) -> Option<HashSet<SegmentId>> {
    let column = in_list.expr().downcast_ref::<Column>()?;
    let search_field = search_field_for_column(reader, schema, column)?;
    let field_type = search_field.field_type();
    let members = in_list
        .list()
        .iter()
        .map(|member| {
            let scalar = extract_physical_scalar_value(member)?;
            PdbOwnedValue::from_execution_scalar(&scalar, &field_type)
        })
        .collect::<Option<Vec<_>>>()?;
    let table = table_for_terms(Arc::clone(snapshot), &search_field, members);
    Some(table.rejected(in_list.negated()))
}

fn comparison_truth(
    reader: &SearchIndexReader,
    binary: &BinaryExpr,
    schema: &SchemaRef,
    snapshot: &Arc<SegmentStatsSnapshot>,
) -> Option<HashSet<SegmentId>> {
    let (column, op, literal) = column_op_literal(binary)?;
    let search_field = search_field_for_column(reader, schema, column)?;
    let value = PdbOwnedValue::from_execution_scalar(literal.value(), &search_field.field_type())?;
    let snapshot = Arc::clone(snapshot);
    let table = match op {
        Operator::Eq | Operator::NotEq => table_for_terms(snapshot, &search_field, vec![value]),
        _ => {
            let (lower, upper) = match op {
                Operator::Lt => (Bound::Unbounded, Bound::Excluded(value)),
                Operator::LtEq => (Bound::Unbounded, Bound::Included(value)),
                Operator::Gt => (Bound::Excluded(value), Bound::Unbounded),
                Operator::GtEq => (Bound::Included(value), Bound::Unbounded),
                _ => return None,
            };
            table_for_range(snapshot, &search_field, &lower, &upper)
        }
    };
    Some(table.rejected(op == Operator::NotEq))
}

/// Check for expressions that we know always evaluate to false
fn expr_always_false(expr: &Arc<dyn PhysicalExpr>, score_col_schema_idx: usize) -> bool {
    // FALSE literals are obviously always false
    if let Some(lit) = expr.downcast_ref::<Literal>() {
        return matches!(lit.value(), ScalarValue::Boolean(Some(false)));
    }
    // score is never null, so 'score IS NULL' is always false
    if let Some(is_null_expr) = expr.downcast_ref::<IsNullExpr>()
        && let Some(col) = is_null_expr.arg().downcast_ref::<Column>()
    {
        return col.index() == score_col_schema_idx;
    }
    false
}

/// Attempt to extract a minimum score threshold from the expression. Bounds propagate as:
/// - `score > t`  => `t`
/// - `score = t`  => `t.next_down()` (rows at exactly `t` must survive)
/// - `AND`        => the minimum of the bounds from either side (so it holds for the whole conjunction)
/// - `OR`  
///     - If both sides contain a bound, keep the minimum. If one side has a bound, use
///       it only in the case the other side always evaluates to false. Any OR with a
///       possibly-true non-score expression cannot provide a threshold, as the non-score
///       arm may be true
///
/// The returned threshold value assumes the threshold check uses > (greater-than) semantics.
/// ASSUMPTION: We intentionally don't check the Operator::Lt variant as DataFusion always puts the column
/// on the left.
///
/// This is necessary for using the blockmax-wand optimization in joins
fn try_extract_score_threshold(
    expr: &Arc<dyn PhysicalExpr>,
    score_col_schema_idx: Option<usize>,
) -> Option<Score> {
    let score_col_schema_idx = score_col_schema_idx?;
    let binary_expr = expr.downcast_ref::<BinaryExpr>()?;
    match binary_expr.op() {
        Operator::Gt => {
            // Look for a binary expr that looks like: score > f32
            let col = binary_expr.left().downcast_ref::<Column>()?;
            if col.index() != score_col_schema_idx {
                return None;
            }
            match binary_expr.right().downcast_ref::<Literal>()?.value() {
                ScalarValue::Float32(Some(t)) => Some(*t),
                _ => None,
            }
        }
        Operator::Eq => {
            // Look for a binary expr that looks like: score = f32
            let col = binary_expr.left().downcast_ref::<Column>()?;
            if col.index() != score_col_schema_idx {
                return None;
            }
            match binary_expr.right().downcast_ref::<Literal>()?.value() {
                // PruningScorer's assume the threshold has greater-than semantics, so
                // take the next representable value below the eq check to keep the threshold valid
                ScalarValue::Float32(Some(t)) => Some(t.next_down()),
                _ => None,
            }
        }
        Operator::And => {
            match (
                try_extract_score_threshold(binary_expr.left(), Some(score_col_schema_idx)),
                try_extract_score_threshold(binary_expr.right(), Some(score_col_schema_idx)),
            ) {
                (Some(left), Some(right)) => Some(left.min(right)),
                (Some(v), None) | (None, Some(v)) => Some(v),
                (None, None) => None,
            }
        }
        Operator::Or => {
            // Members of an OR expression that are always false can be safely ignored, so we can
            // try to pull the score threshold from the other side.
            if expr_always_false(binary_expr.left(), score_col_schema_idx) {
                try_extract_score_threshold(binary_expr.right(), Some(score_col_schema_idx))
            } else if expr_always_false(binary_expr.right(), score_col_schema_idx) {
                try_extract_score_threshold(binary_expr.left(), Some(score_col_schema_idx))
            }
            // If both sides have a threshold-containing part, take the lower threshold: then any matching
            // row satisfies one of the arms and therefore exceeds the smaller bound, so pruning by it is safe
            else if let (Some(left), Some(right)) = (
                try_extract_score_threshold(binary_expr.left(), Some(score_col_schema_idx)),
                try_extract_score_threshold(binary_expr.right(), Some(score_col_schema_idx)),
            ) {
                Some(left.min(right))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Helper to centrally identify string, bytes, dictionary, and deferred string columns. A
/// deferred column is a `UInt64` of term ordinals that string literals compare against once
/// `try_rewrite_binary` has translated them, so it counts as a string here.
fn is_string_like_field(field: &Field) -> bool {
    is_deferred_field(field)
        || matches!(
            field.data_type(),
            DataType::Utf8
                | DataType::LargeUtf8
                | DataType::Utf8View
                | DataType::Binary
                | DataType::LargeBinary
                | DataType::BinaryView
                | DataType::Dictionary(_, _)
        )
}
/// Validates that an expression only contains nodes we can evaluate during pre-filtering.
///
/// NOTE: When this function returns `TreeNodeRecursion::Stop`, it correctly halts *all*
/// traversal across the entire expression tree. If an OR branch contains an unsupported
/// child, the entire expression is rejected.
fn is_supported(
    expr: &Arc<dyn PhysicalExpr>,
    schema: &SchemaRef,
    required_columns: &mut Vec<usize>,
) -> bool {
    let mut supported = true;
    let _ = expr.apply(|node| {
        if let Some(col) = node.downcast_ref::<Column>() {
            // Must map to a valid column index
            let idx = col.index();
            if idx < schema.fields().len() {
                required_columns.push(idx);
            } else {
                pgrx::warning!(
                    "pre_filter: column '{}' has physical index {} which is out of bounds \
                     for schema with {} fields — marking filter as unsupported",
                    col.name(),
                    idx,
                    schema.fields().len()
                );
                supported = false;
                return Ok(datafusion::common::tree_node::TreeNodeRecursion::Stop);
            }
        } else if node.is::<Literal>() {
            // Allowed
        } else if let Some(binary) = node.downcast_ref::<BinaryExpr>() {
            // Only logical and simple comparison operators are allowed
            match binary.op() {
                Operator::Eq
                | Operator::NotEq
                | Operator::Lt
                | Operator::LtEq
                | Operator::Gt
                | Operator::GtEq
                | Operator::And
                | Operator::Or => {}
                _ => {
                    supported = false;
                    return Ok(datafusion::common::tree_node::TreeNodeRecursion::Stop);
                }
            }
        } else if node.is::<IsNullExpr>() || node.is::<NotExpr>() || node.is::<InListExpr>() {
            // Allowed
        } else if node.is::<HashTableLookupExpr>() {
            // We only support HashTableLookupExpr for non-string columns.
            let mut is_numeric = true;
            let mut lookup_columns = Vec::new();

            // We manually inspect the subtree to check the data types of the columns it uses
            let _ = node.apply(|sub_node| {
                if let Some(col) = sub_node.downcast_ref::<Column>() {
                    let idx = col.index();
                    if idx < schema.fields().len() {
                        if is_string_like_field(schema.field(idx)) {
                            is_numeric = false;
                            return Ok(datafusion::common::tree_node::TreeNodeRecursion::Stop);
                        }
                        lookup_columns.push(idx);
                    } else {
                        is_numeric = false;
                        return Ok(datafusion::common::tree_node::TreeNodeRecursion::Stop);
                    }
                }
                Ok(datafusion::common::tree_node::TreeNodeRecursion::Continue)
            });

            if !is_numeric {
                supported = false;
                return Ok(datafusion::common::tree_node::TreeNodeRecursion::Stop);
            }

            required_columns.extend(lookup_columns);

            // We tell DataFusion's main traversal loop to skip visiting
            // the children of this HashTableLookupExpr, as the child is likely
            // an internal DataFusion hashing node that isn't on our allowlist.
            return Ok(datafusion::common::tree_node::TreeNodeRecursion::Jump);
        } else {
            // Any other node type (e.g. CAST, LIKE, UDFs) blocks the expression from pre-filtering
            supported = false;
            return Ok(datafusion::common::tree_node::TreeNodeRecursion::Stop);
        }

        Ok(datafusion::common::tree_node::TreeNodeRecursion::Continue)
    });
    supported
}

/// Attempts to rewrite a binary expression involving a String/Bytes column and a Literal
/// into an equivalent expression over segment-local ordinals.
fn try_rewrite_binary(
    binary: &BinaryExpr,
    ffhelper: &FFHelper,
    segment_ord: SegmentOrdinal,
    schema: &SchemaRef,
) -> datafusion::error::Result<Option<Arc<dyn PhysicalExpr>>> {
    let Some((col, op, lit)) = column_op_literal(binary) else {
        return Ok(None);
    };
    rewrite_col_op_lit(col, &op, lit, ffhelper, segment_ord, schema)
}

fn extract_bytes_from_scalar(scalar: &ScalarValue) -> Option<Option<&[u8]>> {
    match scalar {
        ScalarValue::Utf8(Some(s))
        | ScalarValue::LargeUtf8(Some(s))
        | ScalarValue::Utf8View(Some(s)) => Some(Some(s.as_bytes())),
        ScalarValue::Binary(Some(b))
        | ScalarValue::LargeBinary(Some(b))
        | ScalarValue::BinaryView(Some(b)) => Some(Some(b.as_slice())),

        ScalarValue::Utf8(None)
        | ScalarValue::LargeUtf8(None)
        | ScalarValue::Utf8View(None)
        | ScalarValue::Binary(None)
        | ScalarValue::LargeBinary(None)
        | ScalarValue::BinaryView(None) => Some(None),

        _ => None,
    }
}

fn try_rewrite_in_list(
    in_list: &InListExpr,
    ffhelper: &FFHelper,
    segment_ord: SegmentOrdinal,
    schema: &SchemaRef,
) -> datafusion::error::Result<Option<Arc<dyn PhysicalExpr>>> {
    let col = match in_list.expr().downcast_ref::<Column>() {
        Some(col) => col,
        None => return Ok(None),
    };
    let ff_index = col.index();
    if ff_index >= schema.fields().len() {
        return Ok(None);
    }
    let ff_type = ffhelper.column(segment_ord, ff_index);

    let dict = match ff_type {
        FFType::Text(c) => c.dictionary(),
        FFType::Bytes(c) => c.dictionary(),
        _ => return Ok(None), // Not a string/bytes column. Leave for native DataFusion eval
    };

    let mut ordinals = Vec::with_capacity(in_list.list().len());

    for lit_expr in in_list.list() {
        let lit = match lit_expr.downcast_ref::<Literal>() {
            Some(lit) => lit,
            None => return Ok(None),
        };
        let bytes = match extract_bytes_from_scalar(lit.value()) {
            Some(Some(b)) => b,
            Some(None) => {
                // Push None to preserve 3VL semantics when a NULL is in the IN list
                ordinals.push(None);
                continue;
            }
            None => return Ok(None), // Early abort if non-string literal is found
        };

        let target_ord = dict
            .term_ord(bytes)
            .map_err(|e| {
                datafusion::error::DataFusionError::Execution(format!("Tantivy dict error: {}", e))
            })?
            .unwrap_or(NULL_TERM_ORDINAL);
        ordinals.push(Some(target_ord));
    }

    // Convert the raw vector directly into an Arrow array
    let array = Arc::new(UInt64Array::from(ordinals)) as Arc<dyn Array>;
    let new_col_expr = Arc::new(col.clone()) as Arc<dyn PhysicalExpr>;

    // The rewritten column carries term ordinals, so the validation schema must
    // declare it as `UInt64` to match the ordinal array we just built.
    let mut ord_fields: Vec<Field> = schema.fields().iter().map(|f| f.as_ref().clone()).collect();
    ord_fields[ff_index] = Field::new(
        schema.field(ff_index).name(),
        DataType::UInt64,
        schema.field(ff_index).is_nullable(),
    );
    let ord_schema = Schema::new(ord_fields);
    let new_in_list =
        InListExpr::try_new_from_array(new_col_expr, array, in_list.negated(), &ord_schema)
            .map_err(|e| {
                datafusion::error::DataFusionError::Execution(format!(
                    "try_new_from_array failed: {}",
                    e
                ))
            })?;

    Ok(Some(Arc::new(new_in_list)))
}

/// Rewrites `Column op Literal` to `Column(UInt64) op Literal(UInt64)` if the column is a string type.
fn rewrite_col_op_lit(
    col: &Column,
    op: &Operator,
    lit: &Literal,
    ffhelper: &FFHelper,
    segment_ord: SegmentOrdinal,
    schema: &SchemaRef,
) -> datafusion::error::Result<Option<Arc<dyn PhysicalExpr>>> {
    let ff_index = col.index();
    if ff_index >= schema.fields().len() {
        return Ok(None);
    }
    let ff_type = ffhelper.column(segment_ord, ff_index);

    let bytes = match extract_bytes_from_scalar(lit.value()) {
        Some(Some(b)) => b,
        Some(None) => return Ok(Some(Arc::new(Literal::new(ScalarValue::Boolean(None))))),
        None => return Ok(None), // Not a string/bytes literal. Leave for native DataFusion eval over numerics.
    };

    let dict = match ff_type {
        FFType::Text(c) => c.dictionary(),
        FFType::Bytes(c) => c.dictionary(),
        _ => return Ok(None), // Not a string/bytes column. Leave for native DataFusion eval over numerics.
    };

    if op == &Operator::NotEq {
        let ord_opt = dict.term_ord(bytes).map_err(|e| {
            datafusion::error::DataFusionError::Execution(format!("Tantivy dict error: {}", e))
        })?;
        // If the term does not exist, all non-null values match.
        // We use NULL_TERM_ORDINAL to represent an ordinal that does not exist in the data.
        let target_ord = ord_opt.unwrap_or(NULL_TERM_ORDINAL);

        let col_expr = Arc::new(col.clone()) as Arc<dyn PhysicalExpr>;
        let lit_expr =
            Arc::new(Literal::new(ScalarValue::UInt64(Some(target_ord)))) as Arc<dyn PhysicalExpr>;
        return Ok(Some(
            Arc::new(BinaryExpr::new(col_expr, Operator::NotEq, lit_expr)) as Arc<dyn PhysicalExpr>,
        ));
    }

    // Convert string bounds to native string bounds.
    let (lower, upper) = match op {
        Operator::Lt => (Bound::Unbounded, Bound::Excluded(bytes)),
        Operator::LtEq => (Bound::Unbounded, Bound::Included(bytes)),
        Operator::Gt => (Bound::Excluded(bytes), Bound::Unbounded),
        Operator::GtEq => (Bound::Included(bytes), Bound::Unbounded),
        Operator::Eq => (Bound::Included(bytes), Bound::Included(bytes)),
        _ => return Ok(None),
    };

    // Lookup ordinal bounds.
    let (lo_ord, hi_ord) = dict.term_bounds_to_ord(lower, upper).map_err(|e| {
        datafusion::error::DataFusionError::Execution(format!("Tantivy dict error: {}", e))
    })?;

    // The Column must point to the correct index in our mock RecordBatch
    let col_expr = Arc::new(col.clone()) as Arc<dyn PhysicalExpr>;

    let mut exprs = Vec::new();
    match lo_ord {
        Bound::Included(ord) => {
            let lit_expr =
                Arc::new(Literal::new(ScalarValue::UInt64(Some(ord)))) as Arc<dyn PhysicalExpr>;
            exprs.push(
                Arc::new(BinaryExpr::new(col_expr.clone(), Operator::GtEq, lit_expr))
                    as Arc<dyn PhysicalExpr>,
            );
        }
        Bound::Excluded(ord) => {
            let lit_expr =
                Arc::new(Literal::new(ScalarValue::UInt64(Some(ord)))) as Arc<dyn PhysicalExpr>;
            exprs.push(
                Arc::new(BinaryExpr::new(col_expr.clone(), Operator::Gt, lit_expr))
                    as Arc<dyn PhysicalExpr>,
            );
        }
        Bound::Unbounded => {}
    }

    match hi_ord {
        Bound::Included(ord) => {
            let lit_expr =
                Arc::new(Literal::new(ScalarValue::UInt64(Some(ord)))) as Arc<dyn PhysicalExpr>;
            exprs.push(
                Arc::new(BinaryExpr::new(col_expr.clone(), Operator::LtEq, lit_expr))
                    as Arc<dyn PhysicalExpr>,
            );
        }
        Bound::Excluded(ord) => {
            let lit_expr =
                Arc::new(Literal::new(ScalarValue::UInt64(Some(ord)))) as Arc<dyn PhysicalExpr>;
            exprs.push(
                Arc::new(BinaryExpr::new(col_expr.clone(), Operator::Lt, lit_expr))
                    as Arc<dyn PhysicalExpr>,
            );
        }
        Bound::Unbounded => {}
    }

    if exprs.is_empty() {
        // Condition represents the entire dictionary range.
        Ok(Some(Arc::new(Literal::new(ScalarValue::Boolean(Some(
            true,
        ))))))
    } else if exprs.len() == 1 {
        Ok(Some(exprs.into_iter().next().unwrap()))
    } else {
        // Map exact bounds (lo_ord AND hi_ord) via AND
        Ok(Some(Arc::new(BinaryExpr::new(
            exprs[0].clone(),
            Operator::And,
            exprs[1].clone(),
        ))))
    }
}

/// Flips a comparison operator so that `Literal op Column` becomes `Column flipped_op Literal`.
fn flip_operator(op: &Operator) -> Option<Operator> {
    match op {
        Operator::Lt => Some(Operator::Gt),
        Operator::LtEq => Some(Operator::GtEq),
        Operator::Gt => Some(Operator::Lt),
        Operator::GtEq => Some(Operator::LtEq),
        Operator::Eq => Some(Operator::Eq),
        Operator::NotEq => Some(Operator::NotEq),
        _ => None,
    }
}

fn extract_physical_scalar_value(expr: &Arc<dyn PhysicalExpr>) -> Option<ScalarValue> {
    if let Some(lit) = expr.downcast_ref::<Literal>() {
        return Some(lit.value().clone());
    }
    None
}

fn extract_in_list_exprs<'a>(
    expr: &'a Arc<dyn PhysicalExpr>,
    in_lists: &mut Vec<&'a Arc<dyn PhysicalExpr>>,
) {
    if expr.is::<InListExpr>() {
        in_lists.push(expr);
    } else if let Some(binary) = expr.downcast_ref::<BinaryExpr>()
        && matches!(binary.op(), Operator::And)
    {
        extract_in_list_exprs(binary.left(), in_lists);
        extract_in_list_exprs(binary.right(), in_lists);
    }
}

/// Outcome of trying to convert one join-derived `InList` predicate.
enum InListPushdown {
    /// AND the converted term-set query into the tantivy search.
    Query(PushedDownInList),
    /// Drop the predicate: evaluating it in the scan costs more per row than the hash
    /// join above, which re-checks the same keys against its build table anyway.
    Skip,
    /// Not convertible; keep it as a batch-level pre-filter.
    Keep,
}

fn try_convert_in_list_to_query(
    in_list: &InListExpr,
    schema: &crate::schema::SearchIndexSchema,
    index_created_by_version: Option<crate::api::version::Version>,
    strategy_sink: Option<Arc<AtomicU8>>,
    max_segment_docs: u32,
    sorted_by_field: Option<&str>,
) -> InListPushdown {
    if in_list.negated() {
        return InListPushdown::Keep;
    }

    let Some(col) = in_list.expr().downcast_ref::<Column>() else {
        return InListPushdown::Keep;
    };
    let Some(field) = schema.search_field(col.name()) else {
        return InListPushdown::Keep;
    };

    if field.is_text() && !field.is_keyword() {
        return InListPushdown::Keep;
    }

    let field_type = field.field_type();

    // Check GUC thresholds
    let max_size = crate::gucs::hash_join_inlist_pushdown_max_size() as usize;
    let max_distinct = crate::gucs::hash_join_inlist_pushdown_max_distinct_values() as usize;

    // K cap. Default 20,000 is bench-tuned to the win/lose crossover for
    // pushdown vs PreFilter at N=1M sorted. See
    // gucs::HASH_JOIN_INLIST_PUSHDOWN_MAX_DISTINCT_VALUES doc for the
    // reasoning and trade-off. Set to 0 to disable pushdown.
    if in_list.list().len() > max_distinct {
        return InListPushdown::Keep;
    }

    // Estimate size: this is a rough estimate based on the number of elements
    // and their typical size.
    let estimated_size = in_list.list().len() * 32; // Assume ~32 bytes per element
    if estimated_size > max_size {
        return InListPushdown::Keep;
    }

    // Integrating the term set only pays when the segments admit an index-driven strategy.
    // Past its density gate tantivy's planner falls back to `LinearScan`, a per-candidate
    // fast-field probe that re-does the membership test the hash join above performs against
    // its build table anyway, at a far higher per-row cost than the join's own vectorized
    // probe — so the predicate is dropped rather than kept. The gate only guards that
    // fallback: a keyword column or one without a fast representation routes to the FST
    // automaton, and a column the segments are sorted by routes to gallop, both index-driven
    // at any density.
    // Gate on the largest segment: the big segments carry the linear cost, and a small
    // segment's linear scan is cheap even when it misses its own bitset gate. The looser
    // multi-column threshold is used because the column's docs-per-term isn't known here; a
    // unique-keyed column between the two thresholds still lands on `LinearScan`, an
    // accepted residual since either path is inexpensive in that band.
    // A per-segment exact decision needs doc counts and doc_freqs only tantivy's weight
    // sees. A `TermSetStrategyConfig` flag meaning "a consumer above rechecks membership"
    // would let each segment fall back to match-all instead of `LinearScan`, and this
    // planner-side prediction would go away; until then the largest segment stands in for
    // all of them.
    let linear_fallback_possible = field.is_fast()
        && !field.is_text()
        && !(crate::gucs::term_set_gallop_enabled() && sorted_by_field == Some(col.name()));
    let density = in_list.list().len() as f64 / max_segment_docs.max(1) as f64;
    if linear_fallback_possible && density > crate::gucs::term_set_bitset_max_density_multi() {
        return InListPushdown::Skip;
    }

    let tantivy_schema = schema.tantivy_schema();
    let Ok(tantivy_field) = tantivy_schema.get_field(col.name()) else {
        return InListPushdown::Keep;
    };
    let tantivy_field_type = tantivy_schema.get_field_entry(tantivy_field).field_type();

    let storage_values: Option<Vec<PdbOwnedValue>> = in_list
        .list()
        .iter()
        .map(|expr| {
            let scalar = extract_physical_scalar_value(expr)?;
            // Join-derived InList members are Arrow execution values. For
            // Numeric64 that means already-scaled Int64 — do not re-apply scale
            // via from_scalar (see #6158).
            PdbOwnedValue::from_execution_scalar(&scalar, &field_type)
        })
        .collect();

    let Some(storage_values) = storage_values else {
        return InListPushdown::Keep;
    };
    let terms: Option<Vec<Term>> = storage_values
        .iter()
        .map(|owned_value| {
            value_to_term(
                tantivy_field,
                owned_value,
                tantivy_field_type,
                None,
                index_created_by_version,
            )
            .ok()
        })
        .collect();

    let Some(terms) = terms else {
        return InListPushdown::Keep;
    };
    if terms.is_empty() {
        return InListPushdown::Keep;
    }

    // Build a strategy config from the paradedb.term_set_* GUCs so the
    // dispatch thresholds (kill switch, gallop density gate, and the
    // first-column bitset density gate) can be tuned in production
    // without a recompile. Defaults mirror `TermSetStrategyConfig::default()`
    // in tantivy. `subsequent_bitset_max_density` is not exposed because
    // it gates a branch tantivy doesn't reach in production today;
    // leave it at the tantivy default via struct update syntax. The
    // optional `strategy_sink` is a per-scan AtomicU8 the planner stores
    // its decision into so EXPLAIN ANALYZE can report which strategy fired.
    let cfg = TermSetStrategyConfig {
        gallop_enabled: crate::gucs::term_set_gallop_enabled(),
        bitset_max_density_unique: crate::gucs::term_set_bitset_max_density_unique(),
        bitset_max_density_multi: crate::gucs::term_set_bitset_max_density_multi(),
        strategy_sink,
        ..TermSetStrategyConfig::default()
    };

    let term_set_query = TermSetQuery::new(terms).with_strategy_config(cfg);
    let const_score_query = ConstScoreQuery::new(Box::new(term_set_query), 0.0);
    InListPushdown::Query(PushedDownInList {
        query: Box::new(const_score_query) as Box<dyn Query>,
        field_name: col.name().to_string(),
        canonical_terms: storage_values,
    })
}

/// Try to push down `InList` expressions from `dynamic_filters` into the search query.
///
/// This is called during the first `poll_next` of the scan stream to convert eligible `InList`
/// predicates (e.g. generated from a `HashJoin` build side) into native Tantivy `TermSet` queries.
///
/// By modifying the `SearchIndexReader` directly, this optimization allows Tantivy to filter
/// documents via its inverted index *while* executing the search, rather than filtering them
/// via fast fields after the search has returned.
///
/// If any expressions are successfully pushed down, this function:
/// 1. Combines them into a `BooleanQuery` and `AND`s it into the `reader`'s query.
/// 2. Mutates the provided `dynamic_filters` array in-place, rewriting the DataFusion
///    expressions to replace the pushed down nodes with `lit(true)` so they are not evaluated again.
/// 3. Returns `true` to indicate that a pushdown occurred.
pub fn try_dynamic_filter_pushdown(
    reader: &mut SearchIndexReader,
    dynamic_filters: &mut [Arc<dyn PhysicalExpr>],
    strategy_sink: Option<Arc<AtomicU8>>,
) -> bool {
    let mut pushdowns = Vec::new();
    let mut pushed_down_pointers = HashSet::default();
    let schema = reader.schema();
    let index_created_by_version = reader.index_created_by_version();
    let max_segment_docs = reader
        .segment_readers()
        .iter()
        .map(|sr| sr.num_docs())
        .max()
        .unwrap_or(0);
    let sorted_by_field = reader.sort_order().map(|s| s.field_name.to_string());

    for df in dynamic_filters {
        let Some(dynamic) = df.downcast_ref::<DynamicFilterPhysicalExpr>() else {
            continue;
        };
        let Ok(current_expr) = dynamic.current() else {
            continue;
        };

        let mut extracted_in_lists = Vec::new();
        extract_in_list_exprs(&current_expr, &mut extracted_in_lists);

        for in_list_arc in extracted_in_lists {
            let in_list = in_list_arc.downcast_ref::<InListExpr>().unwrap();

            match try_convert_in_list_to_query(
                in_list,
                schema,
                index_created_by_version,
                strategy_sink.clone(),
                max_segment_docs,
                sorted_by_field.as_deref(),
            ) {
                InListPushdown::Query(pushdown) => {
                    pushdowns.push(pushdown);
                    pushed_down_pointers.insert(Arc::as_ptr(in_list_arc) as *const () as usize);
                }
                InListPushdown::Skip => {
                    pushed_down_pointers.insert(Arc::as_ptr(in_list_arc) as *const () as usize);
                }
                InListPushdown::Keep => {}
            }
        }

        if !pushed_down_pointers.is_empty() {
            // Rewrite the expression to replace those nodes with lit(true)
            let rewritten = current_expr
                .clone()
                .transform_down(|node| {
                    if pushed_down_pointers.contains(&(Arc::as_ptr(&node) as *const () as usize)) {
                        Ok(Transformed::yes(Arc::new(Literal::new(
                            ScalarValue::Boolean(Some(true)),
                        ))))
                    } else {
                        Ok(Transformed::no(node))
                    }
                })
                .unwrap()
                .data;

            // Replace the DynamicFilterPhysicalExpr with the rewritten normal expression!
            *df = rewritten;
            pushed_down_pointers.clear();
        }
    }

    if pushdowns.is_empty() {
        false
    } else {
        *reader = reader.and_query_with_canonical_term_sets(pushdowns);
        true
    }
}

// These are ordinary Rust unit tests. Keeping the module out of a `pg_test` library build avoids
// compiling helper functions after `#[test]` items have been removed by the non-test harness.
#[cfg(test)]
mod tests {
    use super::try_extract_score_threshold;
    use datafusion::logical_expr::Operator;
    use datafusion::physical_expr::PhysicalExpr;
    use datafusion::physical_expr::expressions::{BinaryExpr, Column, is_not_null, is_null, lit};
    use datafusion::scalar::ScalarValue;
    use std::sync::Arc;

    const SCORE_IDX: usize = 0;
    const ID_IDX: usize = 1;

    fn score() -> Arc<dyn PhysicalExpr> {
        Arc::new(Column::new("pdb.score()", SCORE_IDX))
    }

    fn id() -> Arc<dyn PhysicalExpr> {
        Arc::new(Column::new("id", ID_IDX))
    }

    fn f32_lit(v: f32) -> Arc<dyn PhysicalExpr> {
        lit(ScalarValue::Float32(Some(v)))
    }

    fn int_lit(v: i64) -> Arc<dyn PhysicalExpr> {
        lit(ScalarValue::Int64(Some(v)))
    }

    fn bin(
        left: Arc<dyn PhysicalExpr>,
        op: Operator,
        right: Arc<dyn PhysicalExpr>,
    ) -> Arc<dyn PhysicalExpr> {
        Arc::new(BinaryExpr::new(left, op, right))
    }

    #[test]
    fn score_threshold_bare_gt_is_exact() {
        // Single-key `ORDER BY score DESC` publish. The extracted threshold is the
        // unrelaxed t: with no tiebreakers, a row tied with the cutoff can never
        // displace it, so the scorer's strict `score > t` mirrors the filter exactly.
        let expr = bin(score(), Operator::Gt, f32_lit(1.5));
        assert_eq!(
            try_extract_score_threshold(&expr, Some(SCORE_IDX)),
            Some(1.5f32)
        );
    }

    #[test]
    fn score_threshold_lexicographic_chain_with_score_leading() {
        // `ORDER BY score DESC, id ASC` publish: score > t OR (score = t AND id < v).
        // Rows tied at t may still win on the tiebreaker, so the extracted bound is
        // next_down(t): under the scorer's strict `>` semantics that keeps score >= t.
        let chain = bin(
            bin(score(), Operator::Gt, f32_lit(1.5)),
            Operator::Or,
            bin(
                bin(score(), Operator::Eq, f32_lit(1.5)),
                Operator::And,
                bin(id(), Operator::Lt, int_lit(10)),
            ),
        );
        assert_eq!(
            try_extract_score_threshold(&chain, Some(SCORE_IDX)),
            Some(1.5f32.next_down())
        );
    }

    #[test]
    fn score_threshold_nulls_first_wrapper() {
        // DESC NULLS FIRST publish: score IS NULL OR score > t. The IS NULL arm can
        // never match (every scored doc has a score), so the bound still holds.
        let expr = bin(
            is_null(score()).unwrap(),
            Operator::Or,
            bin(score(), Operator::Gt, f32_lit(1.5)),
        );
        assert_eq!(
            try_extract_score_threshold(&expr, Some(SCORE_IDX)),
            Some(1.5f32)
        );
    }

    #[test]
    fn score_threshold_conjunctive_null_guard() {
        // DESC NULLS LAST publish: score IS NOT NULL AND score > t. A conjunct's
        // bound holds for the whole conjunction, so extraction is sound.
        let expr = bin(
            is_not_null(score()).unwrap(),
            Operator::And,
            bin(score(), Operator::Gt, f32_lit(1.5)),
        );
        assert_eq!(
            try_extract_score_threshold(&expr, Some(SCORE_IDX)),
            Some(1.5f32)
        );
    }

    #[test]
    fn score_threshold_rejected_when_score_is_tiebreaker() {
        // `ORDER BY id ASC, score DESC` publish: id < v OR (id = v AND score > t).
        // The left arm admits rows of ANY score, so the expression implies no score
        // bound; extracting t here would block-prune rows that match via `id < v`.
        let chain = bin(
            bin(id(), Operator::Lt, int_lit(10)),
            Operator::Or,
            bin(
                bin(id(), Operator::Eq, int_lit(10)),
                Operator::And,
                bin(score(), Operator::Gt, f32_lit(1.5)),
            ),
        );
        assert_eq!(try_extract_score_threshold(&chain, Some(SCORE_IDX)), None);
    }

    #[test]
    fn score_threshold_rejects_foreign_shapes() {
        // Gt on a non-score column.
        let other_col = bin(id(), Operator::Gt, f32_lit(1.5));
        assert_eq!(
            try_extract_score_threshold(&other_col, Some(SCORE_IDX)),
            None
        );

        // Non-Float32 literal: the score column is always Float32, so a Float64
        // comparison (e.g. cast-normalized by an optimizer pass) must not parse.
        let f64_cmp = bin(score(), Operator::Gt, lit(ScalarValue::Float64(Some(1.5))));
        assert_eq!(try_extract_score_threshold(&f64_cmp, Some(SCORE_IDX)), None);

        // The lit(true) placeholder every dynamic filter holds before the first
        // TopK publish.
        assert_eq!(
            try_extract_score_threshold(&lit(true), Some(SCORE_IDX)),
            None
        );
    }
}
