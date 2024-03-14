use pgrx::*;

use crate::{globals::WriterGlobal, index::SearchIndex, writer::WriterDirectory};

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
    let directory = WriterDirectory::from_index_name(index_name);
    let search_index = SearchIndex::from_cache(&directory)
        .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));

    // Garbage collect the index and clear the writer cache to free up locks.
    let writer_client = WriterGlobal::client();
    search_index
        .vacuum(&writer_client)
        .unwrap_or_else(|err| panic!("error during vacuum on index {index_name}: {err:?}"));

    stats
}
