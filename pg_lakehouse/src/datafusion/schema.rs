use async_std::sync::Mutex;
use async_std::task;
use async_trait::async_trait;
use datafusion::catalog::schema::SchemaProvider;
use datafusion::datasource::TableProvider;
use datafusion::error::Result;
use pgrx::*;
use std::any::Any;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::sync::Arc;
use supabase_wrappers::prelude::*;

use crate::fdw::handler::*;
use crate::fdw::options::*;
use crate::schema::attribute::*;

use super::catalog::CatalogError;
use super::format::*;
use super::provider::*;

#[derive(Clone)]
pub struct LakehouseSchemaProvider {
    schema_name: String,
    tables: Arc<Mutex<HashMap<pg_sys::Oid, Arc<dyn TableProvider>>>>,
}

impl LakehouseSchemaProvider {
    pub fn new(schema_name: &str) -> Self {
        Self {
            schema_name: schema_name.to_string(),
            tables: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn table_impl(&self, table_name: &str) -> Result<Arc<dyn TableProvider>, CatalogError> {
        let pg_relation = unsafe {
            PgRelation::open_with_name(table_name).unwrap_or_else(|err| {
                panic!("{:?}", err);
            })
        };

        let mut tables = self.tables.lock().await;

        let table = match tables.entry(pg_relation.oid()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => {
                let table_options = pg_relation.table_options()?;
                let path = require_option(TableOption::Path.as_str(), &table_options)?;
                let extension = require_option(TableOption::Extension.as_str(), &table_options)?;
                let format = require_option_or(TableOption::Format.as_str(), &table_options, "");

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

                let provider = match TableFormat::from(format) {
                    TableFormat::None => create_listing_provider(path, extension).await?,
                    TableFormat::Delta => create_delta_provider(path, extension).await?,
                };

                for (index, field) in provider.schema().fields().iter().enumerate() {
                    if let Some(attribute) = attribute_map.remove(&index) {
                        can_convert_to_attribute(field, attribute)?;
                    }
                }

                entry.insert(provider)
            }
        };

        Ok(table.clone())
    }
}

#[async_trait]
impl SchemaProvider for LakehouseSchemaProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    // This function never gets called anywhere, so it's safe to leave unimplemented
    fn table_names(&self) -> Vec<String> {
        todo!("table_names not implemented")
    }

    async fn table(&self, table_name: &str) -> Result<Option<Arc<dyn TableProvider>>> {
        let table =
            task::block_on(self.table_impl(table_name)).unwrap_or_else(|err| panic!("{:?}", err));

        Ok(Some(table))
    }

    fn table_exist(&self, table_name: &str) -> bool {
        let pg_relation = match unsafe {
            PgRelation::open_with_name(format!("{}.{}", self.schema_name, table_name).as_str())
        } {
            Ok(relation) => relation,
            Err(_) => return false,
        };

        if !pg_relation.is_foreign_table() {
            return false;
        }

        let foreign_table = unsafe { pg_sys::GetForeignTable(pg_relation.oid()) };
        let foreign_server = unsafe { pg_sys::GetForeignServer((*foreign_table).serverid) };
        let fdw_handler = FdwHandler::from(foreign_server);

        fdw_handler != FdwHandler::Other
    }
}
