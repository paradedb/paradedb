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
//! Dynamic filters allow parent operators (e.g. `SortExec(TopK)`) to push evolving
//! thresholds into scan nodes so that rows failing the threshold are pruned *before*
//! column materialization — at the term-ordinal level for strings and direct
//! fast-field comparisons for numerics. This is critical for `ORDER BY … LIMIT`
//! queries over joins: without it, the scan must materialize every row even though
//! only the Top K are needed.
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
//!                                      caps the scanner batch size so Top K can
//!                                      tighten its threshold between batches
//!        │
//!        │  at poll time
//!        ▼
//! ScanStream::collect_pre_filters    ← calls DynamicFilterPhysicalExpr::current()
//!   → collect_filters()                to get the latest threshold, decomposes
//!   → Vec<PreFilter>                   it into PreFilter(s)
//!        │
//!        ▼
//! Scanner::next()                    ← applies PreFilters via apply_arrow()
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
//! `EXPLAIN (ANALYZE)` on a `PgSearchScan` node shows `rows_pruned` and
//! `rows_scanned` metrics when dynamic filters are active. `rows_pruned > 0`
//! confirms that pre-filtering is working. The `dynamic_filters=N` annotation
//! in the non-ANALYZE plan shows how many filters were pushed down.

use std::ops::Bound;
use std::sync::Arc;

use arrow_schema::SchemaRef;
use datafusion::arrow::array::UInt64Array;
use datafusion::arrow::array::{Array, ArrayRef, BooleanArray};
use datafusion::arrow::compute::cast;
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::tree_node::{Transformed, TransformedResult, TreeNode};
use datafusion::common::ScalarValue;
use datafusion::logical_expr::Operator;
use datafusion::physical_expr::expressions::{
    BinaryExpr, CastExpr, Column, DynamicFilterPhysicalExpr, IsNullExpr, Literal, NotExpr,
};
use datafusion::physical_expr::PhysicalExpr;
use datafusion::physical_plan::expressions::InListExpr;
use datafusion::physical_plan::joins::HashTableLookupExpr;
use tantivy::SegmentOrdinal;

