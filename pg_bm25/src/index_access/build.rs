use pgrx::*;
use std::panic::{self, AssertUnwindSafe};
use tantivy::SingleSegmentIndexWriter;

use crate::index_access::options::ParadeOptions;
use crate::index_access::utils::{
    categorize_tupdesc, create_parade_index, lookup_index_tupdesc, row_to_json,
};
use crate::parade_index::index::ParadeIndex;

const INDEX_WRITER_MEM_BUDGET: usize = 50_000_000;

// For now just pass the count and parade
// index on the build callback state
struct BuildState<'a> {
    count: usize,
    parade_index: &'a mut ParadeIndex,
    writer: &'a mut SingleSegmentIndexWriter,
    memcxt: PgMemoryContexts,
}

impl<'a> BuildState<'a> {
    fn new(parade_index: &'a mut ParadeIndex, writer: &'a mut SingleSegmentIndexWriter) -> Self {
        BuildState {
            parade_index,
            writer,
            count: 0,
            memcxt: PgMemoryContexts::new("ParadeDB build context"),
        }
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
    let table_name = heap_relation.name().to_string();
    let schema_name = heap_relation.namespace().to_string();

    // rdopts are passed on to create_parade_index
    let rdopts: PgBox<ParadeOptions>;
    if !index_relation.rd_options.is_null() {
        rdopts = unsafe { PgBox::from_pg(index_relation.rd_options as *mut ParadeOptions) };
        let token_option = rdopts.get_tokenizer();
        info!("token option: {}", token_option);
    } else {
        info!("index relation has no options");
        let ops = unsafe { PgBox::<ParadeOptions>::alloc0() };
        rdopts = ops.into_pg_boxed();
    }

    // Create ParadeDB Index
    let mut parade_index = create_parade_index(
        index_name.clone(),
        format!("{}.{}", schema_name, table_name),
        rdopts,
    );
    let tantivy_index = parade_index.copy_tantivy_index();
    let mut writer = SingleSegmentIndexWriter::new(tantivy_index, INDEX_WRITER_MEM_BUDGET)
        .expect("failed to create index writer");

    let ntuples = do_heap_scan(
        index_info,
        &heap_relation,
        &index_relation,
        &mut parade_index,
        &mut writer,
    );

    writer.finalize().expect("failed to finalize writer");

    let mut result = unsafe { PgBox::<pg_sys::IndexBuildResult>::alloc0() };
    result.heap_tuples = ntuples as f64;
    result.index_tuples = ntuples as f64;

    result.into_pg()
}

#[pg_guard]
pub extern "C" fn ambuildempty(_index_relation: pg_sys::Relation) {}

fn do_heap_scan<'a>(
    index_info: *mut pg_sys::IndexInfo,
    heap_relation: &'a PgRelation,
    index_relation: &'a PgRelation,
    parade_index: &mut ParadeIndex,
    writer: &mut SingleSegmentIndexWriter,
) -> usize {
    let mut state = BuildState::new(parade_index, writer);
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
    let tupdesc = lookup_index_tupdesc(&index_relation_ref);
    let attributes = categorize_tupdesc(&tupdesc);
    let natts = tupdesc.natts as usize;
    let dropped = (0..tupdesc.natts as usize)
        .map(|i| tupdesc.get(i).unwrap().is_dropped())
        .collect::<Vec<bool>>();

    let state = (state as *mut BuildState).as_mut().unwrap();
    let mut old_context = state.memcxt.set_as_current();

    let values = std::slice::from_raw_parts(values, 1);
    let builder = row_to_json(values[0], &tupdesc, natts, &dropped, &attributes);

    // Insert row to parade index
    state.parade_index.insert(state.writer, ctid, builder);

    old_context.set_as_current();
    state.memcxt.reset();
}
