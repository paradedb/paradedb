use async_std::sync::Mutex;
use async_std::task;
use core::ffi::c_int;
use deltalake::arrow::error::ArrowError;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::common::arrow::array::{ArrayRef, Int64Array};
use once_cell::sync::Lazy;
use pgrx::*;
use shared::postgres::htup::{heap_tuple_header_set_xmax, heap_tuple_header_set_xmin};
use shared::postgres::wal::{relation_needs_wal, SIZEOF_HEAP_TUPLE_HEADER};
use std::ffi::c_char;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;
use thiserror::Error;

use crate::datafusion::catalog::CatalogError;
use crate::datafusion::table::{DataFusionTableError, DatafusionTable};
use crate::datafusion::writer::Writer;
use crate::rmgr::{RM_ANALYTICS_ID, XLOG_ANALYTICS_INSERT};
use crate::storage::metadata::{MetadataError, PgMetadata};
use crate::storage::tid::{RowNumber, TIDError};
use crate::types::array::IntoArrowArray;
use crate::types::datatype::{DataTypeError, PgTypeMod};

const INSERT_XMAX: u32 = 0;

pub static INSERT_MEMORY_CONTEXT: Lazy<Mutex<AtomicPtr<pg_sys::MemoryContextData>>> =
    Lazy::new(|| {
        Mutex::new(AtomicPtr::new(
            PgMemoryContexts::new("insert_memory_context").value(),
        ))
    });

#[pg_guard]
pub extern "C" fn deltalake_slot_callbacks(
    _rel: pg_sys::Relation,
) -> *const pg_sys::TupleTableSlotOps {
    unsafe { &pg_sys::TTSOpsVirtual }
}

#[pg_guard]
pub extern "C" fn deltalake_tuple_insert(
    rel: pg_sys::Relation,
    slot: *mut pg_sys::TupleTableSlot,
    _cid: pg_sys::CommandId,
    _options: c_int,
    _bistate: *mut pg_sys::BulkInsertStateData,
) {
    let mut mut_slot = slot;
    unsafe {
        task::block_on(insert_tuples(rel, &mut mut_slot, 1)).unwrap_or_else(|err| {
            panic!("{}", err);
        });
    }
}

#[pg_guard]
pub extern "C" fn deltalake_multi_insert(
    rel: pg_sys::Relation,
    slots: *mut *mut pg_sys::TupleTableSlot,
    nslots: c_int,
    _cid: pg_sys::CommandId,
    _options: c_int,
    _bistate: *mut pg_sys::BulkInsertStateData,
) {
    unsafe {
        task::block_on(insert_tuples(rel, slots, nslots as usize)).unwrap_or_else(|err| {
            panic!("{}", err);
        });
    }
}

#[pg_guard]
pub extern "C" fn deltalake_finish_bulk_insert(_rel: pg_sys::Relation, _options: c_int) {
    task::block_on(Writer::flush()).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}

#[pg_guard]
pub extern "C" fn deltalake_tuple_insert_speculative(
    _rel: pg_sys::Relation,
    _slot: *mut pg_sys::TupleTableSlot,
    _cid: pg_sys::CommandId,
    _options: c_int,
    _bistate: *mut pg_sys::BulkInsertStateData,
    _specToken: pg_sys::uint32,
) {
    panic!(
        "{}",
        TableInsertError::SpeculativeInsertNotSupported.to_string()
    );
}

