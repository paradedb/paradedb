use core::ffi::c_char;
use pgrx::pg_sys::*;
use pgrx::*;

use crate::datafusion::table::ParadeTable;

#[pg_guard]
#[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
pub unsafe extern "C" fn memam_relation_set_new_filenode(
    rel: Relation,
    _newrnode: *const RelFileNode,
    persistence: c_char,
    _freezeXid: *mut TransactionId,
    _minmulti: *mut MultiXactId,
) {
    create_table(rel, persistence);
}

#[pg_guard]
#[cfg(feature = "pg16")]
pub unsafe extern "C" fn memam_relation_set_new_filelocator(
    rel: Relation,
    _newrlocator: *const RelFileLocator,
    persistence: c_char,
    _freezeXid: *mut TransactionId,
    _minmulti: *mut MultiXactId,
) {
    create_table(rel, persistence);
}

#[inline]
fn create_table(rel: Relation, persistence: c_char) {
    let pg_relation = unsafe { PgRelation::from_pg(rel) };

    match persistence as u8 {
        pg_sys::RELPERSISTENCE_UNLOGGED => {
            panic!("Unlogged tables are not yet supported");
        }
        pg_sys::RELPERSISTENCE_TEMP => {
            panic!("Temp tables are not yet supported");
        }
        pg_sys::RELPERSISTENCE_PERMANENT => {
            let _ = ParadeTable::create(&pg_relation).unwrap();
        }
        _ => {
            panic!("Unknown persistence type");
        }
    };
}
