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
