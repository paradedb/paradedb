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
use crate::postgres::heap::VisibilityChecker as HeapVisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;
use crate::scan::datafusion_plan::SegmentPlan;
use crate::scan::info::ScanInfo;
use crate::scan::Scanner;

#[derive(Debug, Serialize, Deserialize)]
pub struct PgSearchTableProvider {
    scan_info: ScanInfo,
    fields: Vec<WhichFastField>,
    #[serde(skip)]
    schema: OnceLock<SchemaRef>,
}

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
        // TODO: We don't support pushdown here yet (we rely on JoinScan's manual extraction).
        // Return Unsupported for all filters so DataFusion keeps them in the plan if it adds any.
        Ok(vec![
            TableProviderFilterPushDown::Unsupported;
            filters.len()
        ])
    }

    async fn scan(
        &self,
        _state: &dyn Session,
        _projection: Option<&Vec<usize>>,
        _filters: &[Expr],
        _limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // TODO: See TODO in supports_filters_pushdown. We should support limit pushdown here
        // to allow providing a batch size hint to the Scanner.
        //
        // Ignore projection, filters, limit for now as they are handled by the join logic
        // or effectively pre-calculated in `fields`.

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

        let query = self
            .scan_info
            .query
            .clone()
            .unwrap_or(SearchQueryInput::All);

        let reader = SearchIndexReader::open_with_context(
            &index_rel,
            query.clone(),
            self.scan_info.score_needed,
            MvccSatisfies::Snapshot,
            None,
            None,
        )
        .map_err(|e| DataFusionError::Internal(format!("Failed to open reader: {e}")))?;

        let search_results = reader.search();
        let ffhelper = FFHelper::with_fields(&reader, &self.fields);

        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let visibility = HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);

        let scanner = Scanner::new(search_results, None, self.fields.clone(), heap_relid.into());

        Ok(Arc::new(SegmentPlan::new(
            scanner,
            ffhelper,
            Box::new(visibility),
            Some(query),
        )))
    }
}
