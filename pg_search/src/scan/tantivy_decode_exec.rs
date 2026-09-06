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

//! The dictionary-decode half of a deferred lookup.
//!
//! `TantivyDecodeExec` turns a deferred column's term ordinals (State 1) into the
//! `Utf8View` / `BinaryView` values the rest of the plan expects. It never reads a fast-field
//! column: a row that still carries a doc address is a planning error, because the fetch has
//! its own node (`TantivyFetchExec`) or ran inside the scan.
//!
//! Dictionary lookups are random access whatever the row order, and an ordinal is far
//! narrower than the string it names, so this node is the half of the lookup that can be
//! pushed above joins and shuffles without changing its cost per row.

use std::sync::Arc;

use crate::api::HashMap;
use crate::index::fast_fields_helper::{
    ords_to_bytes_array, ords_to_string_array, FFHelper, FFType,
};
use crate::scan::deferred_encode::{DeferredUnion, DeferredValue};
use crate::scan::deferred_lookup::{
    ffhelper_for, preserved_ordering, rebuild_missing_ffhelpers, LookupRebuildContext,
    PhysicalDeferredField,
};
use crate::scan::execution_plan::UnsafeSendStream;

use arrow_array::{new_null_array, ArrayRef, RecordBatch, UInt64Array};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use arrow_select::interleave::interleave;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::PhysicalExpr;
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
use tantivy::SegmentOrdinal;

pub struct TantivyDecodeExec {
    input: Arc<dyn ExecutionPlan>,
    deferred_fields: Vec<PhysicalDeferredField>,
    /// Keyed by index relid; see `TantivyFetchExec::ffhelpers` for the self-join aliasing.
    ffhelpers: HashMap<u32, Arc<FFHelper>>,
    properties: Arc<PlanProperties>,
    metrics: ExecutionPlanMetricsSet,
}

impl std::fmt::Debug for TantivyDecodeExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TantivyDecodeExec")
            .field("decode", &self.deferred_fields.len())
            .finish()
    }
}

