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
//! This module provides the `PgSearchScanPlan`, which handles scanning of `pg_search`
//! index segments. It supports both single-partition (serial) and multi-partition
//! (parallel or sorted) scans.
//!
//! For sorted scans, `create_sorted_scan` can be used to wrap the plan in a
//! `SortPreservingMergeExec` to merge sorted outputs from multiple segments.

use std::any::Any;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use arrow_array::RecordBatch;
use arrow_schema::{SchemaRef, SortOptions};
use datafusion::common::stats::{ColumnStatistics, Precision};
use datafusion::common::{DataFusionError, Result, Statistics};
use datafusion::execution::{RecordBatchStream, SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::expressions::{Column, DynamicFilterPhysicalExpr};
use datafusion::physical_expr::{
    EquivalenceProperties, LexOrdering, PhysicalExpr, PhysicalSortExpr,
};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::filter_pushdown::{
    ChildPushdownResult, FilterPushdownPhase, FilterPushdownPropagation, PushedDown,
};
use datafusion::physical_plan::metrics::{
    BaselineMetrics, ExecutionPlanMetricsSet, MetricBuilder, MetricsSet, RecordOutput,
};
use datafusion::physical_plan::sorts::sort_preserving_merge::SortPreservingMergeExec;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
};
use futures::Stream;
use tantivy::index::SegmentId;

use crate::index::fast_fields_helper::FFHelper;
use crate::index::fast_fields_helper::WhichFastField;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::explain::ExplainFormat;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::options::{SortByDirection, SortByField};
use crate::postgres::ParallelScanState;
use crate::query::SearchQueryInput;
use crate::scan::late_materialization::DeferredField;
use crate::scan::pre_filter::{collect_filters, try_dynamic_filter_pushdown, PreFilter};
use crate::scan::Scanner;

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
}

/// Recipe for a scan partition.
pub enum ScanRecipe {
    /// Eager scan: already have a specific set of segments to scan.
    Eager {
        segment_ids: Vec<SegmentId>,
        scanner_config: ScannerConfig,
    },
    /// Lazy scan: segments are claimed dynamically from parallel state.
    Lazy {
        parallel_state: Option<*mut ParallelScanState>,
        planner_estimated_rows: u64,
        scanner_config: ScannerConfig,
    },
    /// Prefetched scan: scanner is already created and has prefetched data.
    Prefetched { scanner: Scanner },
}

/// State for a scan partition.
/// Uses Arc<FFHelper> so the same FFHelper can be shared across multiple partitions.
pub struct ScanPartition {
    pub recipe: ScanRecipe,
    pub ffhelper: Arc<FFHelper>,
    pub visibility: Box<VisibilityChecker>,
    pub reader: SearchIndexReader,
}

pub type ScanState = ScanPartition;

/// A DataFusion `ExecutionPlan` for scanning `pg_search` index segments.
///
/// This plan represents a scan over one or more index segments. It exposes these
/// segments to DataFusion in two distinct ways:
///
/// 1.  **Lazy Execution (Single Partition)**: For standard queries that do not require
///     globally sorted outputs. The plan is initialized with exactly one partition containing
///     a lazily-evaluated `MultiSegmentSearchResults` stream. The underlying segments are
///     claimed dynamically (if running in parallel) or chained sequentially (if serial),
///     allowing segments to be dynamically load balanced across parallel workers as they process data.
/// 2.  **Eager/Throttled Execution (Multiple Partitions)**: For queries that require
///     globally sorted output (e.g. `ORDER BY` or sort-merge joins). The plan is initialized
///     with multiple pre-opened segments, each exposed as a distinct DataFusion partition.
///     DataFusion will automatically apply a `SortPreservingMergeExec` across these streams
///     to produce a single, globally sorted result.
pub struct PgSearchScanPlan {
    /// Segments to scan, indexed by partition.
    ///
    /// We use a Mutex to allow taking ownership of the scanners during `execute()`.
    /// We wrap the state in `UnsafeSendSync` to satisfy `ExecutionPlan`'s `Send` + `Sync`
    /// requirements. This is safe because we are running in a single-threaded
    /// environment (Postgres), which also means that the duration for which we
    /// hold this Mutex does not impact performance.
    states: Mutex<Vec<Option<UnsafeSendSync<ScanState>>>>,
    /// Estimated row counts for each partition, computed once at construction.
    /// Stored separately so `partition_statistics` is deterministic, even after
    /// the states have been consumed.
    partition_row_counts: Vec<u64>,
    properties: Arc<PlanProperties>,
    query_for_display: SearchQueryInput,
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
    /// `StrategyTag::None as u8` (= 0) means no dispatch happened, e.g. the
    /// InList didn't reach the FastField path (K ≤ 1024 routes to
    /// AutomatonWeight which doesn't write the sink).
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
    /// * `states` - The list of pre-opened segments (one per partition)
    /// * `schema` - Arrow schema for the output
    /// * `query_for_display` - Search query for EXPLAIN
    /// * `sort_order` - Optional sort order declaration for equivalence properties
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        states: Vec<ScanState>,
        schema: SchemaRef,
        query_for_display: SearchQueryInput,
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
        // Ensure we always return at least one partition to satisfy DataFusion distribution
        // requirements (e.g. HashJoinExec mode=CollectLeft requires SinglePartition).
        // If states is empty, execute() will return an EmptyStream for this single partition.
        let partition_count = states.len().max(1);
        let eq_properties = build_equivalence_properties(schema, sort_order);

