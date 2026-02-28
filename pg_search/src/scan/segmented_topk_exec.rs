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
//! The node uses `EmissionType::Both` — rows that bypass ordinal comparison
//! (State 2 and NULL ordinals) are emitted immediately as pass-through batches,
//! while ordinal-comparable rows are collected across all input batches.  Once
//! all input is consumed, the final survivor set is built from the per-segment
//! heaps and the remaining rows are emitted.  This ensures only the minimum
//! necessary rows flow to `TantivyLookupExec` for dictionary decoding.
//!
//! **Compound sorts:** Only the primary sort column is used for ordinal
//! pruning. When the TopK sort has tiebreaker columns (e.g.
//! `ORDER BY val DESC, id ASC LIMIT 25`), all rows tied at the boundary
//! ordinal are retained — the exec cannot distinguish between them without
//! the tiebreaker, so it keeps them all for the final TopK to resolve.
//! This is safe (never drops correct rows) but slightly less aggressive
//! than theoretically possible when there are many duplicates.
//! TODO: rewrite the full TopK sort expression in terms of term ordinals to
//! handle tiebreakers natively.

use crate::api::HashMap;
use crate::index::fast_fields_helper::{FFHelper, FFType, NULL_TERM_ORDINAL};
use crate::scan::deferred_encode::unpack_doc_address;
use crate::scan::execution_plan::UnsafeSendStream;
use arrow_array::{Array, BooleanArray, RecordBatch, UInt64Array, UnionArray};
use arrow_schema::SchemaRef;
use arrow_select::concat::concat_batches;
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
            EmissionType::Both,
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
                row_ordinals: Vec::new(),
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
    /// Per-segment max-heaps of transformed ordinals (heap_val). Used only to
    /// track the cutoff — the K-th best ordinal per segment. Row locations are
    /// NOT stored in the heap; see `row_ordinals` instead.
    segment_heaps: HashMap<SegmentOrdinal, BinaryHeap<u64>>,
    /// Shared thresholds published back to the scanner for early pruning.
    thresholds: Arc<SegmentedThresholds>,
    /// Buffered batches during the collection phase.
    batches: Vec<RecordBatch>,
    /// Per-row ordinal info for rows that went through ordinal comparison
    /// (States 0 and 1, excluding NULLs). Used by `build_survivors` to filter
    /// against the per-segment cutoff, correctly retaining all rows tied at the
    /// boundary (which a bounded heap alone would arbitrarily drop).
    row_ordinals: Vec<(usize, usize, SegmentOrdinal, u64)>,
    state: StreamState,
    rows_input: Count,
    rows_output: Count,
    /// Counts segments that had rows participating in ordinal comparison (States 0+1).
    /// Segments with only State 2 (materialized) or only NULLs are not counted.
    segments_seen: Count,
}

