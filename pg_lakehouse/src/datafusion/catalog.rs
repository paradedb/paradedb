// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

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

use super::context::ContextError;
use super::provider::TableProviderError;
use super::session::SessionError;

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
    ContextError(#[from] ContextError),

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
    SessionError(#[from] SessionError),

    #[error(transparent)]
    TableProviderError(#[from] TableProviderError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Unexpected error: Failed to downcast table provider to Delta table")]
    DowncastDeltaTable,
}
