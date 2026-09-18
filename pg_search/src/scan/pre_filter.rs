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
//! 2. **Segment-Statistics Pushdown:** Before a segment supplies a batch, its statistics are
//!    checked against the current dynamic filters. An impossible active segment is abandoned;
//!    an impossible deferred scorer never opens. Checks are per segment with no cached decisions.
//!    Unsupported expressions and missing statistics retain the segment.
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
//!        │  before each batch
//!        ▼
//! build_filters()                   ← gets the current row filters and score threshold
//!        │
//!        ▼
//! Scanner::next()
//!   → current_segment_matching()    ← selects or claims a segment
//!   → DynamicSegmentPruner::can_match()
//!        │                              checks that segment against the original sources;
//!        │                              drops it if impossible, otherwise reads a batch
//!        ▼
//! PreFilter::apply_arrow()          ← filters doc IDs before column materialization
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
//! For indexes declaring `partition_by`, `segments_pruned_dynamic` in `EXPLAIN (ANALYZE)`
//! counts segments skipped before opening their scorer or abandoned between batches.
//! `rows_pruned` and `rows_scanned` report row-level filtering; `dynamic_filters=N` in the
//! non-ANALYZE plan shows how many filters were pushed down.

use std::any::Any;
use std::cmp::Ordering;
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
use tantivy::{Score, SegmentOrdinal};

use crate::api::{FieldName, HashSet};
use crate::index::fast_fields_helper::{FFHelper, FFType, NULL_TERM_ORDINAL};
use crate::index::reader::index::SearchIndexReader;
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

/// Original dynamic-filter sources, retained before IN-list pushdown rewrites the row filters.
///
/// Producers must only tighten their filters during an execution: once the scanner drops a
/// segment it cannot revisit it, so a filter that loosened afterwards would lose rows silently.
/// This requirement is not checked at runtime.
pub(crate) struct DynamicSegmentPruner {
    sources: Box<[Arc<DynamicFilterPhysicalExpr>]>,
}

impl DynamicSegmentPruner {
    pub(crate) fn new(filters: &[Arc<dyn PhysicalExpr>]) -> Self {
        Self {
            sources: filters
                .iter()
                .filter_map(|source| {
                    let source = Arc::clone(source) as Arc<dyn Any + Send + Sync>;
                    Arc::downcast::<DynamicFilterPhysicalExpr>(source).ok()
                })
                .collect(),
        }
    }

    /// Check only the segment about to supply a batch. An unavailable dynamic predicate gives
    /// no exclusion; the existing row filter or parent operator remains authoritative.
    pub(crate) fn can_match(
        &self,
        reader: &SearchIndexReader,
        segment: SegmentOrdinal,
        schema: &SchemaRef,
    ) -> bool {
        self.sources.iter().all(|source| {
            source.current().map_or(true, |expr| {
                dynamic_can_match(reader, segment, &expr, schema)
            })
        })
    }
}

/// False proves that a segment cannot satisfy the current physical predicate. Unsupported
/// shapes retain the segment. Exact filtering remains with the Tantivy query, row filters,
/// or parent operator.
fn dynamic_can_match(
    reader: &SearchIndexReader,
    segment: SegmentOrdinal,
    expr: &Arc<dyn PhysicalExpr>,
    schema: &SchemaRef,
) -> bool {
    if let Some(binary) = expr.downcast_ref::<BinaryExpr>() {
        return match binary.op() {
            Operator::And => {
                dynamic_can_match(reader, segment, binary.left(), schema)
                    && dynamic_can_match(reader, segment, binary.right(), schema)
            }
            Operator::Or => {
                dynamic_can_match(reader, segment, binary.left(), schema)
                    || dynamic_can_match(reader, segment, binary.right(), schema)
            }
            Operator::Eq
            | Operator::NotEq
            | Operator::Lt
            | Operator::LtEq
            | Operator::Gt
            | Operator::GtEq => {
                comparison_can_match(reader, segment, binary, schema).unwrap_or(true)
            }
            _ => true,
        };
    }
    if let Some(is_null) = expr.downcast_ref::<IsNullExpr>() {
        return physical_column(is_null.arg())
            .and_then(|column| search_field_for_column(reader, schema, column))
            .and_then(|field| {
                reader
                    .segment_stats_snapshot()
                    .empirical(segment as usize, &field)
            })
            .is_none_or(|stats| stats.nullable);
    }
    if let Some(in_list) = expr.downcast_ref::<InListExpr>() {
        return in_list_can_match(reader, segment, in_list, schema).unwrap_or(true);
    }
    if let Some(literal) = expr.downcast_ref::<Literal>() {
        return !matches!(literal.value(), ScalarValue::Boolean(Some(false) | None));
    }
    true
}

