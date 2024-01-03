use async_std::task;
use datafusion::catalog::CatalogProvider;
use datafusion::datasource::file_format::parquet::ParquetFormat;
use datafusion::execution::context::SessionState;
use datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};
use datafusion::prelude::{SessionConfig, SessionContext};
use pgrx::*;
use std::{path::Path, sync::Arc};

use crate::datafusion::catalog::{ParadeCatalog, ParadeCatalogList};
use crate::datafusion::directory::ParquetDirectory;
use crate::datafusion::registry::{CONTEXT, PARADE_CATALOG, PARADE_SCHEMA};
use crate::datafusion::schema::{ParadeSchemaOpts, ParadeSchemaProvider};

#[pg_extern]
pub fn init() {
    let session_config = SessionConfig::from_env()
        .unwrap()
        .with_information_schema(true);

    let rn_config = RuntimeConfig::new();
    let runtime_env = RuntimeEnv::new(rn_config).unwrap();

    let mut context_lock = CONTEXT.write();

    match context_lock.as_mut() {
        Some(context) => {
            let schema = context
                .catalog(PARADE_CATALOG)
                .ok_or("Catalog not found")
                .unwrap()
                .schema(PARADE_SCHEMA)
                .ok_or("Schema not found")
                .unwrap();
            let lister = schema.as_any().downcast_ref::<ParadeSchemaProvider>();
            if let Some(lister) = lister {
                task::block_on(lister.refresh(&context.state())).unwrap();
            }
        }
        None => {
            let mut context =
                SessionContext::new_with_config_rt(session_config.clone(), Arc::new(runtime_env));
            // Create schema provider
            let schema_provider = create_schema_provider(&context.state());
            context.register_catalog_list(Arc::new(ParadeCatalogList::new()));
            // Create and register catalog
            let catalog = ParadeCatalog::new();
            catalog
                .register_schema(PARADE_SCHEMA, schema_provider.clone())
                .unwrap();
            context.register_catalog(PARADE_CATALOG, Arc::new(catalog));
            // Set context
            *context_lock = Some(context);
        }
    }
}

#[inline]
fn create_schema_provider(state: &SessionState) -> Arc<ParadeSchemaProvider> {
    Arc::new(
        task::block_on(ParadeSchemaProvider::create(
            state,
            ParadeSchemaOpts {
                format: Arc::new(ParquetFormat::new().with_enable_pruning(Some(true))),
                dir: Path::new(&ParquetDirectory::schema_path().unwrap()).to_path_buf(),
            },
        ))
        .expect("Could not get schema"),
    )
}
