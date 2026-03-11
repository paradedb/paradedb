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

//! Per-segment streaming Top K with ordinal pruning.
//!
//! `SegmentedTopKExec` sits between `TantivyLookupExec` and its child in the
//! physical plan. It operates on the 3-way deferred `UnionArray` columns emitted
//! by late materialization:
//!   - State 0 (doc_address): unpacks `(segment_ord, doc_id)` and bulk-fetches
//!     term ordinals via `FFHelper`.
//!   - State 1 (term_ordinals): uses ordinals directly (already resolved by
//!     pre-filter memoization).
//!   - State 2 (materialized): already-decoded strings/bytes — always kept.
//!
//! For States 0 and 1, a per-segment bounded heap of size K retains only the
//! top rows per segment. When a segment boundary is detected, the completed
//! segment's survivors are emitted immediately to `TantivyLookupExec`.
//!
//! ## Output bound
//!
//! The cutoff for each segment is the worst (K-th best) `OwnedRow` in that
//! segment's heap. All rows with `OwnedRow <= cutoff` survive. When sort keys
//! are unique, this is exactly K rows per segment. With ties at the boundary,
//! all tied rows are conservatively retained:
//!
//!   survivors_s = K + (T_s - H_s)
//!
//! where `T_s` is the total number of rows in segment `s` sharing the cutoff
//! value, and `H_s` is how many of those occupy heap slots (`H_s >= 1`).
//! Total ordinal-comparable rows reaching `TantivyLookupExec`:
//!
//!   sum_s(survivors_s) <= K * S  (when no boundary ties)
//!
//! where `S` is the number of segments. Pass-through rows (State 2 and NULL
//! ordinals) are emitted immediately and are not bounded by K.
//!
//! ## Streaming and the TopK feedback loop
//!
//! The node uses `EmissionType::Both` — pass-through rows are yielded
//! immediately, and ordinal-comparable survivors are yielded at each segment
//! boundary. This incremental emission is critical: `SortExec(TopK)` receives
//! rows early and tightens its `DynamicFilterPhysicalExpr`, which is pushed
//! down to the scanner and prunes rows from later segments at scan level.
//! Without per-segment streaming, TopK receives zero rows until all input is
//! consumed and its dynamic filter never activates during scanning.
//!
//! **Compound sorts:** Only the primary sort column is used for ordinal
//! pruning. When the Top K sort has tiebreaker columns (e.g.
//! `ORDER BY val DESC, id ASC LIMIT 25`), all rows tied at the boundary
//! ordinal are retained — the exec cannot distinguish between them without
//! the tiebreaker, so it keeps them all for the final Top K to resolve.
//! This is safe (never drops correct rows) but slightly less aggressive
//! than theoretically possible when there are many duplicates.
//! TODO(https://github.com/paradedb/paradedb/issues/4255): rewrite the full
//! Top K sort expression in terms of term ordinals to handle tiebreakers
//! natively.

use crate::api::HashMap;
use crate::index::fast_fields_helper::{FFHelper, FFType, NULL_TERM_ORDINAL};
use crate::scan::deferred_encode::unpack_doc_address;
use crate::scan::execution_plan::UnsafeSendStream;
use arrow_array::{
    Array, ArrayRef, BooleanArray, RecordBatch, StructArray, UInt32Array, UInt64Array, UnionArray,
};
use arrow_schema::SchemaRef;
use arrow_select::concat::concat_batches;
use arrow_select::filter::filter_record_batch;
use datafusion::arrow::row::{OwnedRow, RowConverter, SortField};
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::{EquivalenceProperties, LexOrdering, PhysicalExpr};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::metrics::{
    Count, ExecutionPlanMetricsSet, MetricBuilder, MetricsSet,
};
use datafusion::physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties};
use std::any::Any;
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex};
use tantivy::termdict::TermOrdinal;
use tantivy::{DocId, SegmentOrdinal};

