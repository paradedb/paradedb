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

use std::any::Any;
use std::fmt::Debug;
use std::sync::{Arc, OnceLock};

use arrow_schema::SchemaRef;
use async_trait::async_trait;
use datafusion::catalog::{Session, TableProvider};
use datafusion::common::{DataFusionError, Result, Statistics};
use datafusion::logical_expr::{Expr, TableProviderFilterPushDown, TableType};
use datafusion::physical_plan::ExecutionPlan;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};
use tantivy::index::SegmentId;

use crate::index::fast_fields_helper::{build_arrow_schema, FFHelper, WhichFastField};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::parallel::{checkout_segment, list_segment_ids};
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::ParallelScanState;
use crate::query::SearchQueryInput;
use crate::scan::execution_plan::{PgSearchScanPlan, ScanState};
use crate::scan::filter_pushdown::{combine_with_and, FilterAnalyzer};
use crate::scan::info::{RowEstimate, ScanInfo};
use crate::scan::tantivy_lookup_exec::DeferredField;
use crate::scan::Scanner;

#[derive(Debug, Serialize, Deserialize)]
pub struct PgSearchTableProvider {
    scan_info: ScanInfo,
    fields: Vec<WhichFastField>,
    #[serde(skip)]
    schema: OnceLock<SchemaRef>,
    is_parallel: bool,
    /// Parallel state is skipped during serialization because it's a raw pointer
    /// to shared memory that is only valid in the current process. It is
    /// re-injected by the `PgSearchExtensionCodec` during deserialization
    /// if `is_parallel` is true.
    #[serde(skip)]
    parallel_state: Option<*mut ParallelScanState>,
}

unsafe impl Send for PgSearchTableProvider {}
unsafe impl Sync for PgSearchTableProvider {}

impl PgSearchTableProvider {
    pub fn new(
        scan_info: ScanInfo,
        fields: Vec<WhichFastField>,
        parallel_state: Option<*mut ParallelScanState>,
        is_parallel: bool,
    ) -> Self {
        Self {
            scan_info,
            fields,
            schema: OnceLock::new(),
            is_parallel,
            parallel_state,
        }
    }

    pub(crate) fn is_parallel(&self) -> bool {
        self.is_parallel
    }

    pub(crate) fn set_parallel_state(&mut self, parallel_state: Option<*mut ParallelScanState>) {
        assert!(self.is_parallel);
        self.parallel_state = parallel_state;
    }
    pub fn try_enable_late_materialization(
        &mut self,
        required_early_columns: &crate::api::HashSet<String>,
    ) {
        for wff in self.fields.iter_mut() {
            if let WhichFastField::Named(name, field_type) = wff {
                let is_string_or_bytes = matches!(
                    field_type.arrow_data_type(),
                    arrow_schema::DataType::Utf8View
                        | arrow_schema::DataType::BinaryView
                        | arrow_schema::DataType::LargeUtf8
                        | arrow_schema::DataType::LargeBinary
                );
                if is_string_or_bytes && !required_early_columns.contains(name.as_str()) {
                    let is_bytes = matches!(
                        field_type.arrow_data_type(),
                        arrow_schema::DataType::BinaryView | arrow_schema::DataType::LargeBinary
                    );
                    *wff = WhichFastField::Deferred(name.clone(), *field_type, is_bytes);
                }
            }
        }
    }

    fn get_schema(&self) -> SchemaRef {
        self.schema
            .get_or_init(|| crate::index::fast_fields_helper::build_arrow_schema(&self.fields))
            .clone()
    }

    fn projected_fields_and_schema(
        &self,
        projection: Option<&Vec<usize>>,
    ) -> Result<(Vec<WhichFastField>, SchemaRef)> {
        match projection {
            None => Ok((self.fields.clone(), self.get_schema())),
            Some(indices) => {
                let mut fields = Vec::with_capacity(indices.len());
                for &idx in indices {
                    let field = self.fields.get(idx).ok_or_else(|| {
                        DataFusionError::Execution(format!(
                            "Projection index {idx} out of bounds for {} fields",
                            self.fields.len()
                        ))
                    })?;
                    fields.push(field.clone());
                }
                let schema = build_arrow_schema(&fields);
                Ok((fields, schema))
            }
        }
    }

