use async_std::task;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::common::arrow::datatypes::DataType;
use deltalake::datafusion::common::config::ConfigOptions;
use deltalake::datafusion::common::DataFusionError;
use deltalake::datafusion::datasource::provider_as_source;
use deltalake::datafusion::logical_expr::{AggregateUDF, ScalarUDF, TableSource, WindowUDF};
use deltalake::datafusion::prelude::SessionContext;
use deltalake::datafusion::sql::planner::ContextProvider;
use deltalake::datafusion::sql::TableReference;
use lazy_static::lazy_static;
use parking_lot::{RwLock, RwLockWriteGuard};
use std::sync::Arc;

use crate::datafusion::catalog::{ParadeCatalog, PARADE_CATALOG};
use crate::datafusion::schema::ParadeSchemaProvider;
use crate::errors::ParadeError;

lazy_static! {
    pub static ref CONTEXT: RwLock<Option<SessionContext>> = RwLock::new(None);
}

pub struct DatafusionContext;

impl<'a> DatafusionContext {
    pub fn with_session_context<F, R>(f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(&SessionContext) -> Result<R, ParadeError>,
    {
        let context_lock = CONTEXT.read();
        let context = match context_lock.as_ref() {
            Some(context) => context,
            None => {
                return Err(ParadeError::ContextNotInitialized(
                    "Please run `CALL paradedb.init();` first".to_string(),
                ))
            }
        };

        f(context)
    }

    pub fn with_schema_provider<F, R>(schema_name: &str, f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(&ParadeSchemaProvider) -> Result<R, ParadeError>,
    {
        let context_lock = CONTEXT.read();
        let context = match context_lock.as_ref() {
            Some(context) => context,
            None => {
                return Err(ParadeError::ContextNotInitialized(
                    "Please run `CALL paradedb.init();` first".to_string(),
                ))
            }
        };

        let schema_provider = context
            .catalog(PARADE_CATALOG)
            .ok_or_else(|| ParadeError::NotFound)?
            .schema(schema_name)
            .ok_or_else(|| ParadeError::NotFound)?;

        let parade_provider = schema_provider
            .as_any()
            .downcast_ref::<ParadeSchemaProvider>()
            .ok_or_else(|| ParadeError::NotFound)?;

        f(parade_provider)
    }

    pub fn with_catalog<F, R>(f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(&ParadeCatalog) -> Result<R, ParadeError>,
    {
        let context_lock = CONTEXT.read();
        let context = match context_lock.as_ref() {
            Some(context) => context,
            None => {
                return Err(ParadeError::ContextNotInitialized(
                    "Please run `CALL paradedb.init();` first".to_string(),
                ))
            }
        };

        let catalog_provider = context
            .catalog(PARADE_CATALOG)
            .ok_or_else(|| ParadeError::NotFound)?;

        let parade_catalog = catalog_provider
            .as_any()
            .downcast_ref::<ParadeCatalog>()
            .ok_or_else(|| ParadeError::NotFound)?;

        f(parade_catalog)
    }

    pub fn with_write_lock<F, R>(f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(RwLockWriteGuard<'a, Option<SessionContext>>) -> Result<R, ParadeError>,
    {
        let context_lock = CONTEXT.write();
        f(context_lock)
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
}

impl ContextProvider for ParadeContextProvider {
    fn get_table_provider(
        &self,
        reference: TableReference,
    ) -> Result<Arc<dyn TableSource>, DataFusionError> {
        let table_name = reference.table();
        let schema_name = reference.schema().unwrap_or("public");

        DatafusionContext::with_schema_provider(schema_name, |provider| {
            let table =
                task::block_on(provider.table(table_name)).ok_or_else(|| ParadeError::NotFound)?;

            Ok(provider_as_source(table))
        })
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
