#![allow(unused)]
#![allow(non_snake_case)]

use crate::CONTEXT;
use async_std::stream::StreamExt;
use async_std::task;
use core::ffi::c_char;
use core::ffi::c_int;
use core::ffi::c_void;
use datafusion::arrow::array::{Array, ArrayIter, AsArray, Int32Array, PrimitiveArray, Scalar};
use datafusion::arrow::datatypes::{DataType, Field, Int32Type, Schema, SchemaRef};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::datasource::{DefaultTableSource, MemTable, TableProvider};
use datafusion::error::Result;
use datafusion::execution::context::SessionState;
use datafusion::execution::runtime_env::RuntimeEnv;
use datafusion::execution::{RecordBatchStream, SendableRecordBatchStream};
use datafusion::logical_expr::LogicalPlanBuilder;
use datafusion::physical_plan::memory::MemoryExec;
use datafusion::physical_plan::ExecutionPlan;
use datafusion::physical_planner::{DefaultPhysicalPlanner, PhysicalPlanner};
use datafusion::prelude::{SessionConfig, SessionContext};
use datafusion::sql::TableReference;
use pgrx::pg_sys::*;
use pgrx::IntoDatum;
use pgrx::{FromDatum, PgBox};
use shared::plog;
use std::ptr;
use std::ptr::copy_nonoverlapping;
use std::sync::Arc;
use pgrx::pgbox::AllocatedByRust;

unsafe fn get_table_from_relation(rel: Relation) -> Result<Arc<dyn TableProvider>> {
    let table_name = name_data_to_str(&(*(*rel).rd_rel).relname);
    info!("getting table {}", table_name);
    let table_ref = TableReference::from(table_name);
    task::block_on(CONTEXT.table_provider(table_ref))
}

pub unsafe extern "C" fn memam_slot_callbacks(rel: Relation) -> *const TupleTableSlotOps {
    return &TTSOpsVirtual;
}

// custom DescData representing scan state
struct TableAMScanDescData {
    rs_base: PgBox::<TableScanDescData, AllocatedByRust>,
    stream: Option<SendableRecordBatchStream>, // should this be option: None if scan failed
    curr_batch: Option<RecordBatch>,
}

// TODO: add back other args
// TODO: what should we do if we get a DataFusionError?
async unsafe fn memam_scan_begin_impl(rel: Relation) -> TableScanDesc {
    // let mut scan = unsafe { PgBox::<TableAMScanDescData>::alloc0() };
    let mut scan: TableAMScanDescData = TableAMScanDescData {
        rs_base: unsafe { PgBox::<TableScanDescData>::alloc0() },
        stream: None,
        curr_batch: None,
    };
    scan.rs_base.rs_rd = rel;
    scan.curr_batch = None;
    let table = get_table_from_relation(rel);
    match table {
        Ok(tab) => {
            info!("found table!");
            let scan_exec_plan = tab
                .scan(&CONTEXT.state(), None, &[], None)
                .await
                .map(|plan| {
                    info!("started scan, executing now");
                    plan.execute(0, CONTEXT.task_ctx())
                });
            // TODO how do deal with all these results
            match scan_exec_plan {
                Ok(Ok(stream)) => {
                    info!("scan successful, got stream");
                    scan.stream = Some(stream)
                }
                Err(e) => info!("{:?}", e),
                Ok(Err(e)) => info!("{:?}", e),
            }
            // scan.stream = None;
        }
        Err(e) => info!("{:?}", e),
    }
    info!("casting now");
    // TODO: how do I cast this boi
    return scan.rs_base.into_pg();
}

pub unsafe extern "C" fn memam_scan_begin(
    rel: Relation,
    snapshot: Snapshot,
    nkeys: c_int,
    key: *mut ScanKeyData,
    pscan: ParallelTableScanDesc,
    flags: uint32,
) -> TableScanDesc {
    info!("Calling memam_relation_scan_begin");
    // let mut scan = unsafe { PgBox::<TableScanDescData>::alloc0() };
    task::block_on(memam_scan_begin_impl(rel))
}

pub unsafe extern "C" fn memam_scan_end(scan: TableScanDesc) {
    info!("Calling memam_scan_end");
}

