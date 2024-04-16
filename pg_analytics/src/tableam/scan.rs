use async_std::sync::Mutex;
use async_std::task;
use core::ffi::c_int;
use deltalake::arrow::datatypes::Int64Type;
use deltalake::datafusion::common::arrow::array::{AsArray, Int64Array, RecordBatch};
use deltalake::datafusion::common::arrow::error::ArrowError;
use once_cell::sync::Lazy;
use pgrx::*;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;
use thiserror::Error;

use crate::datafusion::batch::PostgresBatch;
use crate::datafusion::catalog::CatalogError;
use crate::datafusion::stream::Stream;
use crate::datafusion::table::{DataFusionTableError, DatafusionTable};
use crate::datafusion::writer::Writer;
use crate::storage::tid::{RowNumber, TIDError};
use crate::types::datatype::DataTypeError;
use crate::types::datum::GetDatum;

use super::index::index_fetch_tuple;

pub static SCAN_MEMORY_CONTEXT: Lazy<Mutex<AtomicPtr<pg_sys::MemoryContextData>>> =
    Lazy::new(|| {
        Mutex::new(AtomicPtr::new(
            PgMemoryContexts::new("scan_memory_context").value(),
        ))
    });

struct DeltalakeScanDesc {
    rs_base: pg_sys::TableScanDescData,
    curr_batch: Option<Arc<Mutex<RecordBatch>>>,
    tids: Option<Arc<Mutex<Int64Array>>>,
    curr_batch_idx: usize,
}

#[inline]
async fn scan_begin(
    rel: pg_sys::Relation,
    snapshot: pg_sys::Snapshot,
    nkeys: c_int,
    key: *mut pg_sys::ScanKeyData,
    pscan: pg_sys::ParallelTableScanDesc,
    flags: pg_sys::uint32,
) -> Result<pg_sys::TableScanDesc, TableScanError> {
    Writer::flush().await?;

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
            scan.tids = None;

            Ok(scan.into_pg() as pg_sys::TableScanDesc)
        })
    }
}

#[inline]
pub async unsafe fn scan_getnextslot(
    scan: pg_sys::TableScanDesc,
    slot: *mut pg_sys::TupleTableSlot,
) -> Result<bool, TableScanError> {
    let memctx = SCAN_MEMORY_CONTEXT.lock().await.load(Ordering::SeqCst);
    let old_context = pg_sys::MemoryContextSwitchTo(memctx);

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
    let tuple_desc = pg_relation.tuple_desc();
    let table_name = pg_relation.name();
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

        let mut next_batch =
            match Stream::get_next_batch(&table_path, schema_name, table_name).await? {
                Some(batch) => batch,
                None => return Ok(false),
            };

        next_batch.remove_xmin_column()?;
        next_batch.remove_xmax_column()?;

        let tids = next_batch.remove_tid_column()?;
        let tid_array = tids.as_primitive::<Int64Type>();
        (*dscan).curr_batch = Some(Arc::new(Mutex::new(next_batch)));
        (*dscan).tids = Some(Arc::new(Mutex::new(tid_array.clone())));
    }

    let current_batch = (*dscan)
        .curr_batch
        .as_mut()
        .ok_or(TableScanError::RecordBatchNotFound)?
        .lock()
        .await;

    let tids = (*dscan)
        .tids
        .as_mut()
        .ok_or(TableScanError::TIDNotFound)?
        .lock()
        .await;

    for col_index in 0..current_batch.num_columns() {
        let column = current_batch.column(col_index);

        unsafe {
            let attribute = tuple_desc
                .get(col_index)
                .ok_or(TableScanError::AttributeNotFound(col_index))?;
            let tts_value = (*slot).tts_values.add(col_index);
            let tts_isnull = (*slot).tts_isnull.add(col_index);

            if let Some(datum) = column.get_datum(
                (*dscan).curr_batch_idx,
                attribute.type_oid(),
                attribute.type_mod(),
            )? {
                *tts_value = datum;
            } else {
                *tts_isnull = true;
            }
        }
    }

    let row_number = tids.value((*dscan).curr_batch_idx);
    let tts_tid = pg_sys::ItemPointerData::try_from(RowNumber(row_number))?;

    (*slot).tts_tid = tts_tid;
    pg_sys::ExecStoreVirtualTuple(slot);

    (*dscan).curr_batch_idx += 1;

    pg_sys::MemoryContextReset(memctx);
    pg_sys::MemoryContextSwitchTo(old_context);

    Ok(true)
}

#[inline]
async fn scan_end(scan: pg_sys::TableScanDesc) -> Result<(), TableScanError> {
    let dscan = scan as *mut DeltalakeScanDesc;
    let pg_relation = unsafe { PgRelation::from_pg((*dscan).rs_base.rs_rd) };
    let table_path = pg_relation.table_path()?;

    Stream::clear(&table_path).await;

    Ok(())
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
    task::block_on(scan_begin(rel, snapshot, nkeys, key, pscan, flags)).unwrap_or_else(|err| {
        panic!("{}", err);
    })
}

#[pg_guard]
pub extern "C" fn deltalake_scan_end(scan: pg_sys::TableScanDesc) {
    task::block_on(scan_end(scan)).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}

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
pub extern "C" fn deltalake_parallelscan_estimate(_rel: pg_sys::Relation) -> pg_sys::Size {
    panic!("{}", TableScanError::ParallelScanNotSupported.to_string());
}

