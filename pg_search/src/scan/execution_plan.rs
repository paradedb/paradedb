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

//! DataFusion `ExecutionPlan` implementations for scanning `pg_search` indexes.
//!
//! See the [JoinScan README](../../postgres/customscan/joinscan/README.md) for
//! how `PgSearchScanPlan` integrates with the JoinScan physical plan and
//! dynamic filters.
//!
//! This module provides the `PgSearchScanPlan`, which scans `pg_search` index segments as a
//! single lazily-claimed partition: segments are claimed dynamically from `ParallelScanState`
//! in parallel execution, or chained end-to-end when serial.

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use arrow_array::RecordBatch;
use arrow_schema::{SchemaRef, SortOptions};
use datafusion::common::stats::{ColumnStatistics, Precision};
use datafusion::common::{DataFusionError, Result, Statistics};
use datafusion::execution::{RecordBatchStream, SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::{EquivalenceProperties, PhysicalExpr, PhysicalSortExpr};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::expressions::{Column, DynamicFilterPhysicalExpr};
use datafusion::physical_plan::filter_pushdown::{
    ChildPushdownResult, FilterPushdownPhase, FilterPushdownPropagation, PushedDown,
};
use datafusion::physical_plan::metrics::{
    BaselineMetrics, ExecutionPlanMetricsSet, MetricBuilder, MetricsSet, RecordOutput,
};
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
};
use futures::Stream;
use tantivy::Score;

use crate::index::fast_fields_helper::FFHelper;
use crate::index::fast_fields_helper::WhichFastField;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::explain::ExplainFormat;
use crate::postgres::customscan::parallel::list_segment_ids;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::options::{SortByDirection, SortByField};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::ParallelScanState;
use crate::query::SearchQueryInput;
use crate::scan::late_materialization::DeferredField;
use crate::scan::pre_filter::{collect_filters, try_dynamic_filter_pushdown, PreFilter};
use crate::scan::Scanner;
use pgrx::pg_sys;

/// A wrapper that implements Send + Sync unconditionally.
/// UNSAFE: Only use this when you guarantee single-threaded access or manual synchronization.
/// This is safe in pg_search because Postgres extensions run single-threaded.
#[derive(Clone)]
pub(crate) struct UnsafeSendSync<T>(pub T);

unsafe impl<T> Send for UnsafeSendSync<T> {}
unsafe impl<T> Sync for UnsafeSendSync<T> {}

/// Ingredients needed to construct a Scanner for deferred search.
#[derive(Clone)]
pub struct ScannerConfig {
    pub which_fast_fields: Vec<WhichFastField>,
    pub heap_relid: u32,
    pub batch_size_hint: Option<usize>,
    /// `need_scores` the index reader was opened with. Carried so a leader-dispatched worker
    /// re-opens its reader with the same scoring behavior (the reader itself can't travel).
    pub score_needed: bool,
}

/// State for a scan partition.
/// Uses `Arc<FFHelper>` so the same FFHelper can be shared across multiple partitions.
pub struct ScanState {
    pub parallel_state: Option<*mut ParallelScanState>,
    pub source_idx: Option<usize>,
    pub planner_estimated_rows: u64,
    pub scanner_config: ScannerConfig,
    pub ffhelper: Arc<FFHelper>,
    pub visibility: Box<VisibilityChecker>,
    pub reader: SearchIndexReader,
}

