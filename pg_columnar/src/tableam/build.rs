use core::ffi::c_char;
use pgrx::pg_sys::*;
use pgrx::*;

use crate::datafusion::table::DatafusionTable;

#[pg_guard]
pub unsafe extern "C" fn memam_relation_set_new_filenode(
    rel: Relation,
    _newrnode: *const RelFileNode,
    persistence: c_char,
    _freezeXid: *mut TransactionId,
    _minmulti: *mut MultiXactId,
) {
    let pg_relation = unsafe { PgRelation::from_pg(rel) };

    match persistence as u8 {
        pg_sys::RELPERSISTENCE_UNLOGGED => {
            panic!("Unlogged tables are not yet supported");
        }
        pg_sys::RELPERSISTENCE_TEMP => {
            panic!("Temp tables are not yet supported");
        }
        pg_sys::RELPERSISTENCE_PERMANENT => {
            let _ = DatafusionTable::create(&pg_relation).unwrap();
        }
        _ => {
            panic!("Unknown persistence type");
        }
    };
}
