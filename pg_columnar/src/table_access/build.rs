use core::ffi::c_char;
use pgrx::pg_sys::*;
use pgrx::*;

use crate::datafusion::DFTable;

#[pg_guard]
pub unsafe extern "C" fn memam_relation_set_new_filenode(
    rel: Relation,
    newrnode: *const RelFileNode,
    persistence: c_char,
    freezeXid: *mut TransactionId,
    minmulti: *mut MultiXactId,
) {
    let pgrel = unsafe { PgRelation::from_pg(rel) };
    DFTable::create_from_pg(&pgrel, persistence as u8);
}
