use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::catalog::CatalogProvider;
use deltalake::datafusion::common::arrow::datatypes::DataType;
use deltalake::datafusion::common::config::ConfigOptions;
use deltalake::datafusion::common::DataFusionError;
use deltalake::datafusion::datasource::provider_as_source;
use deltalake::datafusion::logical_expr::{AggregateUDF, ScalarUDF, TableSource, WindowUDF};
use deltalake::datafusion::sql::planner::ContextProvider;
use deltalake::datafusion::sql::TableReference;
use pgrx::*;
use std::ffi::{c_char, CStr};
use std::sync::Arc;

use crate::datafusion::session::Session;
use crate::errors::{NotFound, ParadeError};

pub struct QueryContext {
    options: ConfigOptions,
}

impl QueryContext {
    pub fn new() -> Result<Self, ParadeError> {
        Ok(Self {
            options: ConfigOptions::new(),
        })
    }

    fn get_table_source_impl(
        reference: TableReference,
    ) -> Result<Arc<dyn TableSource>, ParadeError> {
        let schema_name = reference.schema();

        if let Some(schema_name) = schema_name {
            // If a schema was provided in the query, i.e. SELECT * FROM <schema>.<table>
            let table_name = reference.table().to_string();
            Session::with_schema_provider(schema_name, |provider| {
                Box::pin(async move {
                    let table = provider
                        .table(&table_name)
                        .await
                        .ok_or(NotFound::Table(table_name))?;

                    Ok(provider_as_source(table))
                })
            })
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
                            Box::pin(async move { Ok(provider.table(&table_name).await.is_some()) })
                        })?;

                    if !table_registered {
                        continue;
                    }

                    let table_name = reference.table().to_string();
                    return Session::with_schema_provider(schema_name, |provider| {
                        Box::pin(async move {
                            let table = provider
                                .table(&table_name)
                                .await
                                .ok_or(NotFound::Table(table_name))?;

                            Ok(provider_as_source(table))
                        })
                    });
                }
            }

            Err(NotFound::Table(reference.table().to_string()).into())
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

    fn get_function_meta(&self, _name: &str) -> Option<Arc<ScalarUDF>> {
        None
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