/// Shared per-segment ordinal thresholds, written by `SegmentedTopKExec`
/// and read by the scanner for early row pruning.
///
/// As the exec builds up its per-segment heaps, it publishes the "worst"
/// ordinal still in the Top K for each segment. The scanner can then skip
/// rows whose ordinal cannot beat that threshold, avoiding ctid lookups,
/// visibility checks, and dictionary materialisation.
///
/// TODO(https://github.com/paradedb/paradedb/issues/4257): Unify `SegmentedThresholds`
/// with the `DynamicFilterPhysicalExpr` infrastructure so that thresholds are pushed down
/// via the standard DataFusion filter push-down path rather than this side-channel.
pub struct SegmentedThresholds {
    /// segment_ord → threshold expression.
    inner: Mutex<HashMap<SegmentOrdinal, Arc<dyn PhysicalExpr>>>,
}

impl SegmentedThresholds {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::default()),
        }
    }

    pub fn set_threshold(&self, seg_ord: SegmentOrdinal, threshold: Arc<dyn PhysicalExpr>) {
        self.inner.lock().unwrap().insert(seg_ord, threshold);
    }

    pub fn get_threshold(&self, seg_ord: SegmentOrdinal) -> Option<Arc<dyn PhysicalExpr>> {
        self.inner.lock().unwrap().get(&seg_ord).cloned()
    }
}

#[derive(Clone, Debug)]
pub struct DeferredSortColumn {
    pub sort_col_idx: usize,
    pub ff_index: usize,
}

pub struct SegmentedTopKExec {
    input: Arc<dyn ExecutionPlan>,
    /// The sort expressions defining the Top K order.
    sort_exprs: LexOrdering,
    /// The deferred string/bytes columns that are part of the Top K order.
    deferred_columns: Vec<DeferredSortColumn>,
    /// FFHelper for Tantivy fast field access (shared with TantivyLookupExec).
    ffhelper: Arc<FFHelper>,
    /// Maximum rows to keep per segment.
    k: usize,
    /// Shared thresholds published back to the scanner for early pruning.
    thresholds: Arc<SegmentedThresholds>,
    properties: PlanProperties,
    metrics: ExecutionPlanMetricsSet,
}

impl std::fmt::Debug for SegmentedTopKExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sort_exprs_str = self
            .sort_exprs
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        f.debug_struct("SegmentedTopKExec")
            .field("expr", &sort_exprs_str)
            .field("k", &self.k)
            .field("deferred_columns", &self.deferred_columns)
            .finish()
    }
}

impl SegmentedTopKExec {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        input: Arc<dyn ExecutionPlan>,
        sort_exprs: LexOrdering,
        deferred_columns: Vec<DeferredSortColumn>,
        ffhelper: Arc<FFHelper>,
        k: usize,
        thresholds: Arc<SegmentedThresholds>,
    ) -> Self {
        let eq_props = EquivalenceProperties::new(input.schema());
        let properties = PlanProperties::new(
            eq_props,
            input.properties().output_partitioning().clone(),
            EmissionType::Both,
            Boundedness::Bounded,
        );
        Self {
            input,
            sort_exprs,
            deferred_columns,
            ffhelper,
            k,
            thresholds,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
        }
    }
}

impl DisplayAs for SegmentedTopKExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let sort_exprs_str = self
            .sort_exprs
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            "SegmentedTopKExec: expr=[{}], k={}",
            sort_exprs_str, self.k
        )
    }
}

impl ExecutionPlan for SegmentedTopKExec {
    fn name(&self) -> &str {
        "SegmentedTopKExec"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn properties(&self) -> &PlanProperties {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.input]
    }

