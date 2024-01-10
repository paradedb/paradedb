use core::ffi::c_int;
use core::ffi::c_void;

#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
use pgrx::pg_sys::varlena;

use pgrx::pg_sys::*;
use pgrx::*;

pub unsafe extern "C" fn memam_scan_begin(
    _rel: Relation,
    _snapshot: Snapshot,
    _nkeys: c_int,
    _key: *mut ScanKeyData,
    _pscan: ParallelTableScanDesc,
    _flags: uint32,
) -> TableScanDesc {
    info!("Calling memam_relation_scan_begin");
    PgBox::<TableScanDescData>::alloc0().into_pg()
}

pub unsafe extern "C" fn memam_scan_end(_scan: TableScanDesc) {
    info!("Calling memam_scan_end");
}

pub unsafe extern "C" fn memam_scan_rescan(
    _scan: TableScanDesc,
    _key: *mut ScanKeyData,
    _set_params: bool,
    _allow_strat: bool,
    _allow_sync: bool,
    _allow_pagemode: bool,
) {
    info!("Calling memam_scan_rescan");
}

pub unsafe extern "C" fn memam_scan_getnextslot(
    _scan: TableScanDesc,
    _direction: ScanDirection,
    _slot: *mut TupleTableSlot,
) -> bool {
    info!("Calling memam_relation_scan_getnextslot");
    false
}

#[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
pub unsafe extern "C" fn memam_scan_set_tidrange(
    _scan: TableScanDesc,
    _mintid: ItemPointer,
    _maxtid: ItemPointer,
) {
    info!("Calling memam_scan_set_tidrange");
}

#[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
pub unsafe extern "C" fn memam_scan_getnextslot_tidrange(
    _scan: TableScanDesc,
    _direction: ScanDirection,
    _slot: *mut TupleTableSlot,
) -> bool {
    info!("Calling memam_scan_getnextslot_tidrange");
    false
}

pub unsafe extern "C" fn memam_parallelscan_estimate(rel: Relation) -> Size {
    info!("Calling memam_parallelscan_estimate");
    table_block_parallelscan_estimate(rel)
}

pub unsafe extern "C" fn memam_parallelscan_initialize(
    rel: Relation,
    pscan: ParallelTableScanDesc,
) -> Size {
    info!("Calling memam_parallelscan_initialize");
    table_block_parallelscan_initialize(rel, pscan)
}

pub unsafe extern "C" fn memam_parallelscan_reinitialize(
    rel: Relation,
    pscan: ParallelTableScanDesc,
) {
    info!("Calling memam_parallelscan_reinitialize");
    table_block_parallelscan_reinitialize(rel, pscan)
}

#[pg_guard]
pub unsafe extern "C" fn memam_index_fetch_begin(rel: Relation) -> *mut IndexFetchTableData {
    info!("Calling memam_index_fetch_begin");
    let mut data = PgBox::<IndexFetchTableData>::alloc0();
    data.rel = rel;

    data.into_pg()
}

pub unsafe extern "C" fn memam_index_fetch_reset(_data: *mut IndexFetchTableData) {
    info!("Calling memam_index_fetch_reset");
}

pub unsafe extern "C" fn memam_index_fetch_end(_data: *mut IndexFetchTableData) {
    info!("Calling memam_index_fetch_end");
}

pub unsafe extern "C" fn memam_index_fetch_tuple(
    _scan: *mut IndexFetchTableData,
    _tid: ItemPointer,
    _snapshot: Snapshot,
    _slot: *mut TupleTableSlot,
    _call_again: *mut bool,
    _all_dead: *mut bool,
) -> bool {
    info!("Calling memam_index_fetch_tuple");
    false
}

pub unsafe extern "C" fn memam_tuple_fetch_row_version(
    _rel: Relation,
    _tid: ItemPointer,
    _snapshot: Snapshot,
    _slot: *mut TupleTableSlot,
) -> bool {
    info!("Calling memam_tuple_fetch_row_version");
    false
}

pub unsafe extern "C" fn memam_tuple_tid_valid(_scan: TableScanDesc, _tid: ItemPointer) -> bool {
    info!("Calling memam_tuple_tid_valid");
    false
}

pub unsafe extern "C" fn memam_tuple_get_latest_tid(_scan: TableScanDesc, _tid: ItemPointer) {
    info!("Calling memam_tuple_get_latest_tid");
}

