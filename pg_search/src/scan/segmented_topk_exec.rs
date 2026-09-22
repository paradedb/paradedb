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
//! `SegmentedTopKExec` sits below the deferred lookup (`TantivyDecodeExec`, and the
//! `TantivyFetchExec` under it when the scan emits doc addresses) in the physical
//! plan. It operates on the packed deferred columns emitted by late materialization
//! (see `deferred_encode`):
//!   - State 0 (doc address): unpacks `(segment_ord, doc_id)` and bulk-fetches
//!     term ordinals via `FFHelper`.
//!   - State 1 (term ordinal): uses the ordinal directly (already resolved by
//!     pre-filter memoization or by the scan).
//!
//! For States 0 and 1, a per-segment Vec-based buffer (capacity 2×K) with
//! QuickSelect retains only the top K rows per segment. All batches are
//! collected during the input phase, and survivors are emitted in a single
//! pass once all input is consumed.
//!
//! ## Global threshold
//!
//! As rows are ingested, a global threshold is published to the scanner.
//! Once a segment's buffer undergoes its first QuickSelect (accumulating 2×K
//! rows), the K-th element's deferred ordinals are converted back to strings
//! via `FFHelper::ord_to_str` and published as a `DynamicFilterPhysicalExpr`.
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
//! Total ordinal-comparable rows reaching the deferred lookup:
//!
//!   sum_s(survivors_s) <= K * S  (when no boundary ties)
//!
//! where `S` is the number of segments. Pass-through rows (NULL
//! ordinals) are emitted immediately and are not bounded by K.
//!
//! **Compound sorts:** every sort column is used, not just the primary. The
//! per-segment buffer keys on the full compound `OwnedRow`, and the global
//! threshold is published as a lexicographic filter over all sort exprs — each
//! deferred column resolved via `ord_to_str`, non-deferred columns read
//! directly — so `ORDER BY val DESC, id ASC LIMIT 25` breaks ties on `id`
//! rather than retaining every row that shares the boundary `val`. Only rows
//! tied across the *entire* sort key are conservatively retained, per the
//! `survivors_s` bound above.

use crate::api::HashMap;
use crate::index::fast_fields_helper::{CanonicalColumn, FFHelper, FFIndex, FFType};
use crate::scan::deferred_encode::{DeferredColumn, DeferredValue};
use crate::scan::deferred_lookup::{LookupRebuildContext, open_rebuilt_ffhelper, rebuild_mvcc};
use crate::scan::execution_plan::UnsafeSendStream;
use arrow_array::{Array, ArrayRef, BooleanArray, RecordBatch, UInt64Array};
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
    ChildPushdownResult, FilterDescription, FilterPushdownPhase, FilterPushdownPropagation,
};
use datafusion::physical_plan::metrics::{
    Count, ExecutionPlanMetricsSet, MetricBuilder, MetricsSet,
};
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties, apply_expression_roots,
};
use std::sync::Arc;
use tantivy::termdict::TermOrdinal;
use tantivy::{DocId, SegmentOrdinal};