    fn analyzer(&self) -> FilterAnalyzer<'_> {
        // Note: baserestrictinfo predicates (in scan_info.query) are single-table predicates
        // applied at the base relation level. The filters we analyze here are join-level
        // predicates that couldn't be applied earlier - they are different predicates,
        // not duplicates.
        FilterAnalyzer::new(&self.fields, self.scan_info.indexrelid)
    }

    /// Combine the base query with any pushed-down filters.
    ///
    /// The base query comes from scan_info.query (single-table predicates from baserestrictinfo).
    /// The filters come from DataFusion's supports_filters_pushdown mechanism - these are
    /// join-level predicates that couldn't be applied at the base relation level, including:
    /// - SearchPredicateUDF: @@@ predicates from cross-table conditions
    /// - Regular SQL predicates: equality, range, IN list on indexed columns
    fn combine_query_with_filters(
        &self,
        base_query: SearchQueryInput,
        filters: &[Expr],
    ) -> SearchQueryInput {
        if filters.is_empty() {
            return base_query;
        }

        let analyzer = self.analyzer();
        let filter_queries: Vec<SearchQueryInput> =
            filters.iter().map(|f| analyzer.analyze(f)).collect();

        if filter_queries.is_empty() {
            return base_query;
        }

        let mut all_queries = vec![base_query];
        all_queries.extend(filter_queries);
        combine_with_and(all_queries).unwrap_or(SearchQueryInput::All)
    }

    /// Creates a single scan partition representing exactly one segment.
    ///
    /// This function directly opens the index segment, calculates the estimated number of
    /// documents contained within it, and initializes a `Scanner` bounded to this specific segment.
    #[allow(clippy::too_many_arguments)]
    fn create_scan_partition(
        &self,
        reader: &SearchIndexReader,
        segment_id: SegmentId,
        fields: &[WhichFastField],
        ffhelper: &Arc<FFHelper>,
        visibility: &VisibilityChecker,
        heap_relid: pg_sys::Oid,
    ) -> ScanState {
        let search_results = reader.search_segments(std::iter::once(segment_id));

        let scanner = Scanner::new(search_results, None, fields.to_vec(), heap_relid.into());
        (
            scanner,
            ffhelper.clone(),
            Box::new(visibility.clone()) as Box<VisibilityChecker>,
        )
    }
    pub fn deferred_fields(&self) -> Vec<DeferredField> {
        let mut deferred = Vec::new();
        for (ff_index, wff) in self.fields.iter().enumerate() {
            if let WhichFastField::Deferred(name, _, is_bytes) = wff {
                deferred.push(DeferredField {
                    field_name: name.clone(),
                    is_bytes: *is_bytes,
                    ff_index,
                });
            }
        }
        deferred
    }
    /// Creates a PgSearchScanPlan from a list of segments.
    fn create_scan(
        &self,
        segments: Vec<ScanState>,
        schema: SchemaRef,
        query_for_display: SearchQueryInput,
        sort_order: Option<&crate::postgres::options::SortByField>,
        ffhelper: Arc<FFHelper>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // Only declare sort order if the field exists in the schema
        let actual_sort_order = sort_order.and_then(|so| {
            let field_name = so.field_name.as_ref();
            if schema.column_with_name(field_name).is_some() {
                Some(so)
            } else {
                None
            }
        });

        let deferred = self.deferred_fields();

        let maybe_ff = if deferred.is_empty() {
            None
        } else {
            Some(ffhelper.clone())
        };

        Ok(Arc::new(PgSearchScanPlan::new(
            segments,
            schema,
            query_for_display,
            actual_sort_order,
            deferred,
            maybe_ff,
        )))
    }

    /// Creates a multi-partition `PgSearchScanPlan` for throttled parallel scans.
    ///
    /// This method is specifically designed for parallel execution paths that require
    /// sorted outputs. Because the results need to be globally sorted, DataFusion needs
    /// to see all segments concurrently as distinct partitions so it can apply a
    /// `SortPreservingMergeExec` across them.
    ///
    /// To ensure fair work distribution among multiple Postgres parallel workers, this
    /// method uses a throttled loop to claim segments from the shared `parallel_state`.
    /// When a worker claims a segment, it explicitly prefetches a single batch from that
    /// segment before attempting to claim the next one. This small amount of work prevents
    /// a single fast-starting worker from immediately checking out all available segments
    /// and starving the other workers.
    #[allow(clippy::too_many_arguments)]
    fn create_throttled_scan(
        &self,
        parallel_state: *mut ParallelScanState,
        reader: &SearchIndexReader,
        fields: &[WhichFastField],
        ffhelper: FFHelper,
        visibility: VisibilityChecker,
        heap_relid: pg_sys::Oid,
        schema: SchemaRef,
        query_for_display: SearchQueryInput,
        sort_order: Option<&crate::postgres::options::SortByField>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let mut segments = Vec::new();
        let ffhelper = Arc::new(ffhelper);

        loop {
            pgrx::check_for_interrupts!();

            let segment_id = unsafe { checkout_segment(parallel_state) };
            let Some(segment_id) = segment_id else {
                break;
            };

            let mut partition = self.create_scan_partition(
                reader,
                segment_id,
                fields,
                &ffhelper,
                &visibility,
                heap_relid,
            );
            // Do real work between checkouts to avoid one worker claiming all segments.
            partition.0.prefetch_next(&ffhelper, &mut partition.2, None);

            segments.push(partition);
        }

        self.create_scan(segments, schema, query_for_display, sort_order, ffhelper)
    }

    /// Creates a multi-partition `PgSearchScanPlan` for eager scans.
    ///
    /// This method is used when we need to expose every segment as its own distinct
    /// DataFusion partition, but we are *not* dynamically claiming segments from a shared
    /// parallel pool (e.g., serial execution with an `ORDER BY`, or a fully replicated
    /// parallel source where every worker must scan the entire table).
    ///
    /// It opens all segments sequentially upfront and assigns each to a distinct `Scanner`
    /// partition. If a sort order is specified, this allows DataFusion to apply a
    /// `SortPreservingMergeExec` across the partitions. Since there is no competition
    /// with other workers for these segments, no prefetching or throttling is necessary.
    #[allow(clippy::too_many_arguments)]
    fn create_eager_scan(
        &self,
        reader: &SearchIndexReader,
        fields: &[WhichFastField],
        ffhelper: FFHelper,
        visibility: VisibilityChecker,
        heap_relid: pg_sys::Oid,
        schema: SchemaRef,
        query_for_display: SearchQueryInput,
        sort_order: Option<&crate::postgres::options::SortByField>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let ffhelper = Arc::new(ffhelper);
        let segments: Vec<ScanState> = reader
            .segment_readers()
            .iter()
            .map(|r| {
                self.create_scan_partition(
                    reader,
                    r.segment_id(),
                    fields,
                    &ffhelper,
                    &visibility,
                    heap_relid,
                )
            })
            .collect();

        self.create_scan(segments, schema, query_for_display, sort_order, ffhelper)
    }

    /// Creates a single-partition `PgSearchScanPlan` for lazy scans.
    ///
    /// This method is used when the output does *not* need to be globally sorted, meaning
    /// DataFusion doesn't need to perform a `SortPreservingMergeExec` across multiple streams.
    /// Thus, we can optimize execution by exposing exactly 1 partition natively and chaining
    /// segments end-to-end.
    ///
    /// In the parallel case, segments are dynamically checked out from `parallel_state` only
    /// when the previous segment is exhausted. This inherently yields execution time to other
    /// parallel workers at segment boundaries without requiring explicit prefetching. In the
    /// serial case, it simply chains all segments.
    ///
    /// We require `planner_estimated_rows` to be passed in because a lazy scan cannot know
    /// exactly which segments it will end up scanning at planning time to sum their individual
    /// sizes. Instead, it relies on the estimated partition size computed during query planning.
    #[allow(clippy::too_many_arguments)]
    fn create_lazy_scan(
        &self,
        reader: &SearchIndexReader,
        fields: &[WhichFastField],
        ffhelper: FFHelper,
        visibility: VisibilityChecker,
        heap_relid: pg_sys::Oid,
        schema: SchemaRef,
        query_for_display: SearchQueryInput,
        planner_estimated_rows: u64,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let search_results = if let Some(parallel_state) = self.parallel_state {
            reader.search_lazy(parallel_state, planner_estimated_rows)
        } else {
            // Unsorted Serial
            reader.search()
        };

        let scanner = Scanner::new(
            search_results,
            None, // batch size hint
            fields.to_vec(),
            heap_relid.into(),
        );

        let ffhelper_arc = Arc::new(ffhelper);
        let state = (
            scanner,
            ffhelper_arc.clone(),
            Box::new(visibility) as Box<VisibilityChecker>,
        );

        self.create_scan(
            vec![state],
            schema,
            query_for_display,
            None, // no sort order
            ffhelper_arc,
        )
    }
}

