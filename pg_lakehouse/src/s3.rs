use async_std::stream::StreamExt;
use async_std::task;
use datafusion::arrow::error::ArrowError;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::arrow::datatypes::{DataType, Field, SchemaBuilder, TimeUnit};
use datafusion::common::DataFusionError;
use datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use datafusion::datasource::{provider_as_source, TableProvider};
use datafusion::logical_expr::LogicalPlanBuilder;
use datafusion::physical_plan::SendableRecordBatchStream;
use datafusion::prelude::SessionContext;
use object_store::aws::AmazonS3;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use supabase_wrappers::prelude::*;
use thiserror::Error;
use url::Url;

use super::cell::*;
use super::format::*;
use super::options::*;

// Because the SessionContext is recreated on each scan, we don't need to worry about
// assigning a unique name to the DataFusion table
const DEFAULT_TABLE_NAME: &str = "listing_table";

#[wrappers_fdw(
    author = "ParadeDB",
    website = "https://github.com/paradedb/paradedb",
    error_type = "S3FdwError"
)]

pub(crate) struct S3Fdw {
    stream: Option<SendableRecordBatchStream>,
    current_batch: Option<RecordBatch>,
    current_batch_index: usize,
    context: SessionContext,
    target_columns: Vec<Column>,
}

impl From<S3FdwError> for pg_sys::panic::ErrorReport {
    fn from(value: S3FdwError) -> Self {
        pg_sys::panic::ErrorReport::new(PgSqlErrorCode::ERRCODE_FDW_ERROR, format!("{}", value), "")
    }
}

impl ForeignDataWrapper<S3FdwError> for S3Fdw {
    fn new(options: &HashMap<String, String>) -> Result<Self, S3FdwError> {
        // Create S3 ObjectStore
        let object_store = AmazonS3::try_from(ServerOptions(options.clone()))?;
        let url = require_option(ServerOption::Url.as_str(), options)?;

        // Create SessionContext with ObjectStore
        let context = SessionContext::new();
        context
            .runtime_env()
            .register_object_store(&Url::parse(url)?, Arc::new(object_store));

        Ok(Self {
            current_batch: None,
            current_batch_index: 0,
            stream: None,
            target_columns: Vec::new(),
            context,
        })
    }

    fn validator(
        opt_list: Vec<Option<String>>,
        catalog: Option<pg_sys::Oid>,
    ) -> Result<(), S3FdwError> {
        if let Some(oid) = catalog {
            match oid {
                FOREIGN_DATA_WRAPPER_RELATION_ID => {}
                FOREIGN_SERVER_RELATION_ID => {
                    for opt in ServerOption::iter() {
                        if opt.is_required() {
                            check_options_contain(&opt_list, opt.as_str())?;
                        }
                    }
                }
                FOREIGN_TABLE_RELATION_ID => {
                    for opt in TableOption::iter() {
                        if opt.is_required() {
                            check_options_contain(&opt_list, opt.as_str())?;
                        }
                    }
                }
                unsupported => return Err(S3FdwError::UnsupportedFdwOid(unsupported)),
            }
        }

        Ok(())
    }

