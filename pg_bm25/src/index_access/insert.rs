use crate::index_access::utils::{
    categorize_tupdesc, get_parade_index, lookup_index_tupdesc, row_to_json,
};
use crate::parade_index::writer::ParadeWriter;
use pgrx::*;
use std::ffi::c_void;

#[allow(clippy::too_many_arguments)]
#[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
#[pg_guard]
pub unsafe extern "C" fn aminsert(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    _isnull: *mut bool,
    heap_tid: pg_sys::ItemPointer,
    _heap_relation: pg_sys::Relation,
    _check_unique: pg_sys::IndexUniqueCheck,
    _index_unchanged: bool,
    index_info: *mut pg_sys::IndexInfo,
) -> bool {
    aminsert_internal(index_relation, values, heap_tid, index_info)
}

#[cfg(any(feature = "pg12", feature = "pg13"))]
#[pg_guard]
pub unsafe extern "C" fn aminsert(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    _isnull: *mut bool,
    heap_tid: pg_sys::ItemPointer,
    _heap_relation: pg_sys::Relation,
    _check_unique: pg_sys::IndexUniqueCheck,
    index_info: *mut pg_sys::IndexInfo,
) -> bool {
    aminsert_internal(index_relation, values, heap_tid, index_info)
}

#[inline(always)]
unsafe fn aminsert_internal(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    ctid: pg_sys::ItemPointer,
    index_info: *mut pg_sys::IndexInfo,
) -> bool {
    let index_info_ref = &mut *index_info;
    let index_relation_ref: PgRelation = PgRelation::from_pg(index_relation);

    if index_info_ref.ii_AmCache.is_null() {
        // Allocate cache data
        let index_name = index_relation_ref.name();
        let parade_index = get_parade_index(index_name);
        let parade_writer = parade_index.parade_writer();

        // Allocate memory in ii_Context and store the pointer in ii_AmCache
        let cache_data = Box::new(parade_writer);
        let cache_ptr = Box::into_raw(cache_data) as *mut c_void;
        index_info_ref.ii_AmCache = cache_ptr;

        // Run cleanup
        register_xact_callback(PgXactCallbackEvent::Commit, move || {
            insert_cleanup(cache_ptr);
        });
    }

    let parade_writer = &mut *(index_info_ref.ii_AmCache as *mut ParadeWriter);

    let tupdesc = lookup_index_tupdesc(&index_relation_ref);
    let attributes = categorize_tupdesc(&tupdesc);
    let natts = tupdesc.natts as usize;
    let dropped = (0..tupdesc.natts as usize)
        .map(|i| tupdesc.get(i).unwrap().is_dropped())
        .collect::<Vec<bool>>();
    let values = std::slice::from_raw_parts(values, 1);
    let builder = row_to_json(values[0], &tupdesc, natts, &dropped, &attributes);

    parade_writer.insert(*ctid, builder);

    true
}

fn insert_cleanup(_am_cache: *mut c_void) {
    info!("WE ARE CLEANING UP THE TRANSACTION");
}
