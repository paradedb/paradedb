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
//! # Dynamic filter tradeoff
//!
//! `SegmentedTopKExec` uses `EmissionType::Final` (collects all input before
//! emitting), which blocks the `SortExec` → `PgSearchScan` dynamic filter
//! feedback loop for the sort column. This means the scan reads all rows rather
//! than pruning progressively. The tradeoff is favorable when dictionary
//! decoding dominates (many rows, wide strings), but may regress when scan-level
//! pruning would be aggressive. Use `SET paradedb.enable_segmented_topk = off`
//! to disable if needed.

use std::any::Any;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use arrow_array::{BooleanArray, RecordBatch, UInt64Array, UnionArray};
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

use crate::index::fast_fields_helper::{FFHelper, FFType, NULL_TERM_ORDINAL};
use crate::scan::deferred_encode::unpack_doc_address;
use crate::scan::execution_plan::UnsafeSendStream;

/// segment_ord -> Vec<(batch_idx, row_idx, term_ord)>
type ExtractedRows = crate::api::HashMap<u32, Vec<(usize, usize, u64)>>;

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
            EmissionType::Final,
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
                state: StreamState::Collecting(Vec::new()),
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

enum StreamState {
    /// Collecting input batches.
    Collecting(Vec<RecordBatch>),
    /// Emitting filtered batches.
    Emitting {
        batches: Vec<RecordBatch>,
        survivors: crate::api::HashSet<(usize, usize)>,
        next_batch: usize,
    },
    Done,
}

struct SegmentedTopKStream {
    input: SendableRecordBatchStream,
    sort_col_idx: usize,
    ff_index: usize,
    ffhelper: Arc<FFHelper>,
    k: usize,
    descending: bool,
    schema: SchemaRef,
    state: StreamState,
    rows_input: Count,
    rows_output: Count,
    /// Counts segments that had rows participating in ordinal comparison (States 0+1).
    /// Segments with only State 2 (materialized) or only NULLs are not counted.
    segments_seen: Count,
}

