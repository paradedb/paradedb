use async_std::task;
use core::ffi::c_int;

use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::arrow::array::{
    ArrayRef, BooleanArray, Date32Array, Float32Array, Float64Array, Int16Array, Int32Array,
    Int64Array, StringArray, Time32SecondArray, TimestampMillisecondArray, UInt32Array,
};
use datafusion::common::arrow::datatypes::Schema;
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::datasource::MemTable;

use pgrx::*;
use std::sync::Arc;

use crate::nodes::utils::{get_datafusion_schema, get_datafusion_table, get_datafusion_table_name};
use crate::tableam::utils::{BULK_INSERT_STATE, CONTEXT};

static MAX_SLOTS: usize = 5_000_000;

#[pg_guard]
pub unsafe extern "C" fn memam_slot_callbacks(
    _rel: pg_sys::Relation,
) -> *const pg_sys::TupleTableSlotOps {
    &pg_sys::TTSOpsVirtual
}

#[pg_guard]
pub unsafe extern "C" fn memam_multi_insert(
    rel: pg_sys::Relation,
    slots: *mut *mut pg_sys::TupleTableSlot,
    nslots: c_int,
    _cid: pg_sys::CommandId,
    _options: c_int,
    _bistate: *mut pg_sys::BulkInsertStateData,
) {
    let pg_relation = PgRelation::from_pg(rel);
    let tuple_desc = pg_relation.tuple_desc();
    let oids = tuple_desc
        .iter()
        .map(|attr| PgOid::from(attr.atttypid))
        .collect::<Vec<PgOid>>();

    let natts = tuple_desc.len();
    let pg_relation = unsafe { PgRelation::from_pg(rel) };
    let table_name = get_datafusion_table_name(&pg_relation).expect("Could not get table name");
    let mut values: Vec<ArrayRef> = vec![];

    set_schema_if_needed(&table_name, &pg_relation);

    for (col_idx, oid) in oids.iter().enumerate().take(natts) {
        match oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::BOOLOID => {
                    let vec: Vec<Option<bool>> = create_datafusion_array(
                        nslots as usize,
                        slots,
                        col_idx,
                        |datum: *mut pg_sys::Datum| bool::from_datum(*datum, false),
                    );
                    values.push(Arc::new(BooleanArray::from(vec)));
                }
                PgBuiltInOids::BPCHAROID | PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                    let vec: Vec<Option<String>> = create_datafusion_array(
                        nslots as usize,
                        slots,
                        col_idx,
                        |datum: *mut pg_sys::Datum| String::from_datum(*datum, false),
                    );
                    values.push(Arc::new(StringArray::from(vec)));
                }
                PgBuiltInOids::INT2OID => {
                    let vec: Vec<Option<i16>> = create_datafusion_array(
                        nslots as usize,
                        slots,
                        col_idx,
                        |datum: *mut pg_sys::Datum| i16::from_datum(*datum, false),
                    );
                    values.push(Arc::new(Int16Array::from(vec)));
                }
                PgBuiltInOids::INT4OID => {
                    let vec: Vec<Option<i32>> = create_datafusion_array(
                        nslots as usize,
                        slots,
                        col_idx,
                        |datum: *mut pg_sys::Datum| i32::from_datum(*datum, false),
                    );
                    values.push(Arc::new(Int32Array::from(vec)));
                }
                PgBuiltInOids::INT8OID => {
                    let vec: Vec<Option<i64>> = create_datafusion_array(
                        nslots as usize,
                        slots,
                        col_idx,
                        |datum: *mut pg_sys::Datum| i64::from_datum(*datum, false),
                    );
                    values.push(Arc::new(Int64Array::from(vec)));
                }
                PgBuiltInOids::OIDOID | PgBuiltInOids::XIDOID => {
                    let vec: Vec<Option<u32>> = create_datafusion_array(
                        nslots as usize,
                        slots,
                        col_idx,
                        |datum: *mut pg_sys::Datum| u32::from_datum(*datum, false),
                    );
                    values.push(Arc::new(UInt32Array::from(vec)));
                }
                PgBuiltInOids::FLOAT4OID => {
                    let vec: Vec<Option<f32>> = create_datafusion_array(
                        nslots as usize,
                        slots,
                        col_idx,
                        |datum: *mut pg_sys::Datum| f32::from_datum(*datum, false),
                    );
                    values.push(Arc::new(Float32Array::from(vec)));
                }
                PgBuiltInOids::FLOAT8OID | PgBuiltInOids::NUMERICOID => {
                    let vec: Vec<Option<f64>> = create_datafusion_array(
                        nslots as usize,
                        slots,
                        col_idx,
                        |datum: *mut pg_sys::Datum| f64::from_datum(*datum, false),
                    );
                    values.push(Arc::new(Float64Array::from(vec)));
                }
                PgBuiltInOids::TIMEOID => {
                    let vec: Vec<Option<i32>> = create_datafusion_array(
                        nslots as usize,
                        slots,
                        col_idx,
                        |datum: *mut pg_sys::Datum| i32::from_datum(*datum, false),
                    );
                    values.push(Arc::new(Time32SecondArray::from(vec)));
                }
                PgBuiltInOids::TIMESTAMPOID => {
                    let vec: Vec<Option<i64>> = create_datafusion_array(
                        nslots as usize,
                        slots,
                        col_idx,
                        |datum: *mut pg_sys::Datum| i64::from_datum(*datum, false),
                    );
                    values.push(Arc::new(TimestampMillisecondArray::from(vec)));
                }
                PgBuiltInOids::DATEOID => {
                    let vec: Vec<Option<i32>> = create_datafusion_array(
                        nslots as usize,
                        slots,
                        col_idx,
                        |datum: *mut pg_sys::Datum| i32::from_datum(*datum, false),
                    );
                    values.push(Arc::new(Date32Array::from(vec)));
                }
                _ => todo!(),
            },
            _ => todo!(),
        }
    }

    let mut bulk_insert_state = BULK_INSERT_STATE.lock().unwrap();
    bulk_insert_state.nslots += nslots as usize;

    if let Some(schema) = &bulk_insert_state.schema {
        let binding = schema.into();
        bulk_insert_state
            .batches
            .push(RecordBatch::try_new(Arc::new(binding), values).expect("Could not create batch"));
    }

    if bulk_insert_state.nslots > MAX_SLOTS {
        drop(bulk_insert_state);
        flush_batches(rel);
    }
}

