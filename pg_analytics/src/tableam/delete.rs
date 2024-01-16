use pgrx::*;

#[pg_guard]
pub extern "C" fn deltalake_tuple_delete(
    _rel: pg_sys::Relation,
    _tid: pg_sys::ItemPointer,
    _cid: pg_sys::CommandId,
    _snapshot: pg_sys::Snapshot,
    _crosscheck: pg_sys::Snapshot,
    _wait: bool,
    _tmfd: *mut pg_sys::TM_FailureData,
    _changingPart: bool,
) -> pg_sys::TM_Result {
    0
}
