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
    let _ = DatafusionContext::init().expect("Failed to initialize context");
}