/// A DataFusion `ExecutionPlan` for scanning `pg_search` index segments.
///
/// The plan exposes exactly one partition containing a lazily-evaluated
/// `MultiSegmentSearchResults` stream. Segments are claimed dynamically from
/// `ParallelScanState` when running in parallel (load-balancing across workers as they
/// process data) or chained sequentially when serial.
pub struct PgSearchScanPlan {
    /// State for this single-partition scan.
    ///
    /// We use a Mutex to allow taking ownership of the scanner during `execute()`.
    /// We wrap the state in `UnsafeSendSync` to satisfy `ExecutionPlan`'s `Send` + `Sync`
    /// requirements. This is safe because we are running in a single-threaded
    /// environment (Postgres), which also means that the duration for which we
    /// hold this Mutex does not impact performance.
    state: Mutex<Option<UnsafeSendSync<ScanState>>>,
    /// Estimated row count, computed once at construction.
    /// Stored separately so `partition_statistics` is deterministic, even after
    /// the state has been consumed.
    planner_estimated_rows: u64,
    /// Number of segments this plan will process, derived at construction time
    /// from ParallelScanState or the reader, and kept around for EXPLAIN after
    /// the state is consumed.
    segment_count: usize,
    properties: Arc<PlanProperties>,
    resolved_query: SearchQueryInput,
    /// Dynamic filters pushed down from parent operators (e.g. Top K threshold
    /// from SortExec, join-key bounds from HashJoinExec). Each batch produced
    /// by the scanner is filtered against all of these expressions so that rows
    /// which cannot contribute to the final result are pruned early.
    dynamic_filters: Vec<Arc<dyn PhysicalExpr>>,
    /// Metrics for EXPLAIN ANALYZE.
    metrics: ExecutionPlanMetricsSet,
    deferred_fields: Vec<DeferredField>,
    /// Shared FFHelper for deferred lookup and deferred visibility.
    ///
    /// A scan may participate in late materialization, deferred visibility, or both.
    /// Callers decide whether they should use it by checking the deferred metadata,
    /// and cloning the Arc is cheap.
    ffhelper: Option<Arc<FFHelper>>,
    pub indexrelid: u32,
    /// The JoinScan source identity when visibility is deferred.
    deferred_ctid_plan_position: Option<usize>,
    /// When true, a HashJoin InList was successfully pushed down to a TermSet query.
    dynamic_filter_pushdown: Arc<AtomicBool>,
    /// Sort order preserved across `with_filter_pushdown` rebuilds so the
    /// rebuilt plan keeps its equivalence properties.
    sort_order: Option<SortByField>,
    /// Captures the per-segment `TermSetStrategy` chosen by the tantivy planner
    /// for the pushed-down dynamic filter (issue #4895). Last-segment-wins is
    /// fine because `EXPLAIN ANALYZE` only asks "did any segment use it?".
    /// Stored as `u8` so it can live behind an `AtomicU8`; round-tripped
    /// through `tantivy::query::StrategyTag` at render time. A value of
    /// `StrategyTag::None as u8` (= 0) means no `TermSetWeight` ran on
    /// this scan to write a tag — the EXPLAIN renderer falls back to
    /// `=true` in that case.
    dynamic_filter_strategy: Arc<AtomicU8>,
}

impl std::fmt::Debug for PgSearchScanPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgSearchScanPlan")
            .field("properties", &self.properties)
            .finish()
    }
}

