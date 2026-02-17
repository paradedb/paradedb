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
//! Dynamic filters (from TopK thresholds, HashJoin key bounds, etc.) are pushed down
//! into [`PgSearchScanPlan`] as `PhysicalExpr`s. At scan time they are decomposed into
//! [`PreFilter`]s and applied inside [`Scanner::next()`](super::batch_scanner::Scanner::next)
//! *before* column materialization — at the term-ordinal level for strings and direct
//! fast-field comparisons for numerics.

use std::ops::Bound;
use std::sync::Arc;

use arrow_schema::SchemaRef;
use datafusion::common::ScalarValue;
use datafusion::logical_expr::Operator;
use datafusion::physical_expr::expressions::{BinaryExpr, Column, IsNullExpr, Literal};
use datafusion::physical_expr::PhysicalExpr;
use tantivy::columnar::BytesColumn;
use tantivy::fastfield::Column as FFColumn;
use tantivy::SegmentOrdinal;

use crate::index::fast_fields_helper::{FFHelper, FFType};

// ============================================================================
// Types
// ============================================================================

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

    // Verify the IS NULL references a Column.
    let _is_null = is_null_side.as_any().downcast_ref::<IsNullExpr>()?;

    // The comparison side must be a simple BinaryExpr(Column op Literal).
    let cmp = comparison_side.as_any().downcast_ref::<BinaryExpr>()?;
    let op = cmp.op();

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

// ============================================================================
// Applying PreFilters during scanning
// ============================================================================

/// Apply a single pre-materialization filter, pruning `ids`/`ctids`/`scores` in-place.
///
/// For string columns this converts the threshold to a term ordinal and compares ordinals
/// (skipping the expensive dictionary walk for pruned docs). For numeric columns the fast
/// field values are compared directly.
pub fn apply_pre_filter(
    ffhelper: &FFHelper,
    segment_ord: SegmentOrdinal,
    filter: &PreFilter,
    ids: &mut Vec<u32>,
    ctids: &mut Vec<u64>,
    scores: &mut Vec<f32>,
) {
    let col = ffhelper.column(segment_ord, filter.ff_index);
    let nulls_pass = filter.nulls_pass;
    match col {
        FFType::Text(col) => filter_by_ordinals(col, filter, ids, ctids, scores),
        FFType::Bytes(col) => filter_by_ordinals(col, filter, ids, ctids, scores),
        FFType::I64(col) => {
            let Some(lo) = try_map_bound(&filter.lower, PreFilterValue::as_i64) else {
                return;
            };
            let Some(hi) = try_map_bound(&filter.upper, PreFilterValue::as_i64) else {
                return;
            };
            filter_by_values(col, lo, hi, nulls_pass, ids, ctids, scores);
        }
        FFType::F64(col) => {
            let Some(lo) = try_map_bound(&filter.lower, PreFilterValue::as_f64) else {
                return;
            };
            let Some(hi) = try_map_bound(&filter.upper, PreFilterValue::as_f64) else {
                return;
            };
            filter_by_values(col, lo, hi, nulls_pass, ids, ctids, scores);
        }
        FFType::U64(col) => {
            let Some(lo) = try_map_bound(&filter.lower, PreFilterValue::as_u64) else {
                return;
            };
            let Some(hi) = try_map_bound(&filter.upper, PreFilterValue::as_u64) else {
                return;
            };
            filter_by_values(col, lo, hi, nulls_pass, ids, ctids, scores);
        }
        // TODO: Support Bool and Date column types here as well.
        _ => {}
    }
}

/// Check whether `val` falls within the given bounds.
fn in_bound<T: PartialOrd>(val: T, lower: &Bound<T>, upper: &Bound<T>) -> bool {
    let lower_ok = match lower {
        Bound::Included(l) => val >= *l,
        Bound::Excluded(l) => val > *l,
        Bound::Unbounded => true,
    };
    let upper_ok = match upper {
        Bound::Included(u) => val <= *u,
        Bound::Excluded(u) => val < *u,
        Bound::Unbounded => true,
    };
    lower_ok && upper_ok
}

/// Compact `ids`, `ctids`, and `scores` in-place, keeping only elements
/// where `keep(index)` returns true.
fn compact_parallel(
    ids: &mut Vec<u32>,
    ctids: &mut Vec<u64>,
    scores: &mut Vec<f32>,
    keep: impl Fn(usize) -> bool,
) {
    let mut write_idx = 0;
    for read_idx in 0..ids.len() {
        if keep(read_idx) {
            if read_idx != write_idx {
                ids[write_idx] = ids[read_idx];
                ctids[write_idx] = ctids[read_idx];
                scores[write_idx] = scores[read_idx];
            }
            write_idx += 1;
        }
    }
    ids.truncate(write_idx);
    ctids.truncate(write_idx);
    scores.truncate(write_idx);
}

/// Map the inner value of a `Bound`, returning `None` if the mapping fails.
fn try_map_bound<'a, T, U>(
    bound: &'a Bound<T>,
    f: impl FnOnce(&'a T) -> Option<U>,
) -> Option<Bound<U>> {
    match bound {
        Bound::Included(v) => f(v).map(Bound::Included),
        Bound::Excluded(v) => f(v).map(Bound::Excluded),
        Bound::Unbounded => Some(Bound::Unbounded),
    }
}

/// Filter by dictionary-encoded column: convert bounds to term ordinals, load ordinals,
/// then compact. Works for both `StrColumn` (which derefs to `BytesColumn`) and
/// `BytesColumn` directly.
fn filter_by_ordinals(
    col: &BytesColumn,
    filter: &PreFilter,
    ids: &mut Vec<u32>,
    ctids: &mut Vec<u64>,
    scores: &mut Vec<f32>,
) {
    let Some(lower) = try_map_bound(&filter.lower, PreFilterValue::as_bytes) else {
        return;
    };
    let Some(upper) = try_map_bound(&filter.upper, PreFilterValue::as_bytes) else {
        return;
    };
    let Ok((lo_ord, hi_ord)) = col.dictionary().term_bounds_to_ord(lower, upper) else {
        return;
    };

    let nulls_pass = filter.nulls_pass;
    let mut ords = vec![None; ids.len()];
    col.ords().first_vals(ids, &mut ords);

    compact_parallel(ids, ctids, scores, |i| match ords[i] {
        Some(ord) => in_bound(ord, &lo_ord, &hi_ord),
        None => nulls_pass,
    });
}

/// Filter by numeric fast-field column: extract typed bounds, load values, then compact.
// TODO: Get Arrow arrays directly from `first_vals` so we can use Arrow compute kernels
// for filtering instead of the manual `compact_parallel` loop.
fn filter_by_values<T: PartialOrd + Copy + std::fmt::Debug + Send + Sync + 'static>(
    col: &FFColumn<T>,
    lower: Bound<T>,
    upper: Bound<T>,
    nulls_pass: bool,
    ids: &mut Vec<u32>,
    ctids: &mut Vec<u64>,
    scores: &mut Vec<f32>,
) {
    let mut vals = vec![None; ids.len()];
    col.first_vals(ids, &mut vals);

    compact_parallel(ids, ctids, scores, |i| match vals[i] {
        Some(v) => in_bound(v, &lower, &upper),
        None => nulls_pass,
    });
}
