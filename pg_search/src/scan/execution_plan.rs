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
//! This module provides the `PgSearchScanPlan`, which handles scanning of `pg_search`
//! index segments. It supports both single-partition (serial) and multi-partition
//! (parallel or sorted) scans.
//!
//! For sorted scans, `create_sorted_scan` can be used to wrap the plan in a
//! `SortPreservingMergeExec` to merge sorted outputs from multiple segments.

use std::any::Any;
use std::pin::Pin;
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
    Count, ExecutionPlanMetricsSet, MetricBuilder, MetricsSet,
};
use datafusion::physical_plan::sorts::sort_preserving_merge::SortPreservingMergeExec;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
};
use futures::Stream;

use crate::index::fast_fields_helper::FFHelper;
use crate::postgres::customscan::explain::ExplainFormat;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::options::{SortByDirection, SortByField};
use crate::query::SearchQueryInput;
use crate::scan::pre_filter::{collect_filters, PreFilter};
use crate::scan::Scanner;

/// A wrapper that implements Send + Sync unconditionally.
/// UNSAFE: Only use this when you guarantee single-threaded access or manual synchronization.
/// This is safe in pg_search because Postgres extensions run single-threaded.
pub(crate) struct UnsafeSendSync<T>(pub T);

unsafe impl<T> Send for UnsafeSendSync<T> {}
unsafe impl<T> Sync for UnsafeSendSync<T> {}

/// State for a scan partition.
/// Uses Arc<FFHelper> so the same FFHelper can be shared across multiple partitions.
pub type ScanState = (Scanner, Arc<FFHelper>, Box<VisibilityChecker>);

