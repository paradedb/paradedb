/// Rust implementations of Postgres functions in src/include/access/htup_details.h
///
/// This can be contributed to pgrx.
use pgrx::*;

/// Sets `xmax` value of the specified [`HeapTupleHeaderData`]
///
/// # Safety
///
/// Caller must ensure `tup` is a valid [`HeapTupleHeaderData`] pointer
pub unsafe fn heap_tuple_header_set_xmax(tup: *mut pg_sys::HeapTupleHeaderData, xid: u32) {
    // #define HeapTupleHeaderSetXmin(tup, xid) \
    // ( \
    //     (tup)->t_choice.t_heap.t_xmin = (xid) \
    // )

    unsafe {
        // SAFETY:  caller has asserted `tup` is a valid HeapTupleHeader pointer
        (*tup).t_choice.t_heap.t_xmax = xid;
    }
}

/// Sets `xmin` value of the specified [`HeapTupleHeaderData`]
///
/// # Safety
///
/// Caller must ensure `tup` is a valid [`HeapTupleHeaderData`] pointer
pub unsafe fn heap_tuple_header_set_xmin(tup: *mut pg_sys::HeapTupleHeaderData, xid: u32) {
    // #define HeapTupleHeaderSetXmin(tup, xid) \
    // ( \
    //     (tup)->t_choice.t_heap.t_xmin = (xid) \
    // )

    unsafe {
        // SAFETY:  caller has asserted `tup` is a valid HeapTupleHeader pointer
        (*tup).t_choice.t_heap.t_xmin = xid;
    }
}