/// Entry in the per-segment bounded heap.
/// Stores (term_ordinal, batch_idx, row_idx).
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct HeapEntry {
    term_ord: u64,
    batch_idx: usize,
    row_idx: usize,
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.term_ord
            .cmp(&other.term_ord)
            .then(self.batch_idx.cmp(&other.batch_idx))
            .then(self.row_idx.cmp(&other.row_idx))
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl SegmentedTopKStream {
    /// Extract rows from the deferred UnionArray column, grouping by segment.
    ///
    /// Returns `(by_segment, always_keep)` where:
    /// - `by_segment`: segment_ord -> Vec<(batch_idx, row_idx, term_ord)> for rows
    ///   that have ordinals (State 0 after FFHelper lookup, or State 1 directly).
    /// - `always_keep`: rows that must survive regardless (State 2 = already materialized).
    fn extract_rows(
        &self,
        batches: &[RecordBatch],
    ) -> Result<(ExtractedRows, Vec<(usize, usize)>)> {
        // State 0 rows need FFHelper lookup: segment_ord -> Vec<(batch_idx, row_idx, doc_id)>
        let mut state0_by_seg: crate::api::HashMap<u32, Vec<(usize, usize, u32)>> =
            crate::api::HashMap::default();
        // State 1 rows already have ordinals: segment_ord -> Vec<(batch_idx, row_idx, term_ord)>
        let mut with_ords: crate::api::HashMap<u32, Vec<(usize, usize, u64)>> =
            crate::api::HashMap::default();
        // State 2 rows are already materialized — always keep.
        let mut always_keep = Vec::new();

        for (batch_idx, batch) in batches.iter().enumerate() {
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

            for row_idx in 0..batch.num_rows() {
                match type_ids[row_idx] {
                    0 => {
                        let packed = doc_addr_child.value(row_idx);
                        let (seg_ord, doc_id) = unpack_doc_address(packed);
                        state0_by_seg
                            .entry(seg_ord)
                            .or_default()
                            .push((batch_idx, row_idx, doc_id));
                    }
                    1 => {
                        let seg_ord = seg_ord_array.value(row_idx);
                        let term_ord = ord_array.value(row_idx);
                        with_ords
                            .entry(seg_ord)
                            .or_default()
                            .push((batch_idx, row_idx, term_ord));
                    }
                    2 => {
                        always_keep.push((batch_idx, row_idx));
                    }
                    _ => unreachable!("Invalid Union state"),
                }
            }
        }

        // Bulk-fetch term ordinals for State 0 rows via FFHelper.
        for (seg_ord, rows) in state0_by_seg {
            let doc_ids: Vec<u32> = rows.iter().map(|(_, _, doc_id)| *doc_id).collect();
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
                    for (batch_idx, row_idx, _) in rows {
                        always_keep.push((batch_idx, row_idx));
                    }
                    continue;
                }
            }

            let entries = with_ords.entry(seg_ord).or_default();
            for (i, (batch_idx, row_idx, _)) in rows.into_iter().enumerate() {
                let ord = term_ords[i].unwrap_or(NULL_TERM_ORDINAL);
                entries.push((batch_idx, row_idx, ord));
            }
        }

        Ok((with_ords, always_keep))
    }

    /// Build per-segment bounded heaps and return the set of surviving (batch_idx, row_idx) pairs.
    fn compute_survivors(
        &self,
        batches: &[RecordBatch],
    ) -> Result<crate::api::HashSet<(usize, usize)>> {
        let (by_segment, always_keep) = self.extract_rows(batches)?;

        let mut survivors = crate::api::HashSet::default();

        // Always-keep rows (State 2) bypass the heap.
        for (batch_idx, row_idx) in always_keep {
            survivors.insert((batch_idx, row_idx));
        }

        self.segments_seen.add(by_segment.len());

        for rows in by_segment.values() {
            if self.descending {
                // DESC: keep K largest ordinals → use min-heap (evict smallest).
                let mut heap: BinaryHeap<Reverse<HeapEntry>> =
                    BinaryHeap::with_capacity(self.k + 1);
                for &(batch_idx, row_idx, ord) in rows {
                    if ord == NULL_TERM_ORDINAL {
                        survivors.insert((batch_idx, row_idx));
                        continue;
                    }
                    let entry = HeapEntry {
                        term_ord: ord,
                        batch_idx,
                        row_idx,
                    };
                    if heap.len() < self.k {
                        heap.push(Reverse(entry));
                    } else if let Some(&Reverse(min)) = heap.peek() {
                        if ord > min.term_ord {
                            heap.pop();
                            heap.push(Reverse(entry));
                        }
                    }
                }
                for Reverse(entry) in heap {
                    survivors.insert((entry.batch_idx, entry.row_idx));
                }
            } else {
                // ASC: keep K smallest ordinals → use max-heap (evict largest).
                let mut heap: BinaryHeap<HeapEntry> = BinaryHeap::with_capacity(self.k + 1);
                for &(batch_idx, row_idx, ord) in rows {
                    if ord == NULL_TERM_ORDINAL {
                        survivors.insert((batch_idx, row_idx));
                        continue;
                    }
                    let entry = HeapEntry {
                        term_ord: ord,
                        batch_idx,
                        row_idx,
                    };
                    if heap.len() < self.k {
                        heap.push(entry);
                    } else if let Some(&max) = heap.peek() {
                        if ord < max.term_ord {
                            heap.pop();
                            heap.push(entry);
                        }
                    }
                }
                for entry in heap {
                    survivors.insert((entry.batch_idx, entry.row_idx));
                }
            }
        }

        Ok(survivors)
    }
}

impl Stream for SegmentedTopKStream {
    type Item = Result<RecordBatch>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            match &mut this.state {
                StreamState::Collecting(batches) => {
                    match Pin::new(&mut this.input).poll_next(cx) {
                        Poll::Ready(Some(Ok(batch))) => {
                            this.rows_input.add(batch.num_rows());
                            batches.push(batch);
                        }
                        Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                        Poll::Ready(None) => {
                            // Input exhausted — compute survivors and transition to emitting.
                            let batches = std::mem::take(batches);
                            // segments_seen is counted inside compute_survivors.
                            let survivors = match this.compute_survivors(&batches) {
                                Ok(s) => s,
                                Err(e) => return Poll::Ready(Some(Err(e))),
                            };

                            this.state = StreamState::Emitting {
                                batches,
                                survivors,
                                next_batch: 0,
                            };
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
                StreamState::Emitting {
                    batches,
                    survivors,
                    next_batch,
                } => {
                    while *next_batch < batches.len() {
                        let batch_idx = *next_batch;
                        *next_batch += 1;
                        let batch = &batches[batch_idx];
                        let num_rows = batch.num_rows();

                        let mask: BooleanArray = (0..num_rows)
                            .map(|row_idx| Some(survivors.contains(&(batch_idx, row_idx))))
                            .collect();

                        let filtered = filter_record_batch(batch, &mask).map_err(|e| {
                            datafusion::common::DataFusionError::ArrowError(Box::new(e), None)
                        })?;

                        if filtered.num_rows() > 0 {
                            this.rows_output.add(filtered.num_rows());
                            return Poll::Ready(Some(Ok(filtered)));
                        }
                    }
                    this.state = StreamState::Done;
                    return Poll::Ready(None);
                }
                StreamState::Done => return Poll::Ready(None),
            }
        }
    }
}

impl RecordBatchStream for SegmentedTopKStream {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}
