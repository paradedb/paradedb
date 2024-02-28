/*
    Scans return tuples from our table to Postgres.
    Because we intercept SELECT queries, not all scan functions need to be implemented.
    The ones implemented are called as part of DELETE and UPDATE operations.
*/

use async_std::task;
use core::ffi::c_int;
use deltalake::datafusion::common::arrow::array::RecordBatch;
use pgrx::*;
use std::any::type_name;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::datatype::{DatafusionMapProducer, DatafusionTypeTranslator};
use crate::datafusion::table::DatafusionTable;
use crate::errors::{NotFound, ParadeError};

struct DeltalakeScanDesc {
    rs_base: pg_sys::TableScanDescData,
    curr_batch: Option<Arc<RecordBatch>>,
    curr_batch_idx: usize,
}

#[pg_guard]
pub extern "C" fn deltalake_scan_begin(
    rel: pg_sys::Relation,
    snapshot: pg_sys::Snapshot,
    nkeys: c_int,
    key: *mut pg_sys::ScanKeyData,
    pscan: pg_sys::ParallelTableScanDesc,
    flags: pg_sys::uint32,
) -> pg_sys::TableScanDesc {
    delta_scan_begin_impl(rel, snapshot, nkeys, key, pscan, flags).expect("Failed to begin scan")
}

#[inline]
fn delta_scan_begin_impl(
    rel: pg_sys::Relation,
    snapshot: pg_sys::Snapshot,
    nkeys: c_int,
    key: *mut pg_sys::ScanKeyData,
    pscan: pg_sys::ParallelTableScanDesc,
    flags: pg_sys::uint32,
) -> Result<pg_sys::TableScanDesc, ParadeError> {
    unsafe {
        PgMemoryContexts::CurrentMemoryContext.switch_to(|_context| {
            let mut scan = PgBox::<DeltalakeScanDesc>::alloc0();
            scan.rs_base.rs_rd = rel;
            scan.rs_base.rs_snapshot = snapshot;
            scan.rs_base.rs_nkeys = nkeys;
            scan.rs_base.rs_key = key;
            scan.rs_base.rs_parallel = pscan;
            scan.rs_base.rs_flags = flags;

            scan.curr_batch = None;
            scan.curr_batch_idx = 0;

            Ok(scan.into_pg() as pg_sys::TableScanDesc)
        })
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
pub unsafe extern "C" fn deltalake_scan_getnextslot(
    scan: pg_sys::TableScanDesc,
    _direction: pg_sys::ScanDirection,
    slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    info!("get next slot");
    unsafe { deltalake_scan_getnextslot_impl(scan, slot).expect("Failed to get next slot") }
}

#[inline]
unsafe fn deltalake_scan_getnextslot_impl(
    scan: pg_sys::TableScanDesc,
    slot: *mut pg_sys::TupleTableSlot,
) -> Result<bool, ParadeError> {
    if let Some(clear) = (*slot)
        .tts_ops
        .as_ref()
        .ok_or(NotFound::Value(
            type_name::<pg_sys::TupleTableSlotOps>().to_string(),
        ))?
        .clear
    {
        clear(slot);
    }

    let dscan = scan as *mut DeltalakeScanDesc;
    let pg_relation = unsafe { PgRelation::from_pg((*dscan).rs_base.rs_rd) };
    let schema_name = pg_relation.namespace();
    let table_path = pg_relation.table_path()?;

    if (*dscan).curr_batch.is_none()
        || (*dscan).curr_batch_idx
            >= (*dscan)
                .curr_batch
                .as_ref()
                .ok_or(NotFound::Value(type_name::<RecordBatch>().to_string()))?
                .num_rows()
    {
        (*dscan).curr_batch_idx = 0;

        (*dscan).curr_batch = match DatafusionContext::with_streams(schema_name, |mut streams| {
            task::block_on(streams.get_next_batch(schema_name, &table_path))
        })? {
            Some(batch) => Some(Arc::new(batch)),
            None => return Ok(false),
        };
    }

    let current_batch = (*dscan)
        .curr_batch
        .as_ref()
        .ok_or(NotFound::Value(type_name::<RecordBatch>().to_string()))?;

    for (col_index, column) in current_batch.columns().iter().enumerate() {
        let dt = column.data_type();
        unsafe {
            let tts_value = (*slot).tts_values.add(col_index);
            if let Some(datum) = DatafusionMapProducer::index_datum(
                dt.to_sql_data_type()?,
                column,
                (*dscan).curr_batch_idx,
            )? {
                *tts_value = datum;
            }
        }
    }

    pg_sys::ExecStoreVirtualTuple(slot);

    (*dscan).curr_batch_idx += 1;
    Ok(true)
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