impl SegmentedTopKStream {
    /// Update the per-segment cutoff heap with a new ordinal. The heap tracks
    /// the K best transformed ordinals to determine the boundary. Row locations
    /// are tracked separately in `row_ordinals`.
    fn update_cutoff_heap(heap: &mut BinaryHeap<u64>, heap_val: u64, k: usize) {
        if heap.len() < k {
            heap.push(heap_val);
        } else if let Some(&worst) = heap.peek() {
            if heap_val < worst {
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

        let union_col = batch
            .column(self.sort_col_idx)
            .as_any()
            .downcast_ref::<UnionArray>()
            .ok_or_else(|| {
                DataFusionError::Internal(
                    "SegmentedTopKExec: sort column should be a deferred UnionArray".into(),
                )
            })?;

        // Dense union: each child is compact (contains only its type's rows).
        // Partition original row indices by type, then iterate compact children.
        let type_ids = union_col.type_ids();
        let offsets = union_col.offsets().ok_or_else(|| {
            DataFusionError::Internal("SegmentedTopKExec: expected dense union with offsets".into())
        })?;

        let mut pass_through = vec![false; num_rows];
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

        // State 0: compact doc address child.
        let mut state0_by_seg: HashMap<SegmentOrdinal, Vec<(usize, u32)>> = HashMap::default();
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
            }
        }

        // State 1: compact term ordinal child.
        let mut with_ords: HashMap<SegmentOrdinal, Vec<(usize, TermOrdinal)>> = HashMap::default();
        if !state1_rows.is_empty() {
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

            for &row_idx in &state1_rows {
                let ci = offsets[row_idx] as usize;
                let seg_ord = seg_ord_array.value(ci);
                let term_ord = if ord_array.is_null(ci) {
                    NULL_TERM_ORDINAL
                } else {
                    ord_array.value(ci)
                };
                with_ords
                    .entry(seg_ord)
                    .or_default()
                    .push((row_idx, term_ord));
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
                    pass_through[row_idx] = true;
                    continue;
                }
                let heap_val = if self.descending { !ord } else { ord };
                Self::update_cutoff_heap(heap, heap_val, self.k);
                self.row_ordinals
                    .push((batch_idx, row_idx, seg_ord, heap_val));
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
                    pass_through[row_idx] = true;
                    continue;
                }
                let heap_val = if self.descending { !ord } else { ord };
                Self::update_cutoff_heap(heap, heap_val, self.k);
                self.row_ordinals
                    .push((batch_idx, row_idx, seg_ord, heap_val));
            }
        }

        // Publish thresholds for segments with full heaps.
        for (seg_ord, heap) in &self.segment_heaps {
            if heap.len() >= self.k {
                let worst = *heap.peek().unwrap();
                let threshold = if self.descending { !worst } else { worst };
                self.thresholds.set_threshold(*seg_ord, threshold);
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

    /// Compact buffered batches by discarding rows that cannot survive the
    /// current per-segment cutoffs. This bounds memory at O(K * segments)
    /// instead of O(N) for large inputs — analogous to the batch compaction
    /// step in upstream DataFusion TopK.
    fn maybe_compact(&mut self) {
        let num_segments = self.segment_heaps.len().max(1);
        if self.row_ordinals.len() <= self.k * num_segments * 4 {
            return;
        }

        // Determine which row_ordinals survive the current cutoffs.
        let mut new_row_ordinals = Vec::new();
        let mut survivors = crate::api::HashSet::default();

        for &(batch_idx, row_idx, seg_ord, heap_val) in &self.row_ordinals {
            let cutoff = self
                .segment_heaps
                .get(&seg_ord)
                .and_then(|h| h.peek().copied());
            let keep = match cutoff {
                Some(cutoff_val) => heap_val <= cutoff_val,
                None => true,
            };
            if keep {
                survivors.insert((batch_idx, row_idx));
                new_row_ordinals.push((batch_idx, row_idx, seg_ord, heap_val));
            }
        }

        // Don't compact if we wouldn't discard at least half the rows.
        if new_row_ordinals.len() * 2 > self.row_ordinals.len() {
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

    /// Build the final survivor set using per-segment cutoffs from the heaps.
    ///
    /// A row survives if its transformed ordinal (heap_val) is <= the cutoff
    /// for its segment. This correctly retains ALL rows tied at the boundary,
    /// which is necessary for compound sorts where a tiebreaker column
    /// distinguishes between rows with the same primary ordinal.
    fn build_survivors(&self) -> crate::api::HashSet<(usize, usize)> {
        let mut survivors = crate::api::HashSet::default();

        for &(batch_idx, row_idx, seg_ord, heap_val) in &self.row_ordinals {
            let cutoff = self
                .segment_heaps
                .get(&seg_ord)
                .and_then(|h| h.peek().copied());
            match cutoff {
                Some(cutoff_val) if heap_val <= cutoff_val => {
                    survivors.insert((batch_idx, row_idx));
                }
                None => {
                    // Segment heap is empty (shouldn't happen), keep the row.
                    survivors.insert((batch_idx, row_idx));
                }
                _ => {} // strictly worse than cutoff — discard
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
                            match this.collect_batch(&batch, batch_idx) {
                                Ok(pass_through) => {
                                    this.batches.push(batch);
                                    this.maybe_compact();
                                    if let Some(pt) = pass_through {
                                        this.rows_output.add(pt.num_rows());
                                        return Poll::Ready(Some(Ok(pt)));
                                    }
                                }
                                Err(e) => return Poll::Ready(Some(Err(e))),
                            }
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
