use async_std::task;
use core::ffi::c_int;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::common::arrow::array::ArrayRef;
use pgrx::*;

use crate::datafusion::commit::commit_writer;

use crate::datafusion::datatype::DatafusionMapProducer;
use crate::datafusion::datatype::DatafusionTypeTranslator;
use crate::datafusion::datatype::PostgresTypeTranslator;
use crate::datafusion::table::DatafusionTable;
use crate::datafusion::writer::Writer;
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
    task::block_on(insert_tuples(rel, &mut mut_slot, 1)).unwrap_or_else(|err| {
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
    task::block_on(insert_tuples(rel, slots, nslots as usize)).unwrap_or_else(|err| {
        panic!("{}", err);
    });
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
}

#[inline]
async fn insert_tuples(
    rel: pg_sys::Relation,
    slots: *mut *mut pg_sys::TupleTableSlot,
    nslots: usize,
) -> Result<(), ParadeError> {
    let pg_relation = unsafe { PgRelation::from_pg(rel) };
    let tuple_desc = pg_relation.tuple_desc();
    let mut values: Vec<ArrayRef> = vec![];

    // Convert the TupleTableSlots into DataFusion arrays
    for (col_idx, attr) in tuple_desc.iter().enumerate() {
        let sql_data_type = attr.type_oid().to_sql_data_type(attr.type_mod())?;
        let datafusion_type = DatafusionTypeTranslator::from_sql_data_type(sql_data_type)?;

        values.push(DatafusionMapProducer::array(
            datafusion_type,
            slots,
            nslots,
            col_idx,
        )?);
    }

    let pg_relation = unsafe { PgRelation::from_pg(rel) };
    let schema_name = pg_relation.namespace();
    let table_path = pg_relation.table_path()?;
    let arrow_schema = pg_relation.arrow_schema()?;
    let batch = RecordBatch::try_new(arrow_schema.clone(), values)?;

    Writer::write(schema_name, &table_path, arrow_schema, &batch).await
}
