use async_std::task;
use core::ffi::c_int;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::common::arrow::array::ArrayRef;
use pgrx::*;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::datatype::{DatafusionMapProducer, PostgresTypeTranslator};
use crate::datafusion::table::DeltaTableProvider;
use crate::errors::ParadeError;

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
    insert_tuples(rel, &mut mut_slot, 1, true).unwrap_or_else(|err| {
        panic!("{}", err);
    });
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
    insert_tuples(rel, slots, nslots as usize, false).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}

#[pg_guard]
pub extern "C" fn deltalake_finish_bulk_insert(rel: pg_sys::Relation, _options: c_int) {
    flush_and_commit(rel).unwrap_or_else(|err| {
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
}

#[inline]
fn flush_and_commit(rel: pg_sys::Relation) -> Result<(), ParadeError> {
    let pg_relation = unsafe { PgRelation::from_pg(rel) };
    let table_name = pg_relation.name();
    let schema_name = pg_relation.namespace();
    let arrow_schema = pg_relation.arrow_schema()?;

    DatafusionContext::with_schema_provider(schema_name, |provider| {
        task::block_on(provider.flush_and_commit(table_name, arrow_schema))
    })?;

    Ok(())
}

#[inline]
fn insert_tuples(
    rel: pg_sys::Relation,
    slots: *mut *mut pg_sys::TupleTableSlot,
    nslots: usize,
    commit: bool,
) -> Result<(), ParadeError> {
    let pg_relation = unsafe { PgRelation::from_pg(rel) };
    let tuple_desc = pg_relation.tuple_desc();
    let mut values: Vec<ArrayRef> = vec![];

    // Convert the TupleTableSlots into DataFusion arrays
    for (col_idx, attr) in tuple_desc.iter().enumerate() {
        values.push(DatafusionMapProducer::array(
            attr.type_oid().to_sql_data_type(attr.type_mod())?,
            slots,
            nslots,
            col_idx,
        )?);
    }

    // Create a RecordBatch
    let table_name = pg_relation.name();
    let schema_name = pg_relation.namespace();
    let arrow_schema = pg_relation.arrow_schema()?;
    let batch = RecordBatch::try_new(arrow_schema.clone(), values)?;

    // Write the RecordBatch to the Delta table
    DatafusionContext::with_schema_provider(schema_name, |provider| {
        task::block_on(provider.write(table_name, batch))?;

        if commit {
            task::block_on(provider.flush_and_commit(table_name, arrow_schema))?;
        }

        Ok(())
    })
}
