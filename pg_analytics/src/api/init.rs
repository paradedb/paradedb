use pgrx::*;

use crate::datafusion::session::ParadeSessionContext;

extension_sql!(
    r#"
    CREATE OR REPLACE PROCEDURE init() LANGUAGE C AS 'MODULE_PATHNAME', 'init';
    "#,
    name = "init"
);
#[pg_guard]
#[no_mangle]
pub extern "C" fn init() {
    let _ = ParadeSessionContext::init(
        ParadeSessionContext::postgres_catalog_oid().expect("Catalog OID not found"),
    )
    .unwrap_or_else(|err| {
        panic!("{}", err);
    });
}
