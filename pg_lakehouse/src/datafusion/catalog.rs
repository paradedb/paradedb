use async_std::sync::RwLock;
use async_std::task;
use datafusion::catalog::schema::SchemaProvider;
use datafusion::catalog::{CatalogProvider, CatalogProviderList};
use datafusion::common::DataFusionError;
use pgrx::*;
use std::{any::Any, collections::HashMap, sync::Arc};
use supabase_wrappers::prelude::OptionsError;
use thiserror::Error;

use crate::schema::attribute::SchemaError;

use super::provider::TableProviderError;

pub struct ParadeCatalog {
    schemas: RwLock<HashMap<String, Arc<dyn SchemaProvider>>>,
}

pub struct ParadeCatalogList {
    catalogs: RwLock<HashMap<String, Arc<dyn CatalogProvider>>>,
}

impl ParadeCatalog {
    pub fn try_new() -> Result<Self, CatalogError> {
        Ok(Self {
            schemas: RwLock::new(HashMap::new()),
        })
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
        let mut schema_map = task::block_on(self.schemas.write());
        schema_map.insert(name.to_owned(), schema.clone());
        Ok(Some(schema))
    }

    fn schema_names(&self) -> Vec<String> {
        let schemas = task::block_on(self.schemas.read());
        schemas.keys().cloned().collect()
    }

    fn schema(&self, name: &str) -> Option<Arc<dyn SchemaProvider>> {
        let schemas = task::block_on(self.schemas.read());
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
    pub fn try_new() -> Result<Self, CatalogError> {
        Ok(Self {
            catalogs: RwLock::new(HashMap::new()),
        })
    }
}

impl CatalogProviderList for ParadeCatalogList {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn register_catalog(
        &self,
        name: String,
        catalog: Arc<dyn CatalogProvider>,
    ) -> Option<Arc<dyn CatalogProvider>> {
        let mut catalog_map = task::block_on(self.catalogs.write());
        catalog_map.insert(name, catalog.clone());
        Some(catalog)
    }

    fn catalog_names(&self) -> Vec<String> {
        let catalog_map = task::block_on(self.catalogs.read());
        catalog_map.keys().cloned().collect()
    }

    fn catalog(&self, name: &str) -> Option<Arc<dyn CatalogProvider>> {
        let catalog_map = task::block_on(self.catalogs.read());
        catalog_map.get(name).cloned()
    }
}

#[derive(Error, Debug)]
pub enum CatalogError {
    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    NulError(#[from] std::ffi::NulError),

    #[error(transparent)]
    OptionsError(#[from] OptionsError),

    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error(transparent)]
    SchemaError(#[from] SchemaError),

    #[error(transparent)]
    TableProviderError(#[from] TableProviderError),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Catalog {0} not found")]
    CatalogNotFound(String),

    #[error("Catalog provider {0} not found")]
    CatalogProviderNotFound(String),

    #[error("Database {0} not found")]
    DatabaseNotFound(String),

    #[error("Schema {0} not found")]
    SchemaNotFound(String),

    #[error("Schema provider {0} not found")]
    SchemaProviderNotFound(String),
}
