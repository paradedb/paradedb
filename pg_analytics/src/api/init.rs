use async_std::task;
use deltalake::datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};
use deltalake::datafusion::prelude::{SessionConfig, SessionContext};
use pgrx::*;
use std::sync::Arc;

use crate::datafusion::catalog::{ParadeCatalog, ParadeCatalogList, PARADE_CATALOG};
use crate::datafusion::context::DatafusionContext;
use crate::datafusion::directory::ParadeDirectory;
use crate::errors::ParadeError;

extension_sql!(
    r#"
    CREATE OR REPLACE PROCEDURE init() LANGUAGE C AS 'MODULE_PATHNAME', 'init';
    "#,
    name = "init"
);
#[pg_guard]
#[no_mangle]
pub extern "C" fn init() {
    init_impl().expect("Failed to initialize context");
}

#[inline]
fn init_impl() -> Result<(), ParadeError> {
    let session_config = SessionConfig::from_env()?.with_information_schema(true);

    let rn_config = RuntimeConfig::new();
    let runtime_env = RuntimeEnv::new(rn_config)?;

    DatafusionContext::with_write_lock(|mut context_lock| {
        let mut context = SessionContext::new_with_config_rt(session_config, Arc::new(runtime_env));

        // Create schema directory if it doesn't exist
        ParadeDirectory::create_delta_path()?;

        // Register catalog list
        context.register_catalog_list(Arc::new(ParadeCatalogList::try_new()?));

        // Create and register catalog
        let catalog = ParadeCatalog::try_new()?;
        task::block_on(catalog.init())?;
        context.register_catalog(PARADE_CATALOG, Arc::new(catalog));

        // Set context
        *context_lock = Some(context);

        Ok::<(), ParadeError>(())
    })?;

    Ok(())
}