impl PgSearchScanPlan {
    /// Creates a new PgSearchScanPlan with pre-opened segments.
    ///
    /// # Arguments
    ///
    /// * `state` - The pre-opened scan state (or None for tests)
    /// * `schema` - Arrow schema for the output
    /// * `resolved_query` - The filter-combined, param-solved query the readers were opened
    ///   with. Used for EXPLAIN and shipped on dispatch.
    /// * `sort_order` - Optional sort order declaration for equivalence properties
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: Option<ScanState>,
        schema: SchemaRef,
        resolved_query: SearchQueryInput,
        sort_order: Option<&SortByField>,
        deferred_fields: Vec<DeferredField>,
        ffhelper: Option<Arc<FFHelper>>,
        indexrelid: u32,
        deferred_ctid_plan_position: Option<usize>,
    ) -> Self {
        let needs_ffhelper = !deferred_fields.is_empty() || deferred_ctid_plan_position.is_some();
        if needs_ffhelper && ffhelper.is_none() {
            panic!("deferred lookup/visibility requires an FFHelper, but ffhelper is None");
        }
        // Ensure we always return exactly one partition to satisfy DataFusion distribution
        // requirements (e.g. HashJoinExec mode=CollectLeft requires SinglePartition).
        // If state is None, execute() will return an EmptyStream for this single partition.
        let eq_properties = build_equivalence_properties(schema, sort_order);

        let properties = Arc::new(PlanProperties::new(
            eq_properties,
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        ));

        let planner_estimated_rows = state
            .as_ref()
            .map(|s| s.planner_estimated_rows)
            .unwrap_or(0);
        let segment_count = state
            .as_ref()
            .map(|s| match s.parallel_state {
                Some(ps) => unsafe { (*ps).source_segment_count(s.source_idx.unwrap_or(0)) },
                None => s.reader.segment_ids().len(),
            })
            .unwrap_or(0);

        Self {
            state: Mutex::new(state.map(UnsafeSendSync)),
            planner_estimated_rows,
            segment_count,
            properties,
            resolved_query,
            dynamic_filters: Vec::new(),
            metrics: ExecutionPlanMetricsSet::new(),
            deferred_fields,
            ffhelper,
            indexrelid,
            deferred_ctid_plan_position,
            dynamic_filter_pushdown: Arc::new(AtomicBool::new(false)),
            sort_order: sort_order.cloned(),
            dynamic_filter_strategy: Arc::new(AtomicU8::new(0)),
        }
    }

    pub fn has_deferred_fields(&self) -> bool {
        !self.deferred_fields.is_empty()
    }

    pub fn ffhelper(&self) -> Option<Arc<FFHelper>> {
        self.ffhelper.clone()
    }

    pub fn deferred_ctid_plan_position(&self) -> Option<usize> {
        self.deferred_ctid_plan_position
    }

    /// Serialize this scan into a transport-neutral descriptor for leader dispatch.
    ///
    /// Only the recipe and the reader-rebuild inputs travel; the live `ScanState` (tantivy
    /// readers, visibility checkers) is process-local and gets rebuilt on the receiving worker
    /// from its own `ParallelScanState`. `resolved_query` is the filter-combined,
    /// param-solved query the reader was opened with, so the receiver needs no `ExprContext`.
    pub(crate) fn encode_for_dispatch(&self) -> Result<Vec<u8>> {
        let state_guard = self
            .state
            .lock()
            .map_err(|e| DataFusionError::Internal(format!("lock PgSearchScanPlan state: {e}")))?;
        let state = state_guard.as_ref().ok_or_else(|| {
            DataFusionError::Internal("PgSearchScan dispatch: partition already consumed".into())
        })?;
        let (source_idx, planner_estimated_rows, scanner_config) = (
            state.0.source_idx,
            state.0.planner_estimated_rows,
            state.0.scanner_config.clone(),
        );

        let schema = self.properties.eq_properties.schema().clone();
        let schema_proto: datafusion_proto::protobuf::Schema =
            schema.as_ref().try_into().map_err(|e| {
                DataFusionError::Internal(format!("PgSearchScan dispatch: schema encode: {e}"))
            })?;

        let descriptor = ScanDispatchDescriptor {
            schema_proto: prost::Message::encode_to_vec(&schema_proto),
            query: self.resolved_query.clone(),
            score_needed: scanner_config.score_needed,
            sort_order: self.sort_order.clone(),
            indexrelid: self.indexrelid,
            deferred_fields: self.deferred_fields.clone(),
            deferred_ctid_plan_position: self.deferred_ctid_plan_position,
            which_fast_fields: scanner_config.which_fast_fields,
            heap_relid: scanner_config.heap_relid,
            batch_size_hint: scanner_config.batch_size_hint,
            source_idx,
            planner_estimated_rows,
        };
        serde_json::to_vec(&descriptor).map_err(|e| {
            DataFusionError::Internal(format!("PgSearchScan dispatch: serialize: {e}"))
        })
    }

    /// Rebuild a scan from a dispatch descriptor, injecting the receiving worker's runtime
    /// state. Mirrors the tail of `PgSearchTableProvider::scan_inner`: open the index reader
    /// under the worker's MVCC view, build the fast-field helper + visibility checker, and wrap
    /// a single lazy partition that claims segments at runtime from `parallel_state`.
    pub(crate) fn decode_for_dispatch(
        buf: &[u8],
        parallel_state: Option<*mut ParallelScanState>,
        expr_context: Option<*mut pg_sys::ExprContext>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let descriptor: ScanDispatchDescriptor = serde_json::from_slice(buf).map_err(|e| {
            DataFusionError::Internal(format!("PgSearchScan dispatch: deserialize: {e}"))
        })?;

        let schema_proto = <datafusion_proto::protobuf::Schema as prost::Message>::decode(
            descriptor.schema_proto.as_slice(),
        )
        .map_err(|e| {
            DataFusionError::Internal(format!("PgSearchScan dispatch: schema decode: {e}"))
        })?;
        let schema: SchemaRef = Arc::new((&schema_proto).try_into().map_err(|e| {
            DataFusionError::Internal(format!("PgSearchScan dispatch: schema parse: {e}"))
        })?);

        let index_rel = PgSearchRelation::open(pg_sys::Oid::from(descriptor.indexrelid));
        let heap_rel = PgSearchRelation::open(pg_sys::Oid::from(descriptor.heap_relid));

        // MVCC view: an MPP source (source_idx Some) reads its per-source frozen segment
        // list from `ParallelScanState`; a standard parallel scan (source_idx None) reads
        // the worker's full segment list. Mirrors the MVCC dispatch in `scan_inner`.
        let mvcc = match (descriptor.source_idx, parallel_state) {
            (None, Some(ps)) => MvccSatisfies::ParallelWorker(unsafe { list_segment_ids(ps) }),
            (Some(idx), Some(ps)) => {
                MvccSatisfies::ParallelWorker(unsafe { (*ps).segment_ids_for_source(idx) })
            }
            (_, None) => MvccSatisfies::Snapshot,
        };

        let query = descriptor.query;
        let needs_tokenizer = query.needs_tokenizer();
        let reader = SearchIndexReader::open_with_context(
            &index_rel,
            query.clone(),
            descriptor.score_needed,
            mvcc,
            expr_context.and_then(std::ptr::NonNull::new),
            // TODO: MPP is currently disabled when a scan requires parameter solving: see
            // https://github.com/paradedb/paradedb/issues/5445.
            None,
            needs_tokenizer,
        )
        .map_err(|e| {
            DataFusionError::Internal(format!("PgSearchScan dispatch: open reader: {e}"))
        })?;

        let ffhelper = Arc::new(FFHelper::with_fields(
            &reader,
            &descriptor.which_fast_fields,
        ));
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let visibility = VisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);

        let scanner_config = ScannerConfig {
            which_fast_fields: descriptor.which_fast_fields,
            heap_relid: descriptor.heap_relid,
            batch_size_hint: descriptor.batch_size_hint,
            score_needed: descriptor.score_needed,
        };
        let state = ScanState {
            parallel_state,
            source_idx: descriptor.source_idx,
            planner_estimated_rows: descriptor.planner_estimated_rows,
            scanner_config,
            ffhelper: Arc::clone(&ffhelper),
            visibility: Box::new(visibility) as Box<VisibilityChecker>,
            reader,
        };

        let deferred = descriptor.deferred_fields;
        let deferred_ctid_plan_position = descriptor.deferred_ctid_plan_position;
        let ffhelper_arg = if deferred.is_empty() && deferred_ctid_plan_position.is_none() {
            None
        } else {
            Some(ffhelper)
        };

        Ok(Arc::new(PgSearchScanPlan::new(
            Some(state),
            schema,
            query,
            descriptor.sort_order.as_ref(),
            deferred,
            ffhelper_arg,
            descriptor.indexrelid,
            deferred_ctid_plan_position,
        )))
    }
}

