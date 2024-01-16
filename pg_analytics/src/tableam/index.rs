/*
    Index scans are not yet supported. These are left unimplemented.
*/

use core::ffi::c_void;
use pgrx::*;

#[pg_guard]
pub extern "C" fn deltalake_index_fetch_begin(
    rel: pg_sys::Relation,
) -> *mut pg_sys::IndexFetchTableData {
    unsafe {
        let mut data = PgBox::<pg_sys::IndexFetchTableData>::alloc0();
        data.rel = rel;

        data.into_pg()
    }
}

#[pg_guard]
pub extern "C" fn deltalake_index_fetch_reset(_data: *mut pg_sys::IndexFetchTableData) {}

#[pg_guard]
pub extern "C" fn deltalake_index_fetch_end(_data: *mut pg_sys::IndexFetchTableData) {}

#[pg_guard]
pub extern "C" fn deltalake_index_fetch_tuple(
    _scan: *mut pg_sys::IndexFetchTableData,
    _tid: pg_sys::ItemPointer,
    _snapshot: pg_sys::Snapshot,
    _slot: *mut pg_sys::TupleTableSlot,
    _call_again: *mut bool,
    _all_dead: *mut bool,
) -> bool {
    false
}

#[pg_guard]
#[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
pub extern "C" fn deltalake_index_delete_tuples(
    _rel: pg_sys::Relation,
    _delstate: *mut pg_sys::TM_IndexDeleteOp,
) -> pg_sys::TransactionId {
    0
}

#[pg_guard]
pub extern "C" fn deltalake_index_build_range_scan(
    _table_rel: pg_sys::Relation,
    _index_rel: pg_sys::Relation,
    _index_info: *mut pg_sys::IndexInfo,
    _allow_sync: bool,
    _anyvisible: bool,
    _progress: bool,
    _start_blockno: pg_sys::BlockNumber,
    _numblocks: pg_sys::BlockNumber,
    _callback: pg_sys::IndexBuildCallback,
    _callback_state: *mut c_void,
    _scan: pg_sys::TableScanDesc,
) -> f64 {
    0.0
}

#[pg_guard]
pub extern "C" fn deltalake_index_validate_scan(
    _table_rel: pg_sys::Relation,
    _index_rel: pg_sys::Relation,
    _index_info: *mut pg_sys::IndexInfo,
    _snapshot: pg_sys::Snapshot,
    _state: *mut pg_sys::ValidateIndexState,
) {
}
