use pgrx::*;
use thiserror::Error;

#[pg_guard]
pub extern "C" fn deltalake_relation_needs_toast_table(_rel: pg_sys::Relation) -> bool {
    false
}

#[pg_guard]
#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
pub extern "C" fn deltalake_relation_toast_am(_rel: pg_sys::Relation) -> pg_sys::Oid {
    pg_sys::Oid::INVALID
}

#[pg_guard]
#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
pub extern "C" fn deltalake_relation_fetch_toast_slice(
    _toastrel: pg_sys::Relation,
    _valueid: pg_sys::Oid,
    _attrsize: pg_sys::int32,
    _sliceoffset: pg_sys::int32,
    _slicelength: pg_sys::int32,
    _result: *mut pg_sys::varlena,
) {
    panic!(
        "{}",
        ToastTableError::FetchToastSliceNotSupported.to_string()
    );
}

#[derive(Error, Debug)]
pub enum ToastTableError {
    #[error("relation_fetch_toast_slice not implemented")]
    FetchToastSliceNotSupported,
}