    fn with_new_children(
        self: Arc<Self>,
        mut children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(Arc::new(SegmentedTopKExec::new(
            children.remove(0),
            self.sort_exprs.clone(),
            self.deferred_columns.clone(),
            Arc::clone(&self.ffhelper),
            self.k,
            Arc::clone(&self.thresholds),
        )))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let mut input_stream = self.input.execute(partition, context)?;
        let rows_input = MetricBuilder::new(&self.metrics).counter("rows_input", partition);
        let rows_output = MetricBuilder::new(&self.metrics).counter("rows_output", partition);
        let segments_seen = MetricBuilder::new(&self.metrics).counter("segments_seen", partition);

        // Build the row converter
        let sort_fields = self
            .sort_exprs
            .iter()
            .map(|expr| {
                let expr_type = expr
                    .expr
                    .data_type(self.properties.eq_properties.schema())?;
                // If it's a deferred column, we treat its sorting type as UInt64 (the ordinal type).
                let data_type = if expr
                    .expr
                    .as_any()
                    .downcast_ref::<datafusion::physical_expr::expressions::Column>()
                    .is_some_and(|c| {
                        self.deferred_columns
                            .iter()
                            .any(|d| d.sort_col_idx == c.index())
                    }) {
                    arrow_schema::DataType::UInt64
                } else {
                    expr_type
                };
                Ok(SortField::new_with_options(data_type, expr.options))
            })
            .collect::<Result<Vec<_>>>()?;

        let row_converter = RowConverter::new(sort_fields)?;

        let segments_flushed =
            MetricBuilder::new(&self.metrics).counter("segments_flushed", partition);

        let mut state = SegmentedTopKState {
            sort_exprs: self.sort_exprs.clone(),
            deferred_columns: self.deferred_columns.clone(),
            ffhelper: Arc::clone(&self.ffhelper),
            k: self.k,
            schema: self.properties.eq_properties.schema().clone(),
            row_converter,
            segment_heaps: HashMap::default(),
            thresholds: Arc::clone(&self.thresholds),
            batches: Vec::new(),
            row_ordinals: Vec::new(),
            current_segment: None,
            rows_input,
            rows_output,
            segments_seen,
            segments_flushed,
        };

        let stream_gen = async_stream::try_stream! {
            use futures::StreamExt;
            while let Some(batch_res) = input_stream.next().await {
                let batch = batch_res?;
                state.rows_input.add(batch.num_rows());

                // Detect segment boundary before ingesting batch.
                let batch_seg = state.detect_batch_segment(&batch)?;
                if let Some(new_seg) = batch_seg {
                    if let Some(prev_seg) = state.current_segment {
                        if prev_seg != new_seg {
                            // Previous segment complete — flush its survivors.
                            for fb in state.flush_segment(prev_seg)? {
                                if fb.num_rows() > 0 {
                                    state.rows_output.add(fb.num_rows());
                                    yield fb;
                                }
                            }
                            // With lazy scans, segments are sequential so all
                            // buffered batches belong to the just-flushed segment.
                            // Clear them to avoid quadratic re-scanning in
                            // subsequent flush_segment calls.
                            if state.row_ordinals.is_empty() {
                                state.batches.clear();
                            }
                        }
                    }
                    state.current_segment = Some(new_seg);
                }

                let batch_idx = state.batches.len();
                match state.collect_batch(&batch, batch_idx)? {
                    Some(pass_through) => {
                        state.batches.push(batch);
                        state.maybe_compact();
                        state.rows_output.add(pass_through.num_rows());
                        yield pass_through;
                    }
                    None => {
                        state.batches.push(batch);
                        state.maybe_compact();
                    }
                }
            }

            // Flush all remaining segments. This handles both the final
            // segment in the clean-boundary case and any segments from
            // mixed-segment batches (e.g. HashJoin coalescing output).
            let remaining_segs: Vec<SegmentOrdinal> =
                state.segment_heaps.keys().cloned().collect();
            for seg in remaining_segs {
                for fb in state.flush_segment(seg)? {
                    if fb.num_rows() > 0 {
                        state.rows_output.add(fb.num_rows());
                        yield fb;
                    }
                }
            }
        };

        // SAFETY: pg_search operates in a single-threaded Tokio executor within Postgres.
        let stream = unsafe {
            UnsafeSendStream::new(stream_gen, self.properties.eq_properties.schema().clone())
        };
        Ok(Box::pin(stream))
    }

    fn metrics(&self) -> Option<MetricsSet> {
        Some(self.metrics.clone_inner())
    }
}

struct SegmentedTopKState {
    sort_exprs: LexOrdering,
    deferred_columns: Vec<DeferredSortColumn>,
    ffhelper: Arc<FFHelper>,
    k: usize,
    schema: SchemaRef,
    row_converter: RowConverter,
    /// Per-segment max-heaps of comparable Rows. We maintain max heaps so that
    /// the 'worst' element (the boundary) is always at the root. We also store the
    /// `(batch_idx, row_idx)` to allow for compaction.
    segment_heaps: HashMap<SegmentOrdinal, BinaryHeap<OwnedRow>>,
    /// Shared thresholds published back to the scanner for early pruning.
    thresholds: Arc<SegmentedThresholds>,
    /// Buffered batches during the collection phase.
    batches: Vec<RecordBatch>,
    /// Keeps track of the heap rows for compaction.
    /// (batch_idx, row_idx, seg_ord, row_data)
    row_ordinals: Vec<(usize, usize, SegmentOrdinal, OwnedRow)>,

