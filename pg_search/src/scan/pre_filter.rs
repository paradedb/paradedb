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
//! Dynamic filters allow parent operators (e.g. `SortExec(TopK)`) to push evolving
//! thresholds into scan nodes so that rows failing the threshold are pruned *before*
//! column materialization — at the term-ordinal level for strings and direct
//! fast-field comparisons for numerics. This is critical for `ORDER BY … LIMIT`
//! queries over joins: without it, the scan must materialize every row even though
//! only the top-K are needed.
//!
//! # Data Flow
//!
//! ```text
//! SortExec(TopK)
//!   creates DynamicFilterPhysicalExpr ("val < current_threshold")
//!        │
//!        │  FilterPushdown pass
//!        ▼
//! FilterPassthroughExec               ← routes filter to correct join side
//!   (wraps SortMergeJoinExec)          using FilterDescription::from_children
//!        │
//!        ▼
//! PgSearchScanPlan                   ← handle_child_pushdown_result stores
//!   .dynamic_filters                   the DynamicFilterPhysicalExpr; when
//!                                      paradedb.dynamic_filter_batch_size > 0,
//!                                      caps the scanner batch size so TopK can
//!                                      tighten its threshold between batches
//!        │
//!        │  at poll time
//!        ▼
//! ScanStream::collect_pre_filters    ← calls DynamicFilterPhysicalExpr::current()
//!   → collect_filters()                to get the latest threshold, decomposes
//!   → Vec<PreFilter>                   it into PreFilter(s)
//!        │
//!        ▼
//! Scanner::next()                    ← applies PreFilters via apply_pre_filter()
//!   prunes doc IDs in-place            before materializing Arrow columns
//! ```
//!
//! # SortMergeJoin Propagation
//!
//! DataFusion's `SortMergeJoinExec` blocks filter pushdown by default (its
//! `gather_filters_for_pushdown` marks all parent filters as unsupported).
//! `FilterPassthroughExec` (in `joinscan::planner`) wraps it and overrides the
//! two filter-pushdown methods to route filters through.
//!
//! Because `SortMergeJoinEnforcer` runs as a physical optimizer rule *after* the
//! initial `FilterPushdown` pass, it causes `with_new_children` on ancestors —
//! which in `SortExec`'s case creates a *new* `DynamicFilterPhysicalExpr` that
//! hasn't been connected yet. A second `FilterPushdown::new_post_optimization()`
//! pass (registered in `joinscan::scan_state::create_session_context`) wires the
//! new filter to the scan.
//!
//! # NULL Handling (`nulls_pass`)
//!
//! TopK on a nullable column emits `col IS NULL OR col < threshold`. Without special
//! handling, the `OR` expression would be skipped entirely (no pre-filter created) or,
//! if naively decomposed, NULLs would be incorrectly pruned. [`try_or_is_null_pattern`]
//! detects this pattern, extracts the comparison, and produces a [`PreFilter`] with
//! `nulls_pass = true`. The apply functions ([`filter_by_ordinals`], [`filter_by_values`])
//! check this flag and let NULL rows through instead of discarding them.
//!
//! # Column Resolution
//!
//! Dynamic filters from parent operators reference columns by the *parent's* schema
//! indices, which may differ from the scan's field order (e.g. after projections or
//! joins). [`collect_filters`] resolves columns by **name** against the scan's schema
//! so that the correct fast-field index is used regardless of plan-level reordering.
//!
//! # Observability
//!
//! `EXPLAIN (ANALYZE)` on a `PgSearchScan` node shows `rows_pruned` and
//! `rows_scanned` metrics when dynamic filters are active. `rows_pruned > 0`
//! confirms that pre-filtering is working. The `dynamic_filters=N` annotation
//! in the non-ANALYZE plan shows how many filters were pushed down.

use std::ops::Bound;
use std::sync::Arc;

