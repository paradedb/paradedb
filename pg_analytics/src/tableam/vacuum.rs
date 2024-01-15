/*
    Vacuums are handled by the process utility hook, so these functions are left unimplemented.
*/

use pgrx::*;

#[pg_guard]
pub extern "C" fn analytics_relation_vacuum(
    _rel: pg_sys::Relation,
    _params: *mut pg_sys::VacuumParams,
    _bstrategy: pg_sys::BufferAccessStrategy,
) {
}

#[pg_guard]
#[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
pub extern "C" fn analytics_relation_copy_data(
    _rel: pg_sys::Relation,
    _newrnode: *const pg_sys::RelFileNode,
) {
}

#[pg_guard]
#[cfg(feature = "pg16")]
pub extern "C" fn analytics_relation_copy_data(
    _rel: pg_sys::Relation,
    _newrnode: *const pg_sys::RelFileLocator,
) {
}

#[pg_guard]
pub extern "C" fn analytics_relation_copy_for_cluster(
    _NewTable: pg_sys::Relation,
    _OldTable: pg_sys::Relation,
    _OldIndex: pg_sys::Relation,
    _use_sort: bool,
    _OldestXmin: pg_sys::TransactionId,
    _xid_cutoff: *mut pg_sys::TransactionId,
    _multi_cutoff: *mut pg_sys::MultiXactId,
    _num_tuples: *mut f64,
    _tups_vacuumed: *mut f64,
    _tups_recently_dead: *mut f64,
) {
}
