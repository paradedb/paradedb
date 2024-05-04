use async_std::task;
use datafusion::arrow::error::ArrowError;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::catalog::CatalogProvider;
use datafusion::common::arrow::datatypes::{DataType, Field, SchemaBuilder};
use datafusion::common::DataFusionError;
use datafusion::physical_plan::SendableRecordBatchStream;
use deltalake::DeltaTableError;
use fdw::format::*;
use fdw::lake::*;
use fdw::options::TableOption;
use iceberg::{Catalog, NamespaceIdent};
use iceberg_catalog_glue::{
    GlueCatalog, GlueCatalogConfig, AWS_ACCESS_KEY_ID, AWS_REGION_NAME, AWS_SECRET_ACCESS_KEY,
};
use iceberg_datafusion::IcebergCatalogProvider;
use pgrx::*;
use std::collections::HashMap;
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
        let format = require_option_or(TableOption::Format.as_str(), options, "");

        pgrx::info!("FORMAT {format}, {:?}", oid_map);
        let provider = match TableFormat::from(format) {
            TableFormat::None => {
                create_listing_provider(options.clone(), oid_map, &self.get_session_state())?
            }
            TableFormat::Delta => task::block_on(create_delta_provider(options.clone()))?,
            TableFormat::Iceberg => task::block_on(create_iceberg_provider(options.clone()))?,
        };

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

#[inline]
fn create_listing_provider(
    table_options: HashMap<String, String>,
    mut oid_map: HashMap<usize, pg_sys::Oid>,
    state: &SessionState,
) -> Result<Arc<dyn TableProvider>, BaseFdwError> {
    let path = require_option(TableOption::Path.as_str(), &table_options)?;
    let extension = require_option(TableOption::Extension.as_str(), &table_options)?;

    let listing_url = ListingTableUrl::parse(path)?;
    let listing_options = ListingOptions::try_from(FileExtension(extension.to_string()))?;

    let inferred_schema = task::block_on(listing_options.infer_schema(state, &listing_url))?;
    let mut schema_builder = SchemaBuilder::new();

    for (index, field) in inferred_schema.fields().iter().enumerate() {
        match oid_map.remove(&index) {
            Some(oid) => {
                // Types can get incorrectly inferred, so we override them
                let data_type = match (oid, field.data_type()) {
                    (pg_sys::BOOLOID, _) => DataType::Boolean,
                    (pg_sys::DATEOID, _) => DataType::Int32,
                    (pg_sys::TIMESTAMPOID, _) => DataType::Int64,
                    (pg_sys::VARCHAROID, _) => DataType::Utf8,
                    (pg_sys::BPCHAROID, _) => DataType::Utf8,
                    (pg_sys::TEXTOID, _) => DataType::Utf8,
                    (pg_sys::INT2OID, _) => DataType::Int16,
                    (pg_sys::INT4OID, _) => DataType::Int32,
                    (pg_sys::INT8OID, _) => DataType::Int64,
                    (pg_sys::FLOAT4OID, _) => DataType::Float32,
                    (pg_sys::FLOAT8OID, _) => DataType::Float64,
                    (_, data_type) => data_type.clone(),
                };
                schema_builder.push(Field::new(field.name(), data_type, field.is_nullable()))
            }
            None => schema_builder.push(field.clone()),
        };
    }

    let updated_schema = Arc::new(schema_builder.finish());

    let listing_config = ListingTableConfig::new(listing_url)
        .with_listing_options(listing_options)
        .with_schema(updated_schema);

    let listing_table = ListingTable::try_new(listing_config)?;

    Ok(Arc::new(listing_table) as Arc<dyn TableProvider>)
}

#[inline]
async fn create_delta_provider(
    table_options: HashMap<String, String>,
) -> Result<Arc<dyn TableProvider>, BaseFdwError> {
    let path = require_option(TableOption::Path.as_str(), &table_options)?;
    let delta_table = deltalake::open_table(path).await?;

    Ok(Arc::new(delta_table) as Arc<dyn TableProvider>)
}

#[inline]
async fn create_iceberg_provider(
    table_options: HashMap<String, String>,
) -> Result<Arc<dyn TableProvider>, BaseFdwError> {
    let path = require_option(TableOption::Path.as_str(), &table_options)?;

    // TODO: Replace with non-REST.
    let config = GlueCatalogConfig::builder()
        .warehouse(path.to_string())
        .props(HashMap::from([
            (AWS_ACCESS_KEY_ID.to_string(), "".to_string()),
            (AWS_SECRET_ACCESS_KEY.to_string(), "".to_string()),
            (AWS_REGION_NAME.to_string(), "us-east-1".to_string()),
            // (
            //     S3_ENDPOINT.to_string(),
            //     format!("http://{}:{}", minio_ip, MINIO_PORT),
            // ),
            // (S3_ACCESS_KEY_ID.to_string(), "admin".to_string()),
            // (S3_SECRET_ACCESS_KEY.to_string(), "password".to_string()),
            // (S3_REGION.to_string(), "us-east-1".to_string()),
        ]))
        .build();
    pgrx::info!("config {config:?}");

    let client = Arc::new(GlueCatalog::new(config).await.unwrap());
    pgrx::info!("client {client:?}");

    client
        .create_namespace(&NamespaceIdent::new("default".into()), HashMap::default())
        .await
        .unwrap();

    let catalog = Arc::new(
        IcebergCatalogProvider::try_new(client)
            .await
            .expect("no catalog provider"),
    );
    pgrx::info!("catalog");

    pgrx::info!("schema names {:?}", catalog.schema_names());
    let schema = catalog.schema("default").expect("no schema");
    pgrx::info!("schema");

    let table = schema
        .table(path)
        .await
        .expect("no inner table")
        .expect("no outer table");
    pgrx::info!("table");

    Ok(table)
}

#[derive(Error, Debug)]
pub enum BaseFdwError {
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
}
