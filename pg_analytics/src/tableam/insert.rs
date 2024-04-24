use async_std::sync::Mutex;
use async_std::task;
use core::ffi::{c_int, c_void};
use deltalake::arrow::error::ArrowError;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::common::arrow::array::{ArrayRef, Int64Array};
use once_cell::sync::Lazy;
use pgrx::*;
use pgrx::pg_sys::AsPgCStr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;
use thiserror::Error;

use crate::datafusion::catalog::CatalogError;
use crate::datafusion::table::{DataFusionTableError, DatafusionTable};
use crate::datafusion::writer::Writer;
use crate::storage::metadata::{MetadataError, PgMetadata};
use crate::storage::tid::{RowNumber, TIDError};
use crate::types::array::IntoArrowArray;
use crate::types::datatype::{DataTypeError, PgTypeMod};

// pub static INSERT_MEMORY_CONTEXT: Lazy<Mutex<AtomicPtr<pg_sys::MemoryContextData>>> =
//     Lazy::new(|| {
//         Mutex::new(AtomicPtr::new(
//             PgMemoryContexts::new("insert_memory_context").value(),
//         ))
//     });

#[pg_guard]
pub extern "C" fn deltalake_slot_callbacks(
    _rel: pg_sys::Relation,
) -> *const pg_sys::TupleTableSlotOps {
    info!("deltalake_slot_callbacks");
    unsafe { &pg_sys::TTSOpsVirtual }
    // unsafe { &pg_sys::TTSOpsHeapTuple }
}

#[pg_guard]
pub extern "C" fn deltalake_tuple_insert(
    rel: pg_sys::Relation,
    slot: *mut pg_sys::TupleTableSlot,
    _cid: pg_sys::CommandId,
    _options: c_int,
    _bistate: *mut pg_sys::BulkInsertStateData,
) {
    // info!("deltalake_tuple_insert");
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
    info!("deltalake_multi_insert");
    unsafe {
        task::block_on(insert_tuples(rel, slots, nslots as usize)).unwrap_or_else(|err| {
            panic!("{}", err);
        });
    }
}

#[pg_guard]
pub extern "C" fn deltalake_finish_bulk_insert(_rel: pg_sys::Relation, _options: c_int) {
    info!("deltalake_finish_bulk_insert");
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
    info!("deltalake_tuple_insert_speculative");
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
    // info!("insert_tuples");
    // In the block below, we switch to the memory context we've defined as a static
    // variable, resetting it before and after we access the column values. We do this
    // because PgTupleDesc "supposed" to free the corresponding Postgres memory when it
    // is dropped... however, in practice, we're not seeing the memory get freed, which is
    // causing huge memory usage when building large indexes.
    // We're using the raw C MemoryContext API here because PgMemoryContexts is getting refactored
    // in pgrx 0.12.0 due to potential memory leaks.

    let pg_relation = PgRelation::from_pg(rel);
    let tuple_desc = pg_relation.tuple_desc();
    let mut column_values: Vec<ArrayRef> = vec![];

    let mut free_varlena_vec = vec![];

    // Convert the TupleTableSlots into DataFusion arrays
    for (col_idx, attr) in tuple_desc.iter().enumerate() {
        let (var_vec_opt, col_vec) = (0..nslots)
                .map(move |row_idx| unsafe {
                    let tuple_table_slot = *slots.add(row_idx);

                    let datum_opt = if (*tuple_table_slot).tts_ops == &pg_sys::TTSOpsBufferHeapTuple
                    {
                        let bslot = tuple_table_slot as *mut pg_sys::BufferHeapTupleTableSlot;
                        let tuple = (*bslot).base.tuple;
                        std::num::NonZeroUsize::new(col_idx + 1).and_then(|attr_num| {
                            htup::heap_getattr_raw(
                                tuple,
                                attr_num,
                                (*tuple_table_slot).tts_tupleDescriptor,
                            )
                        })
                    } else {
                        Some(*(*tuple_table_slot).tts_values.add(col_idx))
                    };

                    let is_null = *(*tuple_table_slot).tts_isnull.add(col_idx);
                    (!is_null).then_some(datum_opt).flatten()
                })
                .into_arrow_array(attr.type_oid(), PgTypeMod(attr.type_mod()))?;
        if let Some(var_vec) = var_vec_opt {
            free_varlena_vec.push(var_vec);
        }
        column_values.push(
            col_vec,
        );
    }

    // Assign TID to each row
    let mut row_numbers: Vec<i64> = vec![];

    for row_idx in 0..nslots {
        unsafe {
            let tuple_table_slot = *slots.add(row_idx);
            let next_row_number = rel.read_next_row_number()?;

            (*tuple_table_slot).tts_tid =
                pg_sys::ItemPointerData::try_from(RowNumber(next_row_number))?;

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
    let xmaxs: Vec<i64> = vec![0; nslots];
    column_values.push(Arc::new(Int64Array::from(xmaxs)));

    let namespace = pg_relation.namespace_raw();
    let schema_name = namespace.to_str().unwrap().to_string();
    let table_path = pg_relation.table_path()?;
    let arrow_schema = Arc::new(pg_relation.arrow_schema_with_reserved_fields()?);

    // Write Arrow arrays to buffer
    let batch = RecordBatch::try_new(arrow_schema.clone(), column_values)?;
    Writer::write(&schema_name, &table_path, arrow_schema, &batch).await?;

    // Free palloced namespace
    pg_sys::pfree(namespace.as_ptr() as *mut std::ffi::c_void);

    // Free palloced varlenas
    for free_var_vec in free_varlena_vec {
        for free_var in free_var_vec {
            // info!("free");
            pg_sys::pfree(free_var as *mut std::ffi::c_void);
        }
    }

    Ok(())
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
