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

use super::utils::relfilenode_from_index_oid;
use crate::index::SearchIndex;
use crate::index::WriterDirectory;
use crate::postgres::utils::row_to_search_document;
use pgrx::{pg_guard, pg_sys, pgrx_extern_c_guard, PgMemoryContexts, PgTupleDesc};
use std::ffi::CStr;
use std::panic::{catch_unwind, resume_unwind};

struct InsertState {
    index: SearchIndex,
    abort_on_drop: bool,
}

impl Drop for InsertState {
    /// When [`InsertState`] is dropped we'll either commit the underlying tantivy index changes
    /// or abort.
    fn drop(&mut self) {
        unsafe {
            pgrx_extern_c_guard(|| {
                if !pg_sys::IsAbortedTransactionBlockState() && !self.abort_on_drop {
                    self.index
                        .commit()
                        .expect("tantivy index commit should succeed");
                } else if let Err(e) = self.index.abort() {
                    if pg_sys::IsAbortedTransactionBlockState() {
                        // we're in an aborted state, so the best we can do is warn that our
                        // attempt to abort the tantivy changes failed
                        pgrx::warning!("failed to abort tantivy index changes: {}", e);
                    } else {
                        // haven't aborted yet so we can raise the error we got during abort
                        panic!("{e}")
                    }
                }
            });
        }
    }
}

impl InsertState {
    fn new(
        database_oid: pg_sys::Oid,
        index_oid: pg_sys::Oid,
        relfilenode: pg_sys::Oid,
    ) -> anyhow::Result<Self> {
        let directory = WriterDirectory::from_oids(
            database_oid.as_u32(),
            index_oid.as_u32(),
            relfilenode.as_u32(),
        );

        Ok(Self {
            index: SearchIndex::open_direct(&directory)?,
            abort_on_drop: false,
        })
    }
}

unsafe fn init_insert_state(
    index_relation: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) -> *mut InsertState {
    assert!(!index_info.is_null());
    let state = (*index_info).ii_AmCache;
    if state.is_null() {
        // we don't have any cached state yet, so create it now
        let state = InsertState::new(
            pg_sys::MyDatabaseId,
            (*index_relation).rd_id,
            relfilenode_from_index_oid((*index_relation).rd_id.as_u32()),
        )
        .expect("should be able to open new SearchIndex for writing");

        // leak it into the MemoryContext for this scan (as specified by the IndexInfo argument)
        //
        // When that memory context is freed by Postgres is when we'll do our tantivy commit/abort
        // of the changes made during `aminsert`
        (*index_info).ii_AmCache = PgMemoryContexts::For((*index_info).ii_Context)
            .leak_and_drop_on_delete(state)
            .cast();
    };

    (*index_info).ii_AmCache.cast()
}

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
    index_info: *mut pg_sys::IndexInfo,
) -> bool {
    let result = catch_unwind(|| {
        let state = &mut *init_insert_state(index_relation, index_info);
        let tupdesc = PgTupleDesc::from_pg_unchecked((*index_relation).rd_att);
        let search_index = &mut state.index;
        let search_document =
            row_to_search_document(*ctid, &tupdesc, values, isnull, &search_index.schema)
                .unwrap_or_else(|err| {
                    panic!(
                        "error creating index entries for index '{}': {err}",
                        CStr::from_ptr((*(*index_relation).rd_rel).relname.data.as_ptr())
                            .to_string_lossy()
                    );
                });
        search_index
            .insert(search_document)
            .expect("insertion into index should succeed");
        true
    });

    match result {
        Ok(result) => result,
        Err(e) => {
            unsafe {
                // SAFETY:  it's possible the `ii_AmCache` field didn't get initialized, and if
                // that's the case there's no need (or way!) to indicate we need to do a tantiy abort
                let state = (*index_info).ii_AmCache.cast::<InsertState>();
                if !state.is_null() {
                    (*state).abort_on_drop = true;
                }
            }

            // bubble up the panic that we caught during `catch_unwind()`
            resume_unwind(e)
        }
    }
}
