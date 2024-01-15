use async_std::task;
use core::ffi::c_char;
use pgrx::*;

use crate::datafusion::context::DatafusionContext;
use crate::errors::ParadeError;

#[pg_guard]
#[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
pub extern "C" fn analytics_relation_set_new_filenode(
    rel: pg_sys::Relation,
    _newrnode: *const pg_sys::RelFileNode,
    persistence: c_char,
    _freezeXid: *mut pg_sys::TransactionId,
    _minmulti: *mut pg_sys::MultiXactId,
) {
    create_table(rel, persistence).expect("Failed to create table");
}

#[pg_guard]
#[cfg(feature = "pg16")]
pub extern "C" fn analytics_relation_set_new_filelocator(
    rel: pg_sys::Relation,
    _newrlocator: *const pg_sys::RelFileLocator,
    persistence: c_char,
    _freezeXid: *mut pg_sys::TransactionId,
    _minmulti: *mut pg_sys::MultiXactId,
) {
    create_table(rel, persistence).expect("Failed to create table");
}

#[inline]
fn create_table(rel: pg_sys::Relation, persistence: c_char) -> Result<(), ParadeError> {
    let pg_relation = unsafe { PgRelation::from_pg(rel) };

    match persistence as u8 {
        pg_sys::RELPERSISTENCE_UNLOGGED => Err(ParadeError::Generic(
            "Unlogged tables are not yet supported".to_string(),
        )),
        pg_sys::RELPERSISTENCE_TEMP => Err(ParadeError::Generic(
            "Temp tables are not yet supported".to_string(),
        )),
        pg_sys::RELPERSISTENCE_PERMANENT => {
            DatafusionContext::with_provider_context(|provider, _| {
                task::block_on(provider.create_table(&pg_relation))
            })?
        }
        _ => Err(ParadeError::Generic("Unknown persistence type".to_string())),
    }
}
