use async_std::task;
use deltalake::datafusion::arrow::datatypes::Schema as ArrowSchema;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::common::arrow::array::ArrayRef;
use pgrx::*;
use std::any::type_name;
use std::panic;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::datatype::{DatafusionMapProducer, PostgresTypeTranslator};
use crate::datafusion::table::DeltaTableProvider;
use crate::errors::{NotFound, ParadeError};

struct BuildState {
    count: usize,
    arrow_schema: Arc<ArrowSchema>,
    table_name: Arc<String>,
    schema_name: Arc<String>,
}

impl BuildState {
    fn new() -> Self {
        BuildState {
            count: 0,
            arrow_schema: Arc::new(ArrowSchema::empty()),
            schema_name: Arc::new(String::new()),
            table_name: Arc::new(String::new()),
        }
    }
}

#[pg_guard]
pub extern "C" fn ambuild(
    heaprel: pg_sys::Relation,
    indexrel: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) -> *mut pg_sys::IndexBuildResult {
    let pg_relation = unsafe { PgRelation::from_pg(heaprel) };
    let index_relation = unsafe { PgRelation::from_pg(indexrel) };
    let schema_name = pg_relation.namespace();

    let _ = DatafusionContext::with_schema_provider(schema_name, |provider| {
        task::block_on(provider.create_table(&pg_relation))
    });

    let state = do_heap_scan(index_info, &pg_relation, &index_relation).unwrap();

    let mut result = unsafe { PgBox::<pg_sys::IndexBuildResult>::alloc0() };
    result.heap_tuples = state.count as f64;
    result.index_tuples = state.count as f64;

    result.into_pg()
}

#[pg_guard]
pub extern "C" fn ambuildempty(_index_relation: pg_sys::Relation) {}

fn do_heap_scan<'a>(
    index_info: *mut pg_sys::IndexInfo,
    heap_relation: &'a PgRelation,
    index_relation: &'a PgRelation,
) -> Result<BuildState, ParadeError> {
    let mut state = BuildState::new();

    let arrow_schema = heap_relation.arrow_schema()?;
    let schema_name = heap_relation.namespace();
    let table_name = heap_relation.name();

    state.arrow_schema = arrow_schema.clone();
    state.schema_name = Arc::new(schema_name.to_string());
    state.table_name = Arc::new(table_name.to_string());

    let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| unsafe {
        pg_sys::IndexBuildHeapScan(
            heap_relation.as_ptr(),
            index_relation.as_ptr(),
            index_info,
            Some(build_callback),
            &mut state,
        );
    }));

    DatafusionContext::with_schema_provider(schema_name, |provider| {
        task::block_on(provider.flush_and_commit(table_name, arrow_schema.clone()))
    })?;

    Ok(state)
}

#[cfg(feature = "pg12")]
#[pg_guard]
unsafe extern "C" fn build_callback(
    index: pg_sys::Relation,
    htup: pg_sys::HeapTuple,
    values: *mut pg_sys::Datum,
    _isnull: *mut bool,
    _tuple_is_alive: bool,
    state: *mut std::os::raw::c_void,
) {
    let htup = htup.as_ref().unwrap();
    let _ = build_callback_internal(htup.t_self, values, state, index);
}

#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
#[pg_guard]
unsafe extern "C" fn build_callback(
    index: pg_sys::Relation,
    ctid: pg_sys::ItemPointer,
    values: *mut pg_sys::Datum,
    _isnull: *mut bool,
    _tuple_is_alive: bool,
    state: *mut std::os::raw::c_void,
) {
    let _ = build_callback_internal(*ctid, values, state, index);
}

#[inline(always)]
unsafe fn build_callback_internal(
    _ctid: pg_sys::ItemPointerData,
    values: *mut pg_sys::Datum,
    state: *mut std::os::raw::c_void,
    index: pg_sys::Relation,
) -> Result<(), ParadeError> {
    check_for_interrupts!();

    let index_relation = PgRelation::from_pg(index);

    let typid = index_relation
        .tuple_desc()
        .get(0)
        .expect("no attribute #0 on tupledesc")
        .type_oid()
        .value();
    let typmod = index_relation
        .tuple_desc()
        .get(0)
        .expect("no attribute #0 on tupledesc")
        .type_mod();

    // lookup the tuple descriptor for the rowtype we're *indexing*, rather than
    // using the tuple descriptor for the index definition itself
    let tuple_desc = PgMemoryContexts::TopTransactionContext.switch_to(|_| {
        PgTupleDesc::from_pg_is_copy(pg_sys::lookup_rowtype_tupdesc_copy(typid, typmod))
    });

    let mut tuple_table_slot =
        pg_sys::MakeTupleTableSlot(tuple_desc.clone().into_pg(), &pg_sys::TTSOpsVirtual);
    let mut datafusion_values: Vec<ArrayRef> = vec![];

    let row = std::slice::from_raw_parts(values, 1)[0];
    let td =
        pg_sys::pg_detoast_datum(row.cast_mut_ptr::<pg_sys::varlena>()) as pg_sys::HeapTupleHeader;

    let mut tmptup = pg_sys::HeapTupleData {
        t_len: varsize(td as *mut pg_sys::varlena) as u32,
        t_self: Default::default(),
        t_tableOid: pg_sys::Oid::INVALID,
        t_data: td,
    };

    let mut datums = vec![pg_sys::Datum::from(0); tuple_desc.natts as usize];
    let mut nulls = vec![false; tuple_desc.natts as usize];

    pg_sys::heap_deform_tuple(
        &mut tmptup,
        tuple_desc.as_ptr(),
        datums.as_mut_ptr(),
        nulls.as_mut_ptr(),
    );

    let mut dropped = 0;
    for (attno, attribute) in tuple_desc.iter().enumerate() {
        // Skip attributes that have been dropped.
        if attribute.is_dropped() {
            dropped += 1;
            continue;
        }

        if let Some(is_null) = nulls.get(attno) {
            if *is_null {
                let tts_isnull = (*tuple_table_slot).tts_isnull.add(attno);
                *tts_isnull = true;
            }
        }

        let tts_value = (*tuple_table_slot).tts_values.add(attno);
        *tts_value = datums[attno - dropped];

        datafusion_values.push(DatafusionMapProducer::array(
            attribute
                .type_oid()
                .to_sql_data_type(attribute.type_mod())?,
            &mut tuple_table_slot,
            1,
            attno,
        )?);
    }

    let state = (state as *mut BuildState)
        .as_mut()
        .ok_or(NotFound::Value(type_name::<BuildState>().to_string()))?;
    let batch = RecordBatch::try_new(state.arrow_schema.clone(), datafusion_values)?;

    DatafusionContext::with_schema_provider(&state.schema_name, |provider| {
        task::block_on(provider.write(&state.table_name, batch))?;
        Ok(())
    })
}
