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
//! The node uses `EmissionType::Final` — it collects all input batches while
//! progressively updating per-segment heaps and publishing ordinal thresholds
//! back to the scanner. Only after all input is consumed does it emit filtered
//! batches containing the exact top-K rows per segment. This ensures only the
//! minimum necessary rows flow to `TantivyLookupExec` for dictionary decoding.
//!
//! **Limitation — compound sorts:** Only the primary sort column is used for
//! ordinal pruning. When the TopK sort has tiebreaker columns (e.g.
//! `ORDER BY val DESC, id ASC LIMIT 25`), rows with duplicate primary ordinals
//! may be over-retained because the tiebreaker is not considered. This is safe
//! (never drops correct rows) but slightly less aggressive.
//! TODO: rewrite the full TopK sort expression in terms of term ordinals to
//! handle tiebreakers.

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
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tantivy::termdict::TermOrdinal;
use tantivy::SegmentOrdinal;

/// Shared per-segment ordinal thresholds, written by `SegmentedTopKExec`
/// and read by the scanner for early row pruning.
///
/// As the exec builds up its per-segment heaps, it publishes the "worst"
/// ordinal still in the top-K for each segment. The scanner can then skip
/// rows whose ordinal cannot beat that threshold, avoiding ctid lookups,
/// visibility checks, and dictionary materialisation.
pub struct SegmentedThresholds {
    /// segment_ord → threshold ordinal.
    inner: Mutex<HashMap<SegmentOrdinal, TermOrdinal>>,
    descending: bool,
    ff_index: usize,
}

impl SegmentedThresholds {
    pub fn new(descending: bool, ff_index: usize) -> Self {
        Self {
            inner: Mutex::new(HashMap::default()),
            descending,
            ff_index,
        }
    }

    pub fn set_threshold(&self, seg_ord: SegmentOrdinal, threshold: TermOrdinal) {
        self.inner.lock().unwrap().insert(seg_ord, threshold);
    }

    pub fn get_threshold(&self, seg_ord: SegmentOrdinal) -> Option<TermOrdinal> {
        self.inner.lock().unwrap().get(&seg_ord).copied()
    }

    pub fn descending(&self) -> bool {
        self.descending
    }

    pub fn ff_index(&self) -> usize {
        self.ff_index
    }
}

pub struct SegmentedTopKExec {
    input: Arc<dyn ExecutionPlan>,
    /// Column name of the deferred 3-way UnionArray (doc_address / term_ordinal / materialized).
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
    /// Shared thresholds published back to the scanner for early pruning.
    thresholds: Arc<SegmentedThresholds>,
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
        thresholds: Arc<SegmentedThresholds>,
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
            thresholds,
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
            Arc::clone(&self.thresholds),
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
                thresholds: Arc::clone(&self.thresholds),
                batches: Vec::new(),
                always_keep: Vec::new(),
                state: StreamState::Collecting,
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

/// Entry in the per-segment bounded heap.
/// Stores the transformed ordinal along with the batch/row location so that
/// the final survivor set can be built after all input is consumed.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct HeapEntry {
    heap_val: u64,
    batch_idx: usize,
    row_idx: usize,
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.heap_val
            .cmp(&other.heap_val)
            .then(self.batch_idx.cmp(&other.batch_idx))
            .then(self.row_idx.cmp(&other.row_idx))
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

enum StreamState {
    /// Collecting input batches while updating heaps and publishing thresholds.
    Collecting,
    /// Emitting filtered batches from the final survivor set.
    Emitting {
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
    /// Persistent per-segment heaps tracking the top-K entries across all batches.
    segment_heaps: HashMap<SegmentOrdinal, BinaryHeap<HeapEntry>>,
    /// Shared thresholds published back to the scanner for early pruning.
    thresholds: Arc<SegmentedThresholds>,
    /// Buffered batches during the collection phase.
    batches: Vec<RecordBatch>,
    /// State 2 (already materialized) and NULL-ordinal rows — always survive
    /// regardless of heaps. These could theoretically be emitted incrementally
    /// since they won't be compared, but we buffer them to avoid partial-batch
    /// emission during the blocking collection phase.
    always_keep: Vec<(usize, usize)>,
    state: StreamState,
    rows_input: Count,
    rows_output: Count,
    /// Counts segments that had rows participating in ordinal comparison (States 0+1).
    /// Segments with only State 2 (materialized) or only NULLs are not counted.
    segments_seen: Count,
}

impl SegmentedTopKStream {
    /// Insert a row into the bounded heap for a segment.
    fn push_into_heap(
        heap: &mut BinaryHeap<HeapEntry>,
        ord: TermOrdinal,
        batch_idx: usize,
        row_idx: usize,
        k: usize,
        descending: bool,
    ) {
        let heap_val = if descending { !ord } else { ord };
        let entry = HeapEntry {
            heap_val,
            batch_idx,
            row_idx,
        };
        if heap.len() < k {
            heap.push(entry);
        } else if let Some(&worst) = heap.peek() {
            if heap_val < worst.heap_val {
                heap.pop();
                heap.push(entry);
            }
        }
    }

