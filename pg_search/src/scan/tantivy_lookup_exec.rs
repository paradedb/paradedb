use std::any::Any;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::index::fast_fields_helper::{
    ords_to_bytes_array, ords_to_string_array, FFHelper, FFType,
};
use crate::scan::deferred_encode::unpack_doc_address;
use crate::scan::execution_plan::UnsafeSendStream;
use arrow_array::{new_null_array, Array, ArrayRef, RecordBatch, UInt64Array, UnionArray};
use arrow_array::{StructArray, UInt32Array};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use arrow_select::interleave::interleave;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::{RecordBatchStream, SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::metrics::{BaselineMetrics, ExecutionPlanMetricsSet, MetricsSet};
use datafusion::physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties};
use futures::Stream;
use tantivy::termdict::TermOrdinal;
use tantivy::{DocId, SegmentOrdinal};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, serde::Serialize, serde::Deserialize)]
pub enum DeferredKind {
    /// Text column deferred for late materialization.
    /// `ff_index` is the index into the FFHelper columns array.
    Text { ff_index: usize },
    /// Bytes column deferred for late materialization.
    /// `ff_index` is the index into the FFHelper columns array.
    Bytes { ff_index: usize },
    /// Ctid column deferred for late visibility checking.
    /// Uses `ffhelper.ctid()` instead of `ffhelper.column()`.
    Ctid,
}

