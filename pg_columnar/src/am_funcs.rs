#![allow(unused)]
#![allow(non_snake_case)]

use pgrx::pg_sys::*;
use pgrx::PgBox;
use core::ffi::c_int;
use core::ffi::c_char;
use core::ffi::c_void;
use std::ptr;
use pgrx::IntoDatum;
use std::ptr::copy_nonoverlapping;
use datafusion::prelude::SessionContext;
use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use datafusion::datasource::MemTable;
use std::sync::Arc;
use datafusion::error::Result;
use datafusion::arrow::record_batch::RecordBatch;


pub unsafe extern "C" fn memam_slot_callbacks(rel: Relation) -> *const TupleTableSlotOps {
	return &TTSOpsVirtual;
}

pub unsafe extern "C" fn memam_scan_begin(rel: Relation, snapshot: Snapshot, nkeys: c_int, key: *mut ScanKeyData, pscan: ParallelTableScanDesc, flags: uint32) -> TableScanDesc {
	let mut scan = unsafe { PgBox::<TableScanDescData>::alloc0() };
	scan.rs_rd = rel;
	return scan.into_pg();
}

pub unsafe extern "C" fn memam_scan_end(scan: TableScanDesc) {

}

pub unsafe extern "C" fn memam_scan_rescan(scan: TableScanDesc, key: *mut ScanKeyData, set_params: bool, allow_strat: bool, allow_sync: bool, allow_pagemode: bool) {

}

pub unsafe extern "C" fn memam_scan_getnextslot(scan: TableScanDesc, direction: ScanDirection, slot: *mut TupleTableSlot) -> bool {
	static mut done: bool = false;

	if done {
		return false;
	}

	// TODO: Use RecordBatch::try_new to create new rows (replace dummy 314)
	let value: int32 = 314;
	let value_datum: Datum = value.into_datum().unwrap();
	let value_isnull: bool = false;

	copy_nonoverlapping::<Datum>(&value_datum, (*slot).tts_values.offset(0), 1);
	copy_nonoverlapping::<bool>(&value_isnull, (*slot).tts_isnull.offset(0), 1);
	ExecStoreVirtualTuple(slot);

	done = true;

	return true;
}

pub unsafe extern "C" fn memam_scan_set_tidrange(scan: TableScanDesc, mintid: ItemPointer, maxtid: ItemPointer) {

}

pub unsafe extern "C" fn memam_scan_getnextslot_tidrange(scan: TableScanDesc, direction: ScanDirection, slot: *mut TupleTableSlot) -> bool {
	return false
}

pub unsafe extern "C" fn memam_parallelscan_estimate(rel: Relation) -> Size {
	return table_block_parallelscan_estimate(rel);
}

pub unsafe extern "C" fn memam_parallelscan_initialize(rel: Relation, pscan: ParallelTableScanDesc) -> Size {
	return table_block_parallelscan_initialize(rel, pscan);
}

pub unsafe extern "C" fn memam_parallelscan_reinitialize(rel: Relation, pscan: ParallelTableScanDesc) {
	return table_block_parallelscan_reinitialize(rel, pscan);
}

pub unsafe extern "C" fn memam_index_fetch_begin(rel: Relation) -> *mut IndexFetchTableData {
	return ptr::null_mut::<IndexFetchTableData>();
}

pub unsafe extern "C" fn memam_index_fetch_reset(data: *mut IndexFetchTableData) {

}

pub unsafe extern "C" fn memam_index_fetch_end(data: *mut IndexFetchTableData) {

}

pub unsafe extern "C" fn memam_index_fetch_tuple(scan: *mut IndexFetchTableData, tid: ItemPointer, snapshot: Snapshot, slot: *mut TupleTableSlot, call_again: *mut bool, all_dead: *mut bool) -> bool {
	return false;
}

pub unsafe extern "C" fn memam_tuple_fetch_row_version(rel: Relation, tid: ItemPointer, snapshot: Snapshot, slot: *mut TupleTableSlot) -> bool {
	return false;
}

pub unsafe extern "C" fn memam_tuple_tid_valid(scan: TableScanDesc, tid: ItemPointer) -> bool {
	return false;
}

pub unsafe extern "C" fn memam_tuple_get_latest_tid(scan: TableScanDesc, tid: ItemPointer) {

}

pub unsafe extern "C" fn memam_tuple_satisfies_snapshot(rel: Relation, slot: *mut TupleTableSlot, snapshot: Snapshot) -> bool {
	return false;
}

pub unsafe extern "C" fn memam_index_delete_tuples(rel: Relation, delstate: *mut TM_IndexDeleteOp) -> TransactionId {
	return 0;
}

pub unsafe extern "C" fn memam_tuple_insert(rel: Relation, slot: *mut TupleTableSlot, cid: CommandId, options: c_int, bistate: *mut BulkInsertStateData) {
	
}

pub unsafe extern "C" fn memam_tuple_insert_speculative(rel: Relation, slot: *mut TupleTableSlot, cid: CommandId, options: c_int, bistate: *mut BulkInsertStateData, specToken: uint32) {

}

pub unsafe extern "C" fn memam_tuple_complete_speculative(rel: Relation, slot: *mut TupleTableSlot, specToken: uint32, succeeded: bool) {

}

pub unsafe extern "C" fn memam_multi_insert(rel: Relation, slots: *mut *mut TupleTableSlot, nslots: c_int, cid: CommandId, options: c_int, bistate: *mut BulkInsertStateData) {

}