    /// Ingest a single batch: extract ordinals, update per-segment heaps,
    /// publish thresholds, and record always-keep rows. The batch itself is
    /// buffered for the emission phase.
    fn collect_batch(&mut self, batch: &RecordBatch, batch_idx: usize) -> Result<()> {
        let num_rows = batch.num_rows();

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
        let mut state0_by_seg: HashMap<SegmentOrdinal, Vec<(usize, u32)>> = HashMap::default();
        // State 1 rows already have ordinals: segment_ord -> Vec<(row_idx, term_ord)>
        let mut with_ords: HashMap<SegmentOrdinal, Vec<(usize, TermOrdinal)>> = HashMap::default();

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
                    self.always_keep.push((batch_idx, row_idx));
                }
                _ => unreachable!("Invalid Union state"),
            }
        }

        // Bulk-fetch term ordinals for State 0 rows via FFHelper, then push
        // directly into the per-segment heaps (no intermediate collection).
        for (seg_ord, rows) in state0_by_seg {
            let doc_ids: Vec<u32> = rows.iter().map(|(_, doc_id)| *doc_id).collect();
            let mut term_ords: Vec<Option<TermOrdinal>> = vec![None; doc_ids.len()];

            let col = self.ffhelper.column(seg_ord, self.ff_index);
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
                        self.ff_index
                    );
                }
            }

            if !self.segment_heaps.contains_key(&seg_ord) {
                self.segments_seen.add(1);
            }
            let heap = self.segment_heaps.entry(seg_ord).or_default();
            for (i, (row_idx, _)) in rows.into_iter().enumerate() {
                let ord = term_ords[i].unwrap_or(NULL_TERM_ORDINAL);
                // TODO: Push NULL ordinals down as a NULL-aware expression
                // rather than unconditionally keeping them.
                if ord == NULL_TERM_ORDINAL {
                    self.always_keep.push((batch_idx, row_idx));
                    continue;
                }
                Self::push_into_heap(heap, ord, batch_idx, row_idx, self.k, self.descending);
            }
        }

        // State 1 rows already have ordinals — push directly into heaps.
        for (seg_ord, rows) in with_ords {
            if !self.segment_heaps.contains_key(&seg_ord) {
                self.segments_seen.add(1);
            }
            let heap = self.segment_heaps.entry(seg_ord).or_default();

            for (row_idx, ord) in rows {
                // TODO: Push NULL ordinals down as a NULL-aware expression
                // rather than unconditionally keeping them.
                if ord == NULL_TERM_ORDINAL {
                    self.always_keep.push((batch_idx, row_idx));
                    continue;
                }
                Self::push_into_heap(heap, ord, batch_idx, row_idx, self.k, self.descending);
            }
        }

        // Publish thresholds for segments with full heaps.
        for (seg_ord, heap) in &self.segment_heaps {
            if heap.len() >= self.k {
                let worst = heap.peek().unwrap().heap_val;
                let threshold = if self.descending { !worst } else { worst };
                self.thresholds.set_threshold(*seg_ord, threshold);
            }
        }

        Ok(())
    }

    /// Build the final survivor set from the heaps and always-keep rows.
    ///
    /// TODO: Like upstream TopK, periodically compact buffered batches
    /// during the collection phase to avoid holding O(N) rows in memory when
    /// only K will survive. This matters for large inputs.
    fn build_survivors(&self) -> crate::api::HashSet<(usize, usize)> {
        let mut survivors = crate::api::HashSet::default();

        for &(batch_idx, row_idx) in &self.always_keep {
            survivors.insert((batch_idx, row_idx));
        }

        for heap in self.segment_heaps.values() {
            for entry in heap.iter() {
                survivors.insert((entry.batch_idx, entry.row_idx));
            }
        }

        survivors
    }
}

impl Stream for SegmentedTopKStream {
    type Item = Result<RecordBatch>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            match &mut this.state {
                StreamState::Collecting => {
                    match Pin::new(&mut this.input).poll_next(cx) {
                        Poll::Ready(Some(Ok(batch))) => {
                            this.rows_input.add(batch.num_rows());
                            let batch_idx = this.batches.len();
                            if let Err(e) = this.collect_batch(&batch, batch_idx) {
                                return Poll::Ready(Some(Err(e)));
                            }
                            this.batches.push(batch);
                        }
                        Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                        Poll::Ready(None) => {
                            // Input exhausted — build survivor set and transition.
                            let survivors = this.build_survivors();
                            this.state = StreamState::Emitting {
                                survivors,
                                next_batch: 0,
                            };
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
                StreamState::Emitting {
                    survivors,
                    next_batch,
                } => {
                    while *next_batch < this.batches.len() {
                        let batch_idx = *next_batch;
                        *next_batch += 1;
                        let batch = &this.batches[batch_idx];
                        let num_rows = batch.num_rows();

                        let mask: BooleanArray = (0..num_rows)
                            .map(|row_idx| Some(survivors.contains(&(batch_idx, row_idx))))
                            .collect();

                        let filtered = filter_record_batch(batch, &mask)
                            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;

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