#[pg_guard]
pub extern "C" fn deltalake_parallelscan_initialize(
    _rel: pg_sys::Relation,
    _pscan: pg_sys::ParallelTableScanDesc,
) -> pg_sys::Size {
    panic!("{}", TableScanError::ParallelScanNotSupported.to_string());
}

#[pg_guard]
pub extern "C" fn deltalake_parallelscan_reinitialize(
    _rel: pg_sys::Relation,
    _pscan: pg_sys::ParallelTableScanDesc,
) {
    panic!("{}", TableScanError::ParallelScanNotSupported.to_string());
}

#[pg_guard]
pub extern "C" fn deltalake_scan_analyze_next_block(
    _scan: pg_sys::TableScanDesc,
    _blockno: pg_sys::BlockNumber,
    _bstrategy: pg_sys::BufferAccessStrategy,
) -> bool {
    true
}

#[pg_guard]
pub extern "C" fn deltalake_scan_analyze_next_tuple(
    scan: pg_sys::TableScanDesc,
    _OldestXmin: pg_sys::TransactionId,
    liverows: *mut f64,
    _deadrows: *mut f64,
    slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    unsafe {
        let next_slot = task::block_on(scan_getnextslot(scan, slot)).unwrap_or_else(|err| {
            panic!("{}", err);
        });

        if next_slot {
            (*liverows) += 1.0;
            return true;
        }
    }

    false
}

#[pg_guard]
pub extern "C" fn deltalake_scan_sample_next_block(
    _scan: pg_sys::TableScanDesc,
    _scanstate: *mut pg_sys::SampleScanState,
) -> bool {
    panic!(
        "{}",
        TableScanError::SampleNextBlockNotSupported.to_string()
    );
}

#[pg_guard]
pub extern "C" fn deltalake_scan_sample_next_tuple(
    _scan: pg_sys::TableScanDesc,
    _scanstate: *mut pg_sys::SampleScanState,
    _slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    panic!(
        "{}",
        TableScanError::SampleNextTupleNotSupported.to_string()
    );
}

#[pg_guard]
pub extern "C" fn deltalake_tuple_fetch_row_version(
    rel: pg_sys::Relation,
    tid: pg_sys::ItemPointer,
    _snapshot: pg_sys::Snapshot,
    slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    task::block_on(Writer::flush()).unwrap_or_else(|err| {
        panic!("{}", err);
    });

    unsafe {
        task::block_on(index_fetch_tuple(rel, slot, tid)).unwrap_or_else(|err| {
            panic!("{}", err);
        })
    }
}

#[pg_guard]
pub extern "C" fn deltalake_tuple_tid_valid(
    _scan: pg_sys::TableScanDesc,
    _tid: pg_sys::ItemPointer,
) -> bool {
    panic!("{}", TableScanError::TIDValidNotSupported.to_string());
}

#[pg_guard]
pub extern "C" fn deltalake_tuple_get_latest_tid(
    _scan: pg_sys::TableScanDesc,
    _tid: pg_sys::ItemPointer,
) {
    panic!("{}", TableScanError::LatestTIDNotSupported.to_string());
}

/*
    Tech debt: We don't use Snapshot to determine which tuples are visible (although we eventually should)
    As such, any tuple that is returned by our table scan is visible, since the table scan filters out
    non-visible tuples.
*/
#[pg_guard]
pub extern "C" fn deltalake_tuple_satisfies_snapshot(
    _rel: pg_sys::Relation,
    _slot: *mut pg_sys::TupleTableSlot,
    _snapshot: pg_sys::Snapshot,
) -> bool {
    true
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
    rel: pg_sys::Relation,
    tid: pg_sys::ItemPointer,
    _snapshot: pg_sys::Snapshot,
    slot: *mut pg_sys::TupleTableSlot,
    _cid: pg_sys::CommandId,
    _mode: pg_sys::LockTupleMode,
    _wait_policy: pg_sys::LockWaitPolicy,
    _flags: pg_sys::uint8,
    _tmfd: *mut pg_sys::TM_FailureData,
) -> pg_sys::TM_Result {
    unsafe {
        task::block_on(index_fetch_tuple(rel, slot, tid)).unwrap_or_else(|err| {
            panic!("{}", err);
        });
    }

    pg_sys::TM_Result_TM_Ok
}

#[derive(Error, Debug)]
pub enum TableScanError {
    #[error(transparent)]
    ArrowError(#[from] ArrowError),

    #[error(transparent)]
    CatalogError(#[from] CatalogError),

    #[error(transparent)]
    DataFusionTableError(#[from] DataFusionTableError),

    #[error(transparent)]
    DataTypeError(#[from] DataTypeError),

    #[error(transparent)]
    TIDError(#[from] TIDError),

    #[error("Could not find attribute {0} in tuple descriptor")]
    AttributeNotFound(usize),

    #[error("Parallel scans are not implemented")]
    ParallelScanNotSupported,

    #[error("TupleTableSlotOps not found in table scan")]
    SlotOpsNotFound,

    #[error("Unexpected error: No RecordBatch found in table scan")]
    RecordBatchNotFound,

    #[error("sample_next_block not implemented")]
    SampleNextBlockNotSupported,

    #[error("sample_next_tuple not implemented")]
    SampleNextTupleNotSupported,

    #[error("tuple_tid_valid not implemented")]
    TIDValidNotSupported,

    #[error("get_latest_tid not implemented")]
    LatestTIDNotSupported,

    #[error("Unexpected error: No TID found in table scan")]
    TIDNotFound,
}
