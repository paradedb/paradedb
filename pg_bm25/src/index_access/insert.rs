use crate::index_access::utils::lookup_index_tupdesc;
use pgrx::*;

use super::utils::get_parade_index;

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
    index_info: *mut pg_sys::IndexInfo,
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
    let tupdesc = lookup_index_tupdesc(&index_relation_ref);
    let parade_index = get_parade_index(index_relation_ref.name());
    let builder = parade_index.json_builder(*ctid, &tupdesc, values);

    parade_index.insert(builder);

    // let parade_writer = PARADE_WRITER_CACHE.get_cached(index_relation_ref.name());
    // let parade_index_key =
    //     ParadeIndexKey::from_json_builder(&parade_writer.key_field_name, &builder).unwrap();

    // // First delete any existing entires with the same key.
    // parade_writer.delete_by_key(&parade_index_key);
    // parade_writer.insert(*ctid, builder);

    // Acquire  a writer, which may involve waiting for a lock to be acquired.
    // If the lock has been acquired in this transaction, the writer will be cached
    // so no further waiting is required. We'll also register some callbacks to
    // release the locks and clear the cache when the transaction ends.
    // PARADE_WRITER_CACHE.clear_cache_on_transaction_end();

    true
}