    /// The segment ordinal currently being accumulated.
    current_segment: Option<SegmentOrdinal>,

    rows_input: Count,
    rows_output: Count,
    /// Counts segments that had rows participating in ordinal comparison (States 0+1).
    /// Segments with only State 2 (materialized) or only NULLs are not counted.
    segments_seen: Count,
    segments_flushed: Count,
}

impl SegmentedTopKState {
    /// Update the per-segment cutoff heap with a new ordinal. The heap tracks
    /// the K best transformed ordinals to determine the boundary. Row locations
    /// are tracked separately in `row_ordinals`.
    fn update_cutoff_heap(heap: &mut BinaryHeap<OwnedRow>, heap_val: OwnedRow, k: usize) {
        if heap.len() < k {
            heap.push(heap_val);
        } else if let Some(worst) = heap.peek() {
            if &heap_val < worst {
                heap.pop();
                heap.push(heap_val);
            }
        }
    }

    /// Ingest a single batch: extract ordinals, update per-segment heaps,
    /// and publish thresholds. The batch is buffered for the final emission
    /// phase. Returns a pass-through batch of rows that bypass ordinal
    /// comparison (State 2 and NULL ordinals) so they can be emitted
    /// immediately.
    fn collect_batch(
        &mut self,
        batch: &RecordBatch,
        batch_idx: usize,
    ) -> Result<Option<RecordBatch>> {
        let num_rows = batch.num_rows();
        let mut pass_through = vec![false; num_rows];
        let mut row_to_seg = vec![None; num_rows];
        let mut deferred_ords: HashMap<usize, Vec<Option<TermOrdinal>>> = HashMap::default();

        for deferred_col in &self.deferred_columns {
            let global_term_ords = self.extract_deferred_ordinals(
                batch,
                deferred_col,
                num_rows,
                &mut pass_through,
                &mut row_to_seg,
            )?;
            deferred_ords.insert(deferred_col.sort_col_idx, global_term_ords);
        }

        // Build the evaluation arrays for the RowConverter
        let mut sort_arrays = Vec::with_capacity(self.sort_exprs.len());
        for expr in &self.sort_exprs {
            let col_idx = expr
                .expr
                .as_any()
                .downcast_ref::<datafusion::physical_expr::expressions::Column>()
                .map(|c| c.index());

            if let Some(Some(ords)) = col_idx.map(|idx| deferred_ords.get(&idx)) {
                // Use our artificially constructed ordinals array
                let ords_array = Arc::new(UInt64Array::from(ords.clone())) as ArrayRef;
                sort_arrays.push(ords_array);
            } else {
                let val = expr.expr.evaluate(batch)?;
                sort_arrays.push(val.into_array(num_rows)?);
            }
        }

        let converted_rows = self.row_converter.convert_columns(&sort_arrays)?;

        for row_idx in 0..num_rows {
            if pass_through[row_idx] {
                continue;
            }
            if let Some(seg_ord) = row_to_seg[row_idx] {
                if !self.segment_heaps.contains_key(&seg_ord) {
                    self.segments_seen.add(1);
                }
                let heap = self.segment_heaps.entry(seg_ord).or_default();

                let heap_val = converted_rows.row(row_idx).owned();
                Self::update_cutoff_heap(heap, heap_val.clone(), self.k);
                self.row_ordinals
                    .push((batch_idx, row_idx, seg_ord, heap_val));
            }
        }

        // Publish thresholds for segments with full heaps.
        for (seg_ord, heap) in &self.segment_heaps {
            if heap.len() >= self.k {
                if let Some(worst) = heap.peek() {
                    // Extract the values from the worst row
                    let arrays = self
                        .row_converter
                        .convert_rows(std::iter::once(worst.row()))?;
                    if let Some(expr) = self.build_filter_expression(&arrays) {
                        self.thresholds.set_threshold(*seg_ord, expr);
                    }
                }
            }
        }

        // Emit pass-through rows (State 2 + NULL ordinals) immediately.
        if pass_through.iter().any(|&b| b) {
            let mask: BooleanArray = pass_through.into_iter().map(Some).collect();
            let filtered = filter_record_batch(batch, &mask)
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
            if filtered.num_rows() > 0 {
                return Ok(Some(filtered));
            }
        }

        Ok(None)
    }

