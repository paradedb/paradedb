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
use crate::postgres::storage::block::{
    DeleteMetaEntry, DirectoryEntry, SegmentMetaEntry, DELETE_METAS_START, DIRECTORY_START,
    SEGMENT_METAS_START,
};
use crate::postgres::storage::utils::{BM25BufferCache, BM25Page};
use crate::postgres::storage::LinkedItemList;
use pgrx::*;

#[pg_guard]
pub extern "C" fn amvacuumcleanup(
    info: *mut pg_sys::IndexVacuumInfo,
    stats: *mut pg_sys::IndexBulkDeleteResult,
) -> *mut pg_sys::IndexBulkDeleteResult {
    let info = unsafe { PgBox::from_pg(info) };
    if info.analyze_only {
        return stats;
    }

    let index_relation = unsafe { PgRelation::from_pg(info.index) };

    // vacuum the index, which is effectively just a forced commit() plus a wait_merging_threads()
    SearchIndexWriter::new(
        index_relation.oid(),
        BlockDirectoryType::Mvcc,
        WriterResources::Vacuum,
    )
    .expect("amvacuumcleanup: should be able to open a SearchIndexWriter")
    .vacuum()
    .expect("amvacuumcleanup: SearchIndexWriter.vacuum() should succeed");

    unsafe {
        // Garbage collect linked lists
        // If new LinkedItemLists are created they should be garbage collected here
        let index_oid = index_relation.oid();
        let mut directory = LinkedItemList::<DirectoryEntry>::open(index_oid, DIRECTORY_START);
        directory
            .garbage_collect()
            .expect("garbage collection should succeed");
        let mut segment_metas =
            LinkedItemList::<SegmentMetaEntry>::open(index_oid, SEGMENT_METAS_START);
        segment_metas
            .garbage_collect()
            .expect("garbage collection should succeed");
        let mut delete_metas =
            LinkedItemList::<DeleteMetaEntry>::open(index_oid, DELETE_METAS_START);
        delete_metas
            .garbage_collect()
            .expect("garbage collection should succeed");

        // Return all recyclable pages to the free space map
        let nblocks =
            pg_sys::RelationGetNumberOfBlocksInFork(info.index, pg_sys::ForkNumber::MAIN_FORKNUM);
        let cache = BM25BufferCache::open(index_oid);
        let heap_oid = pg_sys::IndexGetRelation(index_oid, false);
        let heap_relation = pg_sys::RelationIdGetRelation(heap_oid);

        for blockno in 0..nblocks {
            let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
            let page = pg_sys::BufferGetPage(buffer);
            if page.recyclable(heap_relation) {
                cache.record_free_index_page(blockno);
            }
            pg_sys::UnlockReleaseBuffer(buffer);
        }
        pg_sys::RelationClose(heap_relation);
        pg_sys::IndexFreeSpaceMapVacuum(info.index);
    }

    // TODO: Mark XIDs as frozen
    // TODO: Update stats
    stats
}
