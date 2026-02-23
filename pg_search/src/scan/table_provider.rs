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

use arrow_schema::{Field, Schema, SchemaRef};
use async_trait::async_trait;
use datafusion::catalog::{Session, TableProvider};
use datafusion::common::{DataFusionError, Result, Statistics};
use datafusion::logical_expr::{Expr, TableProviderFilterPushDown, TableType};
use datafusion::physical_plan::ExecutionPlan;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};
use tantivy::index::SegmentId;

use crate::index::fast_fields_helper::{FFHelper, WhichFastField};
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
    late_mat_enabled: bool,
    deferred_fields: Vec<DeferredField>,
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
            late_mat_enabled: false,
            deferred_fields: Vec::new(),
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
        required_early_columns: &std::collections::HashSet<String>,
    ) {
        if self.late_mat_enabled {
            return;
        }
        let mut deferred = Vec::new();
        for (ff_index, wff) in self.fields.iter().enumerate() {
            if let WhichFastField::Named(name, _) = wff {
                let is_string_or_bytes = matches!(
                    wff.arrow_data_type(),
                    arrow_schema::DataType::Utf8View
                        | arrow_schema::DataType::BinaryView
                        | arrow_schema::DataType::LargeUtf8
                        | arrow_schema::DataType::LargeBinary
                );
                if is_string_or_bytes && !required_early_columns.contains(name.as_str()) {
                    deferred.push(DeferredField {
                        field_name: name.clone(),
                        is_bytes: matches!(
                            wff.arrow_data_type(),
                            arrow_schema::DataType::BinaryView
                                | arrow_schema::DataType::LargeBinary
                        ),
                        ff_index,
                    });
                }
            }
        }
        if !deferred.is_empty() {
            self.deferred_fields = deferred;
            self.late_mat_enabled = true;
        }
    }

    fn get_schema(&self) -> SchemaRef {
        self.schema
            .get_or_init(|| build_schema(&self.fields, &self.deferred_fields))
            .clone()
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

    /// Creates a single scan partition for a segment.
    fn create_scan_partition(
        &self,
        reader: &SearchIndexReader,
        segment_id: SegmentId,
        ffhelper: &Arc<FFHelper>,
        visibility: &VisibilityChecker,
        heap_relid: pg_sys::Oid,
    ) -> ScanState {
        let search_results = reader.search_segments(std::iter::once(segment_id));
        let deferred_indices = self.deferred_fields.iter().map(|d| d.ff_index).collect();
        let scanner = Scanner::new(
            search_results,
            None,
            self.fields.clone(),
            heap_relid.into(),
            deferred_indices,
        );
        (
            scanner,
            Arc::clone(ffhelper),
            Box::new(visibility.clone()) as Box<VisibilityChecker>,
        )
    }

    /// Creates a PgSearchScanPlan from a list of segments.
    fn create_scan(
        &self,
        segments: Vec<ScanState>,
        query_for_display: SearchQueryInput,
        sort_order: Option<&crate::postgres::options::SortByField>,
        ffhelper: Arc<FFHelper>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // Only declare sort order if the field exists in the schema
        let actual_sort_order = sort_order.and_then(|so| {
            let field_name = so.field_name.as_ref();
            if self.get_schema().column_with_name(field_name).is_some() {
                Some(so)
            } else {
                None
            }
        });
        let force_late = std::env::var("PARADEDB_FORCE_LATE_MAT").is_ok();
        let is_late_mat = self.late_mat_enabled || force_late;

        let (deferred, maybe_ff) = if is_late_mat {
            (self.deferred_fields.clone(), Some(Arc::clone(&ffhelper)))
        } else {
            (Vec::new(), None)
        };

        Ok(Arc::new(PgSearchScanPlan::new(
            segments,
            self.get_schema(),
            query_for_display,
            actual_sort_order,
            deferred,
            maybe_ff,
        )))
    }

    /// Creates a PgSearchScanPlan for throttled parallel scans.
    ///
    /// This method uses a throttled loop to claim segments from the shared `parallel_state`.
    /// It ensures fair work distribution by prefetching data from each claimed segment
    /// before attempting to claim the next one.
    #[allow(clippy::too_many_arguments)]
    fn create_throttled_scan(
        &self,
        parallel_state: *mut ParallelScanState,
        reader: &SearchIndexReader,
        ffhelper: FFHelper,
        visibility: VisibilityChecker,
        heap_relid: pg_sys::Oid,
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

            let mut partition =
                self.create_scan_partition(reader, segment_id, &ffhelper, &visibility, heap_relid);
            // Do real work between checkouts to avoid one worker claiming all segments.
            partition.0.prefetch_next(
                &ffhelper,
                &mut partition.2,
                &[],
                &std::sync::Arc::new(arrow_schema::Schema::empty()),
            );

            segments.push(partition);
        }

        self.create_scan(segments, query_for_display, sort_order, ffhelper)
    }

    /// Creates a PgSearchScanPlan for eager scans (or fully replicated parallel joins).
    ///
    /// This method opens all segments upfront as separate partitions, allowing DataFusion
    /// to treat them as distinct streams (sorted or not).
    fn create_eager_scan(
        &self,
        reader: &SearchIndexReader,
        ffhelper: FFHelper,
        visibility: VisibilityChecker,
        heap_relid: pg_sys::Oid,
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
                    &ffhelper,
                    &visibility,
                    heap_relid,
                )
            })
            .collect();

        self.create_scan(segments, query_for_display, sort_order, ffhelper)
    }
}

fn build_schema(fields: &[WhichFastField], deferred: &[DeferredField]) -> SchemaRef {
    let deferred_names: std::collections::HashSet<&str> =
        deferred.iter().map(|d| d.field_name.as_str()).collect();

    let arrow_fields: Vec<Field> = fields
        .iter()
        .map(|f| {
            let name = f.name();
            if deferred_names.contains(name.as_str()) {
                Field::new(name, arrow_schema::DataType::UInt64, true) // packed DocAddress
            } else {
                Field::new(name, f.arrow_data_type(), true)
            }
        })
        .collect();

    // NO __pdb_segment_ord appended here
    Arc::new(Schema::new(arrow_fields))
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
        _projection: Option<&Vec<usize>>,
        filters: &[Expr],
        _limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // TODO: We should support limit pushdown here to allow providing a batch size hint
        // to the Scanner.

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

        let ffhelper = FFHelper::with_fields(&reader, &self.fields);
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let visibility = VisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);
        let sort_order = self.scan_info.sort_order.as_ref();

        if let Some(parallel_state) = self.parallel_state {
            // In joinscan, the "partitioning source" (the first source) uses the throttled checkout
            // strategy to dynamically claim segments.
            self.create_throttled_scan(
                parallel_state,
                &reader,
                ffhelper,
                visibility,
                heap_relid,
                query,
                sort_order,
            )
        } else if let Some(sort_order) = sort_order {
            // Serial sorted scan (or replicated source with sorting)
            self.create_eager_scan(
                &reader,
                ffhelper,
                visibility,
                heap_relid,
                query,
                Some(sort_order),
            )
        } else {
            // For serial/replicated scans without sorting, we use a multi-partition scan
            // (1 partition per segment).
            //
            // Exposing segments as partitions to DataFusion allows parallel processing within
            // the DataFusion executor if we configured target_partitions > 1 (though
            // currently joinscan uses 1). It also unifies the code path.
            self.create_eager_scan(&reader, ffhelper, visibility, heap_relid, query, None)
        }
    }
}
