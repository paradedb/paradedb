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

use crate::index::writer::index::SearchIndexWriter;
use crate::index::{BlockDirectoryType, WriterResources};
use crate::postgres::utils::row_to_search_document;
use pgrx::itemptr::item_pointer_get_both;
use pgrx::{pg_guard, pg_sys, PgRelation, PgTupleDesc};
use std::ffi::CStr;

#[allow(clippy::too_many_arguments)]
#[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16", feature = "pg17"))]
#[pg_guard]
pub unsafe extern "C" fn aminsert(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    heap_tid: pg_sys::ItemPointer,
    _heap_relation: pg_sys::Relation,
    _check_unique: pg_sys::IndexUniqueCheck::Type,
    _index_unchanged: bool,
    index_info: *mut pg_sys::IndexInfo,
) -> bool {
    aminsert_internal(index_relation, values, isnull, heap_tid, index_info)
}

#[cfg(feature = "pg13")]
#[pg_guard]
pub unsafe extern "C" fn aminsert(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    heap_tid: pg_sys::ItemPointer,
    _heap_relation: pg_sys::Relation,
    _check_unique: pg_sys::IndexUniqueCheck::Type,
    index_info: *mut pg_sys::IndexInfo,
) -> bool {
    aminsert_internal(index_relation, values, isnull, heap_tid, index_info)
}

#[inline(always)]
unsafe fn aminsert_internal(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    ctid: pg_sys::ItemPointer,
    _index_info: *mut pg_sys::IndexInfo,
) -> bool {
    let tupdesc = PgTupleDesc::from_pg_unchecked((*index_relation).rd_att);
    let indexrel = PgRelation::from_pg(index_relation);
    let mut writer = SearchIndexWriter::open(
        &indexrel,
        BlockDirectoryType::Mvcc,
        WriterResources::Statement,
    )
    .expect("must be able to open SearchIndexWriter in aminsert");

    let search_document = row_to_search_document(&tupdesc, values, isnull, &writer.schema)
        .unwrap_or_else(|err| {
            panic!(
                "error creating index entries for index '{}': {err}",
                CStr::from_ptr((*(*index_relation).rd_rel).relname.data.as_ptr()).to_string_lossy()
            );
        });
    writer
        .insert(search_document, item_pointer_get_both(*ctid))
        .expect("insertion into index should succeed");

    writer
        .commit_inserts()
        .expect("tantivy index commit should succeed");

    true
}