pub unsafe extern "C" fn memam_scan_rescan(
    scan: TableScanDesc,
    key: *mut ScanKeyData,
    set_params: bool,
    allow_strat: bool,
    allow_sync: bool,
    allow_pagemode: bool,
) {
    info!("Calling memam_scan_rescan");
}

async unsafe fn memam_scan_getnextslot_impl(
    tscan: *mut TableAMScanDescData,
    slot: *mut TupleTableSlot,
) -> bool {
    info!("Calling memam_scan_getnextslot_impl");
    if (*tscan).stream.is_none() {
        info!("returning false");
        return false;
    }
    // TODO: quickfix said to as_mut() this
    let mut stream = &(*tscan).stream;
    if (*tscan).curr_batch.is_none() {
        info!("curr_batch is none");
        let next_batch = stream.unwrap().next().await;
        info!("here a");
        match next_batch {
            Some(Ok(batch)) => {
                info!("here b");
                (*tscan).curr_batch = Some(batch);
                info!("here c");
            }
            _ => (),
        };
    }
    if (*tscan).curr_batch.is_none() {
        info!("returning false 2");
        return false;
    }
    info!("here 1");
    // TODO: quickfix said to clone this, is that ok?
    let batch = (*tscan).curr_batch.clone().unwrap();
    let batch_len = batch.num_rows();
    if batch_len > 0 {
        // the batch is 2-dimensional! :( I only want the first guy in it
        let single_batch = batch.slice(0, 1);
        (*tscan).curr_batch = Some(batch.slice(1, batch_len - 1));
        let mut col_index = 0;
        for col in single_batch.columns() {
            info!("here col {}", col_index);
            // TODO: casework based on data type and put it into slot and put it into slot
            match col.data_type() {
                DataType::Int32 => {
                    let prim: &PrimitiveArray<Int32Type> = col.as_primitive();
                    let value_datum: Datum = prim.value(0).into_datum().unwrap();
                    info!("found value {:?} in col {}", value_datum, col_index);
                    // TODO: actually figure out whether null or not
                    let value_isnull = false;
                    copy_nonoverlapping::<Datum>(
                        &value_datum,
                        (*slot).tts_values.offset(col_index),
                        1,
                    );
                    copy_nonoverlapping::<bool>(
                        &value_isnull,
                        (*slot).tts_isnull.offset(col_index),
                        1,
                    );
                }
                _ => (),
            }
            col_index += 1;
        }
        return true;
    } else {
        return false;
    }
}
pub unsafe extern "C" fn memam_scan_getnextslot(
    scan: TableScanDesc,
    direction: ScanDirection,
    slot: *mut TupleTableSlot,
) -> bool {
    info!("Calling memam_relation_scan_getnextslot");
    static mut done: bool = false;

    if done {
        return false;
    }

    // cast scan as TableAMScanDescData
    let tscan = scan as *mut TableAMScanDescData;
    // TODO: grab the stream and get next

    task::block_on(memam_scan_getnextslot_impl(tscan, slot))

    /*
    // TODO: Use RecordBatch::try_new to create new rows (replace dummy 314)
    let value: int32 = 314;
    let value_datum: Datum = value.into_datum().unwrap();
    let value_isnull: bool = false;

    copy_nonoverlapping::<Datum>(&value_datum, (*slot).tts_values.offset(0), 1);
    copy_nonoverlapping::<bool>(&value_isnull, (*slot).tts_isnull.offset(0), 1);
    ExecStoreVirtualTuple(slot);

    done = true;

    return true;
    */
}

pub unsafe extern "C" fn memam_scan_set_tidrange(
    scan: TableScanDesc,
    mintid: ItemPointer,
    maxtid: ItemPointer,
) {
    info!("Calling memam_scan_set_tidrange");
}

pub unsafe extern "C" fn memam_scan_getnextslot_tidrange(
    scan: TableScanDesc,
    direction: ScanDirection,
    slot: *mut TupleTableSlot,
) -> bool {
    info!("Calling memam_scan_getnextslot_tidrange");
    return false;
}

pub unsafe extern "C" fn memam_parallelscan_estimate(rel: Relation) -> Size {
    info!("Calling memam_parallelscan_estimate");
    return table_block_parallelscan_estimate(rel);
}

pub unsafe extern "C" fn memam_parallelscan_initialize(
    rel: Relation,
    pscan: ParallelTableScanDesc,
) -> Size {
    info!("Calling memam_parallelscan_initialize");
    return table_block_parallelscan_initialize(rel, pscan);
}

