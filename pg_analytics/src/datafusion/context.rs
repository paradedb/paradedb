use async_std::task;
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
use std::ffi::{CStr, CString};
use std::sync::Arc;

use crate::datafusion::session::ParadeSessionContext;
use crate::errors::{NotFound, ParadeError};

trait PostgresSchema {
    fn is_temp_schema(&self) -> Result<bool, ParadeError>;
}

impl PostgresSchema for str {
    fn is_temp_schema(&self) -> Result<bool, ParadeError> {
        let c_schema_name = CString::new(self)?;
        let schema_oid = unsafe { pg_sys::get_namespace_oid(c_schema_name.as_ptr(), false) };
        Ok(unsafe { pg_sys::isTempNamespace(schema_oid) })
    }
}

pub struct ParadeContextProvider {
    options: ConfigOptions,
}

impl ParadeContextProvider {
    pub fn new() -> Result<Self, ParadeError> {
        Ok(Self {
            options: ConfigOptions::new(),
        })
    }

    fn get_table_source_impl(
        reference: TableReference,
    ) -> Result<Arc<dyn TableSource>, ParadeError> {
        let table_name = reference.table();

        match reference.schema() {
            Some(name) => match name.is_temp_schema()? {
                true => ParadeSessionContext::with_temp_schema_provider(name, |provider| {
                    let table = task::block_on(provider.table(table_name))
                        .ok_or(NotFound::Table(table_name.to_string()))?;

                    Ok(provider_as_source(table))
                }),
                false => ParadeSessionContext::with_permanent_schema_provider(name, |provider| {
                    let table = task::block_on(provider.table(table_name))
                        .ok_or(NotFound::Table(table_name.to_string()))?;

                    Ok(provider_as_source(table))
                }),
            },
            None => {
                let current_schemas = unsafe {
                    direct_function_call::<Array<pg_sys::Datum>>(
                        pg_sys::current_schemas,
                        &[Some(pg_sys::Datum::from(true))],
                    )
                };

                if let Some(current_schemas) = current_schemas {
                    for datum in current_schemas.iter().flatten() {
                        let schema_name =
                            unsafe { CStr::from_ptr(datum.cast_mut_ptr::<i8>()).to_str()? };
                        let schema_registered =
                            ParadeSessionContext::with_postgres_catalog(|catalog| {
                                Ok(catalog.schema(schema_name).is_some())
                            })?;

                        if !schema_registered {
                            continue;
                        }

                        let table_registered = match schema_name.is_temp_schema()? {
                            true => ParadeSessionContext::with_temp_schema_provider(
                                schema_name,
                                |provider| Ok(task::block_on(provider.table(table_name)).is_some()),
                            ),
                            false => ParadeSessionContext::with_permanent_schema_provider(
                                schema_name,
                                |provider| Ok(task::block_on(provider.table(table_name)).is_some()),
                            ),
                        }?;

                        if !table_registered {
                            continue;
                        }

                        return match schema_name.is_temp_schema()? {
                            true => ParadeSessionContext::with_temp_schema_provider(
                                schema_name,
                                |provider| {
                                    let table = task::block_on(provider.table(table_name))
                                        .ok_or(NotFound::Table(table_name.to_string()))?;

                                    Ok(provider_as_source(table))
                                },
                            ),
                            false => ParadeSessionContext::with_permanent_schema_provider(
                                schema_name,
                                |provider| {
                                    let table = task::block_on(provider.table(table_name))
                                        .ok_or(NotFound::Table(table_name.to_string()))?;

                                    Ok(provider_as_source(table))
                                },
                            ),
                        };
                    }
                }

                Err(NotFound::Table(table_name.to_string()).into())
            }
        }
    }
}

impl ContextProvider for ParadeContextProvider {
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
