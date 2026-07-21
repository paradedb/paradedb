use std::sync::Arc;

use crate::index::fast_fields_helper::WhichFastField;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;

use crate::index::fast_fields_helper::{
    for_each_segment, ords_to_bytes_array, ords_to_string_array, CanonicalColumn, FFHelper, FFType,
};
use crate::scan::execution_plan::UnsafeSendStream;

use arrow_array::{new_null_array, Array, ArrayRef, RecordBatch, UInt64Array, UnionArray};
use arrow_array::{StructArray, UInt32Array};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use arrow_select::interleave::interleave;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::{EquivalenceProperties, PhysicalExpr};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::filter_pushdown::{
    ChildFilterDescription, ChildPushdownResult, FilterDescription, FilterPushdownPhase,
    FilterPushdownPropagation,
};
use datafusion::physical_plan::metrics::{
    BaselineMetrics, ExecutionPlanMetricsSet, MetricsSet, RecordOutput,
};
use datafusion::physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties};
use tantivy::termdict::TermOrdinal;
use tantivy::{DocId, SegmentOrdinal};

/// Tracks a deferred column inside DataFusion's physical execution plan.
///
/// Unlike the logical `DeferredField` which uses the base column's string name, this struct
/// identifies the column strictly by its `usize` index within the physical `RecordBatch`.
/// This is necessary because DataFusion physical schemas (`arrow_schema::Schema`) drop
/// all relation qualifiers and names are no longer used for strict identity.
///
/// The `display_name` is preserved purely for `EXPLAIN` rendering and debugging; it should
/// never be used for matching columns in the physical plan.
#[derive(
    Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct PhysicalDeferredField {
    /// The positional index of the column in the physical Arrow schema
    pub col_idx: usize,
    /// A human-readable name used purely for `EXPLAIN` formatting
    pub display_name: String,
    pub is_bytes: bool,
    pub canonical: CanonicalColumn,
    #[serde(default)]
    pub rebuild: Option<crate::scan::late_materialization::DeferredLookupRebuild>,
}

impl PhysicalDeferredField {
    pub fn output_data_type(&self) -> DataType {
        if self.is_bytes {
            DataType::BinaryView
        } else {
            DataType::Utf8View
        }
    }
}

use crate::api::HashMap;

pub struct TantivyLookupExec {
    input: Arc<dyn ExecutionPlan>,
    deferred_fields: Vec<PhysicalDeferredField>,
    decoders: Vec<DecoderInfo>,
    ffhelpers: HashMap<u32, Arc<FFHelper>>,
    properties: Arc<PlanProperties>,
    metrics: ExecutionPlanMetricsSet,
}

impl std::fmt::Debug for TantivyLookupExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TantivyLookupExec")
            .field("decode", &self.deferred_fields.len())
            .finish()
    }
}
// TODO: Replace this loop with a direct bulk `ords_to_strings` / `ords_to_bytes`
// fetcher if the Tantivy fast field API exposes one in the future.
impl TantivyLookupExec {
    pub fn new(
        input: Arc<dyn ExecutionPlan>,
        deferred_fields: Vec<PhysicalDeferredField>,
        ffhelpers: HashMap<u32, Arc<FFHelper>>,
    ) -> Result<Self> {
        let (output_schema, decoders) =
            build_schema_and_decoders(input.schema(), &deferred_fields)?;
        let mut eq_props = EquivalenceProperties::new(output_schema.clone());
        // Propagate input ordering: TantivyLookupExec preserves row order
        // within batches (uses interleave), so if the input is sorted the
        // output retains that ordering.
        if let Some(input_ordering) = input.properties().output_ordering() {
            // Rewrite ordering expressions to reference the output schema
            // (column names are preserved, only data types change for deferred cols).
            use datafusion::physical_expr::expressions::Column;
            let rewritten: Vec<_> = input_ordering
                .iter()
                .filter_map(|sort_expr| {
                    if let Some(col) = sort_expr.expr.downcast_ref::<Column>() {
                        let new_idx = col.index();
                        if new_idx < output_schema.fields().len() {
                            let new_col =
                                Arc::new(Column::new(col.name(), new_idx)) as Arc<dyn PhysicalExpr>;
                            return Some(datafusion::physical_expr::PhysicalSortExpr {
                                expr: new_col,
                                options: sort_expr.options,
                            });
                        }
                    }
                    None
                })
                .collect();
            if rewritten.len() == input_ordering.len() {
                if let Some(lex) = datafusion::physical_expr::LexOrdering::new(rewritten) {
                    eq_props.add_ordering(lex);
                }
            }
        }
        let properties = Arc::new(PlanProperties::new(
            eq_props,
            input.properties().output_partitioning().clone(),
            EmissionType::Incremental,
            Boundedness::Bounded,
        ));
        Ok(Self {
            input,
            deferred_fields,
            decoders,
            ffhelpers,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
        })
    }

