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
use crate::postgres::storage::block::{DirectoryEntry, MetaPageData, METADATA_BLOCKNO};
use crate::postgres::storage::linked_list::LinkedItemList;
use crate::postgres::storage::utils::{BM25BufferCache, BM25Page};
use anyhow::Result;
use pgrx::*;
use std::path::PathBuf;
use tantivy::directory::{Lock, MANAGED_LOCK};
use tantivy::index::Index;
use tantivy::{Directory, IndexWriter};

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

            // Commit does garbage collect as well, no need to explicitly call it
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

        // Vacuum the linked list of segment components
        let cache = unsafe { BM25BufferCache::open(index_oid) };
        let heap_oid = unsafe { pg_sys::IndexGetRelation(index_oid, false) };
        let heap_relation = unsafe { pg_sys::RelationIdGetRelation(heap_oid) };

        unsafe {
            vacuum_directory(index_oid, blocking_stats.deleted_paths)
                .expect("vacuum segment components should succeed");
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

        // TODO: Update stats
        stats
    })
}

fn alive_segment_components(
    segment_components_list: &LinkedItemList<DirectoryEntry>,
    paths_to_delete: Vec<PathBuf>,
) -> Result<Vec<DirectoryEntry>> {
    unsafe {
        Ok(segment_components_list
            .list_all_items()?
            .into_iter()
            .map(|(entry, _, _)| entry)
            .filter(|entry| !paths_to_delete.contains(&entry.path))
            .collect::<Vec<_>>())
    }
}

unsafe fn vacuum_directory(relation_oid: pg_sys::Oid, paths_deleted: Vec<PathBuf>) -> Result<()> {
    // let directory = BlockingDirectory::new(relation_oid);
    // let cache = BM25BufferCache::open(relation_oid);
    // // This lock is necessary because we are reading the segment components list, appending, and then overwriting
    // // If another process were to insert a segment component while we are doing this, that component would be forever lost
    // let _lock = directory.acquire_lock(&Lock {
    //     filepath: MANAGED_LOCK.filepath.clone(),
    //     is_blocking: true,
    // });

    // let mut new_segment_components = LinkedItemList::<DirectoryEntry>::create(relation_oid);
    // let buffer = cache.get_buffer(METADATA_BLOCKNO, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
    // let page = pg_sys::BufferGetPage(buffer);
    // let metadata = pg_sys::PageGetContents(page) as *mut MetaPageData;
    // let start_blockno = (*metadata).directory_start;

    // let old_segment_components = LinkedItemList::<DirectoryEntry>::open_with_lock(
    //     relation_oid,
    //     start_blockno,
    //     Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
    // );
    // let alive_segment_components =
    //     alive_segment_components(&old_segment_components, paths_deleted.clone())?;

    // old_segment_components.delete();

    // (*metadata).directory_start = pg_sys::BufferGetBlockNumber(new_segment_components.lock_buffer);
    // new_segment_components.write(alive_segment_components)?;

    // pg_sys::MarkBufferDirty(buffer);
    // pg_sys::UnlockReleaseBuffer(buffer);

    Ok(())
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

    #[pg_test]
    unsafe fn test_alive_segment_components() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let metadata = bm25_metadata(relation_oid);
        let start_blockno = metadata.directory_start;
        let mut segment_components_list = LinkedItemList::<DirectoryEntry>::open_with_lock(
            relation_oid,
            start_blockno,
            Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
        );

        let paths = (0..3)
            .map(|_| PathBuf::from(format!("{:?}.term", Uuid::new_v4())))
            .collect::<Vec<_>>();

        let segments_to_vacuum = paths
            .iter()
            .map(|path| DirectoryEntry {
                path: path.to_path_buf(),
                start: 0,
                total_bytes: 100,
                xmin: pg_sys::GetCurrentTransactionId(),
                xmax: pg_sys::InvalidTransactionId,
            })
            .collect::<Vec<_>>();

        segment_components_list
            .write(segments_to_vacuum.clone())
            .unwrap();

        let dead_paths = vec![paths[0].clone(), paths[2].clone()];
        let alive_segments =
            alive_segment_components(&segment_components_list, dead_paths).unwrap();
        assert!(!alive_segments.contains(&segments_to_vacuum[0]));
        assert!(!alive_segments.contains(&segments_to_vacuum[2]));
    }

    #[pg_test]
    unsafe fn test_vacuum_directory() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let old_start_blockno = bm25_metadata(relation_oid).directory_start;
        let mut old_directory = LinkedItemList::<DirectoryEntry>::open_with_lock(
            relation_oid,
            old_start_blockno,
            Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
        );

        let paths = (0..3)
            .map(|_| PathBuf::from(format!("{:?}.term", Uuid::new_v4())))
            .collect::<Vec<_>>();

        let segments_to_vacuum = paths
            .iter()
            .map(|path| DirectoryEntry {
                path: path.to_path_buf(),
                start: 0,
                total_bytes: 100,
                xmin: pg_sys::GetCurrentTransactionId(),
                xmax: pg_sys::InvalidTransactionId,
            })
            .collect::<Vec<_>>();

        old_directory.write(segments_to_vacuum.clone()).unwrap();

        // Test that old directory contains dead entries
        let alive_segments = old_directory
            .list_all_items()
            .unwrap()
            .into_iter()
            .map(|(entry, _, _)| entry)
            .collect::<Vec<_>>();
        assert!(alive_segments.contains(&segments_to_vacuum[0]));
        assert!(alive_segments.contains(&segments_to_vacuum[2]));

        // Perform vacuum
        let dead_paths = vec![paths[0].clone(), paths[2].clone()];
        vacuum_directory(relation_oid, dead_paths).unwrap();

        let cache = BM25BufferCache::open(relation_oid);
        let mut blockno = old_start_blockno;

        // Test that entries were marked as dead after vacuum
        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
            let page = pg_sys::BufferGetPage(buffer);
            let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
            assert_ne!(
                (*special).delete_xid.value,
                pg_sys::FullTransactionId::default().value
            );
            blockno = (*special).next_blockno;
            pg_sys::UnlockReleaseBuffer(buffer);
        }

        // Test that a new directory was created
        let new_start_blockno = bm25_metadata(relation_oid).directory_start;
        assert_ne!(old_start_blockno, new_start_blockno);

        // Test that new directory does not contain dead entries
        let new_directory = LinkedItemList::<DirectoryEntry>::open_with_lock(
            relation_oid,
            new_start_blockno,
            Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
        );
        let alive_segments = new_directory
            .list_all_items()
            .unwrap()
            .into_iter()
            .map(|(entry, _, _)| entry)
            .collect::<Vec<_>>();
        assert!(!alive_segments.contains(&segments_to_vacuum[0]));
        assert!(!alive_segments.contains(&segments_to_vacuum[2]));

        // Test that old entries were not physically deleted
        let (entry, _, _) = old_directory
            .lookup(paths[0].clone(), |entry, path| entry.path == *path)
            .unwrap();
        assert_eq!(entry.path, paths[0]);

        let (entry, _, _) = old_directory
            .lookup(paths[2].clone(), |entry, path| entry.path == *path)
            .unwrap();
        assert_eq!(entry.path, paths[2]);
    }
}