/// Transport-neutral description of a `PgSearchScanPlan` for leader dispatch. Carries the
/// recipe plus the inputs needed to re-open the reader on the receiving worker; the live tantivy
/// state is rebuilt there from the worker's own `ParallelScanState`.
#[derive(serde::Serialize, serde::Deserialize)]
struct ScanDispatchDescriptor {
    /// Arrow schema, `datafusion_proto::protobuf::Schema`-encoded (arrow schema isn't serde).
    schema_proto: Vec<u8>,
    query: SearchQueryInput,
    score_needed: bool,
    sort_order: Option<SortByField>,
    indexrelid: u32,
    deferred_fields: Vec<DeferredField>,
    deferred_ctid_plan_position: Option<usize>,
    which_fast_fields: Vec<WhichFastField>,
    heap_relid: u32,
    batch_size_hint: Option<usize>,
    /// `Some(i)` for an MPP source (claims from source `i`'s pool); `None` for
    /// single-counter checkout (basescan and non-MPP parallel joins). All-sources position.
    source_idx: Option<usize>,
    planner_estimated_rows: u64,
}

/// Build `EquivalenceProperties` with the specified sort ordering.
///
/// If `sort_order` is `Some`, the returned properties will declare that the
/// data is sorted by the specified field in the specified direction.
/// If `sort_order` is `None`, returns empty equivalence properties.
fn build_equivalence_properties(
    schema: SchemaRef,
    sort_order: Option<&SortByField>,
) -> EquivalenceProperties {
    let mut eq_properties = EquivalenceProperties::new(schema.clone());

    if let Some(sort_field) = sort_order {
        // Find the column index for the sort field
        let field_name = sort_field.field_name.as_ref();
        if let Some((col_idx, _)) = schema.column_with_name(field_name) {
            let sort_options = SortOptions {
                descending: matches!(sort_field.direction, SortByDirection::Desc),
                // Tantivy's sort behavior:
                // - ASC: nulls sort first
                // - DESC: nulls sort last
                nulls_first: matches!(sort_field.direction, SortByDirection::Asc),
            };

            let sort_expr = PhysicalSortExpr {
                expr: Arc::new(Column::new(field_name, col_idx)),
                options: sort_options,
            };

            // Add the ordering to the equivalence properties
            eq_properties.add_ordering(std::iter::once(sort_expr));
        }
    }

    eq_properties
}