impl TantivyDecodeExec {
    pub fn new(
        input: Arc<dyn ExecutionPlan>,
        deferred_fields: Vec<PhysicalDeferredField>,
        ffhelpers: HashMap<u32, Arc<FFHelper>>,
    ) -> Result<Self> {
        let output_schema = build_output_schema(input.schema(), &deferred_fields)?;
        let properties = Arc::new(PlanProperties::new(
            preserved_ordering(&input, output_schema),
            input.properties().output_partitioning().clone(),
            EmissionType::Incremental,
            Boundedness::Bounded,
        ));
        Ok(Self {
            input,
            deferred_fields,
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

    pub(crate) fn ffhelpers(&self) -> &HashMap<u32, Arc<FFHelper>> {
        &self.ffhelpers
    }

    /// Serialize for leader dispatch. The `ffhelpers` are live and don't travel; the worker
    /// pulls them from the scans in its decoded subtree, keyed by index relid.
    pub(crate) fn encode_for_dispatch(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&self.deferred_fields).map_err(|e| {
            DataFusionError::Internal(format!("TantivyDecodeExec dispatch: serialize: {e}"))
        })
    }

    pub(crate) fn decode_for_dispatch(
        buf: &[u8],
        input: Arc<dyn ExecutionPlan>,
        mut ffhelpers: HashMap<u32, Arc<FFHelper>>,
        parallel_state: Option<*mut crate::postgres::ParallelScanState>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let deferred_fields: Vec<PhysicalDeferredField> =
            serde_json::from_slice(buf).map_err(|e| {
                DataFusionError::Internal(format!("TantivyDecodeExec dispatch: deserialize: {e}"))
            })?;
        rebuild_missing_ffhelpers(
            &deferred_fields,
            &mut ffhelpers,
            LookupRebuildContext { parallel_state },
        )?;
        Ok(Arc::new(TantivyDecodeExec::new(
            input,
            deferred_fields,
            ffhelpers,
        )?))
    }
}

/// The input schema with each deferred union column replaced by its decoded type.
fn build_output_schema(
    input_schema: SchemaRef,
    deferred: &[PhysicalDeferredField],
) -> Result<SchemaRef> {
    let mut fields: Vec<Field> = input_schema
        .fields()
        .iter()
        .map(|f| f.as_ref().clone())
        .collect();
    for d in deferred {
        let field = fields.get_mut(d.col_idx).ok_or_else(|| {
            DataFusionError::Plan(format!(
                "TantivyDecodeExec: column {} ('{}') is past the input schema",
                d.col_idx, d.display_name
            ))
        })?;
        if !matches!(field.data_type(), DataType::Union(_, _)) {
            return Err(DataFusionError::Plan(format!(
                "TantivyDecodeExec: column {} ('{}') is {:?}, expected a deferred union",
                d.col_idx,
                d.display_name,
                field.data_type()
            )));
        }
        *field = Field::new(field.name(), d.output_data_type(), true);
    }
    Ok(Arc::new(Schema::new(fields)))
}

impl DisplayAs for TantivyDecodeExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "TantivyDecodeExec: decode=[{}]",
            self.deferred_fields
                .iter()
                .map(|d| d.display_name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl ExecutionPlan for TantivyDecodeExec {
    fn name(&self) -> &str {
        "TantivyDecodeExec"
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

    fn apply_expressions(
        &self,
        _f: &mut dyn FnMut(
            &Arc<dyn PhysicalExpr>,
        ) -> Result<datafusion::common::tree_node::TreeNodeRecursion>,
    ) -> Result<datafusion::common::tree_node::TreeNodeRecursion> {
        Ok(datafusion::common::tree_node::TreeNodeRecursion::Continue)
    }

    fn with_new_children(
        self: Arc<Self>,
        mut children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(Arc::new(TantivyDecodeExec::new(
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
        let deferred_fields = self.deferred_fields.clone();
        let ffhelpers = self.ffhelpers.clone();
        let schema = self.properties.eq_properties.schema().clone();

        let stream_gen = async_stream::try_stream! {
            use futures::StreamExt;
            while let Some(batch_res) = input_stream.next().await {
                let timer = baseline_metrics.elapsed_compute().timer();
                let result = batch_res
                    .and_then(|batch| decode_batch(batch, &deferred_fields, &ffhelpers, &schema));
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

fn decode_batch(
    batch: RecordBatch,
    deferred_fields: &[PhysicalDeferredField],
    ffhelpers: &HashMap<u32, Arc<FFHelper>>,
    schema: &SchemaRef,
) -> Result<RecordBatch> {
    let mut columns = batch.columns().to_vec();
    for field in deferred_fields {
        let ffhelper = ffhelper_for(ffhelpers, field)?;
        columns[field.col_idx] = decode_term_ordinals(ffhelper, field, &columns[field.col_idx])?;
    }
    RecordBatch::try_new(schema.clone(), columns)
        .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
}

/// Decodes a State 1 deferred column into a `Utf8View` / `BinaryView` array.
///
/// Rows are grouped by segment for one bulk dictionary lookup per segment, and Arrow's
/// `interleave` reassembles the original row order without copying the view buffers.
fn decode_term_ordinals(
    ffhelper: &FFHelper,
    field: &PhysicalDeferredField,
    column: &ArrayRef,
) -> Result<ArrayRef> {
    let union = DeferredUnion::try_new(column.as_ref())?;
    let num_rows = column.len();
    let num_segments = ffhelper.num_segments();
    let mut by_segment: Vec<Vec<(usize, TermOrdinal)>> = vec![Vec::new(); num_segments];
    let mut null_rows: Vec<usize> = Vec::new();
    for (row, value) in union.values().enumerate() {
        match value {
            DeferredValue::TermOrdinal {
                segment_ord,
                term_ord: Some(term_ord),
            } => {
                let entry = by_segment.get_mut(segment_ord as usize).ok_or_else(|| {
                    DataFusionError::Execution(format!(
                        "TantivyDecodeExec: column '{}' row {row} names segment {segment_ord}, but its index has {num_segments} segments",
                        field.display_name
                    ))
                })?;
                entry.push((row, term_ord));
            }
            DeferredValue::TermOrdinal { term_ord: None, .. } | DeferredValue::Null => {
                null_rows.push(row)
            }
            DeferredValue::DocAddress(_) => {
                return Err(DataFusionError::Internal(format!(
                    "TantivyDecodeExec: column '{}' row {row} still carries a doc address; a TantivyFetchExec must resolve it first",
                    field.display_name
                )));
            }
        }
    }

    let mut segment_arrays: Vec<ArrayRef> = Vec::new();
    let mut indices: Vec<(usize, usize)> = vec![(0, 0); num_rows];

    if !null_rows.is_empty() {
        segment_arrays.push(new_null_array(&field.output_data_type(), 1));
        let null_array_idx = segment_arrays.len() - 1;
        for row in null_rows {
            indices[row] = (null_array_idx, 0);
        }
    }

    for (segment_ord, rows) in by_segment.into_iter().enumerate() {
        if rows.is_empty() {
            continue;
        }
        let ords_array = UInt64Array::from_iter_values(rows.iter().map(|(_, ord)| *ord));
        let array = match (
            field.is_bytes,
            ffhelper.column(segment_ord as SegmentOrdinal, field.canonical.ff_index),
        ) {
            (true, FFType::Bytes(col)) => ords_to_bytes_array(col.clone(), &ords_array)?,
            (false, FFType::Text(col)) => ords_to_string_array(col.clone(), &ords_array)?,
            (is_bytes, _) => {
                return Err(DataFusionError::Execution(format!(
                    "TantivyDecodeExec: column '{}' at fast-field index {} is not a {} column",
                    field.display_name,
                    field.canonical.ff_index,
                    if is_bytes { "Bytes" } else { "Text" }
                )));
            }
        };
        segment_arrays.push(array);
        let array_idx = segment_arrays.len() - 1;
        for (idx_within_segment, (row, _)) in rows.into_iter().enumerate() {
            indices[row] = (array_idx, idx_within_segment);
        }
    }

    if segment_arrays.is_empty() {
        return Ok(new_null_array(&field.output_data_type(), num_rows));
    }

    let segment_arrays_refs: Vec<&dyn arrow_array::Array> =
        segment_arrays.iter().map(|a| a.as_ref()).collect();
    interleave(&segment_arrays_refs, &indices)
        .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
}
