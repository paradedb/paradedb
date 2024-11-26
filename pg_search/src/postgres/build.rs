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

use crate::index::{SearchIndex, WriterResources};
use crate::postgres::index::get_fields;
use crate::postgres::insert::init_insert_state;
use crate::postgres::storage::block::{
    MetaPageData, INDEX_WRITER_LOCK_BLOCKNO, MANAGED_LOCK_BLOCKNO, METADATA_BLOCKNO,
    META_LOCK_BLOCKNO, TANTIVY_META_BLOCKNO,
};
use crate::postgres::storage::utils::BM25BufferCache;
use crate::postgres::utils::row_to_search_document;
use pgrx::*;
use std::ffi::CStr;
use std::time::Instant;

// For now just pass the count on the build callback state
struct BuildState {
    count: usize,
    memctx: PgMemoryContexts,
    index_info: *mut pg_sys::IndexInfo,
    tupdesc: PgTupleDesc<'static>,
    start: Instant,
}

impl BuildState {
    fn new(indexrel: &PgRelation, index_info: *mut pg_sys::IndexInfo) -> Self {
        BuildState {
            count: 0,
            memctx: PgMemoryContexts::new("pg_search_index_build"),
            index_info,
            tupdesc: unsafe { PgTupleDesc::from_pg_copy(indexrel.rd_att) },
            start: Instant::now(),
        }
    }
}

#[pg_guard]
pub extern "C" fn ambuild(
    heaprel: pg_sys::Relation,
    indexrel: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) -> *mut pg_sys::IndexBuildResult {
    let heap_relation = unsafe { PgRelation::from_pg(heaprel) };
    let index_relation = unsafe { PgRelation::from_pg(indexrel) };
    let index_oid = index_relation.oid();

    // Create the metadata blocks for the index
    unsafe { create_metadata(index_oid) };

    // ensure we only allow one `USING bm25` index on this relation, accounting for a REINDEX
    // and accounting for CONCURRENTLY.
    unsafe {
        let index_tuple = &(*index_relation.rd_index);
        let is_reindex = !index_tuple.indisvalid;
        let is_concurrent = (*index_info).ii_Concurrent;

        if !is_reindex {
            for existing_index in heap_relation.indices(pg_sys::AccessShareLock as _) {
                if existing_index.oid() == index_oid {
                    // the index we're about to build already exists on the table.
                    continue;
                }

                if is_bm25_index(&existing_index) && !is_concurrent {
                    panic!("a relation may only have one `USING bm25` index");
                }
            }
        }
    }

    let (fields, key_field_index) = unsafe { get_fields(&index_relation) };
    // If there's only two fields in the vector, then those are just the Key and Ctid fields,
    // which we added above, and the user has not specified any fields to index.
    if fields.len() == 2 {
        panic!("no fields specified")
    }

    SearchIndex::create_index(index_oid, fields, key_field_index)
        .expect("error creating new index instance");

    let state = do_heap_scan(index_info, &heap_relation, &index_relation);
    unsafe {
        let insert_state = init_insert_state(indexrel, index_info, WriterResources::CreateIndex);
        (*insert_state).try_commit().expect("commit should succeed");
    }

    let mut result = unsafe { PgBox::<pg_sys::IndexBuildResult>::alloc0() };
    result.heap_tuples = state.count as f64;
    result.index_tuples = state.count as f64;
    result.into_pg()
}

#[pg_guard]
pub extern "C" fn ambuildempty(_index_relation: pg_sys::Relation) {}

fn do_heap_scan<'a>(
    index_info: *mut pg_sys::IndexInfo,
    heap_relation: &'a PgRelation,
    index_relation: &'a PgRelation,
) -> BuildState {
    let mut state = BuildState::new(index_relation, index_info);
    unsafe {
        pg_sys::IndexBuildHeapScan(
            heap_relation.as_ptr(),
            index_relation.as_ptr(),
            index_info,
            Some(build_callback),
            &mut state,
        );

        let insert_state = init_insert_state(
            index_relation.as_ptr(),
            index_info,
            WriterResources::CreateIndex,
        );
        if let Some(writer) = (*insert_state).writer.take() {
            writer
                .commit()
                .unwrap_or_else(|e| panic!("failed to commit new tantivy index: {e}"));

            // writer
            //     .underlying_writer
            //     .wait_merging_threads()
            //     .unwrap_or_else(|e| panic!("failed to wait for index merge: {e}"));
        }
    }

    state
}

#[pg_guard]
unsafe extern "C" fn build_callback(
    index: pg_sys::Relation,
    ctid: pg_sys::ItemPointer,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    _tuple_is_alive: bool,
    state: *mut std::os::raw::c_void,
) {
    build_callback_internal(*ctid, values, isnull, state, index);
}