/// Translate a `tantivy::query::StrategyTag` back into the human-readable
/// strategy name surfaced in `EXPLAIN ANALYZE` output.
fn strategy_name(strategy: tantivy::query::StrategyTag) -> &'static str {
    use tantivy::query::StrategyTag;
    match strategy {
        StrategyTag::None => "none",
        StrategyTag::Gallop => "gallop",
        StrategyTag::Linear => "linear",
        StrategyTag::Bitset => "bitset_from_postings",
        StrategyTag::Automaton => "automaton",
        StrategyTag::Empty => "empty",
    }
}

impl DisplayAs for PgSearchScanPlan {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PgSearchScan: segments={}", self.segment_count,)?;
        if !self.dynamic_filters.is_empty() {
            write!(f, ", dynamic_filters={}", self.dynamic_filters.len())?;
        }
        if self.dynamic_filter_pushdown.load(Ordering::Relaxed) {
            // Render a single token. `TermSetWeight` writes the chosen
            // `StrategyTag` to the sink on every dispatch, so the value
            // is the strategy name (`gallop` / `linear` /
            // `bitset_from_postings` / `automaton` / `empty`). Falls
            // back to `true` only when pushdown was indicated but no
            // `TermSetWeight` ran to record a tag — e.g., the dynamic
            // filter handled a non-TermSet shape, or the scan
            // short-circuited before any segment was processed.
            let tag = self.dynamic_filter_strategy.load(Ordering::Relaxed);
            let strategy = tantivy::query::StrategyTag::try_from(tag)
                .unwrap_or(tantivy::query::StrategyTag::None);
            if matches!(strategy, tantivy::query::StrategyTag::None) {
                write!(f, ", dynamic_filter_pushdown=true")?;
            } else {
                write!(f, ", dynamic_filter_pushdown={}", strategy_name(strategy))?;
            }
        }
        write!(f, ", query={}", self.resolved_query.explain_format())
    }
}

