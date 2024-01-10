use async_std::task;
use core::ffi::c_int;
use deltalake::datafusion::arrow::datatypes::Schema as ArrowSchema;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::common::arrow::array::ArrayRef;
use pgrx::*;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::directory::ParquetDirectory;
use crate::datafusion::error::{arrow_err_to_string, delta_err_to_string};
use crate::datafusion::registry::{PARADE_CATALOG, PARADE_SCHEMA};
use crate::datafusion::schema::ParadeSchemaProvider;
use crate::datafusion::substrait::{DatafusionMap, DatafusionMapProducer, SubstraitTranslator};
use crate::datafusion::table::ParadeTable;

#[pg_guard]
pub extern "C" fn memam_slot_callbacks(_rel: pg_sys::Relation) -> *const pg_sys::TupleTableSlotOps {
    unsafe { &pg_sys::TTSOpsVirtual }
}

#[pg_guard]
pub extern "C" fn memam_tuple_insert(
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
pub extern "C" fn memam_multi_insert(
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
pub extern "C" fn memam_finish_bulk_insert(rel: pg_sys::Relation, _options: c_int) {
    let pg_relation = unsafe { PgRelation::from_pg(rel) };
    let parade_table = ParadeTable::from_pg(&pg_relation).expect("Failed to get Parade table");
    let table_name = parade_table.name().expect("Failed to get table name");
    let table_oid = parade_table
        .oid()
        .expect("Failed to get table oid")
        .to_string();
    let delta_table = task::block_on(deltalake::open_table(
        ParquetDirectory::table_path(&table_oid).expect("Failed to get table path"),
    ))
    .expect("Failed to open Delta table");

    DatafusionContext::with_read(|context| {
        let schema_provider = context
            .catalog(PARADE_CATALOG)
            .expect("Catalog not found")
            .schema(PARADE_SCHEMA)
            .expect("Schema not found");

        let lister = schema_provider
            .as_any()
            .downcast_ref::<ParadeSchemaProvider>()
            .expect("Failed to downcast schema provider");

        let _ = lister.flush_and_commit(&table_name, delta_table);
    });
}

#[inline]
fn insert_tuples(
    rel: pg_sys::Relation,
    slots: *mut *mut pg_sys::TupleTableSlot,
    nslots: usize,
    commit: bool,
) -> Result<(), String> {
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
        DatafusionMapProducer::map(oid.to_substrait().unwrap(), |df_map: DatafusionMap| {
            values.push((df_map.array)(slots, nslots, col_idx));
        })?;
    }

    // Create a RecordBatch
    let parade_table = ParadeTable::from_pg(&pg_relation)?;
    let table_name = parade_table.name()?;
    let arrow_schema = ArrowSchema::from(parade_table.schema()?);

    let batch =
        RecordBatch::try_new(Arc::new(arrow_schema), values).map_err(arrow_err_to_string())?;

    // Write the RecordBatch to the Delta table
    DatafusionContext::with_read(|context| {
        let schema_provider = context
            .catalog(PARADE_CATALOG)
            .expect("Catalog not found")
            .schema(PARADE_SCHEMA)
            .expect("Schema not found");

        let lister = schema_provider
            .as_any()
            .downcast_ref::<ParadeSchemaProvider>()
            .expect("Failed to downcast schema provider");

        let _ = lister.write(&table_name, batch);

        if commit {
            let delta_table = task::block_on(deltalake::open_table(ParquetDirectory::table_path(
                &parade_table.oid()?.to_string(),
            )?))
            .map_err(delta_err_to_string())?;
            let _ = lister.flush_and_commit(&table_name, delta_table);
        }

        Ok(())
    })
}