    /// Helper to extract term ordinals from a deferred UnionArray.
    /// Mutates `pass_through` for rows that contain State 2 or NULLs, and populates `row_to_seg` mapping.
    fn extract_deferred_ordinals(
        &self,
        batch: &RecordBatch,
        deferred_col: &DeferredSortColumn,
        num_rows: usize,
        pass_through: &mut [bool],
        row_to_seg: &mut [Option<SegmentOrdinal>],
    ) -> Result<Vec<Option<TermOrdinal>>> {
        let union_col = batch
            .column(deferred_col.sort_col_idx)
            .as_any()
            .downcast_ref::<UnionArray>()
            .ok_or_else(|| {
                DataFusionError::Internal(
                    "SegmentedTopKExec: sort column should be a deferred UnionArray".into(),
                )
            })?;

        let type_ids = union_col.type_ids();
        let offsets = union_col.offsets().ok_or_else(|| {
            DataFusionError::Internal("SegmentedTopKExec: expected dense union with offsets".into())
        })?;

        let mut state0_rows: Vec<usize> = Vec::new();
        let mut state1_rows: Vec<usize> = Vec::new();
        for row_idx in 0..num_rows {
            match type_ids[row_idx] {
                0 => state0_rows.push(row_idx),
                1 => state1_rows.push(row_idx),
                2 => pass_through[row_idx] = true,
                _ => unreachable!("Invalid Union state"),
            }
        }

        let mut global_term_ords: Vec<Option<TermOrdinal>> = vec![None; num_rows];

        // State 0: compact doc address child.
        let mut state0_by_seg: HashMap<SegmentOrdinal, Vec<(usize, DocId)>> = HashMap::default();
        if !state0_rows.is_empty() {
            let doc_addr_child = union_col
                .child(0)
                .as_any()
                .downcast_ref::<UInt64Array>()
                .ok_or_else(|| {
                    DataFusionError::Internal(
                        "SegmentedTopKExec: child 0 should be UInt64 doc addresses".into(),
                    )
                })?;
            for &row_idx in &state0_rows {
                let packed = doc_addr_child.value(offsets[row_idx] as usize);
                let (seg_ord, doc_id) = unpack_doc_address(packed);
                state0_by_seg
                    .entry(seg_ord)
                    .or_default()
                    .push((row_idx, doc_id));
                row_to_seg[row_idx] = Some(seg_ord);
            }
        }

        // State 1: compact term ordinal child.
        if !state1_rows.is_empty() {
            let term_ord_child = union_col
                .child(1)
                .as_any()
                .downcast_ref::<StructArray>()
                .ok_or_else(|| {
                    DataFusionError::Internal(
                        "SegmentedTopKExec: child 1 should be StructArray of term ordinals".into(),
                    )
                })?;
            let seg_ord_array = term_ord_child
                .column(0)
                .as_any()
                .downcast_ref::<UInt32Array>()
                .ok_or_else(|| {
                    DataFusionError::Internal(
                        "SegmentedTopKExec: term_ordinal.segment_ord should be UInt32".into(),
                    )
                })?;
            let ord_array = term_ord_child
                .column(1)
                .as_any()
                .downcast_ref::<UInt64Array>()
                .ok_or_else(|| {
                    DataFusionError::Internal(
                        "SegmentedTopKExec: term_ordinal.term_ord should be UInt64".into(),
                    )
                })?;

            for &row_idx in &state1_rows {
                let ci = offsets[row_idx] as usize;
                let seg_ord = seg_ord_array.value(ci);
                row_to_seg[row_idx] = Some(seg_ord);
                if !ord_array.is_null(ci) {
                    let ord = ord_array.value(ci);
                    if ord != NULL_TERM_ORDINAL {
                        global_term_ords[row_idx] = Some(ord);
                    } else {
                        pass_through[row_idx] = true;
                    }
                } else {
                    pass_through[row_idx] = true;
                }
            }
        }

        // Bulk-fetch term ordinals for State 0 rows via FFHelper
        for (seg_ord, rows) in state0_by_seg {
            let doc_ids: Vec<DocId> = rows.iter().map(|(_, doc_id)| *doc_id).collect();
            let mut term_ords: Vec<Option<TermOrdinal>> = vec![None; doc_ids.len()];

            let col = self.ffhelper.column(seg_ord, deferred_col.ff_index);
            match col {
                FFType::Text(str_col) => {
                    str_col.ords().first_vals(&doc_ids, &mut term_ords);
                }
                FFType::Bytes(bytes_col) => {
                    bytes_col.ords().first_vals(&doc_ids, &mut term_ords);
                }
                _ => {
                    panic!(
                            "SegmentedTopKExec: ff_index {} is not a Text or Bytes dictionary column \
                             — the optimizer should never plan this node for non-dictionary columns",
                            deferred_col.ff_index
                        );
                }
            }

            for (i, (row_idx, _)) in rows.into_iter().enumerate() {
                let ord = term_ords[i].unwrap_or(NULL_TERM_ORDINAL);
                if ord == NULL_TERM_ORDINAL {
                    pass_through[row_idx] = true;
                } else {
                    global_term_ords[row_idx] = Some(ord);
                }
            }
        }

        Ok(global_term_ords)
    }
    /// Build a per-segment filter from the worst heap entry's ordinal arrays.
    ///
    /// The deferred columns remain as UInt64 ordinals (valid only within the
    /// segment). The pre-filter evaluates them directly against fast-field
    /// ordinals, skipping dictionary materialization.
    fn build_filter_expression(
        &self,
        threshold_arrays: &[ArrayRef],
    ) -> Option<Arc<dyn PhysicalExpr>> {
        use datafusion::common::ScalarValue;

        let values: Vec<ScalarValue> = threshold_arrays
            .iter()
            .map(|a| ScalarValue::try_from_array(a, 0))
            .collect::<std::result::Result<_, _>>()
            .ok()?;
        Self::build_lexicographic_filter(&self.sort_exprs, &values)
    }

