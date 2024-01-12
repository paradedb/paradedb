use async_std::task;
use deltalake::datafusion::catalog::CatalogProvider;
use deltalake::datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};
use deltalake::datafusion::prelude::{SessionConfig, SessionContext};
use pgrx::*;
use std::path::Path;
use std::sync::Arc;

use crate::datafusion::catalog::{ParadeCatalog, ParadeCatalogList, PARADE_CATALOG};
use crate::datafusion::context::DatafusionContext;
use crate::datafusion::directory::ParquetDirectory;
use crate::datafusion::schema::{ParadeSchemaProvider, PARADE_SCHEMA};

extension_sql!(
    r#"
    CREATE OR REPLACE PROCEDURE init() LANGUAGE C AS 'MODULE_PATHNAME', 'init';
    "#,
    name = "init"
);
#[pg_guard]
#[no_mangle]
pub extern "C" fn init() {
    let session_config = SessionConfig::from_env()
        .expect("Failed to create session config")
        .with_information_schema(true);

    let rn_config = RuntimeConfig::new();
    let runtime_env = RuntimeEnv::new(rn_config).expect("Failed to create runtime env");

    DatafusionContext::with_write_lock(|mut context_lock| {
        if context_lock.as_mut().is_none() {
            let mut context =
                SessionContext::new_with_config_rt(session_config, Arc::new(runtime_env));
            // Create an empty schema provider
            let schema_provider = Arc::new(
                task::block_on(ParadeSchemaProvider::try_new(
                    Path::new(&ParquetDirectory::schema_path().expect("Failed to get schema path"))
                        .to_path_buf(),
                ))
                .expect("Failed to create schema provider"),
            );
            // Register catalog list
            context.register_catalog_list(Arc::new(ParadeCatalogList::new()));
            // Create and register catalog
            let catalog = ParadeCatalog::new();
            catalog
                .register_schema(PARADE_SCHEMA, schema_provider)
                .expect("Failed to register schema");
            context.register_catalog(PARADE_CATALOG, Arc::new(catalog));
            // Set context
            *context_lock = Some(context);
        }
    });

    // Load the schema provider with tables
    DatafusionContext::with_provider_context(|provider, _| {
        task::block_on(provider.init()).expect("Failed to refresh schema provider");
    });
}
