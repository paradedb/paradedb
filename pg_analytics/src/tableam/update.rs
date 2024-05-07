use pgrx::*;
use thiserror::Error;

#[pg_guard]
#[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
pub extern "C" fn deltalake_tuple_update(
    _rel: pg_sys::Relation,
    _otid: pg_sys::ItemPointer,
    _slot: *mut pg_sys::TupleTableSlot,
    _cid: pg_sys::CommandId,
    _snapshot: pg_sys::Snapshot,
    _crosscheck: pg_sys::Snapshot,
    _wait: bool,
    _tmfd: *mut pg_sys::TM_FailureData,
    _lockmode: *mut pg_sys::LockTupleMode,
    _update_indexes: *mut bool,
) -> pg_sys::TM_Result {
    panic!("{}", UpdateError::UpdateNotSupported.to_string());
}

#[pg_guard]
#[cfg(feature = "pg16")]
pub extern "C" fn deltalake_tuple_update(
    _rel: pg_sys::Relation,
    _otid: pg_sys::ItemPointer,
    _slot: *mut pg_sys::TupleTableSlot,
    _cid: pg_sys::CommandId,
    _snapshot: pg_sys::Snapshot,
    _crosscheck: pg_sys::Snapshot,
    _wait: bool,
    _tmfd: *mut pg_sys::TM_FailureData,
    _lockmode: *mut pg_sys::LockTupleMode,
    _update_indexes: *mut pg_sys::TU_UpdateIndexes,
) -> pg_sys::TM_Result {
    panic!("{}", UpdateError::UpdateNotSupported.to_string());
}

#[derive(Error, Debug)]
pub enum UpdateError {
    #[error("UPDATE is not currently supported because Parquet tables are append only.")]
    UpdateNotSupported,
}