impl ExecutionPlan for PgSearchScanPlan {
    fn name(&self) -> &str {
        "PgSearchScan"
    }

    fn properties(&self) -> &Arc<PlanProperties> {
        &self.properties
    }

    fn partition_statistics(&self, partition: Option<usize>) -> Result<Arc<Statistics>> {
        let num_rows = match partition {
            Some(0) | None => Precision::Inexact(self.planner_estimated_rows as usize),
            Some(_) => Precision::Absent,
        };

        let column_statistics = self
            .properties
            .eq_properties
            .schema()
            .fields
            .iter()
            .map(|_| ColumnStatistics::default())
            .collect();

        Ok(Arc::new(Statistics {
            num_rows,
            total_byte_size: Precision::Absent,
            column_statistics,
        }))
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }

    fn with_new_children(
        self: Arc<Self>,
        _children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn execute(
        &self,
        partition: usize,
        _context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let mut state_guard = self.state.lock().map_err(|e| {
            DataFusionError::Internal(format!("Failed to lock PgSearchScanPlan state: {e}"))
        })?;

        if partition >= self.properties.output_partitioning().partition_count() {
            return Err(DataFusionError::Internal(format!(
                "Partition {} out of range (have {} partitions)",
                partition,
                self.properties.output_partitioning().partition_count()
            )));
        }

        // Handle the case where no state was provided (test cases) or already consumed.
        let state_opt = state_guard.take();
        if state_opt.is_none() {
            let schema = self.properties.eq_properties.schema().clone();
            return Ok(Box::pin(unsafe {
                UnsafeSendStream::new(futures::stream::empty(), schema)
            }));
        }

        let UnsafeSendSync(ScanState {
            parallel_state,
            source_idx,
            planner_estimated_rows,
            scanner_config,
            ffhelper,
            mut visibility,
            mut reader,
        }) = state_opt.unwrap();

        let has_dynamic_filters = !self.dynamic_filters.is_empty();
        let rows_scanned = has_dynamic_filters
            .then(|| MetricBuilder::new(&self.metrics).counter("rows_scanned", partition));
        let rows_pruned = has_dynamic_filters
            .then(|| MetricBuilder::new(&self.metrics).counter("rows_pruned", partition));

        let baseline_metrics = BaselineMetrics::new(&self.metrics, partition);
        let schema = self.properties.eq_properties.schema().clone();
        let score_column_schema_idx: Option<usize> = schema
            .column_with_name(&WhichFastField::Score.name())
            .map(|(idx, _)| idx);
        let dynamic_filters = self.dynamic_filters.clone();

        // Capture self-references for the async block
        let dynamic_filter_pushdown = self.dynamic_filter_pushdown.clone();
        let dynamic_filter_strategy = self.dynamic_filter_strategy.clone();

        let stream_gen = async_stream::try_stream! {
            // Optimized Search Integration:
            // We initialize the search here, inside the stream, because for HashJoin
            // this block is evaluated lazily during the first `poll_next`, which happens
            // AFTER the build side has completed and dynamic filters are published.
            let mut dynamic_filters = dynamic_filters.clone();
            if !dynamic_filters.is_empty()
                && try_dynamic_filter_pushdown(
                    &mut reader,
                    &mut dynamic_filters,
                    Some(dynamic_filter_strategy.clone()),
                )
            {
                dynamic_filter_pushdown.store(true, Ordering::Relaxed);
            }


            let search_results = match parallel_state {
                Some(ps) => reader.search_lazy(ps, source_idx, planner_estimated_rows),
                // No shared scan state even though the plan may carry per-source claim
                // markers: the serial fallback (size gate, short launch). The plan was
                // built while MPP was eligible but executes as a plain serial scan, so
                // search everything. Workers always get `parallel_state` injected at
                // decode and never take this arm.
                None => reader.search(),
            };
            let mut scanner = Scanner::new(
                search_results,
                scanner_config.batch_size_hint,
                scanner_config.which_fast_fields,
                scanner_config.heap_relid,
            );
            let df_batch_size = crate::gucs::dynamic_filter_batch_size();
            if df_batch_size > 0 {
                scanner.set_batch_size(df_batch_size as usize);
            }

            loop {
                let timer = baseline_metrics.elapsed_compute().timer();
                let (pre_filters, score_threshold) = build_filters(&dynamic_filters, &schema, score_column_schema_idx);
                let pre_filters_wrapper = if pre_filters.is_empty() {
                    None
                } else {
                    Some(crate::scan::pre_filter::PreFilters {
                        filters: &pre_filters,
                        schema: &schema,
                    })
                };

                scanner.set_score_threshold(score_threshold);
                let next_batch = scanner.next(
                    &ffhelper,
                    &mut visibility,
                    pre_filters_wrapper.as_ref(),
                );
                timer.done();

                match next_batch {
                    Some(batch) => {
                        let record_batch = batch.to_record_batch(&schema);
                        yield record_batch.record_output(&baseline_metrics);
                    }
                    None => {
                        // Flush pre-materialization filter stats from Scanner.
                        if let Some(ref counter) = rows_scanned {
                            counter.add(scanner.pre_filter_rows_scanned);
                        }
                        if let Some(ref counter) = rows_pruned {
                            counter.add(scanner.pre_filter_rows_pruned);
                        }
                        break;
                    }
                }
            }
            baseline_metrics.done();
        };

        // SAFETY: pg_search operates in a single-threaded Tokio executor within Postgres,
        // so it is safe to wrap !Send types for use within DataFusion.
        let stream = unsafe {
            UnsafeSendStream::new(stream_gen, self.properties.eq_properties.schema().clone())
        };
        Ok(Box::pin(stream))
    }

    fn metrics(&self) -> Option<MetricsSet> {
        Some(self.metrics.clone_inner())
    }

    fn handle_child_pushdown_result(
        &self,
        phase: FilterPushdownPhase,
        child_pushdown_result: ChildPushdownResult,
        _config: &datafusion::common::config::ConfigOptions,
    ) -> Result<FilterPushdownPropagation<Arc<dyn ExecutionPlan>>> {
        // Only handle dynamic filters in the Post phase (Top K pushdown happens here).
        if !matches!(phase, FilterPushdownPhase::Post) {
            return Ok(FilterPushdownPropagation::if_all(child_pushdown_result));
        }

        // Collect all DynamicFilterPhysicalExpr instances from the parent filters.
        // Multiple sources may push dynamic filters (e.g. Top K from SortExec,
        // join-key bounds from HashJoinExec). We accept and apply all of them.
        let mut dynamic_filters = Vec::new();
        let mut filters = Vec::with_capacity(child_pushdown_result.parent_filters.len());

        for filter_result in &child_pushdown_result.parent_filters {
            if filter_result.filter.is::<DynamicFilterPhysicalExpr>() {
                dynamic_filters.push(Arc::clone(&filter_result.filter));
                filters.push(PushedDown::Yes);
            } else {
                filters.push(filter_result.any());
            }
        }

        if !dynamic_filters.is_empty() {
            // Transfer state from the old plan to the new one.
            let state = self
                .state
                .lock()
                .map_err(|e| {
                    DataFusionError::Internal(format!(
                        "Failed to lock PgSearchScanPlan state during filter pushdown: {e}"
                    ))
                })?
                .take()
                .map(|s| s.0);

            let resolved_query = self.resolved_query.clone();

            let new_plan = Arc::new(PgSearchScanPlan {
                state: Mutex::new(state.map(UnsafeSendSync)),
                planner_estimated_rows: self.planner_estimated_rows,
                segment_count: self.segment_count,
                properties: self.properties.clone(),
                resolved_query,
                dynamic_filters,
                metrics: self.metrics.clone(),
                deferred_fields: self.deferred_fields.clone(),
                ffhelper: self.ffhelper.clone(),
                indexrelid: self.indexrelid,
                deferred_ctid_plan_position: self.deferred_ctid_plan_position,
                dynamic_filter_pushdown: Arc::new(AtomicBool::new(
                    self.dynamic_filter_pushdown.load(Ordering::Relaxed),
                )),
                sort_order: self.sort_order.clone(),
                dynamic_filter_strategy: Arc::new(AtomicU8::new(
                    self.dynamic_filter_strategy.load(Ordering::Relaxed),
                )),
            });
            Ok(
                FilterPushdownPropagation::with_parent_pushdown_result(filters)
                    .with_updated_node(new_plan as Arc<dyn ExecutionPlan>),
            )
        } else {
            Ok(FilterPushdownPropagation::if_all(child_pushdown_result))
        }
    }
}

/// Evaluate the current dynamic filter expressions and convert them into
/// [`PreFilter`]s that the `Scanner` can apply before column materialization.
///
/// While doing that, we also attempt to extract a top-k score threshold if one exists.
/// We process the threshold-containing expression as we do the rest of the expressions.
/// The threshold-containing expression may be top-level, so we need to allow for
/// the rest of the expression to be applied.
///
/// This is called on every `poll_next` (or loop iteration) so that tightening thresholds (e.g.
/// from Top K) are picked up immediately.
///
/// Only filter predicates that can be lowered to fast-field or term-ordinal
/// comparisons are retained. Anything else (unsupported types, non-comparison
/// operators) is silently dropped — the parent operator is still responsible
/// for enforcing the full predicate, so correctness is not affected.
fn build_filters(
    dynamic_filters: &[Arc<dyn PhysicalExpr>],
    schema: &SchemaRef,
    score_col_schema_idx: Option<usize>,
) -> (Vec<PreFilter>, Option<Score>) {
    let mut filters = Vec::new();
    let mut score_threshold = None;
    for df in dynamic_filters {
        if let Some(dynamic) = df.downcast_ref::<DynamicFilterPhysicalExpr>() {
            if let Ok(current_expr) = dynamic.current() {
                collect_filters(
                    &current_expr,
                    schema,
                    &mut filters,
                    score_col_schema_idx,
                    &mut score_threshold,
                );
            }
        } else {
            collect_filters(
                df,
                schema,
                &mut filters,
                score_col_schema_idx,
                &mut score_threshold,
            );
        }
    }
    (filters, score_threshold)
}

/// A wrapper that unsafely implements Send for a Stream.
///
/// This is used to wrap `ScanStream` which is !Send because it contains Tantivy and Postgres
/// state that is not Send. This is safe because pg_search operates in a single-threaded
/// Tokio executor within Postgres, and these objects will never cross thread boundaries.
pub(crate) struct UnsafeSendStream<T> {
    stream: T,
    schema: SchemaRef,
}

impl<T> UnsafeSendStream<T> {
    pub(crate) unsafe fn new(stream: T, schema: SchemaRef) -> Self {
        Self { stream, schema }
    }
}

unsafe impl<T> Send for UnsafeSendStream<T> {}

impl<T: Stream> Stream for UnsafeSendStream<T> {
    type Item = T::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().stream).poll_next(cx) }
    }
}

impl<T: Stream<Item = Result<RecordBatch>>> RecordBatchStream for UnsafeSendStream<T> {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use std::sync::Arc;

    use arrow_schema::{Schema, SchemaRef};
    use pgrx::prelude::*;

    use crate::query::SearchQueryInput;

    use super::PgSearchScanPlan;

    fn empty_schema() -> SchemaRef {
        Arc::new(Schema::empty())
    }

    #[pg_test]
    #[should_panic(expected = "deferred lookup/visibility requires an FFHelper")]
    fn deferred_visibility_requires_ffhelper() {
        let _ = PgSearchScanPlan::new(
            None,
            empty_schema(),
            SearchQueryInput::All,
            None,
            Vec::new(),
            None,
            0,
            Some(1),
        );
    }

    #[pg_test]
    fn can_construct_plan() {
        let _ = PgSearchScanPlan::new(
            None,
            empty_schema(),
            SearchQueryInput::All,
            None,
            Vec::new(),
            None,
            0,
            None,
        );
    }
}
