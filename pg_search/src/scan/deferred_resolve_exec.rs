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

use std::sync::Arc;

use arrow_array::{Array, ArrayRef, RecordBatch, UnionArray};
use arrow_schema::{Field, Schema, SchemaRef};
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties};
use futures::StreamExt;

use crate::api::{HashMap, HashSet};
use crate::index::fast_fields_helper::FFHelper;
use crate::postgres::ParallelScanState;
use crate::scan::deferred_encode::term_ordinal_type;
use crate::scan::tantivy_lookup_exec::PhysicalDeferredField;

/// DeferredResolveExec replaces `UnionArray` columns containing `(doc_address | term_ordinal)`
/// with a native `StructArray(segment_id, term_ord)`. It achieves this by intercepting
/// `State 0` (doc_address) rows and using the provided `FFHelper` to resolve them to term ordinals.
pub struct DeferredResolveExec {
    pub input: Arc<dyn ExecutionPlan>,
    pub ffhelper: Arc<FFHelper>,
    // Information about the columns that are deferred Unions
    pub deferred_fields: Vec<PhysicalDeferredField>,
    pub schema: SchemaRef,
    pub properties: Arc<PlanProperties>,
}

impl std::fmt::Debug for DeferredResolveExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeferredResolveExec")
            .field("deferred_fields", &self.deferred_fields)
            .finish_non_exhaustive()
    }
}

fn resolve_union_to_struct(
    union_col: &UnionArray,
    ffhelper: &FFHelper,
    field: &PhysicalDeferredField,
) -> datafusion::common::Result<ArrayRef> {
    use crate::index::fast_fields_helper::FFType;
    use crate::index::fast_fields_helper::NULL_TERM_ORDINAL;
    use crate::scan::deferred_encode::term_ordinal_type;
    use crate::scan::deferred_encode::unpack_doc_address;
    use arrow_array::{StructArray, UInt32Array, UInt64Array};
    use datafusion::error::DataFusionError;
    use std::collections::HashMap;
    use tantivy::{DocId, SegmentOrdinal};

    let num_rows = union_col.len();
    let type_ids = union_col.type_ids();
    let offsets = union_col.offsets().ok_or_else(|| {
        DataFusionError::Internal("DeferredResolveExec: expected dense union with offsets".into())
    })?;

    let mut state0_rows: Vec<usize> = Vec::new();
    let mut state1_rows: Vec<usize> = Vec::new();
    for row_idx in 0..num_rows {
        match type_ids[row_idx] {
            0 => state0_rows.push(row_idx),
            1 => state1_rows.push(row_idx),
            _ => unreachable!("Invalid Union state"),
        }
    }

    let mut final_seg_ords = vec![0u32; num_rows];
    let mut final_term_ords = vec![NULL_TERM_ORDINAL; num_rows];

    // State 0
    let mut state0_by_seg: HashMap<SegmentOrdinal, Vec<(usize, DocId)>> = HashMap::default();
    if !state0_rows.is_empty() {
        let doc_addr_child = union_col
            .child(0)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        for &row_idx in &state0_rows {
            let packed = doc_addr_child.value(offsets[row_idx] as usize);
            let (seg_ord, doc_id) = unpack_doc_address(packed);
            state0_by_seg
                .entry(seg_ord)
                .or_default()
                .push((row_idx, doc_id));
            final_seg_ords[row_idx] = seg_ord;
        }
    }

    // State 1
    if !state1_rows.is_empty() {
        let term_ord_child = union_col
            .child(1)
            .as_any()
            .downcast_ref::<StructArray>()
            .unwrap();
        let seg_ord_array = term_ord_child
            .column(0)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        let ord_array = term_ord_child
            .column(1)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();

        for &row_idx in &state1_rows {
            let ci = offsets[row_idx] as usize;
            final_seg_ords[row_idx] = seg_ord_array.value(ci);
            if !ord_array.is_null(ci) {
                final_term_ords[row_idx] = ord_array.value(ci);
            }
        }
    }

    // Resolve State 0 term ordinals
    for (seg_ord, rows) in state0_by_seg {
        let doc_ids: Vec<DocId> = rows.iter().map(|(_, doc_id)| *doc_id).collect();
        let mut term_ords: Vec<Option<tantivy::termdict::TermOrdinal>> = vec![None; doc_ids.len()];

        let col = ffhelper.column(seg_ord, field.canonical.ff_index);
        match col {
            FFType::Text(str_col) => {
                str_col.ords().first_vals(&doc_ids, &mut term_ords);
            }
            FFType::Bytes(bytes_col) => {
                bytes_col.ords().first_vals(&doc_ids, &mut term_ords);
            }
            _ => {
                return Err(DataFusionError::Internal(
                    "Expected text or bytes field for deferred column".into(),
                ))
            }
        }

        for (i, &(row_idx, _)) in rows.iter().enumerate() {
            if let Some(ord) = term_ords[i] {
                final_term_ords[row_idx] = ord;
            }
        }
    }

    let seg_ord_array = Arc::new(UInt32Array::from(final_seg_ords)) as ArrayRef;
    let term_ord_array = Arc::new(UInt64Array::from(final_term_ords)) as ArrayRef;
    let struct_array = StructArray::try_new(
        match term_ordinal_type() {
            arrow_schema::DataType::Struct(fields) => fields,
            _ => unreachable!(),
        },
        vec![seg_ord_array, term_ord_array],
        None,
    )?;

    Ok(Arc::new(struct_array))
}

