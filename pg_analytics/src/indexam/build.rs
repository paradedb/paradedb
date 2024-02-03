use async_std::task;
use deltalake::datafusion::common::arrow::array::ArrayRef;
use pgrx::*;
use std::panic;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::datatype::{DatafusionMapProducer, PostgresTypeTranslator};

struct BuildState {
    count: usize,
}

impl BuildState {
    fn new() -> Self {
        BuildState { count: 0 }
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

    DatafusionContext::with_schema_provider(schema_name, |provider| {
        task::block_on(provider.create_table(&pg_relation))
    });

    let state = do_heap_scan(index_info, &pg_relation, &index_relation);

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
) -> BuildState {
    let mut state = BuildState::new();
    let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| unsafe {
        pg_sys::IndexBuildHeapScan(
            heap_relation.as_ptr(),
            index_relation.as_ptr(),
            index_info,
            Some(build_callback),
            &mut state,
        );
    }));
    state
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

    build_callback_internal(htup.t_self, values, state, index);
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
    build_callback_internal(*ctid, values, state, index);
}

#[inline(always)]
unsafe fn build_callback_internal(
    ctid: pg_sys::ItemPointerData,
    values: *mut pg_sys::Datum,
    _state: *mut std::os::raw::c_void,
    index: pg_sys::Relation,
) {
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
    let mut values: Vec<ArrayRef> = vec![];

    for (col_idx, attr) in tuple_desc.iter().enumerate() {
        values.push(
            DatafusionMapProducer::array(
                attr.type_oid().to_sql_data_type(attr.type_mod()).unwrap(),
                &mut tuple_table_slot,
                1,
                col_idx,
            )
            .unwrap(),
        );
    }

    info!("values: {:?}", values);
}
