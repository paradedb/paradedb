use async_std::task;
use datafusion::catalog::schema::SchemaProvider;
use datafusion::common::arrow::datatypes::DataType;
use datafusion::common::config::ConfigOptions;
use datafusion::common::DataFusionError;
use datafusion::datasource::provider_as_source;
use datafusion::execution::FunctionRegistry;
use datafusion::logical_expr::{AggregateUDF, ScalarUDF, TableSource, WindowUDF};
use datafusion::sql::planner::ContextProvider;
use datafusion::sql::TableReference;
use pgrx::*;
use std::ffi::{c_char, CStr};
use std::sync::Arc;
use thiserror::Error;

use crate::datafusion::format::*;
use crate::schema::attribute::*;

use super::plan::*;
use super::provider::*;
use super::session::*;

pub struct QueryContext {
    options: ConfigOptions,
}

impl QueryContext {
    pub fn new() -> Self {
        Self {
            options: ConfigOptions::new(),
        }
    }
}

impl ContextProvider for QueryContext {
    fn get_table_source(
        &self,
        reference: TableReference,
    ) -> Result<Arc<dyn TableSource>, DataFusionError> {
        task::block_on(get_table_source(reference))
            .map_err(|err| DataFusionError::Execution(err.to_string()))
    }

    fn get_function_meta(&self, name: &str) -> Option<Arc<ScalarUDF>> {
        let context = Session::session_context().unwrap_or_else(|err| {
            panic!("{}", err);
        });

        context.udf(name).ok()
    }

    fn get_aggregate_meta(&self, _name: &str) -> Option<Arc<AggregateUDF>> {
        None
    }

    fn get_variable_type(&self, _variable_names: &[String]) -> Option<DataType> {
        None
    }

    fn get_window_meta(&self, _name: &str) -> Option<Arc<WindowUDF>> {
        None
    }

    fn options(&self) -> &ConfigOptions {
        &self.options
    }

    fn udfs_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn udafs_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn udwfs_names(&self) -> Vec<String> {
        Vec::new()
    }
}

pub async fn get_table_source(
    reference: TableReference<'_>,
) -> Result<Arc<dyn TableSource>, ContextError> {
    let catalog_name = Session::catalog_name()?;
    let schema_name = reference.schema();

    match schema_name {
        Some(schema_name) => {
            // If a schema was provided in the query, i.e. SELECT * FROM <schema>.<table>
            let _ = Session::schema_provider(schema_name)?;
            get_source(&catalog_name, schema_name, reference.table()).await
        }
        None => {
            // If no schema was provided in the query, i.e. SELECT * FROM <table>
            // Read all schemas from the Postgres search path and cascade through them
            // until a table is found
            let current_schemas = unsafe {
                direct_function_call::<Array<pg_sys::Datum>>(
                    pg_sys::current_schemas,
                    &[Some(pg_sys::Datum::from(true))],
                )
            };

            if let Some(current_schemas) = current_schemas {
                for datum in current_schemas.iter().flatten() {
                    let schema_name =
                        unsafe { CStr::from_ptr(datum.cast_mut_ptr::<c_char>()).to_str()? };
                    let table_name = reference.table().to_string();
                    let schema_provider = Session::schema_provider(schema_name)?;

                    if !schema_provider.table_exist(&table_name.clone()) {
                        continue;
                    }

                    return get_source(&catalog_name, schema_name, reference.table()).await;
                }
            }

            Err(ContextError::TableNotFound(reference.table().to_string()))
        }
    }
}

#[inline]
async fn get_source(
    catalog_name: &str,
    schema_name: &str,
    table_name: &str,
) -> Result<Arc<dyn TableSource>, ContextError> {
    let catalog_name = catalog_name.to_string();
    let schema_name = schema_name.to_string();
    let table_name = table_name.to_string();
    let context = Session::session_context()?;
    let table_reference = TableReference::full(catalog_name, schema_name, table_name);
    let provider = context.table_provider(table_reference).await?;

    Ok(provider_as_source(provider))
}

#[derive(Error, Debug)]
pub enum ContextError {
    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error(transparent)]
    DeltaTableError(#[from] deltalake::DeltaTableError),

    #[error(transparent)]
    FormatError(#[from] FormatError),

    #[error(transparent)]
    OpendalError(#[from] opendal::Error),

    #[error(transparent)]
    OptionsError(#[from] supabase_wrappers::options::OptionsError),

    #[error(transparent)]
    LogicalPlanError(#[from] LogicalPlanError),

    #[error(transparent)]
    ObjectStoreError(#[from] deltalake::ObjectStoreError),

    #[error(transparent)]
    SchemaError(#[from] SchemaError),

    #[error(transparent)]
    TableProviderError(#[from] TableProviderError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error(transparent)]
    SessionError(#[from] SessionError),

    #[error("No table registered with name {0}")]
    TableNotFound(String),

    #[error("Could not get definition for view {0}")]
    ViewNotFound(String),
}
