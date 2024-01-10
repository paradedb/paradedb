use async_std::task;
use deltalake::datafusion::catalog::CatalogProvider;

use deltalake::datafusion::execution::context::SessionState;
use deltalake::datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};
use deltalake::datafusion::prelude::{SessionConfig, SessionContext};
use pgrx::*;
use std::path::Path;
use std::sync::Arc;

use crate::datafusion::catalog::{ParadeCatalog, ParadeCatalogList};
use crate::datafusion::context::DatafusionContext;
use crate::datafusion::directory::ParquetDirectory;
use crate::datafusion::registry::{PARADE_CATALOG, PARADE_SCHEMA};
use crate::datafusion::schema::{ParadeSchemaOpts, ParadeSchemaProvider};

#[pg_guard]
#[no_mangle]
extern "C" fn pg_finfo_init() -> &'static pg_sys::Pg_finfo_record {
    const V1_API: pg_sys::Pg_finfo_record = pg_sys::Pg_finfo_record { api_version: 1 };
    &V1_API
}

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
    let mut context_lock = DatafusionContext::write_lock().expect("Failed to get context lock");

    match context_lock.as_mut() {
        Some(context) => {
            let schema = context
                .catalog(PARADE_CATALOG)
                .expect("Catalog not found")
                .schema(PARADE_SCHEMA)
                .expect("Schema not found");

            let lister = schema
                .as_any()
                .downcast_ref::<ParadeSchemaProvider>()
                .expect("Failed to downcast schema provider");

            task::block_on(lister.refresh(&context.state()))
                .expect("Failed to refresh schema provider");
        }
        None => {
            let mut context =
                SessionContext::new_with_config_rt(session_config, Arc::new(runtime_env));
            // Create schema provider
            let schema_provider = create_schema_provider(&context.state());
            task::block_on(schema_provider.refresh(&context.state()))
                .expect("Failed to refresh schema provider");

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
    }
}

#[inline]
fn create_schema_provider(state: &SessionState) -> Arc<ParadeSchemaProvider> {
    Arc::new(
        task::block_on(ParadeSchemaProvider::try_new(
            state,
            ParadeSchemaOpts {
                dir: Path::new(
                    &ParquetDirectory::schema_path().expect("Failed to get schema path"),
                )
                .to_path_buf(),
            },
        ))
        .expect("Failed to create schema provider"),
    )
}
