use pgrx::*;

use crate::sparse_index::index::{from_index_name, get_index_path, resize_if_needed};
use crate::sparse_index::sparse::Sparse;

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
    heap_tid: pg_sys::ItemPointer,
) -> bool {
    let index_relation_ref: PgRelation = PgRelation::from_pg(index_relation);
    let index_name = index_relation_ref.name();

    let values = std::slice::from_raw_parts(values, 1);
    let sparse_vector: Option<Sparse> = FromDatum::from_datum(values[0], false);
    let mut sparse_index = from_index_name(index_name);
    let index_path = get_index_path(index_name);

    // Resize index if needed
    resize_if_needed(&mut sparse_index);

    if let Some(sparse_vector) = sparse_vector {
        let tid = item_pointer_to_u64(*heap_tid) as usize;
        sparse_index.add_sparse_vector(sparse_vector.entries, tid);
        sparse_index.save_index(index_path);
        true
    } else {
        false
    }
}