pub unsafe extern "C" fn memam_tuple_delete(rel: Relation, tid: ItemPointer, cid: CommandId, snapshot: Snapshot, crosscheck: Snapshot, wait: bool, tmfd: *mut TM_FailureData, changingPart: bool) -> TM_Result {
	return 0;
}

pub unsafe extern "C" fn memam_tuple_update(rel: Relation, otid: ItemPointer, slot: *mut TupleTableSlot, cid: CommandId, snapshot: Snapshot, crosscheck: Snapshot, wait: bool, tmfd: *mut TM_FailureData, lockmode: *mut LockTupleMode, update_indexes: *mut bool) -> TM_Result {
	return 0;
}

pub unsafe extern "C" fn memam_tuple_lock(rel: Relation, tid: ItemPointer, snapshot: Snapshot, slot: *mut TupleTableSlot, cid: CommandId, mode: LockTupleMode, wait_policy: LockWaitPolicy, flags: uint8, tmfd: *mut TM_FailureData) -> TM_Result {
	return 0;
}

pub unsafe extern "C" fn memam_finish_bulk_insert(rel: Relation, options: c_int) {

}

pub unsafe extern "C" fn memam_relation_set_new_filenode(rel: Relation, newrnode: *const RelFileNode, persistence: c_char, freezeXid: *mut TransactionId, minmulti: *mut MultiXactId) {
	let schema = SchemaRef::new(Schema::new(Vec::<Field>::new()));
	// TODO: put proper column names and types here and use vec! instead of ::new

	// Empty table
	let mem_table = match MemTable::try_new(schema, vec![Vec::<RecordBatch>::new()]).ok() {
		Some(mem_table) => {
			let ctx = SessionContext::new();
			ctx.register_table(pgrx::name_data_to_str(&(*(*rel).rd_rel).relname), Arc::new(mem_table));
		},
		None => panic!("Could not create table")
	};
}

pub unsafe extern "C" fn memam_relation_nontransactional_truncate(rel: Relation) {

}

pub unsafe extern "C" fn memam_relation_copy_data(rel: Relation, newrnode: *const RelFileNode) {

}

pub unsafe extern "C" fn memam_relation_copy_for_cluster(NewTable: Relation, OldTable: Relation, OldIndex: Relation, use_sort: bool, OldestXmin: TransactionId, xid_cutoff: *mut TransactionId, multi_cutoff: *mut MultiXactId, num_tuples: *mut f64, tups_vacuumed: *mut f64, tups_recently_dead: *mut f64) {

}

pub unsafe extern "C" fn memam_relation_vacuum(rel: Relation, params: *mut VacuumParams, bstrategy: BufferAccessStrategy) {

}

pub unsafe extern "C" fn memam_scan_analyze_next_block(scan: TableScanDesc, blockno: BlockNumber, bstrategy: BufferAccessStrategy) -> bool {
	return false;
}

pub unsafe extern "C" fn memam_scan_analyze_next_tuple(scan: TableScanDesc, OldestXmin: TransactionId, liverows: *mut f64, deadrows: *mut f64, slot: *mut TupleTableSlot) -> bool {
	return false;
}

pub unsafe extern "C" fn memam_index_build_range_scan(table_rel: Relation, index_rel: Relation, index_info: *mut IndexInfo, allow_sync: bool, anyvisible: bool, progress: bool, start_blockno: BlockNumber, numblocks: BlockNumber, callback: IndexBuildCallback, callback_state: *mut c_void, scan: TableScanDesc) -> f64 {
	return 0.0;
}

pub unsafe extern "C" fn memam_index_validate_scan(table_rel: Relation, index_rel: Relation, index_info: *mut IndexInfo, snapshot: Snapshot, state: *mut ValidateIndexState) {

}

pub unsafe extern "C" fn memam_relation_size(rel: Relation, forkNumber: ForkNumber) -> uint64 {
	return 0;
}

pub unsafe extern "C" fn memam_relation_needs_toast_table(rel: Relation) -> bool {
	return false;
}

pub unsafe extern "C" fn memam_relation_toast_am(rel: Relation) -> Oid {
	return Oid::INVALID;
}

pub unsafe extern "C" fn memam_relation_fetch_toast_slice(toastrel: Relation, valueid: Oid, attrsize: int32, sliceoffset: int32, slicelength: int32, result: *mut varlena) {

}

pub unsafe extern "C" fn memam_relation_estimate_size(rel: Relation, attr_widths: *mut int32, pages: *mut BlockNumber, tuples: *mut f64, allvisfrac: *mut f64) {

}

pub unsafe extern "C" fn memam_scan_bitmap_next_block(scan: TableScanDesc, tbmres: *mut TBMIterateResult) -> bool {
	return false;
}

pub unsafe extern "C" fn memam_scan_bitmap_next_tuple(scan: TableScanDesc, tbmres: *mut TBMIterateResult, slot: *mut TupleTableSlot) -> bool {
	return false;
}

pub unsafe extern "C" fn memam_scan_sample_next_block(scan: TableScanDesc, scanstate: *mut SampleScanState) -> bool {
	return false;
}

pub unsafe extern "C" fn memam_scan_sample_next_tuple(scan: TableScanDesc, scanstate: *mut SampleScanState, slot: *mut TupleTableSlot) -> bool {
	return false;
}
