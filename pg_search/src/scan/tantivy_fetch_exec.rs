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

//! The column-fetch half of a deferred lookup.
//!
//! `TantivyFetchExec` reads fast-field columns for rows that still carry a packed doc
//! address: a deferred string/bytes column moves from State 0 to State 1 (its term ordinal
//! in the segment dictionary, packed into the same word), and a `ctid_<plan_position>`
//! column moves from a packed address to a real ctid. The schema does not change; a State 1
//! row passes through untouched, so a scan that resolved ordinals already costs nothing
//! here.
//!
//! Fast-field reads are cheapest in doc order, which a join above the scan no longer keeps,
//! so this node is the part of the lookup that is sensitive to where it sits in the plan.

use std::sync::{Arc, Mutex};

use crate::api::HashMap;
use crate::index::fast_fields_helper::{FFHelper, FFType, for_each_segment};
use crate::index::mvcc::{MvccSatisfies, SegmentView};
use crate::postgres::customscan::joinscan::visibility_filter::{
    DeferredCtidMaterializationState, materialize_deferred_ctid,
};
use crate::scan::deferred_encode::{DeferredColumn, DeferredValue, unpack_doc_address};
use crate::scan::deferred_lookup::{
    LookupRebuildContext, PhysicalDeferredField, ffhelper_for, open_rebuilt_ffhelper,
    preserved_ordering, rebuild_missing_ffhelpers,
};
use crate::scan::execution_plan::UnsafeSendStream;

use arrow_array::{ArrayRef, RecordBatch, UInt64Array};
use arrow_schema::DataType;
use datafusion::common::stats::ColumnStatistics;
use datafusion::common::{DataFusionError, Result, Statistics};
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
use datafusion::physical_plan::{
    ChildStats, DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties, StatisticsArgs,
};
use tantivy::termdict::TermOrdinal;
use tantivy::{DocId, SegmentOrdinal};

/// A `ctid_<plan_position>` column (packed doc-addresses) that a `TantivyFetchExec` resolves
/// to real ctids, so a `VisibilityFilterExec` above it consumes real ctids directly.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CtidColumnLookup {
    /// Column index of the ctid column in the physical batch.
    pub col_idx: usize,
    /// The source's plan position, keying the resolver for its index.
    pub plan_position: usize,
}

/// Serialized shape for MPP dispatch: the deferred columns this node fetches, the ctid
/// columns it resolves, and their resolver indexes for the network-boundary rebuild.
#[derive(serde::Serialize, serde::Deserialize)]
struct FetchDispatchPayload {
    fetch_fields: Vec<PhysicalDeferredField>,
    ctid_columns: Vec<CtidColumnLookup>,
    /// `(plan_position, indexrelid)` for each wired ctid resolver, so a worker whose scan sits
    /// behind a network boundary can rebuild the resolver from its index segment view.
    ctid_resolver_indexes: Vec<(usize, u32)>,
}

/// One wired ctid resolver: the index it reads and the fast-field helper over its segments.
type CtidResolver = (u32, Arc<FFHelper>);

pub struct TantivyFetchExec {
    input: Arc<dyn ExecutionPlan>,
    /// Deferred string/bytes columns whose doc addresses this node resolves to term ordinals.
    fetch_fields: Vec<PhysicalDeferredField>,
    /// Keyed by index relid, so a self-join folds both aliases onto one helper. Term ordinals
    /// and doc ids are per-reader, so the surviving helper reads the other alias correctly
    /// only while both sources share a segment view. Keying by `(plan_position, indexrelid)`
    /// would remove the aliasing.
    ffhelpers: HashMap<u32, Arc<FFHelper>>,
    /// `ctid_<plan_position>` columns this exec resolves from packed doc-addresses to real ctids.
    ctid_columns: Vec<CtidColumnLookup>,
    /// Per-plan_position `(indexrelid, FFHelper)` for resolving the ctid columns, wired by
    /// `VisibilityCtidResolverRule` after plan construction. Indexed by plan_position.
    ctid_resolvers: Mutex<Vec<Option<CtidResolver>>>,
    properties: Arc<PlanProperties>,
    metrics: ExecutionPlanMetricsSet,
}

impl std::fmt::Debug for TantivyFetchExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TantivyFetchExec")
            .field("fetch", &self.fetch_fields.len())
            .field("ctids", &self.ctid_columns.len())
            .finish()
    }
}