pub unsafe extern "C" fn memam_parallelscan_reinitialize(
    rel: Relation,
    pscan: ParallelTableScanDesc,
) {
    info!("Calling memam_parallelscan_reinitialize");
    return table_block_parallelscan_reinitialize(rel, pscan);
}

pub unsafe extern "C" fn memam_index_fetch_begin(rel: Relation) -> *mut IndexFetchTableData {
    info!("Calling memam_index_fetch_begin");
    return ptr::null_mut::<IndexFetchTableData>();
}

pub unsafe extern "C" fn memam_index_fetch_reset(data: *mut IndexFetchTableData) {
    info!("Calling memam_index_fetch_reset");
}

pub unsafe extern "C" fn memam_index_fetch_end(data: *mut IndexFetchTableData) {
    info!("Calling memam_index_fetch_end");
}

pub unsafe extern "C" fn memam_index_fetch_tuple(
    scan: *mut IndexFetchTableData,
    tid: ItemPointer,
    snapshot: Snapshot,
    slot: *mut TupleTableSlot,
    call_again: *mut bool,
    all_dead: *mut bool,
) -> bool {
    info!("Calling memam_index_fetch_tuple");
    return false;
}

pub unsafe extern "C" fn memam_tuple_fetch_row_version(
    rel: Relation,
    tid: ItemPointer,
    snapshot: Snapshot,
    slot: *mut TupleTableSlot,
) -> bool {
    info!("Calling memam_tuple_fetch_row_version");
    return false;
}

pub unsafe extern "C" fn memam_tuple_tid_valid(scan: TableScanDesc, tid: ItemPointer) -> bool {
    info!("Calling memam_tuple_tid_valid");
    return false;
}

pub unsafe extern "C" fn memam_tuple_get_latest_tid(scan: TableScanDesc, tid: ItemPointer) {
    info!("Calling memam_tuple_get_latest_tid");
}

pub unsafe extern "C" fn memam_tuple_satisfies_snapshot(
    rel: Relation,
    slot: *mut TupleTableSlot,
    snapshot: Snapshot,
) -> bool {
    info!("Calling memam_tuple_satisfies_snapshot");
    return false;
}

pub unsafe extern "C" fn memam_index_delete_tuples(
    rel: Relation,
    delstate: *mut TM_IndexDeleteOp,
) -> TransactionId {
    info!("Calling memam_index_delete_tuples");
    return 0;
}

// exec_plan contains the recordbatch of tuple to insert
async unsafe fn memam_tuple_insert_impl(rel: Relation, exec_plan: MemoryExec) {
    let table = get_table_from_relation(rel);
    if let Ok(provider) = table {
        // TODO: correct to use session context's state?
        provider.insert_into(&CONTEXT.state(), Arc::new(exec_plan), false);
    }
}

pub unsafe extern "C" fn memam_tuple_insert(
    rel: Relation,
    slot: *mut TupleTableSlot,
    cid: CommandId,
    options: c_int,
    bistate: *mut BulkInsertStateData,
) {
    info!("Calling memam_tuple_insert");
    // TupleDesc desc = RelationGetDescr(relation);
    // get the table name from relation: relation->rd_rel->relname
    // look up the table (hopefully registered using ctx.register_table) using one of their table functions
    // let table_ref = TableReference::from(name_data_to_str(&(*(*rel).rd_rel).relname));
    // use insert_into with the memtable
    // I have to input a SessionState and an ExecutionPlan
    // column.value = DatumGetInt32(slot->tts_values[i]);
    // desc->natts is number of columns in the tuple
    // create a logical plan ?? or use the logical plan builder??
    // to represent insert

    // create a record batch using try_new
    // read the tuple from slot->tts_values?
    // the data is in slot->tts_values: ith entry <-> ith column
    // read tuple desc from it
    // TODO: don't just assume defaults and only read first val
    let num_cols = (*slot).tts_nvalid;
    // let desc = (*slot).tts_tupleDescriptor;
    let vals = (*slot).tts_values;
    info!("{:?}", *vals);
    if num_cols > 0 {
        let id_array = vec![i32::from_datum(*vals, false).unwrap()];
        // create a schema for the recordbatch
        // test: schema is just one column of Int32
        let field = Field::new("a", DataType::Int32, false);
        let schema = SchemaRef::new(Schema::new(vec![field]));
        let batch =
            RecordBatch::try_new(schema, vec![Arc::new(Int32Array::from(id_array))]).unwrap();
        let schema = batch.schema();
        // use MemoryExec to read this recordbatch
        let memory_exec = MemoryExec::try_new(&[vec![batch]], schema.clone(), None);
        if let Ok(exec_plan) = memory_exec {
            task::block_on(memam_tuple_insert_impl(rel, exec_plan));
        }
    }
}

