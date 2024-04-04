use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::catalog::CatalogProvider;
use deltalake::datafusion::common::arrow::datatypes::DataType;
use deltalake::datafusion::common::config::ConfigOptions;
use deltalake::datafusion::common::DataFusionError;

use deltalake::datafusion::datasource::provider_as_source;
use deltalake::datafusion::execution::FunctionRegistry;
use deltalake::datafusion::logical_expr::{AggregateUDF, ScalarUDF, TableSource, WindowUDF};
use deltalake::datafusion::sql::planner::ContextProvider;
use deltalake::datafusion::sql::TableReference;
use pgrx::*;
use std::ffi::{c_char, CStr};
use std::sync::Arc;

use super::catalog::CatalogError;
use super::directory::ParadeDirectory;
use super::session::Session;
use super::table::PgTableProvider;

pub struct QueryContext {
    options: ConfigOptions,
}

impl QueryContext {
    pub fn new() -> Result<Self, CatalogError> {
        Ok(Self {
            options: ConfigOptions::new(),
        })
    }

    fn get_table_source_impl(
        reference: TableReference,
    ) -> Result<Arc<dyn TableSource>, CatalogError> {
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
                    let schema_registered =
                        Session::with_catalog(|catalog| Ok(catalog.schema(schema_name).is_some()))?;

                    if !schema_registered {
                        continue;
                    }

                    let table_name = reference.table().to_string();
                    let table_registered =
                        Session::with_schema_provider(schema_name, |provider| {
                            Box::pin(async move { Ok(provider.table_exist(&table_name)) })
                        })?;

                    if !table_registered {
                        continue;
                    }

                    return get_source(schema_name, reference.table());
                }
            }

            Err(CatalogError::TableNotFound(reference.table().to_string()))
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
}

#[inline]
fn get_source(schema_name: &str, table_name: &str) -> Result<Arc<dyn TableSource>, CatalogError> {
    let schema_name = schema_name.to_string();
    let table_name = table_name.to_string();

    Session::with_tables(&schema_name.clone(), |mut tables| {
        Box::pin(async move {
            let table_path = ParadeDirectory::table_path_from_name(&schema_name, &table_name)?;
            let delta_table = tables.get_ref(&table_path).await?;
            let provider =
                PgTableProvider::new(delta_table.clone(), &schema_name, &table_name).await?;

            Ok(provider_as_source(Arc::new(provider)))
        })
    })
}