impl TantivyFetchExec {
    pub fn new(
        input: Arc<dyn ExecutionPlan>,
        fetch_fields: Vec<PhysicalDeferredField>,
        ffhelpers: HashMap<u32, Arc<FFHelper>>,
        ctid_columns: Vec<CtidColumnLookup>,
    ) -> Result<Self> {
        let schema = input.schema();
        for field in &fetch_fields {
            let input_field = schema.fields().get(field.col_idx).ok_or_else(|| {
                DataFusionError::Plan(format!(
                    "TantivyFetchExec: column {} ('{}') is past the input schema",
                    field.col_idx, field.display_name
                ))
            })?;
            if input_field.data_type() != &DataType::UInt64 {
                return Err(DataFusionError::Plan(format!(
                    "TantivyFetchExec: column {} ('{}') is {:?}, expected a deferred UInt64 column",
                    field.col_idx,
                    field.display_name,
                    input_field.data_type()
                )));
            }
        }
        // Rows keep their order and their types; only a row's state changes. Only the
        // ordering is carried up, not the input's equivalence classes: above a hash join
        // those would let DataFusion rewrite a Top-K sort key onto the other join side and
        // move its dynamic filter off the probe scan.
        let properties = Arc::new(PlanProperties::new(
            preserved_ordering(&input, schema),
            input.properties().output_partitioning().clone(),
            EmissionType::Incremental,
            Boundedness::Bounded,
        ));
        let resolver_len = ctid_columns
            .iter()
            .map(|c| c.plan_position)
            .max()
            .map_or(0, |m| m + 1);
        Ok(Self {
            input,
            fetch_fields,
            ffhelpers,
            ctid_columns,
            ctid_resolvers: Mutex::new(vec![None; resolver_len]),
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
        })
    }

    pub fn fetch_fields(&self) -> &[PhysicalDeferredField] {
        &self.fetch_fields
    }

    pub fn ctid_columns(&self) -> &[CtidColumnLookup] {
        &self.ctid_columns
    }

    pub(crate) fn ffhelpers(&self) -> &HashMap<u32, Arc<FFHelper>> {
        &self.ffhelpers
    }

    /// Rebuilds this node over `input` with `fetch_fields`, keeping the ctid columns and the
    /// resolvers already wired for them.
    pub(crate) fn with_input_and_fields(
        &self,
        input: Arc<dyn ExecutionPlan>,
        fetch_fields: Vec<PhysicalDeferredField>,
    ) -> Result<Self> {
        let exec = TantivyFetchExec::new(
            input,
            fetch_fields,
            self.ffhelpers.clone(),
            self.ctid_columns.clone(),
        )?;
        for (plan_pos, resolver) in self
            .ctid_resolvers
            .lock()
            .expect("ctid_resolvers lock poisoned")
            .iter()
            .enumerate()
        {
            if let Some((indexrelid, ffhelper)) = resolver {
                exec.set_ctid_resolver(plan_pos, *indexrelid, ffhelper.clone());
            }
        }
        Ok(exec)
    }

    /// Wire the FFHelper that resolves the given source's ctid column. Mirrors
    /// `VisibilityFilterExec::set_ctid_resolver`.
    pub fn set_ctid_resolver(&self, plan_pos: usize, indexrelid: u32, ffhelper: Arc<FFHelper>) {
        let mut resolvers = self
            .ctid_resolvers
            .lock()
            .expect("ctid_resolvers lock poisoned");
        if plan_pos >= resolvers.len() {
            resolvers.resize(plan_pos + 1, None);
        }
        resolvers[plan_pos] = Some((indexrelid, ffhelper));
    }

    /// Serialize for leader dispatch. The `ffhelpers` are live and don't travel; the worker
    /// pulls them from the scans in its decoded subtree, keyed by index relid.
    pub(crate) fn encode_for_dispatch(&self) -> Result<Vec<u8>> {
        let ctid_resolver_indexes: Vec<(usize, u32)> = self
            .ctid_resolvers
            .lock()
            .expect("ctid_resolvers lock poisoned")
            .iter()
            .enumerate()
            .filter_map(|(pos, r)| r.as_ref().map(|(relid, _)| (pos, *relid)))
            .collect();
        serde_json::to_vec(&FetchDispatchPayload {
            fetch_fields: self.fetch_fields.clone(),
            ctid_columns: self.ctid_columns.clone(),
            ctid_resolver_indexes,
        })
        .map_err(|e| {
            DataFusionError::Internal(format!("TantivyFetchExec dispatch: serialize: {e}"))
        })
    }