pub unsafe extern "C" fn memam_tuple_satisfies_snapshot(
    _rel: Relation,
    _slot: *mut TupleTableSlot,
    _snapshot: Snapshot,
) -> bool {
    info!("Calling memam_tuple_satisfies_snapshot");
    false
}

#[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
pub unsafe extern "C" fn memam_index_delete_tuples(
    _rel: Relation,
    _delstate: *mut TM_IndexDeleteOp,
) -> TransactionId {
    info!("Calling memam_index_delete_tuples");
    0
}

#[pg_guard]
pub unsafe extern "C" fn memam_tuple_insert(
    _rel: Relation,
    _slot: *mut TupleTableSlot,
    _cid: CommandId,
    _options: c_int,
    _bistate: *mut BulkInsertStateData,
) {
    info!("Calling memam_tuple_insert");
}

pub unsafe extern "C" fn memam_tuple_insert_speculative(
    _rel: Relation,
    _slot: *mut TupleTableSlot,
    _cid: CommandId,
    _options: c_int,
    _bistate: *mut BulkInsertStateData,
    _specToken: uint32,
) {
    info!("Calling memam_tuple_insert_speculative");
}

pub unsafe extern "C" fn memam_tuple_complete_speculative(
    _rel: Relation,
    _slot: *mut TupleTableSlot,
    _specToken: uint32,
    _succeeded: bool,
) {
    info!("Calling memam_tuple_complete_speculative");
}

pub unsafe extern "C" fn memam_tuple_delete(
    _rel: Relation,
    _tid: ItemPointer,
    _cid: CommandId,
    _snapshot: Snapshot,
    _crosscheck: Snapshot,
    _wait: bool,
    _tmfd: *mut TM_FailureData,
    _changingPart: bool,
) -> TM_Result {
    info!("Calling memam_tuple_delete");
    0
}

#[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
pub unsafe extern "C" fn memam_tuple_update(
    _rel: Relation,
    _otid: ItemPointer,
    _slot: *mut TupleTableSlot,
    _cid: CommandId,
    _snapshot: Snapshot,
    _crosscheck: Snapshot,
    _wait: bool,
    _tmfd: *mut TM_FailureData,
    _lockmode: *mut LockTupleMode,
    _update_indexes: *mut bool,
) -> TM_Result {
    info!("Calling memam_tuple_update");
    0
}

#[cfg(feature = "pg16")]
pub unsafe extern "C" fn memam_tuple_update(
    _rel: Relation,
    _otid: ItemPointer,
    _slot: *mut TupleTableSlot,
    _cid: CommandId,
    _snapshot: Snapshot,
    _crosscheck: Snapshot,
    _wait: bool,
    _tmfd: *mut TM_FailureData,
    _lockmode: *mut LockTupleMode,
    _update_indexes: *mut TU_UpdateIndexes,
) -> TM_Result {
    info!("Calling memam_tuple_update");
    0
}

pub unsafe extern "C" fn memam_tuple_lock(
    _rel: Relation,
    _tid: ItemPointer,
    _snapshot: Snapshot,
    _slot: *mut TupleTableSlot,
    _cid: CommandId,
    _mode: LockTupleMode,
    _wait_policy: LockWaitPolicy,
    _flags: uint8,
    _tmfd: *mut TM_FailureData,
) -> TM_Result {
    info!("Calling memam_tuple_lock");
    0
}

pub unsafe extern "C" fn memam_relation_nontransactional_truncate(_rel: Relation) {
    info!("Calling memam_relation_nontransactional_truncate");
}

#[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
pub unsafe extern "C" fn memam_relation_copy_data(_rel: Relation, _newrnode: *const RelFileNode) {
    info!("Calling memam_relation_copy_data");
}

#[cfg(feature = "pg16")]
pub unsafe extern "C" fn memam_relation_copy_data(
    _rel: Relation,
    _newrnode: *const RelFileLocator,
) {
    info!("Calling memam_relation_copy_data");
}

pub unsafe extern "C" fn memam_relation_copy_for_cluster(
    _NewTable: Relation,
    _OldTable: Relation,
    _OldIndex: Relation,
    _use_sort: bool,
    _OldestXmin: TransactionId,
    _xid_cutoff: *mut TransactionId,
    _multi_cutoff: *mut MultiXactId,
    _num_tuples: *mut f64,
    _tups_vacuumed: *mut f64,
    _tups_recently_dead: *mut f64,
) {
    info!("Calling memam_relation_copy_for_cluster");
}

