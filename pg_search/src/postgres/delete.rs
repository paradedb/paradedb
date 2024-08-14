// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use pgrx::{pg_sys::ItemPointerData, *};

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
    let directory = WriterDirectory::from_index_oid(index_relation.oid().as_u32());
    let search_index = SearchIndex::from_disk(&directory)
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
        let should_delete = |ctid_val| unsafe {
            let mut ctid = ItemPointerData::default();
            pgrx::u64_to_item_pointer(ctid_val, &mut ctid);
            actual_callback(&mut ctid, callback_state)
        };
        match search_index.should_delete(&writer_client, should_delete) {
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
