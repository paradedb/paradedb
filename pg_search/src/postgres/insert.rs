use super::utils::get_search_index;
use crate::{env::register_commit_callback, globals::WriterGlobal};
use pgrx::*;

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
    _index_info: *mut pg_sys::IndexInfo,
) -> bool {
    aminsert_internal(index_relation, values, heap_tid)
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
    _index_info: *mut pg_sys::IndexInfo,
) -> bool {
    aminsert_internal(index_relation, values, heap_tid)
}

#[inline(always)]
unsafe fn aminsert_internal(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    ctid: pg_sys::ItemPointer,
) -> bool {
    let index_relation_ref: PgRelation = PgRelation::from_pg(index_relation);
    let tupdesc = index_relation_ref.tuple_desc();
    let index_name = index_relation_ref.name();
    let search_index = get_search_index(index_name);
    let search_document = search_index
        .row_to_search_document(*ctid, &tupdesc, values)
        .unwrap_or_else(|err| {
            panic!("error creating index entries for index '{index_name}': {err:?}",)
        });

    let writer_client = WriterGlobal::client();
    register_commit_callback(&writer_client, search_index.directory.clone())
        .expect("could not register commit callbacks for insert operation");

    search_index
        .insert(&writer_client, search_document)
        .unwrap_or_else(|err| panic!("error inserting document during insert callback: {err:?}"));

    true
}
