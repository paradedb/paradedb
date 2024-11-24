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

use crate::index::blocking::{BlockingDirectory, SEGMENT_COMPONENT_CACHE};
use crate::index::channel::{
    ChannelDirectory, ChannelRequest, ChannelRequestHandler, ChannelResponse,
};
use crate::index::WriterResources;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::postgres::storage::block::{
    bm25_metadata, MetaPageData, SegmentComponentOpaque, METADATA_BLOCKNO,
};
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

        // Update the cache
        for path in &blocking_stats.deleted_paths {
            SEGMENT_COMPONENT_CACHE.write().remove(path);
        }

        // Vacuum the linked list of segment components
        let cache = unsafe { BM25BufferCache::open(index_oid) };
        let heap_oid = unsafe { pg_sys::IndexGetRelation(index_oid, false) };
        let heap_relation = unsafe { pg_sys::RelationIdGetRelation(heap_oid) };

        unsafe {
            vacuum_segment_components(index_oid, blocking_stats.deleted_paths)
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
    segment_components_list: &LinkedItemList<SegmentComponentOpaque>,
    paths_to_delete: Vec<PathBuf>,
) -> Result<Vec<SegmentComponentOpaque>> {
    unsafe {
        Ok(segment_components_list
            .list_all_items()?
            .into_iter()
            .filter(|opaque| {
                println!(
                    "target {:?} paths to delete {:?} should delete {}",
                    opaque.path,
                    paths_to_delete,
                    !paths_to_delete.contains(&opaque.path)
                );
                !paths_to_delete.contains(&opaque.path)
            })
            .collect::<Vec<_>>())
    }
}

unsafe fn vacuum_segment_components(
    relation_oid: pg_sys::Oid,
    paths_deleted: Vec<PathBuf>,
) -> Result<()> {
    let directory = BlockingDirectory::new(relation_oid);
    let cache = BM25BufferCache::open(relation_oid);
    // This lock is necessary because we are reading the segment components list, appending, and then overwriting
    // If another process were to insert a segment component while we are doing this, that component would be forever lost
    let _lock = directory.acquire_lock(&Lock {
        filepath: MANAGED_LOCK.filepath.clone(),
        is_blocking: true,
    });

    let metadata = bm25_metadata(relation_oid);
    let start_blockno = metadata.directory_start;

    let mut old_segment_components =
        unsafe { LinkedItemList::<SegmentComponentOpaque>::open(relation_oid, start_blockno) };
    let alive_segment_components =
        alive_segment_components(&old_segment_components, paths_deleted.clone())?;

    old_segment_components.delete();

    let mut new_segment_components = LinkedItemList::<SegmentComponentOpaque>::create(relation_oid);
    // TODO: Change metadata segment components first blockno
    new_segment_components.write(alive_segment_components)
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    use crate::postgres::storage::block::SegmentComponentOpaque;
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
        let mut segment_components_list =
            unsafe { LinkedItemList::<SegmentComponentOpaque>::open(relation_oid, start_blockno) };

        let paths = (0..3)
            .map(|_| PathBuf::from(format!("{:?}.term", Uuid::new_v4())))
            .collect::<Vec<_>>();

        let segments_to_vacuum = paths
            .iter()
            .map(|path| SegmentComponentOpaque {
                path: path.to_path_buf(),
                start: 0,
                total_bytes: 100,
                xid: 0,
            })
            .collect::<Vec<_>>();

        segment_components_list
            .write(segments_to_vacuum.clone())
            .unwrap();

        let dead_paths = vec![paths[0].clone(), paths[2].clone()];
        let alive_segments =
            alive_segment_components(&segment_components_list, dead_paths).unwrap();
        assert_eq!(alive_segments, vec![segments_to_vacuum[1].clone()]);
    }
}
