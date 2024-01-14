use crate::index_access::options::ParadeOptions;
use crate::index_access::utils::{create_parade_index, get_parade_index, lookup_index_tupdesc};
use pgrx::*;
use std::panic::{self, AssertUnwindSafe};

// For now just pass the count and parade
// index on the build callback state
struct BuildState {
    count: usize,
}

impl BuildState {
    fn new() -> Self {
        BuildState { count: 0 }
    }
}

#[pg_guard]
// TODO: remove the unsafe
pub extern "C" fn ambuild(
    heaprel: pg_sys::Relation,
    indexrel: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) -> *mut pg_sys::IndexBuildResult {
    let heap_relation = unsafe { PgRelation::from_pg(heaprel) };
    let index_relation = unsafe { PgRelation::from_pg(indexrel) };
    let index_name = index_relation.name().to_string();

    // rdopts are passed on to create_parade_index
    let rdopts: PgBox<ParadeOptions> = if !index_relation.rd_options.is_null() {
        unsafe { PgBox::from_pg(index_relation.rd_options as *mut ParadeOptions) }
    } else {
        let ops = unsafe { PgBox::<ParadeOptions>::alloc0() };
        ops.into_pg_boxed()
    };

    create_parade_index(index_name.clone(), &heap_relation, rdopts).unwrap();

    let state = do_heap_scan(index_info, &heap_relation, &index_relation);

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
    let _ = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
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

#[allow(unused_variables)]
#[inline(always)]
unsafe extern "C" fn build_callback_internal(
    ctid: pg_sys::ItemPointerData,
    values: *mut pg_sys::Datum,
    _state: *mut std::os::raw::c_void,
    index: pg_sys::Relation,
) {
    check_for_interrupts!();

    let index_relation_ref: PgRelation = PgRelation::from_pg(index);
    let tupdesc = lookup_index_tupdesc(&index_relation_ref);
    let index_name = index_relation_ref.name();
    let parade_index = get_parade_index(index_name);
    // let index_entries = parade_index
    //     .row_to_index_entries(ctid, &tupdesc, values)
    //     .unwrap_or_else(|err| {
    //         panic!("error creating index entries for index '{index_name}': {err:?}",)
    //     });

    // parade_index.insert(index_entries).unwrap_or_else(|err| {
    //     panic!("error inserting json builder during index build callback: {err:?}")
    // });
}