    /// Build a chained lexicographic filter expression from threshold values.
    ///
    /// For `ORDER BY a ASC, b ASC` with thresholds `(t_a, t_b)`, produces:
    ///   `a < t_a OR (a = t_a AND b < t_b)`
    ///
    /// Handles NULL semantics via IS NULL / IS NOT NULL based on NULLS FIRST/LAST.
    fn build_lexicographic_filter(
        sort_exprs: &LexOrdering,
        values: &[datafusion::common::ScalarValue],
    ) -> Option<Arc<dyn PhysicalExpr>> {
        use datafusion::logical_expr::Operator;
        use datafusion::physical_expr::expressions::{is_not_null, is_null, lit, BinaryExpr};

        let mut filters = Vec::with_capacity(values.len());
        let mut prev_eq: Option<Arc<dyn PhysicalExpr>> = None;

        for (sort_expr, value) in sort_exprs.iter().zip(values) {
            let col_expr = &sort_expr.expr;
            let op = if sort_expr.options.descending {
                Operator::Gt
            } else {
                Operator::Lt
            };

            let value_null = value.is_null();

            // col <op> threshold
            let comparison = Arc::new(BinaryExpr::new(
                Arc::clone(col_expr),
                op,
                lit(value.clone()),
            )) as Arc<dyn PhysicalExpr>;

            // Wrap with NULL handling.
            let filter = match (sort_expr.options.nulls_first, value_null) {
                (true, true) => lit(false),
                (true, false) => {
                    let is_null_expr = is_null(Arc::clone(col_expr)).ok()?;
                    Arc::new(BinaryExpr::new(is_null_expr, Operator::Or, comparison))
                        as Arc<dyn PhysicalExpr>
                }
                (false, true) => is_not_null(Arc::clone(col_expr)).ok()?,
                (false, false) => comparison,
            };

            // col = threshold (for tiebreaker chaining).
            let mut eq_expr = Arc::new(BinaryExpr::new(
                Arc::clone(col_expr),
                Operator::Eq,
                lit(value.clone()),
            )) as Arc<dyn PhysicalExpr>;
            if value_null {
                let is_null_expr = is_null(Arc::clone(col_expr)).ok()?;
                eq_expr = Arc::new(BinaryExpr::new(is_null_expr, Operator::Or, eq_expr));
            }

            // Chain: first column stands alone; subsequent columns are
            // gated by "all prior columns equal their thresholds".
            match prev_eq.take() {
                None => {
                    filters.push(filter);
                }
                Some(p) => {
                    filters.push(Arc::new(BinaryExpr::new(
                        Arc::clone(&p),
                        Operator::And,
                        filter,
                    )));
                    eq_expr = Arc::new(BinaryExpr::new(p, Operator::And, eq_expr));
                }
            }
            prev_eq = Some(eq_expr);
        }

        filters
            .into_iter()
            .reduce(|a, b| Arc::new(BinaryExpr::new(a, Operator::Or, b)) as Arc<dyn PhysicalExpr>)
    }

