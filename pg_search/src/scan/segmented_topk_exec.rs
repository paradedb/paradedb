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

//! Per-segment Top K with global threshold pruning.
//!
//! See the [JoinScan README](../../postgres/customscan/joinscan/README.md) for
//! how this node fits into the overall physical plan and pruning pipeline.
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
//! top rows per segment. All batches are collected during the input phase,
//! and survivors are emitted in a single pass once all input is consumed.
//!
//! ## Global threshold
//!
//! As rows are ingested, a global threshold is published to the scanner:
//!
//! Once the global heap across all segments reaches K entries, the worst
//!
//! entry's deferred ordinals are converted back to strings via
//! `FFHelper::ord_to_str` and published as a `DynamicFilterPhysicalExpr`.
//! DataFusion's standard filter pushdown mechanism routes this to
//! `PgSearchScanPlan`, where `pre_filter::try_rewrite_binary` translates
//! the string literals to per-segment ordinal bounds automatically.
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

use crate::api::{HashMap, HashSet};
use crate::index::fast_fields_helper::{CanonicalColumn, FFHelper, FFType, NULL_TERM_ORDINAL};
use crate::postgres::customscan::joinscan::build::CtidColumn;
use crate::postgres::customscan::joinscan::visibility_filter::{
    materialize_deferred_ctid, DeferredCtidMaterializationState,
};
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
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
use datafusion::physical_expr::expressions::DynamicFilterPhysicalExpr;
use datafusion::physical_expr::{EquivalenceProperties, LexOrdering, PhysicalExpr};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::filter_pushdown::{
    ChildFilterDescription, ChildPushdownResult, FilterDescription, FilterPushdownPhase,
    FilterPushdownPropagation,
};
use datafusion::physical_plan::metrics::{
    Count, ExecutionPlanMetricsSet, MetricBuilder, MetricsSet,
};
use datafusion::physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties};
use pgrx::pg_sys;
use std::any::Any;
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex};
use tantivy::termdict::TermOrdinal;
use tantivy::{DocId, SegmentOrdinal};

#[derive(Clone, Debug)]
pub struct DeferredSortColumn {
    pub sort_col_idx: usize,
    pub canonical: CanonicalColumn,
}

/// Visibility data absorbed from a `VisibilityFilterExec` during the `SegmentedTopKRule`
/// optimization pass.
///
/// When `SegmentedTopKExec` absorbs a `VisibilityFilterExec` that was its direct child,
/// it takes ownership of the plan_position/OID pairs and ctid resolvers so it can
/// perform MVCC visibility checks inline — right after each prune cycle and at final
/// emission — instead of deferring them to a separate downstream node.
pub struct AbsorbedVisibilityData {
    plan_pos_oids: Vec<(usize, pg_sys::Oid)>,
    table_names: Vec<String>,
    /// Per-plan_position FFHelpers for resolving packed DocAddresses to real ctids.
    /// Wired by `VisibilityCtidResolverRule` after plan construction.
    ctid_resolvers: Mutex<Vec<Option<Arc<FFHelper>>>>,
}

impl std::fmt::Debug for AbsorbedVisibilityData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AbsorbedVisibilityData")
            .field("plan_pos_oids", &self.plan_pos_oids)
            .field("table_names", &self.table_names)
            .finish_non_exhaustive()
    }
}

impl AbsorbedVisibilityData {
    pub fn new(plan_pos_oids: Vec<(usize, pg_sys::Oid)>, table_names: Vec<String>) -> Self {
        let resolver_len = plan_pos_oids
            .iter()
            .map(|(p, _)| *p)
            .max()
            .map_or(0, |m| m + 1);
        Self {
            plan_pos_oids,
            table_names,
            ctid_resolvers: Mutex::new(vec![None; resolver_len]),
        }
    }

    pub fn plan_pos_oids(&self) -> &[(usize, pg_sys::Oid)] {
        &self.plan_pos_oids
    }

