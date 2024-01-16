/*
    Storage and plan cost estimates are handled by Datafusion.
    These functions should never be called.
*/

#[cfg(any(feature = "pg12", feature = "pg13"))]
use core::ffi::c_int;
use pgrx::*;

#[pg_guard]
pub extern "C" fn deltalake_relation_nontransactional_truncate(_rel: pg_sys::Relation) {}

#[pg_guard]
pub extern "C" fn deltalake_relation_size(
    _rel: pg_sys::Relation,
    _forkNumber: pg_sys::ForkNumber,
) -> pg_sys::uint64 {
    0
}

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
}

#[pg_guard]
pub extern "C" fn deltalake_relation_estimate_size(
    _rel: pg_sys::Relation,
    _attr_widths: *mut pg_sys::int32,
    _pages: *mut pg_sys::BlockNumber,
    _tuples: *mut f64,
    _allvisfrac: *mut f64,
) {
}

#[pg_guard]
#[cfg(any(feature = "pg12", feature = "pg13"))]
pub extern "C" fn deltalake_compute_xid_horizon_for_tuples(
    _rel: pg_sys::Relation,
    _items: *mut pg_sys::ItemPointerData,
    _nitems: c_int,
) -> pg_sys::TransactionId {
    0
}