pub unsafe extern "C" fn memam_tuple_insert_speculative(
    rel: Relation,
    slot: *mut TupleTableSlot,
    cid: CommandId,
    options: c_int,
    bistate: *mut BulkInsertStateData,
    specToken: uint32,
) {
    info!("Calling memam_tuple_insert_speculative");
}

pub unsafe extern "C" fn memam_tuple_complete_speculative(
    rel: Relation,
    slot: *mut TupleTableSlot,
    specToken: uint32,
    succeeded: bool,
) {
    info!("Calling memam_tuple_complete_speculative");
}

pub unsafe extern "C" fn memam_multi_insert(
    rel: Relation,
    slots: *mut *mut TupleTableSlot,
    nslots: c_int,
    cid: CommandId,
    options: c_int,
    bistate: *mut BulkInsertStateData,
) {
    info!("Calling memam_multi_insert");
}

pub unsafe extern "C" fn memam_tuple_delete(
    rel: Relation,
    tid: ItemPointer,
    cid: CommandId,
    snapshot: Snapshot,
    crosscheck: Snapshot,
    wait: bool,
    tmfd: *mut TM_FailureData,
    changingPart: bool,
) -> TM_Result {
    info!("Calling memam_tuple_delete");
    return 0;
}

pub unsafe extern "C" fn memam_tuple_update(
    rel: Relation,
    otid: ItemPointer,
    slot: *mut TupleTableSlot,
    cid: CommandId,
    snapshot: Snapshot,
    crosscheck: Snapshot,
    wait: bool,
    tmfd: *mut TM_FailureData,
    lockmode: *mut LockTupleMode,
    update_indexes: *mut bool,
) -> TM_Result {
    info!("Calling memam_tuple_update");
    return 0;
}

pub unsafe extern "C" fn memam_tuple_lock(
    rel: Relation,
    tid: ItemPointer,
    snapshot: Snapshot,
    slot: *mut TupleTableSlot,
    cid: CommandId,
    mode: LockTupleMode,
    wait_policy: LockWaitPolicy,
    flags: uint8,
    tmfd: *mut TM_FailureData,
) -> TM_Result {
    info!("Calling memam_tuple_lock");
    return 0;
}

pub unsafe extern "C" fn memam_finish_bulk_insert(rel: Relation, options: c_int) {
    info!("Calling memam_finish_bulk_insert");
}

pub unsafe extern "C" fn memam_relation_set_new_filenode(
    rel: Relation,
    newrnode: *const RelFileNode,
    persistence: c_char,
    freezeXid: *mut TransactionId,
    minmulti: *mut MultiXactId,
) {
    info!("Calling memam_relation_set_new_filenode");
    // TODO: put proper column names and types here and use vec! instead of ::new
    // TODO: I think we should read through pgrx::tupdesc::PgTupleDesc for how to get the schema
    // for now let's have one column with int32
    let field = Field::new("a", DataType::Int32, false);
    let schema = SchemaRef::new(Schema::new(vec![field]));

    // Empty table
    let mem_table = match MemTable::try_new(schema, vec![Vec::<RecordBatch>::new()]).ok() {
        Some(mem_table) => {
            // let ctx = SessionContext::new();
            CONTEXT.register_table(
                name_data_to_str(&(*(*rel).rd_rel).relname),
                Arc::new(mem_table),
            );
        }
        None => info!("Could not create table"),
    };
}

pub unsafe extern "C" fn memam_relation_nontransactional_truncate(rel: Relation) {
    info!("Calling memam_relation_nontransactional_truncate");
}

pub unsafe extern "C" fn memam_relation_copy_data(rel: Relation, newrnode: *const RelFileNode) {
    info!("Calling memam_relation_copy_data");
}

