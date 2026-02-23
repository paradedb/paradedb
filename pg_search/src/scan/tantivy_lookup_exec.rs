use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use arrow_array::builder::{BinaryViewBuilder, StringViewBuilder};
use arrow_array::{ArrayRef, RecordBatch, UInt64Array};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::{RecordBatchStream, SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties};
use futures::Stream;

use crate::index::fast_fields_helper::{FFHelper, FFType};
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
            "TantivyLookup: decode=[{}]",
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
        "TantivyLookup"
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
        let stream = unsafe {
            UnsafeSendStream::new(LookupStream {
                input: input_stream,
                deferred_col_indices,
                deferred_fields: self.deferred_fields.clone(),
                ffhelper: Arc::clone(&self.ffhelper),
                schema: self.properties.eq_properties.schema().clone(),
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
                output_columns.push(decode_doc_addresses(
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

fn decode_doc_addresses(
    ffhelper: &FFHelper,
    doc_addr_array: &UInt64Array,
    ff_index: usize,
    is_bytes: bool,
    num_rows: usize,
) -> Result<ArrayRef> {
    let mut by_seg: HashMap<u32, Vec<(usize, tantivy::DocId)>> = HashMap::new();
    for row in 0..num_rows {
        let packed = doc_addr_array.value(row);
        let seg_ord = (packed >> 32) as u32;
        let doc_id = (packed & 0xFFFF_FFFF) as u32;
        by_seg.entry(seg_ord).or_default().push((row, doc_id));
    }

    // Sort doc_ids within each segment for sequential access (first_vals efficiency)
    for rows in by_seg.values_mut() {
        rows.sort_unstable_by_key(|(_, doc_id)| *doc_id);
    }

    if is_bytes {
        let mut result: Vec<Option<Vec<u8>>> = vec![None; num_rows];
        for (seg_ord, rows) in &by_seg {
            if let FFType::Bytes(bytes_col) = ffhelper.column(*seg_ord, ff_index) {
                // Step 1: doc_id → term_ord
                let ids: Vec<tantivy::DocId> = rows.iter().map(|(_, doc_id)| *doc_id).collect();
                let mut term_ords: Vec<Option<u64>> = vec![None; ids.len()];
                bytes_col.ords().first_vals(&ids, &mut term_ords);

                // Step 2: term_ord → bytes. Sort by ordinal for sequential dictionary access.
                let mut ord_idx_pairs: Vec<(usize, u64)> = term_ords
                    .iter()
                    .enumerate()
                    .filter_map(|(i, maybe_ord)| maybe_ord.map(|ord| (i, ord)))
                    .collect();
                ord_idx_pairs.sort_unstable_by_key(|(_, ord)| *ord);

                let mut buffer = Vec::new();
                for (i, ord) in ord_idx_pairs {
                    let (row_idx, _) = rows[i];
                    buffer.clear();
                    if bytes_col.ord_to_bytes(ord, &mut buffer).is_ok() {
                        result[row_idx] = Some(buffer.clone());
                    }
                }
            }
        }
        let mut b = BinaryViewBuilder::with_capacity(num_rows);
        for v in result {
            match v {
                Some(x) => b.append_value(&x),
                None => b.append_null(),
            }
        }
        Ok(Arc::new(b.finish()))
    } else {
        let mut result: Vec<Option<String>> = vec![None; num_rows];
        for (seg_ord, rows) in &by_seg {
            if let FFType::Text(str_col) = ffhelper.column(*seg_ord, ff_index) {
                // Step 1: doc_id → term_ord via ords().first_vals()
                let ids: Vec<tantivy::DocId> = rows.iter().map(|(_, doc_id)| *doc_id).collect();
                let mut term_ords: Vec<Option<u64>> = vec![None; ids.len()];
                str_col.ords().first_vals(&ids, &mut term_ords);

                // Step 2: term_ord → string. Sort by ordinal for sequential dictionary access.
                let mut ord_idx_pairs: Vec<(usize, u64)> = term_ords
                    .iter()
                    .enumerate()
                    .filter_map(|(i, maybe_ord)| maybe_ord.map(|ord| (i, ord)))
                    .collect();
                ord_idx_pairs.sort_unstable_by_key(|(_, ord)| *ord);

                let mut s = String::new();
                for (i, ord) in ord_idx_pairs {
                    let (row_idx, _) = rows[i];
                    s.clear();
                    if str_col.ord_to_str(ord, &mut s).is_ok() {
                        result[row_idx] = Some(s.clone());
                    }
                }
            }
        }
        let mut b = StringViewBuilder::with_capacity(num_rows);
        for v in result {
            match v {
                Some(s) => b.append_value(&s),
                None => b.append_null(),
            }
        }
        Ok(Arc::new(b.finish()))
    }
}

impl Stream for LookupStream {
    type Item = Result<RecordBatch>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.input).poll_next(cx) {
            Poll::Ready(Some(Ok(batch))) => Poll::Ready(Some(self.enrich_batch(batch))),
            other => other,
        }
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