use arrow_schema::SchemaRef;
use datafusion::arrow::array::{Array, ArrayRef, BooleanArray};
use datafusion::arrow::compute::is_null;
use datafusion::arrow::compute::kernels::boolean;
use datafusion::arrow::compute::kernels::cmp;
use datafusion::arrow::compute::kernels::zip;
use datafusion::common::ScalarValue;
use datafusion::logical_expr::Operator;
use datafusion::physical_expr::expressions::{BinaryExpr, Column, IsNullExpr, Literal};
use datafusion::physical_expr::PhysicalExpr;
use tantivy::columnar::BytesColumn;
use tantivy::SegmentOrdinal;

use crate::index::fast_fields_helper::{FFHelper, FFType};

/// A pre-materialization filter applied inside [`Scanner::next()`](super::batch_scanner::Scanner::next)
/// between visibility checks and column materialization. By filtering at the
/// term-ordinal or fast-field level, we skip expensive term dictionary I/O for
/// pruned documents.
pub struct PreFilter {
    /// Index into `which_fast_fields` (== schema field index == ff_index).
    pub ff_index: usize,
    /// Lower bound of the accepted range.
    pub lower: Bound<PreFilterValue>,
    /// Upper bound of the accepted range.
    pub upper: Bound<PreFilterValue>,
    /// When true, rows with NULL values for this column pass the filter.
    /// This is needed for TopK dynamic filters which produce
    /// `col IS NULL OR col < threshold`.
    pub nulls_pass: bool,
}

/// A typed threshold value for pre-materialization filtering.
pub enum PreFilterValue {
    /// Raw bytes for Text/Bytes columns — converted to a term ordinal per-segment.
    Bytes(Vec<u8>),
    /// 64-bit signed integer.
    I64(i64),
    /// 64-bit float.
    F64(f64),
    /// 64-bit unsigned integer.
    U64(u64),
}

impl PreFilterValue {
    fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(b) => Some(b),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Self::I64(v) => Some(*v),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            Self::F64(v) => Some(*v),
            _ => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            Self::U64(v) => Some(*v),
            _ => None,
        }
    }
}

impl PreFilter {
    /// Apply the filter to an Arrow array, returning a boolean mask.
    ///
    /// This method does *not* filter the array in-place. Instead, it returns a
    /// `BooleanArray` mask where `true` indicates values that satisfy the filter
    /// and `false` indicates values that should be pruned.
    pub fn apply_arrow(
        &self,
        ffhelper: &FFHelper,
        segment_ord: SegmentOrdinal,
        array: &ArrayRef,
    ) -> Result<BooleanArray, String> {
        let col = ffhelper.column(segment_ord, self.ff_index);
        match col {
            FFType::Text(col) => self.apply_arrow_ordinals(col, array),
            FFType::Bytes(col) => self.apply_arrow_ordinals(col, array),
            FFType::I64(_) => {
                let lower = map_bound(&self.lower, |v: &PreFilterValue| {
                    v.as_i64().map(|v| ScalarValue::Int64(Some(v)))
                })
                .ok_or("Failed to map lower bound for I64")?;
                let upper = map_bound(&self.upper, |v: &PreFilterValue| {
                    v.as_i64().map(|v| ScalarValue::Int64(Some(v)))
                })
                .ok_or("Failed to map upper bound for I64")?;
                apply_arrow_bounds(array, lower, upper, self.nulls_pass)
            }
            FFType::F64(_) => {
                let lower = map_bound(&self.lower, |v: &PreFilterValue| {
                    v.as_f64().map(|v| ScalarValue::Float64(Some(v)))
                })
                .ok_or("Failed to map lower bound for F64")?;
                let upper = map_bound(&self.upper, |v: &PreFilterValue| {
                    v.as_f64().map(|v| ScalarValue::Float64(Some(v)))
                })
                .ok_or("Failed to map upper bound for F64")?;
                apply_arrow_bounds(array, lower, upper, self.nulls_pass)
            }
            FFType::U64(_) => {
                let lower = map_bound(&self.lower, |v: &PreFilterValue| {
                    v.as_u64().map(|v| ScalarValue::UInt64(Some(v)))
                })
                .ok_or("Failed to map lower bound for U64")?;
                let upper = map_bound(&self.upper, |v: &PreFilterValue| {
                    v.as_u64().map(|v| ScalarValue::UInt64(Some(v)))
                })
                .ok_or("Failed to map upper bound for U64")?;
                apply_arrow_bounds(array, lower, upper, self.nulls_pass)
            }
            // TODO: Support Bool and Date column types here as well.
            _ => Ok(BooleanArray::from(vec![true; array.len()])),
        }
    }