use crate::index::fast_fields_helper::{FFHelper, FFType, NULL_TERM_ORDINAL};

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
                if let Some(dyn_filter) = node.as_any().downcast_ref::<DynamicFilterPhysicalExpr>()
                {
                    let current_expr = dyn_filter.current().map_err(|e| {
                        datafusion::error::DataFusionError::Execution(format!(
                            "DynamicFilter error: {}",
                            e
                        ))
                    })?;
                    return Ok(Transformed::yes(current_expr));
                } else if let Some(cast) = node.as_any().downcast_ref::<CastExpr>() {
                    if cast.cast_type() == &cast.expr().data_type(schema)? {
                        return Ok(Transformed::yes(Arc::clone(cast.expr())));
                    }
                    return Ok(Transformed::no(node));
                } else if let Some(binary) = node.as_any().downcast_ref::<BinaryExpr>() {
                    if let Some(rewritten) =
                        try_rewrite_binary(binary, ffhelper, segment_ord, schema)?
                    {
                        return Ok(Transformed::yes(rewritten));
                    }
                } else if let Some(in_list) = node.as_any().downcast_ref::<InListExpr>() {
                    if let Some(rewritten) =
                        try_rewrite_in_list(in_list, ffhelper, segment_ord, schema)?
                    {
                        return Ok(Transformed::yes(rewritten));
                    }
                }
                Ok(Transformed::no(node))
            })
            .data()
            .map_err(|e| format!("Failed to rewrite string expr: {}", e))?;

        let rewritten_expr = rewritten_string_expr
            .transform(|node| {
                if let Some(col) = node.as_any().downcast_ref::<Column>() {
                    let orig_idx = col.index();
                    if orig_idx < schema.fields().len() {
                        if let Some(new_idx) = self
                            .required_columns
                            .iter()
                            .position(|&idx| idx == orig_idx)
                        {
                            let new_col = Column::new(col.name(), new_idx);
                            return Ok(
                                Transformed::yes(Arc::new(new_col) as Arc<dyn PhysicalExpr>),
                            );
                        }
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

            let schema_type = schema.field(ff_index).data_type();

            // Cast numeric fast fields to match the expected DataFusion schema type
            if !is_string_like_type(schema_type) && array.data_type() != schema_type {
                array = cast(&array, schema_type).map_err(|e| {
                    format!(
                        "Failed to cast Tantivy fast field from {:?} to DataFusion schema type {:?}: {}",
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
pub fn collect_filters(expr: &Arc<dyn PhysicalExpr>, schema: &SchemaRef, out: &mut Vec<PreFilter>) {
    // Split top-level ANDs to maximize early pruning
    if let Some(binary) = expr.as_any().downcast_ref::<BinaryExpr>() {
        if matches!(binary.op(), Operator::And) {
            collect_filters(binary.left(), schema, out);
            collect_filters(binary.right(), schema, out);
            return;
        }
    }

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

/// Helper to centrally identify string, bytes, dictionary, and deferred string columns.
fn is_string_like_type(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::Utf8
            | DataType::LargeUtf8
            | DataType::Utf8View
            | DataType::Binary
            | DataType::LargeBinary
            | DataType::BinaryView
            | DataType::Dictionary(_, _)
            | DataType::Union(_, _)
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
        let node_any = node.as_any();

        if let Some(col) = node_any.downcast_ref::<Column>() {
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
        } else if node_any.downcast_ref::<Literal>().is_some() {
            // Allowed
        } else if let Some(binary) = node_any.downcast_ref::<BinaryExpr>() {
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
        } else if node_any.downcast_ref::<IsNullExpr>().is_some()
            || node_any.downcast_ref::<NotExpr>().is_some()
            || node_any.downcast_ref::<InListExpr>().is_some()
        {
            // Allowed
        } else if node_any.downcast_ref::<HashTableLookupExpr>().is_some() {
            // We only support HashTableLookupExpr for non-string columns.
            let mut is_numeric = true;
            let mut lookup_columns = Vec::new();

            // We manually inspect the subtree to check the data types of the columns it uses
            let _ = node.apply(|sub_node| {
                if let Some(col) = sub_node.as_any().downcast_ref::<Column>() {
                    let idx = col.index();
                    if idx < schema.fields().len() {
                        let data_type = schema.field(idx).data_type();

                        if is_string_like_type(data_type) {
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
    let left_col = binary.left().as_any().downcast_ref::<Column>();
    let right_lit = binary.right().as_any().downcast_ref::<Literal>();

    if let (Some(col), Some(lit)) = (left_col, right_lit) {
        return rewrite_col_op_lit(col, binary.op(), lit, ffhelper, segment_ord, schema);
    }

    let left_lit = binary.left().as_any().downcast_ref::<Literal>();
    let right_col = binary.right().as_any().downcast_ref::<Column>();

    if let (Some(lit), Some(col)) = (left_lit, right_col) {
        if let Some(flipped_op) = flip_operator(binary.op()) {
            return rewrite_col_op_lit(col, &flipped_op, lit, ffhelper, segment_ord, schema);
        }
    }

    Ok(None)
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

        ScalarValue::Union(Some((_, boxed_val)), _, _) => extract_bytes_from_scalar(boxed_val),
        ScalarValue::Union(None, _, _) => Some(None),

        _ => None,
    }
}

fn try_rewrite_in_list(
    in_list: &InListExpr,
    ffhelper: &FFHelper,
    segment_ord: SegmentOrdinal,
    schema: &SchemaRef,
) -> datafusion::error::Result<Option<Arc<dyn PhysicalExpr>>> {
    let col = match in_list.expr().as_any().downcast_ref::<Column>() {
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
        let lit = match lit_expr.as_any().downcast_ref::<Literal>() {
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

    // Bypass schema validation entirely
    let new_in_list = InListExpr::try_new_from_array(new_col_expr, array, in_list.negated())
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