    pub(crate) fn decode_for_dispatch(
        buf: &[u8],
        input: Arc<dyn ExecutionPlan>,
        mut ffhelpers: HashMap<u32, Arc<FFHelper>>,
        ctid_resolvers: Vec<(usize, u32, Arc<FFHelper>)>,
        index_segment_views: &[SegmentView],
        parallel_state: Option<*mut crate::postgres::ParallelScanState>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let payload: FetchDispatchPayload = serde_json::from_slice(buf).map_err(|e| {
            DataFusionError::Internal(format!("TantivyFetchExec dispatch: deserialize: {e}"))
        })?;
        rebuild_missing_ffhelpers(
            &payload.fetch_fields,
            &mut ffhelpers,
            LookupRebuildContext { parallel_state },
        )?;
        let exec =
            TantivyFetchExec::new(input, payload.fetch_fields, ffhelpers, payload.ctid_columns)?;
        // Wire the ctid resolvers found in the decoded subtree.
        for (plan_pos, indexrelid, ffhelper) in &ctid_resolvers {
            exec.set_ctid_resolver(*plan_pos, *indexrelid, Arc::clone(ffhelper));
        }
        // A ctid column whose scan sits behind a network boundary is not in the subtree; rebuild
        // its resolver from the index segment view (ctid access needs no field layout).
        for (plan_pos, indexrelid) in payload.ctid_resolver_indexes {
            if ctid_resolvers.iter().any(|(pos, _, _)| *pos == plan_pos) {
                continue;
            }
            let view = index_segment_views.get(plan_pos).cloned().ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "TantivyFetchExec dispatch: missing segment view for plan_position {plan_pos}"
                ))
            })?;
            let ffhelper =
                open_rebuilt_ffhelper(indexrelid, &[], MvccSatisfies::ParallelWorker(view))?;
            exec.set_ctid_resolver(plan_pos, indexrelid, ffhelper);
        }
        Ok(Arc::new(exec))
    }
}

impl DisplayAs for TantivyFetchExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // A resolved ctid is a fast-field fetch like any other column; only its consumer
        // differs, so both kinds share one list.
        let names = self
            .fetch_fields
            .iter()
            .map(|d| d.display_name.clone())
            .chain(
                self.ctid_columns
                    .iter()
                    .map(|c| format!("ctid_{}", c.plan_position)),
            )
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "TantivyFetchExec: fetch=[{names}]")
    }
}

