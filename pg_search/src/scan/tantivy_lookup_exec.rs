use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use arrow_array::{ArrayRef, RecordBatch, UInt64Array};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::{RecordBatchStream, SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::metrics::{BaselineMetrics, ExecutionPlanMetricsSet, MetricsSet};
use datafusion::physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties};
use futures::Stream;

use crate::index::fast_fields_helper::{
    ords_to_bytes_array, ords_to_string_array, FFHelper, FFType, NULL_TERM_ORDINAL,
};
use arrow_select::interleave::interleave;
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
        let output_schema = build_output_schema(input.schema(), &deferred_fields)?;
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
            ffhelper,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
        })
    }
}

fn build_output_schema(input_schema: SchemaRef, deferred: &[DeferredField]) -> Result<SchemaRef> {
    let decode_map: HashMap<String, Field> = deferred
        .iter()
        .map(|d| {
            (
                d.field_name.clone(),
                Field::new(&d.field_name, d.output_data_type(), true),
            )
        })
        .collect();

    let fields: Vec<Field> = input_schema
        .fields()
        .iter()
        .map(|f| {
            let name = f.name().as_str();
            if let Some(decoded) = decode_map.get(name) {
                decoded.clone()
            } else {
                f.as_ref().clone()
            }
        })
        .collect();

    Ok(Arc::new(Schema::new(fields)))
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
        let input_schema = self.input.schema();
        let deferred_col_indices: Vec<usize> = self
            .deferred_fields
            .iter()
            .map(|d| {
                input_schema
                    .column_with_name(&d.field_name)
                    .map(|(i, _)| i)
                    .ok_or_else(|| {
                        DataFusionError::Execution(format!(
                            "TantivyLookupExec: missing deferred column '{}'",
                            d.field_name
                        ))
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        let baseline_metrics = BaselineMetrics::new(&self.metrics, partition);
        let stream = unsafe {
            UnsafeSendStream::new(LookupStream {
                input: input_stream,
                deferred_col_indices,
                deferred_fields: self.deferred_fields.clone(),
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
    deferred_col_indices: Vec<usize>,
    deferred_fields: Vec<DeferredField>,
    ffhelper: Arc<FFHelper>,
    schema: SchemaRef,
    baseline_metrics: BaselineMetrics,
}

impl LookupStream {
    fn enrich_batch(&self, batch: RecordBatch) -> Result<RecordBatch> {
        let num_rows = batch.num_rows();
        let mut output_columns: Vec<ArrayRef> = Vec::with_capacity(self.schema.fields().len());

        for output_field in self.schema.fields() {
            let name = output_field.name();
            if let Some(d_idx) = self
                .deferred_fields
                .iter()
                .position(|d| &d.field_name == name)
            {
                let field = &self.deferred_fields[d_idx];
                let doc_addr_array = batch
                    .column(self.deferred_col_indices[d_idx])
                    .as_any()
                    .downcast_ref::<UInt64Array>()
                    .ok_or_else(|| {
                        DataFusionError::Execution(format!(
                            "expected UInt64 DocAddress column for '{}'",
                            name
                        ))
                    })?;
                output_columns.push(materialize_deferred_column(
                    &self.ffhelper,
                    doc_addr_array,
                    field.ff_index,
                    field.is_bytes,
                    num_rows,
                )?);
            } else {
                let (col_idx, _) = batch.schema().column_with_name(name).ok_or_else(|| {
                    DataFusionError::Execution(format!("missing column '{}'", name))
                })?;
                output_columns.push(batch.column(col_idx).clone());
            }
        }
        RecordBatch::try_new(self.schema.clone(), output_columns)
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
    }
}

/// Materializes deferred `DocAddress` values into their original text or bytes representation.
///
/// This function converts packed `DocAddress` values (segment ordinal and document ID) into
/// an Arrow `ArrayRef` matching the requested String or Binary view array type. To maximize
/// efficiency, it groups requests by segment, sorts them for sequential dictionary access,
/// fetches materialized columns per segment, and then uses Arrow's `interleave` to reconstruct
/// the data in the original input row order.
fn materialize_deferred_column(
    ffhelper: &FFHelper,
    doc_addr_array: &UInt64Array,
    ff_index: usize,
    is_bytes: bool,
    num_rows: usize,
) -> Result<ArrayRef> {
    // 1. Group requests by segment ordinal to process one segment at a time.
    let mut by_seg: HashMap<u32, Vec<(usize, tantivy::DocId)>> = HashMap::new();
    for row in 0..num_rows {
        let packed = doc_addr_array.value(row);
        let seg_ord = (packed >> 32) as u32;
        let doc_id = (packed & 0xFFFF_FFFF) as u32;
        by_seg.entry(seg_ord).or_default().push((row, doc_id));
    }

    // 2. Sort doc_ids within each segment for sequential access (first_vals efficiency).
    for rows in by_seg.values_mut() {
        rows.sort_unstable_by_key(|(_, doc_id)| *doc_id);
    }

    let mut segment_arrays: Vec<ArrayRef> = Vec::with_capacity(by_seg.len());
    let mut indices: Vec<(usize, usize)> = vec![(0, 0); num_rows];

    // Sort seg_ords to ensure deterministic behavior across executions.
    let mut seg_ords: Vec<u32> = by_seg.keys().copied().collect();
    seg_ords.sort_unstable();

    for (array_idx, seg_ord) in seg_ords.into_iter().enumerate() {
        let rows = by_seg.remove(&seg_ord).unwrap();

        let ids: Vec<tantivy::DocId> = rows.iter().map(|(_, doc_id)| *doc_id).collect();
        let mut term_ords: Vec<Option<u64>> = vec![None; ids.len()];

        // 3. Perform a bulk dictionary lookup for the entire segment.
        let array = if is_bytes {
            if let FFType::Bytes(bytes_col) = ffhelper.column(seg_ord, ff_index) {
                bytes_col.ords().first_vals(&ids, &mut term_ords);
                ords_to_bytes_array(
                    bytes_col.clone(),
                    term_ords
                        .into_iter()
                        .map(|o| o.unwrap_or(NULL_TERM_ORDINAL)),
                )
            } else {
                return Err(DataFusionError::Execution(format!(
                    "Expected Bytes column for index {}",
                    ff_index
                )));
            }
        } else if let FFType::Text(str_col) = ffhelper.column(seg_ord, ff_index) {
            str_col.ords().first_vals(&ids, &mut term_ords);
            ords_to_string_array(
                str_col.clone(),
                term_ords
                    .into_iter()
                    .map(|o| o.unwrap_or(NULL_TERM_ORDINAL)),
            )
        } else {
            return Err(DataFusionError::Execution(format!(
                "Expected Text column for index {}",
                ff_index
            )));
        };
        segment_arrays.push(array);

        // 4. Map the sorted, segment-local results back to their original row indices
        // in the global `RecordBatch` for interleaving.
        for (idx_within_segment, (original_row_idx, _)) in rows.into_iter().enumerate() {
            indices[original_row_idx] = (array_idx, idx_within_segment);
        }
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

struct UnsafeSendStream<T>(T);
impl<T> UnsafeSendStream<T> {
    unsafe fn new(t: T) -> Self {
        Self(t)
    }
}
unsafe impl<T> Send for UnsafeSendStream<T> {}
impl<T: Stream> Stream for UnsafeSendStream<T> {
    type Item = T::Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().0).poll_next(cx) }
    }
}
impl<T: RecordBatchStream> RecordBatchStream for UnsafeSendStream<T> {
    fn schema(&self) -> SchemaRef {
        self.0.schema()
    }
}