#[pg_guard]
pub unsafe extern "C" fn memam_finish_bulk_insert(rel: pg_sys::Relation, _options: c_int) {
    flush_batches(rel);
}

#[inline]
unsafe fn create_datafusion_array<T, F>(
    nslots: usize,
    slots: *mut *mut pg_sys::TupleTableSlot,
    col_idx: usize,
    from_datum: F,
) -> Vec<Option<T>>
where
    F: Fn(*mut pg_sys::Datum) -> Option<T>,
{
    let mut vec = Vec::with_capacity(nslots);
    for row_idx in 0..nslots {
        let tuple_table_slot = *slots.add(row_idx);
        let datum = (*tuple_table_slot).tts_values.add(col_idx);
        let tts_is_null = (*tuple_table_slot).tts_isnull.add(col_idx);

        let value = if *tts_is_null {
            None
        } else {
            from_datum(datum)
        };

        vec.push(value);
    }
    vec
}

#[inline]
unsafe fn flush_batches(rel: pg_sys::Relation) {
    let pg_relation = PgRelation::from_pg(rel);
    let table_name = get_datafusion_table_name(&pg_relation).expect("Could not get table name");
    let mut bulk_insert_state = BULK_INSERT_STATE.lock().unwrap();

    if bulk_insert_state.batches.is_empty() {
        return;
    }

    if let Some(schema) = &bulk_insert_state.schema {
        let table = Arc::new(
            MemTable::try_new(
                Arc::new(Schema::from(schema)),
                vec![bulk_insert_state.batches.clone()],
            )
            .expect("Could not create MemTable"),
        );
        let df = CONTEXT
            .read_table(table)
            .expect("Could not create dataframe");
        let _ = task::block_on(df.write_table(&table_name, DataFrameWriteOptions::new()));
        bulk_insert_state.batches.clear();
        bulk_insert_state.nslots = 0;
    }
}

#[inline]
unsafe fn set_schema_if_needed(table_name: &str, pg_relation: &PgRelation) {
    let mut bulk_insert_state = BULK_INSERT_STATE.lock().unwrap();

    if bulk_insert_state.schema.is_none() {
        let table_source =
            get_datafusion_table(table_name, pg_relation).expect("Could not get table source");
        let df_schema =
            get_datafusion_schema(table_name, table_source).expect("Could not get schema");

        bulk_insert_state.schema = Some(df_schema.clone());
    }
}