#[inline]
async unsafe fn insert_tuples(
    rel: pg_sys::Relation,
    slots: *mut *mut pg_sys::TupleTableSlot,
    nslots: usize,
) -> Result<(), TableInsertError> {
    // In the block below, we switch to the memory context we've defined as a static
    // variable, resetting it before and after we access the column values. We do this
    // because PgTupleDesc "supposed" to free the corresponding Postgres memory when it
    // is dropped... however, in practice, we're not seeing the memory get freed, which is
    // causing huge memory usage when building large indexes.
    // We're using the raw C MemoryContext API here because PgMemoryContexts is getting refactored
    // in pgrx 0.12.0 due to potential memory leaks.
    let memctx = INSERT_MEMORY_CONTEXT.lock().await.load(Ordering::SeqCst);
    let old_context = pg_sys::MemoryContextSwitchTo(memctx);

    let pg_relation = PgRelation::from_pg(rel);
    let table_oid = pg_relation.oid();
    let pg_tuple_desc = pg_relation.tuple_desc();
    let tuple_desc = pg_tuple_desc.clone().into_pg();

    let mut column_values: Vec<ArrayRef> = vec![];

    // Convert the TupleTableSlots into DataFusion arrays
    for (col_idx, attr) in pg_tuple_desc.iter().enumerate() {
        column_values.push(
            (0..nslots)
                .map(move |row_idx| unsafe {
                    let slot = *slots.add(row_idx);
                    let mut should_free = true;
                    let heap_tuple =
                        pg_sys::ExecFetchSlotHeapTuple(slot, true, &mut should_free as *mut bool);
                    // attnum is 1 indexed for heap_att* functions
                    let attnum = col_idx as i32 + 1;
                    let mut is_null = pg_sys::heap_attisnull(heap_tuple, attnum, tuple_desc);
                    let datum = pg_sys::heap_getattr(heap_tuple, attnum, tuple_desc, &mut is_null);

                    if relation_needs_wal(rel) {
                        prepare_insert(table_oid, heap_tuple);
                        pg_sys::XLogBeginInsert();

                        let tuple_data = (*heap_tuple).t_data as *mut c_char;
                        let tuple_data_no_header = tuple_data.add(SIZEOF_HEAP_TUPLE_HEADER);

                        pg_sys::XLogRegisterData(
                            tuple_data_no_header,
                            (*heap_tuple).t_len - SIZEOF_HEAP_TUPLE_HEADER as u32,
                        );
                        pg_sys::XLogInsert(RM_ANALYTICS_ID, XLOG_ANALYTICS_INSERT);
                    }

                    (!is_null).then_some(datum)
                })
                .into_arrow_array(attr.type_oid(), PgTypeMod(attr.type_mod()))?,
        );
    }

    // Assign TID to each row
    let mut row_numbers: Vec<i64> = vec![];

    for row_idx in 0..nslots {
        unsafe {
            let slot = *slots.add(row_idx);
            let next_row_number = rel.read_next_row_number()?;

            (*slot).tts_tid = pg_sys::ItemPointerData::try_from(RowNumber(next_row_number))?;

            row_numbers.push(next_row_number);
            rel.write_next_row_number(next_row_number + 1)?;
        }
    }

    column_values.push(Arc::new(Int64Array::from(row_numbers.clone())));

    // Assign xmin to each row
    let transaction_id = unsafe { pg_sys::GetCurrentTransactionId() } as i64;
    let xmins: Vec<i64> = vec![transaction_id; nslots];
    column_values.push(Arc::new(Int64Array::from(xmins)));

    // Assign xmax to each row
    let xmaxs: Vec<i64> = vec![INSERT_XMAX as i64; nslots];
    column_values.push(Arc::new(Int64Array::from(xmaxs)));

    let schema_name = pg_relation.namespace().to_string();
    let table_path = pg_relation.table_path()?;
    let arrow_schema = Arc::new(pg_relation.arrow_schema_with_reserved_fields()?);

    // Write Arrow arrays to buffer
    let batch = RecordBatch::try_new(arrow_schema.clone(), column_values)?;
    Writer::write(&schema_name, &table_path, arrow_schema, &batch).await?;

    pg_sys::MemoryContextReset(memctx);
    pg_sys::MemoryContextSwitchTo(old_context);

    Ok(())
}

/// Based on Postgres' heap_prepare_insert() in src/backend/access/heap/heapam.c
#[inline]
unsafe fn prepare_insert(table_oid: pg_sys::Oid, heap_tuple: pg_sys::HeapTuple) {
    heap_tuple_header_set_xmin((*heap_tuple).t_data, pg_sys::GetCurrentTransactionId());
    heap_tuple_header_set_xmax((*heap_tuple).t_data, INSERT_XMAX);

    (*heap_tuple).t_tableOid = table_oid;
}

#[derive(Error, Debug)]
pub enum TableInsertError {
    #[error(transparent)]
    ArrowError(#[from] ArrowError),

    #[error(transparent)]
    CatalogError(#[from] CatalogError),

    #[error(transparent)]
    DataFusionTableError(#[from] DataFusionTableError),

    #[error(transparent)]
    DataTypeError(#[from] DataTypeError),

    #[error(transparent)]
    MetadataError(#[from] MetadataError),

    #[error(transparent)]
    NulError(#[from] std::ffi::NulError),

    #[error(transparent)]
    TIDError(#[from] TIDError),

    #[error("Inserts with ON CONFLICT are not yet supported")]
    SpeculativeInsertNotSupported,
}
