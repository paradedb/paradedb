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

use crate::index::WriterResources;
use crate::postgres::index::open_search_index;
use crate::postgres::options::SearchIndexCreateOptions;
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

    let index_relation = unsafe { PgRelation::from_pg(info.index) };
    let index_name = index_relation.name();

    let search_index =
        open_search_index(&index_relation).expect("should be able to open search index");
    let options = index_relation.rd_options as *mut SearchIndexCreateOptions;
    let mut writer = search_index
        .get_writer(WriterResources::Vacuum, unsafe {
            options.as_ref().unwrap()
        })
        .unwrap_or_else(|err| panic!("error loading index writer from directory: {err}"));

    // Garbage collect the index and clear the writer cache to free up locks.
    search_index
        .vacuum(&writer)
        .unwrap_or_else(|err| panic!("error during vacuum on index {index_name}: {err:?}"));

    // we also need to make sure segments get merged.
    //
    // we can force this by doing a .commit(), even tho we don't have changes
    // then directly taking control of the underlying_writer and waiting for the merge threads
    // to complete
    writer.commit().expect("commit should succeed");
    writer
        .underlying_writer
        .take()
        .unwrap()
        .wait_merging_threads()
        .expect("wait_merging_threads() should succeed");

    stats
}
