use async_std::task;
use core::ffi::{c_int, c_void};
use deltalake::datafusion::common::arrow::array::RecordBatch;
use pgrx::*;
use std::any::type_name;
use std::sync::Arc;

use crate::datafusion::stream::Stream;
use crate::datafusion::table::DatafusionTable;
use crate::errors::{NotFound, ParadeError};
use crate::types::datum::GetDatum;

struct IndexScanDesc {
    rs_base: pg_sys::IndexFetchTableData,
    curr_batch: Option<Arc<RecordBatch>>,
    curr_batch_idx: usize,
    completed: bool,
}

#[pg_guard]
pub extern "C" fn deltalake_index_fetch_begin(
    rel: pg_sys::Relation,
) -> *mut pg_sys::IndexFetchTableData {
    unsafe {
        PgMemoryContexts::CurrentMemoryContext.switch_to(|_context| {
            let mut scan = PgBox::<IndexScanDesc>::alloc0();
            scan.rs_base.rel = rel;
            scan.curr_batch = None;
            scan.curr_batch_idx = 0;
            scan.completed = false;

            scan.into_pg() as *mut pg_sys::IndexFetchTableData
        })
    }
}

#[pg_guard]
pub extern "C" fn deltalake_index_fetch_reset(_data: *mut pg_sys::IndexFetchTableData) {}

#[pg_guard]
pub extern "C" fn deltalake_index_fetch_end(_data: *mut pg_sys::IndexFetchTableData) {
    info!("fetch end");
}

#[pg_guard]
pub extern "C" fn deltalake_index_fetch_tuple(
    scan: *mut pg_sys::IndexFetchTableData,
    tid: pg_sys::ItemPointer,
    _snapshot: pg_sys::Snapshot,
    slot: *mut pg_sys::TupleTableSlot,
    call_again: *mut bool,
    all_dead: *mut bool,
) -> bool {
    unsafe {
        // *call_again = false;

        // if !all_dead.is_null() {
        //     *all_dead = false;
        // }

        task::block_on(index_fetch_tuple_impl(scan, slot, tid)).expect("Failed to get next slot")
    }
}

#[inline]
async unsafe fn index_fetch_tuple_impl(
    scan: *mut pg_sys::IndexFetchTableData,
    slot: *mut pg_sys::TupleTableSlot,
    tid: pg_sys::ItemPointer,
) -> Result<bool, ParadeError> {
    info!("fetch tuple");
    let dscan = scan as *mut IndexScanDesc;
    if (*dscan).completed {
        return Ok(false);
    }

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

    let pg_relation = unsafe { PgRelation::from_pg((*dscan).rs_base.rel) };
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

        (*dscan).curr_batch = match Stream::get_next_batch(schema_name, &table_path).await? {
            Some(batch) => Some(Arc::new(batch)),
            None => {
                (*dscan).completed = true;
                return Ok(false);
            }
        };
    }

    let current_batch = (*dscan)
        .curr_batch
        .as_ref()
        .ok_or(NotFound::Value(type_name::<RecordBatch>().to_string()))?;

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

    (*slot).tts_tid = *tid;
    pg_sys::ExecStoreVirtualTuple(slot);

    (*dscan).curr_batch_idx += 1;

    Ok(true)
}

#[pg_guard]
#[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
pub extern "C" fn deltalake_index_delete_tuples(
    _rel: pg_sys::Relation,
    _delstate: *mut pg_sys::TM_IndexDeleteOp,
) -> pg_sys::TransactionId {
    0
}

#[pg_guard]
pub extern "C" fn deltalake_index_build_range_scan(
    _table_rel: pg_sys::Relation,
    _index_rel: pg_sys::Relation,
    _index_info: *mut pg_sys::IndexInfo,
    _allow_sync: bool,
    _anyvisible: bool,
    _progress: bool,
    _start_blockno: pg_sys::BlockNumber,
    _numblocks: pg_sys::BlockNumber,
    _callback: pg_sys::IndexBuildCallback,
    _callback_state: *mut c_void,
    _scan: pg_sys::TableScanDesc,
) -> f64 {
    0.0
}

#[pg_guard]
pub extern "C" fn deltalake_index_validate_scan(
    _table_rel: pg_sys::Relation,
    _index_rel: pg_sys::Relation,
    _index_info: *mut pg_sys::IndexInfo,
    _snapshot: pg_sys::Snapshot,
    _state: *mut pg_sys::ValidateIndexState,
) {
    info!("validate scan");
}
