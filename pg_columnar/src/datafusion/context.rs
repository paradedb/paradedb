use datafusion::common::arrow::datatypes::DataType;
use datafusion::common::config::ConfigOptions;
use datafusion::common::{plan_err, DataFusionError};
use datafusion::datasource::provider_as_source;
use datafusion::logical_expr::{AggregateUDF, ScalarUDF, TableSource, WindowUDF};
use datafusion::prelude::SessionContext;
use datafusion::sql::planner::ContextProvider;
use datafusion::sql::TableReference;
use lazy_static::lazy_static;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::collections::HashMap;

use std::sync::Arc;

use crate::datafusion::registry::{PARADE_CATALOG, PARADE_SCHEMA};
use crate::datafusion::schema::ParadeSchemaProvider;

lazy_static! {
    pub static ref CONTEXT: RwLock<Option<SessionContext>> = RwLock::new(None);
}

pub struct DatafusionContext;

impl<'a> DatafusionContext {
    pub fn with_read<F, R>(f: F) -> R
    where
        F: FnOnce(&SessionContext) -> R,
    {
        let context_lock = CONTEXT.read();
        let context = context_lock
            .as_ref()
            .ok_or("Run SELECT paradedb.init(); first.")
            .expect("No columnar context found");
        f(context)
    }

    #[allow(dead_code)]
    pub fn with_write<F, R>(f: F) -> R
    where
        F: FnOnce(&mut SessionContext) -> R,
    {
        let mut context_lock = CONTEXT.write();
        let context = context_lock
            .as_mut()
            .ok_or("Run SELECT paradedb.init(); first.")
            .expect("No columnar context found");
        f(context)
    }

    #[allow(dead_code)]
    pub fn read_lock() -> Result<RwLockReadGuard<'a, Option<SessionContext>>, String> {
        Ok(CONTEXT.read())
    }

    pub fn write_lock() -> Result<RwLockWriteGuard<'a, Option<SessionContext>>, String> {
        Ok(CONTEXT.write())
    }
}

pub struct ParadeContextProvider {
    options: ConfigOptions,
    tables: HashMap<String, Arc<dyn TableSource>>,
}

impl ParadeContextProvider {
    pub fn new() -> Self {
        DatafusionContext::with_read(|context| {
            let schema_provider = context
                .catalog(PARADE_CATALOG)
                .expect("Catalog not found")
                .schema(PARADE_SCHEMA)
                .expect("Schema not found");

            let lister = schema_provider
                .as_any()
                .downcast_ref::<ParadeSchemaProvider>()
                .expect("Failed to downcast schema provider");

            let table_providers = lister.tables().expect("Failed to get tables");
            let mut tables = HashMap::new();

            for (table_oid, provider) in table_providers.iter() {
                tables.insert(table_oid.to_string(), provider_as_source(provider.clone()));
            }

            Self {
                options: ConfigOptions::new(),
                tables,
            }
        })
    }
}

impl ContextProvider for ParadeContextProvider {
    fn get_table_source(
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