    pub fn deferred_fields(&self) -> &[PhysicalDeferredField] {
        &self.deferred_fields
    }

    pub fn ffhelper(&self, indexrelid: u32) -> Option<&Arc<FFHelper>> {
        self.ffhelpers.get(&indexrelid)
    }

    /// Serialize for leader dispatch. The `ffhelpers` are live and don't travel; the worker
    /// pulls them from the scans in its decoded subtree, keyed by index relid. `decoders` is
    /// derived from `deferred_fields`, so it's recomputed on decode.
    pub(crate) fn encode_for_dispatch(&self) -> datafusion::common::Result<Vec<u8>> {
        serde_json::to_vec(&self.deferred_fields).map_err(|e| {
            datafusion::common::DataFusionError::Internal(format!(
                "TantivyLookupExec dispatch: serialize: {e}"
            ))
        })
    }

    pub(crate) fn decode_for_dispatch(
        buf: &[u8],
        input: Arc<dyn ExecutionPlan>,
        mut ffhelpers: HashMap<u32, Arc<FFHelper>>,
        parallel_state: Option<*mut crate::postgres::ParallelScanState>,
    ) -> datafusion::common::Result<Arc<dyn ExecutionPlan>> {
        let deferred_fields: Vec<PhysicalDeferredField> =
            serde_json::from_slice(buf).map_err(|e| {
                datafusion::common::DataFusionError::Internal(format!(
                    "TantivyLookupExec dispatch: deserialize: {e}"
                ))
            })?;
        rebuild_missing_ffhelpers(
            &deferred_fields,
            &mut ffhelpers,
            LookupRebuildContext { parallel_state },
        )?;
        Ok(Arc::new(TantivyLookupExec::new(
            input,
            deferred_fields,
            ffhelpers,
        )?))
    }
}

/// Which snapshot a rebuilt fast-field reader reads. Both decode paths reach the same segments
/// in the same order the addresses were packed against, they just resolve the set differently.
#[derive(Clone, Copy)]
pub(crate) struct LookupRebuildContext {
    pub parallel_state: Option<*mut crate::postgres::ParallelScanState>,
}

/// Resolve the segment view a rebuilt helper opens for one deferred column's index.
pub(crate) fn rebuild_mvcc(
    context: LookupRebuildContext,
    rebuild: &crate::scan::late_materialization::DeferredLookupRebuild,
) -> Result<MvccSatisfies> {
    if let Some(source_idx) = rebuild.source_idx {
        let ps = context.parallel_state.ok_or_else(|| {
            DataFusionError::Internal(
                "ffhelper rebuild: parallel scan requires a ParallelScanState".into(),
            )
        })?;
        Ok(MvccSatisfies::ParallelWorker(unsafe {
            (*ps).segment_ids_for_source(source_idx)
        }))
    } else {
        Ok(MvccSatisfies::Snapshot)
    }
}

/// Open a fast-field helper for `indexrelid` with each rebuild entry laid out at its original
/// `ff_index` (`Junk` fills the gaps), over the segment view `mvcc` picks.
pub(crate) fn open_rebuilt_ffhelper(
    indexrelid: u32,
    entries: &[(
        usize,
        &crate::scan::late_materialization::DeferredLookupRebuild,
    )],
    mvcc: MvccSatisfies,
) -> Result<Arc<FFHelper>> {
    let index_rel = PgSearchRelation::open(pgrx::pg_sys::Oid::from(indexrelid));
    let reader = SearchIndexReader::open_with_context(
        &index_rel,
        SearchQueryInput::All,
        /* need_scores */ false,
        mvcc,
        None,
        None,
        /* needs_tokenizer_manager */ false,
    )
    .map_err(|e| DataFusionError::Internal(format!("ffhelper rebuild: open reader: {e}")))?;

    let width = entries.iter().map(|(i, _)| i + 1).max().unwrap_or(0);
    let mut which: Vec<WhichFastField> = vec![WhichFastField::Junk(String::new()); width];
    for (ff_index, rb) in entries {
        which[*ff_index] = WhichFastField::Named(rb.field_name.clone(), rb.field_type);
    }
    Ok(Arc::new(FFHelper::with_fields(&reader, &which)))
}

