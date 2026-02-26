use std::any::Any;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::index::fast_fields_helper::{
    ords_to_bytes_array, ords_to_string_array, FFHelper, FFType,
};
use crate::scan::deferred_encode::unpack_doc_address;
use crate::scan::execution_plan::UnsafeSendStream;
use arrow_array::{Array, ArrayRef, RecordBatch, UInt64Array, UnionArray};
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
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DeferredField {
    pub field_name: String,
    pub is_bytes: bool,
    pub ff_index: usize,
}

impl DeferredField {
    pub fn output_data_type(&self) -> DataType {
        if self.is_bytes {
            DataType::BinaryView
        } else {
            DataType::Utf8View
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
                fields.push(Field::new(&d.field_name, d.output_data_type(), true));
                decoders.push(DecoderInfo {
                    col_idx,
                    is_bytes: d.is_bytes,
                    ff_index: d.ff_index,
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
    // Extract the type_ids buffer that tells us which state each row is in
    let type_ids = union_array.type_ids();

    // Extract the underlying arrays from the union
    let doc_address_child = union_array
        .child(0)
        .as_any()
        .downcast_ref::<UInt64Array>()
        .ok_or_else(|| {
            DataFusionError::Execution(
                "expected UInt64Array for doc_address child in deferred union".into(),
            )
        })?;

    let term_ord_child = union_array
        .child(1)
        .as_any()
        .downcast_ref::<StructArray>()
        .ok_or_else(|| {
            DataFusionError::Execution(
                "expected StructArray for term_ord child in deferred union".into(),
            )
        })?;

    let materialized_child = union_array.child(2);

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

    // 1. Group requests by segment ordinal to process one segment at a time.
    // We maintain separate groups for State 0 (needs doc_id lookup) and State 1 (already has ordinals).
    let mut state_0_by_seg: crate::api::HashMap<u32, Vec<(usize, tantivy::DocId)>> =
        crate::api::HashMap::default();
    let mut state_1_by_seg: crate::api::HashMap<u32, Vec<(usize, Option<u64>)>> =
        crate::api::HashMap::default();
    let mut pre_materialized_rows = Vec::new();

    for row in 0..num_rows {
        match type_ids[row] {
            0 => {
                let packed = doc_address_child.value(row);
                let (seg_ord, doc_id) = unpack_doc_address(packed);
                state_0_by_seg
                    .entry(seg_ord)
                    .or_default()
                    .push((row, doc_id));
            }
            1 => {
                let seg_ord = seg_ord_array.value(row);
                // manually adding null ordinal as arrow silently removes them
                let term_ord = if ord_array.is_null(row) {
                    None
                } else {
                    Some(ord_array.value(row))
                };
                state_1_by_seg
                    .entry(seg_ord)
                    .or_default()
                    .push((row, term_ord));
            }
            2 => {
                pre_materialized_rows.push(row);
            }
            _ => unreachable!("Invalid Union State"),
        }
    }

    let mut segment_arrays: Vec<ArrayRef> = Vec::new();
    let mut indices: Vec<(usize, usize)> = vec![(0, 0); num_rows];

    // Map pre-materialized rows (State 2) directly.
    if !pre_materialized_rows.is_empty() {
        segment_arrays.push(materialized_child.clone());
        let array_idx = 0;
        for row_idx in pre_materialized_rows {
            indices[row_idx] = (array_idx, row_idx);
        }
    }

    // Helper closure to handle Step 3 and Step 4 cleanly for both State 0 and State 1
    let mut process_ordinals = |seg_ord: u32, rows: Vec<(usize, Option<u64>)>| -> Result<()> {
        let ords: Vec<Option<u64>> = rows.iter().map(|(_, ord)| *ord).collect();
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
    let mut seg_ords_0: Vec<u32> = state_0_by_seg.keys().copied().collect();
    seg_ords_0.sort_unstable();

    for seg_ord in seg_ords_0 {
        let rows = state_0_by_seg.remove(&seg_ord).ok_or_else(|| {
            DataFusionError::Execution(format!("Segment {} missing from state 0 map", seg_ord))
        })?;

        let ids: Vec<tantivy::DocId> = rows.iter().map(|(_, doc_id)| *doc_id).collect();
        let mut term_ords: Vec<Option<u64>> = vec![None; ids.len()];

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

    let mut seg_ords_1: Vec<u32> = state_1_by_seg.keys().copied().collect();
    seg_ords_1.sort_unstable();

    for seg_ord in seg_ords_1 {
        let rows = state_1_by_seg.remove(&seg_ord).ok_or_else(|| {
            DataFusionError::Execution(format!("Segment {} missing from state 1 map", seg_ord))
        })?;

        process_ordinals(seg_ord, rows)?;
    }

    // 5. Use Arrow's interleave to perform zero-copy (for views) reassembly of the
    // segment arrays into the final array matching the original row order.
    let segment_arrays_refs: Vec<&dyn arrow_array::Array> =
        segment_arrays.iter().map(|a| a.as_ref()).collect();
    interleave(&segment_arrays_refs, &indices)
        .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
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