impl ExecutionPlan for TantivyFetchExec {
    fn name(&self) -> &str {
        "TantivyFetchExec"
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

    fn child_stats_requests(&self, partition: Option<usize>) -> Vec<ChildStats> {
        vec![ChildStats::At(partition)]
    }

    /// The rows pass through unchanged; only the fetched columns' contents change.
    fn statistics_from_inputs(
        &self,
        input_stats: &[Arc<Statistics>],
        _args: &StatisticsArgs,
    ) -> Result<Arc<Statistics>> {
        let mut statistics = input_stats[0].as_ref().clone();
        for field in &self.fetch_fields {
            if let Some(column) = statistics.column_statistics.get_mut(field.col_idx) {
                *column = ColumnStatistics::new_unknown();
            }
        }
        Ok(Arc::new(statistics))
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
        Ok(Arc::new(self.with_input_and_fields(
            children.remove(0),
            self.fetch_fields.clone(),
        )?))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let mut input_stream = self.input.execute(partition, context)?;
        let baseline_metrics = BaselineMetrics::new(&self.metrics, partition);
        let fetch_fields = self.fetch_fields.clone();
        let ffhelpers = self.ffhelpers.clone();
        let ctid_columns = self.ctid_columns.clone();
        let ctid_resolvers = self
            .ctid_resolvers
            .lock()
            .expect("ctid_resolvers lock poisoned")
            .clone();

        let stream_gen = async_stream::try_stream! {
            use futures::StreamExt;
            let mut ctid_state = DeferredCtidMaterializationState::default();
            while let Some(batch_res) = input_stream.next().await {
                let timer = baseline_metrics.elapsed_compute().timer();
                let result = batch_res
                    .and_then(|batch| fetch_batch(batch, &fetch_fields, &ffhelpers))
                    .and_then(|batch| {
                        resolve_ctid_columns(batch, &ctid_columns, &ctid_resolvers, &mut ctid_state)
                    });
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

/// Replaces each fetched column's State 0 rows with their term ordinals.
fn fetch_batch(
    batch: RecordBatch,
    fetch_fields: &[PhysicalDeferredField],
    ffhelpers: &HashMap<u32, Arc<FFHelper>>,
) -> Result<RecordBatch> {
    if fetch_fields.is_empty() {
        return Ok(batch);
    }
    let schema = batch.schema();
    let mut columns = batch.columns().to_vec();
    for field in fetch_fields {
        let ffhelper = ffhelper_for(ffhelpers, field)?;
        columns[field.col_idx] = fetch_term_ordinals(ffhelper, field, &columns[field.col_idx])?;
    }
    RecordBatch::try_new(schema, columns)
        .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
}

/// Resolves a deferred column's doc addresses to term ordinals. A column with no State 0
/// rows is returned as is.
fn fetch_term_ordinals(
    ffhelper: &FFHelper,
    field: &PhysicalDeferredField,
    column: &ArrayRef,
) -> Result<ArrayRef> {
    let deferred = DeferredColumn::try_new(column.as_ref())?;
    let num_segments = ffhelper.num_segments();
    let mut packed_rows: Vec<(usize, u64)> = Vec::new();
    for (row, value) in deferred.values().enumerate() {
        if let DeferredValue::DocAddress(packed) = value {
            let (segment_ord, _) = unpack_doc_address(packed);
            if segment_ord as usize >= num_segments {
                return Err(DataFusionError::Execution(format!(
                    "TantivyFetchExec: column '{}' row {row} names segment {segment_ord}, but its index has {num_segments} segments",
                    field.display_name
                )));
            }
            packed_rows.push((row, packed));
        }
    }
    if packed_rows.is_empty() {
        return Ok(Arc::clone(column));
    }

    let mut resolved: Vec<(usize, SegmentOrdinal, Option<TermOrdinal>)> =
        Vec::with_capacity(packed_rows.len());
    for_each_segment(
        num_segments,
        packed_rows.into_iter(),
        |segment_ord, rows| {
            let ids: Vec<DocId> = rows.iter().map(|(_, doc_id)| *doc_id).collect();
            let mut ords: Vec<Option<TermOrdinal>> = vec![None; ids.len()];
            match (
                field.is_bytes,
                ffhelper.column(segment_ord, field.canonical.ff_index),
            ) {
                (true, FFType::Bytes(col)) => col.ords().first_vals(&ids, &mut ords),
                (false, FFType::Text(col)) => col.ords().first_vals(&ids, &mut ords),
                (is_bytes, _) => {
                    return Err(DataFusionError::Execution(format!(
                        "TantivyFetchExec: column '{}' at fast-field index {} is not a {} column",
                        field.display_name,
                        field.canonical.ff_index,
                        if is_bytes { "Bytes" } else { "Text" }
                    )));
                }
            }
            resolved.extend(
                rows.into_iter()
                    .zip(ords)
                    .map(|((row, _), ord)| (row, segment_ord, ord)),
            );
            Ok(())
        },
    )?;

    Ok(deferred.with_term_ordinals(resolved))
}

/// Replaces each configured ctid column's packed doc-addresses with real ctids, using the
/// resolver wired for that column's plan position. A no-op when no ctid columns are configured.
fn resolve_ctid_columns(
    batch: RecordBatch,
    ctid_columns: &[CtidColumnLookup],
    resolvers: &[Option<CtidResolver>],
    state: &mut DeferredCtidMaterializationState,
) -> Result<RecordBatch> {
    if ctid_columns.is_empty() {
        return Ok(batch);
    }
    let schema = batch.schema();
    let mut columns = batch.columns().to_vec();
    for ctid_column in ctid_columns {
        let (_, ffhelper) = resolvers
            .get(ctid_column.plan_position)
            .and_then(|r| r.as_ref())
            .ok_or_else(|| {
                DataFusionError::Execution(format!(
                    "TantivyFetchExec: no ctid resolver wired for plan_position {}. VisibilityCtidResolverRule must run before execute.",
                    ctid_column.plan_position
                ))
            })?;
        let packed = columns[ctid_column.col_idx]
            .as_any()
            .downcast_ref::<UInt64Array>()
            .ok_or_else(|| {
                DataFusionError::Execution(format!(
                    "TantivyFetchExec: ctid column (idx {}) is not UInt64",
                    ctid_column.col_idx
                ))
            })?;
        columns[ctid_column.col_idx] = materialize_deferred_ctid(ffhelper, packed, state)?;
    }
    RecordBatch::try_new(schema, columns)
        .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
}
