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
use datafusion::common::TableReference;
use datafusion::common::{DataFusionError, Result, Statistics};
use datafusion::execution::TaskContext;
use datafusion::logical_expr::{
    Expr, Extension, LogicalPlan, ScalarUDF, TableProviderFilterPushDown, TableType,
};
use datafusion::physical_plan::ExecutionPlan;
use datafusion_proto::logical_plan::LogicalExtensionCodec;

use crate::postgres::customscan::joinscan::udf::RowInSetUDF;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};

use crate::index::fast_fields_helper::{FFHelper, FastFieldType, WhichFastField};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::heap::VisibilityChecker as HeapVisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;
use crate::scan::datafusion_plan::ScanPlan;
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
        .map(|f| match f {
            WhichFastField::Ctid => Field::new("ctid", arrow_schema::DataType::UInt64, true),
            WhichFastField::Score => {
                Field::new("pdb.score()", arrow_schema::DataType::Float32, true)
            }
            WhichFastField::Named(name, typ) => {
                let dt = match typ {
                    FastFieldType::Int64 => arrow_schema::DataType::Int64,
                    FastFieldType::UInt64 => arrow_schema::DataType::UInt64,
                    FastFieldType::Float64 => arrow_schema::DataType::Float64,
                    FastFieldType::Bool => arrow_schema::DataType::Boolean,
                    FastFieldType::String => arrow_schema::DataType::Utf8View,
                    FastFieldType::Date => {
                        arrow_schema::DataType::Timestamp(arrow_schema::TimeUnit::Nanosecond, None)
                    }
                };
                Field::new(name, dt, true)
            }
            WhichFastField::TableOid => {
                Field::new("tableoid", arrow_schema::DataType::UInt32, true)
            }
            WhichFastField::Junk(name) => Field::new(name, arrow_schema::DataType::Null, true),
        })
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
            query,
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

        Ok(Arc::new(ScanPlan::new(
            scanner,
            ffhelper,
            Box::new(visibility),
        )))
    }
}

/// Datafusion `LogicalPlan`s are serialized/deserialized with protobuf.
/// Any custom nodes (e.g. UDFs, table providers) must use this codec to instruct
/// DataFusion how to serialize/deserialize them.
#[derive(Debug, Default)]
pub struct PgSearchExtensionCodec;

impl LogicalExtensionCodec for PgSearchExtensionCodec {
    fn try_decode(
        &self,
        _buf: &[u8],
        _inputs: &[LogicalPlan],
        _ctx: &TaskContext,
    ) -> Result<Extension> {
        Err(DataFusionError::NotImplemented(
            "Extension node decoding not implemented".to_string(),
        ))
    }

    fn try_encode(&self, _node: &Extension, _buf: &mut Vec<u8>) -> Result<()> {
        Err(DataFusionError::NotImplemented(
            "Extension node encoding not implemented".to_string(),
        ))
    }

    fn try_decode_table_provider(
        &self,
        buf: &[u8],
        _table_ref: &TableReference,
        _schema: SchemaRef,
        _ctx: &TaskContext,
    ) -> Result<Arc<dyn TableProvider>> {
        let provider: PgSearchTableProvider = serde_json::from_slice(buf).map_err(|e| {
            DataFusionError::Internal(format!("Failed to deserialize PgSearchTableProvider: {e}"))
        })?;
        Ok(Arc::new(provider))
    }

    fn try_encode_table_provider(
        &self,
        _table_ref: &TableReference,
        node: Arc<dyn TableProvider>,
        buf: &mut Vec<u8>,
    ) -> Result<()> {
        let provider = node
            .as_any()
            .downcast_ref::<PgSearchTableProvider>()
            .ok_or_else(|| {
                DataFusionError::Internal(
                    "TableProvider is not a PgSearchTableProvider".to_string(),
                )
            })?;
        let bytes = serde_json::to_vec(provider).map_err(|e| {
            DataFusionError::Internal(format!("Failed to serialize PgSearchTableProvider: {e}"))
        })?;
        buf.extend_from_slice(&bytes);
        Ok(())
    }

    fn try_decode_udf(&self, name: &str, buf: &[u8]) -> Result<Arc<ScalarUDF>> {
        match name {
            "row_in_set" => {
                let udf: RowInSetUDF = serde_json::from_slice(buf).map_err(|e| {
                    DataFusionError::Internal(format!("Failed to deserialize RowInSetUDF: {e}"))
                })?;
                Ok(Arc::new(ScalarUDF::new_from_impl(udf)))
            }
            _ => Err(DataFusionError::NotImplemented(format!(
                "UDF '{}' deserialization not implemented",
                name
            ))),
        }
    }

    fn try_encode_udf(&self, node: &ScalarUDF, buf: &mut Vec<u8>) -> Result<()> {
        let name = node.name();
        match name {
            "row_in_set" => {
                let udf = node
                    .inner()
                    .as_any()
                    .downcast_ref::<RowInSetUDF>()
                    .ok_or_else(|| {
                        DataFusionError::Internal("UDF is not a RowInSetUDF".to_string())
                    })?;
                let bytes = serde_json::to_vec(udf).map_err(|e| {
                    DataFusionError::Internal(format!("Failed to serialize RowInSetUDF: {e}"))
                })?;
                buf.extend_from_slice(&bytes);
                Ok(())
            }
            _ => Err(DataFusionError::NotImplemented(format!(
                "UDF '{}' serialization not implemented",
                name
            ))),
        }
    }
}
