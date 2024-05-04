use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::catalog::CatalogProvider;
use deltalake::datafusion::common::arrow::datatypes::DataType;
use deltalake::datafusion::common::config::ConfigOptions;
use deltalake::datafusion::common::DataFusionError;
use deltalake::datafusion::datasource::{provider_as_source, view::ViewTable};
use deltalake::datafusion::execution::FunctionRegistry;
use deltalake::datafusion::logical_expr::{AggregateUDF, ScalarUDF, TableSource, WindowUDF};
use deltalake::datafusion::sql::planner::ContextProvider;
use deltalake::datafusion::sql::TableReference;
use pgrx::*;
use std::ffi::{c_char, CStr};
use std::sync::Arc;
use thiserror::Error;

use super::catalog::CatalogError;
use super::directory::ParadeDirectory;
use super::plan::LogicalPlanDetails;
use super::query::QueryString;
use super::schema::ParadeSchemaProvider;
use super::session::Session;
use super::table::PgTableProvider;

pub struct QueryContext {
    options: ConfigOptions,
}

impl QueryContext {
    pub fn new() -> Result<Self, ContextError> {
        Ok(Self {
            options: ConfigOptions::new(),
        })
    }

    fn get_table_source_impl(
        reference: TableReference,
    ) -> Result<Arc<dyn TableSource>, ContextError> {
        let schema_name = reference.schema();

        if let Some(schema_name) = schema_name {
            // If a schema was provided in the query, i.e. SELECT * FROM <schema>.<table>
            get_source(schema_name, reference.table())
        } else {
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

                    Session::with_catalog(|catalog| {
                        Box::pin(async move {
                            if catalog.schema(schema_name).is_none() {
                                let new_schema_provider =
                                    Arc::new(ParadeSchemaProvider::try_new(schema_name).await?);
                                catalog.register_schema(schema_name, new_schema_provider)?;
                            }

                            Ok(())
                        })
                    })?;

                    let table_name = reference.table().to_string();
                    let table_exists = Session::with_schema_provider(schema_name, |provider| {
                        Box::pin(async move { Ok(provider.table_exist(&table_name)) })
                    })?;

                    if !table_exists {
                        continue;
                    }

                    return get_source(schema_name, reference.table());
                }
            }

            // If no table was found, try to register it as a view
            let pg_relation = (match schema_name {
                None => unsafe { PgRelation::open_with_name(reference.table()) },
                Some(schema_name) => unsafe {
                    PgRelation::open_with_name(
                        format!("{}.{}", schema_name, reference.table()).as_str(),
                    )
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

                let plan = LogicalPlanDetails::try_from(QueryString(&view_definition))
                    .map_err(|_| ContextError::ViewParseError)?;
                let view_table = ViewTable::try_new(plan.logical_plan(), None)?;
                return Ok(provider_as_source(Arc::new(view_table)));
            }

            Err(ContextError::TableNotFound(reference.table().to_string()))
        }
    }
}

impl ContextProvider for QueryContext {
    fn get_table_source(
        &self,
        reference: TableReference,
    ) -> Result<Arc<dyn TableSource>, DataFusionError> {
        Self::get_table_source_impl(reference)
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
fn get_source(schema_name: &str, table_name: &str) -> Result<Arc<dyn TableSource>, ContextError> {
    let schema_name = schema_name.to_string();
    let table_name = table_name.to_string();

    Ok(Session::with_tables(&schema_name.clone(), |mut tables| {
        Box::pin(async move {
            let table_path =
                ParadeDirectory::table_path_from_name(&schema_name.clone(), &table_name)?;
            let delta_table = tables.get_ref(&table_path).await?;
            let provider =
                PgTableProvider::new(delta_table.clone(), &schema_name, &table_name).await?;

            Ok(provider_as_source(Arc::new(provider)))
        })
    })?)
}

#[derive(Error, Debug)]
pub enum ContextError {
    #[error(transparent)]
    CatalogError(#[from] CatalogError),

    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("No table registered with name {0}")]
    TableNotFound(String),

    #[error("Could not get definition for view {0}")]
    ViewNotFound(String),

    #[error("Could not parse view definition")]
    ViewParseError,
}
