// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::index::get_index_schema;
use crate::index::mvcc::MVCCDirectory;
use crate::postgres::build_parallel::build_index;
use crate::postgres::storage::block::{
    SegmentMetaEntry, CLEANUP_LOCK, METADATA, SCHEMA_START, SEGMENT_METAS_START, SETTINGS_START,
};
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::metadata::MetaPageMut;
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use pgrx::*;
use tantivy::{Index, IndexSettings};

#[pg_guard]
pub extern "C-unwind" fn ambuild(
    heaprel: pg_sys::Relation,
    indexrel: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) -> *mut pg_sys::IndexBuildResult {
    let heap_relation = unsafe { PgRelation::from_pg(heaprel) };
    let index_relation = unsafe { PgRelation::from_pg(indexrel) };

    // ensure we only allow one `USING bm25` index on this relation, accounting for a REINDEX
    // and accounting for CONCURRENTLY.
    unsafe {
        let index_tuple = &(*index_relation.rd_index);
        let is_reindex = !index_tuple.indisvalid;
        let is_concurrent = (*index_info).ii_Concurrent;

        if !is_reindex {
            for existing_index in heap_relation.indices(pg_sys::AccessShareLock as _) {
                if existing_index.oid() == index_relation.oid() {
                    // the index we're about to build already exists on the table.
                    continue;
                }

                if is_bm25_index(&existing_index) && !is_concurrent {
                    panic!("a relation may only have one `USING bm25` index");
                }
            }
        }
    }

    unsafe {
        ambuildempty(indexrel);

        let index_oid = index_relation.oid();
        let (heap_tuples, segment_ids) =
            build_index(heap_relation, index_relation, (*index_info).ii_Concurrent)
                .unwrap_or_else(|e| panic!("{e}"));

        let mut result = PgBox::<pg_sys::IndexBuildResult>::alloc0();
        result.heap_tuples = heap_tuples;
        result.index_tuples = heap_tuples;

        let metadata = MetaPageMut::new(index_oid);
        metadata
            .record_create_index_segment_ids(segment_ids.iter())
            .expect("do_heap_scan: should be able to record segment ids in merge lock");

        pg_sys::FlushRelationBuffers(indexrel);
        result.into_pg()
    }
}

#[pg_guard]
pub unsafe extern "C-unwind" fn ambuildempty(index_relation: pg_sys::Relation) {
    let indexrel = unsafe { PgRelation::from_pg(index_relation) };
    unsafe {
        init_fixed_buffers(&indexrel);
    }

    let schema = get_index_schema(&indexrel).unwrap_or_else(|e| panic!("{e}"));
    let directory = MVCCDirectory::snapshot(indexrel.oid());
    let settings = IndexSettings {
        docstore_compress_dedicated_thread: false,
        ..Default::default()
    };
    Index::create(directory, schema.into(), settings).unwrap_or_else(|e| panic!("{e}"));
}

pub fn is_bm25_index(indexrel: &PgRelation) -> bool {
    indexrel.rd_amhandler == bm25_amhandler_oid().unwrap_or_default()
}

fn bm25_amhandler_oid() -> Option<pg_sys::Oid> {
    unsafe {
        let name = pg_sys::Datum::from(c"bm25".as_ptr());
        let pg_am_entry = pg_sys::SearchSysCache1(pg_sys::SysCacheIdentifier::AMNAME as _, name);
        if pg_am_entry.is_null() {
            return None;
        }

        let mut is_null = false;
        let datum = pg_sys::SysCacheGetAttr(
            pg_sys::SysCacheIdentifier::AMNAME as _,
            pg_am_entry,
            pg_sys::Anum_pg_am_amhandler as _,
            &mut is_null,
        );
        let oid = pg_sys::Oid::from_datum(datum, is_null);
        pg_sys::ReleaseSysCache(pg_am_entry);
        oid
    }
}

unsafe fn init_fixed_buffers(index_relation: &PgRelation) {
    let relation_oid = index_relation.oid();
    let mut bman = BufferManager::new(relation_oid);

    // Init merge lock buffer
    let mut merge_lock = bman.new_buffer();
    assert_eq!(merge_lock.number(), METADATA);
    merge_lock.init_page();

    // Init cleanup lock buffer
    let mut cleanup_lock = bman.new_buffer();
    assert_eq!(cleanup_lock.number(), CLEANUP_LOCK);
    cleanup_lock.init_page();

    // initialize all the other required buffers
    let schema = LinkedBytesList::create(relation_oid);
    let settings = LinkedBytesList::create(relation_oid);
    let segment_metas = LinkedItemList::<SegmentMetaEntry>::create(relation_oid);

    assert_eq!(schema.header_blockno, SCHEMA_START);
    assert_eq!(settings.header_blockno, SETTINGS_START);
    assert_eq!(segment_metas.header_blockno, SEGMENT_METAS_START);
}
