use async_std::task;
use core::ffi::c_int;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::common::arrow::array::{ArrayRef, Int64Array};
use deltalake::datafusion::common::arrow::datatypes::{DataType, Field, Schema as ArrowSchema};
use pgrx::*;
use std::cell::RefCell;
use std::sync::Arc;

use crate::datafusion::commit::commit_writer;
use crate::datafusion::table::{DatafusionTable, RESERVED_TID_FIELD};
use crate::datafusion::writer::Writer;
use crate::errors::{NotSupported, ParadeError};
use crate::tableam::{TableMetadata, FIRST_BLOCK_NUMBER};
use crate::types::array::IntoArrowArray;
use crate::types::datatype::PgTypeMod;

thread_local! {
    static INSERT_MEM_CTX: RefCell<PgMemoryContexts> = RefCell::new(
        PgMemoryContexts::new("pg_analytics_insert_tuples")
    );
}

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
    // unsafe {
    //     let mut ctid = pg_sys::ItemPointerData::default();
    //     pgrx::u64_to_item_pointer(1, &mut ctid);

    //     (*slot).tts_tid = ctid;

    // }

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
    task::block_on(commit_writer()).unwrap_or_else(|err| {
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
    panic!("{}", NotSupported::SpeculativeInsert.to_string());
}

#[inline]
async unsafe fn insert_tuples(
    rel: pg_sys::Relation,
    slots: *mut *mut pg_sys::TupleTableSlot,
    nslots: usize,
) -> Result<(), ParadeError> {
    // In the block below, we switch to the memory context we've defined as a static
    // variable, resetting it before and after we access the column values. We do this
    // because PgTupleDesc "supposed" to free the corresponding Postgres memory when it
    // is dropped... however, in practice, we're not seeing the memory get freed, which is
    // causing huge memory usage when building large indexes.
    //
    // By running in our own memory context, we can force the memory to be freed with
    // the call to reset().
    let (schema_name, table_path, arrow_schema, column_values) =
        INSERT_MEM_CTX.with(|memcxt_ref| {
            let mut memcxt = memcxt_ref.borrow_mut();
            memcxt.reset();
            memcxt.switch_to(|_| -> Result<_, ParadeError> {
                let pg_relation = PgRelation::from_pg(rel);
                let tuple_desc = pg_relation.tuple_desc();
                let mut column_values: Vec<ArrayRef> = vec![];

                // Convert the TupleTableSlots into DataFusion arrays
                for (col_idx, attr) in tuple_desc.iter().enumerate() {
                    column_values.push(
                        (0..nslots)
                            .map(move |row_idx| unsafe {
                                let tuple_table_slot = *slots.add(row_idx);

                                let datum_opt = if (*tuple_table_slot).tts_ops
                                    == &pg_sys::TTSOpsBufferHeapTuple
                                {
                                    let bslot =
                                        tuple_table_slot as *mut pg_sys::BufferHeapTupleTableSlot;
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
                            .into_arrow_array(attr.type_oid(), PgTypeMod(attr.type_mod()))?,
                    );
                }

                let schema_name = pg_relation.namespace().to_string();
                let table_path = pg_relation.table_path()?;
                let arrow_schema = pg_relation.arrow_schema()?;

                Ok((schema_name, table_path, arrow_schema, column_values))
            })
        })?;

    column_values.push(Arc::new(Int64Array::from(
        (0..nslots)
            .map(|_| next_row_number(rel))
            .collect::<Vec<i64>>(),
    )));

    let pg_relation = unsafe { PgRelation::from_pg(rel) };
    let schema_name = pg_relation.namespace();
    let table_path = pg_relation.table_path()?;
    let arrow_schema = Arc::new(ArrowSchema::try_merge(vec![
        pg_relation.arrow_schema()?,
        ArrowSchema::new(vec![Field::new(RESERVED_TID_FIELD, DataType::Int64, false)]),
    ])?);
    let batch = RecordBatch::try_new(arrow_schema.clone(), column_values)?;

    Writer::write(&schema_name, &table_path, arrow_schema, &batch).await?;

    INSERT_MEM_CTX.with(|memcxt_ref| {
        let mut memcxt = memcxt_ref.borrow_mut();
        memcxt.reset();
    });

    Ok(())
}

#[inline]
fn next_row_number(rel: pg_sys::Relation) -> i64 {
    unsafe {
        let buffer = pg_sys::ReadBufferExtended(
            rel,
            pg_sys::ForkNumber_MAIN_FORKNUM,
            FIRST_BLOCK_NUMBER,
            pg_sys::ReadBufferMode_RBM_NORMAL,
            std::ptr::null_mut(),
        );

        pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as i32);
        let page = pg_sys::BufferGetPage(buffer);
        let metadata = pg_sys::PageGetSpecialPointer(page) as *mut TableMetadata;
        let next_row_number = (*metadata).max_row_number + 1;
        (*metadata).max_row_number = next_row_number;

        pg_sys::MarkBufferDirty(buffer);
        pg_sys::UnlockReleaseBuffer(buffer);

        next_row_number
    }
}