#[async_trait]
impl TableProvider for PgSearchTableProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.get_schema()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    fn statistics(&self) -> Option<Statistics> {
        use datafusion::common::stats::{ColumnStatistics, Precision};

        let num_rows = match self.scan_info.estimate {
            RowEstimate::Known(n) => Precision::Inexact(n as usize),
            RowEstimate::Unknown => Precision::Absent,
        };

        let column_statistics = self
            .get_schema()
            .fields
            .iter()
            .map(|_| ColumnStatistics::default())
            .collect();

        Some(Statistics {
            num_rows,
            total_byte_size: Precision::Absent,
            column_statistics,
        })
    }

    fn supports_filters_pushdown(
        &self,
        filters: &[&Expr],
    ) -> Result<Vec<TableProviderFilterPushDown>> {
        let analyzer = self.analyzer();
        let results = filters
            .iter()
            .map(|filter| {
                if analyzer.supports(filter) {
                    TableProviderFilterPushDown::Exact
                } else {
                    TableProviderFilterPushDown::Unsupported
                }
            })
            .collect();
        Ok(results)
    }

    async fn scan(
        &self,
        _state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        _limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // TODO: We should support limit pushdown here to allow providing a batch size hint
        // to the Scanner.

        let (projected_fields, projected_schema) = self.projected_fields_and_schema(projection)?;
        let heap_relid = self.scan_info.heaprelid;
        let index_relid = self.scan_info.indexrelid;

        let heap_rel = PgSearchRelation::open(heap_relid);
        let index_rel = PgSearchRelation::open(index_relid);

        // Start with the base query from scan_info
        let base_query = self.scan_info.query.clone();

        // Convert pushed-down filters to SearchQueryInput and combine with base query
        let query = self.combine_query_with_filters(base_query, filters);

        // Determine MVCC strategy based on whether we are running in parallel mode
        let mvcc_style = if let Some(parallel_state) = self.parallel_state {
            unsafe {
                if pg_sys::ParallelWorkerNumber == -1 {
                    // Leader only sees snapshot-visible segments
                    MvccSatisfies::Snapshot
                } else {
                    // Workers see all segments listed in shared state
                    MvccSatisfies::ParallelWorker(list_segment_ids(parallel_state))
                }
            }
        } else {
            MvccSatisfies::Snapshot
        };

        let reader = SearchIndexReader::open_with_context(
            &index_rel,
            query.clone(),
            self.scan_info.score_needed,
            mvcc_style,
            None,
            None,
        )
        .map_err(|e| DataFusionError::Internal(format!("Failed to open reader: {e}")))?;

        let ffhelper = FFHelper::with_fields(&reader, &projected_fields);
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let visibility = VisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);
        let sort_order = self.scan_info.sort_order.as_ref();

        if let Some(sort_order) = sort_order {
            if let Some(parallel_state) = self.parallel_state {
                // In joinscan, the "partitioning source" (the first source) uses the throttled checkout
                // strategy to dynamically claim segments.
                self.create_throttled_scan(
                    parallel_state,
                    &reader,
                    &projected_fields,
                    ffhelper,
                    visibility,
                    heap_relid,
                    projected_schema,
                    query,
                    Some(sort_order),
                )
            } else {
                // Serial sorted scan (or replicated source with sorting)
                self.create_eager_scan(
                    &reader,
                    &projected_fields,
                    ffhelper,
                    visibility,
                    heap_relid,
                    projected_schema,
                    query,
                    Some(sort_order),
                )
            }
        } else {
            // When parallel execution is planned, we expect `estimated_rows_per_worker` to be explicitly computed.
            // For serial execution, it will also be computed (divided by 1).
            let total_estimated_rows = self.scan_info.estimated_rows_per_worker.unwrap_or_else(|| {
                panic!("PgSearchTableProvider requires `estimated_rows_per_worker` to be explicitly set during planning");
            });

            self.create_lazy_scan(
                &reader,
                &projected_fields,
                ffhelper,
                visibility,
                heap_relid,
                projected_schema,
                query,
                total_estimated_rows,
            )
        }
    }
}