/// The Arrow type a deferred sort column materializes NULLs and values as.
///
/// Every segment of one index stores a column under the same type, but a segment whose
/// documents never carry a JSON path has no column at all, and asking that one would pick
/// a type the `RowConverter` then rejects for the segments that do have it.
fn deferred_sort_data_type(ffhelper: &FFHelper, ff_index: FFIndex) -> arrow_schema::DataType {
    (0..ffhelper.num_segments() as SegmentOrdinal)
        .find_map(|segment_ord| match ffhelper.column(segment_ord, ff_index) {
            FFType::Bytes(_) => Some(arrow_schema::DataType::BinaryView),
            FFType::Junk => None,
            _ => Some(arrow_schema::DataType::Utf8View),
        })
        .unwrap_or(arrow_schema::DataType::Utf8View)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DeferredSortColumn {
    pub sort_col_idx: usize,
    pub canonical: CanonicalColumn,
    /// How a dispatched fragment rebuilds the fast-field helper when the column's scan is not
    /// in its decoded subtree (the top-k above a network boundary).
    #[serde(default)]
    pub rebuild: Option<crate::scan::late_materialization::DeferredLookupRebuild>,
}

pub struct SegmentedTopKExec {
    input: Arc<dyn ExecutionPlan>,
    /// The sort expressions defining the Top K order.
    sort_exprs: LexOrdering,
    /// The deferred string/bytes columns that are part of the Top K order.
    deferred_columns: Vec<DeferredSortColumn>,
    /// FFHelper for Tantivy fast field access (shared with the deferred lookup nodes).
    ffhelper: Arc<FFHelper>,
    /// Maximum rows to keep per segment.
    k: usize,
    /// Dynamic filter pushed down through DataFusion's standard filter pushdown
    /// mechanism. Updated at runtime with a global threshold (materialized
    /// string literals) that the scanner's `try_rewrite_binary` translates to
    /// per-segment ordinal bounds. Stored as `Arc<dyn PhysicalExpr>` so the
    /// same instance can round-trip through the deduplicating proto converter
    /// on worker dispatch and stay identity-shared with the scans below.
    dynamic_filter: Arc<dyn PhysicalExpr>,
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
    pub fn new(
        input: Arc<dyn ExecutionPlan>,
        sort_exprs: LexOrdering,
        deferred_columns: Vec<DeferredSortColumn>,
        ffhelper: Arc<FFHelper>,
        k: usize,
        parent_filter: Option<Arc<dyn PhysicalExpr>>,
    ) -> Self {
        use datafusion::physical_expr::expressions::lit;

        let mut eq_props = EquivalenceProperties::new(input.schema());
        eq_props.add_ordering(sort_exprs.clone());
        let output_partitioning = input.properties().output_partitioning().clone();

        let properties = Arc::new(PlanProperties::new(
            eq_props,
            output_partitioning,
            EmissionType::Final,
            Boundedness::Bounded,
        ));

        // When the enclosing SortExec has already minted a `DynamicFilterPhysicalExpr`
        // (via `SortExec::with_filter`), take ownership of it instead of minting a
        // fresh one. The scans below have already been wired to this filter by the
        // standard `FilterPushdown` pass, so subsequent `update()`s from this node's
        // heap propagate to the same recipients the SortExec would have driven. If no
        // parent filter is provided (e.g. worker decode path with no shipped filter),
        // fall back to minting. See #5635.
        let dynamic_filter: Arc<dyn PhysicalExpr> = parent_filter.unwrap_or_else(|| {
            let children: Vec<Arc<dyn PhysicalExpr>> =
                sort_exprs.iter().map(|e| Arc::clone(&e.expr)).collect();
            Arc::new(DynamicFilterPhysicalExpr::new(children, lit(true)))
        });

        Self {
            input,
            sort_exprs,
            deferred_columns,
            ffhelper,
            k,
            dynamic_filter,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
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
                    .downcast_ref::<datafusion::physical_expr::expressions::Column>()
                    .and_then(|c| {
                        deferred_columns
                            .iter()
                            .find(|d| d.sort_col_idx == c.index())
                    });
                let data_type = if let Some(deferred) = is_deferred {
                    deferred_sort_data_type(ffhelper, deferred.canonical.ff_index)
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

    /// Serialize for leader dispatch. The `ffhelper` is live and doesn't travel; the worker
    /// pulls it from the scan in its decoded subtree. The `dynamic_filter` ships as an
    /// identity-stamped proto expression so that decoding through the fragment's
    /// deduplicating proto converter re-shares one instance with the same filter shipped
    /// by the scan below (see #5766), keeping the worker's top-k threshold wired to its
    /// scan's `PreFilter`. `decoders`/`properties` are derived.
    pub(crate) fn encode_for_dispatch(
        &self,
        proto_converter: &dyn datafusion_proto::physical_plan::PhysicalProtoConverterExtension,
    ) -> Result<Vec<u8>> {
        let codec = datafusion_proto::physical_plan::DefaultPhysicalExtensionCodec {};
        let proto_conv = datafusion_proto::physical_plan::DefaultPhysicalProtoConverter {};
        let sort_proto = datafusion_proto::physical_plan::to_proto::serialize_physical_sort_exprs(
            self.sort_exprs.iter().cloned(),
            &codec,
            &proto_conv,
        )?;
        let sort_bytes: Vec<Vec<u8>> = sort_proto
            .iter()
            .map(prost::Message::encode_to_vec)
            .collect();
        // Ship the dynamic filter as a proto expression. The deduplicating converter
        // re-shares it on decode with the scans below via `expr_id` (see #5766).
        let dynamic_filter_bytes = {
            let node = proto_converter.physical_expr_to_proto(&self.dynamic_filter, &codec)?;
            prost::Message::encode_to_vec(&node)
        };
        let payload = (
            sort_bytes,
            self.deferred_columns.clone(),
            self.k,
            dynamic_filter_bytes,
        );
        serde_json::to_vec(&payload).map_err(|e| {
            DataFusionError::Internal(format!("SegmentedTopKExec dispatch: serialize: {e}"))
        })
    }

    pub(crate) fn decode_for_dispatch(
        buf: &[u8],
        input: Arc<dyn ExecutionPlan>,
        ffhelpers: HashMap<u32, Arc<FFHelper>>,
        ctx: &TaskContext,
        parallel_state: Option<*mut crate::postgres::ParallelScanState>,
        proto_converter: &dyn datafusion_proto::physical_plan::PhysicalProtoConverterExtension,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let (sort_bytes, deferred_columns, k, dynamic_filter_bytes): (
            Vec<Vec<u8>>,
            Vec<DeferredSortColumn>,
            usize,
            Vec<u8>,
        ) = serde_json::from_slice(buf).map_err(|e| {
            DataFusionError::Internal(format!("SegmentedTopKExec dispatch: deserialize: {e}"))
        })?;
        // The deferred sort columns all resolve against one index (the sorted relation), and
        // `ff_index` is relative to that index's fast-field list. A join leaves the other
        // index's scan in the same subtree, so pick the helper by `indexrelid` instead of
        // grabbing whichever scan comes first. When that scan is behind a network boundary
        // (no helper in the subtree), rebuild one over the same segment view the scan's
        // reader opens, so segment ordering matches the ordinals the producers packed.
        let ffhelper = match deferred_columns.first() {
            Some(first) => match ffhelpers.get(&first.canonical.indexrelid).cloned() {
                Some(helper) => helper,
                None => {
                    let entries: Vec<_> = deferred_columns
                        .iter()
                        .filter_map(|d| d.rebuild.as_ref().map(|rb| (d.canonical.ff_index, rb)))
                        .collect();
                    let (_, first_rb) = entries.first().ok_or_else(|| {
                        DataFusionError::Internal(format!(
                            "SegmentedTopKExec dispatch: no ffhelper for indexrelid {} and no \
                             rebuild info",
                            first.canonical.indexrelid
                        ))
                    })?;
                    let mvcc = rebuild_mvcc(LookupRebuildContext { parallel_state }, first_rb)?;
                    open_rebuilt_ffhelper(first.canonical.indexrelid, &entries, mvcc)?
                }
            },
            None => ffhelpers
                .into_values()
                .next()
                .unwrap_or_else(|| Arc::new(FFHelper::empty())),
        };
        let sort_proto = sort_bytes
            .iter()
            .map(|b| {
                <datafusion_proto::protobuf::PhysicalSortExprNode as prost::Message>::decode(
                    b.as_slice(),
                )
            })
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| {
                DataFusionError::Internal(format!("SegmentedTopKExec dispatch: sort decode: {e}"))
            })?;
        let codec = datafusion_proto::physical_plan::DefaultPhysicalExtensionCodec {};
        let proto_conv = datafusion_proto::physical_plan::DefaultPhysicalProtoConverter {};
        let decode_ctx =
            datafusion_proto::physical_plan::PhysicalPlanDecodeContext::new(ctx, &codec);
        let input_schema = input.schema();
        let exprs = datafusion_proto::physical_plan::from_proto::parse_physical_sort_exprs(
            &sort_proto,
            &decode_ctx,
            input_schema.as_ref(),
            &proto_conv,
        )?;
        let sort_exprs = LexOrdering::new(exprs).ok_or_else(|| {
            DataFusionError::Internal("SegmentedTopKExec dispatch: empty sort order".into())
        })?;
        // Decode the shipped dynamic filter through the deduplicating proto converter so
        // that the returned Arc is the same instance the scans below decoded (see #5766).
        // This re-wires the worker's top-k threshold to its own scan's `PreFilter` without
        // needing a trailing `FilterPushdown(Post)` pass on the decoded fragment.
        let parent_filter: Option<Arc<dyn PhysicalExpr>> = {
            let node = <datafusion_proto::protobuf::PhysicalExprNode as prost::Message>::decode(
                dynamic_filter_bytes.as_slice(),
            )
            .map_err(|e| {
                DataFusionError::Internal(format!(
                    "SegmentedTopKExec dispatch: dynamic filter decode: {e}"
                ))
            })?;
            let expr = proto_converter.proto_to_physical_expr(
                &node,
                input.schema().as_ref(),
                &decode_ctx,
            )?;
            Some(expr)
        };
        Ok(Arc::new(SegmentedTopKExec::new(
            input,
            sort_exprs,
            deferred_columns,
            ffhelper,
            k,
            parent_filter,
        )))
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

    fn properties(&self) -> &Arc<PlanProperties> {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.input]
    }

    fn apply_expressions(
        &self,
        f: &mut dyn FnMut(
            &Arc<dyn PhysicalExpr>,
        ) -> Result<datafusion::common::tree_node::TreeNodeRecursion>,
    ) -> Result<datafusion::common::tree_node::TreeNodeRecursion> {
        apply_expression_roots([&self.dynamic_filter], f)
    }

    fn with_new_children(
        self: Arc<Self>,
        mut children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // Preserve the existing dynamic filter so that filter pushdown wiring
        // (which already holds a reference) stays connected.
        let new = SegmentedTopKExec::new(
            children.remove(0),
            self.sort_exprs.clone(),
            self.deferred_columns.clone(),
            Arc::clone(&self.ffhelper),
            self.k,
            Some(Arc::clone(&self.dynamic_filter)),
        );
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

        let input_schema = self.input.schema();

        // Build the row converter
        let sort_fields = self
            .sort_exprs
            .iter()
            .map(|expr| {
                let expr_type = expr.expr.data_type(&input_schema)?;
                // If it's a deferred column, we treat its sorting type as UInt64 (the ordinal type).
                let data_type = if expr
                    .expr
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
            &input_schema,
        )?;

        let mut state = SegmentedTopKState {
            sort_exprs: self.sort_exprs.clone(),
            deferred_columns: self.deferred_columns.clone(),
            ffhelper: Arc::clone(&self.ffhelper),
            k: self.k,
            schema: input_schema,
            row_converter,
            segment_bufs: Vec::new(),
            segment_cutoffs: Vec::new(),
            dynamic_filter: Arc::clone(&self.dynamic_filter),
            batches: Vec::new(),
            pass_through_rows: Vec::new(),
            last_segment_cutoffs: Vec::new(),
            mat_row_converter,
            last_published_global: None,
            rows_input,
            rows_output,
            segments_seen,
            pass_through_scratch: Vec::new(),
            row_to_seg_scratch: Vec::new(),
            sort_arrays_scratch: Vec::with_capacity(self.sort_exprs.len()),
        };

        let stream_gen = async_stream::try_stream! {
            use futures::StreamExt;
            while let Some(batch_res) = input_stream.next().await {
                let batch = batch_res?;
                state.rows_input.add(batch.num_rows());

                // Store the batch BEFORE collecting. Cloning a RecordBatch only bumps the
                // column Arcs.
                let batch_idx = state.batches.len();
                state.batches.push(batch);
                let batch = state.batches[batch_idx].clone();
                state.collect_batch(&batch, batch_idx)?;
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

        let schema = self.input.schema();
        // Route parent filters to our child based on column compatibility,
        // and add our own dynamic filter as a self-filter.
        let child_desc = crate::scan::filter_pushdown::schema_preserving_child_filter_description(
            &parent_filters,
            &schema,
            None,
        )?
        .with_self_filter(Arc::clone(&self.dynamic_filter));
        Ok(FilterDescription::new().with_child(child_desc))
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

/// One segment's rolling buffer of top-K candidates.
///
/// `rows` holds `(batch_idx, row_idx, sort row)` entries whose locations refer to
/// [`SegmentedTopKState::batches`]. The buffer fills up to its capacity (`2 * K`)
/// and is then pruned back to its K best rows by [`SegmentedTopKState::truncate_top_k`].
#[derive(Default)]
struct SegmentBuf {
    rows: Vec<(usize, usize, OwnedRow)>,
}

/// A row that had a NULL ordinal in at least one deferred sort column, so it
/// bypasses the ordinal-tracked segment buffers and is carried straight to
/// the final sort. `row_val` is the same compound ordinal key `collect_batch`
/// builds for every row, so a NULL in one deferred column leaves the other
/// columns' ordinals intact.
struct PassThroughRow {
    batch_idx: usize,
    row_idx: usize,
    /// `None` when the row is NULL in every deferred sort column.
    seg_ord: Option<SegmentOrdinal>,
    row_val: OwnedRow,
}

struct SegmentedTopKState {
    sort_exprs: LexOrdering,
    deferred_columns: Vec<DeferredSortColumn>,
    ffhelper: Arc<FFHelper>,
    k: usize,
    schema: SchemaRef,
    row_converter: RowConverter,
    /// Per-segment rolling buffers, indexed by `SegmentOrdinal` (dense, 0..N).
    /// Filled in `collect_batch`; pruned to the K best rows by `truncate_top_k`
    /// whenever a buffer reaches its capacity.
    segment_bufs: Vec<Option<SegmentBuf>>,
    /// Per-segment K-th best row (the cutoff threshold) after the most recent
    /// `truncate_top_k`, indexed by `SegmentOrdinal`. `None` for segments that have not
    /// yet accumulated K rows.
    segment_cutoffs: Vec<Option<OwnedRow>>,
    /// Dynamic filter updated with global thresholds (materialized strings).
    /// Pushed down through DataFusion's standard filter pushdown to the scanner.
    /// Held as `Arc<dyn PhysicalExpr>` so it stays identity-shared with the
    /// worker-decoded scan filters (see the exec's field for the rationale).
    dynamic_filter: Arc<dyn PhysicalExpr>,
    /// Buffered batches during the collection phase.
    batches: Vec<RecordBatch>,
    /// Buffered pass-through rows (had a NULL ordinal in at least one deferred
    /// sort column) that bypass ordinal comparison. Included in the final sort
    /// with per-column ordinals so a NULL in one column does not force the
    /// other columns to NULL.
    pass_through_rows: Vec<PassThroughRow>,

    /// For each segment that has a cutoff, we cache the resolved values of its current
    /// K-th best row (the cutoff threshold). Indexed by `SegmentOrdinal`; `None` for
    /// segments without a resolved cutoff yet.
    /// Tuple: (local ordinal OwnedRow, materialized ScalarValues, materialized OwnedRow)
    last_segment_cutoffs: Vec<Option<(OwnedRow, Vec<datafusion::common::ScalarValue>, OwnedRow)>>,

    /// Row converter for materialized sorts, used to compare resolved thresholds lexicographically.
    mat_row_converter: RowConverter,

    /// Cache of the last published global threshold to avoid redundant filter updates.
    /// Stores the best of the worst materialized rows across segments.
    last_published_global: Option<OwnedRow>,

    rows_input: Count,
    rows_output: Count,
    /// Counts segments that had rows participating in ordinal comparison (States 0+1).
    /// Segments with only NULLs are not counted.
    segments_seen: Count,

    /// Scratch buffers to avoid per-batch allocation
    pass_through_scratch: Vec<bool>,
    row_to_seg_scratch: Vec<Option<SegmentOrdinal>>,
    sort_arrays_scratch: Vec<ArrayRef>,
}

impl SegmentedTopKState {
    /// Return a mutable reference to the per-segment slot at `idx`, growing the
    /// vector with `None`s as needed. `SegmentOrdinal` is dense (0..N), so these
    /// per-segment vectors are indexed directly by the ordinal.
    fn ensure_slot<T>(vec: &mut Vec<Option<T>>, idx: usize) -> &mut Option<T> {
        if idx >= vec.len() {
            vec.resize_with(idx + 1, || None);
        }
        &mut vec[idx]
    }

    fn get_or_create_segment_buf(&mut self, seg_idx: usize) -> &mut SegmentBuf {
        let slot = Self::ensure_slot(&mut self.segment_bufs, seg_idx);
        if slot.is_none() {
            self.segments_seen.add(1);
        }
        slot.get_or_insert_with(SegmentBuf::default)
    }

    /// Per-segment buffer capacity: `2 * K`.
    fn buffer_capacity(&self) -> usize {
        2 * self.k
    }

    /// Prune one segment's buffer down to its K best rows:
    ///
    /// 1. QuickSelect the K best rows in the buffer,
    /// 2. record the K-th best as the segment cutoff and truncate the buffer to K,
    /// 3. publish the (possibly improved) global threshold.
    fn truncate_top_k(&mut self, seg_idx: usize) -> Result<()> {
        if self.k == 0 {
            return Ok(());
        }

        let cutoff = {
            let Some(buf) = self.segment_bufs.get_mut(seg_idx).and_then(|b| b.as_mut()) else {
                return Ok(());
            };
            if buf.rows.len() < self.k {
                return Ok(());
            }
            buf.rows
                .select_nth_unstable_by(self.k - 1, |a, b| a.2.cmp(&b.2));
            let cutoff = buf.rows[self.k - 1].2.clone();
            buf.rows.truncate(self.k);
            cutoff
        };
        *Self::ensure_slot(&mut self.segment_cutoffs, seg_idx) = Some(cutoff);

        self.publish_global_threshold()
    }

    /// Ingest a single batch: extract ordinals, update per-segment buffers,
    /// and publish thresholds. The batch is buffered for the final emission
    /// phase. Pass-through rows (NULL ordinals) are buffered
    /// in `pass_through_rows` for the final sort + limit.
    fn collect_batch(&mut self, batch: &RecordBatch, batch_idx: usize) -> Result<()> {
        let num_rows = batch.num_rows();
        self.pass_through_scratch.clear();
        self.pass_through_scratch.resize(num_rows, false);
        self.row_to_seg_scratch.clear();
        self.row_to_seg_scratch.resize(num_rows, None);
        let mut deferred_ords: HashMap<usize, Vec<Option<TermOrdinal>>> = HashMap::default();

        for deferred_col in &self.deferred_columns {
            let global_term_ords = Self::extract_deferred_ordinals(
                &self.ffhelper,
                batch,
                deferred_col,
                num_rows,
                &mut self.pass_through_scratch,
                &mut self.row_to_seg_scratch,
            )?;
            deferred_ords.insert(deferred_col.sort_col_idx, global_term_ords);
        }

        // Build the evaluation arrays for the RowConverter. The per-column
        // ordinals land in the converted row, so a NULL in one deferred column
        // no longer blanks the others and nothing downstream needs the map.
        self.sort_arrays_scratch.clear();
        for expr in &self.sort_exprs {
            let col_idx = expr
                .expr
                .downcast_ref::<datafusion::physical_expr::expressions::Column>()
                .map(|c| c.index());

            if let Some(Some(ords)) = col_idx.map(|idx| deferred_ords.remove(&idx)) {
                // Use our artificially constructed ordinals array
                let ords_array = Arc::new(UInt64Array::from(ords)) as ArrayRef;
                self.sort_arrays_scratch.push(ords_array);
            } else {
                let val = expr.expr.evaluate(batch)?;
                self.sort_arrays_scratch.push(val.into_array(num_rows)?);
            }
        }

        let converted_rows = self
            .row_converter
            .convert_columns(&self.sort_arrays_scratch)?;

        let capacity = self.buffer_capacity();

        for row_idx in 0..num_rows {
            if self.pass_through_scratch[row_idx] {
                // Keep the compound key the converter already built for this
                // row. A row that is NULL in one deferred column but not in
                // another keeps that column's segment for the final decode; a
                // row with no segment is NULL in every deferred column and
                // needs no dictionary.
                if self.row_to_seg_scratch[row_idx].is_none() {
                    debug_assert!(
                        self.sort_arrays_scratch
                            .iter()
                            .any(|arr| arr.is_null(row_idx)),
                        "pass-through row without resolved segment must have at least one NULL sort column"
                    );
                }
                self.pass_through_rows.push(PassThroughRow {
                    batch_idx,
                    row_idx,
                    seg_ord: self.row_to_seg_scratch[row_idx],
                    row_val: converted_rows.row(row_idx).owned(),
                });
                continue;
            }

            if let Some(seg_idx) = self.row_to_seg_scratch[row_idx].map(|s| s as usize) {
                let row_view = converted_rows.row(row_idx);

                // Pre-filter: rows already worse than this segment's cutoff cannot
                // enter the top K, so drop them before they reach the buffer.
                if self
                    .segment_cutoffs
                    .get(seg_idx)
                    .and_then(|c| c.as_ref())
                    .is_some_and(|cutoff| row_view.as_ref() > cutoff.as_ref())
                {
                    continue;
                }

                let buf_len = {
                    let buf = self.get_or_create_segment_buf(seg_idx);
                    buf.rows.push((batch_idx, row_idx, row_view.owned()));
                    buf.rows.len()
                };

                if self.k > 0 && buf_len >= capacity {
                    self.truncate_top_k(seg_idx)?;
                }
            }
        }

        Ok(())
    }

    /// Helper to extract term ordinals from a deferred column.
    /// Mutates `pass_through` for rows that contain NULLs, and populates `row_to_seg` mapping.
    /// A NULL row has no segment: it is a NULL value or a row an outer join null-extended.
    fn extract_deferred_ordinals(
        ffhelper: &FFHelper,
        batch: &RecordBatch,
        deferred_col: &DeferredSortColumn,
        num_rows: usize,
        pass_through: &mut [bool],
        row_to_seg: &mut [Option<SegmentOrdinal>],
    ) -> Result<Vec<Option<TermOrdinal>>> {
        let column = batch.column(deferred_col.sort_col_idx);
        let deferred = DeferredColumn::try_new(column.as_ref()).map_err(|e| {
            DataFusionError::Internal(format!(
                "SegmentedTopKExec: sort column at index {}: {e}",
                deferred_col.sort_col_idx
            ))
        })?;

        let mut global_term_ords: Vec<Option<TermOrdinal>> = vec![None; num_rows];
        let mut state0_by_seg: HashMap<SegmentOrdinal, Vec<(usize, DocId)>> = HashMap::default();
        for (row_idx, value) in deferred.values().enumerate() {
            match value {
                DeferredValue::DocAddress(doc_address) => {
                    state0_by_seg
                        .entry(doc_address.segment_ord)
                        .or_default()
                        .push((row_idx, doc_address.doc_id));
                    row_to_seg[row_idx] = Some(doc_address.segment_ord);
                }
                DeferredValue::TermOrdinal {
                    segment_ord,
                    term_ord,
                } => {
                    row_to_seg[row_idx] = Some(segment_ord);
                    global_term_ords[row_idx] = Some(term_ord);
                }
                DeferredValue::Null => pass_through[row_idx] = true,
            }
        }

        // Bulk-fetch term ordinals for State 0 rows via FFHelper
        for (seg_ord, rows) in state0_by_seg {
            let doc_ids: Vec<DocId> = rows.iter().map(|(_, doc_id)| *doc_id).collect();
            let mut term_ords: Vec<Option<TermOrdinal>> = vec![None; doc_ids.len()];

            let col = ffhelper.column(seg_ord, deferred_col.canonical.ff_index);
            match col {
                FFType::Text(str_col) => {
                    str_col.ords().first_vals(&doc_ids, &mut term_ords);
                }
                FFType::Bytes(bytes_col) => {
                    bytes_col.ords().first_vals(&doc_ids, &mut term_ords);
                }
                // No column in this segment: every row stays NULL.
                FFType::Junk => {}
                _ => {
                    panic!(
                        "SegmentedTopKExec: ff_index {} is not a Text or Bytes dictionary column \
                             — the optimizer should never plan this node for non-dictionary columns",
                        deferred_col.canonical.ff_index
                    );
                }
            }

            for (i, (row_idx, _)) in rows.into_iter().enumerate() {
                match term_ords[i] {
                    Some(ord) => global_term_ords[row_idx] = Some(ord),
                    None => pass_through[row_idx] = true,
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
        use datafusion::physical_expr::expressions::{BinaryExpr, is_not_null, is_null, lit};

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
        let full_segment_cutoffs: Vec<(SegmentOrdinal, OwnedRow)> = self
            .segment_cutoffs
            .iter()
            .enumerate()
            .filter_map(|(i, cutoff)| cutoff.as_ref().map(|c| (i as SegmentOrdinal, c.clone())))
            .collect();

        for (seg_ord, worst_local) in full_segment_cutoffs {
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

        if changed
            && let Some(expr) = Self::build_lexicographic_filter(&self.sort_exprs, &best_values)
            && let Some(df) = self
                .dynamic_filter
                .downcast_ref::<DynamicFilterPhysicalExpr>()
        {
            let _ = df.update(expr);
            self.last_published_global = Some(best_row);
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
        if let Some((cached_local, vals, row)) = self
            .last_segment_cutoffs
            .get(seg_ord as usize)
            .and_then(|c| c.as_ref())
            && cached_local == worst_local
        {
            return Ok((vals.clone(), row.clone()));
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
        *Self::ensure_slot(&mut self.last_segment_cutoffs, seg_ord as usize) =
            Some((worst_local.clone(), values.clone(), mat_row.clone()));
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
                self.materialize_deferred_ordinal(seg_ord, term_ord, deferred)?
            } else {
                ScalarValue::try_from_array(&arrays[i], 0)?
            };
            values.push(value);
        }
        Ok(values)
    }

    /// Compact the stored batches by discarding rows no longer referenced by any
    /// per-segment buffer (or pass-through). This bounds memory at O(K * segments)
    /// instead of O(N) for large inputs — analogous to the batch compaction
    /// step in upstream DataFusion Top K.
    fn maybe_compact(&mut self) -> Result<()> {
        // Fire only when the stored batches hold at least twice as many rows as are
        // still referenced, so each compaction at least halves the stored rows
        // (amortized O(1) work per input row). The floor keeps small inputs from
        // ever compacting, mirroring the previous 4 * K * segments trigger.
        let referenced: usize = self
            .segment_bufs
            .iter()
            .flatten()
            .map(|b| b.rows.len())
            .sum::<usize>()
            + self.pass_through_rows.len();
        let stored: usize = self.batches.iter().map(|b| b.num_rows()).sum();
        let num_segments = self.segment_bufs.iter().flatten().count().max(1);
        let floor = 4 * self.k * num_segments;
        if stored < (2 * referenced).max(floor) {
            return Ok(());
        }

        // Eagerly truncate every buffer first so compaction works with fresh top-K
        // survivors, and buffers holding more than K rows are pruned before their
        // rows are copied.
        for seg_idx in 0..self.segment_bufs.len() {
            let has_rows = self.segment_bufs[seg_idx]
                .as_ref()
                .is_some_and(|b| !b.rows.is_empty());
            if has_rows {
                self.truncate_top_k(seg_idx)?;
            }
        }

        // Survivors are exactly the rows still referenced by a buffer or pass-through.
        let mut survivors = crate::api::HashSet::default();
        for buf in self.segment_bufs.iter().flatten() {
            for (batch_idx, row_idx, _) in &buf.rows {
                survivors.insert((*batch_idx, *row_idx));
            }
        }
        for pt in &self.pass_through_rows {
            survivors.insert((pt.batch_idx, pt.row_idx));
        }

        if survivors.is_empty() {
            self.batches.clear();
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

        let compacted = concat_batches(&self.schema, &filtered_batches)
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;

        // Remap buffer rows and pass_through_rows into the single compacted batch.
        for buf in self.segment_bufs.iter_mut().flatten() {
            for entry in &mut buf.rows {
                let new_ri = mapping[&(entry.0, entry.1)];
                entry.0 = 0;
                entry.1 = new_ri;
            }
        }
        for entry in &mut self.pass_through_rows {
            let new_ri = mapping[&(entry.batch_idx, entry.row_idx)];
            entry.batch_idx = 0;
            entry.row_idx = new_ri;
        }

        self.batches = vec![compacted];
        Ok(())
    }

    /// Resolve a `(seg_ord, term_ord)` pair on a deferred sort column to the
    /// materialized `ScalarValue` that `mat_row_converter` expects.
    ///
    /// A failed lookup, or a column that is not a dictionary, is an error
    /// rather than a NULL: a NULL here would sort as a NULL and silently
    /// change the order of the result.
    fn materialize_deferred_ordinal(
        &self,
        seg_ord: SegmentOrdinal,
        term_ord: TermOrdinal,
        deferred: &DeferredSortColumn,
    ) -> Result<datafusion::common::ScalarValue> {
        use datafusion::common::ScalarValue;
        match self.ffhelper.column(seg_ord, deferred.canonical.ff_index) {
            FFType::Text(str_col) => {
                let mut s = String::new();
                if !str_col.ord_to_str(term_ord, &mut s).map_err(|e| {
                    DataFusionError::Internal(format!("Failed to resolve string ordinal: {e}"))
                })? {
                    return Err(DataFusionError::Internal(format!(
                        "SegmentedTopKExec: term ordinal {term_ord} was not found in segment \
                         {seg_ord} for fast-field index {}",
                        deferred.canonical.ff_index
                    )));
                }
                Ok(ScalarValue::Utf8View(Some(s)))
            }
            FFType::Bytes(bytes_col) => {
                let mut b = Vec::new();
                if !bytes_col.ord_to_bytes(term_ord, &mut b).map_err(|e| {
                    DataFusionError::Internal(format!("Failed to resolve bytes ordinal: {e}"))
                })? {
                    return Err(DataFusionError::Internal(format!(
                        "SegmentedTopKExec: term ordinal {term_ord} was not found in segment \
                         {seg_ord} for fast-field index {}",
                        deferred.canonical.ff_index
                    )));
                }
                Ok(ScalarValue::BinaryView(Some(b)))
            }
            _ => Err(DataFusionError::Internal(
                "Unexpected column type for deferred field".to_string(),
            )),
        }
    }

    /// Perform the final sort + limit after all input is consumed.
    ///
    ///
    /// Steps:
    /// 1. Collect candidates from the per-segment buffers.
    /// 2. Merge them with pass-through rows into the candidate set.
    /// 3. Materialize sort column values for each candidate.
    /// 4. Sort candidates by materialized values, take top K.
    /// 5. Emit a single sorted batch.
    fn emit_final_topk(&mut self) -> Result<Option<RecordBatch>> {
        use datafusion::common::ScalarValue;

        // 1. Collect candidates: every row still held in a per-segment buffer. Each
        //    buffer holds its segment's current top K plus any not-yet-truncated
        //    recent rows, all within the segment cutoff (enforced on insert), so the
        //    true per-segment top K is always a subset of the buffer.
        //
        // Ordinal-tracked survivors and pass-through rows carry the same compound
        // key, so both resolve through one path below.
        type Candidate = (usize, usize, Option<SegmentOrdinal>, OwnedRow);
        let mut candidates: Vec<Candidate> = Vec::new();

        for (seg_idx, slot) in self.segment_bufs.iter().enumerate() {
            let Some(buf) = slot else { continue };
            for (batch_idx, row_idx, row_val) in &buf.rows {
                candidates.push((
                    *batch_idx,
                    *row_idx,
                    Some(seg_idx as SegmentOrdinal),
                    row_val.clone(),
                ));
            }
        }

        // Always include pass-through rows (had a NULL ordinal in at least one
        // deferred column).
        for pt in &self.pass_through_rows {
            candidates.push((pt.batch_idx, pt.row_idx, pt.seg_ord, pt.row_val.clone()));
        }

        if candidates.is_empty() {
            return Ok(None);
        }

        // 3. Materialize sort column values for each candidate and build a
        //    second RowConverter using materialized data types (Utf8View/BinaryView
        //    for deferred columns, original type for non-deferred).
        struct SortCol<'a> {
            expr: &'a datafusion::physical_expr::PhysicalSortExpr,
            deferred: Option<&'a DeferredSortColumn>,
            mat_type: arrow_schema::DataType,
        }

        let sort_cols: Vec<SortCol> = self
            .sort_exprs
            .iter()
            .map(|expr| {
                let deferred = expr
                    .expr
                    .downcast_ref::<datafusion::physical_expr::expressions::Column>()
                    .and_then(|c| {
                        self.deferred_columns
                            .iter()
                            .find(|d| d.sort_col_idx == c.index())
                    });
                let mat_type = if let Some(deferred) = deferred {
                    deferred_sort_data_type(&self.ffhelper, deferred.canonical.ff_index)
                } else {
                    expr.expr
                        .data_type(&self.schema)
                        .unwrap_or(arrow_schema::DataType::Utf8View)
                };
                SortCol {
                    expr,
                    deferred,
                    mat_type,
                }
            })
            .collect();

        // A NULL must match the RowConverter's declared field type:
        // convert_columns rejects mismatches ("expected BinaryView got
        // Utf8View" for a NULL in a Bytes-backed NUMERIC sort key).
        // If the type is unsupported, propagate the error rather than
        // substituting a differently typed NULL that the converter will reject.
        let typed_null = |sort_col: &SortCol| -> Result<ScalarValue> {
            ScalarValue::try_from(&sort_col.mat_type)
        };

        // Batch-convert every candidate's compound key in a single convert_rows
        // call. We pass `Row<'_>` directly to avoid cloning `OwnedRow`.
        let ord_arrays: Vec<ArrayRef> = self
            .row_converter
            .convert_rows(candidates.iter().map(|(_, _, _, row_val)| row_val.row()))
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;

        // Build column-major ScalarValues and batch-convert all candidates at once.
        let mut column_values: Vec<Vec<ScalarValue>> = (0..self.sort_exprs.len())
            .map(|_| Vec::with_capacity(candidates.len()))
            .collect();

        for (cand_idx, (batch_idx, row_idx, seg_ord, _)) in candidates.iter().enumerate() {
            for (i, sort_col) in sort_cols.iter().enumerate() {
                let value = if let Some(deferred) = sort_col.deferred {
                    // Each deferred column carries its own ordinal in the compound
                    // key, so a NULL in one column does not blank out the others.
                    let term_ord = ord_arrays[i]
                        .as_any()
                        .downcast_ref::<UInt64Array>()
                        .filter(|a| a.is_valid(cand_idx))
                        .map(|a| a.value(cand_idx));
                    match term_ord {
                        Some(term_ord) => {
                            let seg_ord = seg_ord.ok_or_else(|| {
                                DataFusionError::Internal(
                                    "SegmentedTopKExec: a row with a term ordinal has no segment"
                                        .into(),
                                )
                            })?;
                            self.materialize_deferred_ordinal(seg_ord, term_ord, deferred)?
                        }
                        None => typed_null(sort_col)?,
                    }
                } else {
                    // Non-deferred column: evaluate directly from the batch.
                    let batch = &self.batches[*batch_idx];
                    let val = sort_col.expr.expr.evaluate(batch)?;
                    let arr = val.into_array(batch.num_rows())?;
                    ScalarValue::try_from_array(&arr, *row_idx).or_else(|_| typed_null(sort_col))?
                };
                column_values[i].push(value);
            }
        }

        // Batch convert all candidates in a single convert_columns call.
        let arrays: Vec<ArrayRef> = column_values
            .into_iter()
            .map(|col| ScalarValue::iter_to_array(col))
            .collect::<Result<Vec<_>>>()?;
        let converted = self
            .mat_row_converter
            .convert_columns(&arrays)
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
        let mut mat_rows: Vec<(usize, OwnedRow)> = Vec::with_capacity(candidates.len());
        for idx in 0..candidates.len() {
            mat_rows.push((idx, converted.row(idx).owned()));
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
                let (batch_idx, row_idx, _, _) = &candidates[*candidate_idx];
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

        let result = RecordBatch::try_new(self.schema.clone(), output_columns)
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;

        Ok(Some(result))
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;

    // Disambiguate the `Array` trait (glob-imported via `super::*` from multiple arrow
    // re-exports) so `array.is_valid`/`len`/`value` resolve unambiguously.
    use arrow_array::Array;
    use std::collections::BTreeSet;

    use crate::index::fast_fields_helper::WhichFastField;
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::reader::index::SearchIndexReader;
    use crate::postgres::rel::PgSearchRelation;
    use crate::query::SearchQueryInput;
    use crate::scan::deferred_encode::{build_state_doc_address, deferred_field};
    use crate::scan::segmented_topk_exec::DeferredSortColumn;
    use crate::schema::SearchFieldType;

    use arrow_schema::{Field, Schema};
    use datafusion::execution::TaskContext;
    use datafusion::physical_expr::expressions::Column;
    use datafusion::physical_expr::{LexOrdering, PhysicalSortExpr};
    use datafusion::physical_plan::ExecutionPlan;
    use datafusion::physical_plan::test::TestMemoryExec;
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
            USING paradedb (id, name, (sort_col::pdb.unicode_words('columnar=true'))) WITH (target_segment_count = 4);
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
            deferred_field("sort_col"),
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

                let name_array = build_state_doc_address(seg_ord as u32, &doc_ids);
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
                    },
                    rebuild: None,
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
