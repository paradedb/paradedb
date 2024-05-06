use async_std::task;
use datafusion::common::arrow::datatypes::DataType;
use datafusion::common::config::ConfigOptions;
use datafusion::common::DataFusionError;
use datafusion::datasource::{provider_as_source, view::ViewTable};
use datafusion::execution::FunctionRegistry;
use datafusion::logical_expr::{AggregateUDF, LogicalPlan, ScalarUDF, TableSource, WindowUDF};
use datafusion::sql::planner::ContextProvider;
use datafusion::sql::TableReference;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use supabase_wrappers::prelude::*;
use thiserror::Error;

use crate::fdw::format::*;
use crate::fdw::handler::*;
use crate::fdw::object_store::*;
use crate::fdw::options::*;
use crate::schema::attribute::*;

use super::plan::*;
use super::provider::*;
use super::session::Session;

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
        Session::with_session_context(|context| {
            let context_res = context.udf(name);
            Box::pin(async move { Ok(context_res?) })
        })
        .ok()
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

#[inline]
async fn get_table_source(
    reference: TableReference<'_>,
) -> Result<Arc<dyn TableSource>, ContextError> {
    let schema_name = reference.schema();

    // If no table was found, try to register it as a view
    let pg_relation = (match schema_name {
        None => unsafe { PgRelation::open_with_name(reference.table()) },
        Some(schema_name) => unsafe {
            PgRelation::open_with_name(format!("{}.{}", schema_name, reference.table()).as_str())
        },
    })
    .map_err(|_| ContextError::TableNotFound(reference.table().to_string()))?;

    if pg_relation.is_view() {
        let view_definition = unsafe {
            direct_function_call::<String>(
                pg_sys::pg_get_viewdef,
                &[Some(pg_sys::Datum::from(pg_relation.oid()))],
            )
            .ok_or(ContextError::ViewNotFound(reference.table().to_string()))?
        };

        let logical_plan = LogicalPlan::try_from(QueryString(&view_definition))?;
        let view_table = ViewTable::try_new(logical_plan, None)?;
        return Ok(provider_as_source(Arc::new(view_table)));
    }

    if pg_relation.is_foreign_table() {
        let foreign_table = unsafe { pg_sys::GetForeignTable(pg_relation.oid()) };
        let foreign_server = unsafe { pg_sys::GetForeignServer((*foreign_table).serverid) };
        let fdw_handler = unsafe { FdwHandler::from((*foreign_server).fdwid) };

        if fdw_handler != FdwHandler::Other {
            // Get foreign table and server options
            let table_options = unsafe { options_to_hashmap((*foreign_table).options)? };

            // Create provider for foreign table
            let mut attribute_map: HashMap<usize, PgAttribute> = pg_relation
                .tuple_desc()
                .iter()
                .enumerate()
                .map(|(index, attribute)| {
                    (
                        index,
                        PgAttribute::new(attribute.name(), attribute.atttypid),
                    )
                })
                .collect();

            let format = require_option_or(TableOption::Format.as_str(), &table_options, "");
            let path = require_option(TableOption::Path.as_str(), &table_options)?.to_string();
            let extension =
                require_option(TableOption::Extension.as_str(), &table_options)?.to_string();

            let provider = match TableFormat::from(format) {
                TableFormat::None => Session::with_session_context(|context| {
                    Box::pin(async move {
                        Ok(create_listing_provider(&path, &extension, &context.state()).await?)
                    })
                })?,
                TableFormat::Delta => create_delta_provider(&path, &extension).await?,
            };

            for (index, field) in provider.schema().fields().iter().enumerate() {
                if let Some(attribute) = attribute_map.remove(&index) {
                    can_convert_to_attribute(field, attribute)?;
                }
            }

            return Ok(provider_as_source(provider));
        }
    }

    Err(ContextError::TableNotFound(reference.table().to_string()))
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
    ObjectStoreError(#[from] ObjectStoreError),

    #[error(transparent)]
    OptionsError(#[from] supabase_wrappers::options::OptionsError),

    #[error(transparent)]
    LogicalPlanError(#[from] LogicalPlanError),

    #[error(transparent)]
    SchemaError(#[from] SchemaError),

    #[error(transparent)]
    TableProviderError(#[from] TableProviderError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("No table registered with name {0}")]
    TableNotFound(String),

    #[error("Could not get definition for view {0}")]
    ViewNotFound(String),
}