    fn apply_arrow_ordinals(
        &self,
        col: &BytesColumn,
        array: &ArrayRef,
    ) -> Result<BooleanArray, String> {
        let lower = map_bound(&self.lower, PreFilterValue::as_bytes)
            .ok_or("Failed to map lower bound for Bytes")?;
        let upper = map_bound(&self.upper, PreFilterValue::as_bytes)
            .ok_or("Failed to map upper bound for Bytes")?;

        let (lo_ord, hi_ord) = col
            .dictionary()
            .term_bounds_to_ord(lower, upper)
            .map_err(|e| format!("Failed to lookup term bounds: {e}"))?;

        let lo_scalar = match lo_ord {
            Bound::Included(v) => Bound::Included(ScalarValue::UInt64(Some(v))),
            Bound::Excluded(v) => Bound::Excluded(ScalarValue::UInt64(Some(v))),
            Bound::Unbounded => Bound::Unbounded,
        };
        let hi_scalar = match hi_ord {
            Bound::Included(v) => Bound::Included(ScalarValue::UInt64(Some(v))),
            Bound::Excluded(v) => Bound::Excluded(ScalarValue::UInt64(Some(v))),
            Bound::Unbounded => Bound::Unbounded,
        };

        apply_arrow_bounds(array, lo_scalar, hi_scalar, self.nulls_pass)
    }
}

/// Apply bounds to an Arrow array using compute kernels.
fn apply_arrow_bounds(
    array: &ArrayRef,
    lower: Bound<ScalarValue>,
    upper: Bound<ScalarValue>,
    nulls_pass: bool,
) -> Result<BooleanArray, String> {
    let lower_mask = match lower {
        Bound::Included(val) => cmp::gt_eq(array, &val.to_scalar().map_err(|e| e.to_string())?),
        Bound::Excluded(val) => cmp::gt(array, &val.to_scalar().map_err(|e| e.to_string())?),
        Bound::Unbounded => Ok(BooleanArray::from(vec![true; array.len()])),
    }
    .map_err(|e| format!("Failed to apply lower bound: {e}"))?;

    let upper_mask = match upper {
        Bound::Included(val) => cmp::lt_eq(array, &val.to_scalar().map_err(|e| e.to_string())?),
        Bound::Excluded(val) => cmp::lt(array, &val.to_scalar().map_err(|e| e.to_string())?),
        Bound::Unbounded => Ok(BooleanArray::from(vec![true; array.len()])),
    }
    .map_err(|e| format!("Failed to apply upper bound: {e}"))?;

    let mask = boolean::and(&lower_mask, &upper_mask)
        .map_err(|e| format!("Failed to combine bounds: {e}"))?;
    if mask.null_count() == 0 {
        return Ok(mask);
    }

    // Handle NULLs: replace them with `nulls_pass` (true or false) so the result is fully valid.
    let null_mask = is_null(array).map_err(|e| format!("Failed to check nulls: {e}"))?;
    let replacement = ScalarValue::Boolean(Some(nulls_pass))
        .to_array_of_size(array.len())
        .map_err(|e| format!("Failed to create replacement array: {e}"))?;

    // If input is NULL (null_mask is true), use replacement (nulls_pass).
    // Otherwise use the comparison result.
    let result = zip::zip(&null_mask, &replacement, &mask)
        .map_err(|e| format!("Failed to zip nulls: {e}"))?;

    let result_bool = result
        .as_any()
        .downcast_ref::<BooleanArray>()
        .ok_or_else(|| "Failed to downcast zip result to BooleanArray".to_string())?
        .clone();

    Ok(result_bool)
}