        let properties = Arc::new(PlanProperties::new(
            eq_properties,
            Partitioning::UnknownPartitioning(partition_count),
            EmissionType::Incremental,
            Boundedness::Bounded,
        ));

        let partition_row_counts: Vec<u64> = if states.is_empty() {
            vec![0]
        } else {
            states
                .iter()
                .map(|s| match &s.recipe {
                    ScanRecipe::Eager { segment_ids, .. } => s
                        .reader
                        .estimated_docs_in_segments(segment_ids.iter().cloned()),
                    ScanRecipe::Lazy {
                        planner_estimated_rows,
                        ..
                    } => *planner_estimated_rows,
                    ScanRecipe::Prefetched { scanner } => scanner.estimated_rows(),
                })
                .collect()
        };

        let wrapped_states: Vec<Option<UnsafeSendSync<ScanState>>> = states
            .into_iter()
            .map(|s| Some(UnsafeSendSync(s)))
            .collect();

        Self {
            states: Mutex::new(wrapped_states),
            partition_row_counts,
            properties,
            query_for_display,
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

    /// Produce a plan identical to `dyn_plan` but with `dynamic_filters` emptied.
    ///
    /// Used by MPP: the join's dynamic-filter Arc is pushed to the probe-side
    /// scan by `FilterPushdown`, but MPP needs to apply it *after* the probe
    /// shuffle (so local bounds are not applied to rows destined for peer
    /// participants). We strip it from the scan and re-apply it via a
    /// `FilterExec` above the post-shuffle output.
    ///
    /// Transfers scan state out of the original plan — the original becomes
    /// a dead stub whose `execute` returns empty streams. Returns the plan
    /// unchanged when there are no dynamic filters to strip.
    ///
    /// # Caller contract
    ///
    /// This is a primitive: it strips unconditionally and does not validate
    /// that re-attaching the filter elsewhere is safe. It is correct only
    /// when the caller is rebuilding the plan such that the stripped
    /// filter will be reapplied above a `ShuffleExec` that crosses
    /// participant boundaries.
    ///
    /// In particular, do *not* call this on a scan whose enclosing
    /// `HashJoinExec` lives entirely on one participant (e.g., a join that
    /// is itself below a shuffle): the dynamic filter is fully populated
    /// locally there and dropping it loses a valuable optimization with no
    /// correctness benefit.
    #[allow(dead_code)] // walker (PR #4870) is the live caller
    pub fn strip_dynamic_filters_from_dyn(
        dyn_plan: Arc<dyn ExecutionPlan>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let scan = match dyn_plan.as_any().downcast_ref::<PgSearchScanPlan>() {
            Some(s) => s,
            None => {
                return Err(DataFusionError::Internal(
                    "strip_dynamic_filters_from_dyn called on a non-PgSearchScanPlan ExecutionPlan"
                        .into(),
                ));
            }
        };

        if scan.dynamic_filters.is_empty() {
            return Ok(dyn_plan);
        }

        let taken = {
            let mut guard = scan.states.lock().map_err(|e| {
                DataFusionError::Internal(format!("Failed to lock PgSearchScanPlan state: {e}"))
            })?;
            std::mem::take(&mut *guard)
        };
        let states: Vec<ScanState> = taken.into_iter().flatten().map(|u| u.0).collect();

        let schema = scan.properties.eq_properties.schema().clone();
        let new_plan = PgSearchScanPlan::new(
            states,
            schema,
            scan.query_for_display.clone(),
            scan.sort_order.as_ref(),
            scan.deferred_fields.clone(),
            scan.ffhelper.clone(),
            scan.indexrelid,
            scan.deferred_ctid_plan_position,
        );
        Ok(Arc::new(new_plan))
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
/// strategy name surfaced in `EXPLAIN ANALYZE` output. The match is
/// exhaustive on the enum so follow-ups A and B (filling in the
/// posting-direct and bitset-from-postings dispatch arms) don't need to
/// revisit this site — the compiler will flag any new variant.
fn strategy_name(strategy: tantivy::query::StrategyTag) -> &'static str {
    use tantivy::query::StrategyTag;
    match strategy {
        StrategyTag::None => "none",
        StrategyTag::Gallop => "gallop",
        StrategyTag::Linear => "linear",
        StrategyTag::Bitset => "bitset_from_postings",
        StrategyTag::Posting => "posting_direct",
        StrategyTag::Hash => "hash_probe",
    }
}

impl DisplayAs for PgSearchScanPlan {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "PgSearchScan: segments={}",
            self.states.lock().unwrap().len(),
        )?;
        if !self.dynamic_filters.is_empty() {
            write!(f, ", dynamic_filters={}", self.dynamic_filters.len())?;
        }
        if self.dynamic_filter_pushdown.load(Ordering::Relaxed) {
            // Render a single token. When the strategy framework engaged
            // (K > 1024 reached FastFieldTermSetWeight and select_strategy
            // wrote the sink), the value is the strategy name. Otherwise
            // (K ≤ 1024 routed via AutomatonWeight, which doesn't write the
            // sink) it falls back to "true".
            let tag = self.dynamic_filter_strategy.load(Ordering::Relaxed);
            let strategy = tantivy::query::StrategyTag::try_from(tag)
                .unwrap_or(tantivy::query::StrategyTag::None);
            if matches!(strategy, tantivy::query::StrategyTag::None) {
                write!(f, ", dynamic_filter_pushdown=true")?;
            } else {
                write!(f, ", dynamic_filter_pushdown={}", strategy_name(strategy))?;
            }
        }
        write!(f, ", query={}", self.query_for_display.explain_format())
    }
}

