use async_std::sync::RwLock;
use datafusion::catalog::schema::SchemaProvider;
use datafusion::catalog::{CatalogProvider, CatalogProviderList};
use datafusion::common::DataFusionError;
use pgrx::*;
use shared::block_on;
use std::{any::Any, collections::HashMap, sync::Arc};
use supabase_wrappers::prelude::OptionsError;
use thiserror::Error;

use crate::schema::attribute::SchemaError;

use super::provider::TableProviderError;

#[derive(Clone)]
pub struct LakehouseCatalog {
    schemas: Arc<RwLock<HashMap<String, Arc<dyn SchemaProvider>>>>,
}

#[derive(Clone)]
pub struct LakehouseCatalogList {
    catalogs: Arc<RwLock<HashMap<String, Arc<dyn CatalogProvider>>>>,
}

impl LakehouseCatalog {
    pub fn new() -> Self {
        Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl CatalogProvider for LakehouseCatalog {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn register_schema(
        &self,
        name: &str,
        schema: Arc<dyn SchemaProvider>,
    ) -> Result<Option<Arc<dyn SchemaProvider>>, DataFusionError> {
        let mut schema_map = block_on!(self.schemas.write());
        schema_map.insert(name.to_owned(), schema.clone());
        Ok(Some(schema))
    }

    fn schema_names(&self) -> Vec<String> {
        let schemas = block_on!(self.schemas.read());
        schemas.keys().cloned().collect()
    }

    fn schema(&self, name: &str) -> Option<Arc<dyn SchemaProvider>> {
        let schemas = block_on!(self.schemas.read());
        match schemas.get(name) {
            Some(schema) => Some(schema.clone() as Arc<dyn SchemaProvider>),
            None => None,
        }
    }
}

impl LakehouseCatalogList {
    pub fn new() -> Self {
        Self {
            catalogs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl CatalogProviderList for LakehouseCatalogList {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn register_catalog(
        &self,
        name: String,
        catalog: Arc<dyn CatalogProvider>,
    ) -> Option<Arc<dyn CatalogProvider>> {
        let mut catalog_map = block_on!(self.catalogs.write());
        catalog_map.insert(name, catalog.clone());
        Some(catalog)
    }

    fn catalog_names(&self) -> Vec<String> {
        let catalog_map = block_on!(self.catalogs.read());
        catalog_map.keys().cloned().collect()
    }

    fn catalog(&self, name: &str) -> Option<Arc<dyn CatalogProvider>> {
        let catalog_map = block_on!(self.catalogs.read());
        catalog_map.get(name).cloned()
    }
}

#[derive(Error, Debug)]
pub enum CatalogError {
    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error(transparent)]
    DeltaTableError(#[from] deltalake::DeltaTableError),

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

    #[allow(unused)]
    #[error("Unexpected error: Failed to downcast table provider to Delta table")]
    DowncastDeltaTable,
}
