use async_std::task;
use core::ffi::{c_int, c_void};
use deltalake::arrow::datatypes::Int64Type;
use deltalake::datafusion::common::arrow::array::{AsArray, RecordBatch};
use deltalake::datafusion::common::DataFusionError;
use deltalake::datafusion::common::ScalarValue;
use deltalake::datafusion::logical_expr::expr::Expr;
use deltalake::datafusion::logical_expr::{col, LogicalPlanBuilder};
use deltalake::datafusion::sql::TableReference;
use pgrx::*;
use shared::postgres::tid::{RowNumber, TIDError};
use std::mem::size_of;
use std::ptr::{addr_of_mut, null_mut};
use std::sync::Arc;
use thiserror::Error;

use super::scan::scan_getnextslot;
use crate::datafusion::session::Session;
use crate::datafusion::stream::Stream;
use crate::datafusion::table::{DatafusionTable, RESERVED_TID_FIELD};
use crate::errors::ParadeError;
use crate::types::datatype::DataTypeError;
use crate::types::datum::GetDatum;

struct IndexScanDesc {
    rs_base: pg_sys::IndexFetchTableData,
}

#[pg_guard]
pub extern "C" fn deltalake_index_fetch_begin(
    rel: pg_sys::Relation,
) -> *mut pg_sys::IndexFetchTableData {
    unsafe {
        PgMemoryContexts::CurrentMemoryContext.switch_to(|_context| {
            let mut scan = PgBox::<IndexScanDesc>::alloc0();
            scan.rs_base.rel = rel;
            scan.into_pg() as *mut pg_sys::IndexFetchTableData
        })
    }
}

#[pg_guard]
pub extern "C" fn deltalake_index_fetch_reset(_data: *mut pg_sys::IndexFetchTableData) {
    info!("fetch reset");
}

#[pg_guard]
pub extern "C" fn deltalake_index_fetch_end(_data: *mut pg_sys::IndexFetchTableData) {
    info!("scan done");
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
        *call_again = false;

        if !all_dead.is_null() {
            *all_dead = false;
        }

        task::block_on(index_fetch_tuple_impl(scan, slot, tid)).unwrap_or_else(|err| {
            panic!("{}", err);
        })
    }
}

#[inline]
async unsafe fn index_fetch_tuple_impl(
    scan: *mut pg_sys::IndexFetchTableData,
    slot: *mut pg_sys::TupleTableSlot,
    tid: pg_sys::ItemPointer,
) -> Result<bool, IndexScanError> {
    let dscan = scan as *mut IndexScanDesc;

    if let Some(clear) = (*slot)
        .tts_ops
        .as_ref()
        .ok_or(IndexScanError::NoTupleTableSlotOps)?
        .clear
    {
        clear(slot);
    }

    let pg_relation = unsafe { PgRelation::from_pg((*dscan).rs_base.rel) };
    let oid = pg_relation.oid();
    let table_name = pg_relation.name().to_string();
    let schema_name = pg_relation.namespace().to_string();
    let catalog_name = Session::catalog_name()?;
    let RowNumber(row_number) = RowNumber::try_from(*tid)?;

    let dataframe = Session::with_session_context(|context| {
        Box::pin(async move {
            let arrow_schema = pg_relation.arrow_schema()?;
            let column_names = arrow_schema
                .fields()
                .iter()
                .map(|field| field.name().as_str())
                .filter(|&name| name != RESERVED_TID_FIELD)
                .collect::<Vec<&str>>();

            let reference = TableReference::full(catalog_name, schema_name, table_name);
            let table = context.table(reference).await?;

            Ok(table
                .filter(col(RESERVED_TID_FIELD).eq(Expr::Literal(ScalarValue::from(row_number))))?
                .select_columns(&column_names)?)
        })
    })?;

    match dataframe.collect().await?.as_slice() {
        [] => Ok(false),
        [batch] => {
            if batch.num_rows() > 1 {
                return Err(IndexScanError::DuplicateRowNumber(row_number));
            }

            for col_index in 0..batch.num_columns() {
                let column = batch.column(col_index);

                unsafe {
                    let tts_value = (*slot).tts_values.add(col_index);
                    let tts_isnull = (*slot).tts_isnull.add(col_index);

                    if let Some(datum) = column.get_datum(0)? {
                        *tts_value = datum;
                    } else {
                        *tts_isnull = true;
                    }
                }
            }

            (*slot).tts_tableOid = oid;
            (*slot).tts_tid = *tid;
            pg_sys::ExecStoreVirtualTuple(slot);

            Ok(true)
        }
        _ => Err(IndexScanError::DuplicateBatch(row_number)),
    }
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
    table_rel: pg_sys::Relation,
    index_rel: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
    allow_sync: bool,
    anyvisible: bool,
    progress: bool,
    start_blockno: pg_sys::BlockNumber,
    numblocks: pg_sys::BlockNumber,
    callback: pg_sys::IndexBuildCallback,
    callback_state: *mut c_void,
    scan: pg_sys::TableScanDesc,
) -> f64 {
    task::block_on(index_build_range_scan(
        table_rel,
        index_rel,
        index_info,
        allow_sync,
        anyvisible,
        progress,
        start_blockno,
        numblocks,
        callback,
        callback_state,
        scan,
    ))
    .unwrap_or_else(|err| {
        panic!("{}", err);
    })
}

