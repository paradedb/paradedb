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
use pgrx::{pg_sys::ItemPointerData, *};

#[pg_guard]
pub extern "C" fn ambulkdelete(
    info: *mut pg_sys::IndexVacuumInfo,
    stats: *mut pg_sys::IndexBulkDeleteResult,
    callback: pg_sys::IndexBulkDeleteCallback,
    callback_state: *mut ::std::os::raw::c_void,
) -> *mut pg_sys::IndexBulkDeleteResult {
    let info = unsafe { PgBox::from_pg(info) };
    let mut stats = unsafe { PgBox::from_pg(stats) };
    let index_relation = unsafe { PgRelation::from_pg(info.index) };

    let search_index =
        open_search_index(&index_relation).expect("should be able to open search index");
    let reader = search_index
        .get_reader()
        .unwrap_or_else(|err| panic!("error loading index reader in bulkdelete: {err}"));
    let mut writer = search_index
        .get_writer(WriterResources::Vacuum)
        .unwrap_or_else(|err| panic!("error loading index writer in bulkdelete: {err}"));

    if stats.is_null() {
        stats = unsafe {
            PgBox::from_pg(
                pg_sys::palloc0(std::mem::size_of::<pg_sys::IndexBulkDeleteResult>()).cast(),
            )
        };
    }

    if let Some(actual_callback) = callback {
        let should_delete = |ctid_val| unsafe {
            let mut ctid = ItemPointerData::default();
            crate::postgres::utils::u64_to_item_pointer(ctid_val, &mut ctid);
            actual_callback(&mut ctid, callback_state)
        };
        match search_index.delete(&reader, &writer, should_delete) {
            Ok((deleted, not_deleted)) => {
                stats.pages_deleted += deleted;
                stats.num_pages += not_deleted;
            }
            Err(err) => {
                panic!("error: {err:?}")
            }
        }
    }

    pgrx::warning!("committing vacuum");
    writer
        .commit()
        .unwrap_or_else(|err| panic!("error committing to index in ambulkdelete: {err}"));

    stats.into_pg()
}
