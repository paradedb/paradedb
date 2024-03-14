use pgrx::*;

use crate::{
    env::register_commit_callback, globals::WriterGlobal, index::SearchIndex,
    writer::WriterDirectory,
};

#[pg_guard]
pub extern "C" fn ambulkdelete(
    info: *mut pg_sys::IndexVacuumInfo,
    stats: *mut pg_sys::IndexBulkDeleteResult,
    callback: pg_sys::IndexBulkDeleteCallback,
    callback_state: *mut ::std::os::raw::c_void,
) -> *mut pg_sys::IndexBulkDeleteResult {
    let info = unsafe { PgBox::from_pg(info) };
    let mut stats = unsafe { PgBox::from_pg(stats) };
    let index_rel: pg_sys::Relation = info.index;
    let index_relation = unsafe { PgRelation::from_pg(index_rel) };
    let index_name = index_relation.name();
    let directory = WriterDirectory::from_index_name(index_name);
    let search_index = SearchIndex::from_cache(&directory)
        .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));

    if stats.is_null() {
        stats = unsafe {
            PgBox::from_pg(
                pg_sys::palloc0(std::mem::size_of::<pg_sys::IndexBulkDeleteResult>()).cast(),
            )
        };
    }

    let writer_client = WriterGlobal::client();
    register_commit_callback(&writer_client, search_index.directory.clone())
        .expect("could not register commit callbacks for delete operation");

    if let Some(actual_callback) = callback {
        match search_index.delete(&writer_client, |ctid| unsafe {
            actual_callback(ctid, callback_state)
        }) {
            Ok((deleted, not_deleted)) => {
                stats.pages_deleted += deleted;
                stats.num_pages += not_deleted;
            }
            Err(err) => {
                panic!("error: {err:?}")
            }
        }
    }

    stats.into_pg()
}
