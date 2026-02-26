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

//! Per-segment Top-K pruning using term ordinals.
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
//! top rows, reducing input to `TantivyLookupExec` from N to at most
//! `K * num_segments`.
//!
//! The node uses `EmissionType::Incremental` — each input batch is filtered and
//! emitted immediately using persistent per-segment heaps. This allows the
//! downstream `SortExec` to start processing early and enables its dynamic
//! filter feedback loop for progressive scan pruning.

use crate::api::HashMap;
use crate::index::fast_fields_helper::{FFHelper, FFType, NULL_TERM_ORDINAL};
use crate::scan::deferred_encode::unpack_doc_address;
use crate::scan::execution_plan::UnsafeSendStream;
use arrow_array::{Array, BooleanArray, RecordBatch, UInt64Array, UnionArray};
use arrow_schema::SchemaRef;
use arrow_select::filter::filter_record_batch;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::{RecordBatchStream, SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::metrics::{
    Count, ExecutionPlanMetricsSet, MetricBuilder, MetricsSet,
};
use datafusion::physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties};
use futures::Stream;
use std::any::Any;
use std::collections::BinaryHeap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

pub struct SegmentedTopKExec {
    input: Arc<dyn ExecutionPlan>,
    /// Column name containing packed DocAddresses (UInt64: segment_ord << 32 | doc_id).
    sort_column_name: String,
    /// Column index in input schema.
    sort_col_idx: usize,
    /// Fast field index for term ordinal lookups via FFHelper.
    ff_index: usize,
    /// FFHelper for Tantivy fast field access (shared with TantivyLookupExec).
    ffhelper: Arc<FFHelper>,
    /// Maximum rows to keep per segment.
    k: usize,
    /// true = DESC, false = ASC.
    descending: bool,
    /// true = BytesColumn, false = StrColumn.
    /// Only used for plan reconstruction in `with_new_children`; execution logic
    /// dynamically matches on `FFType::Text` vs `FFType::Bytes`.
    is_bytes: bool,
    properties: PlanProperties,
    metrics: ExecutionPlanMetricsSet,
}

impl std::fmt::Debug for SegmentedTopKExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SegmentedTopKExec")
            .field("sort_col", &self.sort_column_name)
            .field("k", &self.k)
            .finish()
    }
}

impl SegmentedTopKExec {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        input: Arc<dyn ExecutionPlan>,
        sort_column_name: String,
        sort_col_idx: usize,
        ff_index: usize,
        ffhelper: Arc<FFHelper>,
        k: usize,
        descending: bool,
        is_bytes: bool,
    ) -> Self {
        let eq_props = EquivalenceProperties::new(input.schema());
        let properties = PlanProperties::new(
            eq_props,
            input.properties().output_partitioning().clone(),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );
        Self {
            input,
            sort_column_name,
            sort_col_idx,
            ff_index,
            ffhelper,
            k,
            descending,
            is_bytes,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
        }
    }
}

impl DisplayAs for SegmentedTopKExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let direction = if self.descending { "DESC" } else { "ASC" };
        write!(
            f,
            "SegmentedTopKExec: sort_col={}, k={}, direction={}",
            self.sort_column_name, self.k, direction
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
            self.sort_column_name.clone(),
            self.sort_col_idx,
            self.ff_index,
            Arc::clone(&self.ffhelper),
            self.k,
            self.descending,
            self.is_bytes,
        )))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let input_stream = self.input.execute(partition, context)?;
        let rows_input = MetricBuilder::new(&self.metrics).counter("rows_input", partition);
        let rows_output = MetricBuilder::new(&self.metrics).counter("rows_output", partition);
        let segments_seen = MetricBuilder::new(&self.metrics).counter("segments_seen", partition);

        // SAFETY: pg_search operates in a single-threaded Tokio executor within Postgres.
        let stream = unsafe {
            UnsafeSendStream::new(SegmentedTopKStream {
                input: input_stream,
                sort_col_idx: self.sort_col_idx,
                ff_index: self.ff_index,
                ffhelper: Arc::clone(&self.ffhelper),
                k: self.k,
                descending: self.descending,
                schema: self.properties.eq_properties.schema().clone(),
                segment_heaps: HashMap::default(),
                rows_input,
                rows_output,
                segments_seen,
            })
        };
        Ok(Box::pin(stream))
    }

    fn metrics(&self) -> Option<MetricsSet> {
        Some(self.metrics.clone_inner())
    }
}

struct SegmentedTopKStream {
    input: SendableRecordBatchStream,
    sort_col_idx: usize,
    ff_index: usize,
    ffhelper: Arc<FFHelper>,
    k: usize,
    descending: bool,
    schema: SchemaRef,
    /// Persistent per-segment heaps. Each heap stores at most K ordinals
    /// (transformed via `heap_ord` for unified max-heap comparison).
    segment_heaps: HashMap<u32, BinaryHeap<u64>>,
    rows_input: Count,
    rows_output: Count,
    /// Counts segments that had rows participating in ordinal comparison (States 0+1).
    /// Segments with only State 2 (materialized) or only NULLs are not counted.
    segments_seen: Count,
}

