use crate::parade_index::writer::PARADE_WRITER_CACHE;
use pgrx::*;

#[pg_guard]
pub extern "C" fn amvacuumcleanup(
    info: *mut pg_sys::IndexVacuumInfo,
    stats: *mut pg_sys::IndexBulkDeleteResult,
) -> *mut pg_sys::IndexBulkDeleteResult {
    let info = unsafe { PgBox::from_pg(info) };
    let mut stats = stats;

    if info.analyze_only {
        return stats;
    }

    if stats.is_null() {
        stats =
            unsafe { pg_sys::palloc0(std::mem::size_of::<pg_sys::IndexBulkDeleteResult>()).cast() };
    }

    let index_rel: pg_sys::Relation = info.index;
    let index_relation = unsafe { PgRelation::from_pg(index_rel) };
    let index_name = index_relation.name();

    // Garbage collect the index and clear the writer cache to free up locks.
    let parade_writer = unsafe { PARADE_WRITER_CACHE.get_cached(index_name) };
    parade_writer.garbage_collect();
    unsafe {
        PARADE_WRITER_CACHE.clear_cache();
    }

    stats
}
