use async_std::task;
use datafusion::arrow::error::ArrowError;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::DataFusionError;
use datafusion::physical_plan::SendableRecordBatchStream;
use deltalake::DeltaTableError;
use iceberg_datafusion::IcebergCatalogProvider;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use supabase_wrappers::prelude::*;
use thiserror::Error;

use crate::datafusion::context::ContextError;
use crate::datafusion::format::*;
use crate::datafusion::provider::*;
use crate::datafusion::session::Session;
use crate::fdw::options::TableOption;
use crate::schema::attribute::*;
use crate::schema::cell::*;

pub trait BaseFdw {
    // Public methods
    fn register_object_store(
        server_options: HashMap<String, String>,
        user_mapping_options: HashMap<String, String>,
    ) -> Result<(), ContextError>;

    // Getter methods
    fn get_current_batch(&self) -> Option<RecordBatch>;
    fn get_current_batch_index(&self) -> usize;
    fn get_target_columns(&self) -> Vec<Column>;

    // Setter methods
    fn set_current_batch(&mut self, batch: Option<RecordBatch>);
    fn set_current_batch_index(&mut self, index: usize);
    fn set_stream(&mut self, stream: Option<SendableRecordBatchStream>);
    fn set_target_columns(&mut self, columns: &[Column]);

    // DataFusion methods
    async fn get_next_batch(&mut self) -> Result<Option<RecordBatch>, BaseFdwError>;

    // Default trait methods
    fn begin_scan_impl(
        &mut self,
        _quals: &[Qual],
        columns: &[Column],
        _sorts: &[Sort],
        limit: &Option<Limit>,
        options: HashMap<String, String>,
    ) -> Result<(), BaseFdwError> {
        self.set_target_columns(columns);

        let mut attribute_map: HashMap<usize, PgAttribute> = columns
            .iter()
            .cloned()
            .map(|col| (col.num - 1, PgAttribute::new(&col.name, col.type_oid)))
            .collect();

        let limit = limit.clone();
        let columns = columns.to_vec();

        // The Oids passed into options where converted to strings, and their Display impls
        // cause them to appear like: 'oid=#'. As a hack, we'll just strip that and parse the
        // remaining string as a u32
        let database_oid = options
            .get("database_oid")
            .expect("no database_oid passed to being_scan_impl")
            .strip_prefix("oid=#")
            .and_then(|s| s.parse::<u32>().ok())
            .expect("error parsing database oid in begin_scan_impl")
            .clone();
        let schema_oid = options
            .get("namespace_oid")
            .expect("no database_oid passed to being_scan_impl")
            .strip_prefix("oid=#")
            .and_then(|s| s.parse::<u32>().ok())
            .expect("error parsing schema oid in begin_scan_impl")
            .clone();
        let table_oid = options
            .get("table_oid")
            .expect("no database_oid passed to being_scan_impl")
            .strip_prefix("oid=#")
            .and_then(|s| s.parse::<u32>().ok())
            .expect("error parsing table oid in begin_scan_impl")
            .clone();

        let foreign_table = unsafe { pg_sys::GetForeignTable(pg_sys::Oid::from(table_oid)) };
        let foreign_server = unsafe { pg_sys::GetForeignServer((*foreign_table).serverid) };
        let user_mapping_options = unsafe { user_mapping_options(foreign_server) };

        let result = Session::with_session_context(|context| {
            Box::pin(async move {
                let format = require_option_or(TableOption::Format.as_str(), &options, "");
                let path = require_option(TableOption::Path.as_str(), &options)?;
                let extension = require_option(TableOption::Extension.as_str(), &options)?;

                let provider = match TableFormat::from(format) {
                    TableFormat::None => {
                        task::block_on(create_listing_provider(path, extension, &context.state()))?
                    }
                    TableFormat::Delta => task::block_on(create_delta_provider(path, extension))?,
                    TableFormat::Iceberg => {
                        let table_config = IcebergTableConfigBuilder::default()
                            .warehouse(path)
                            .database(database_oid.to_string())
                            .namespace(schema_oid.to_string())
                            .table(table_oid.to_string())
                            .columns(&columns)
                            .map_err(|err| BaseFdwError::Unexpected(err.to_string()))
                            .expect("could not convert columns to arrow schema")
                            .build()
                            .map_err(|err| BaseFdwError::Unexpected(err.to_string()))
                            .expect("could not build iceberg table config");
                        let catalog_config = IcebergCatalogConfigBuilder::default()
                            .aws_access_key_opt(user_mapping_options.get("aws_access_key"))
                            .aws_secret_access_key_opt(
                                user_mapping_options.get("aws_secret_access_key"),
                            )
                            .aws_region("us-east-1")
                            .build()
                            .map_err(|err| BaseFdwError::Unexpected(err.to_string()))
                            .expect("could not build iceberg catalog config");

                        let catalog = task::block_on(catalog_config.catalog(&table_config))
                            .expect("could not create iceberg catalog");

                        task::block_on(catalog_config.ensure_namespace(&catalog, &table_config))
                            .expect("could not create iceberg namespace");

                        task::block_on(catalog_config.ensure_table(&catalog, &table_config))
                            .expect("could not create iceberg table");

                        let table_provider =
                            task::block_on(catalog_config.table_provider(&table_config))
                                .expect("could not build iceberg table provider");

                        context.register_catalog(
                            "iceberg",
                            Arc::new(
                                IcebergCatalogProvider::try_new(Arc::new(catalog))
                                    .await
                                    .expect("could not create iceberg catalog provider"),
                            ),
                        );

                        table_provider
                    }
                };

                for (index, field) in provider.schema().fields().iter().enumerate() {
                    if let Some(attribute) = attribute_map.remove(&index) {
                        can_convert_to_attribute(field, attribute)?;
                    }
                }

                let mut dataframe = context.read_table(provider)?.select_columns(
                    &columns
                        .iter()
                        .map(|c| c.name.as_str())
                        .collect::<Vec<&str>>(),
                )?;

                if let Some(limit) = limit {
                    dataframe =
                        dataframe.limit(limit.offset as usize, Some(limit.count as usize))?;
                }

                Ok(context
                    .execute_logical_plan(dataframe.logical_plan().clone())
                    .await?)
            })
        })?;

        self.set_stream(Some(task::block_on(result.execute_stream())?));

        Ok(())
    }

    fn iter_scan_impl(&mut self, row: &mut Row) -> Result<Option<()>, BaseFdwError> {
        if self.get_current_batch().is_none()
            || self.get_current_batch_index()
                >= self
                    .get_current_batch()
                    .as_ref()
                    .ok_or(BaseFdwError::BatchNotFound)?
                    .num_rows()
        {
            self.set_current_batch_index(0);
            let next_batch = task::block_on(self.get_next_batch())?;

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
        self.set_stream(None);
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
    ArrowError(#[from] ArrowError),

    #[error("could not convert oid/typemod to arrow data type: {0}")]
    ArrowConversionError(String),

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
    SchemaError(#[from] SchemaError),

    #[error(transparent)]
    TableProviderError(#[from] TableProviderError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error("Unexpected error: Expected RecordBatch but found None")]
    BatchNotFound,

    #[error("Received unexpected option \"{0}\". Valid options are: {1:?}")]
    InvalidOption(String, Vec<String>),

    #[error("Unexpected error: Expected SendableRecordBatchStream but found None")]
    StreamNotFound,

    #[error("Received unsupported FDW oid {0:?}")]
    UnsupportedFdwOid(PgOid),

    #[error("Unexpected error: {0}")]
    Unexpected(String),
}
