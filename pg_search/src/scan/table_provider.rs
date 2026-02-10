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

use crate::index::fast_fields_helper::{FFHelper, WhichFastField};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::parallel::list_segment_ids;
use crate::postgres::heap::VisibilityChecker as HeapVisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::ParallelScanState;
use crate::query::SearchQueryInput;
use crate::scan::execution_plan::{ParallelSegmentPlan, SegmentPlan};
use crate::scan::filter_pushdown::{combine_with_and, FilterAnalyzer};
use crate::scan::info::ScanInfo;
use crate::scan::{Scanner, VisibilityChecker};

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

    fn get_schema(&self) -> SchemaRef {
        self.schema
            .get_or_init(|| build_schema(&self.fields))
            .clone()
    }

    fn analyzer(&self) -> FilterAnalyzer<'_> {
        // Note: baserestrictinfo predicates (in scan_info.query) are single-table predicates
        // applied at the base relation level. The filters we analyze here are join-level
        // predicates that couldn't be applied earlier - they are different predicates,
        // not duplicates.
        FilterAnalyzer::new(
            &self.fields,
            self.scan_info
                .indexrelid
                .expect("PgSearchTableProvider requires indexrelid to be set"),
        )
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
}

fn build_schema(fields: &[WhichFastField]) -> SchemaRef {
    let arrow_fields: Vec<Field> = fields
        .iter()
        .map(|f| Field::new(f.name(), f.arrow_data_type(), true))
        .collect();
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
        // TODO: Provide a useful implementation of statistics to allow DataFusion to
        // re-order joins effectively.
        None
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

        let heap_relid = self
            .scan_info
            .heaprelid
            .ok_or_else(|| DataFusionError::Internal("Missing heaprelid".into()))?;
        let index_relid = self
            .scan_info
            .indexrelid
            .ok_or_else(|| DataFusionError::Internal("Missing indexrelid".into()))?;

        let heap_rel = PgSearchRelation::open(heap_relid);
        let index_rel = PgSearchRelation::open(index_relid);

        // Start with the base query from scan_info
        let base_query = self
            .scan_info
            .query
            .clone()
            .unwrap_or(SearchQueryInput::All);

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
        let visibility = HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);

        if let Some(parallel_state) = self.parallel_state {
            let parts = (
                parallel_state,
                reader,
                Arc::new(ffhelper),
                Box::new(visibility) as Box<dyn VisibilityChecker>,
                self.fields.clone(),
                heap_relid.into(),
            );
            // TODO: In joinscan, only the "partitioning source" (the first source) needs to use
            // ParallelSegmentPlan to dynamically claim segments. Subsequent sources (inner sides
            // of the join) are fully replicated and should likely use MultiSegmentPlan to
            // expose sorted partitions to DataFusion, allowing for sorted merge joins.
            // See https://github.com/paradedb/paradedb/issues/4062
            Ok(Arc::new(ParallelSegmentPlan::new(parts, self.get_schema())))
        } else {
            let search_results = reader.search();
            let scanner =
                Scanner::new(search_results, None, self.fields.clone(), heap_relid.into());

            Ok(Arc::new(SegmentPlan::new(
                scanner,
                ffhelper,
                Box::new(visibility),
                query,
            )))
        }
    }
}