#[inline(always)]
unsafe fn build_callback_internal(
    ctid: pg_sys::ItemPointerData,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    state: *mut std::os::raw::c_void,
    indexrel: pg_sys::Relation,
) {
    check_for_interrupts!();
    let build_state = (state as *mut BuildState)
        .as_mut()
        .expect("BuildState pointer should not be null");

    let tupdesc = &build_state.tupdesc;
    let insert_state = init_insert_state(
        indexrel,
        build_state.index_info,
        WriterResources::CreateIndex,
    );
    let search_index = &(*insert_state).index;
    let writer = (*insert_state)
        .writer
        .as_mut()
        .expect("writer should not be null");
    let schema = &(*insert_state).index.schema;
    // In the block below, we switch to the memory context we've defined on our build
    // state, resetting it before and after. We do this because we're looking up a
    // PgTupleDesc... which is supposed to free the corresponding Postgres memory when it
    // is dropped. However, in practice, we're not seeing the memory get freed, which is
    // causing huge memory usage when building large indexes.
    //
    // By running in our own memory context, we can force the memory to be freed with
    // the call to reset().
    unsafe {
        build_state.memctx.reset();
        build_state.memctx.switch_to(|_| {
            let search_document =
                row_to_search_document(ctid, tupdesc, values, isnull, schema).unwrap_or_else(|err| {
                    panic!(
                        "error creating index entries for index '{}': {err}",
                        CStr::from_ptr((*(*indexrel).rd_rel).relname.data.as_ptr())
                            .to_string_lossy()
                    );
                });
            search_index
                .insert(writer, search_document)
                .unwrap_or_else(|err| {
                    panic!("error inserting document during build callback.  See Postgres log for more information: {err:?}")
                });
        });
        build_state.memctx.reset();

        // important to count the number of items we've indexed for proper statistics updates,
        // especially after CREATE INDEX has finished
        build_state.count += 1;

        if crate::gucs::log_create_index_progress() && build_state.count % 100_000 == 0 {
            let secs = build_state.start.elapsed().as_secs_f64();
            let rate = build_state.count as f64 / secs;
            pgrx::log!(
                "processed {} rows in {secs:.2} seconds ({rate:.2} per second)",
                build_state.count,
            );
        }
    }
}

fn is_bm25_index(indexrel: &PgRelation) -> bool {
    unsafe {
        // SAFETY:  we ensure that `indexrel.rd_indam` is non null and can be dereferenced
        !indexrel.rd_indam.is_null() && (*indexrel.rd_indam).ambuild == Some(ambuild)
    }
}

unsafe fn create_metadata(relation_oid: pg_sys::Oid) {
    let cache = BM25BufferCache::open(relation_oid);
    let metadata_buffer = cache.new_buffer();
    let page = pg_sys::BufferGetPage(metadata_buffer);
    let metadata = pg_sys::PageGetContents(page) as *mut MetaPageData;

    let writer_lock_buffer = cache.new_buffer();
    let meta_lock_buffer = cache.new_buffer();
    let managed_lock_buffer = cache.new_buffer();
    let tantivy_meta_buffer = cache.new_buffer();

    let directory_buffer = cache.new_buffer();
    let directory_blockno = pg_sys::BufferGetBlockNumber(directory_buffer);
    (*metadata).directory_start = directory_blockno;

    let segment_metas_buffer = cache.new_buffer();
    let segment_metas_blockno = pg_sys::BufferGetBlockNumber(segment_metas_buffer);
    (*metadata).segment_metas_start = segment_metas_blockno;

    assert!(pg_sys::BufferGetBlockNumber(metadata_buffer) == METADATA_BLOCKNO);
    assert!(pg_sys::BufferGetBlockNumber(writer_lock_buffer) == INDEX_WRITER_LOCK_BLOCKNO);
    assert!(pg_sys::BufferGetBlockNumber(meta_lock_buffer) == META_LOCK_BLOCKNO);
    assert!(pg_sys::BufferGetBlockNumber(managed_lock_buffer) == MANAGED_LOCK_BLOCKNO);
    assert!(pg_sys::BufferGetBlockNumber(tantivy_meta_buffer) == TANTIVY_META_BLOCKNO);

    pg_sys::MarkBufferDirty(metadata_buffer);
    pg_sys::MarkBufferDirty(writer_lock_buffer);
    pg_sys::MarkBufferDirty(meta_lock_buffer);
    pg_sys::MarkBufferDirty(managed_lock_buffer);
    pg_sys::MarkBufferDirty(directory_buffer);
    pg_sys::MarkBufferDirty(tantivy_meta_buffer);
    pg_sys::MarkBufferDirty(segment_metas_buffer);

    pg_sys::UnlockReleaseBuffer(metadata_buffer);
    pg_sys::UnlockReleaseBuffer(writer_lock_buffer);
    pg_sys::UnlockReleaseBuffer(meta_lock_buffer);
    pg_sys::UnlockReleaseBuffer(managed_lock_buffer);
    pg_sys::UnlockReleaseBuffer(directory_buffer);
    pg_sys::UnlockReleaseBuffer(tantivy_meta_buffer);
    pg_sys::UnlockReleaseBuffer(segment_metas_buffer);
}
