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

use crate::index::fast_fields_helper::FFType;
use crate::index::{open_search_reader, open_search_writer, WriterResources};
use crate::postgres::utils::u64_to_item_pointer;
use pgrx::*;

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

    let reader = open_search_reader(&index_relation)
        .expect("ambulkdelete: should be able to open SearchIndexReader");
    let mut writer = open_search_writer(&index_relation, WriterResources::Vacuum)
        .expect("ambulkdelete: should be able to open SearchIndexWriter");
    let ctid_field = reader
        .schema
        .schema
        .get_field("ctid")
        .expect("ambulkdelete: ctid field should exist in index schema");
    for segment_reader in reader.searcher.segment_readers() {
        let fast_fields = segment_reader.fast_fields();
        let ctid_ff = FFType::new(fast_fields, "ctid");
        if let FFType::U64(ff) = ctid_ff {
            for ctid in ff.iter().filter_map(|ctid_u64| unsafe {
                let mut ipd = pg_sys::ItemPointerData::default();
                u64_to_item_pointer(ctid_u64, &mut ipd);

                let callback_fn = callback.as_ref().unwrap();
                callback_fn(&mut ipd, callback_state)
                    .then(|| tantivy::Term::from_field_u64(ctid_field, ctid_u64))
            }) {
                writer.delete_term(ctid);
            }
        }
    }
    let blocking_stats = writer
        .vacuum()
        .expect("ambulkdelete: tantivy vacuum should succeed");

    if stats.is_null() {
        stats = unsafe {
            PgBox::from_pg(
                pg_sys::palloc0(std::mem::size_of::<pg_sys::IndexBulkDeleteResult>()).cast(),
            )
        };
        stats.pages_deleted = 0;
    }

    stats.pages_deleted += blocking_stats.deleted_paths.len() as u32;
    stats.into_pg()
}
