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
    bm25_metadata, DirectoryEntry, MVCCEntry, MetaPageData, PgItem, SegmentMetaEntry,
    METADATA_BLOCKNO,
};
use crate::postgres::storage::linked_list::LinkedItemList;
use crate::postgres::storage::utils::{BM25BufferCache, BM25Page};
use anyhow::Result;
use pgrx::*;
use std::fmt::Debug;
use std::path::PathBuf;
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

        let blocking_stats = handler
            .receive_blocking(Some(|_| false))
            .expect("blocking handler should succeed");

        // Vacuum all linked lists
        // If a new LinkedItemList is created, it should be vacuumed here
        let cache = unsafe { BM25BufferCache::open(index_oid) };
        let heap_oid = unsafe { pg_sys::IndexGetRelation(index_oid, false) };
        let heap_relation = unsafe { pg_sys::RelationIdGetRelation(heap_oid) };

        // Hold an exclusive lock on the metadata page since we're changing the addresses of the
        // linked lists
        unsafe {
            let metadata_buffer =
                cache.get_buffer(METADATA_BLOCKNO, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
            let metadata_page = pg_sys::BufferGetPage(metadata_buffer);
            let metadata = pg_sys::PageGetContents(metadata_page) as *mut MetaPageData;

            let new_directory_start = vacuum_entries::<DirectoryEntry>(
                index_oid,
                heap_relation,
                (*metadata).directory_start,
            )
            .expect("vacuum entries should succeed");

            let new_segment_metas_start = vacuum_entries::<SegmentMetaEntry>(
                index_oid,
                heap_relation,
                (*metadata).segment_metas_start,
            )
            .expect("vacuum entries should succeed");

            (*metadata).directory_start = new_directory_start;
            (*metadata).segment_metas_start = new_segment_metas_start;
            pg_sys::MarkBufferDirty(metadata_buffer);
            pg_sys::UnlockReleaseBuffer(metadata_buffer);
        }

        // Return all recyclable pages to the free space map
        let nblocks = unsafe {
            pg_sys::RelationGetNumberOfBlocksInFork(info.index, pg_sys::ForkNumber::MAIN_FORKNUM)
        };

        for blockno in 0..nblocks {
            unsafe {
                let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
                let page = pg_sys::BufferGetPage(buffer);
                if page.recyclable(heap_relation) {
                    cache.record_free_index_page(blockno);
                }
                pg_sys::UnlockReleaseBuffer(buffer);
            }
        }

        unsafe {
            pg_sys::RelationClose(heap_relation);
            pg_sys::IndexFreeSpaceMapVacuum(info.index)
        };
    });
    // TODO: Update stats
    stats
}

unsafe fn vacuum_entries<T>(
    index_oid: pg_sys::Oid,
    heap_relation: pg_sys::Relation,
    start: pg_sys::BlockNumber,
) -> Result<pg_sys::BlockNumber>
where
    T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry,
{
    let old_list =
        LinkedItemList::<T>::open_with_lock(index_oid, start, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));

    let mut entries_to_keep = vec![];
    for (entry, _, _) in old_list.list_all_items()? {
        let xmax = entry.xmax();
        if xmax == pg_sys::InvalidTransactionId
            || !pg_sys::GlobalVisCheckRemovableXid(heap_relation, entry.xmax())
        {
            entries_to_keep.push(entry);
        } else {
            crate::log_message(&format!("-- Vacuuming entry {:?}", entry));
        }
    }

    let mut new_list = LinkedItemList::<T>::create(index_oid);
    new_list.write(entries_to_keep)?;
    Ok(pg_sys::BufferGetBlockNumber(new_list.lock_buffer))
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    use crate::postgres::storage::block::DirectoryEntry;
    use crate::postgres::storage::block::{bm25_metadata, BM25PageSpecialData};
    use crate::postgres::storage::linked_list::LinkedItemList;
}
