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

use dashmap::DashSet;
use datafusion::common::DataFusionError;
use datafusion::datasource::object_store::ObjectStoreRegistry;
use datafusion::execution::object_store::DefaultObjectStoreRegistry;
use object_store::ObjectStore;
use std::sync::Arc;
use url::Url;

#[derive(Debug)]
pub struct LakehouseObjectStoreRegistry {
    registry: DefaultObjectStoreRegistry,
    urls: DashSet<Url>,
}

impl LakehouseObjectStoreRegistry {
    pub fn new() -> Self {
        Self {
            registry: DefaultObjectStoreRegistry::new(),
            urls: DashSet::new(),
        }
    }

    pub fn contains_url(&self, url: &Url) -> bool {
        self.urls.contains(url)
    }
}

impl Default for LakehouseObjectStoreRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectStoreRegistry for LakehouseObjectStoreRegistry {
    fn register_store(
        &self,
        url: &Url,
        store: Arc<dyn ObjectStore>,
    ) -> Option<Arc<dyn ObjectStore>> {
        self.urls.insert(url.clone());
        self.registry.register_store(url, store)
    }

    fn get_store(&self, url: &Url) -> Result<Arc<dyn ObjectStore>, DataFusionError> {
        self.registry.get_store(url)
    }
}