pub unsafe extern "C" fn memam_relation_vacuum(
    _rel: Relation,
    _params: *mut VacuumParams,
    _bstrategy: BufferAccessStrategy,
) {
    info!("Calling memam_relation_vacuum");
}

pub unsafe extern "C" fn memam_scan_analyze_next_block(
    _scan: TableScanDesc,
    _blockno: BlockNumber,
    _bstrategy: BufferAccessStrategy,
) -> bool {
    info!("Calling memam_scan_analyze_next_block");
    false
}

pub unsafe extern "C" fn memam_scan_analyze_next_tuple(
    _scan: TableScanDesc,
    _OldestXmin: TransactionId,
    _liverows: *mut f64,
    _deadrows: *mut f64,
    _slot: *mut TupleTableSlot,
) -> bool {
    info!("Calling memam_scan_analyze_next_tuple");
    false
}

pub unsafe extern "C" fn memam_index_build_range_scan(
    _table_rel: Relation,
    _index_rel: Relation,
    _index_info: *mut IndexInfo,
    _allow_sync: bool,
    _anyvisible: bool,
    _progress: bool,
    _start_blockno: BlockNumber,
    _numblocks: BlockNumber,
    _callback: IndexBuildCallback,
    _callback_state: *mut c_void,
    _scan: TableScanDesc,
) -> f64 {
    info!("Calling memam_index_build_range_scan");
    0.0
}

pub unsafe extern "C" fn memam_index_validate_scan(
    _table_rel: Relation,
    _index_rel: Relation,
    _index_info: *mut IndexInfo,
    _snapshot: Snapshot,
    _state: *mut ValidateIndexState,
) {
    info!("Calling memam_index_validate_scan");
}

pub unsafe extern "C" fn memam_relation_size(_rel: Relation, _forkNumber: ForkNumber) -> uint64 {
    info!("Calling memam_relation_size");
    0
}

pub unsafe extern "C" fn memam_relation_needs_toast_table(_rel: Relation) -> bool {
    info!("Calling memam_relation_needs_toast_table");
    false
}

#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
pub unsafe extern "C" fn memam_relation_toast_am(_rel: Relation) -> Oid {
    info!("Calling memam_relation_needs_toast_am");
    Oid::INVALID
}

#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
pub unsafe extern "C" fn memam_relation_fetch_toast_slice(
    _toastrel: Relation,
    _valueid: Oid,
    _attrsize: int32,
    _sliceoffset: int32,
    _slicelength: int32,
    _result: *mut varlena,
) {
    info!("Calling memam_relation_fetch_toast_slice");
}

pub unsafe extern "C" fn memam_relation_estimate_size(
    _rel: Relation,
    _attr_widths: *mut int32,
    _pages: *mut BlockNumber,
    _tuples: *mut f64,
    _allvisfrac: *mut f64,
) {
    info!("Calling memam_relation_estimate_size");
}

pub unsafe extern "C" fn memam_scan_bitmap_next_block(
    _scan: TableScanDesc,
    _tbmres: *mut TBMIterateResult,
) -> bool {
    info!("Calling memam_scan_bitmap_next_block");
    false
}

pub unsafe extern "C" fn memam_scan_bitmap_next_tuple(
    _scan: TableScanDesc,
    _tbmres: *mut TBMIterateResult,
    _slot: *mut TupleTableSlot,
) -> bool {
    info!("Calling memam_scan_bitmap_next_tuple");
    false
}

pub unsafe extern "C" fn memam_scan_sample_next_block(
    _scan: TableScanDesc,
    _scanstate: *mut SampleScanState,
) -> bool {
    info!("Calling memam_scan_sample_next_block");
    false
}

pub unsafe extern "C" fn memam_scan_sample_next_tuple(
    _scan: TableScanDesc,
    _scanstate: *mut SampleScanState,
    _slot: *mut TupleTableSlot,
) -> bool {
    info!("Calling memam_scan_sample_next_tuple");
    false
}

#[cfg(any(feature = "pg12", feature = "pg13"))]
pub unsafe extern "C" fn memam_compute_xid_horizon_for_tuples(
    _rel: Relation,
    _items: *mut ItemPointerData,
    _nitems: c_int,
) -> TransactionId {
    info!("Calling memam_compute_xid_horizon_for_tuples");
    0
}