impl DeferredResolveExec {
    pub fn try_new(
        input: Arc<dyn ExecutionPlan>,
        ffhelper: Arc<FFHelper>,
        deferred_fields: Vec<PhysicalDeferredField>,
    ) -> datafusion::common::Result<Self> {
        let input_schema = input.schema();
        let mut fields = input_schema
            .fields()
            .iter()
            .map(|f| f.as_ref().clone())
            .collect::<Vec<_>>();

        for field in &deferred_fields {
            let idx = field.col_idx;
            fields[idx] = Field::new(&field.display_name, term_ordinal_type(), true);
        }
        let schema = Arc::new(Schema::new(fields));

        let properties = Arc::new(PlanProperties::new(
            EquivalenceProperties::new(Arc::clone(&schema)),
            input.properties().output_partitioning().clone(),
            EmissionType::Final,
            Boundedness::Bounded,
        ));

        Ok(Self {
            input,
            ffhelper,
            deferred_fields,
            schema,
            properties,
        })
    }
}

impl std::fmt::Display for DeferredResolveExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeferredResolveExec")
    }
}

impl DisplayAs for DeferredResolveExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(f, "DeferredResolveExec")
            }
            DisplayFormatType::TreeRender => todo!(),
        }
    }
}

impl ExecutionPlan for DeferredResolveExec {
    fn name(&self) -> &str {
        "DeferredResolveExec"
    }

    fn properties(&self) -> &Arc<PlanProperties> {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.input]
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> datafusion::common::Result<Arc<dyn ExecutionPlan>> {
        Self::try_new(
            children[0].clone(),
            Arc::clone(&self.ffhelper),
            self.deferred_fields.clone(),
        )
        .map(|e| Arc::new(e) as Arc<dyn ExecutionPlan>)
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> datafusion::common::Result<SendableRecordBatchStream> {
        let input_stream = self.input.execute(partition, context)?;
        Ok(Box::pin(DeferredResolveStream {
            input_stream,
            schema: Arc::clone(&self.schema),
            _ffhelper: Arc::clone(&self.ffhelper),
            deferred_fields: self.deferred_fields.clone(),
        }))
    }
}

impl DeferredResolveExec {
    pub(crate) fn encode_for_dispatch(&self) -> datafusion::common::Result<Vec<u8>> {
        serde_json::to_vec(&self.deferred_fields).map_err(|e| {
            datafusion::common::DataFusionError::Internal(format!(
                "DeferredResolveExec dispatch: serialize: {e}"
            ))
        })
    }

    pub(crate) fn decode_for_dispatch(
        buf: &[u8],
        input: Arc<dyn ExecutionPlan>,
        mut ffhelpers: HashMap<u32, Arc<FFHelper>>,
        non_partitioning_segment_ids: &[HashSet<tantivy::index::SegmentId>],
        parallel_state: Option<*mut ParallelScanState>,
    ) -> datafusion::common::Result<Arc<dyn ExecutionPlan>> {
        use crate::scan::tantivy_lookup_exec::{rebuild_missing_ffhelpers, LookupRebuildContext};
        let deferred_fields: Vec<PhysicalDeferredField> =
            serde_json::from_slice(buf).map_err(|e| {
                datafusion::common::DataFusionError::Internal(format!(
                    "DeferredResolveExec dispatch: deserialize: {e}"
                ))
            })?;
        rebuild_missing_ffhelpers(
            &deferred_fields,
            &mut ffhelpers,
            LookupRebuildContext {
                non_partitioning_segment_ids,
                parallel_state,
            },
        )?;

        let indexrelid = deferred_fields
            .first()
            .map(|f| f.canonical.indexrelid)
            .ok_or_else(|| {
                datafusion::common::DataFusionError::Internal(
                    "DeferredResolveExec requires at least one deferred field".into(),
                )
            })?;

        let ffhelper = ffhelpers
            .get(&indexrelid)
            .cloned()
            .unwrap_or_else(|| Arc::new(FFHelper::empty()));

        Ok(Arc::new(DeferredResolveExec::try_new(
            input,
            ffhelper,
            deferred_fields,
        )?))
    }
}

struct DeferredResolveStream {
    input_stream: SendableRecordBatchStream,
    schema: SchemaRef,
    _ffhelper: Arc<FFHelper>,
    deferred_fields: Vec<PhysicalDeferredField>,
}

impl datafusion::execution::RecordBatchStream for DeferredResolveStream {
    fn schema(&self) -> SchemaRef {
        Arc::clone(&self.schema)
    }
}

impl futures::Stream for DeferredResolveStream {
    type Item = datafusion::common::Result<RecordBatch>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let poll = self.input_stream.poll_next_unpin(cx);
        match poll {
            std::task::Poll::Ready(Some(Ok(batch))) => {
                let mut new_columns = batch.columns().to_vec();
                for field in &self.deferred_fields {
                    let col_idx = field.col_idx;
                    if let Some(_union_col) =
                        new_columns[col_idx].as_any().downcast_ref::<UnionArray>()
                    {
                        let resolved = resolve_union_to_struct(_union_col, &self._ffhelper, field)?;
                        new_columns[col_idx] = resolved;
                    }
                }
                std::task::Poll::Ready(Some(
                    RecordBatch::try_new(Arc::clone(&self.schema), new_columns).map_err(Into::into),
                ))
            }
            other => other,
        }
    }
}
