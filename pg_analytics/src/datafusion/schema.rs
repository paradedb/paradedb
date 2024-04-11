use async_std::sync::Mutex;
use async_trait::async_trait;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::datasource::TableProvider;
use deltalake::datafusion::error::Result;
use std::any::Any;
use std::path::Path;
use std::sync::Arc;

use super::catalog::CatalogError;
use super::directory::ParadeDirectory;
use super::table::{PgTableProvider, Tables};

pub struct ParadeSchemaProvider {
    schema_name: String,
    tables: Arc<Mutex<Tables>>,
}

impl ParadeSchemaProvider {
    pub async fn try_new(schema_name: &str) -> Result<Self, CatalogError> {
        Ok(Self {
            schema_name: schema_name.to_string(),
            tables: Arc::new(Mutex::new(Tables::new(schema_name)?)),
        })
    }

    pub fn tables(&self) -> Result<Arc<Mutex<Tables>>, CatalogError> {
        Ok(self.tables.clone())
    }
}

#[async_trait]
impl SchemaProvider for ParadeSchemaProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    // This function never gets called anywhere, so it's safe to leave unimplemented
    fn table_names(&self) -> Vec<String> {
        todo!("table_names not implemented")
    }

    async fn table(&self, table_name: &str) -> Option<Arc<dyn TableProvider>> {
        let tables = Self::tables(self).expect("Failed to get tables");
        let table_path = ParadeDirectory::table_path_from_name(&self.schema_name, table_name)
            .unwrap_or_else(|err| {
                panic!("{}", err);
            });

        Some(
            table_impl(tables, &table_path, &self.schema_name, table_name)
                .await
                .unwrap_or_else(|err| {
                    panic!("{}", err);
                }),
        )
    }

    fn table_exist(&self, table_name: &str) -> bool {
        ParadeDirectory::table_path_from_name(&self.schema_name, table_name).is_ok()
    }
}

#[inline]
async fn table_impl(
    tables: Arc<Mutex<Tables>>,
    table_path: &Path,
    schema_name: &str,
    table_name: &str,
) -> Result<Arc<dyn TableProvider>, CatalogError> {
    let mut tables = tables.lock().await;
    let delta_table = tables.get_ref(table_path).await?.clone();

    Ok(
        Arc::new(PgTableProvider::new(delta_table, schema_name, table_name).await?)
            as Arc<dyn TableProvider>,
    )
}