pub unsafe extern "C" fn memam_relation_copy_for_cluster(
    NewTable: Relation,
    OldTable: Relation,
    OldIndex: Relation,
    use_sort: bool,
    OldestXmin: TransactionId,
    xid_cutoff: *mut TransactionId,
    multi_cutoff: *mut MultiXactId,
    num_tuples: *mut f64,
    tups_vacuumed: *mut f64,
    tups_recently_dead: *mut f64,
) {
    info!("Calling memam_relation_copy_for_cluster");
}

pub unsafe extern "C" fn memam_relation_vacuum(
    rel: Relation,
    params: *mut VacuumParams,
    bstrategy: BufferAccessStrategy,
) {
    info!("Calling memam_relation_vacuum");
}

pub unsafe extern "C" fn memam_scan_analyze_next_block(
    scan: TableScanDesc,
    blockno: BlockNumber,
    bstrategy: BufferAccessStrategy,
) -> bool {
    info!("Calling memam_scan_analyze_next_block");
    return false;
}

pub unsafe extern "C" fn memam_scan_analyze_next_tuple(
    scan: TableScanDesc,
    OldestXmin: TransactionId,
    liverows: *mut f64,
    deadrows: *mut f64,
    slot: *mut TupleTableSlot,
) -> bool {
    info!("Calling memam_scan_analyze_next_tuple");
    return false;
}

pub unsafe extern "C" fn memam_index_build_range_scan(
    table_rel: Relation,
    index_rel: Relation,
    index_info: *mut IndexInfo,
    allow_sync: bool,
    anyvisible: bool,
    progress: bool,
    start_blockno: BlockNumber,
    numblocks: BlockNumber,
    callback: IndexBuildCallback,
    callback_state: *mut c_void,
    scan: TableScanDesc,
) -> f64 {
    info!("Calling memam_index_build_range_scan");
    return 0.0;
}

pub unsafe extern "C" fn memam_index_validate_scan(
    table_rel: Relation,
    index_rel: Relation,
    index_info: *mut IndexInfo,
    snapshot: Snapshot,
    state: *mut ValidateIndexState,
) {
    info!("Calling memam_index_validate_scan");
}

pub unsafe extern "C" fn memam_relation_size(rel: Relation, forkNumber: ForkNumber) -> uint64 {
    info!("Calling memam_relation_size");
    return 0;
}

pub unsafe extern "C" fn memam_relation_needs_toast_table(rel: Relation) -> bool {
    info!("Calling memam_relation_needs_toast_table");
    return false;
}

pub unsafe extern "C" fn memam_relation_toast_am(rel: Relation) -> Oid {
    info!("Calling memam_relation_needs_toast_am");
    return Oid::INVALID;
}

pub unsafe extern "C" fn memam_relation_fetch_toast_slice(
    toastrel: Relation,
    valueid: Oid,
    attrsize: int32,
    sliceoffset: int32,
    slicelength: int32,
    result: *mut varlena,
) {
    info!("Calling memam_relation_fetch_toast_slice");
}

pub unsafe extern "C" fn memam_relation_estimate_size(
    rel: Relation,
    attr_widths: *mut int32,
    pages: *mut BlockNumber,
    tuples: *mut f64,
    allvisfrac: *mut f64,
) {
    info!("Calling memam_relation_estimate_size");
}

pub unsafe extern "C" fn memam_scan_bitmap_next_block(
    scan: TableScanDesc,
    tbmres: *mut TBMIterateResult,
) -> bool {
    info!("Calling memam_scan_bitmap_next_block");
    return false;
}

pub unsafe extern "C" fn memam_scan_bitmap_next_tuple(
    scan: TableScanDesc,
    tbmres: *mut TBMIterateResult,
    slot: *mut TupleTableSlot,
) -> bool {
    info!("Calling memam_scan_bitmap_next_tuple");
    return false;
}

pub unsafe extern "C" fn memam_scan_sample_next_block(
    scan: TableScanDesc,
    scanstate: *mut SampleScanState,
) -> bool {
    info!("Calling memam_scan_sample_next_block");
    return false;
}

pub unsafe extern "C" fn memam_scan_sample_next_tuple(
    scan: TableScanDesc,
    scanstate: *mut SampleScanState,
    slot: *mut TupleTableSlot,
) -> bool {
    info!("Calling memam_scan_sample_next_tuple");
    return false;
}
