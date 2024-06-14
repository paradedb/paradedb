use anyhow::Result;
use datafusion::arrow::error::ArrowError;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::catalog::CatalogProvider;
use datafusion::common::DataFusionError;
use datafusion::prelude::DataFrame;
use datafusion::sql::TableReference;
use deltalake::DeltaTableError;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use supabase_wrappers::prelude::*;
use thiserror::Error;
use url::Url;

use crate::datafusion::context::ContextError;
use crate::datafusion::format::*;
use crate::datafusion::provider::*;
use crate::datafusion::schema::LakehouseSchemaProvider;
use crate::datafusion::session::*;
use crate::schema::attribute::*;
use crate::schema::cell::*;

pub trait BaseFdw {
    // Public methods
    fn register_object_store(
        url: &Url,
        format: TableFormat,
        server_options: HashMap<String, String>,
        user_mapping_options: HashMap<String, String>,
    ) -> Result<()>;

    // Getter methods
    fn get_current_batch(&self) -> Option<RecordBatch>;
    fn get_current_batch_index(&self) -> usize;
    fn get_target_columns(&self) -> Vec<Column>;

    // Setter methods
    fn set_current_batch(&mut self, batch: Option<RecordBatch>);
    fn set_current_batch_index(&mut self, index: usize);
    fn set_dataframe(&mut self, dataframe: DataFrame);
    async fn create_stream(&mut self) -> Result<()>;
    fn clear_stream(&mut self);
    fn set_target_columns(&mut self, columns: &[Column]);

    // DataFusion methods
    async fn get_next_batch(&mut self) -> Result<Option<RecordBatch>>;

    // Default trait methods
    async fn begin_scan_impl(
        &mut self,
        _quals: &[Qual],
        columns: &[Column],
        _sorts: &[Sort],
        limit: &Option<Limit>,
        options: HashMap<String, String>,
    ) -> Result<(), BaseFdwError> {
        self.set_target_columns(columns);

        let oid_u32: u32 = options
            .get(OPTS_TABLE_KEY)
            .ok_or(BaseFdwError::TableOidNotFound)?
            .parse()?;
        let table_oid = pg_sys::Oid::from(oid_u32);
        let pg_relation = unsafe { PgRelation::open(table_oid) };
        let schema_name = pg_relation.namespace().to_string();
        let catalog = Session::catalog()?;

        if catalog.schema(&schema_name).is_none() {
            let new_schema_provider = Arc::new(LakehouseSchemaProvider::new(&schema_name));
            catalog.register_schema(&schema_name, new_schema_provider)?;
        }

        let limit = limit.clone();
        let context = Session::session_context()?;

        let reference = TableReference::full(
            Session::catalog_name()?,
            pg_relation.namespace(),
            pg_relation.name(),
        );
        let mut dataframe = context.table(reference).await?;
        if let Some(limit) = limit {
            dataframe = dataframe.limit(limit.offset as usize, Some(limit.count as usize))?;
        }

        self.set_dataframe(dataframe);

        Ok(())
    }

    async fn iter_scan_impl(&mut self, row: &mut Row) -> Result<Option<()>, BaseFdwError> {
        self.create_stream().await?;

        if self.get_current_batch().is_none()
            || self.get_current_batch_index()
                >= self
                    .get_current_batch()
                    .as_ref()
                    .ok_or(BaseFdwError::BatchNotFound)?
                    .num_rows()
        {
            self.set_current_batch_index(0);
            let next_batch = self.get_next_batch().await?;

            if next_batch.is_none() {
                return Ok(None);
            }

            self.set_current_batch(next_batch);
        }

        let current_batch_binding = self.get_current_batch();
        let current_batch = current_batch_binding
            .as_ref()
            .ok_or(BaseFdwError::BatchNotFound)?;
        let current_batch_index = self.get_current_batch_index();

        for (column_index, target_column) in
            self.get_target_columns().clone().into_iter().enumerate()
        {
            let batch_column = current_batch.column(column_index);
            let cell = batch_column.get_cell(
                current_batch_index,
                target_column.type_oid,
                target_column.type_mod,
            )?;
            row.push(target_column.name.as_str(), cell);
        }

        self.set_current_batch_index(current_batch_index + 1);

        Ok(Some(()))
    }

    fn end_scan_impl(&mut self) -> Result<(), BaseFdwError> {
        self.clear_stream();
        Ok(())
    }
}

impl From<BaseFdwError> for pg_sys::panic::ErrorReport {
    fn from(value: BaseFdwError) -> Self {
        pg_sys::panic::ErrorReport::new(PgSqlErrorCode::ERRCODE_FDW_ERROR, format!("{}", value), "")
    }
}

#[derive(Error, Debug)]
pub enum BaseFdwError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error(transparent)]
    ArrowError(#[from] ArrowError),

    #[error(transparent)]
    ContextError(#[from] ContextError),

    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error(transparent)]
    DataTypeError(#[from] DataTypeError),

    #[error(transparent)]
    DeltaTableError(#[from] DeltaTableError),

    #[error(transparent)]
    FormatError(#[from] FormatError),

    #[error(transparent)]
    OptionsError(#[from] supabase_wrappers::options::OptionsError),

    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error(transparent)]
    SchemaError(#[from] SchemaError),

    #[error(transparent)]
    SessionError(#[from] SessionError),

    #[error(transparent)]
    TableProviderError(#[from] TableProviderError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error("Unexpected error: Expected RecordBatch but found None")]
    BatchNotFound,

    #[error("Unexpected error: DataFrame not found")]
    DataFrameNotFound,

    #[error("Received unexpected option \"{0}\". Valid options are: {1:?}")]
    InvalidOption(String, Vec<String>),

    #[error("Unexpected error: Expected SendableRecordBatchStream but found None")]
    StreamNotFound,

    #[error("Unexpected error: Table OID not found")]
    TableOidNotFound,

    #[error("Received unsupported FDW oid {0:?}")]
    UnsupportedFdwOid(PgOid),

    #[error("Url path {0:?} cannot be a base")]
    UrlNotBase(Url),
}