impl DeferredKind {
    /// Returns `(ff_index, is_bytes)` for Text/Bytes variants.
    ///
    /// # Panics
    /// Panics for `Ctid`, which is resolved by `VisibilityFilterExec` and should
    /// never appear in `TantivyLookupExec` deferred fields.
    pub fn ff_index_and_is_bytes(&self) -> (usize, bool) {
        match self {
            DeferredKind::Text { ff_index } => (*ff_index, false),
            DeferredKind::Bytes { ff_index } => (*ff_index, true),
            DeferredKind::Ctid => {
                unreachable!("TantivyLookupExec should never receive DeferredKind::Ctid fields")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct DeferredField {
    pub field_name: String,
    pub kind: DeferredKind,
}

impl DeferredField {
    pub fn output_data_type(&self) -> DataType {
        match self.kind {
            DeferredKind::Text { .. } => DataType::Utf8View,
            DeferredKind::Bytes { .. } => DataType::BinaryView,
            DeferredKind::Ctid => DataType::UInt64,
        }
    }
}

pub struct TantivyLookupExec {
    input: Arc<dyn ExecutionPlan>,
    deferred_fields: Vec<DeferredField>,
    decoders: Vec<DecoderInfo>,
    ffhelper: Arc<FFHelper>,
    properties: PlanProperties,
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
        deferred_fields: Vec<DeferredField>,
        ffhelper: Arc<FFHelper>,
    ) -> Result<Self> {
        let (output_schema, decoders) =
            build_schema_and_decoders(input.schema(), &deferred_fields)?;
        let eq_props = EquivalenceProperties::new(output_schema);
        let properties = PlanProperties::new(
            eq_props,
            input.properties().output_partitioning().clone(),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );
        Ok(Self {
            input,
            deferred_fields,
            decoders,
            ffhelper,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
        })
    }

    pub fn deferred_fields(&self) -> &[DeferredField] {
        &self.deferred_fields
    }

    pub fn ffhelper(&self) -> &Arc<FFHelper> {
        &self.ffhelper
    }
}

#[derive(Clone, Debug)]
pub struct DecoderInfo {
    pub col_idx: usize,
    pub is_bytes: bool,
    pub ff_index: usize,
}

fn build_schema_and_decoders(
    input_schema: SchemaRef,
    deferred: &[DeferredField],
) -> Result<(SchemaRef, Vec<DecoderInfo>)> {
    let mut fields: Vec<Field> = Vec::with_capacity(input_schema.fields().len());
    let mut decoders = Vec::new();
    let mut deferred_pool = deferred.to_vec();

    // Iterate through the input schema exactly once.
    // This pairs the first "description" in the schema with the first "description"
    // in the deferred pool, removing it so the second one pairs correctly.
    for (col_idx, field) in input_schema.fields().iter().enumerate() {
        let name = field.name();
        let is_union = matches!(field.data_type(), DataType::Union(_, _));

        if is_union {
            if let Some(pos) = deferred_pool.iter().position(|d| &d.field_name == name) {
                let d = deferred_pool.remove(pos);
                let (ff_index, is_bytes) = d.kind.ff_index_and_is_bytes();
                fields.push(Field::new(&d.field_name, d.output_data_type(), true));
                decoders.push(DecoderInfo {
                    col_idx,
                    ff_index,
                    is_bytes,
                });
                continue;
            }
        }

        // Pass through fields that are not unions or not in our deferred pool
        fields.push(field.as_ref().clone());
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
                .map(|d| d.field_name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl ExecutionPlan for TantivyLookupExec {
    fn name(&self) -> &str {
        "TantivyLookupExec"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn metrics(&self) -> Option<MetricsSet> {
        Some(self.metrics.clone_inner())
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
        Ok(Arc::new(TantivyLookupExec::new(
            children.remove(0),
            self.deferred_fields.clone(),
            Arc::clone(&self.ffhelper),
        )?))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let input_stream = self.input.execute(partition, context)?;
        let baseline_metrics = BaselineMetrics::new(&self.metrics, partition);
        let stream = unsafe {
            UnsafeSendStream::new(LookupStream {
                input: input_stream,
                decoders: self.decoders.clone(),
                ffhelper: Arc::clone(&self.ffhelper),
                schema: self.properties.eq_properties.schema().clone(),
                baseline_metrics,
            })
        };
        Ok(Box::pin(stream))
    }
}

struct LookupStream {
    input: SendableRecordBatchStream,
    decoders: Vec<DecoderInfo>,
    ffhelper: Arc<FFHelper>,
    schema: SchemaRef,
    baseline_metrics: BaselineMetrics,
}

impl LookupStream {
    fn enrich_batch(&self, batch: RecordBatch) -> Result<RecordBatch> {
        let num_rows = batch.num_rows();
        // Clone the input arrays. We will overwrite the deferred ones by exact index.
        let mut output_columns: Vec<ArrayRef> = batch.columns().to_vec();

        for decoder in &self.decoders {
            let union_array = output_columns[decoder.col_idx]
                .as_any()
                .downcast_ref::<arrow_array::UnionArray>()
                .ok_or_else(|| {
                    DataFusionError::Execution(format!(
                        "expected UnionArray for deferred column at index {}",
                        decoder.col_idx
                    ))
                })?;

            // Replace the raw UnionArray with the decoded String/Binary array
            output_columns[decoder.col_idx] = materialize_deferred_column(
                &self.ffhelper,
                union_array,
                decoder.ff_index,
                decoder.is_bytes,
                num_rows,
            )?;
        }

        RecordBatch::try_new(self.schema.clone(), output_columns)
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
    }
}

/// Materializes deferred union values into their original text or bytes representation.
///
/// This function converts a 3-way `UnionArray` (containing either a packed `DocAddress`,
/// `TermOrdinal`s, or already-materialized strings) into an Arrow `ArrayRef` matching the
/// requested String or Binary view array type. To maximize efficiency, it groups requests
/// by segment, sorts them for sequential dictionary access, fetches materialized columns
/// per segment, and then uses Arrow's `interleave` to reconstruct the data in the
/// original input row order.
fn materialize_deferred_column(
    ffhelper: &FFHelper,
    union_array: &UnionArray,
    ff_index: usize,
    is_bytes: bool,
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
    let mut state_2_rows: Vec<usize> = Vec::new();
    for row in 0..num_rows {
        match type_ids[row] {
            0 => state_0_rows.push(row),
            1 => state_1_rows.push(row),
            2 => state_2_rows.push(row),
            _ => unreachable!("Invalid Union State"),
        }
    }

    // 1. Group requests by segment ordinal to process one segment at a time.
    let mut state_0_by_seg: crate::api::HashMap<SegmentOrdinal, Vec<(usize, DocId)>> =
        crate::api::HashMap::default();
    if !state_0_rows.is_empty() {
        let doc_address_child = union_array
            .child(0)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .ok_or_else(|| {
                DataFusionError::Execution(
                    "expected UInt64Array for doc_address child in deferred union".into(),
                )
            })?;
        for &row in &state_0_rows {
            let packed = doc_address_child.value(offsets[row] as usize);
            let (seg_ord, doc_id) = unpack_doc_address(packed);
            state_0_by_seg
                .entry(seg_ord)
                .or_default()
                .push((row, doc_id));
        }
    }

    let mut state_1_by_seg: crate::api::HashMap<SegmentOrdinal, Vec<(usize, Option<TermOrdinal>)>> =
        crate::api::HashMap::default();
    if !state_1_rows.is_empty() {
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

        for &row in &state_1_rows {
            let ci = offsets[row] as usize;
            let seg_ord = seg_ord_array.value(ci);
            let term_ord = if ord_array.is_null(ci) {
                None
            } else {
                Some(ord_array.value(ci))
            };
            state_1_by_seg
                .entry(seg_ord)
                .or_default()
                .push((row, term_ord));
        }
    }

    let mut segment_arrays: Vec<ArrayRef> = Vec::new();
    let mut indices: Vec<(usize, usize)> = vec![(0, 0); num_rows];

    // Map pre-materialized rows (State 2) directly from the compact child.
    if !state_2_rows.is_empty() {
        let materialized_child = union_array.child(2);
        segment_arrays.push(materialized_child.clone());
        let array_idx = 0;
        for &row_idx in &state_2_rows {
            indices[row_idx] = (array_idx, offsets[row_idx] as usize);
        }
    }

    // Helper closure to handle Step 3 and Step 4 cleanly for both State 0 and State 1
    let mut process_ordinals =
        |seg_ord: SegmentOrdinal, rows: Vec<(usize, Option<TermOrdinal>)>| -> Result<()> {
            let ords: Vec<Option<TermOrdinal>> = rows.iter().map(|(_, ord)| *ord).collect();
            let ords_array = UInt64Array::from(ords);

            // 3. Perform a bulk dictionary lookup for the entire segment.
            let array = if is_bytes {
                if let FFType::Bytes(bytes_col) = ffhelper.column(seg_ord, ff_index) {
                    ords_to_bytes_array(bytes_col.clone(), &ords_array)
                } else {
                    return Err(DataFusionError::Execution(format!(
                        "Expected Bytes column for index {}",
                        ff_index
                    )));
                }
            } else if let FFType::Text(str_col) = ffhelper.column(seg_ord, ff_index) {
                ords_to_string_array(str_col.clone(), &ords_array)
            } else {
                return Err(DataFusionError::Execution(format!(
                    "Expected Text column for index {}",
                    ff_index
                )));
            }?;

            segment_arrays.push(array);
            let array_idx = segment_arrays.len() - 1;

            // 4. Map the sorted, segment-local results back to their original row indices
            // in the global `RecordBatch` for interleaving.
            for (idx_within_segment, (original_row_idx, _)) in rows.into_iter().enumerate() {
                indices[original_row_idx] = (array_idx, idx_within_segment);
            }
            Ok(())
        };

    // Sort seg_ords to ensure deterministic behavior across executions.
    let mut seg_ords_0: Vec<SegmentOrdinal> = state_0_by_seg.keys().copied().collect();
    seg_ords_0.sort_unstable();

    for seg_ord in seg_ords_0 {
        let rows = state_0_by_seg.remove(&seg_ord).ok_or_else(|| {
            DataFusionError::Execution(format!("Segment {} missing from state 0 map", seg_ord))
        })?;

        let ids: Vec<DocId> = rows.iter().map(|(_, doc_id)| *doc_id).collect();
        let mut term_ords: Vec<Option<TermOrdinal>> = vec![None; ids.len()];

        if is_bytes {
            if let FFType::Bytes(bytes_col) = ffhelper.column(seg_ord, ff_index) {
                bytes_col.ords().first_vals(&ids, &mut term_ords);
            }
        } else if let FFType::Text(str_col) = ffhelper.column(seg_ord, ff_index) {
            str_col.ords().first_vals(&ids, &mut term_ords);
        }

        let rows_with_ords: Vec<_> = rows
            .into_iter()
            .zip(term_ords)
            .map(|((row_idx, _), ord)| (row_idx, ord))
            .collect();

        process_ordinals(seg_ord, rows_with_ords)?;
    }

    let mut seg_ords_1: Vec<SegmentOrdinal> = state_1_by_seg.keys().copied().collect();
    seg_ords_1.sort_unstable();

    for seg_ord in seg_ords_1 {
        let rows = state_1_by_seg.remove(&seg_ord).ok_or_else(|| {
            DataFusionError::Execution(format!("Segment {} missing from state 1 map", seg_ord))
        })?;

        process_ordinals(seg_ord, rows)?;
    }

    if segment_arrays.is_empty() {
        // All rows were somehow unhandled — return a null array of the right type.
        return Ok(new_null_array(
            &if is_bytes {
                DataType::BinaryView
            } else {
                DataType::Utf8View
            },
            num_rows,
        ));
    }

    // 5. Use Arrow's interleave to perform zero-copy (for views) reassembly of the
    // segment arrays into the final array matching the original row order.
    let segment_arrays_refs: Vec<&dyn arrow_array::Array> =
        segment_arrays.iter().map(|a| a.as_ref()).collect();
    interleave(&segment_arrays_refs, &indices)
        .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
}

/// Build a `UInt64Array` (as `ArrayRef`) from a slice of optional u64 values.
pub(crate) fn uint64_array_from_options(vals: &[Option<u64>]) -> ArrayRef {
    let mut builder = arrow_array::builder::UInt64Builder::with_capacity(vals.len());
    for val in vals {
        match val {
            Some(v) => builder.append_value(*v),
            None => builder.append_null(),
        }
    }
    Arc::new(builder.finish())
}

/// Materializes deferred ctid columns: unpacks DocAddresses and resolves to real ctids.
///
/// Takes a UInt64Array of packed DocAddresses (segment_ord << 32 | doc_id) and uses
/// the FFHelper to look up the real ctid for each document. Null entries pass through
/// as nulls in the output.
pub(crate) fn materialize_deferred_ctid(
    ffhelper: &FFHelper,
    doc_addr_array: &UInt64Array,
    num_rows: usize,
) -> Result<ArrayRef> {
    // Group by segment, tracking null rows separately.
    let mut by_seg: crate::api::HashMap<u32, Vec<(usize, tantivy::DocId)>> =
        crate::api::HashMap::default();
    let mut null_indices: Vec<usize> = Vec::new();

    for row in 0..num_rows {
        if doc_addr_array.is_null(row) {
            null_indices.push(row);
            continue;
        }
        let packed = doc_addr_array.value(row);
        let (seg_ord, doc_id) = unpack_doc_address(packed);
        by_seg.entry(seg_ord).or_default().push((row, doc_id));
    }

    let mut segment_arrays: Vec<ArrayRef> = Vec::new();
    // Default (0, 0) for null rows is safe: null rows are masked out after
    // interleave, so the value at segment_arrays[0][0] is never used in output.
    // segment_arrays[0] is guaranteed to exist when there are any non-null rows.
    let mut indices: Vec<(usize, usize)> = vec![(0, 0); num_rows];

    let mut seg_ords: Vec<u32> = by_seg.keys().copied().collect();
    seg_ords.sort_unstable();

    for seg_ord in seg_ords {
        // Safe: seg_ords was derived from by_seg.keys() above.
        let mut rows = by_seg.remove(&seg_ord).unwrap();
        // FFHelper::ctid() uses columnar fast field access which is optimized for
        // sequential reads. Sorting by doc_id ensures ascending access order,
        // avoiding expensive random seeks into the fast field column.
        rows.sort_unstable_by_key(|(_, doc_id)| *doc_id);

        let ids: Vec<tantivy::DocId> = rows.iter().map(|(_, doc_id)| *doc_id).collect();
        let mut ctid_results: Vec<Option<u64>> = vec![None; ids.len()];
        ffhelper.ctid(seg_ord).as_u64s(&ids, &mut ctid_results);

        let array = uint64_array_from_options(&ctid_results);

        segment_arrays.push(array);
        let array_idx = segment_arrays.len() - 1;
        for (idx_within_segment, (original_row_idx, _)) in rows.into_iter().enumerate() {
            indices[original_row_idx] = (array_idx, idx_within_segment);
        }
    }

    // If all rows are null, return a null UInt64 array.
    if segment_arrays.is_empty() {
        return Ok(arrow_array::new_null_array(&DataType::UInt64, num_rows));
    }

    let segment_arrays_refs: Vec<&dyn arrow_array::Array> =
        segment_arrays.iter().map(|a| a.as_ref()).collect();
    let mut result = interleave(&segment_arrays_refs, &indices)
        .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;

    // Apply null mask for rows that had null packed addresses.
    if !null_indices.is_empty() {
        let existing_nulls = result.nulls().cloned();
        let mut null_buf = arrow_buffer::BooleanBufferBuilder::new(num_rows);
        null_buf.append_n(num_rows, true);
        for &idx in &null_indices {
            null_buf.set_bit(idx, false);
        }
        let new_nulls = arrow_buffer::NullBuffer::from(null_buf.finish());
        let combined = match existing_nulls {
            Some(existing) => {
                let combined_buf = existing.inner() & new_nulls.inner();
                arrow_buffer::NullBuffer::from(combined_buf)
            }
            None => new_nulls,
        };
        result = arrow_array::make_array(
            result
                .to_data()
                .into_builder()
                .nulls(Some(combined))
                .build()
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?,
        );
    }

    Ok(result)
}

impl Stream for LookupStream {
    type Item = Result<RecordBatch>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let poll = Pin::new(&mut self.input).poll_next(cx);
        let final_poll = match poll {
            Poll::Ready(Some(Ok(batch))) => {
                let timer = self.baseline_metrics.elapsed_compute().timer();
                let result = self.enrich_batch(batch);
                timer.done();
                Poll::Ready(Some(result))
            }
            other => other,
        };
        self.baseline_metrics.record_poll(final_poll)
    }
}
impl RecordBatchStream for LookupStream {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::deferred_encode::deferred_union_data_type;
    use arrow_schema::{Field, Schema};

    #[test]
    fn uint64_array_from_options_all_some() {
        let arr = uint64_array_from_options(&[Some(1), Some(2), Some(3)]);
        assert_eq!(arr.len(), 3);
        let u64arr = arr.as_any().downcast_ref::<UInt64Array>().unwrap();
        assert_eq!(u64arr.null_count(), 0);
        assert_eq!(u64arr.value(0), 1);
        assert_eq!(u64arr.value(1), 2);
        assert_eq!(u64arr.value(2), 3);
    }

    #[test]
    fn uint64_array_from_options_with_nulls() {
        let arr = uint64_array_from_options(&[Some(1), None, Some(3)]);
        assert_eq!(arr.len(), 3);
        let u64arr = arr.as_any().downcast_ref::<UInt64Array>().unwrap();
        assert!(!u64arr.is_null(0));
        assert!(u64arr.is_null(1));
        assert!(!u64arr.is_null(2));
        assert_eq!(u64arr.value(0), 1);
        assert_eq!(u64arr.value(2), 3);
    }

    #[test]
    fn uint64_array_from_options_empty() {
        let arr = uint64_array_from_options(&[]);
        assert_eq!(arr.len(), 0);
    }

    #[test]
    fn deferred_kind_ff_index_text() {
        assert_eq!(
            DeferredKind::Text { ff_index: 5 }.ff_index_and_is_bytes(),
            (5, false)
        );
    }

    #[test]
    fn deferred_kind_ff_index_bytes() {
        assert_eq!(
            DeferredKind::Bytes { ff_index: 3 }.ff_index_and_is_bytes(),
            (3, true)
        );
    }

    #[test]
    #[should_panic]
    fn deferred_kind_ctid_panics() {
        DeferredKind::Ctid.ff_index_and_is_bytes();
    }

    #[test]
    fn build_schema_and_decoders_replaces_union() {
        let input_schema = Arc::new(Schema::new(vec![
            Field::new("ctid", DataType::UInt64, false),
            Field::new("description", deferred_union_data_type(false), true),
            Field::new("score", DataType::Float64, false),
        ]));
        let deferred = vec![DeferredField {
            field_name: "description".to_string(),
            kind: DeferredKind::Text { ff_index: 0 },
        }];
        let (schema, decoders) = build_schema_and_decoders(input_schema, &deferred).unwrap();
        assert_eq!(schema.field(1).name(), "description");
        assert_eq!(*schema.field(1).data_type(), DataType::Utf8View);
        assert_eq!(*schema.field(0).data_type(), DataType::UInt64);
        assert_eq!(*schema.field(2).data_type(), DataType::Float64);
        assert_eq!(decoders.len(), 1);
        assert_eq!(decoders[0].col_idx, 1);
        assert_eq!(decoders[0].ff_index, 0);
        assert!(!decoders[0].is_bytes);
    }

    #[test]
    fn build_schema_and_decoders_duplicate_names() {
        let input_schema = Arc::new(Schema::new(vec![
            Field::new("val", deferred_union_data_type(false), true),
            Field::new("val", deferred_union_data_type(false), true),
        ]));
        let deferred = vec![
            DeferredField {
                field_name: "val".to_string(),
                kind: DeferredKind::Text { ff_index: 0 },
            },
            DeferredField {
                field_name: "val".to_string(),
                kind: DeferredKind::Text { ff_index: 1 },
            },
        ];
        let (_, decoders) = build_schema_and_decoders(input_schema, &deferred).unwrap();
        assert_eq!(decoders.len(), 2);
        assert_eq!(decoders[0].col_idx, 0);
        assert_eq!(decoders[0].ff_index, 0);
        assert_eq!(decoders[1].col_idx, 1);
        assert_eq!(decoders[1].ff_index, 1);
    }

    #[test]
    fn build_schema_and_decoders_no_deferred() {
        let input_schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, true),
        ]));
        let (schema, decoders) = build_schema_and_decoders(input_schema.clone(), &[]).unwrap();
        assert_eq!(*schema, *input_schema);
        assert!(decoders.is_empty());
    }

    #[test]
    fn deferred_field_output_data_type() {
        assert_eq!(
            DeferredField {
                field_name: "a".to_string(),
                kind: DeferredKind::Text { ff_index: 0 }
            }
            .output_data_type(),
            DataType::Utf8View
        );
        assert_eq!(
            DeferredField {
                field_name: "b".to_string(),
                kind: DeferredKind::Bytes { ff_index: 0 }
            }
            .output_data_type(),
            DataType::BinaryView
        );
        assert_eq!(
            DeferredField {
                field_name: "c".to_string(),
                kind: DeferredKind::Ctid
            }
            .output_data_type(),
            DataType::UInt64
        );
    }
}