    pub fn set_ctid_resolver(&self, plan_pos: usize, ffhelper: Arc<FFHelper>) {
        let mut resolvers = self
            .ctid_resolvers
            .lock()
            .expect("AbsorbedVisibilityData ctid_resolvers lock poisoned");
        if plan_pos >= resolvers.len() {
            resolvers.resize(plan_pos + 1, None);
        }
        resolvers[plan_pos] = Some(ffhelper);
    }
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
    /// Dynamic filter pushed down through DataFusion's standard filter pushdown
    /// mechanism. Updated at runtime with a global threshold (materialized
    /// string literals) that the scanner's `try_rewrite_binary` translates to
    /// per-segment ordinal bounds.
    dynamic_filter: Arc<DynamicFilterPhysicalExpr>,
    /// Visibility data absorbed from a `VisibilityFilterExec` during plan optimization.
    /// Present only for inner-join plans where VFExec was the direct child of
    /// `TantivyLookupExec`. When present, this node owns MVCC visibility checking.
    visibility_data: Option<Arc<AbsorbedVisibilityData>>,
    properties: Arc<PlanProperties>,
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
        visibility_data: Option<Arc<AbsorbedVisibilityData>>,
    ) -> Self {
        use datafusion::physical_expr::expressions::lit;

        let mut eq_props = EquivalenceProperties::new(input.schema());
        eq_props.add_ordering(sort_exprs.clone());
        let properties = Arc::new(PlanProperties::new(
            eq_props,
            input.properties().output_partitioning().clone(),
            EmissionType::Final,
            Boundedness::Bounded,
        ));

        // Create a DynamicFilterPhysicalExpr with the sort expression columns
        // as children. The initial expression is `lit(true)` (no filtering).
        // At runtime, `update()` replaces this with the global threshold.
        let children: Vec<Arc<dyn PhysicalExpr>> =
            sort_exprs.iter().map(|e| Arc::clone(&e.expr)).collect();
        let dynamic_filter = Arc::new(DynamicFilterPhysicalExpr::new(children, lit(true)));

        Self {
            input,
            sort_exprs,
            deferred_columns,
            ffhelper,
            k,
            dynamic_filter,
            visibility_data,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
        }
    }

    /// Returns the `(plan_position, heap_oid)` pairs whose ctid columns need
    /// visibility checking. Empty when no `VisibilityFilterExec` was absorbed.
    pub fn plan_pos_oids(&self) -> &[(usize, pg_sys::Oid)] {
        self.visibility_data
            .as_deref()
            .map(AbsorbedVisibilityData::plan_pos_oids)
            .unwrap_or(&[])
    }

    /// Wire an FFHelper for resolving packed DocAddresses to real ctids for
    /// the given plan_position. Called by `VisibilityCtidResolverRule`.
    pub fn set_ctid_resolver(&self, plan_pos: usize, ffhelper: Arc<FFHelper>) {
        if let Some(vd) = &self.visibility_data {
            vd.set_ctid_resolver(plan_pos, ffhelper);
        }
    }

    fn create_mat_row_converter(
        sort_exprs: &LexOrdering,
        deferred_columns: &[DeferredSortColumn],
        ffhelper: &FFHelper,
        schema: &arrow_schema::Schema,
    ) -> Result<RowConverter> {
        let materialized_sort_fields: Vec<SortField> = sort_exprs
            .iter()
            .map(|expr| {
                let is_deferred = expr
                    .expr
                    .as_any()
                    .downcast_ref::<datafusion::physical_expr::expressions::Column>()
                    .and_then(|c| {
                        deferred_columns
                            .iter()
                            .find(|d| d.sort_col_idx == c.index())
                    });
                let data_type = if let Some(deferred) = is_deferred {
                    let col = ffhelper.column(0, deferred.canonical.ff_index);
                    match col {
                        FFType::Bytes(_) => arrow_schema::DataType::BinaryView,
                        _ => arrow_schema::DataType::Utf8View,
                    }
                } else {
                    expr.expr
                        .data_type(schema)
                        .unwrap_or(arrow_schema::DataType::Utf8View)
                };
                SortField::new_with_options(data_type, expr.options)
            })
            .collect();

        Ok(RowConverter::new(materialized_sort_fields)?)
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

    fn properties(&self) -> &Arc<PlanProperties> {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.input]
    }

    fn with_new_children(
        self: Arc<Self>,
        mut children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let mut new = SegmentedTopKExec::new(
            children.remove(0),
            self.sort_exprs.clone(),
            self.deferred_columns.clone(),
            Arc::clone(&self.ffhelper),
            self.k,
            self.visibility_data.clone(),
        );
        // Preserve the existing dynamic filter so that filter pushdown
        // wiring (which already holds a reference) stays connected.
        new.dynamic_filter = Arc::clone(&self.dynamic_filter);
        Ok(Arc::new(new))
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
        // Only register rows_filtered_invisible in the MetricsSet when visibility
        // filtering is actually active. For non-visibility plans the counter would
        // always be 0, and render_plan_with_metrics shows every registered metric
        // (including zeros), which would pollute EXPLAIN ANALYZE output and break
        // pg_regress .out files. A standalone Count::new() is functionally equivalent
        // for increment purposes but is never added to the MetricsSet.
        let rows_filtered_invisible = if self.visibility_data.is_some() {
            MetricBuilder::new(&self.metrics).counter("rows_filtered_invisible", partition)
        } else {
            Count::new()
        };

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

        let mat_row_converter = Self::create_mat_row_converter(
            &self.sort_exprs,
            &self.deferred_columns,
            &self.ffhelper,
            self.properties.eq_properties.schema(),
        )?;

        // Build per-relation visibility checker entries from the absorbed VFExec data
        // (if any). These are created eagerly here — before the async stream body —
        // so that `execute()` can return a clean `Err` on misconfiguration rather
        // than failing mid-stream.
        let visibility_entries: Vec<StkVisibilityEntry> = if let Some(vd) = &self.visibility_data {
            let resolvers = vd
                .ctid_resolvers
                .lock()
                .expect("ctid_resolvers lock poisoned")
                .clone();
            // SAFETY: GetActiveSnapshot is safe during query execution.
            let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
            if snapshot.is_null() {
                return Err(DataFusionError::Execution(
                    "SegmentedTopKExec: requires an active Postgres snapshot \
                         for visibility checking"
                        .into(),
                ));
            }
            let schema = self.properties.eq_properties.schema();
            let mut entries = Vec::with_capacity(vd.plan_pos_oids.len());
            for &(plan_pos, heap_oid) in &vd.plan_pos_oids {
                let col_name = CtidColumn::new(plan_pos).to_string();
                let (col_idx, _) = schema.column_with_name(&col_name).ok_or_else(|| {
                    DataFusionError::Execution(format!(
                        "SegmentedTopKExec: missing ctid column '{}' \
                                 for visibility checking",
                        col_name
                    ))
                })?;
                let heaprel = PgSearchRelation::open(heap_oid);
                let checker = VisibilityChecker::with_rel_and_snap(&heaprel, snapshot);
                let resolver =
                    resolvers
                        .get(plan_pos)
                        .and_then(|r| r.clone())
                        .ok_or_else(|| {
                            DataFusionError::Execution(format!(
                                "SegmentedTopKExec: no ctid resolver wired for \
                                 plan_position {plan_pos}. \
                                 VisibilityCtidResolverRule must run before execute."
                            ))
                        })?;
                entries.push(StkVisibilityEntry {
                    col_idx,
                    checker,
                    resolver,
                    deferred_ctid_state: DeferredCtidMaterializationState::default(),
                });
            }
            entries
        } else {
            Vec::new()
        };

        let mut state = SegmentedTopKState {
            sort_exprs: self.sort_exprs.clone(),
            deferred_columns: self.deferred_columns.clone(),
            ffhelper: Arc::clone(&self.ffhelper),
            k: self.k,
            schema: self.properties.eq_properties.schema().clone(),
            row_converter,
            segment_heaps: HashMap::default(),
            distinct_segments: HashSet::default(),
            dynamic_filter: Arc::clone(&self.dynamic_filter),
            batches: Vec::new(),
            row_ordinals: Vec::new(),
            pass_through_rows: Vec::new(),
            last_segment_cutoffs: HashMap::default(),
            mat_row_converter,
            last_published_global: None,
            visibility_entries,
            rows_input,
            rows_output,
            segments_seen,
            rows_filtered_invisible,
        };

        let stream_gen = async_stream::try_stream! {
            use futures::StreamExt;
            while let Some(batch_res) = input_stream.next().await {
                let batch = batch_res?;
                state.rows_input.add(batch.num_rows());

                let batch_idx = state.batches.len();
                state.collect_batch(&batch, batch_idx)?;
                state.batches.push(batch);
                state.maybe_compact()?;
            }

            // All input consumed — perform final sort + limit and emit exactly K rows.
            let final_batch = state.emit_final_topk()?;
            if let Some(batch) = final_batch {
                state.rows_output.add(batch.num_rows());
                yield batch;
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

    /// Pushes `SegmentedTopKExec`'s own [`DynamicFilterPhysicalExpr`] (the global
    /// threshold with materialized string literals) down to child nodes via
    /// DataFusion's standard filter pushdown mechanism.
    fn gather_filters_for_pushdown(
        &self,
        phase: FilterPushdownPhase,
        parent_filters: Vec<Arc<dyn PhysicalExpr>>,
        _config: &datafusion::common::config::ConfigOptions,
    ) -> Result<FilterDescription> {
        // Only push filters in the Post phase (same as SortExec).
        if !matches!(phase, FilterPushdownPhase::Post) {
            return Ok(FilterDescription::all_unsupported(
                &parent_filters,
                &self.children(),
            ));
        }

        // Route parent filters to our child based on column compatibility,
        // and add our own dynamic filter as a self-filter.
        Ok(FilterDescription::new().with_child(
            ChildFilterDescription::from_child(&parent_filters, &self.input)?
                .with_self_filter(Arc::clone(&self.dynamic_filter) as Arc<dyn PhysicalExpr>),
        ))
    }

    fn handle_child_pushdown_result(
        &self,
        _phase: FilterPushdownPhase,
        child_pushdown_result: ChildPushdownResult,
        _config: &datafusion::common::config::ConfigOptions,
    ) -> Result<FilterPushdownPropagation<Arc<dyn ExecutionPlan>>> {
        // Pass through: report parent filter support based on what the child accepted.
        Ok(FilterPushdownPropagation::if_all(child_pushdown_result))
    }
}

/// Per-plan_position runtime state for ctid resolution and visibility checking.
/// One entry per absorbed `(plan_pos, heap_oid)` pair; empty when no
/// `VisibilityFilterExec` was absorbed.
struct StkVisibilityEntry {
    /// Index of the `ctid_{plan_position}` column in the input batch schema.
    col_idx: usize,
    /// MVCC visibility checker for this relation.
    checker: VisibilityChecker,
    /// Resolves packed DocAddresses to real ctids before visibility checking.
    resolver: Arc<FFHelper>,
    /// Reusable scratch buffers for packed DocAddress materialization.
    deferred_ctid_state: DeferredCtidMaterializationState,
}

struct SegmentedTopKState {
    sort_exprs: LexOrdering,
    deferred_columns: Vec<DeferredSortColumn>,
    ffhelper: Arc<FFHelper>,
    k: usize,
    schema: SchemaRef,
    row_converter: RowConverter,
    /// Per-segment max-heaps of comparable Rows. The root (peek) is always the K-th
    /// best row for that segment (the per-segment cutoff threshold).
    ///
    /// **Non-visibility plans:** updated in `collect_batch` on every ingested row, so the
    /// heap reflects the live running top-K per segment at all times.
    ///
    /// **Visibility plans:** updated ONLY inside `maybe_compact()` after visibility
    /// checking. This invariant guarantees every cutoff in the heap corresponds to a row
    /// that was alive at the last prune cycle, so the published global threshold is always
    /// based on visible (alive) rows — never on dead rows sitting in the buffer.
    segment_heaps: HashMap<SegmentOrdinal, BinaryHeap<OwnedRow>>,
    /// Set of all segment ordinals encountered during collection.
    ///
    /// Used to compute the prune-cycle trigger (`2 × K × num_segments` for visibility
    /// plans). Unlike `segment_heaps`, this is updated in `collect_batch` for EVERY
    /// ingested row — including visibility plans where `segment_heaps` is left empty
    /// until the first prune cycle — and is never cleared. It only grows as new Tantivy
    /// segments are encountered, giving an accurate segment count for the trigger.
    distinct_segments: HashSet<SegmentOrdinal>,
    /// Dynamic filter updated with global thresholds (materialized strings).
    /// Pushed down through DataFusion's standard filter pushdown to the scanner.
    dynamic_filter: Arc<DynamicFilterPhysicalExpr>,
    /// Buffered batches during the collection phase.
    batches: Vec<RecordBatch>,
    /// Keeps track of the heap rows for compaction.
    /// (batch_idx, row_idx, seg_ord, row_data)
    row_ordinals: Vec<(usize, usize, SegmentOrdinal, OwnedRow)>,
    /// Buffered pass-through rows (State 2 and NULL ordinals) that bypass
    /// ordinal comparison. These are included in the final sort + limit.
    pass_through_rows: Vec<(usize, usize)>,

    /// For each segment heap that has reached size `k`, we cache the resolved values
    /// of its current worst row (the root of the heap).
    /// Tuple: (local ordinal OwnedRow, materialized ScalarValues, materialized OwnedRow)
    last_segment_cutoffs:
        HashMap<SegmentOrdinal, (OwnedRow, Vec<datafusion::common::ScalarValue>, OwnedRow)>,

    /// Row converter for materialized sorts, used to compare resolved thresholds lexicographically.
    mat_row_converter: RowConverter,

    /// Cache of the last published global threshold to avoid redundant filter updates.
    /// Stores the best of the worst materialized rows across segments.
    last_published_global: Option<OwnedRow>,

    rows_input: Count,
    rows_output: Count,
    /// Counts segments that had rows participating in ordinal comparison (States 0+1).
    /// Segments with only State 2 (materialized) or only NULLs are not counted.
    segments_seen: Count,
    /// Counts rows removed because they were dead (invisible) under the current snapshot.
    /// Incremented during prune cycles (maybe_compact) and at final emission (emit_final_topk).
    rows_filtered_invisible: Count,
    /// Runtime visibility checker entries, one per absorbed `(plan_pos, heap_oid)` pair.
    /// Empty when no `VisibilityFilterExec` was absorbed.
    visibility_entries: Vec<StkVisibilityEntry>,
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
    /// phase. Pass-through rows (State 2 and NULL ordinals) are buffered
    /// in `pass_through_rows` for the final sort + limit.
    fn collect_batch(&mut self, batch: &RecordBatch, batch_idx: usize) -> Result<()> {
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
                // Track distinct segments for the compaction trigger. Always updated here
                // so that the trigger denominator is accurate even for visibility plans
                // where segment_heaps is not touched until maybe_compact().
                if self.distinct_segments.insert(seg_ord) {
                    self.segments_seen.add(1);
                }

                let heap_val = converted_rows.row(row_idx).owned();

                if self.visibility_entries.is_empty() {
                    // Non-visibility plan: update the per-segment cutoff heap directly
                    // and buffer the row unconditionally. No dead rows exist, so the
                    // heap and the threshold it produces are always correct.
                    let heap = self.segment_heaps.entry(seg_ord).or_default();
                    Self::update_cutoff_heap(heap, heap_val.clone(), self.k);
                    self.row_ordinals
                        .push((batch_idx, row_idx, seg_ord, heap_val));
                } else {
                    // Visibility plan: segment_heaps is NEVER updated here. Heaps are
                    // rebuilt inside maybe_compact() only after visibility checking, so
                    // every cutoff in segment_heaps always reflects a row that was alive
                    // at the last prune cycle. This is the Vec+QuickSelect invariant:
                    // "threshold always based on visible rows."
                    //
                    // Pre-filter using the last prune cycle's alive cutoff (read-only).
                    // Rows provably worse than the alive K-th are rejected early to bound
                    // buffer growth between prune cycles.
                    let is_candidate = match self.segment_heaps.get(&seg_ord).and_then(|h| h.peek())
                    {
                        Some(cutoff) => &heap_val <= cutoff,
                        // Before first prune cycle, or first row for this segment: accept.
                        None => true,
                    };
                    if is_candidate {
                        self.row_ordinals
                            .push((batch_idx, row_idx, seg_ord, heap_val));
                    }
                }
            }
        }

        // Non-visibility plans: publish threshold after each batch so PgSearchScan can
        // prune as early as possible. Safe: no dead rows can inflate the threshold.
        //
        // Visibility plans: threshold is published ONLY inside maybe_compact() after
        // visibility checking. Publishing here would base the threshold on rows that have
        // not yet been checked for MVCC visibility — dead rows in the buffer would inflate
        // the threshold, causing PgSearchScan to prune live rows it shouldn't.
        if self.visibility_entries.is_empty() {
            self.publish_global_threshold()?;
        }

        // Buffer pass-through rows (State 2 + NULL ordinals) for the final sort.
        for (row_idx, &is_pt) in pass_through.iter().enumerate() {
            if is_pt {
                self.pass_through_rows.push((batch_idx, row_idx));
            }
        }

        Ok(())
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
        let column = batch.column(deferred_col.sort_col_idx);
        let union_col = column
            .as_any()
            .downcast_ref::<UnionArray>()
            .ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "SegmentedTopKExec: sort column should be a deferred UnionArray but found {:?} at index {}",
                    column.data_type(), deferred_col.sort_col_idx
                ))
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

            let col = self
                .ffhelper
                .column(seg_ord, deferred_col.canonical.ff_index);
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
                            deferred_col.canonical.ff_index
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

    /// Build the set of all survivors across all segments. A row survives if
    /// its `OwnedRow` is <= the cutoff (worst heap entry) for its segment.
    fn build_survivors(&self) -> crate::api::HashSet<(usize, usize)> {
        let mut survivors = crate::api::HashSet::default();
        for (batch_idx, row_idx, seg_ord, heap_val) in &self.row_ordinals {
            let dominated = self
                .segment_heaps
                .get(seg_ord)
                .and_then(|h| h.peek())
                .is_some_and(|cutoff| heap_val <= cutoff);
            if dominated {
                survivors.insert((*batch_idx, *row_idx));
            }
        }
        survivors
    }

    /// Evaluates the current local thresholds across all segments to determine
    /// if a new global threshold can be published.
    ///
    /// This method is responsible for computing a safe, conservative global threshold
    /// by finding the "best of the worst" materialized cutoff among all full segments.
    /// By only resolving the worst entry of each segment rather than every row, it
    /// minimizes the overhead of translating segment-local ordinals into global strings.
    fn publish_global_threshold(&mut self) -> Result<()> {
        let mut best_worst_mat_row: Option<OwnedRow> = None;
        let mut best_worst_values: Option<Vec<datafusion::common::ScalarValue>> = None;

        // 1. Examine the "worst" row (the root of the heap) for each segment that
        //    has reached size `K`.
        let full_segment_heaps: Vec<(SegmentOrdinal, OwnedRow)> = self
            .segment_heaps
            .iter()
            .filter(|(_, heap)| heap.len() >= self.k)
            .filter_map(|(&seg_ord, heap)| heap.peek().map(|row| (seg_ord, row.clone())))
            .collect();

        for (seg_ord, worst_local) in full_segment_heaps {
            // 2. Resolve the local ordinal threshold into a materialized row.
            let (mat_values, mat_row) = self.resolve_segment_cutoff(seg_ord, &worst_local)?;

            // 3. Find the "best of the worst" (minimum of maximums) among all segments'
            //    thresholds. If we use a bound greater than any full segment's local cutoff,
            //    we might prune competitive rows in other segments. By taking the tightest
            //    upper bound across all full segments, we ensure a mathematically safe
            //    threshold for global pruning.
            match &best_worst_mat_row {
                None => {
                    best_worst_mat_row = Some(mat_row);
                    best_worst_values = Some(mat_values);
                }
                Some(current_best) => {
                    if &mat_row < current_best {
                        best_worst_mat_row = Some(mat_row);
                        best_worst_values = Some(mat_values);
                    }
                }
            }
        }

        // 4. Finally, if the newly calculated global threshold is better than the one
        //    we previously published, build a new dynamic filter expression and
        //    push it down to the scanner.
        let (Some(best_row), Some(best_values)) = (best_worst_mat_row, best_worst_values) else {
            return Ok(());
        };

        let changed = match &self.last_published_global {
            Some(prev) => &best_row != prev,
            None => true,
        };

        if changed {
            if let Some(expr) = Self::build_lexicographic_filter(&self.sort_exprs, &best_values) {
                let _ = self.dynamic_filter.update(expr);
                self.last_published_global = Some(best_row);
            }
        }

        Ok(())
    }

    /// Resolves a segment-local ordinal threshold into a globally comparable materialized row.
    ///
    /// It attempts to reuse previously resolved values if the local threshold hasn't changed.
    /// If the threshold is new, it pays the cost to decode the segment-local ordinals via
    /// `FFHelper::ord_to_str` and then constructs a materialized `OwnedRow`.
    fn resolve_segment_cutoff(
        &mut self,
        seg_ord: SegmentOrdinal,
        worst_local: &OwnedRow,
    ) -> Result<(Vec<datafusion::common::ScalarValue>, OwnedRow)> {
        // a. Compare this local threshold with a cached version from the previous
        //    batch. If the threshold hasn't changed, reuse the materialized string
        //    values. If it has changed, pay the cost to resolve the segment-local
        //    ordinals into global string/bytes values via `resolve_global_threshold_values`.
        if let Some((cached_local, vals, row)) = self.last_segment_cutoffs.get(&seg_ord) {
            if cached_local == worst_local {
                return Ok((vals.clone(), row.clone()));
            }
        }

        let arrays = self
            .row_converter
            .convert_rows(std::iter::once(worst_local.row()))?;

        let values = self.resolve_global_threshold_values(&arrays, seg_ord)?;

        let val_arrays = values
            .iter()
            .map(|v| v.to_array())
            .collect::<Result<Vec<_>, _>>()?;

        // b. Convert these materialized scalar values into an `OwnedRow` using
        //    `mat_row_converter`. This enables fast, correct lexicographical comparison
        //    of values across different segments.
        let converted = self.mat_row_converter.convert_columns(&val_arrays)?;

        let mat_row = converted.row(0).owned();
        self.last_segment_cutoffs.insert(
            seg_ord,
            (worst_local.clone(), values.clone(), mat_row.clone()),
        );
        Ok((values, mat_row))
    }

    /// Resolve threshold values for the global filter.
    ///
    /// For deferred columns, converts ordinals back to materialized strings
    /// via `FFHelper::ord_to_str`. For non-deferred columns, reads the scalar
    /// directly from the array. Returns `None` if any conversion fails.
    fn resolve_global_threshold_values(
        &self,
        arrays: &[ArrayRef],
        seg_ord: SegmentOrdinal,
    ) -> Result<Vec<datafusion::common::ScalarValue>> {
        use datafusion::common::ScalarValue;

        let mut values = Vec::with_capacity(self.sort_exprs.len());
        for (i, sort_expr) in self.sort_exprs.iter().enumerate() {
            let is_deferred = sort_expr
                .expr
                .as_any()
                .downcast_ref::<datafusion::physical_expr::expressions::Column>()
                .and_then(|c| {
                    self.deferred_columns
                        .iter()
                        .find(|d| d.sort_col_idx == c.index())
                });

            let value = if let Some(deferred) = is_deferred {
                let term_ord = arrays[i]
                    .as_any()
                    .downcast_ref::<UInt64Array>()
                    .ok_or_else(|| {
                        datafusion::error::DataFusionError::Internal(
                            "Expected UInt64Array for deferred ordinal".to_string(),
                        )
                    })?
                    .value(0);
                let col = self.ffhelper.column(seg_ord, deferred.canonical.ff_index);
                match col {
                    FFType::Text(str_col) => {
                        let mut s = String::new();
                        str_col.ord_to_str(term_ord, &mut s).map_err(|e| {
                            datafusion::error::DataFusionError::Internal(format!(
                                "Failed to resolve string ordinal: {}",
                                e
                            ))
                        })?;
                        ScalarValue::Utf8View(Some(s))
                    }
                    FFType::Bytes(bytes_col) => {
                        let mut b = Vec::new();
                        bytes_col.ord_to_bytes(term_ord, &mut b).map_err(|e| {
                            datafusion::error::DataFusionError::Internal(format!(
                                "Failed to resolve bytes ordinal: {}",
                                e
                            ))
                        })?;
                        ScalarValue::BinaryView(Some(b))
                    }
                    _ => {
                        return Err(datafusion::error::DataFusionError::Internal(
                            "Unexpected column type for deferred field".to_string(),
                        ));
                    }
                }
            } else {
                ScalarValue::try_from_array(&arrays[i], 0)?
            };
            values.push(value);
        }
        Ok(values)
    }

    /// Compact buffered batches by discarding rows that cannot survive the
    /// current per-segment cutoffs. This bounds memory at O(K * segments)
    /// instead of O(N) for large inputs — analogous to the batch compaction
    /// step in upstream DataFusion Top K.
    ///
    /// When visibility data was absorbed, dead rows are also removed here so
    /// that the global threshold is never inflated by rows that are invisible
    /// in the heap. `segment_heaps` is rebuilt from the visible survivors.
    fn maybe_compact(&mut self) -> Result<()> {
        // Use `distinct_segments` (not `segment_heaps`) to count segments: for visibility
        // plans, segment_heaps is empty before the first prune cycle, which would cause
        // segment_heaps.len() to under-count and fire prune cycles too early.
        let num_segments = self.distinct_segments.len().max(1);

        // Visibility plans use the Vec+QuickSelect trigger: prune when the buffer holds
        // 2×K rows per segment. This ensures visibility is checked before the threshold
        // is (re)published, satisfying the spec invariant: "threshold always based on
        // visible rows."
        //
        // Non-visibility plans use the original 4×K factor (unchanged behavior).
        let trigger = if self.visibility_entries.is_empty() {
            self.k * num_segments * 4
        } else {
            2 * self.k * num_segments
        };
        // Q122-129 fix: include pass_through_rows in the trigger count.
        // Plans where all rows have NULL/State-2 sort columns accumulate only
        // pass_through_rows (row_ordinals stays empty), so without this addition
        // the trigger would never fire and visibility checking would be deferred
        // entirely to emit_final_topk.
        if self.row_ordinals.len() + self.pass_through_rows.len() < trigger {
            return Ok(());
        }

        // Step 1: Cutoff filter — retain only rows within per-segment bounds.
        // Capture the pre-take length for the compaction skip check below (BUG-C fix:
        // after std::mem::take the original Vec has capacity 0, making the ratio check
        // always true and bypassing the "don't compact if < half rows discarded" guard).
        let old_row_ordinals_len = self.row_ordinals.len();
        let mut new_row_ordinals = Vec::new();
        for (batch_idx, row_idx, seg_ord, heap_val) in std::mem::take(&mut self.row_ordinals) {
            let keep = match self.segment_heaps.get(&seg_ord).and_then(|h| h.peek()) {
                Some(cutoff_val) => &heap_val <= cutoff_val,
                None => true,
            };
            if keep {
                new_row_ordinals.push((batch_idx, row_idx, seg_ord, heap_val));
            }
        }

        // Step 2: Visibility filter — remove dead rows so the global threshold
        // reflects only live rows. Rebuild segment_heaps from visible survivors.
        // pass_through_rows are NOT checked here; they are checked in emit_final_topk.
        if !self.visibility_entries.is_empty() && !new_row_ordinals.is_empty() {
            let row_keys: Vec<(usize, usize)> = new_row_ordinals
                .iter()
                .map(|(bi, ri, _, _)| (*bi, *ri))
                .collect();
            // Discard corrected ctids at compaction time: batches keep packed
            // DocAddresses (real ctid write-back happens at emit_final_topk).
            let (visible_mask, _) = self.check_rows_visible(&row_keys)?;
            let any_invisible = visible_mask.iter().any(|&v| !v);
            if any_invisible {
                self.rows_filtered_invisible
                    .add(visible_mask.iter().filter(|&&v| !v).count());
                new_row_ordinals = new_row_ordinals
                    .into_iter()
                    .zip(visible_mask)
                    .filter(|(_, visible)| *visible)
                    .map(|(row, _)| row)
                    .collect();
                // Rebuild segment_heaps from visible survivors so the global
                // threshold accurately reflects only live rows.
                self.segment_heaps.clear();
                // BUG-E fix: stale last_segment_cutoffs referred to the old heap
                // state; clear it so publish_global_threshold recomputes correctly.
                self.last_segment_cutoffs.clear();
                for (_, _, seg_ord, heap_val) in &new_row_ordinals {
                    let heap = self.segment_heaps.entry(*seg_ord).or_default();
                    Self::update_cutoff_heap(heap, heap_val.clone(), self.k);
                }
                // BUG-B fix: if dead-row removal made any segment heap underfull
                // (fewer than K entries), the previously published threshold is now
                // too aggressive — it blocks rows with scores below the dead row's
                // ordinal that are needed to refill the heap.  Retract to lit(true)
                // so those rows can flow in, then republish the correct threshold
                // from the rebuilt (possibly underfull) heaps.
                // publish_global_threshold() returns early without touching
                // dynamic_filter when no heap has reached size K, so we must reset
                // it explicitly here before calling it.
                let any_underfull = self.segment_heaps.values().any(|h| h.len() < self.k);
                if any_underfull {
                    use datafusion::physical_expr::expressions::lit;
                    let _ = self.dynamic_filter.update(lit(true));
                    self.last_published_global = None;
                }
                self.publish_global_threshold()?;
            }
        }

        // Step 3: Populate survivors set for batch compaction.
        let mut survivors = crate::api::HashSet::default();
        for (batch_idx, row_idx, _, _) in &new_row_ordinals {
            survivors.insert((*batch_idx, *row_idx));
        }
        // Include pass-through rows in the survivor set so they aren't discarded.
        for &(batch_idx, row_idx) in &self.pass_through_rows {
            survivors.insert((batch_idx, row_idx));
        }

        // Don't compact if we wouldn't discard at least half the rows.
        // Use old_row_ordinals_len (captured before take()) — after take() the Vec
        // is empty with capacity 0, which would make this check always true.
        if new_row_ordinals.len() * 2 > old_row_ordinals_len {
            self.row_ordinals = new_row_ordinals;
            return Ok(());
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

            let filtered = filter_record_batch(batch, &mask)
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
            filtered_batches.push(filtered);
        }

        if filtered_batches.is_empty() {
            self.batches.clear();
            self.row_ordinals.clear();
            self.pass_through_rows.clear();
            return Ok(());
        }

        let compacted = concat_batches(&self.schema, &filtered_batches)
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;

        // Remap row_ordinals into the single compacted batch.
        for entry in &mut new_row_ordinals {
            let new_ri = mapping[&(entry.0, entry.1)];
            entry.0 = 0;
            entry.1 = new_ri;
        }

        // Remap pass_through_rows into the single compacted batch.
        for entry in &mut self.pass_through_rows {
            let new_ri = mapping[&(entry.0, entry.1)];
            entry.0 = 0;
            entry.1 = new_ri;
        }

        self.row_ordinals = new_row_ordinals;
        self.batches = vec![compacted];
        Ok(())
    }

    /// Check visibility for a set of rows identified by `(batch_idx, row_idx)` pairs.
    ///
    /// For each absorbed `(plan_pos, heap_oid)` pair, extracts the packed DocAddress
    /// from the ctid column of the stored batch, resolves it to a real ctid via
    /// `FFHelper`, then calls `VisibilityChecker::check_batch`. The overall result
    /// is the AND of all per-relation visibility masks.
    ///
    /// Returns `(visible_mask, corrected_ctids_per_entry)`:
    /// - `visible_mask[i]` is `false` when the i-th row is invisible to any relation.
    /// - `corrected_ctids_per_entry[e][i]` is the HOT-corrected real ctid for the i-th
    ///   row as seen by entry `e`. `None` means invisible (or unresolvable) for that
    ///   entry. These HOT-corrected values must be used in the final output so that
    ///   `fetch_tuple_direct` (which does NOT follow HOT chains) receives the right ctid.
    ///
    /// Returns all-true / empty-corrected immediately when `visibility_entries` is empty.
    #[allow(clippy::type_complexity)]
    fn check_rows_visible(
        &mut self,
        row_keys: &[(usize, usize)],
    ) -> Result<(Vec<bool>, Vec<Vec<Option<u64>>>)> {
        if self.visibility_entries.is_empty() || row_keys.is_empty() {
            return Ok((vec![true; row_keys.len()], Vec::new()));
        }

        let n = row_keys.len();
        let mut overall_visible = vec![true; n];
        let mut all_corrected: Vec<Vec<Option<u64>>> =
            Vec::with_capacity(self.visibility_entries.len());

        for entry in self.visibility_entries.iter_mut() {
            // Track which input positions had a null ctid so we can mark them
            // invisible regardless of what materialize_deferred_ctid returns.
            // We must NOT use 0 as a sentinel: (seg_ord=0, doc_id=0) is a valid
            // packed DocAddress that could resolve to a real ctid.
            let mut input_null: Vec<bool> = Vec::with_capacity(n);

            // Extract packed doc addresses from the stored batches.
            let mut packed_values: Vec<u64> = Vec::with_capacity(n);
            for &(batch_idx, row_idx) in row_keys {
                let col = self.batches[batch_idx].column(entry.col_idx);
                let arr = col.as_any().downcast_ref::<UInt64Array>().ok_or_else(|| {
                    DataFusionError::Internal("SegmentedTopKExec: ctid column is not UInt64".into())
                })?;
                if arr.is_null(row_idx) {
                    input_null.push(true);
                    packed_values.push(0u64); // placeholder; overridden below
                } else {
                    input_null.push(false);
                    packed_values.push(arr.value(row_idx));
                }
            }

            // Resolve packed DocAddresses → real ctids via FFHelper.
            let packed_array = UInt64Array::from(packed_values);
            let resolved_array = materialize_deferred_ctid(
                &entry.resolver,
                &packed_array,
                &mut entry.deferred_ctid_state,
            )?;
            let resolved = resolved_array
                .as_any()
                .downcast_ref::<UInt64Array>()
                .ok_or_else(|| {
                    DataFusionError::Internal(
                        "SegmentedTopKExec: resolved ctid array is not UInt64".into(),
                    )
                })?;

            // Collect valid (non-null, non-null-input) ctids with their original indices.
            // check_batch panics on None inputs, so we filter to resolvable rows only.
            let mut valid: Vec<(usize, u64)> = Vec::with_capacity(n);
            for (i, vis) in overall_visible.iter_mut().enumerate() {
                if input_null[i] || resolved.is_null(i) {
                    // Null input ctid or unresolvable → treat as invisible.
                    *vis = false;
                } else {
                    valid.push((i, resolved.value(i)));
                }
            }

            // Per-entry HOT-corrected ctids: None for invisible rows, Some(ctid) for
            // visible rows. Populated from check_batch results below.
            let mut entry_corrected: Vec<Option<u64>> = vec![None; n];

            if !valid.is_empty() {
                let ctids_for_check: Vec<Option<u64>> =
                    valid.iter().map(|(_, c)| Some(*c)).collect();
                let mut results: Vec<Option<u64>> = vec![None; valid.len()];
                entry.checker.check_batch(&ctids_for_check, &mut results);

                for ((orig_idx, _), result) in valid.iter().zip(results.iter()) {
                    match result {
                        // heap_hot_search_buffer returned the HOT-corrected ctid.
                        // Store it so emit_final_topk can write it to the output
                        // column instead of the raw index ctid.
                        Some(corrected_ctid) => {
                            entry_corrected[*orig_idx] = Some(*corrected_ctid);
                        }
                        None => {
                            overall_visible[*orig_idx] = false;
                        }
                    }
                }
            }

            all_corrected.push(entry_corrected);
        }

        Ok((overall_visible, all_corrected))
    }

    /// Perform the final sort + limit after all input is consumed.
    ///
    ///
    /// Steps:
    /// 1. Build ordinal survivors (rows within per-segment cutoffs).
    /// 2. Merge ordinal survivors with pass-through rows into candidate set.
    ///    2a. (Visibility) Filter invisible rows from candidates, including pass-through rows.
    /// 3. Materialize sort column values for each candidate.
    /// 4. Sort candidates by materialized values, take top K.
    /// 5. Emit a single sorted batch.
    fn emit_final_topk(&mut self) -> Result<Option<RecordBatch>> {
        use datafusion::common::ScalarValue;

        // 1. Build ordinal survivors.
        //
        // When visibility data is present, dead rows in the top-K heap slots would
        // cause build_survivors to exclude live rows with worse ordinals.  Example:
        // k=5, rows 1–5 are in the heap (ordinals 0–4), rows 6–15 are pruned out,
        // but rows 1–3 are dead → only 2 live rows survive.  To avoid this, we skip
        // the ordinal-based pre-filter and include ALL row_ordinals as candidates;
        // the visibility pass removes dead rows and the final sort+limit yields the
        // correct k live rows.  The extra visibility checks (over all row_ordinals
        // rather than just the top-k survivors) are acceptable for correctness.
        type Candidate = (usize, usize, Option<(SegmentOrdinal, OwnedRow)>);
        let mut candidates: Vec<Candidate> = Vec::new();

        if !self.visibility_entries.is_empty() {
            // Include every row from row_ordinals so that after dead rows are removed
            // we still have enough live rows to fill k slots.
            for (batch_idx, row_idx, seg_ord, heap_val) in &self.row_ordinals {
                candidates.push((*batch_idx, *row_idx, Some((*seg_ord, heap_val.clone()))));
            }
        } else {
            // 1. Build ordinal survivors (normal path, no visibility data).
            let ordinal_survivors = self.build_survivors();
            // 2. Collect all candidates: ordinal survivors only.
            for (batch_idx, row_idx, seg_ord, heap_val) in &self.row_ordinals {
                if ordinal_survivors.contains(&(*batch_idx, *row_idx)) {
                    candidates.push((*batch_idx, *row_idx, Some((*seg_ord, heap_val.clone()))));
                }
            }
        }

        // Always include pass-through rows (State 2 + NULL ordinals).
        for &(batch_idx, row_idx) in &self.pass_through_rows {
            candidates.push((batch_idx, row_idx, None));
        }

        // 2a. Visibility filter: remove invisible rows from candidates.
        //     pass_through_rows are checked here (they bypass the prune cycle).
        //
        // corrected_lookup[entry_idx][(batch_idx, row_idx)] = HOT-corrected real ctid.
        // Populated here and consumed in the final output column write below so that
        // fetch_tuple_direct (which does NOT follow HOT chains) gets the right address.
        // Declared outside the if block so it remains in scope for the output step.
        let corrected_lookup: Vec<HashMap<(usize, usize), u64>>;

        if !self.visibility_entries.is_empty() && !candidates.is_empty() {
            let row_keys: Vec<(usize, usize)> =
                candidates.iter().map(|(bi, ri, _)| (*bi, *ri)).collect();
            let (visible_mask, corrected_per_entry) = self.check_rows_visible(&row_keys)?;
            // Build per-entry (batch_idx, row_idx) → HOT-corrected ctid lookup.
            corrected_lookup = corrected_per_entry
                .into_iter()
                .map(|ctids| {
                    row_keys
                        .iter()
                        .zip(ctids)
                        .filter_map(|(key, opt)| opt.map(|c| (*key, c)))
                        .collect::<HashMap<_, _>>()
                })
                .collect();
            let invisible_count = visible_mask.iter().filter(|&&v| !v).count();
            if invisible_count > 0 {
                self.rows_filtered_invisible.add(invisible_count);
            }
            candidates = candidates
                .into_iter()
                .zip(visible_mask)
                .filter(|(_, visible)| *visible)
                .map(|(c, _)| c)
                .collect();
        } else {
            corrected_lookup = Vec::new();
        }

        if candidates.is_empty() {
            return Ok(None);
        }

        // 3. Materialize sort column values for each candidate and build a
        //    second RowConverter using materialized data types (Utf8View/BinaryView
        //    for deferred columns, original type for non-deferred).
        let materialized_sort_fields: Vec<SortField> = self
            .sort_exprs
            .iter()
            .map(|expr| {
                let is_deferred = expr
                    .expr
                    .as_any()
                    .downcast_ref::<datafusion::physical_expr::expressions::Column>()
                    .and_then(|c| {
                        self.deferred_columns
                            .iter()
                            .find(|d| d.sort_col_idx == c.index())
                    });
                let data_type = if let Some(deferred) = is_deferred {
                    let col = self.ffhelper.column(0, deferred.canonical.ff_index);
                    match col {
                        FFType::Bytes(_) => arrow_schema::DataType::BinaryView,
                        _ => arrow_schema::DataType::Utf8View,
                    }
                } else {
                    expr.expr
                        .data_type(&self.schema)
                        .unwrap_or(arrow_schema::DataType::Utf8View)
                };
                SortField::new_with_options(data_type, expr.options)
            })
            .collect();

        let mat_row_converter = RowConverter::new(materialized_sort_fields)?;

        // For each candidate, resolve materialized ScalarValues and convert to OwnedRow.
        let mut mat_rows: Vec<(usize, OwnedRow)> = Vec::with_capacity(candidates.len());

        for (idx, (batch_idx, row_idx, ord_info)) in candidates.iter().enumerate() {
            let mut values = Vec::with_capacity(self.sort_exprs.len());

            for (i, sort_expr) in self.sort_exprs.iter().enumerate() {
                let is_deferred = sort_expr
                    .expr
                    .as_any()
                    .downcast_ref::<datafusion::physical_expr::expressions::Column>()
                    .and_then(|c| {
                        self.deferred_columns
                            .iter()
                            .find(|d| d.sort_col_idx == c.index())
                    });

                let value = if let Some(deferred) = is_deferred {
                    if let Some((seg_ord, ord_row)) = ord_info {
                        // Ordinal survivor: convert ordinal back to string.
                        let arrays = self
                            .row_converter
                            .convert_rows(std::iter::once(ord_row.row()))
                            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
                        let term_ord = arrays[i]
                            .as_any()
                            .downcast_ref::<UInt64Array>()
                            .map(|a| a.value(0));
                        if let Some(term_ord) = term_ord {
                            let col = self.ffhelper.column(*seg_ord, deferred.canonical.ff_index);
                            match col {
                                FFType::Text(str_col) => {
                                    let mut s = String::new();
                                    if str_col.ord_to_str(term_ord, &mut s).is_ok() {
                                        ScalarValue::Utf8View(Some(s))
                                    } else {
                                        ScalarValue::Utf8View(None)
                                    }
                                }
                                FFType::Bytes(bytes_col) => {
                                    let mut b = Vec::new();
                                    if bytes_col.ord_to_bytes(term_ord, &mut b).is_ok() {
                                        ScalarValue::BinaryView(Some(b))
                                    } else {
                                        ScalarValue::BinaryView(None)
                                    }
                                }
                                _ => ScalarValue::Utf8View(None),
                            }
                        } else {
                            ScalarValue::Utf8View(None)
                        }
                    } else {
                        // Pass-through row: extract materialized value from
                        // UnionArray State 2 child.
                        let batch = &self.batches[*batch_idx];
                        let union_col = batch
                            .column(deferred.sort_col_idx)
                            .as_any()
                            .downcast_ref::<UnionArray>();
                        if let Some(union_arr) = union_col {
                            let type_ids = union_arr.type_ids();
                            let offsets = union_arr.offsets();
                            if let Some(offsets) = offsets {
                                if type_ids[*row_idx] == 2 {
                                    // State 2: materialized child
                                    let child = union_arr.child(2);
                                    let ci = offsets[*row_idx] as usize;
                                    ScalarValue::try_from_array(child, ci)
                                        .unwrap_or(ScalarValue::Utf8View(None))
                                } else {
                                    // NULL ordinal pass-through
                                    ScalarValue::Utf8View(None)
                                }
                            } else {
                                ScalarValue::Utf8View(None)
                            }
                        } else {
                            ScalarValue::Utf8View(None)
                        }
                    }
                } else {
                    // Non-deferred column: evaluate directly from the batch.
                    let batch = &self.batches[*batch_idx];
                    let val = sort_expr.expr.evaluate(batch)?;
                    let arr = val.into_array(batch.num_rows())?;
                    ScalarValue::try_from_array(&arr, *row_idx)
                        .unwrap_or(ScalarValue::Utf8View(None))
                };
                values.push(value);
            }

            // Convert ScalarValues to arrays and then to an OwnedRow.
            let arrays: Vec<ArrayRef> = values
                .iter()
                .map(|v| v.to_array())
                .collect::<Result<Vec<_>>>()?;
            let converted = mat_row_converter
                .convert_columns(&arrays)
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
            mat_rows.push((idx, converted.row(0).owned()));
        }

        // 4. Sort candidates by materialized OwnedRow and take top K.
        mat_rows.sort_by(|a, b| a.1.cmp(&b.1));
        mat_rows.truncate(self.k);

        if mat_rows.is_empty() {
            return Ok(None);
        }

        // 5. Emit a single sorted batch.
        //    Concatenate all buffered batches into one mega-batch, then use
        //    row indices to select and reorder the winners.
        let mut batch_offsets: Vec<usize> = Vec::with_capacity(self.batches.len());
        let mut running = 0usize;
        for batch in &self.batches {
            batch_offsets.push(running);
            running += batch.num_rows();
        }

        let mega_batch = if self.batches.len() == 1 {
            self.batches[0].clone()
        } else {
            concat_batches(&self.schema, &self.batches)
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?
        };

        // Compute global row index for each winner.
        let indices: Vec<usize> = mat_rows
            .iter()
            .map(|(candidate_idx, _)| {
                let (batch_idx, row_idx, _) = &candidates[*candidate_idx];
                batch_offsets[*batch_idx] + row_idx
            })
            .collect();

        // Use interleave to reorder columns. interleave expects (array_idx, row_idx)
        // pairs — with a single source array, array_idx is always 0.
        let interleave_indices: Vec<(usize, usize)> = indices.iter().map(|&ri| (0, ri)).collect();

        let mut output_columns = Vec::with_capacity(mega_batch.num_columns());
        for col in mega_batch.columns() {
            let col_refs: Vec<&dyn arrow_array::Array> = vec![col.as_ref()];
            let reordered = arrow_select::interleave::interleave(&col_refs, &interleave_indices)
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
            output_columns.push(reordered);
        }

        let mut result = RecordBatch::try_new(self.schema.clone(), output_columns)
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;

        // Write HOT-corrected real ctids to the absorbed ctid output columns.
        //
        // When SegmentedTopKRule absorbs a VisibilityFilterExec, the batches flowing
        // through come directly from HashJoinExec and therefore still carry packed
        // DocAddresses in the ctid_N columns.  We must resolve them before emitting
        // so downstream JoinScanState can pass a real ctid to fetch_tuple_direct.
        //
        // We use the HOT-corrected values from corrected_lookup (populated by
        // check_rows_visible above via heap_hot_search_buffer) rather than calling
        // materialize_deferred_ctid again.  materialize_deferred_ctid would return
        // the raw index ctid, which is wrong for rows whose heap tuple has been
        // moved by a HOT update: fetch_tuple_direct does NOT follow HOT chains, so
        // it would silently return no data or stale data for those rows.
        if !corrected_lookup.is_empty() {
            let mut columns: Vec<ArrayRef> = result.columns().to_vec();
            for (entry_idx, entry) in self.visibility_entries.iter().enumerate() {
                // Build the corrected-ctid array for the K winners in sorted order.
                // Every winner was visible (invisible rows were filtered out above),
                // so corrected_lookup[entry_idx] must contain an entry for each.
                let corrected: Vec<Option<u64>> = mat_rows
                    .iter()
                    .map(|(candidate_idx, _)| {
                        let (batch_idx, row_idx, _) = &candidates[*candidate_idx];
                        corrected_lookup[entry_idx]
                            .get(&(*batch_idx, *row_idx))
                            .copied()
                    })
                    .collect();
                columns[entry.col_idx] = Arc::new(UInt64Array::from(corrected)) as ArrayRef;
            }
            result = RecordBatch::try_new(self.schema.clone(), columns)
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
        }

        Ok(Some(result))
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;

    use std::collections::BTreeSet;

    use crate::index::fast_fields_helper::WhichFastField;
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::reader::index::SearchIndexReader;
    use crate::postgres::rel::PgSearchRelation;
    use crate::query::SearchQueryInput;
    use crate::scan::deferred_encode::{build_state_doc_address, deferred_union_data_type};
    use crate::scan::segmented_topk_exec::DeferredSortColumn;
    use crate::schema::SearchFieldType;

    use arrow_schema::{Field, Schema};
    use datafusion::execution::TaskContext;
    use datafusion::physical_expr::expressions::Column;
    use datafusion::physical_expr::{LexOrdering, PhysicalSortExpr};
    use datafusion::physical_plan::test::TestMemoryExec;
    use datafusion::physical_plan::ExecutionPlan;
    use futures::StreamExt;
    use pgrx::prelude::*;
    use proptest::prelude::*;

    fn setup_test_table() {
        Spi::run(
            r#"
            DROP TABLE IF EXISTS segmented_topk_test;
            CREATE TABLE segmented_topk_test (
                id SERIAL PRIMARY KEY,
                name TEXT,
                sort_col TEXT
            );
            INSERT INTO segmented_topk_test (name, sort_col)
            SELECT 'lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor ' ||
                   'incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis ' ||
                   'nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. ' ||
                   'Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu ' ||
                   'fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in ' ||
                   'culpa qui officia deserunt mollit anim id est laborum.',
                   'val_' || lpad(id::text, 6, '0')
            FROM generate_series(1, 35000) id;
            "#,
        )
        .expect("failed to setup test table");
    }

    #[pg_test]
    fn test_segmented_topk_exec() {
        setup_test_table();

        // Force the single builder to create many segments by artificially restricting memory.
        Spi::run("SET max_parallel_workers = 0;").unwrap();
        Spi::run("SET max_parallel_maintenance_workers = 0;").unwrap();
        Spi::run("SET maintenance_work_mem = '15MB';").unwrap();

        // Create an index with target_segment_count = 4 to guarantee multiple segments.
        Spi::run(
            r#"
            CREATE INDEX segmented_topk_test_idx ON segmented_topk_test 
            USING bm25 (id, name, sort_col) 
            WITH (
                key_field = 'id', 
                target_segment_count = 4, 
                text_fields = '{"sort_col": {"fast": true}}'
            );
            "#,
        )
        .unwrap();

        let index_oid =
            Spi::get_one::<pgrx::pg_sys::Oid>("SELECT 'segmented_topk_test_idx'::regclass;")
                .unwrap()
                .unwrap();
        let index_rel = PgSearchRelation::open(index_oid);

        let reader = SearchIndexReader::open(
            &index_rel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();

        assert_eq!(reader.total_segment_count(), 4);

        let fields = vec![
            WhichFastField::Named(
                "sort_col".to_string(),
                SearchFieldType::Text(pgrx::pg_sys::TEXTOID),
            ),
            WhichFastField::Named(
                "id".to_string(),
                SearchFieldType::I64(pgrx::pg_sys::INT4OID),
            ),
        ];
        let ffhelper = Arc::new(crate::index::fast_fields_helper::FFHelper::with_fields(
            &reader, &fields,
        ));

        let schema = Arc::new(Schema::new(vec![
            Field::new("sort_col", deferred_union_data_type(false), true),
            Field::new("id", arrow_schema::DataType::Int64, true),
        ]));

        let segment_readers = reader.segment_readers();

        let max_docs_per_segment: Vec<u32> =
            segment_readers.iter().map(|sr| sr.max_doc()).collect();

        // Proptest to pick random subsets of doc_ids from the existing segments
        proptest!(|(
            subset_selector in proptest::collection::vec(
                proptest::collection::vec(any::<bool>(), 0..1000),
                max_docs_per_segment.len()
            )
        )| {
            let mut batches = vec![];
            let mut all_selected_ids = BTreeSet::new();

            for (seg_ord, segment_reader) in segment_readers.iter().enumerate() {
                let max_doc = segment_reader.max_doc();
                let ffr = segment_reader.fast_fields();
                let id_col = ffr.i64("id").expect("id field missing");

                let mut doc_ids = vec![];

                // Use the random boolean selector to pick doc_ids
                let selectors = subset_selector.get(seg_ord);
                for doc_id in 0..max_doc {
                    // Default to selecting the document if we don't have enough booleans
                    let should_select = selectors.and_then(|s| s.get(doc_id as usize)).copied().unwrap_or(true);
                    if should_select {
                        doc_ids.push(doc_id);
                        let val = id_col.first(doc_id).unwrap_or_default();
                        all_selected_ids.insert(val);
                    }
                }

                if doc_ids.is_empty() {
                    continue;
                }

                let name_array = build_state_doc_address(seg_ord as u32, &doc_ids, false);
                let mut id_builder = arrow_array::builder::Int64Builder::with_capacity(doc_ids.len());

                for doc_id in &doc_ids {
                    let val = id_col.first(*doc_id).unwrap_or_default();
                    id_builder.append_value(val);
                }
                let id_array = Arc::new(id_builder.finish()) as ArrayRef;

                let batch = RecordBatch::try_new(schema.clone(), vec![name_array, id_array]).unwrap();
                batches.push(batch);
            }

            if batches.is_empty() {
                return Ok(());
            }

            let memory_exec = TestMemoryExec::try_new_exec(&[batches], schema.clone(), None).unwrap();

            let sort_exprs = LexOrdering::new(vec![
                PhysicalSortExpr {
                    expr: Arc::new(Column::new("sort_col", 0)),
                    options: datafusion::arrow::compute::SortOptions {
                        descending: false,
                        nulls_first: false,
                    },
                }
            ]).unwrap();

            let deferred_columns = vec![
                DeferredSortColumn {
                    sort_col_idx: 0,
                    canonical: crate::index::fast_fields_helper::CanonicalColumn {
                        indexrelid: index_oid.to_u32(),
                        ff_index: 0,
                    }
                }
            ];

            let topk_exec = SegmentedTopKExec::new(
                memory_exec,
                sort_exprs,
                deferred_columns,
                ffhelper.clone(),
                10,
                None,
            );

            let task_ctx = Arc::new(TaskContext::default());
            let mut stream = topk_exec.execute(0, task_ctx).unwrap();

            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();

            let mut result_ids = vec![];

            runtime.block_on(async {
                while let Some(batch) = stream.next().await {
                    let batch = batch.unwrap();
                    let col = batch.column(1); // 'id' column
                    let array = col.as_any().downcast_ref::<arrow_array::Int64Array>().unwrap();
                    for i in 0..array.len() {
                        if array.is_valid(i) {
                            result_ids.push(array.value(i));
                        }
                    }
                }
            });

            let expected_limit = all_selected_ids.len().min(10);
            prop_assert_eq!(result_ids.len(), expected_limit);

            // Because sort_col is lpad(id, 6, '0'), numeric sort matches string sort!
            // We just grab the smallest K items from our BTreeSet.
            let expected_ids: Vec<i64> = all_selected_ids.into_iter().take(expected_limit).collect();
            prop_assert_eq!(result_ids, expected_ids);
        });
    }
}
