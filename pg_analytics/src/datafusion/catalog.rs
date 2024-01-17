use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::catalog::{CatalogList, CatalogProvider};
use deltalake::datafusion::common::DataFusionError;
use parking_lot::RwLock;
use pgrx::*;
use std::{any::Any, collections::HashMap, ffi::CStr, sync::Arc};

use crate::datafusion::directory::ParadeDirectory;
use crate::datafusion::schema::ParadeSchemaProvider;
use crate::errors::ParadeError;

pub static PARADE_CATALOG: &str = "datafusion";

pub struct ParadeCatalog {
    schemas: RwLock<HashMap<String, Arc<dyn SchemaProvider>>>,
}

pub struct ParadeCatalogList {
    catalogs: RwLock<HashMap<String, Arc<dyn CatalogProvider>>>,
}

impl ParadeCatalog {
    pub fn try_new() -> Result<Self, ParadeError> {
        Ok(Self {
            schemas: RwLock::new(HashMap::new()),
        })
    }

    pub async fn init(&self) -> Result<(), ParadeError> {
        let delta_dir = ParadeDirectory::delta_path()?;

        for entry in std::fs::read_dir(delta_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let schema_oid = path
                    .file_name()
                    .ok_or_else(|| ParadeError::NotFound)?
                    .to_str()
                    .ok_or_else(|| ParadeError::NotFound)?
                    .parse()?;

                let pg_oid = unsafe { pg_sys::Oid::from_u32_unchecked(schema_oid) };

                let schema_name = unsafe {
                    let schema_name = pg_sys::get_namespace_name(pg_oid);
                    if schema_name.is_null() {
                        continue;
                    }

                    CStr::from_ptr(schema_name).to_str()?.to_owned()
                };

                let schema_provider = Arc::new(
                    ParadeSchemaProvider::try_new(
                        schema_name.as_str(),
                        ParadeDirectory::schema_path(pg_oid)?,
                    )
                    .await?,
                );

                schema_provider.init().await?;

                Self::register_schema(self, schema_name.as_str(), schema_provider)?;
            }
        }

        Ok(())
    }
}

impl CatalogProvider for ParadeCatalog {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn register_schema(
        &self,
        name: &str,
        schema: Arc<dyn SchemaProvider>,
    ) -> Result<Option<Arc<dyn SchemaProvider>>, DataFusionError> {
        let mut schema_map = self.schemas.write();
        schema_map.insert(name.to_owned(), schema.clone());
        Ok(Some(schema))
    }

    fn schema_names(&self) -> Vec<String> {
        let schemas = self.schemas.read();
        schemas.keys().cloned().collect()
    }

    fn schema(&self, name: &str) -> Option<Arc<dyn SchemaProvider>> {
        let schemas = self.schemas.read();
        let maybe_schema = schemas.get(name);
        if let Some(schema) = maybe_schema {
            let schema = schema.clone() as Arc<dyn SchemaProvider>;
            Some(schema)
        } else {
            None
        }
    }
}

impl ParadeCatalogList {
    pub fn try_new() -> Result<Self, ParadeError> {
        Ok(Self {
            catalogs: RwLock::new(HashMap::new()),
        })
    }
}

impl CatalogList for ParadeCatalogList {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn register_catalog(
        &self,
        name: String,
        catalog: Arc<dyn CatalogProvider>,
    ) -> Option<Arc<dyn CatalogProvider>> {
        let mut cats = self.catalogs.write();
        cats.insert(name, catalog.clone());
        Some(catalog)
    }

    fn catalog_names(&self) -> Vec<String> {
        let cats = self.catalogs.read();
        cats.keys().cloned().collect()
    }

    fn catalog(&self, name: &str) -> Option<Arc<dyn CatalogProvider>> {
        let cats = self.catalogs.read();
        cats.get(name).cloned()
    }
}
