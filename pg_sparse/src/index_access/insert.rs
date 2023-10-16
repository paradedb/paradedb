use pgrx::*;

use crate::sparse_index::{Sparse, SparseIndex};

#[allow(clippy::too_many_arguments)]
#[cfg(any(feature = "pg14", feature = "pg15"))]
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

#[cfg(any(feature = "pg11", feature = "pg12", feature = "pg13"))]
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
    let index_name = index_relation_ref.name().to_string();

    let mut sparse_index = SparseIndex::from_index_name(index_name);
    let values = std::slice::from_raw_parts(values, 1);
    let sparse_vector: Option<Sparse> = FromDatum::from_datum(values[0], false);

    if let Some(sparse_vector) = sparse_vector {
        sparse_index.insert(sparse_vector, *heap_tid);
        true
    } else {
        false
    }
}
