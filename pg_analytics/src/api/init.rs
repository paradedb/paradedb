use pgrx::*;

use crate::datafusion::session::Session;

extension_sql!(
    r#"
    CREATE OR REPLACE PROCEDURE init() LANGUAGE C AS 'MODULE_PATHNAME', 'init';
    "#,
    name = "init"
);
#[pg_guard]
#[no_mangle]
pub extern "C" fn init() {
    let _ = Session::init(Session::catalog_oid().expect("Catalog OID not found")).unwrap_or_else(
        |err| {
            panic!("{}", err);
        },
    );
}
