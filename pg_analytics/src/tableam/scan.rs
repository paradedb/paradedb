/*
    Scans i.e. SELECT queries are handled by the ExecutorRun hook.
    These functions should never be called.
*/

use core::ffi::c_int;
use pgrx::*;

#[pg_guard]
pub extern "C" fn deltalake_scan_begin(
    rel: pg_sys::Relation,
    snapshot: pg_sys::Snapshot,
    nkeys: c_int,
    key: *mut pg_sys::ScanKeyData,
    pscan: pg_sys::ParallelTableScanDesc,
    flags: pg_sys::uint32,
) -> pg_sys::TableScanDesc {
    unsafe {
        let mut data = PgBox::<pg_sys::TableScanDescData>::alloc0();
        data.rs_rd = rel;
        data.rs_snapshot = snapshot;
        data.rs_nkeys = nkeys;
        data.rs_key = key;
        data.rs_parallel = pscan;
        data.rs_flags = flags;

        data.into_pg()
    }
}

#[pg_guard]
pub extern "C" fn deltalake_scan_end(_scan: pg_sys::TableScanDesc) {}

#[pg_guard]
pub extern "C" fn deltalake_scan_rescan(
    _scan: pg_sys::TableScanDesc,
    _key: *mut pg_sys::ScanKeyData,
    _set_params: bool,
    _allow_strat: bool,
    _allow_sync: bool,
    _allow_pagemode: bool,
) {
}

#[pg_guard]
pub extern "C" fn deltalake_scan_getnextslot(
    _scan: pg_sys::TableScanDesc,
    _direction: pg_sys::ScanDirection,
    _slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    false
}

#[pg_guard]
#[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
pub extern "C" fn deltalake_scan_set_tidrange(
    _scan: pg_sys::TableScanDesc,
    _mintid: pg_sys::ItemPointer,
    _maxtid: pg_sys::ItemPointer,
) {
}

#[pg_guard]
#[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
pub extern "C" fn deltalake_scan_getnextslot_tidrange(
    _scan: pg_sys::TableScanDesc,
    _direction: pg_sys::ScanDirection,
    _slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    false
}

#[pg_guard]
pub extern "C" fn deltalake_parallelscan_estimate(rel: pg_sys::Relation) -> pg_sys::Size {
    unsafe { pg_sys::table_block_parallelscan_estimate(rel) }
}

#[pg_guard]
pub extern "C" fn deltalake_parallelscan_initialize(
    rel: pg_sys::Relation,
    pscan: pg_sys::ParallelTableScanDesc,
) -> pg_sys::Size {
    unsafe { pg_sys::table_block_parallelscan_initialize(rel, pscan) }
}

#[pg_guard]
pub extern "C" fn deltalake_parallelscan_reinitialize(
    rel: pg_sys::Relation,
    pscan: pg_sys::ParallelTableScanDesc,
) {
    unsafe { pg_sys::table_block_parallelscan_reinitialize(rel, pscan) }
}

#[pg_guard]
pub extern "C" fn deltalake_scan_analyze_next_block(
    _scan: pg_sys::TableScanDesc,
    _blockno: pg_sys::BlockNumber,
    _bstrategy: pg_sys::BufferAccessStrategy,
) -> bool {
    false
}

#[pg_guard]
pub extern "C" fn deltalake_scan_analyze_next_tuple(
    _scan: pg_sys::TableScanDesc,
    _OldestXmin: pg_sys::TransactionId,
    _liverows: *mut f64,
    _deadrows: *mut f64,
    _slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    false
}

#[pg_guard]
pub extern "C" fn deltalake_scan_bitmap_next_block(
    _scan: pg_sys::TableScanDesc,
    _tbmres: *mut pg_sys::TBMIterateResult,
) -> bool {
    false
}

#[pg_guard]
pub extern "C" fn deltalake_scan_bitmap_next_tuple(
    _scan: pg_sys::TableScanDesc,
    _tbmres: *mut pg_sys::TBMIterateResult,
    _slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    false
}

#[pg_guard]
pub extern "C" fn deltalake_scan_sample_next_block(
    _scan: pg_sys::TableScanDesc,
    _scanstate: *mut pg_sys::SampleScanState,
) -> bool {
    false
}

#[pg_guard]
pub extern "C" fn deltalake_scan_sample_next_tuple(
    _scan: pg_sys::TableScanDesc,
    _scanstate: *mut pg_sys::SampleScanState,
    _slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    false
}

#[pg_guard]
pub extern "C" fn deltalake_tuple_fetch_row_version(
    _rel: pg_sys::Relation,
    _tid: pg_sys::ItemPointer,
    _snapshot: pg_sys::Snapshot,
    _slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    false
}

#[pg_guard]
pub extern "C" fn deltalake_tuple_tid_valid(
    _scan: pg_sys::TableScanDesc,
    _tid: pg_sys::ItemPointer,
) -> bool {
    false
}

#[pg_guard]
pub extern "C" fn deltalake_tuple_get_latest_tid(
    _scan: pg_sys::TableScanDesc,
    _tid: pg_sys::ItemPointer,
) {
}

#[pg_guard]
pub extern "C" fn deltalake_tuple_satisfies_snapshot(
    _rel: pg_sys::Relation,
    _slot: *mut pg_sys::TupleTableSlot,
    _snapshot: pg_sys::Snapshot,
) -> bool {
    false
}

#[pg_guard]
pub extern "C" fn deltalake_tuple_complete_speculative(
    _rel: pg_sys::Relation,
    _slot: *mut pg_sys::TupleTableSlot,
    _specToken: pg_sys::uint32,
    _succeeded: bool,
) {
}

#[pg_guard]
pub extern "C" fn deltalake_tuple_lock(
    _rel: pg_sys::Relation,
    _tid: pg_sys::ItemPointer,
    _snapshot: pg_sys::Snapshot,
    _slot: *mut pg_sys::TupleTableSlot,
    _cid: pg_sys::CommandId,
    _mode: pg_sys::LockTupleMode,
    _wait_policy: pg_sys::LockWaitPolicy,
    _flags: pg_sys::uint8,
    _tmfd: *mut pg_sys::TM_FailureData,
) -> pg_sys::TM_Result {
    0
}
