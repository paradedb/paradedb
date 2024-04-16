use async_trait::async_trait;
use datafusion_federation::FederatedTableProviderAdaptor;
use datafusion_federation_sql::{MultiSchemaProvider, SQLFederationProvider, SQLSchemaProvider};
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::datasource::TableProvider;
use pgrx::*;
use std::any::Any;
use std::sync::Arc;

pub struct PgMultiSchemaProvider {
    provider: MultiSchemaProvider,
}

impl PgMultiSchemaProvider {
    pub fn new(children: Vec<Arc<dyn SchemaProvider>>) -> Self {
        Self {
            provider: MultiSchemaProvider::new(children),
        }
    }
}

#[async_trait]
impl SchemaProvider for PgMultiSchemaProvider {
    fn as_any(&self) -> &dyn Any {
        self.provider.as_any()
    }

    fn table_names(&self) -> Vec<String> {
        self.provider.table_names()
    }

    async fn table(&self, name: &str) -> Option<Arc<dyn TableProvider>> {
        info!("table called {:?}", name);
        self.provider.table(name).await
    }

    fn table_exist(&self, name: &str) -> bool {
        self.provider.table_exist(name)
    }
}
