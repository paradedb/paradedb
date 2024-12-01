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

use crate::index::blocking::BlockingDirectory;
use crate::index::channel::{
    ChannelDirectory, ChannelRequest, ChannelRequestHandler, ChannelResponse,
};
use crate::index::WriterResources;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::postgres::storage::block::{
    DirectoryEntry, SegmentMetaEntry, DIRECTORY_START, SEGMENT_METAS_START,
};
use crate::postgres::storage::utils::{BM25BufferCache, BM25Page};
use crate::postgres::storage::LinkedItemList;
use pgrx::*;
use tantivy::index::Index;
use tantivy::IndexWriter;

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
    let index_oid = index_relation.oid();
    let options = index_relation.rd_options as *mut SearchIndexCreateOptions;
    let (parallelism, memory_budget, _, _) =
        WriterResources::Vacuum.resources(unsafe { options.as_ref().unwrap() });

    let (request_sender, request_receiver) = crossbeam::channel::unbounded::<ChannelRequest>();
    let (response_sender, response_receiver) = crossbeam::channel::unbounded::<ChannelResponse>();
    let (request_sender_clone, response_receiver_clone) =
        (request_sender.clone(), response_receiver.clone());

    // Let Tantivy merge and garbage collect segments
    std::thread::scope(|s| {
        s.spawn(|| {
            let channel_directory = ChannelDirectory::new(
                request_sender_clone.clone(),
                response_receiver_clone.clone(),
            );
            let channel_index = Index::open(channel_directory).expect("channel index should open");
            let mut writer: IndexWriter = channel_index
                .writer_with_num_threads(parallelism.into(), memory_budget)
                .unwrap();

            writer.commit().unwrap();
            writer.wait_merging_threads().unwrap();
            request_sender_clone
                .send(ChannelRequest::Terminate)
                .unwrap();
        });

        let blocking_directory = BlockingDirectory::new(index_oid);
        let mut handler = ChannelRequestHandler::open(
            blocking_directory,
            index_oid,
            response_sender.clone(),
            request_receiver.clone(),
        );

        handler
            .receive_blocking(Some(|_| false))
            .expect("blocking handler should succeed");

        // Garbage collect linked lists
        // If new LinkedItemLists are created they should be garbage collected here
        unsafe {
            let mut directory = LinkedItemList::<DirectoryEntry>::open(index_oid, DIRECTORY_START);
            directory
                .garbage_collect()
                .expect("garbage collection should succeed");
        }
        unsafe {
            let mut segment_metas =
                LinkedItemList::<SegmentMetaEntry>::open(index_oid, SEGMENT_METAS_START);
            segment_metas
                .garbage_collect()
                .expect("garbage collection should succeed");
        }

        // Return all recyclable pages to the free space map
        unsafe {
            let nblocks = pg_sys::RelationGetNumberOfBlocksInFork(
                info.index,
                pg_sys::ForkNumber::MAIN_FORKNUM,
            );
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
            pg_sys::IndexFreeSpaceMapVacuum(info.index);
        }
    });
    // TODO: Update stats
    stats
}