    /// Downcast a batch column to a dense `UnionArray` and return it with its offsets.
    fn get_deferred_union(batch: &RecordBatch, col_idx: usize) -> Result<&UnionArray> {
        batch
            .column(col_idx)
            .as_any()
            .downcast_ref::<UnionArray>()
            .ok_or_else(|| {
                DataFusionError::Internal(
                    "SegmentedTopKExec: sort column should be a deferred UnionArray".into(),
                )
            })
    }

    /// Extract `segment_ord` from a single row of a deferred UnionArray.
    /// Returns `None` for State 2 (materialized) rows.
    fn segment_ord_from_union_row(
        union_col: &UnionArray,
        offsets: &[i32],
        row_idx: usize,
    ) -> Result<Option<SegmentOrdinal>> {
        match union_col.type_ids()[row_idx] {
            0 => {
                let packed = union_col
                    .child(0)
                    .as_any()
                    .downcast_ref::<UInt64Array>()
                    .expect("child 0 should be UInt64 doc addresses")
                    .value(offsets[row_idx] as usize);
                let (seg_ord, _) = unpack_doc_address(packed);
                Ok(Some(seg_ord))
            }
            1 => {
                let struct_child = union_col
                    .child(1)
                    .as_any()
                    .downcast_ref::<StructArray>()
                    .expect("child 1 should be StructArray");
                let seg_ord = struct_child
                    .column(0)
                    .as_any()
                    .downcast_ref::<UInt32Array>()
                    .expect("segment_ord should be UInt32")
                    .value(offsets[row_idx] as usize);
                Ok(Some(seg_ord))
            }
            2 => Ok(None),
            _ => unreachable!("Invalid Union state"),
        }
    }

    /// Peek at the first deferred column to find the segment this batch belongs to.
    /// Scans rows until it finds one that is not State 2 (materialized).
    fn detect_batch_segment(&self, batch: &RecordBatch) -> Result<Option<SegmentOrdinal>> {
        if self.deferred_columns.is_empty() {
            return Ok(None);
        }
        let union_col = Self::get_deferred_union(batch, self.deferred_columns[0].sort_col_idx)?;
        let offsets = union_col.offsets().ok_or_else(|| {
            DataFusionError::Internal("SegmentedTopKExec: expected dense union with offsets".into())
        })?;
        for row_idx in 0..batch.num_rows() {
            if let Some(seg) = Self::segment_ord_from_union_row(union_col, offsets, row_idx)? {
                return Ok(Some(seg));
            }
        }
        Ok(None)
    }

    /// Flush the completed segment's top-K survivors and update the global heap.
    fn flush_segment(&mut self, finished_seg: SegmentOrdinal) -> Result<Vec<RecordBatch>> {
        // 1. Partition row_ordinals: survivors for this segment vs rows from other segments.
        let survivors = self.collect_survivors(finished_seg);

        // 2. Filter buffered batches to keep only survivor rows.
        let result = self.filter_batches(&survivors)?;

        // 3. Remove the flushed segment's heap (no longer needed).
        self.segment_heaps.remove(&finished_seg);

        self.segments_flushed.add(1);
        Ok(result)
    }