#[inline]
async fn index_build_range_scan(
    table_rel: pg_sys::Relation,
    index_rel: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
    allow_sync: bool,
    anyvisible: bool,
    progress: bool,
    start_blockno: pg_sys::BlockNumber,
    numblocks: pg_sys::BlockNumber,
    callback: pg_sys::IndexBuildCallback,
    callback_state: *mut c_void,
    scan: pg_sys::TableScanDesc,
) -> Result<f64, IndexScanError> {
    if start_blockno != 0 || numblocks != pg_sys::InvalidBlockNumber {
        return Err(IndexScanError::IndexNotSupported);
    }

    unsafe {
        let scan = pg_sys::table_beginscan_strat(
            table_rel,
            addr_of_mut!(pg_sys::SnapshotAnyData) as *mut pg_sys::SnapshotData,
            0,
            null_mut(),
            true,
            allow_sync,
        );

        if progress {
            // todo!()
        }

        let executor_state = pg_sys::CreateExecutorState();
        let context = match (*executor_state).es_per_tuple_exprcontext.is_null() {
            true => pg_sys::MakePerTupleExprContext(executor_state),
            false => (*executor_state).es_per_tuple_exprcontext,
        };
        (*context).ecxt_scantuple = pg_sys::table_slot_create(table_rel, null_mut());
        let predicate = pg_sys::ExecPrepareQual((*index_info).ii_Predicate, executor_state);

        let mut tuple_count = 0.0;
        let mut last_block_number = pg_sys::InvalidBlockNumber;
        let slot = (*context).ecxt_scantuple;

        while scan_getnextslot(scan, slot).await? {
            check_for_interrupts!();

            let current_block_number =
                item_pointer_get_block_number(&(*slot).tts_tid as *const pg_sys::ItemPointerData);

            if progress && current_block_number != last_block_number {
                last_block_number = current_block_number;
            }

            pg_sys::MemoryContextReset((*context).ecxt_per_tuple_memory);

            if !predicate.is_null() && !pg_sys::ExecQual(predicate, context) {
                continue;
            }

            let values = pg_sys::palloc0(pg_sys::INDEX_MAX_KEYS as usize * size_of::<pg_sys::Datum>())
                as *mut pg_sys::Datum;
            let nulls = pg_sys::palloc0(pg_sys::INDEX_MAX_KEYS as usize * size_of::<bool>()) as *mut bool;

            pg_sys::FormIndexDatum(index_info, slot, executor_state, values, nulls);

            #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
            if let Some(callback) = callback {
                callback(
                    index_rel,
                    &mut (*slot).tts_tid as *mut pg_sys::ItemPointerData,
                    values,
                    nulls,
                    true,
                    callback_state,
                );
            }

            // #[cfg(feature = "pg12")]
            // todo!();

            tuple_count += 1.0;
        }

        pg_sys::table_endscan(scan);

        if progress {
            // todo!();
        }

        pg_sys::ExecDropSingleTupleTableSlot((*context).ecxt_scantuple);
        pg_sys::FreeExecutorState(executor_state);
        
        (*index_info).ii_PredicateState = null_mut();
        (*index_info).ii_ExpressionsState = null_mut();

        Ok(tuple_count)
    }
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

#[derive(Error, Debug)]
pub enum IndexScanError {
    #[error(transparent)]
    DataFusion(#[from] DataFusionError),

    #[error(transparent)]
    DataType(#[from] DataTypeError),

    #[error(transparent)]
    ParadeError(#[from] ParadeError),

    #[error(transparent)]
    TIDError(#[from] TIDError),

    #[error("More than one row with row number {0} was found")]
    DuplicateRowNumber(i64),

    #[error("More than one batch with row number {0} was found")]
    DuplicateBatch(i64),

    #[error("This index is not supported because it is not suited for column-oriented data")]
    IndexNotSupported,

    #[error("TupleTableSlotOps not found")]
    NoTupleTableSlotOps,
}
