use pgrx::*;

use crate::datafusion::context::DatafusionContext;

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