impl ExecutionPlan for PgSearchScanPlan {
    fn name(&self) -> &str {
        "PgSearchScan"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn properties(&self) -> &Arc<PlanProperties> {
        &self.properties
    }

    fn partition_statistics(&self, partition: Option<usize>) -> Result<Statistics> {
        let num_rows = match partition {
            Some(i) => {
                if i >= self.partition_row_counts.len() {
                    Precision::Absent
                } else {
                    Precision::Inexact(self.partition_row_counts[i] as usize)
                }
            }
            None => {
                let sum: u64 = self.partition_row_counts.iter().sum();
                Precision::Inexact(sum as usize)
            }
        };

        let column_statistics = self
            .properties
            .eq_properties
            .schema()
            .fields
            .iter()
            .map(|_| ColumnStatistics::default())
            .collect();

        Ok(Statistics {
            num_rows,
            total_byte_size: Precision::Absent,
            column_statistics,
        })
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
        let mut states = self.states.lock().map_err(|e| {
            DataFusionError::Internal(format!("Failed to lock PgSearchScanPlan state: {e}"))
        })?;

        if partition >= self.properties.output_partitioning().partition_count() {
            return Err(DataFusionError::Internal(format!(
                "Partition {} out of range (have {} partitions)",
                partition,
                self.properties.output_partitioning().partition_count()
            )));
        }

        // Handle the case where no segments were claimed (EmptyStream).
        if states.is_empty() {
            let schema = self.properties.eq_properties.schema().clone();
            return Ok(Box::pin(unsafe {
                UnsafeSendStream::new(futures::stream::empty(), schema)
            }));
        }

        let UnsafeSendSync(ScanState {
            recipe,
            ffhelper,
            mut visibility,
            mut reader,
        }) = states[partition].take().ok_or_else(|| {
            DataFusionError::Internal(format!("Partition {} has already been executed", partition))
        })?;

        let has_dynamic_filters = !self.dynamic_filters.is_empty();
        let rows_scanned = has_dynamic_filters
            .then(|| MetricBuilder::new(&self.metrics).counter("rows_scanned", partition));
        let rows_pruned = has_dynamic_filters
            .then(|| MetricBuilder::new(&self.metrics).counter("rows_pruned", partition));

        let baseline_metrics = BaselineMetrics::new(&self.metrics, partition);
        let schema = self.properties.eq_properties.schema().clone();
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

            let mut scanner = match recipe {
                ScanRecipe::Prefetched { scanner } => scanner,
                other_recipe => {
                    let (search_results, scanner_config) = match other_recipe {
                        ScanRecipe::Eager { segment_ids, scanner_config } => {
                            (reader.search_segments(segment_ids.into_iter()), scanner_config)
                        }
                        ScanRecipe::Lazy {
                            parallel_state,
                            planner_estimated_rows,
                            scanner_config,
                        } => {
                            let res = if let Some(ps) = parallel_state {
                                reader.search_lazy(ps, planner_estimated_rows)
                            } else {
                                reader.search()
                            };
                            (res, scanner_config)
                        }
                        ScanRecipe::Prefetched { .. } => unreachable!(),
                    };
                    Scanner::new(
                        search_results,
                        scanner_config.batch_size_hint,
                        scanner_config.which_fast_fields,
                        scanner_config.heap_relid,
                    )
                }
            };
            let df_batch_size = crate::gucs::dynamic_filter_batch_size();
            if df_batch_size > 0 {
                scanner.set_batch_size(df_batch_size as usize);
            }

            loop {
                let timer = baseline_metrics.elapsed_compute().timer();
                let pre_filters = build_filters(&dynamic_filters, &schema);
                let pre_filters_wrapper = if pre_filters.is_empty() {
                    None
                } else {
                    Some(crate::scan::pre_filter::PreFilters {
                        filters: &pre_filters,
                        schema: &schema,
                    })
                };

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
            if filter_result
                .filter
                .as_any()
                .downcast_ref::<DynamicFilterPhysicalExpr>()
                .is_some()
            {
                dynamic_filters.push(Arc::clone(&filter_result.filter));
                filters.push(PushedDown::Yes);
            } else {
                filters.push(filter_result.any());
            }
        }

        if !dynamic_filters.is_empty() {
            // Transfer state from the old plan to the new one.
            let states: Vec<_> = self
                .states
                .lock()
                .map_err(|e| {
                    DataFusionError::Internal(format!(
                        "Failed to lock PgSearchScanPlan state during filter pushdown: {e}"
                    ))
                })?
                .drain(..)
                .collect();

            let query_for_display = self.query_for_display.clone();

            let new_plan = Arc::new(PgSearchScanPlan {
                states: Mutex::new(states),
                partition_row_counts: self.partition_row_counts.clone(),
                properties: self.properties.clone(),
                query_for_display,
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
/// This is called on every `poll_next` (or loop iteration) so that tightening thresholds (e.g.
/// from Top K) are picked up immediately.
///
/// Only filter predicates that can be lowered to fast-field or term-ordinal
/// comparisons are retained. Anything else (unsupported types, non-comparison
/// operators) is silently dropped — the parent operator is still responsible
/// for enforcing the full predicate, so correctness is not affected.
fn build_filters(dynamic_filters: &[Arc<dyn PhysicalExpr>], schema: &SchemaRef) -> Vec<PreFilter> {
    let mut filters = Vec::new();
    for df in dynamic_filters {
        if let Some(dynamic) = df.as_any().downcast_ref::<DynamicFilterPhysicalExpr>() {
            if let Ok(current_expr) = dynamic.current() {
                collect_filters(&current_expr, schema, &mut filters);
            }
        } else {
            collect_filters(df, schema, &mut filters);
        }
    }
    filters
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

// ============================================================================
// Builder for creating sorted scans with SortPreservingMergeExec
// ============================================================================

/// Creates a sorted scan plan with `SortPreservingMergeExec` to merge sorted segments.
///
/// When there is only one segment, returns the `PgSearchScanPlan` directly without
/// the merge layer (no merging needed for a single partition).
///
/// Returns `None` if the sort field is not present in the schema (e.g., the sort column
/// was not projected in the scan). In this case, the caller should fall back to an
/// unsorted scan to avoid producing incorrectly ordered results.
pub fn create_sorted_scan(
    states: Vec<ScanState>,
    schema: SchemaRef,
    query_for_display: SearchQueryInput,
    sort_order: &SortByField,
    indexrelid: u32,
) -> Result<Arc<dyn ExecutionPlan>> {
    // Validate that the sort field exists in the schema
    let field_name = sort_order.field_name.as_ref();
    let col_idx = match schema.column_with_name(field_name) {
        Some((idx, _)) => idx,
        None => {
            // Sort field is not in the schema - cannot create sorted merge.
            return Err(DataFusionError::Internal(format!(
                "Sort field '{}' not found in scan schema",
                field_name
            )));
        }
    };

    let segment_count = states.len();
    let segment_scan = Arc::new(PgSearchScanPlan::new(
        states,
        schema.clone(),
        query_for_display,
        Some(sort_order),
        Vec::new(),
        None,
        indexrelid,
        None,
    ));

    // For a single segment, no merging is needed
    if segment_count == 1 {
        return Ok(segment_scan);
    }

    let sort_options = SortOptions {
        descending: matches!(sort_order.direction, SortByDirection::Desc),
        nulls_first: matches!(sort_order.direction, SortByDirection::Asc),
    };

    let sort_expr = PhysicalSortExpr {
        expr: Arc::new(Column::new(field_name, col_idx)),
        options: sort_options,
    };

    let ordering =
        LexOrdering::new(vec![sort_expr]).expect("sort expression should create valid ordering");

    // Wrap with SortPreservingMergeExec to merge sorted partitions
    Ok(Arc::new(SortPreservingMergeExec::new(
        ordering,
        segment_scan,
    )))
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
            vec![],
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
            vec![],
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