    /// Partition `row_ordinals`: rows from `finished_seg` that beat the cutoff
    /// become survivors; rows from other segments are kept for later.
    fn collect_survivors(
        &mut self,
        finished_seg: SegmentOrdinal,
    ) -> crate::api::HashSet<(usize, usize)> {
        let cutoff = self
            .segment_heaps
            .get(&finished_seg)
            .and_then(|h| h.peek())
            .cloned();

        let mut survivors = crate::api::HashSet::default();
        let mut remaining = Vec::new();

        for (batch_idx, row_idx, seg_ord, heap_val) in std::mem::take(&mut self.row_ordinals) {
            if seg_ord != finished_seg {
                remaining.push((batch_idx, row_idx, seg_ord, heap_val));
            } else if cutoff.as_ref().is_none_or(|c| &heap_val <= c) {
                survivors.insert((batch_idx, row_idx));
            }
        }
        self.row_ordinals = remaining;
        survivors
    }

    /// Filter buffered batches to emit only the rows in `survivors`.
    fn filter_batches(
        &self,
        survivors: &crate::api::HashSet<(usize, usize)>,
    ) -> Result<Vec<RecordBatch>> {
        let mut result = Vec::new();
        for (batch_idx, batch) in self.batches.iter().enumerate() {
            let mask: BooleanArray = (0..batch.num_rows())
                .map(|row_idx| Some(survivors.contains(&(batch_idx, row_idx))))
                .collect();
            if mask.true_count() > 0 {
                let filtered = filter_record_batch(batch, &mask)
                    .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
                if filtered.num_rows() > 0 {
                    result.push(filtered);
                }
            }
        }
        Ok(result)
    }

    /// Compact buffered batches by discarding rows that cannot survive the
    /// current per-segment cutoffs. This bounds memory at O(K * segments)
    /// instead of O(N) for large inputs — analogous to the batch compaction
    /// step in upstream DataFusion Top K.
    fn maybe_compact(&mut self) {
        let num_segments = self.segment_heaps.len().max(1);
        if self.row_ordinals.len() <= self.k * num_segments * 4 {
            return;
        }

        // Determine which row_ordinals survive the current cutoffs.
        let mut new_row_ordinals = Vec::new();
        let mut survivors = crate::api::HashSet::default();

        // Use take() so we own row_ordinals and can move the OwnedRows.
        for (batch_idx, row_idx, seg_ord, heap_val) in std::mem::take(&mut self.row_ordinals) {
            let keep = match self.segment_heaps.get(&seg_ord).and_then(|h| h.peek()) {
                Some(cutoff_val) => &heap_val <= cutoff_val,
                None => true,
            };
            if keep {
                survivors.insert((batch_idx, row_idx));
                new_row_ordinals.push((batch_idx, row_idx, seg_ord, heap_val));
            }
        }

        // Don't compact if we wouldn't discard at least half the rows.
        if new_row_ordinals.len() * 2 > self.row_ordinals.capacity() {
            self.row_ordinals = new_row_ordinals;
            return;
        }

        // Filter each stored batch, build old→new row mapping, concatenate.
        let mut filtered_batches = Vec::new();
        let mut mapping: HashMap<(usize, usize), usize> = HashMap::default();
        let mut global_offset = 0usize;

        for (batch_idx, batch) in self.batches.iter().enumerate() {
            let mask: BooleanArray = (0..batch.num_rows())
                .map(|ri| Some(survivors.contains(&(batch_idx, ri))))
                .collect();

            if mask.true_count() == 0 {
                continue;
            }

            for ri in 0..batch.num_rows() {
                if survivors.contains(&(batch_idx, ri)) {
                    mapping.insert((batch_idx, ri), global_offset);
                    global_offset += 1;
                }
            }

            let filtered = filter_record_batch(batch, &mask).expect("compaction filter failed");
            filtered_batches.push(filtered);
        }

        if filtered_batches.is_empty() {
            self.batches.clear();
            self.row_ordinals.clear();
            return;
        }

        let compacted =
            concat_batches(&self.schema, &filtered_batches).expect("compaction concat failed");

        // Remap row_ordinals into the single compacted batch.
        for entry in &mut new_row_ordinals {
            let new_ri = mapping[&(entry.0, entry.1)];
            entry.0 = 0;
            entry.1 = new_ri;
        }

        self.row_ordinals = new_row_ordinals;
        self.batches = vec![compacted];
    }
}