/// A DataFusion `ExecutionPlan` for scanning `pg_search` index segments.
///
/// This plan represents a scan over one or more segments, where each segment
/// corresponds to a DataFusion partition. It handles both:
///
/// 1.  **Serial Scans**: The plan is initialized with a single partition, or segments are
///     processed lazily.
/// 2.  **Parallel/Sorted Scans**: The plan is initialized with multiple pre-opened segments,
///     each exposed as a distinct partition.
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
    properties: PlanProperties,
    query_for_display: SearchQueryInput,
    /// Dynamic filters pushed down from parent operators (e.g. TopK threshold
    /// from SortExec, join-key bounds from HashJoinExec). Each batch produced
    /// by the scanner is filtered against all of these expressions so that rows
    /// which cannot contribute to the final result are pruned early.
    dynamic_filters: Vec<Arc<dyn PhysicalExpr>>,
    /// Metrics for EXPLAIN ANALYZE.
    metrics: ExecutionPlanMetricsSet,
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
    pub fn new(
        states: Vec<ScanState>,
        schema: SchemaRef,
        query_for_display: SearchQueryInput,
        sort_order: Option<&SortByField>,
    ) -> Self {
        // Ensure we always return at least one partition to satisfy DataFusion distribution
        // requirements (e.g. HashJoinExec mode=CollectLeft requires SinglePartition).
        // If states is empty, execute() will return an EmptyStream for this single partition.
        let partition_count = states.len().max(1);
        let eq_properties = build_equivalence_properties(schema, sort_order);

        let properties = PlanProperties::new(
            eq_properties,
            Partitioning::UnknownPartitioning(partition_count),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );

        let partition_row_counts: Vec<u64> = if states.is_empty() {
            vec![0]
        } else {
            states
                .iter()
                .map(|(scanner, _, _)| scanner.estimated_rows())
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
        }
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

    fn properties(&self) -> &PlanProperties {
        &self.properties
    }

    fn statistics(&self) -> Result<Statistics> {
        self.partition_statistics(None)
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
            return Ok(Box::pin(EmptyStream::new(
                self.properties.eq_properties.schema().clone(),
            )));
        }

        let UnsafeSendSync((scanner, ffhelper, visibility)) =
            states[partition].take().ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "Partition {} has already been executed",
                    partition
                ))
            })?;

        let has_dynamic_filters = !self.dynamic_filters.is_empty();
        let rows_scanned = has_dynamic_filters
            .then(|| MetricBuilder::new(&self.metrics).counter("rows_scanned", partition));
        let rows_pruned = has_dynamic_filters
            .then(|| MetricBuilder::new(&self.metrics).counter("rows_pruned", partition));

        // SAFETY: pg_search operates in a single-threaded Tokio executor within Postgres,
        // so it is safe to wrap !Send types for use within DataFusion.
        let stream = unsafe {
            UnsafeSendStream::new(ScanStream {
                scanner,
                ffhelper,
                visibility,
                schema: self.properties.eq_properties.schema().clone(),
                dynamic_filters: self.dynamic_filters.clone(),
                rows_scanned,
                rows_pruned,
            })
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
        // Only handle dynamic filters in the Post phase (TopK pushdown happens here).
        if !matches!(phase, FilterPushdownPhase::Post) {
            return Ok(FilterPushdownPropagation::if_all(child_pushdown_result));
        }

        // Collect all DynamicFilterPhysicalExpr instances from the parent filters.
        // Multiple sources may push dynamic filters (e.g. TopK from SortExec,
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
            let mut states: Vec<_> = self
                .states
                .lock()
                .map_err(|e| {
                    DataFusionError::Internal(format!(
                        "Failed to lock PgSearchScanPlan state during filter pushdown: {e}"
                    ))
                })?
                .drain(..)
                .collect();

            // When the GUC is set, cap the scanner batch size so that TopK
            // can tighten its threshold between batches.
            let df_batch_size = crate::gucs::dynamic_filter_batch_size();
            if df_batch_size > 0 {
                for UnsafeSendSync(ref mut inner) in states.iter_mut().flatten() {
                    inner.0.set_batch_size(df_batch_size as usize);
                }
            }

            let new_plan = Arc::new(PgSearchScanPlan {
                states: Mutex::new(states),
                partition_row_counts: self.partition_row_counts.clone(),
                properties: self.properties.clone(),
                query_for_display: self.query_for_display.clone(),
                dynamic_filters,
                metrics: self.metrics.clone(),
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

struct ScanStream {
    scanner: Scanner,
    ffhelper: Arc<FFHelper>,
    visibility: Box<VisibilityChecker>,
    schema: SchemaRef,
    dynamic_filters: Vec<Arc<dyn PhysicalExpr>>,
    /// Metrics counters for EXPLAIN ANALYZE (only set when dynamic filters are present).
    rows_scanned: Option<Count>,
    rows_pruned: Option<Count>,
}

impl ScanStream {
    /// Evaluate the current dynamic filter expressions and convert them into
    /// [`PreFilter`]s that the `Scanner` can apply before column materialization.
    ///
    /// This is called on every `poll_next` so that tightening thresholds (e.g.
    /// from TopK) are picked up immediately.
    ///
    /// Only filter predicates that can be lowered to fast-field or term-ordinal
    /// comparisons are retained. Anything else (unsupported types, non-comparison
    /// operators) is silently dropped — the parent operator is still responsible
    /// for enforcing the full predicate, so correctness is not affected.
    fn build_filters(&self) -> Vec<PreFilter> {
        let mut filters = Vec::new();
        for df in &self.dynamic_filters {
            if let Some(dynamic) = df.as_any().downcast_ref::<DynamicFilterPhysicalExpr>() {
                if let Ok(current_expr) = dynamic.current() {
                    collect_filters(&current_expr, &self.schema, &mut filters);
                }
            }
        }
        filters
    }
}

impl Stream for ScanStream {
    type Item = Result<RecordBatch>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let pre_filters = this.build_filters();
        let pre_filters_wrapper = if pre_filters.is_empty() {
            None
        } else {
            Some(crate::scan::pre_filter::PreFilters {
                filters: &pre_filters,
                schema: &this.schema,
            })
        };

        match this.scanner.next(
            &this.ffhelper,
            &mut this.visibility,
            pre_filters_wrapper.as_ref(),
        ) {
            Some(batch) => Poll::Ready(Some(Ok(batch.to_record_batch(&this.schema)))),
            None => {
                // Flush pre-materialization filter stats from Scanner.
                if let Some(ref counter) = this.rows_scanned {
                    counter.add(this.scanner.pre_filter_rows_scanned);
                }
                if let Some(ref counter) = this.rows_pruned {
                    counter.add(this.scanner.pre_filter_rows_pruned);
                }
                Poll::Ready(None)
            }
        }
    }
}

impl RecordBatchStream for ScanStream {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

/// A wrapper that unsafely implements Send for a Stream.
///
/// This is used to wrap `ScanStream` which is !Send because it contains Tantivy and Postgres
/// state that is not Send. This is safe because pg_search operates in a single-threaded
/// Tokio executor within Postgres, and these objects will never cross thread boundaries.
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

/// A stream that produces no batches.
struct EmptyStream {
    schema: SchemaRef,
}

impl EmptyStream {
    fn new(schema: SchemaRef) -> Self {
        Self { schema }
    }
}

impl Stream for EmptyStream {
    type Item = Result<RecordBatch>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(None)
    }
}

impl RecordBatchStream for EmptyStream {
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
