use core::ffi::c_char;
use pgrx::pg_sys::*;
use pgrx::*;

use crate::tableam::utils::create_from_pg;

#[pg_guard]
pub unsafe extern "C" fn memam_relation_set_new_filenode(
    rel: Relation,
    _newrnode: *const RelFileNode,
    persistence: c_char,
    _freezeXid: *mut TransactionId,
    _minmulti: *mut MultiXactId,
) {
    let pgrel = unsafe { PgRelation::from_pg(rel) };
    create_from_pg(&pgrel, persistence as u8).expect("Failed to create table");
}