/// Map the inner value of a `Bound`.
fn map_bound<'a, T, U>(
    bound: &'a Bound<T>,
    f: impl FnOnce(&'a T) -> Option<U>,
) -> Option<Bound<U>> {
    match bound {
        Bound::Included(v) => f(v).map(Bound::Included),
        Bound::Excluded(v) => f(v).map(Bound::Excluded),
        Bound::Unbounded => Some(Bound::Unbounded),
    }
}

// ============================================================================
// PhysicalExpr → PreFilter decomposition
// ============================================================================

/// Recursively decompose a `PhysicalExpr` into `PreFilter`s.
///
/// Handles:
/// - `BinaryExpr(Column, Lt/LtEq/Gt/GtEq, Literal)` and the reversed form
/// - `BinaryExpr(left, And, right)` — recurses into both children
/// - `BinaryExpr(IsNull(col), Or, comparison)` — TopK dynamic filter pattern
/// - Anything else (including `Literal(true)`) is silently skipped.
///
/// The `schema` parameter is used for column name resolution. Dynamic filters
/// from parent operators (e.g. TopK above a SortMergeJoin) reference columns
/// by the parent's schema indices, which may differ from the scan's schema.
/// Name-based lookup ensures the correct fast field index is used.
pub fn collect_filters(expr: &dyn PhysicalExpr, schema: &SchemaRef, out: &mut Vec<PreFilter>) {
    if let Some(binary) = expr.as_any().downcast_ref::<BinaryExpr>() {
        let op = binary.op();

        // Handle AND: recurse into both children.
        if matches!(op, Operator::And) {
            collect_filters(binary.left().as_ref(), schema, out);
            collect_filters(binary.right().as_ref(), schema, out);
            return;
        }

        // Handle OR with IS NULL — TopK dynamic filters produce
        // `col IS NULL OR col < threshold`. Extract the comparison
        // and set nulls_pass=true so NULLs are not incorrectly pruned.
        if matches!(op, Operator::Or) {
            if let Some(pf) = try_or_is_null_pattern(binary, schema) {
                out.push(pf);
                return;
            }
        }

        // Try Column op Literal
        if let Some(pf) = try_column_op_literal(binary.left(), op, binary.right(), schema, false) {
            out.push(pf);
            return;
        }

        // Try Literal op Column (reversed)
        if let Some(reversed_op) = flip_operator(op) {
            if let Some(pf) =
                try_column_op_literal(binary.right(), &reversed_op, binary.left(), schema, false)
            {
                out.push(pf);
            }
        }
    }
}