/// Rebuild the fast-field readers for deferred columns whose scan lives in a different plan
/// fragment (a lookup above a network shuffle finds no scan in its decoded subtree). The
/// `context` picks how the segment set is resolved; either way the reader's segment ordering
/// matches the ordering the addresses were packed against.
fn rebuild_missing_ffhelpers(
    deferred_fields: &[PhysicalDeferredField],
    ffhelpers: &mut HashMap<u32, Arc<FFHelper>>,
    context: LookupRebuildContext,
) -> Result<()> {
    // Ordinal-typed columns keep a scan's helper when one decoded in this fragment (its layout
    // lines up by construction); they rebuild only on a worker whose fragment has that scan
    // behind a network boundary.
    let mut rebuild_indexes: crate::api::HashSet<u32> = Default::default();
    for f in deferred_fields {
        if f.rebuild.is_none() {
            continue;
        }
        let scan_is_elsewhere = !ffhelpers.contains_key(&f.canonical.indexrelid);
        if scan_is_elsewhere {
            rebuild_indexes.insert(f.canonical.indexrelid);
        }
    }

    // Group by index so two columns of the same index share one reader, and lay out every
    // rebuildable column of a rebuilding index, not just the ones that triggered it: the
    // rebuilt helper replaces the map entry, so it has to serve all of them.
    let mut per_index: HashMap<u32, Vec<&PhysicalDeferredField>> = HashMap::default();
    for f in deferred_fields {
        if f.rebuild.is_some() && rebuild_indexes.contains(&f.canonical.indexrelid) {
            per_index.entry(f.canonical.indexrelid).or_default().push(f);
        }
    }

    for (indexrelid, fields) in per_index {
        let mvcc = rebuild_mvcc(context, fields[0].rebuild.as_ref().unwrap())?;
        let entries: Vec<(
            usize,
            &crate::scan::late_materialization::DeferredLookupRebuild,
        )> = fields
            .iter()
            .map(|f| (f.canonical.ff_index, f.rebuild.as_ref().unwrap()))
            .collect();
        ffhelpers.insert(
            indexrelid,
            open_rebuilt_ffhelper(indexrelid, &entries, mvcc)?,
        );
    }
    Ok(())
}
#[derive(Clone, Debug)]
pub struct DecoderInfo {
    pub col_idx: usize,
    pub is_bytes: bool,
    pub canonical: CanonicalColumn,
}

fn build_schema_and_decoders(
    input_schema: SchemaRef,
    deferred: &[PhysicalDeferredField],
) -> Result<(SchemaRef, Vec<DecoderInfo>)> {
    let mut fields: Vec<Field> = Vec::with_capacity(input_schema.fields().len());
    let mut decoders = Vec::new();
    let mut deferred_pool = deferred.to_vec();

    // Iterate through the input schema exactly once.
    // This pairs the first "description" in the schema with the first "description"
    // in the deferred pool, removing it so the second one pairs correctly.
    for (col_idx, field) in input_schema.fields().iter().enumerate() {
        let is_union = matches!(field.data_type(), DataType::Union(_, _));

        if is_union {
            if let Some(pos) = deferred_pool.iter().position(|d| d.col_idx == col_idx) {
                let d = deferred_pool.remove(pos);
                fields.push(Field::new(field.name(), d.output_data_type(), true));
                decoders.push(DecoderInfo {
                    col_idx,
                    is_bytes: d.is_bytes,
                    canonical: d.canonical,
                });
            } else {
                fields.push(field.as_ref().clone());
            }
        } else {
            // Pass through fields that are not unions or not in our deferred pool
            fields.push(field.as_ref().clone());
        }
    }

    Ok((Arc::new(Schema::new(fields)), decoders))
}