    fn begin_scan(
        &mut self,
        _quals: &[Qual],
        columns: &[Column],
        _sorts: &[Sort],
        limit: &Option<Limit>,
        options: &HashMap<String, String>,
    ) -> Result<(), S3FdwError> {
        self.target_columns = columns.to_vec();

        // Register ListingTable with SessionContext
        let table_url = require_option(TableOption::Url.as_str(), options)?;
        let format = require_option(TableOption::Format.as_str(), options)?;
        let listing_url = ListingTableUrl::parse(table_url)?;
        let listing_options = ListingOptions::try_from(FileFormat(format.to_string()))?;

        let inferred_schema =
            task::block_on(listing_options.infer_schema(&self.context.state(), &listing_url))?;
        let mut schema_builder = SchemaBuilder::new();
        let mut columns_map: HashMap<usize, pg_sys::Oid> = columns
            .iter()
            .cloned()
            .map(|col| (col.num - 1, col.type_oid))
            .collect();

        for (index, field) in inferred_schema.fields().iter().enumerate() {
            match columns_map.remove(&index) {
                Some(oid) => {
                    // Some types like DATE and TIMESTAMP get incorrectly inferred as
                    // Int32/Int64, so we need to override them
                    let data_type = match (oid, field.data_type()) {
                        (pg_sys::DATEOID, _) => DataType::Date32,
                        (pg_sys::TIMESTAMPOID, _) => {
                            DataType::Timestamp(TimeUnit::Microsecond, None)
                        }
                        (_, data_type) => data_type.clone(),
                    };
                    schema_builder.push(Field::new(field.name(), data_type, field.is_nullable()))
                }
                None => schema_builder.push(field.clone()),
            };
        }

        let updated_schema = Arc::new(schema_builder.finish());

        info!("Inferred schema: {:?}", updated_schema);

        let listing_config = ListingTableConfig::new(listing_url)
            .with_listing_options(listing_options)
            .with_schema(updated_schema);

        let listing_table = ListingTable::try_new(listing_config)?;
        let provider = Arc::new(listing_table);
        self.context
            .register_table(DEFAULT_TABLE_NAME, provider.clone())?;

        // Construct LogicalPlan
        let mut logical_plan = LogicalPlanBuilder::scan(
            DEFAULT_TABLE_NAME,
            provider_as_source(provider as Arc<dyn TableProvider>),
            Some(columns.iter().map(|c| c.num - 1).collect::<Vec<usize>>()),
        )?;

        if let Some(limit) = limit {
            logical_plan = logical_plan.limit(limit.offset as usize, Some(limit.count as usize))?;
        }

        let dataframe = task::block_on(self.context.execute_logical_plan(logical_plan.build()?))?;
        self.stream = Some(task::block_on(dataframe.execute_stream())?);

        Ok(())
    }

    fn iter_scan(&mut self, row: &mut Row) -> Result<Option<()>, S3FdwError> {
        if self.current_batch.is_none()
            || self.current_batch_index
                >= self
                    .current_batch
                    .as_ref()
                    .ok_or(S3FdwError::BatchNotFound)?
                    .num_rows()
        {
            self.current_batch_index = 0;
            self.current_batch = match task::block_on(
                self.stream
                    .as_mut()
                    .ok_or(S3FdwError::StreamNotFound)?
                    .next(),
            ) {
                Some(Ok(b)) => Some(b),
                None => None,
                Some(Err(err)) => {
                    return Err(S3FdwError::DataFusionError(err));
                }
            };

            if self.current_batch.is_none() {
                return Ok(None);
            }
        }

        let current_batch = self
            .current_batch
            .as_ref()
            .ok_or(S3FdwError::BatchNotFound)?;
        let current_batch_index = self.current_batch_index;

        if current_batch.num_columns() != self.target_columns.len() {
            return Err(S3FdwError::ColumnMismatch(
                self.target_columns.len(),
                current_batch.num_columns(),
            ));
        }

        for (column_index, target_column) in self.target_columns.clone().into_iter().enumerate() {
            let batch_column = current_batch.column(column_index);
            let cell = batch_column.get_cell(current_batch_index, target_column.type_oid)?;
            row.push(target_column.name.as_str(), cell);
        }

        self.current_batch_index += 1;

        Ok(Some(()))
    }

    fn end_scan(&mut self) -> Result<(), S3FdwError> {
        self.stream = None;
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum S3FdwError {
    #[error(transparent)]
    ArrowError(#[from] ArrowError),

    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error(transparent)]
    DataTypeError(#[from] DataTypeError),

    #[error(transparent)]
    FileFormatError(#[from] FileFormatError),

    #[error(transparent)]
    ObjectStoreError(#[from] object_store::Error),

    #[error(transparent)]
    OptionsError(#[from] super::options::OptionsError),

    #[error(transparent)]
    SupabaseOptionsError(#[from] supabase_wrappers::options::OptionsError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error("Unexpected error: Expected RecordBatch but found None")]
    BatchNotFound,

    #[error("Expected {0} columns but scan returned {1} columns")]
    ColumnMismatch(usize, usize),

    #[error("Unexpected error: Expected SendableRecordBatchStream but found None")]
    StreamNotFound,

    #[error("Received unsupported FDW oid {0:?}")]
    UnsupportedFdwOid(pg_sys::Oid),
}