/// Try to match the TopK pattern `IsNull(col) OR col op Literal`.
/// Returns a PreFilter with `nulls_pass=true` if matched.
fn try_or_is_null_pattern(binary: &BinaryExpr, schema: &SchemaRef) -> Option<PreFilter> {
    let (is_null_side, comparison_side) = if binary
        .left()
        .as_any()
        .downcast_ref::<IsNullExpr>()
        .is_some()
    {
        (binary.left(), binary.right())
    } else if binary
        .right()
        .as_any()
        .downcast_ref::<IsNullExpr>()
        .is_some()
    {
        (binary.right(), binary.left())
    } else {
        return None;
    };

    let is_null = is_null_side.as_any().downcast_ref::<IsNullExpr>()?;
    let is_null_col = is_null.arg().as_any().downcast_ref::<Column>()?;

    // The comparison side must be a simple BinaryExpr(Column op Literal).
    let cmp = comparison_side.as_any().downcast_ref::<BinaryExpr>()?;
    let op = cmp.op();

    // Extract the comparison column and verify it matches the IS NULL column.
    let cmp_col = cmp
        .left()
        .as_any()
        .downcast_ref::<Column>()
        .or_else(|| cmp.right().as_any().downcast_ref::<Column>())?;
    if is_null_col.name() != cmp_col.name() {
        return None;
    }

    // Try Column op Literal
    if let Some(pf) = try_column_op_literal(cmp.left(), op, cmp.right(), schema, true) {
        return Some(pf);
    }

    // Try Literal op Column (reversed)
    if let Some(reversed_op) = flip_operator(op) {
        if let Some(pf) = try_column_op_literal(cmp.right(), &reversed_op, cmp.left(), schema, true)
        {
            return Some(pf);
        }
    }

    None
}

/// Try to build a `PreFilter` from `Column op Literal`.
///
/// Uses `schema` for name-based column resolution so that filters from parent
/// operators (whose column indices differ from the scan's) are correctly mapped.
fn try_column_op_literal(
    left: &Arc<dyn PhysicalExpr>,
    op: &Operator,
    right: &Arc<dyn PhysicalExpr>,
    schema: &SchemaRef,
    nulls_pass: bool,
) -> Option<PreFilter> {
    let col = left.as_any().downcast_ref::<Column>()?;
    let lit = right.as_any().downcast_ref::<Literal>()?;
    let value = scalar_to_pre_filter_value(lit.value())?;

    // Resolve the column index using the scan's schema by name.
    // This handles cross-plan filters where the column index in the expression
    // (from a parent operator's schema) doesn't match the scan's field order.
    let ff_index = schema.column_with_name(col.name())?.0;

    let (lower, upper) = match op {
        Operator::Lt => (Bound::Unbounded, Bound::Excluded(value)),
        Operator::LtEq => (Bound::Unbounded, Bound::Included(value)),
        Operator::Gt => (Bound::Excluded(value), Bound::Unbounded),
        Operator::GtEq => (Bound::Included(value), Bound::Unbounded),
        _ => return None,
    };

    Some(PreFilter {
        ff_index,
        lower,
        upper,
        nulls_pass,
    })
}

/// Flip a comparison operator so that `Literal op Column` becomes `Column flipped_op Literal`.
fn flip_operator(op: &Operator) -> Option<Operator> {
    match op {
        Operator::Lt => Some(Operator::Gt),
        Operator::LtEq => Some(Operator::GtEq),
        Operator::Gt => Some(Operator::Lt),
        Operator::GtEq => Some(Operator::LtEq),
        _ => None,
    }
}

/// Convert a DataFusion `ScalarValue` to a `PreFilterValue`.
fn scalar_to_pre_filter_value(scalar: &ScalarValue) -> Option<PreFilterValue> {
    match scalar {
        ScalarValue::Utf8(Some(s))
        | ScalarValue::Utf8View(Some(s))
        | ScalarValue::LargeUtf8(Some(s)) => Some(PreFilterValue::Bytes(s.as_bytes().to_vec())),
        ScalarValue::Int64(Some(v)) => Some(PreFilterValue::I64(*v)),
        ScalarValue::Int32(Some(v)) => Some(PreFilterValue::I64(*v as i64)),
        ScalarValue::Int16(Some(v)) => Some(PreFilterValue::I64(*v as i64)),
        ScalarValue::Float64(Some(v)) => Some(PreFilterValue::F64(*v)),
        ScalarValue::Float32(Some(v)) => Some(PreFilterValue::F64(*v as f64)),
        ScalarValue::UInt64(Some(v)) => Some(PreFilterValue::U64(*v)),
        ScalarValue::UInt32(Some(v)) => Some(PreFilterValue::U64(*v as u64)),
        _ => None,
    }
}