impl DisplayAs for TantivyLookupExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "TantivyLookupExec: decode=[{}]",
            self.deferred_fields
                .iter()
                .map(|d| d.display_name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl ExecutionPlan for TantivyLookupExec {
    fn name(&self) -> &str {
        "TantivyLookupExec"
    }

    fn metrics(&self) -> Option<MetricsSet> {
        Some(self.metrics.clone_inner())
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
        Ok(Arc::new(TantivyLookupExec::new(
            children.remove(0),
            self.deferred_fields.clone(),
            self.ffhelpers.clone(),
        )?))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let mut input_stream = self.input.execute(partition, context)?;
        let baseline_metrics = BaselineMetrics::new(&self.metrics, partition);
        let decoders = self.decoders.clone();
        let ffhelpers = self.ffhelpers.clone();
        let schema = self.properties.eq_properties.schema().clone();

        let stream_gen = async_stream::try_stream! {
            use futures::StreamExt;
            while let Some(batch_res) = input_stream.next().await {
                let timer = baseline_metrics.elapsed_compute().timer();
                let result = match batch_res {
                    Ok(batch) => enrich_batch(batch, &decoders, &ffhelpers, &schema),
                    Err(e) => Err(e),
                };
                timer.done();

                yield result.record_output(&baseline_metrics)?;
            }
            baseline_metrics.done();
        };

        let stream = unsafe {
            UnsafeSendStream::new(stream_gen, self.properties.eq_properties.schema().clone())
        };
        Ok(Box::pin(stream))
    }

    fn gather_filters_for_pushdown(
        &self,
        phase: FilterPushdownPhase,
        parent_filters: Vec<Arc<dyn PhysicalExpr>>,
        _config: &datafusion::common::config::ConfigOptions,
    ) -> Result<FilterDescription> {
        if !matches!(phase, FilterPushdownPhase::Post) {
            return Ok(FilterDescription::all_unsupported(
                &parent_filters,
                &self.children(),
            ));
        }
        let child_desc = ChildFilterDescription::from_child(&parent_filters, &self.input)?;
        Ok(FilterDescription::new().with_child(child_desc))
    }

    fn handle_child_pushdown_result(
        &self,
        _phase: FilterPushdownPhase,
        child_pushdown_result: ChildPushdownResult,
        _config: &datafusion::common::config::ConfigOptions,
    ) -> Result<FilterPushdownPropagation<Arc<dyn ExecutionPlan>>> {
        Ok(FilterPushdownPropagation::if_all(child_pushdown_result))
    }
}

enum DeferredColumnKind {
    Text { ff_index: usize },
    Bytes { ff_index: usize },
}

fn enrich_batch(
    batch: RecordBatch,
    decoders: &[DecoderInfo],
    ffhelpers: &HashMap<u32, Arc<FFHelper>>,
    schema: &SchemaRef,
) -> Result<RecordBatch> {
    let num_rows = batch.num_rows();
    // Clone the input arrays. We will overwrite the deferred ones by exact index.
    let mut output_columns: Vec<ArrayRef> = batch.columns().to_vec();

    for decoder in decoders {
        let union_array = output_columns[decoder.col_idx]
            .as_any()
            .downcast_ref::<arrow_array::UnionArray>()
            .ok_or_else(|| {
                DataFusionError::Execution(format!(
                    "expected UnionArray for deferred column at index {}",
                    decoder.col_idx
                ))
            })?;

        let ffhelper = ffhelpers
            .get(&decoder.canonical.indexrelid)
            .ok_or_else(|| {
                DataFusionError::Execution(format!(
                    "missing FFHelper for relation ID {}",
                    decoder.canonical.indexrelid
                ))
            })?;

        let ffcolumn = if decoder.is_bytes {
            DeferredColumnKind::Bytes {
                ff_index: decoder.canonical.ff_index,
            }
        } else {
            DeferredColumnKind::Text {
                ff_index: decoder.canonical.ff_index,
            }
        };

        // Replace the raw UnionArray with the decoded String/Binary array
        output_columns[decoder.col_idx] =
            materialize_deferred_column(ffhelper, &ffcolumn, union_array, num_rows)?;
    }

    RecordBatch::try_new(schema.clone(), output_columns)
        .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
}

type OrdsBySegment = Vec<Vec<(usize, Option<TermOrdinal>)>>;

/// Resolves State 0 (packed doc addresses) to term ordinals, grouped by segment.
///
/// Union states: 0 = packed (segment_ord, doc_id), 1 = pre-resolved (segment_ord, term_ord).
///
/// Returns the same shape as State 1: a vector indexed by segment ordinal,
/// of `(row_index, Option<TermOrdinal>)` pair vectors.
fn resolve_doc_addresses_to_term_ords(
    ffhelper: &FFHelper,
    ffcolumn: &DeferredColumnKind,
    union_array: &UnionArray,
    offsets: &[i32],
    state_0_rows: &[usize],
) -> Result<OrdsBySegment> {
    let num_segments = ffhelper.num_segments();
    let mut ords_by_seg: OrdsBySegment = vec![Vec::new(); num_segments];
    if state_0_rows.is_empty() {
        return Ok(ords_by_seg);
    }

    let doc_address_child = union_array
        .child(0)
        .as_any()
        .downcast_ref::<UInt64Array>()
        .ok_or_else(|| {
            DataFusionError::Execution(
                "expected UInt64Array for doc_address child in deferred union".into(),
            )
        })?;
    let packed_iter = state_0_rows
        .iter()
        .map(|&row| (row, doc_address_child.value(offsets[row] as usize)));

    for_each_segment(num_segments, packed_iter, |seg_ord, rows| {
        let ids: Vec<DocId> = rows.iter().map(|(_, doc_id)| *doc_id).collect();
        let mut term_ords: Vec<Option<TermOrdinal>> = vec![None; ids.len()];

        match ffcolumn {
            DeferredColumnKind::Bytes { ff_index } => {
                if let FFType::Bytes(bytes_col) = ffhelper.column(seg_ord, *ff_index) {
                    bytes_col.ords().first_vals(&ids, &mut term_ords);
                }
            }
            DeferredColumnKind::Text { ff_index } => {
                if let FFType::Text(str_col) = ffhelper.column(seg_ord, *ff_index) {
                    str_col.ords().first_vals(&ids, &mut term_ords);
                }
            }
        };

        let entry = &mut ords_by_seg[seg_ord as usize];
        for ((row_idx, _), ord) in rows.into_iter().zip(term_ords) {
            entry.push((row_idx, ord));
        }
        Ok(())
    })?;

    Ok(ords_by_seg)
}

/// Extracts State 1 — the union variant carrying pre-resolved (segment_ord, term_ord) pairs —
/// from the dense union's StructArray child, grouped by segment.
fn extract_term_ords(
    ffhelper: &FFHelper,
    union_array: &UnionArray,
    offsets: &[i32],
    state_1_rows: &[usize],
) -> Result<OrdsBySegment> {
    let num_segments = ffhelper.num_segments();
    let mut ords_by_seg: OrdsBySegment = vec![Vec::new(); num_segments];
    if state_1_rows.is_empty() {
        return Ok(ords_by_seg);
    }

    let term_ord_child = union_array
        .child(1)
        .as_any()
        .downcast_ref::<StructArray>()
        .ok_or_else(|| {
            DataFusionError::Execution(
                "expected StructArray for term_ord child in deferred union".into(),
            )
        })?;
    let seg_ord_array = term_ord_child
        .column(0)
        .as_any()
        .downcast_ref::<UInt32Array>()
        .ok_or_else(|| {
            DataFusionError::Execution("expected UInt32Array for seg_ord column".into())
        })?;
    let ord_array = term_ord_child
        .column(1)
        .as_any()
        .downcast_ref::<UInt64Array>()
        .ok_or_else(|| {
            DataFusionError::Execution("expected UInt64Array for term_ord column".into())
        })?;

    for &row in state_1_rows {
        let ci = offsets[row] as usize;
        let seg_ord = seg_ord_array.value(ci);
        let term_ord = if ord_array.is_null(ci) {
            None
        } else {
            Some(ord_array.value(ci))
        };
        ords_by_seg[seg_ord as usize].push((row, term_ord));
    }

    Ok(ords_by_seg)
}

/// Decodes term ordinals into string/bytes arrays and records their positions
/// in `segment_arrays` and `indices` for later interleaving.
fn decode_term_ordinals(
    ffhelper: &FFHelper,
    ffcolumn: &DeferredColumnKind,
    ords_by_seg: OrdsBySegment,
    segment_arrays: &mut Vec<ArrayRef>,
    indices: &mut [(usize, usize)],
) -> Result<()> {
    for (seg_ord, rows) in ords_by_seg.into_iter().enumerate() {
        if rows.is_empty() {
            continue;
        }
        let ords: Vec<Option<TermOrdinal>> = rows.iter().map(|(_, ord)| *ord).collect();
        let ords_array = UInt64Array::from(ords);

        // Perform a bulk dictionary lookup for the entire segment.
        let array: Result<ArrayRef> = match ffcolumn {
            DeferredColumnKind::Bytes { ff_index } => {
                if let FFType::Bytes(bytes_col) =
                    ffhelper.column(seg_ord as SegmentOrdinal, *ff_index)
                {
                    ords_to_bytes_array(bytes_col.clone(), &ords_array)
                } else {
                    Err(DataFusionError::Execution(format!(
                        "Expected Bytes column for index {}",
                        ff_index
                    )))
                }
            }
            DeferredColumnKind::Text { ff_index } => {
                if let FFType::Text(str_col) = ffhelper.column(seg_ord as SegmentOrdinal, *ff_index)
                {
                    ords_to_string_array(str_col.clone(), &ords_array)
                } else {
                    Err(DataFusionError::Execution(format!(
                        "Expected Text column for index {}",
                        ff_index
                    )))
                }
            }
        };

        segment_arrays.push(array?);
        let array_idx = segment_arrays.len() - 1;

        // Map the sorted, segment-local results back to their original row indices
        // in the global `RecordBatch` for interleaving.
        for (idx_within_segment, (original_row_idx, _)) in rows.into_iter().enumerate() {
            indices[original_row_idx] = (array_idx, idx_within_segment);
        }
    }
    Ok(())
}

/// Materializes deferred union values into their original text or bytes representation.
///
/// This function converts a 2-way `UnionArray` (containing either a packed `DocAddress`
/// or `TermOrdinal`s) into an Arrow `ArrayRef` matching the
/// requested String or Binary view array type. To maximize efficiency, it groups requests
/// by segment, sorts them for sequential dictionary access, fetches materialized columns
/// per segment, and then uses Arrow's `interleave` to reconstruct the data in the
/// original input row order.
fn materialize_deferred_column(
    ffhelper: &FFHelper,
    ffcolumn: &DeferredColumnKind,
    union_array: &UnionArray,
    num_rows: usize,
) -> Result<ArrayRef> {
    // Dense union: each child is compact (contains only its type's rows).
    // Partition original row indices by type, then iterate compact children.
    let type_ids = union_array.type_ids();
    let offsets = union_array.offsets().ok_or_else(|| {
        DataFusionError::Execution("expected dense union with offsets in deferred column".into())
    })?;

    let mut state_0_rows: Vec<usize> = Vec::new();
    let mut state_1_rows: Vec<usize> = Vec::new();
    for row in 0..num_rows {
        match type_ids[row] {
            0 => state_0_rows.push(row),
            1 => state_1_rows.push(row),
            _ => unreachable!("Invalid Union State"),
        }
    }

    let mut segment_arrays: Vec<ArrayRef> = Vec::new();
    let mut indices: Vec<(usize, usize)> = vec![(0, 0); num_rows];

    // Step 1. Resolve State 0 (packed doc addresses) and State 1 (pre-resolved term
    // ordinals) into a unified collection of term ordinals grouped by segment
    let mut resolved_term_ords = resolve_doc_addresses_to_term_ords(
        ffhelper,
        ffcolumn,
        union_array,
        offsets,
        &state_0_rows,
    )?;
    let preresolved_term_ords = extract_term_ords(ffhelper, union_array, offsets, &state_1_rows)?;
    // Merge State 0 (now resolved to term ords) and State 1 into a single collection.
    for (seg_ord, rows) in preresolved_term_ords.into_iter().enumerate() {
        resolved_term_ords[seg_ord].extend(rows);
    }

    // Step 2. Dictionary-decode the term ordinals all into string/bytes arrays.
    decode_term_ordinals(
        ffhelper,
        ffcolumn,
        resolved_term_ords,
        &mut segment_arrays,
        &mut indices,
    )?;

    if segment_arrays.is_empty() {
        // All rows were somehow unhandled — return a null array of the right type.
        return Ok(new_null_array(
            &if matches!(ffcolumn, DeferredColumnKind::Bytes { ff_index: _ }) {
                DataType::BinaryView
            } else {
                DataType::Utf8View
            },
            num_rows,
        ));
    }

    // Step 3. Use Arrow's interleave to perform zero-copy (for views) reassembly of the
    // segment arrays into the final array matching the original row order.
    let segment_arrays_refs: Vec<&dyn arrow_array::Array> =
        segment_arrays.iter().map(|a| a.as_ref()).collect();
    interleave(&segment_arrays_refs, &indices)
        .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
}
