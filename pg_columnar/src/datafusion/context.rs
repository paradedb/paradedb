use async_std::task;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::common::arrow::datatypes::DataType;
use deltalake::datafusion::common::config::ConfigOptions;
use deltalake::datafusion::common::{plan_err, DataFusionError};
use deltalake::datafusion::datasource::provider_as_source;
use deltalake::datafusion::logical_expr::{AggregateUDF, ScalarUDF, TableSource, WindowUDF};
use deltalake::datafusion::prelude::SessionContext;
use deltalake::datafusion::sql::planner::ContextProvider;
use deltalake::datafusion::sql::TableReference;
use lazy_static::lazy_static;
use parking_lot::{RwLock, RwLockWriteGuard};
use std::collections::HashMap;
use std::sync::Arc;

use crate::datafusion::catalog::PARADE_CATALOG;
use crate::datafusion::schema::{ParadeSchemaProvider, PARADE_SCHEMA};
use crate::errors::ParadeError;

lazy_static! {
    pub static ref CONTEXT: RwLock<Option<SessionContext>> = RwLock::new(None);
}

pub struct DatafusionContext;

impl<'a> DatafusionContext {
    pub fn with_provider_context<F, R>(f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(&ParadeSchemaProvider, &SessionContext) -> R,
    {
        let context_lock = CONTEXT.read();
        let context = context_lock
            .as_ref()
            .ok_or_else(|| ParadeError::ContextNotInitialized)?;

        let schema_provider = context
            .catalog(PARADE_CATALOG)
            .ok_or_else(|| ParadeError::NotFound)?
            .schema(PARADE_SCHEMA)
            .ok_or_else(|| ParadeError::NotFound)?;

        let parade_provider = schema_provider
            .as_any()
            .downcast_ref::<ParadeSchemaProvider>()
            .ok_or_else(|| ParadeError::NotFound)?;

        Ok(f(parade_provider, context))
    }

    pub fn with_write_lock<F, R>(f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(RwLockWriteGuard<'a, Option<SessionContext>>) -> R,
    {
        let context_lock = CONTEXT.write();
        Ok(f(context_lock))
    }
}

pub struct ParadeContextProvider {
    options: ConfigOptions,
    tables: HashMap<String, Arc<dyn TableSource>>,
}

impl ParadeContextProvider {
    pub fn new() -> Result<Self, ParadeError> {
        DatafusionContext::with_provider_context(|provider, _| {
            let table_names = provider.table_names();
            let mut tables = HashMap::new();

            for table_name in table_names.iter() {
                let table_provider = task::block_on(provider.table(table_name))
                    .ok_or_else(|| ParadeError::NotFound)?;
                tables.insert(table_name.to_string(), provider_as_source(table_provider));
            }

            Ok(Self {
                options: ConfigOptions::new(),
                tables,
            })
        })?
    }
}

impl ContextProvider for ParadeContextProvider {
    fn get_table_provider(
        &self,
        name: TableReference,
    ) -> Result<Arc<dyn TableSource>, DataFusionError> {
        match self.tables.get(name.table()) {
            Some(table) => Ok(table.clone()),
            _ => plan_err!("Table not found: {}", name.table()),
        }
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
