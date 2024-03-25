/*
    Scans return tuples from our table to Postgres.
    Because we intercept SELECT queries, not all scan functions need to be implemented.
    The ones implemented are called as part of DELETE and UPDATE operations.
*/

use async_std::sync::Mutex;
use async_std::task;
use core::ffi::c_int;
use deltalake::arrow::datatypes::Int64Type;
use deltalake::datafusion::common::arrow::array::{AsArray, RecordBatch};
use pgrx::*;
use shared::postgres::tid::{RowNumber, TIDError};
use std::any::type_name;
use std::sync::Arc;
use thiserror::Error;

use crate::datafusion::batch::{PostgresBatch, RecordBatchError};
use crate::datafusion::stream::Stream;
use crate::datafusion::table::DatafusionTable;
use crate::errors::ParadeError;
use crate::types::datatype::DataTypeError;
use crate::types::datum::GetDatum;

struct DeltalakeScanDesc {
    rs_base: pg_sys::TableScanDescData,
    curr_batch: Option<Arc<Mutex<RecordBatch>>>,
    curr_batch_idx: usize,
}

#[inline]
fn scan_begin(
    rel: pg_sys::Relation,
    snapshot: pg_sys::Snapshot,
    nkeys: c_int,
    key: *mut pg_sys::ScanKeyData,
    pscan: pg_sys::ParallelTableScanDesc,
    flags: pg_sys::uint32,
) -> Result<pg_sys::TableScanDesc, TableScanError> {
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

#[inline]
pub async unsafe fn scan_getnextslot(
    scan: pg_sys::TableScanDesc,
    slot: *mut pg_sys::TupleTableSlot,
) -> Result<bool, TableScanError> {
    if let Some(clear) = (*slot)
        .tts_ops
        .as_ref()
        .ok_or(TableScanError::SlotOpsNotFound)?
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
                .ok_or(TableScanError::RecordBatchNotFound)?
                .lock()
                .await
                .num_rows()
    {
        (*dscan).curr_batch_idx = 0;

        (*dscan).curr_batch = match Stream::get_next_batch(schema_name, &table_path).await? {
            Some(batch) => Some(Arc::new(Mutex::new(batch))),
            None => return Ok(false),
        };
    }

    let mut current_batch = (*dscan)
        .curr_batch
        .as_mut()
        .ok_or(TableScanError::RecordBatchNotFound)?
        .lock()
        .await;

    for col_index in 0..current_batch.num_columns() {
        let column = current_batch.column(col_index);

        unsafe {
            let tts_value = (*slot).tts_values.add(col_index);
            let tts_isnull = (*slot).tts_isnull.add(col_index);

            if let Some(datum) = column.get_datum((*dscan).curr_batch_idx)? {
                *tts_value = datum;
            } else {
                *tts_isnull = true;
            }
        }
    }

    let tids = current_batch.remove_tid_column()?;
    let row_number = tids.as_primitive::<Int64Type>().value((*dscan).curr_batch_idx);
    let tts_tid = pg_sys::ItemPointerData::try_from(RowNumber(row_number))?;

    (*slot).tts_tid = tts_tid;
    pg_sys::ExecStoreVirtualTuple(slot);

    (*dscan).curr_batch_idx += 1;
    Ok(true)
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
    scan_begin(rel, snapshot, nkeys, key, pscan, flags).unwrap_or_else(|err| {
        panic!("{}", err);
    })
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
    unsafe {
        task::block_on(scan_getnextslot(scan, slot)).unwrap_or_else(|err| {
            panic!("{}", err);
        })
    }
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

#[derive(Error, Debug)]
pub enum TableScanError {
    #[error(transparent)]
    DataTypeError(#[from] DataTypeError),

    #[error(transparent)]
    ParadeError(#[from] ParadeError),

    #[error(transparent)]
    RecordBatchError(#[from] RecordBatchError),

    #[error(transparent)]
    TIDError(#[from] TIDError),

    #[error("TupleTableSlotOps not found in table scan")]
    SlotOpsNotFound,

    #[error("Unexpected error: No RecordBatch found in table scan")]
    RecordBatchNotFound
}
