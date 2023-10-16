use pgrx::*;
use std::panic::{self, AssertUnwindSafe};

use crate::sparse_index::SparseIndex;

struct BuildState<'a> {
    count: usize,
    sparse_index: &'a mut SparseIndex,
    memcxt: PgMemoryContexts,
}

impl<'a> BuildState<'a> {
    fn new(sparse_index: &'a mut SparseIndex) -> Self {
        BuildState {
            sparse_index,
            count: 0,
            memcxt: PgMemoryContexts::new("SparseIndex build context"),
        }
    }
}

#[pg_guard]
pub extern "C" fn ambuild(
    heaprel: pg_sys::Relation,
    indexrel: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) -> *mut pg_sys::IndexBuildResult {
    info!("Reached ambuild");
    let heap_relation = unsafe { PgRelation::from_pg(heaprel) };
    let index_relation = unsafe { PgRelation::from_pg(indexrel) };
    let index_name = index_relation.name().to_string();
    let table_name = heap_relation.name().to_string();
    let schema_name = heap_relation.namespace().to_string();

    // Create SparseIndex
    let mut sparse_index = SparseIndex::new(index_name.clone());

    let ntuples = do_heap_scan(
        index_info,
        &heap_relation,
        &index_relation,
        &mut sparse_index,
    );

    let mut result = unsafe { PgBox::<pg_sys::IndexBuildResult>::alloc0() };
    result.heap_tuples = ntuples as f64;
    result.index_tuples = ntuples as f64;

    result.into_pg()
}

#[pg_guard]
pub extern "C" fn ambuildempty(_index_relation: pg_sys::Relation) {
    info!("ambuildempty")
}

fn do_heap_scan<'a>(
    index_info: *mut pg_sys::IndexInfo,
    heap_relation: &'a PgRelation,
    index_relation: &'a PgRelation,
    sparse_index: &mut SparseIndex,
) -> usize {
    let mut state = BuildState::new(sparse_index);
    let _ = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        pg_sys::IndexBuildHeapScan(
            heap_relation.as_ptr(),
            index_relation.as_ptr(),
            index_info,
            Some(build_callback),
            &mut state,
        );
    }));
    state.count
}

#[cfg(any(feature = "pg10", feature = "pg11", feature = "pg12"))]
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

#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
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
unsafe extern "C" fn build_callback_internal(
    ctid: pg_sys::ItemPointerData,
    values: *mut pg_sys::Datum,
    state: *mut std::os::raw::c_void,
    index: pg_sys::Relation,
) {
    check_for_interrupts!();

    let index_relation_ref = unsafe { PgRelation::from_pg(index) };
    // let tupdesc = lookup_index_tupdesc(&index_relation_ref);
    // let attributes = categorize_tupdesc(&tupdesc);
    // let natts = tupdesc.natts as usize;
    // let dropped = (0..tupdesc.natts as usize)
    //     .map(|i| tupdesc.get(i).unwrap().is_dropped())
    //     .collect::<Vec<bool>>();

    let state = (state as *mut BuildState).as_mut().unwrap();
    let mut old_context = state.memcxt.set_as_current();

    let values = std::slice::from_raw_parts(values, 1);
    info!("{:?}", values);
    // let builder = row_to_json(values[0], &tupdesc, natts, &dropped, &attributes);

    // Insert row to parade index
    // state.parade_index.insert(state.writer, ctid, builder);

    old_context.set_as_current();
    state.memcxt.reset();
}