/// The column an expression reads. A cast is looked through only when it cannot manufacture a
/// NULL: an unsafe cast errors on an invalid value, so nullness passes through unchanged.
fn physical_column(expr: &Arc<dyn PhysicalExpr>) -> Option<&Column> {
    if let Some(column) = expr.downcast_ref::<Column>() {
        Some(column)
    } else if let Some(cast) = expr.downcast_ref::<CastExpr>()
        && !cast.cast_options().safe
    {
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
    let name = FieldName::from(schema.fields().get(column.index())?.name().as_str());
    reader
        .schema()
        .stats_field(&name)
        .filter(SearchField::stats_order_matches_values)
}

/// Bounds in the same Arrow representation as the scanned column. Conversion happens once
/// per predicate/segment check, not once per IN-list member. No decoded values are cached.
struct ScalarStats {
    min: ScalarValue,
    max: ScalarValue,
    nullable: bool,
}

impl ScalarStats {
    fn for_column(
        reader: &SearchIndexReader,
        segment: SegmentOrdinal,
        schema: &SchemaRef,
        column: &Column,
    ) -> Option<Self> {
        let field = search_field_for_column(reader, schema, column)?;
        let stats = reader
            .segment_stats_snapshot()
            .empirical(segment as usize, &field)?;
        let data_type = schema.field(column.index()).data_type();
        let convert = |value: &PdbOwnedValue| match (data_type, value) {
            // The snapshot lifts datetime bounds into Date. Arrow execution uses the same
            // PostgreSQL-epoch microseconds as FFHelper, including legacy datetime columns.
            (
                DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, None),
                PdbOwnedValue::Date(v),
            ) => Some(ScalarValue::TimestampMicrosecond(
                Some(v.into_inner()),
                None,
            )),
            _ => value.to_scalar(data_type),
        };
        Some(Self {
            min: convert(&stats.min)?,
            max: convert(&stats.max)?,
            nullable: stats.nullable,
        })
    }

    /// None means the comparison cannot establish exclusion. NULLs, NaNs and incompatible
    /// scalar types do not supply an ordered bound; the exact predicate still evaluates them.
    fn can_match(&self, op: Operator, value: &ScalarValue) -> Option<bool> {
        if [&self.min, &self.max, value].into_iter().any(|v| {
            v.is_null()
                || matches!(v,
                ScalarValue::Float64(Some(f)) if f.is_nan())
                || matches!(v, ScalarValue::Float32(Some(f)) if f.is_nan())
        }) || self.min.data_type() != value.data_type()
            || self.max.data_type() != value.data_type()
        {
            return None;
        }
        // DataFusion's binary comparisons normalize signed zero, but its IN-list filters
        // compare float bits. Retain the segment when a bound and literal are opposite zeros.
        let compare = |bound: &ScalarValue| match (value, bound) {
            (ScalarValue::Float64(Some(a)), ScalarValue::Float64(Some(b)))
                if a == b && a.to_bits() != b.to_bits() =>
            {
                None
            }
            (ScalarValue::Float32(Some(a)), ScalarValue::Float32(Some(b)))
                if a == b && a.to_bits() != b.to_bits() =>
            {
                None
            }
            _ => value.partial_cmp(bound),
        };
        let lower = compare(&self.min)?;
        let upper = compare(&self.max)?;
        Some(match op {
            Operator::Eq => lower != Ordering::Less && upper != Ordering::Greater,
            Operator::NotEq => {
                self.nullable || lower != Ordering::Equal || upper != Ordering::Equal
            }
            Operator::Lt => lower == Ordering::Greater,
            Operator::LtEq => lower != Ordering::Less,
            Operator::Gt => upper == Ordering::Less,
            Operator::GtEq => upper != Ordering::Greater,
            _ => return None,
        })
    }
}

fn in_list_can_match(
    reader: &SearchIndexReader,
    segment: SegmentOrdinal,
    in_list: &InListExpr,
    schema: &SchemaRef,
) -> Option<bool> {
    let column = in_list.expr().downcast_ref::<Column>()?;
    let stats = ScalarStats::for_column(reader, segment, schema, column)?;
    let mut members = in_list.list().iter().map(extract_physical_scalar_value);
    Some(if in_list.negated() {
        // This check rejects a segment when its non-nullable bounds equal a list member.
        members.all(|value| {
            value
                .and_then(|v| stats.can_match(Operator::NotEq, v))
                .unwrap_or(true)
        })
    } else {
        members.any(|value| {
            value
                .and_then(|v| stats.can_match(Operator::Eq, v))
                .unwrap_or(true)
        })
    })
}

fn operator_bounds<T: Clone>(op: Operator, value: T) -> Option<(Bound<T>, Bound<T>)> {
    Some(match op {
        Operator::Lt => (Bound::Unbounded, Bound::Excluded(value)),
        Operator::LtEq => (Bound::Unbounded, Bound::Included(value)),
        Operator::Gt => (Bound::Excluded(value), Bound::Unbounded),
        Operator::GtEq => (Bound::Included(value), Bound::Unbounded),
        Operator::Eq => (Bound::Included(value.clone()), Bound::Included(value)),
        _ => return None,
    })
}