impl SegmentedTopKStream {
    /// Process a single batch: filter rows against persistent per-segment heaps and return
    /// only the surviving rows.
    fn process_batch(&mut self, batch: &RecordBatch) -> Result<RecordBatch> {
        let num_rows = batch.num_rows();
        let mut keep = vec![false; num_rows];

        let union_col = batch
            .column(self.sort_col_idx)
            .as_any()
            .downcast_ref::<UnionArray>()
            .ok_or_else(|| {
                DataFusionError::Internal(
                    "SegmentedTopKExec: sort column should be a deferred UnionArray".into(),
                )
            })?;

        let type_ids = union_col.type_ids();
        let doc_addr_child = union_col
            .child(0)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .ok_or_else(|| {
                DataFusionError::Internal(
                    "SegmentedTopKExec: child 0 should be UInt64 doc addresses".into(),
                )
            })?;

        let term_ord_child = union_col
            .child(1)
            .as_any()
            .downcast_ref::<arrow_array::StructArray>()
            .ok_or_else(|| {
                DataFusionError::Internal(
                    "SegmentedTopKExec: child 1 should be StructArray of term ordinals".into(),
                )
            })?;
        let seg_ord_array = term_ord_child
            .column(0)
            .as_any()
            .downcast_ref::<arrow_array::UInt32Array>()
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

        // State 0 rows need FFHelper lookup: segment_ord -> Vec<(row_idx, doc_id)>
        let mut state0_by_seg: HashMap<u32, Vec<(usize, u32)>> = HashMap::default();
        // State 1 rows already have ordinals: segment_ord -> Vec<(row_idx, term_ord)>
        let mut with_ords: HashMap<u32, Vec<(usize, u64)>> = HashMap::default();

        for row_idx in 0..num_rows {
            match type_ids[row_idx] {
                0 => {
                    let packed = doc_addr_child.value(row_idx);
                    let (seg_ord, doc_id) = unpack_doc_address(packed);
                    state0_by_seg
                        .entry(seg_ord)
                        .or_default()
                        .push((row_idx, doc_id));
                }
                1 => {
                    let seg_ord = seg_ord_array.value(row_idx);
                    let term_ord = if ord_array.is_null(row_idx) {
                        NULL_TERM_ORDINAL
                    } else {
                        ord_array.value(row_idx)
                    };
                    with_ords
                        .entry(seg_ord)
                        .or_default()
                        .push((row_idx, term_ord));
                }
                2 => {
                    keep[row_idx] = true;
                }
                _ => unreachable!("Invalid Union state"),
            }
        }

        // Bulk-fetch term ordinals for State 0 rows via FFHelper.
        for (seg_ord, rows) in state0_by_seg {
            let doc_ids: Vec<u32> = rows.iter().map(|(_, doc_id)| *doc_id).collect();
            let mut term_ords: Vec<Option<u64>> = vec![None; doc_ids.len()];

            let col = self.ffhelper.column(seg_ord, self.ff_index);
            match col {
                FFType::Text(str_col) => {
                    str_col.ords().first_vals(&doc_ids, &mut term_ords);
                }
                FFType::Bytes(bytes_col) => {
                    bytes_col.ords().first_vals(&doc_ids, &mut term_ords);
                }
                _ => {
                    // Not a dictionary column; keep all rows from this segment.
                    for (row_idx, _) in rows {
                        keep[row_idx] = true;
                    }
                    continue;
                }
            }

            let entries = with_ords.entry(seg_ord).or_default();
            for (i, (row_idx, _)) in rows.into_iter().enumerate() {
                let ord = term_ords[i].unwrap_or(NULL_TERM_ORDINAL);
                entries.push((row_idx, ord));
            }
        }

        // Track newly seen segments.
        let new_segments = with_ords
            .keys()
            .filter(|seg| !self.segment_heaps.contains_key(seg))
            .count();
        self.segments_seen.add(new_segments);

        // Filter rows against persistent per-segment heaps.
        let descending = self.descending;
        let k = self.k;
        for (seg_ord, rows) in with_ords {
            let heap = self.segment_heaps.entry(seg_ord).or_default();

            for (row_idx, ord) in rows {
                if ord == NULL_TERM_ORDINAL {
                    keep[row_idx] = true;
                    continue;
                }

                let heap_val = if descending { !ord } else { ord };
                if heap.len() < k {
                    heap.push(heap_val);
                    keep[row_idx] = true;
                } else if let Some(&worst) = heap.peek() {
                    if heap_val < worst {
                        heap.pop();
                        heap.push(heap_val);
                        keep[row_idx] = true;
                    }
                }
            }
        }

        let mask: BooleanArray = keep.into_iter().map(Some).collect();
        filter_record_batch(batch, &mask)
            .map_err(|e| datafusion::common::DataFusionError::ArrowError(Box::new(e), None))
    }
}

impl Stream for SegmentedTopKStream {
    type Item = Result<RecordBatch>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            match Pin::new(&mut this.input).poll_next(cx) {
                Poll::Ready(Some(Ok(batch))) => {
                    this.rows_input.add(batch.num_rows());
                    let filtered = match this.process_batch(&batch) {
                        Ok(f) => f,
                        Err(e) => return Poll::Ready(Some(Err(e))),
                    };
                    if filtered.num_rows() > 0 {
                        this.rows_output.add(filtered.num_rows());
                        return Poll::Ready(Some(Ok(filtered)));
                    }
                    // All rows pruned in this batch — try next.
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl RecordBatchStream for SegmentedTopKStream {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}
