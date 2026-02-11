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
use datafusion::logical_expr::{Expr, TableProviderFilterPushDown, TableSource, TableType};
use datafusion::physical_plan::ExecutionPlan;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};

use crate::index::fast_fields_helper::{FFHelper, WhichFastField};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::heap::VisibilityChecker as HeapVisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;
use crate::scan::execution_plan::ScanState;
use crate::scan::filter_pushdown::{combine_with_and, FilterAnalyzer};
use crate::scan::info::{RowEstimate, ScanInfo};
use crate::scan::{Scanner, VisibilityChecker};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MppParticipantConfig {
    pub index: usize,
    pub total_participants: usize,
}

impl datafusion::common::config::ExtensionOptions for MppParticipantConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn cloned(&self) -> Box<dyn datafusion::common::config::ExtensionOptions> {
        Box::new(self.clone())
    }

    fn set(&mut self, _key: &str, _value: &str) -> Result<()> {
        Err(DataFusionError::Internal(
            "MppParticipantConfig is read-only".into(),
        ))
    }

    fn entries(&self) -> Vec<datafusion::common::config::ConfigEntry> {
        vec![]
    }
}

impl datafusion::common::config::ConfigExtension for MppParticipantConfig {
    const PREFIX: &'static str = "mpp";
}

/// A DataFusion `TableProvider` for scanning ParadeDB indexes.
///
/// This provider supports parallel execution via a symmetrical MPP model.
/// Every participant (leader or background worker) executes the same physical plan,
/// and this provider performs segment slicing at the physical level based on the
/// participant's index provided in the session configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct PgSearchTableProvider {
    scan_info: ScanInfo,
    fields: Vec<WhichFastField>,
    #[serde(skip)]
    schema: OnceLock<SchemaRef>,
}

unsafe impl Send for PgSearchTableProvider {}
unsafe impl Sync for PgSearchTableProvider {}

impl PgSearchTableProvider {
    pub fn new(scan_info: ScanInfo, fields: Vec<WhichFastField>) -> Self {
        Self {
            scan_info,
            fields,
            schema: OnceLock::new(),
        }
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

    /// Creates a MultiSegmentPlan for serial scans (or fully replicated parallel joins).
    ///
    /// This method opens all segments upfront as separate partitions, allowing DataFusion
    /// to treat them as distinct streams (sorted or not).
    #[allow(clippy::too_many_arguments)]
    fn create_serial_scan(
        &self,
        reader: &SearchIndexReader,
        ffhelper: FFHelper,
        visibility: HeapVisibilityChecker,
        heap_relid: pg_sys::Oid,
        query_for_display: SearchQueryInput,
        sort_order: Option<&crate::postgres::options::SortByField>,
        participant_index: usize,
        total_participants: usize,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let ffhelper = Arc::new(ffhelper);
        let segment_readers = reader.segment_readers();
        let segments: Vec<ScanState> = segment_readers
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                // Context-aware slicing
                if total_participants > 1 {
                    let (start, length) = crate::parallel_worker::chunk_range(
                        segment_readers.len(),
                        total_participants,
                        participant_index,
                    );
                    *i >= start && *i < start + length
                } else {
                    true
                }
            })
            .map(|(_, r)| {
                let search_results = reader.search_segments(std::iter::once(r.segment_id()));
                let scanner =
                    Scanner::new(search_results, None, self.fields.clone(), heap_relid.into());
                (
                    scanner,
                    Arc::clone(&ffhelper),
                    Box::new(visibility.clone()) as Box<dyn VisibilityChecker>,
                )
            })
            .collect();

        // Only declare sort order if the field exists in the schema
        let actual_sort_order = sort_order.and_then(|so| {
            let field_name = so.field_name.as_ref();
            if self.get_schema().column_with_name(field_name).is_some() {
                Some(so)
            } else {
                None
            }
        });

        let partitioning = if total_participants > 1 {
            let schema = self.get_schema();
            // Find CTID column for partitioning.
            // We use CTID because it's guaranteed to be unique and present.
            let ctid_col = self
                .fields
                .iter()
                .enumerate()
                .find(|(_, ff)| matches!(ff, WhichFastField::Ctid));

            match ctid_col {
                Some((idx, _)) => {
                    let field = &schema.fields()[idx];
                    let expr = Arc::new(datafusion::physical_expr::expressions::Column::new(
                        field.name(),
                        idx,
                    ))
                        as Arc<dyn datafusion::physical_expr::PhysicalExpr>;
                    datafusion::physical_plan::Partitioning::Hash(vec![expr], total_participants)
                }
                None => {
                    datafusion::physical_plan::Partitioning::UnknownPartitioning(total_participants)
                }
            }
        } else {
            datafusion::physical_plan::Partitioning::UnknownPartitioning(segments.len().max(1))
        };

        Ok(Arc::new(crate::scan::execution_plan::MultiSegmentPlan::new_with_partitioning(
            segments,
            self.get_schema(),
            query_for_display,
            actual_sort_order,
            Some(partitioning),
        )))
    }
}

impl TableSource for PgSearchTableProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.get_schema()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
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
        use datafusion::common::stats::{ColumnStatistics, Precision};

        let num_rows = match self.scan_info.estimate {
            Some(RowEstimate::Known(n)) => Precision::Inexact(n as usize),
            _ => Precision::Absent,
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

        let reader = SearchIndexReader::open_with_context(
            &index_rel,
            query.clone(),
            self.scan_info.score_needed,
            MvccSatisfies::Snapshot,
            None,
            None,
        )
        .map_err(|e| DataFusionError::Internal(format!("Failed to open reader: {e}")))?;

        let ffhelper = FFHelper::with_fields(&reader, &self.fields);
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let visibility = HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);
        let sort_order = self.scan_info.sort_order.as_ref();

        let mpp_config = _state
            .config_options()
            .extensions
            .get::<MppParticipantConfig>();
        let participant_index = mpp_config.map(|c| c.index).unwrap_or(0);
        let total_participants = mpp_config.map(|c| c.total_participants).unwrap_or(1);

        if let Some(sort_order) = sort_order {
            // Serial sorted scan (or replicated source with sorting)
            self.create_serial_scan(
                &reader,
                ffhelper,
                visibility,
                heap_relid,
                query,
                Some(sort_order),
                participant_index,
                total_participants,
            )
        } else {
            // For serial/replicated scans without sorting, we use MultiSegmentPlan
            // (1 partition per segment).
            //
            // Using MultiSegmentPlan is beneficial because it exposes segments as partitions to
            // DataFusion, allowing parallel processing within the DataFusion executor.
            // It also unifies the code path.
            self.create_serial_scan(
                &reader,
                ffhelper,
                visibility,
                heap_relid,
                query,
                None,
                participant_index,
                total_participants,
            )
        }
    }
}