fn comparison_can_match(
    reader: &SearchIndexReader,
    segment: SegmentOrdinal,
    binary: &BinaryExpr,
    schema: &SchemaRef,
) -> Option<bool> {
    let (column, op, literal) = column_op_literal(binary)?;
    ScalarStats::for_column(reader, segment, schema, column)?.can_match(op, literal.value())
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

    let Some((lower, upper)) = operator_bounds(*op, bytes) else {
        return Ok(None);
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

fn extract_physical_scalar_value(expr: &Arc<dyn PhysicalExpr>) -> Option<&ScalarValue> {
    expr.downcast_ref::<Literal>().map(Literal::value)
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
    /// AND the converted term-set query into the Tantivy search.
    Query { query: Box<dyn Query> },
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

    if in_list.list().is_empty() {
        return InListPushdown::Keep;
    }
    let terms = in_list
        .list()
        .iter()
        .map(|member| {
            let scalar = extract_physical_scalar_value(member)?;
            // Join keys are execution values: Numeric64 is already scaled (see #6158).
            let owned_value = PdbOwnedValue::from_execution_scalar(scalar, &field_type)?;
            value_to_term(
                tantivy_field,
                &owned_value,
                tantivy_field_type,
                None,
                index_created_by_version,
            )
            .ok()
        })
        .collect::<Option<Vec<Term>>>();
    let Some(terms) = terms else {
        return InListPushdown::Keep;
    };
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
    InListPushdown::Query {
        query: Box::new(ConstScoreQuery::new(Box::new(term_set_query), 0.0)),
    }
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
    let mut queries = Vec::new();
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
                InListPushdown::Query { query } => {
                    queries.push(query);
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

    if queries.is_empty() {
        return false;
    }
    let additional_query = tantivy::query::BooleanQuery::new(
        queries
            .into_iter()
            .map(|query| (tantivy::query::Occur::Must, query))
            .collect(),
    );
    *reader = reader.and_query(Box::new(additional_query));
    true
}

#[cfg(test)]
mod unit_tests {
    use super::try_extract_score_threshold;
    use datafusion::logical_expr::Operator;
    use datafusion::physical_expr::PhysicalExpr;
    use datafusion::physical_expr::expressions::{BinaryExpr, Column, is_not_null, is_null, lit};
    use datafusion::scalar::ScalarValue;
    use std::sync::Arc;

    /// Check exclusion against DataFusion's actual row evaluation, including boundary equality,
    /// NULLs, signed zero, infinities and values whose ordering differs between representations.
    #[test]
    fn scalar_statistics_agree_with_datafusion_comparisons() {
        use super::ScalarStats;
        use arrow_array::{Array, BooleanArray, RecordBatch};
        use arrow_schema::{Field, Schema};

        let cases = vec![
            (
                vec![ScalarValue::Int64(Some(-10)), ScalarValue::Int64(Some(20))],
                vec![
                    ScalarValue::Int64(Some(-11)),
                    ScalarValue::Int64(Some(0)),
                    ScalarValue::Int64(Some(21)),
                ],
            ),
            (
                vec![
                    ScalarValue::UInt64(Some(0)),
                    ScalarValue::UInt64(Some(u64::MAX)),
                ],
                vec![ScalarValue::UInt64(Some(10))],
            ),
            (
                vec![
                    ScalarValue::Float64(Some(-0.0)),
                    ScalarValue::Float64(Some(0.0)),
                ],
                vec![
                    ScalarValue::Float64(Some(-1.0)),
                    ScalarValue::Float64(Some(1.0)),
                ],
            ),
            (
                vec![
                    ScalarValue::Float64(Some(f64::NEG_INFINITY)),
                    ScalarValue::Float64(Some(f64::INFINITY)),
                ],
                vec![
                    ScalarValue::Float64(Some(1.5)),
                    ScalarValue::Float64(Some(f64::NAN)),
                ],
            ),
            (
                vec![
                    ScalarValue::Utf8View(Some("Apple".into())),
                    ScalarValue::Utf8View(Some("zebra".into())),
                ],
                vec![
                    ScalarValue::Utf8View(Some("0".into())),
                    ScalarValue::Utf8View(Some("middle".into())),
                    ScalarValue::Utf8View(Some("zz".into())),
                ],
            ),
            (
                vec![
                    ScalarValue::Boolean(Some(false)),
                    ScalarValue::Boolean(Some(true)),
                ],
                vec![],
            ),
            // Equal extrema exercise exclusions for NOT IN / != as well as ordinary ranges.
            (
                vec![
                    ScalarValue::Int64(Some(1500)),
                    ScalarValue::Int64(Some(1500)),
                ],
                vec![
                    ScalarValue::Int64(Some(1499)),
                    ScalarValue::Int64(Some(1501)),
                ],
            ),
            (
                vec![
                    ScalarValue::TimestampMicrosecond(Some(-1), None),
                    ScalarValue::TimestampMicrosecond(Some(1), None),
                ],
                vec![ScalarValue::TimestampMicrosecond(Some(0), None)],
            ),
        ];
        let cases = cases.into_iter().chain([-0.0_f64, 0.0].map(|zero| {
            (
                vec![ScalarValue::Float64(Some(zero)); 2],
                vec![ScalarValue::Float64(Some(-zero))],
            )
        }));
        for (values, extra_bounds) in cases {
            let data_type = values[0].data_type();
            let null = ScalarValue::try_from(&data_type).unwrap();
            let schema = Arc::new(Schema::new(vec![Field::new("v", data_type, true)]));
            let mut bounds = values.clone();
            bounds.extend(extra_bounds);
            bounds.push(null.clone());
            for nullable in [false, true] {
                let stats = ScalarStats {
                    min: values[0].clone(),
                    max: values[1].clone(),
                    nullable,
                };
                let mut rows = values.clone();
                if nullable {
                    rows.push(null.clone());
                }
                let batch = RecordBatch::try_new(
                    Arc::clone(&schema),
                    vec![ScalarValue::iter_to_array(rows).unwrap()],
                )
                .unwrap();
                for value in &bounds {
                    for op in [
                        Operator::Eq,
                        Operator::NotEq,
                        Operator::Lt,
                        Operator::LtEq,
                        Operator::Gt,
                        Operator::GtEq,
                    ] {
                        let expr =
                            BinaryExpr::new(Arc::new(Column::new("v", 0)), op, lit(value.clone()));
                        let result = expr
                            .evaluate(&batch)
                            .unwrap()
                            .into_array(batch.num_rows())
                            .unwrap();
                        let any_matches = result
                            .as_any()
                            .downcast_ref::<BooleanArray>()
                            .unwrap()
                            .iter()
                            .any(|v| v == Some(true));
                        let decision = stats.can_match(op, value);
                        if matches!(op, Operator::Eq | Operator::NotEq) {
                            for list_len in [1, 32] {
                                let membership = datafusion::physical_expr::expressions::in_list(
                                    Arc::new(Column::new("v", 0)),
                                    vec![lit(value.clone()); list_len],
                                    &(op == Operator::NotEq),
                                    &schema,
                                )
                                .unwrap();
                                let result = membership
                                    .evaluate(&batch)
                                    .unwrap()
                                    .into_array(batch.num_rows())
                                    .unwrap();
                                let any_matches = result
                                    .as_any()
                                    .downcast_ref::<BooleanArray>()
                                    .unwrap()
                                    .iter()
                                    .any(|v| v == Some(true));
                                assert!(
                                    decision != Some(false) || !any_matches,
                                    "membership {op:?} {value:?}, length {list_len}"
                                );
                            }
                        }

                        assert!(
                            decision != Some(false) || !any_matches,
                            "{op:?} {value:?}, bounds {:?}..{:?}",
                            stats.min,
                            stats.max
                        );
                        // Ordering predicates are exact on the extrema. Equality can also
                        // match an unobserved interior value, so min/max cannot prove a gap.
                        let ambiguous_zero = [&stats.min, &stats.max].into_iter().any(|bound| {
                            matches!((value, bound), (ScalarValue::Float64(Some(a)), ScalarValue::Float64(Some(b))) if a == b && a.to_bits() != b.to_bits())
                        });
                        if ambiguous_zero {
                            assert_eq!(decision, None);
                        }
                        if op != Operator::Eq
                            && !ambiguous_zero
                            && !nullable
                            && !value.is_null()
                            && !matches!(value, ScalarValue::Float64(Some(v)) if v.is_nan())
                        {
                            assert_eq!(
                                decision,
                                Some(any_matches),
                                "{op:?} {value:?}, bounds {:?}..{:?}",
                                stats.min,
                                stats.max
                            );
                        }
                    }
                }
            }
        }
        let unknown = ScalarStats {
            min: ScalarValue::Float64(Some(f64::NAN)),
            max: ScalarValue::Float64(Some(f64::NAN)),
            nullable: false,
        };
        assert_eq!(
            unknown.can_match(Operator::Eq, &ScalarValue::Float64(Some(0.0))),
            None
        );
        let incompatible = ScalarStats {
            min: ScalarValue::Int64(Some(1)),
            max: ScalarValue::Int64(Some(1)),
            nullable: false,
        };
        assert_eq!(
            incompatible.can_match(Operator::Eq, &ScalarValue::Utf8View(Some("1".into()))),
            None
        );
    }

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

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::reader::index::MultiSegmentSearchResults;
    use crate::index::reader::index::test_support::segmented_index_fixture;
    use crate::postgres::rel::PgSearchRelation;
    use crate::query::SearchQueryInput;
    use crate::schema::SearchFieldType;
    use datafusion::arrow::array::{Array, Float64Array};
    use datafusion::arrow::compute::CastOptions;
    use datafusion::arrow::datatypes::{DataType, Field as ArrowField, Schema as ArrowSchema};
    use datafusion::arrow::record_batch::RecordBatch;
    use datafusion::arrow::util::display::FormatOptions;
    use datafusion::common::ScalarValue;
    use datafusion::logical_expr::Operator;
    use datafusion::physical_expr::PhysicalExpr;
    use datafusion::physical_expr::expressions::{
        BinaryExpr, CastExpr, Column, DynamicFilterPhysicalExpr, Literal, in_list, is_null, lit,
    };
    use pgrx::prelude::*;
    use tantivy::index::SegmentId;

    fn open_snapshot_reader(
        index_rel: &PgSearchRelation,
        query: SearchQueryInput,
        need_scores: bool,
    ) -> SearchIndexReader {
        SearchIndexReader::open(index_rel, query, need_scores, MvccSatisfies::Snapshot).unwrap()
    }

    fn column(name: &str, index: usize) -> Arc<dyn PhysicalExpr> {
        Arc::new(Column::new(name, index))
    }

    fn int_literal(value: i64) -> Arc<dyn PhysicalExpr> {
        lit(ScalarValue::Int64(Some(value)))
    }

    fn binary(
        left: Arc<dyn PhysicalExpr>,
        op: Operator,
        right: Arc<dyn PhysicalExpr>,
    ) -> Arc<dyn PhysicalExpr> {
        Arc::new(BinaryExpr::new(left, op, right))
    }

    fn dynamic_i64_bound(
        op: Operator,
        value: i64,
    ) -> (Arc<DynamicFilterPhysicalExpr>, Arc<dyn PhysicalExpr>) {
        let column = column("id", 0);
        let expression = binary(Arc::clone(&column), op, int_literal(value));
        let dynamic = Arc::new(DynamicFilterPhysicalExpr::new(vec![column], expression));
        (Arc::clone(&dynamic), dynamic as Arc<dyn PhysicalExpr>)
    }

    fn dynamic(
        column: Arc<dyn PhysicalExpr>,
        predicate: Arc<dyn PhysicalExpr>,
    ) -> Arc<dyn PhysicalExpr> {
        Arc::new(DynamicFilterPhysicalExpr::new(vec![column], predicate))
    }

    fn single_field_schema(name: &str, data_type: DataType) -> Arc<ArrowSchema> {
        Arc::new(ArrowSchema::new(vec![ArrowField::new(
            name, data_type, false,
        )]))
    }

    fn index_from_sql(index_name: &str, setup: &str) -> PgSearchRelation {
        Spi::run(setup).unwrap();
        unsafe { pgrx::pg_sys::CommandCounterIncrement() };
        let index_oid =
            Spi::get_one::<pgrx::pg_sys::Oid>(&format!("SELECT '{index_name}'::regclass::oid"))
                .unwrap()
                .unwrap();
        PgSearchRelation::open(index_oid)
    }

    fn dynamic_schema() -> Arc<ArrowSchema> {
        Arc::new(ArrowSchema::new(vec![
            ArrowField::new("id", DataType::Int64, false),
            ArrowField::new("bucket", DataType::Int64, false),
        ]))
    }

    fn rejected_segments(
        reader: &SearchIndexReader,
        filters: &[Arc<dyn PhysicalExpr>],
        schema: &Arc<ArrowSchema>,
    ) -> HashSet<SegmentId> {
        let pruner = DynamicSegmentPruner::new(filters);
        reader
            .segment_readers()
            .iter()
            .enumerate()
            .filter(|(ord, _)| !pruner.can_match(reader, *ord as SegmentOrdinal, schema))
            .map(|(_, segment)| segment.segment_id())
            .collect()
    }

    // Drain through the same segment boundary used by Scanner, without applying row filters.
    fn count_dynamic(
        results: &mut MultiSegmentSearchResults,
        reader: &SearchIndexReader,
        pruner: &DynamicSegmentPruner,
        schema: &Arc<ArrowSchema>,
    ) -> usize {
        let mut count = 0;
        while let Some(segment) =
            results.current_segment_matching(|ord| pruner.can_match(reader, ord, schema))
        {
            count += segment.count();
            results.current_segment_pop();
        }
        count
    }

    #[pg_test]
    fn evolving_dynamic_range_skips_deferred_and_active_scorers() {
        use crate::index::reader::scorer::test_support::SCORERS_OPENED;

        let (index_rel, _heap) = segmented_index_fixture("evolving_range_pruning_test", 4, false);
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let (dynamic, dynamic_expr) = dynamic_i64_bound(Operator::Gt, 0);

        assert!(
            rejected_segments(&reader, &[Arc::clone(&dynamic_expr)], &dynamic_schema()).is_empty()
        );
        dynamic
            .update(binary(column("id", 0), Operator::Gt, int_literal(25)))
            .unwrap();
        let rejected = rejected_segments(&reader, &[Arc::clone(&dynamic_expr)], &dynamic_schema());
        assert_eq!(rejected.len(), 2, "1..20 cannot satisfy id > 25");

        SCORERS_OPENED.store(0, std::sync::atomic::Ordering::Relaxed);
        let mut remaining = reader.search();
        let pruner = DynamicSegmentPruner::new(&[Arc::clone(&dynamic_expr)]);
        assert_eq!(
            count_dynamic(&mut remaining, &reader, &pruner, &dynamic_schema()),
            20,
            "only the two possible segments open"
        );
        assert_eq!(
            SCORERS_OPENED.load(std::sync::atomic::Ordering::Relaxed),
            2,
            "dynamically rejected deferred scorers must never open"
        );
        assert_eq!(
            remaining.take_runtime_skipped(),
            2,
            "each rejected deferred segment is skipped exactly once"
        );
        assert_eq!(remaining.take_runtime_skipped(), 0);

        // Activate one scorer, then choose a direction whose tighter bound rejects that segment.
        // This models a Top-K cutoff becoming selective between scanner batches.
        let mut active = reader.search();
        SCORERS_OPENED.store(0, std::sync::atomic::Ordering::Relaxed);
        let (_, first_address) = active.next().expect("one segment scorer becomes active");
        let active_segment = reader
            .searcher()
            .segment_reader(first_address.segment_ord)
            .segment_id();
        let gt_rejected =
            rejected_segments(&reader, &[Arc::clone(&dynamic_expr)], &dynamic_schema());
        let (op, broad, tight) = if gt_rejected.contains(&active_segment) {
            (Operator::Gt, 0, 25)
        } else {
            (Operator::Lt, 100, 15)
        };
        dynamic
            .update(binary(column("id", 0), op, int_literal(broad)))
            .unwrap();
        assert!(
            rejected_segments(&reader, &[Arc::clone(&dynamic_expr)], &dynamic_schema()).is_empty()
        );
        dynamic
            .update(binary(column("id", 0), op, int_literal(tight)))
            .unwrap();
        let rejected = rejected_segments(&reader, &[Arc::clone(&dynamic_expr)], &dynamic_schema());
        assert_eq!(rejected.len(), 2);
        assert!(
            rejected.contains(&active_segment),
            "the tightened bound must make the active segment impossible"
        );
        assert_eq!(
            count_dynamic(&mut active, &reader, &pruner, &dynamic_schema()),
            20
        );
        assert_eq!(
            SCORERS_OPENED.load(std::sync::atomic::Ordering::Relaxed),
            3,
            "one active scorer plus two possible segments; the fourth never opens"
        );
        assert_eq!(
            active.take_runtime_skipped(),
            2,
            "the abandoned active segment and the never-opened segment are both skipped"
        );
    }

    #[pg_test]
    fn dynamic_skips_stay_consumed_when_precision_regresses() {
        let index_rel = index_from_sql(
            "dynamic_nan_pruning_test_idx",
            "CREATE TABLE dynamic_nan_pruning_test (
                 id bigint PRIMARY KEY,
                 value double precision NOT NULL
             );
             CREATE INDEX dynamic_nan_pruning_test_idx
             ON dynamic_nan_pruning_test
             USING paradedb (id, value)
             WITH (target_segment_count = 8, background_layer_sizes = '0');
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO dynamic_nan_pruning_test VALUES (1, 0.0);
             INSERT INTO dynamic_nan_pruning_test VALUES (2, 10.0);
             INSERT INTO dynamic_nan_pruning_test VALUES (3, 'NaN');
             RESET paradedb.global_mutable_segment_rows;",
        );
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let snapshot = reader.segment_stats_snapshot();
        assert_eq!(snapshot.len(), 3, "the fixture must contain three segments");

        let schema = single_field_schema("value", DataType::Float64);
        let value = column("value", 0);
        let above_negative_one = binary(
            Arc::clone(&value),
            Operator::Gt,
            lit(ScalarValue::Float64(Some(-1.0))),
        );
        let above_five = binary(
            Arc::clone(&value),
            Operator::Gt,
            lit(ScalarValue::Float64(Some(5.0))),
        );
        let dynamic = Arc::new(DynamicFilterPhysicalExpr::new(
            vec![Arc::clone(&value)],
            above_negative_one,
        ));
        let dynamic_expr = Arc::clone(&dynamic) as Arc<dyn PhysicalExpr>;
        let pruner = DynamicSegmentPruner::new(&[Arc::clone(&dynamic_expr)]);
        assert!(rejected_segments(&reader, &[Arc::clone(&dynamic_expr)], &schema).is_empty());
        dynamic.update(Arc::clone(&above_five)).unwrap();
        let rejected = rejected_segments(&reader, &[Arc::clone(&dynamic_expr)], &schema);
        assert_eq!(rejected.len(), 1);
        // Eager segment iterators are consumed from the end. Visit the rejected segment first,
        // then pause before opening the next one to model a batch boundary.
        let rejected_id = *rejected.iter().next().unwrap();
        let mut ids = reader.segment_ids();
        ids.retain(|id| *id != rejected_id);
        ids.push(rejected_id);
        let mut results = reader.search_segments(ids.into_iter());
        assert!(
            results
                .current_segment_matching(|ord| pruner.can_match(&reader, ord, &schema))
                .is_some()
        );
        assert_eq!(results.take_runtime_skipped(), 1);

        let above_nan = binary(
            value,
            Operator::Gt,
            lit(ScalarValue::Float64(Some(f64::NAN))),
        );
        let values = RecordBatch::try_new(
            Arc::clone(&schema),
            vec![Arc::new(Float64Array::from(vec![0.0, 10.0, f64::NAN]))],
        )
        .unwrap();
        let evaluate = |predicate: &Arc<dyn PhysicalExpr>| {
            predicate
                .evaluate(&values)
                .unwrap()
                .into_array(values.num_rows())
                .unwrap()
                .as_any()
                .downcast_ref::<datafusion::arrow::array::BooleanArray>()
                .unwrap()
                .iter()
                .collect::<Vec<_>>()
        };
        assert_eq!(
            evaluate(&above_five),
            vec![Some(false), Some(true), Some(true)]
        );
        assert_eq!(
            evaluate(&above_nan),
            vec![Some(false), Some(false), Some(false)],
            "the NaN threshold is semantically tighter under Arrow total ordering"
        );

        dynamic.update(above_nan).unwrap();
        assert!(
            rejected_segments(&reader, &[dynamic_expr], &schema).is_empty(),
            "NaN deliberately makes a fresh statistics proof fail open"
        );
        // Nothing is remembered between checks, so a fresh one keeps all three segments. The
        // segment already discarded is gone from the iterator and cannot come back.
        assert_eq!(count_dynamic(&mut results, &reader, &pruner, &schema), 2);
        assert_eq!(results.take_runtime_skipped(), 0);
        assert_eq!(
            count_dynamic(&mut reader.search(), &reader, &pruner, &schema),
            3
        );
    }

    #[pg_test]
    fn dynamic_in_list_rejects_segments() {
        let (index_rel, _heap) = segmented_index_fixture("dynamic_in_list_pruning_test", 4, false);
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let membership = |negated: bool| {
            let id = column("id", 0);
            let members = vec![int_literal(15), int_literal(16)];
            dynamic(
                Arc::clone(&id),
                in_list(id, members, &negated, &dynamic_schema()).unwrap(),
            )
        };

        let rejected = rejected_segments(&reader, &[membership(false)], &dynamic_schema());
        assert_eq!(
            rejected.len(),
            3,
            "only the segment holding 11..20 can contain 15 or 16"
        );
        assert!(
            rejected_segments(&reader, &[membership(true)], &dynamic_schema()).is_empty(),
            "NOT IN can only reject a segment whose every row equals a member"
        );

        let mut results = reader.search();
        let pruner = DynamicSegmentPruner::new(&[membership(false)]);
        assert_eq!(
            count_dynamic(&mut results, &reader, &pruner, &dynamic_schema()),
            10
        );
        assert_eq!(results.take_runtime_skipped(), 3);
    }

    #[pg_test]
    fn skipped_in_list_pushdown_keeps_its_dynamic_proof() {
        let (index_rel, _heap) = segmented_index_fixture("skip_in_list_pruning_test", 4, false);
        let mut reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);

        let id = column("id", 0);
        let members = (11..=20).map(int_literal).collect();
        let dynamic = Arc::new(DynamicFilterPhysicalExpr::new(
            vec![Arc::clone(&id)],
            in_list(id, members, &false, &dynamic_schema()).unwrap(),
        ));
        let mut filters = vec![Arc::clone(&dynamic) as Arc<dyn PhysicalExpr>];
        let pruner = DynamicSegmentPruner::new(&filters);

        // Density 10/10 exceeds any gate at zero, so the pushdown takes `Skip`.
        Spi::run("SET paradedb.term_set_bitset_max_density_multi = 0").unwrap();
        let pushed = try_dynamic_filter_pushdown(&mut reader, &mut filters, None);
        Spi::run("RESET paradedb.term_set_bitset_max_density_multi").unwrap();
        assert!(!pushed, "Skip must not install a Tantivy term set");
        assert_eq!(
            reader.segment_pruning_estimate().candidate_segments,
            4,
            "the reader's static proof is untouched by a skipped pushdown"
        );
        assert!(
            filters[0].downcast_ref::<Literal>().is_some(),
            "the skipped membership predicate is rewritten out of the row filter"
        );

        let mut results = reader.search();
        assert_eq!(
            count_dynamic(&mut results, &reader, &pruner, &dynamic_schema()),
            10
        );
        assert_eq!(results.take_runtime_skipped(), 3);
    }

    /// A disjunction keeps any segment either side can satisfy. A filter that is constantly
    /// false rejects every segment, while the `lit(true)` a producer publishes before its first
    /// update rejects none.
    #[pg_test]
    fn dynamic_disjunctions_and_constant_filters_reject_segments() {
        let (index_rel, _heap) = segmented_index_fixture("dynamic_or_literal", 4, false);
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let id = || column("id", 0);

        // Fixture ids run 1..=40 over four segments of ten.
        let either_end = dynamic(
            id(),
            binary(
                binary(id(), Operator::Gt, int_literal(35)),
                Operator::Or,
                binary(id(), Operator::Lt, int_literal(5)),
            ),
        );
        assert_eq!(
            rejected_segments(&reader, &[either_end], &dynamic_schema()).len(),
            2,
            "only the segments holding 11..30 can satisfy neither side"
        );

        let constant = |value| dynamic(id(), lit(ScalarValue::Boolean(value)));
        assert_eq!(
            rejected_segments(&reader, &[constant(Some(false))], &dynamic_schema()).len(),
            4,
            "a filter that is always false rejects every segment"
        );
        assert_eq!(
            rejected_segments(&reader, &[constant(None)], &dynamic_schema()).len(),
            4,
            "NULL in filter position rejects every row"
        );
        assert!(
            rejected_segments(&reader, &[constant(Some(true))], &dynamic_schema()).is_empty(),
            "the placeholder published before a producer's first update rejects nothing"
        );
    }

    #[pg_test]
    fn dynamic_multi_field_checks_reject_segments() {
        use crate::index::reader::scorer::test_support::SCORERS_OPENED;

        let (index_rel, _heap) = segmented_index_fixture("dynamic_segment_pruning_test", 4, false);
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let (_, id_filter) = dynamic_i64_bound(Operator::Gt, 20);
        let bucket = column("bucket", 1);
        let bucket_filter = dynamic(
            Arc::clone(&bucket),
            binary(bucket, Operator::Lt, int_literal(300)),
        );
        let rejected = rejected_segments(
            &reader,
            &[Arc::clone(&id_filter), Arc::clone(&bucket_filter)],
            &dynamic_schema(),
        );
        assert_eq!(rejected.len(), 3);
        let mut results = reader.search();
        let pruner = DynamicSegmentPruner::new(&[id_filter, bucket_filter]);
        SCORERS_OPENED.store(0, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(
            count_dynamic(&mut results, &reader, &pruner, &dynamic_schema()),
            10
        );
        assert_eq!(SCORERS_OPENED.load(std::sync::atomic::Ordering::Relaxed), 1);

        let bucket = column("bucket", 1);
        let null_filter = dynamic(Arc::clone(&bucket), is_null(bucket).unwrap());
        assert_eq!(
            rejected_segments(&reader, &[null_filter], &dynamic_schema()).len(),
            4,
            "IS NULL rejects every segment whose statistics prove bucket is always present"
        );
    }

    #[pg_test]
    fn dynamic_text_ordering_and_timestamp_bounds() {
        let index_rel = index_from_sql(
            "dynamic_scalar_bounds_idx",
            r#"
            CREATE TABLE dynamic_scalar_bounds (id bigint PRIMARY KEY, raw_text text, folded text, stamp timestamp);
            CREATE INDEX dynamic_scalar_bounds_idx ON dynamic_scalar_bounds
            USING paradedb (id, raw_text, folded, stamp)
            WITH (background_layer_sizes = '0', text_fields = '{
                "raw_text": {"fast": true, "normalizer": "raw", "tokenizer": {"type": "keyword"}},
                "folded": {"fast": true, "normalizer": "lowercase", "tokenizer": {"type": "keyword"}}
            }');
            SET paradedb.global_mutable_segment_rows = 0;
            INSERT INTO dynamic_scalar_bounds VALUES (1, 'Zulu', 'Zulu', '2000-01-01 00:00:00');
            RESET paradedb.global_mutable_segment_rows;
        "#,
        );
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        assert_eq!(reader.segment_stats_snapshot().len(), 1);
        for (name, data_type, inside, outside, can_prune) in [
            (
                "raw_text",
                DataType::Utf8View,
                ScalarValue::Utf8View(Some("Zulu".into())),
                ScalarValue::Utf8View(Some("zzzz".into())),
                true,
            ),
            (
                "folded",
                DataType::Utf8View,
                ScalarValue::Utf8View(Some("zulu".into())),
                ScalarValue::Utf8View(Some("zzzz".into())),
                false,
            ),
            (
                "stamp",
                DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, None),
                ScalarValue::TimestampMicrosecond(Some(0), None),
                ScalarValue::TimestampMicrosecond(Some(1), None),
                true,
            ),
        ] {
            let field = reader.schema().search_field(name).unwrap();
            assert!(
                reader
                    .segment_stats_snapshot()
                    .empirical(0, &field)
                    .is_some()
            );
            let schema = single_field_schema(name, data_type);
            for (value, equal) in [(inside, true), (outside, false)] {
                let col = column(name, 0);
                for negated in [false, true] {
                    let op = if negated {
                        Operator::NotEq
                    } else {
                        Operator::Eq
                    };
                    let comparison = binary(Arc::clone(&col), op, lit(value.clone()));
                    let membership = in_list(
                        Arc::clone(&col),
                        vec![lit(value.clone())],
                        &negated,
                        &schema,
                    )
                    .unwrap();
                    for predicate in [comparison, membership] {
                        assert_eq!(
                            rejected_segments(
                                &reader,
                                &[dynamic(Arc::clone(&col), predicate)],
                                &schema
                            )
                            .len(),
                            usize::from(can_prune && equal == negated),
                            "{name}, negated={negated}"
                        );
                    }
                }
            }
        }
    }

    #[pg_test]
    fn dynamic_numeric64_comparisons_use_storage_scale() {
        let index_rel = index_from_sql(
            "dynamic_numeric64_pruning_test_idx",
            "CREATE TABLE dynamic_numeric64_pruning_test (
                 id bigint PRIMARY KEY,
                 price numeric(10, 2) NOT NULL
             );
             CREATE INDEX dynamic_numeric64_pruning_test_idx
             ON dynamic_numeric64_pruning_test
             USING paradedb (id, price)
             WITH (target_segment_count = 8,
                   background_layer_sizes = '0');
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO dynamic_numeric64_pruning_test
             SELECT g, g::numeric(10, 2) FROM generate_series(10, 20) g;
             RESET paradedb.global_mutable_segment_rows;",
        );
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let price_field = reader.schema().search_field("price").unwrap();
        assert!(
            matches!(price_field.field_type(), SearchFieldType::Numeric64(_, 2)),
            "the fixture must exercise the scaled Numeric64 representation"
        );
        let snapshot = reader.segment_stats_snapshot();
        assert!(snapshot.len() > 0, "the fixture must contain a segment");
        assert!(
            (0..snapshot.len()).all(|idx| snapshot.empirical(idx, &price_field).is_some()),
            "the fixture must have readable Numeric64 statistics so missing stats cannot make the test pass"
        );

        // DataFusion exposes Numeric64 fast fields as storage-scaled Int64 values. For scale 2,
        // 15.00 arrives as 1500. Treating 1500 as a logical value would scale it again to 150000
        // and incorrectly reject the segment containing prices 10.00 through 20.00.
        let schema = single_field_schema("price", DataType::Int64);
        let price_above = |storage_value: i64| {
            let price = column("price", 0);
            dynamic(
                Arc::clone(&price),
                binary(price, Operator::Gt, int_literal(storage_value)),
            )
        };

        for (value, excluded) in [(1500, false), (2001, true)] {
            let price = column("price", 0);
            let predicate = in_list(
                Arc::clone(&price),
                vec![int_literal(value)],
                &false,
                &schema,
            )
            .unwrap();
            assert_eq!(
                rejected_segments(&reader, &[dynamic(price, predicate)], &schema).len(),
                if excluded { snapshot.len() } else { 0 }
            );
        }

        let rejected = rejected_segments(&reader, &[price_above(1500)], &schema);
        assert!(
            rejected.is_empty(),
            "a storage-scaled bound inside the segment's range must not reject it"
        );
        let mut results = reader.search();
        let pruner = DynamicSegmentPruner::new(&[price_above(1500)]);
        assert_eq!(count_dynamic(&mut results, &reader, &pruner, &schema), 11);

        let rejected = rejected_segments(&reader, &[price_above(2000)], &schema);
        assert_eq!(
            rejected.len(),
            snapshot.len(),
            "a storage-scaled bound above the segment's maximum rejects it"
        );
        let mut results = reader.search();
        let pruner = DynamicSegmentPruner::new(&[price_above(2000)]);
        assert_eq!(count_dynamic(&mut results, &reader, &pruner, &schema), 0);
    }

    #[pg_test]
    fn dynamic_is_null_through_safe_cast_fails_open() {
        let index_rel = index_from_sql(
            "dynamic_safe_cast_pruning_test_idx",
            "CREATE TABLE dynamic_safe_cast_pruning_test (
                 id bigint PRIMARY KEY,
                 f double precision NOT NULL
             );
             CREATE INDEX dynamic_safe_cast_pruning_test_idx
             ON dynamic_safe_cast_pruning_test
             USING paradedb (id, f)
             WITH (target_segment_count = 8, background_layer_sizes = '0');
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO dynamic_safe_cast_pruning_test VALUES (1, 1e10);
             RESET paradedb.global_mutable_segment_rows;",
        );
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let search_field = reader.schema().search_field("f").unwrap();
        let snapshot = reader.segment_stats_snapshot();
        assert_eq!(snapshot.len(), 1, "the fixture must contain one segment");
        assert!(
            snapshot
                .empirical(0, &search_field)
                .is_some_and(|stats| !stats.nullable),
            "the fixture must prove `f` always present so only the cast guard can keep the segment"
        );

        let schema = single_field_schema("f", DataType::Float64);
        let f = column("f", 0);
        let cast_to_int32 = |safe: bool| {
            Arc::new(CastExpr::new(
                Arc::clone(&f),
                DataType::Int32,
                Some(CastOptions {
                    safe,
                    format_options: FormatOptions::default(),
                }),
            )) as Arc<dyn PhysicalExpr>
        };

        let predicate = is_null(cast_to_int32(true)).unwrap();
        let batch = RecordBatch::try_new(
            Arc::clone(&schema),
            vec![Arc::new(Float64Array::from(vec![1e10]))],
        )
        .unwrap();
        let evaluated = predicate.evaluate(&batch).unwrap().into_array(1).unwrap();
        let evaluated = evaluated
            .as_any()
            .downcast_ref::<datafusion::arrow::array::BooleanArray>()
            .unwrap();
        assert!(
            evaluated.value(0),
            "a safe cast turns the out-of-range value into NULL, so the authoritative predicate matches"
        );

        let rejected = rejected_segments(
            &reader,
            &[dynamic(Arc::clone(&f), Arc::clone(&predicate))],
            &schema,
        );
        assert!(
            rejected.is_empty(),
            "IS NULL through a safe cast must not be proven from the column's own nullability"
        );
        let mut results = reader.search();
        let pruner = DynamicSegmentPruner::new(&[dynamic(Arc::clone(&f), predicate)]);
        assert_eq!(
            count_dynamic(&mut results, &reader, &pruner, &schema),
            1,
            "failing open must retain the matching row"
        );

        let unsafe_predicate = is_null(cast_to_int32(false)).unwrap();
        assert_eq!(
            rejected_segments(&reader, &[dynamic(f, unsafe_predicate)], &schema).len(),
            1,
            "an unsafe cast cannot produce NULL, so the column's nullability proves the segment"
        );
    }

    #[pg_test]
    fn dynamic_cast_comparisons_fail_open() {
        let index_rel = index_from_sql(
            "dynamic_cast_pruning_test_idx",
            "CREATE TABLE dynamic_cast_pruning_test (
                 id bigint PRIMARY KEY,
                 f double precision NOT NULL
             );
             CREATE INDEX dynamic_cast_pruning_test_idx
             ON dynamic_cast_pruning_test
             USING paradedb (id, f)
             WITH (target_segment_count = 8, background_layer_sizes = '0');
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO dynamic_cast_pruning_test VALUES (1, 2.1);
             RESET paradedb.global_mutable_segment_rows;",
        );
        let reader = open_snapshot_reader(&index_rel, SearchQueryInput::All, false);
        let search_field = reader.schema().search_field("f").unwrap();
        assert!(
            matches!(search_field.field_type(), SearchFieldType::F64(_)),
            "the fixture must exercise raw floating-point statistics"
        );
        let snapshot = reader.segment_stats_snapshot();
        assert_eq!(snapshot.len(), 1, "the fixture must contain one segment");
        assert!(
            snapshot.empirical(0, &search_field).is_some(),
            "the fixture must have readable statistics so missing stats cannot make the test pass"
        );

        let schema = single_field_schema("f", DataType::Float64);
        let f = column("f", 0);
        let cast =
            Arc::new(CastExpr::new(Arc::clone(&f), DataType::Int64, None)) as Arc<dyn PhysicalExpr>;
        let predicate = binary(cast, Operator::Eq, int_literal(2));

        let batch = RecordBatch::try_new(
            Arc::clone(&schema),
            vec![Arc::new(Float64Array::from(vec![2.1]))],
        )
        .unwrap();
        let evaluated = predicate.evaluate(&batch).unwrap().into_array(1).unwrap();
        let evaluated = evaluated
            .as_any()
            .downcast_ref::<datafusion::arrow::array::BooleanArray>()
            .unwrap();
        assert!(
            evaluated.value(0),
            "the authoritative DataFusion predicate must match 2.1 after casting to BIGINT"
        );

        let rejected = rejected_segments(
            &reader,
            &[dynamic(Arc::clone(&f), Arc::clone(&predicate))],
            &schema,
        );
        assert!(
            rejected.is_empty(),
            "a casted comparison must not reject a segment using raw-column statistics"
        );

        let mut results = reader.search();
        let pruner = DynamicSegmentPruner::new(&[dynamic(f, predicate)]);
        assert_eq!(
            count_dynamic(&mut results, &reader, &pruner, &schema),
            1,
            "failing open must retain the matching row"
        );
    }
}
