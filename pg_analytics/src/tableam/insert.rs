use async_std::task;
use core::ffi::c_int;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::common::arrow::array::ArrayRef;
use pgrx::*;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::substrait::{DatafusionMap, DatafusionMapProducer, SubstraitTranslator};
use crate::datafusion::table::ParadeTable;
use crate::errors::ParadeError;

#[pg_guard]
pub extern "C" fn analytics_slot_callbacks(
    _rel: pg_sys::Relation,
) -> *const pg_sys::TupleTableSlotOps {
    unsafe { &pg_sys::TTSOpsVirtual }
}

#[pg_guard]
pub extern "C" fn analytics_tuple_insert(
    rel: pg_sys::Relation,
    slot: *mut pg_sys::TupleTableSlot,
    _cid: pg_sys::CommandId,
    _options: c_int,
    _bistate: *mut pg_sys::BulkInsertStateData,
) {
    let mut mut_slot = slot;
    insert_tuples(rel, &mut mut_slot, 1, true).expect("Failed to insert tuple");
}

#[pg_guard]
pub extern "C" fn analytics_multi_insert(
    rel: pg_sys::Relation,
    slots: *mut *mut pg_sys::TupleTableSlot,
    nslots: c_int,
    _cid: pg_sys::CommandId,
    _options: c_int,
    _bistate: *mut pg_sys::BulkInsertStateData,
) {
    insert_tuples(rel, slots, nslots as usize, false).expect("Failed to insert tuples");
}

#[pg_guard]
pub extern "C" fn analytics_finish_bulk_insert(rel: pg_sys::Relation, _options: c_int) {
    flush_and_commit(rel).expect("Failed to commit tuples");
}

#[pg_guard]
pub extern "C" fn analytics_tuple_insert_speculative(
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
    let parade_table = ParadeTable::from_pg(&pg_relation)?;
    let arrow_schema = parade_table.arrow_schema()?;

    DatafusionContext::with_provider_context(|provider, _| {
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
    let oids = tuple_desc
        .iter()
        .map(|attr| PgOid::from(attr.atttypid))
        .collect::<Vec<PgOid>>();

    let natts = tuple_desc.len();
    let mut values: Vec<ArrayRef> = vec![];

    // Convert the TupleTableSlots into Datafusion arrays
    for (col_idx, oid) in oids.iter().enumerate().take(natts) {
        DatafusionMapProducer::map(oid.to_substrait()?, |df_map: DatafusionMap| {
            values.push((df_map.array)(slots, nslots, col_idx));
        })?;
    }

    // Create a RecordBatch
    let parade_table = ParadeTable::from_pg(&pg_relation)?;
    let table_name = parade_table.name()?;
    let arrow_schema = parade_table.arrow_schema()?;
    let batch = RecordBatch::try_new(arrow_schema.clone(), values)?;

    // Write the RecordBatch to the Delta table
    DatafusionContext::with_provider_context(|provider, _| {
        task::block_on(provider.write(&table_name, batch))?;

        if commit {
            task::block_on(provider.flush_and_commit(&table_name, arrow_schema))?;
        }

        Ok(())
    })
}
