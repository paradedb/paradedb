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

use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use pgrx::pg_sys;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::slice::from_raw_parts;
use std::sync::Arc;
use std::{io, result};
use tantivy::directory::{DirectoryLock, FileHandle, Lock, WatchCallback, WatchHandle, WritePtr};
use tantivy::{
    directory::error::{DeleteError, LockError, OpenReadError, OpenWriteError},
    directory::{INDEX_WRITER_LOCK, MANAGED_LOCK, META_LOCK},
    error::TantivyError,
};
use tantivy::{Directory, IndexMeta, index::SegmentMetaInventory};

use super::lock::BlockingLock;
use crate::index::reader::segment_component::SegmentComponentReader;
use crate::index::writer::segment_component::SegmentComponentWriter;
use crate::postgres::storage::block::{
    bm25_max_free_space, bm25_max_item_size, bm25_metadata, BM25PageSpecialData, BlockNumberList,
    DirectoryEntry, SegmentMetaEntry, INDEX_WRITER_LOCK_BLOCKNO, MANAGED_LOCK_BLOCKNO,
    META_LOCK_BLOCKNO, TANTIVY_META_BLOCKNO,
};
use crate::postgres::storage::linked_list::{LinkedBytesList, LinkedItemList, PgItem};
use crate::postgres::storage::utils::{BM25BufferCache, BM25Page};

/// Defined by Tantivy in core/mod.rs
pub static META_FILEPATH: Lazy<&'static Path> = Lazy::new(|| Path::new("meta.json"));

/// Tantivy Directory trait implementation over block storage
#[derive(Clone, Debug)]
pub struct BlockingDirectory {
    relation_oid: pg_sys::Oid,
}

impl BlockingDirectory {
    pub fn new(relation_oid: pg_sys::Oid) -> Self {
        Self { relation_oid }
    }

    pub unsafe fn acquire_blocking_lock(&self, lock: &Lock) -> Result<BlockingLock> {
        let blockno = if lock.filepath == META_LOCK.filepath {
            META_LOCK_BLOCKNO
        } else if lock.filepath == MANAGED_LOCK.filepath {
            MANAGED_LOCK_BLOCKNO
        } else if lock.filepath == INDEX_WRITER_LOCK.filepath {
            INDEX_WRITER_LOCK_BLOCKNO
        } else {
            bail!("acquire_lock unexpected lock {:?}", lock)
        };

        Ok(BlockingLock::new(self.relation_oid, blockno))
    }

    /// ambulkdelete wants to know how many pages were deleted, but the Directory trait doesn't let delete
    /// return a value, so we implement our own delete method
    pub fn try_delete(&self, path: &Path) -> Result<Option<DirectoryEntry>> {
        let (entry, blockno, offsetno) = unsafe { self.directory_lookup(path)? };

        if unsafe {
            pg_sys::TransactionIdDidCommit(entry.xmin) || pg_sys::TransactionIdDidAbort(entry.xmin)
        } {
            unsafe {
                let entry_with_xmax = DirectoryEntry {
                    xmax: pg_sys::GetCurrentTransactionId(),
                    ..entry.clone()
                };
                let cache = BM25BufferCache::open(self.relation_oid);
                let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
                let page = pg_sys::BufferGetPage(buffer);
                let PgItem(item, size) = entry_with_xmax.clone().into();
                let overwrite = pg_sys::PageIndexTupleOverwrite(page, offsetno, item, size);
                assert!(overwrite, "setting xmax for {:?} should succeed", path);

                pg_sys::MarkBufferDirty(buffer);
                pg_sys::UnlockReleaseBuffer(buffer);
            }

            // TODO: Move this over to linked_list
            // let block_list = LinkedBytesList::open_with_lock(self.relation_oid, entry.start, Some(pg_sys::BUFFER_LOCK_SHARE));
            // let BlockNumberList(blocks) = unsafe { block_list.read_all().into() };
            // let cache = unsafe { BM25BufferCache::open(self.relation_oid) };

            // // Mark pages as deleted, but don't actually free them
            // // It's important that only VACUUM frees pages, because pages might still be used by other transactions
            // for blockno in blocks {
            //     unsafe {
            //         let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
            //         let page = pg_sys::BufferGetPage(buffer);
            //         page.mark_deleted();

            //         pg_sys::MarkBufferDirty(buffer);
            //         pg_sys::UnlockReleaseBuffer(buffer);
            //     }
            // }

            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    pub unsafe fn directory_lookup(
        &self,
        path: &Path,
    ) -> Result<(DirectoryEntry, pg_sys::BlockNumber, pg_sys::OffsetNumber)> {
        let metadata = bm25_metadata(self.relation_oid);
        let directory = LinkedItemList::<DirectoryEntry>::open_with_lock(
            self.relation_oid,
            metadata.directory_start,
            Some(pg_sys::BUFFER_LOCK_SHARE),
        );
        let result = directory.lookup(path, |opaque, path| opaque.path == *path)?;
        Ok(result)
    }
}

impl Directory for BlockingDirectory {
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        let (opaque, _, _) = unsafe {
            self.directory_lookup(path)
                .map_err(|err| OpenReadError::IoError {
                    io_error: io::Error::new(io::ErrorKind::Other, err.to_string()).into(),
                    filepath: PathBuf::from(path),
                })?
        };

        Ok(Arc::new(unsafe {
            SegmentComponentReader::new(self.relation_oid, opaque)
        }))
    }

    fn open_write(&self, path: &Path) -> result::Result<WritePtr, OpenWriteError> {
        let result = unsafe { SegmentComponentWriter::new(self.relation_oid, path) };
        Ok(io::BufWriter::with_capacity(
            unsafe { bm25_max_free_space() },
            Box::new(result),
        ))
    }

    /// atomic_write is used by Tantivy to write to managed.json, meta.json, and create .lock files
    /// This function should never be called by our Tantivy fork because we write to managed.json and meta.json ourselves
    fn atomic_write(&self, path: &Path, _data: &[u8]) -> io::Result<()> {
        unimplemented!("atomic_write should not be called for {:?}", path);
    }

    /// atomic_read is used by Tantivy to read from managed.json and meta.json
    /// This function should never be called by our Tantivy fork because we read from them ourselves
    fn atomic_read(&self, path: &Path) -> result::Result<Vec<u8>, OpenReadError> {
        unimplemented!("atomic_read should not be called for {:?}", path);
    }

    fn delete(&self, _path: &Path) -> result::Result<(), DeleteError> {
        unimplemented!("BlockingDirectory should not call delete");
    }

    // Internally, Tantivy only uses this for meta.json, which should always exist
    fn exists(&self, _path: &Path) -> Result<bool, OpenReadError> {
        Ok(true)
    }

    fn acquire_lock(&self, lock: &Lock) -> result::Result<DirectoryLock, LockError> {
        let blocking_lock = unsafe {
            self.acquire_blocking_lock(lock)
                .expect("acquire blocking lock should succeed")
        };
        Ok(DirectoryLock::from(Box::new(blocking_lock)))
    }

    // Internally, tantivy only uses this API to detect new commits to implement the
    // `OnCommitWithDelay` `ReloadPolicy`. Not implementing watch in a `Directory` only prevents
    // the `OnCommitWithDelay` `ReloadPolicy` to work properly.
    fn watch(&self, _watch_callback: WatchCallback) -> tantivy::Result<WatchHandle> {
        unimplemented!("OnCommitWithDelay ReloadPolicy not supported");
    }

    /// Postgres block storage handles flushing to disk for us
    /// We do not need to and should not implement this ourselves
    fn sync_directory(&self) -> io::Result<()> {
        Ok(())
    }

    fn list_managed_files(&self) -> tantivy::Result<HashSet<PathBuf>> {
        unsafe {
            let metadata = bm25_metadata(self.relation_oid);
            let segment_components = LinkedItemList::<DirectoryEntry>::open_with_lock(
                self.relation_oid,
                metadata.directory_start,
                Some(pg_sys::BUFFER_LOCK_SHARE),
            );

            Ok(segment_components
                .list_all_items()
                .map_err(|err| TantivyError::InternalError(err.to_string()))?
                .into_iter()
                .map(|(entry, _, _)| entry.path)
                .collect())
        }
    }

    // This is intentionally a no-op
    // This function is called by Tantivy in two places: during garbage collection and when a new segment is created
    // In the garbage collection case, we want to handle this ourselves because we need to do transaction visibility checks
    // In the new segment case, we want to handle this ourselves because we also store the segment's byte length and block numbers alongside the path
    fn register_files_as_managed(
        &self,
        _files: Vec<PathBuf>,
        _overwrite: bool,
    ) -> tantivy::Result<()> {
        Ok(())
    }

    fn save_meta(&self, meta: &IndexMeta) -> tantivy::Result<()> {
        let cache = unsafe { BM25BufferCache::open(self.relation_oid) };
        let opstamp = meta.opstamp;
        let current_xid = unsafe { pg_sys::GetCurrentTransactionId() };

        let mut new_segments = meta.segments.clone();
        let new_segment_ids = new_segments
            .iter()
            .map(|segment| segment.id())
            .collect::<Vec<_>>();

        let mut segment_metas = LinkedItemList::<SegmentMetaEntry>::open_with_lock(
            self.relation_oid,
            unsafe { bm25_metadata(self.relation_oid).segment_metas_start },
            Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
        );

        unsafe {
            for (entry, blockno, offsetno) in segment_metas.list_all_items().unwrap() {
                if let Some(index) = new_segments
                    .iter()
                    .position(|segment| segment.id() == entry.meta.segment_id)
                {
                    new_segments.remove(index);
                } else {
                    let entry_with_xmax = SegmentMetaEntry {
                        xmax: current_xid,
                        ..entry.clone()
                    };

                    let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
                    let page = pg_sys::BufferGetPage(buffer);
                    let PgItem(item, size) = entry_with_xmax.clone().into();
                    let overwrite = pg_sys::PageIndexTupleOverwrite(page, offsetno, item, size);
                    assert!(
                        overwrite,
                        "setting xmax for {:?} should succeed",
                        entry.meta.segment_id
                    );

                    pg_sys::MarkBufferDirty(buffer);
                    pg_sys::UnlockReleaseBuffer(buffer);
                }
            }
        }

        let entries = new_segments
            .iter()
            .map(|segment| SegmentMetaEntry {
                meta: segment.tracked.as_ref().clone(),
                opstamp,
                xmin: current_xid,
                xmax: pg_sys::InvalidTransactionId,
            })
            .collect::<Vec<_>>();

        unsafe {
            segment_metas
                .write(entries)
                .expect("save new metas should succeed");
        }

        Ok(())
    }

    fn load_metas(&self, inventory: &SegmentMetaInventory) -> tantivy::Result<IndexMeta> {
        // let mut segment_metas = LinkedItemList::<SegmentMetaEntry>::open_with_lock(
        //     self.relation_oid,
        //     unsafe { bm25_metadata(self.relation_oid).segment_metas_start },
        //     Some(pg_sys::BUFFER_LOCK_SHARE),
        // );

        // let mut alive_segments = vec![];
        // let mut opstamp = 0;
        // let mut max_xmin = 0;
        // let snapshot = unsafe { pg_sys::GetTransactionSnapshot() };

        // for (entry, _, _) in segment_metas {
        //     let segment_meta = entry.meta.clone();
        //     let in_progress: &[u32] = unsafe { from_raw_parts(snapshot.xip, snapshot.xcnt) };
        //     let invisible = segment_meta.xmin >= snapshot.xmin
        //         || segment_meta.xmax >= snapshot.xmax
        //         || in_progress.contains(&snapshot.xmin)
        //         || !pg_sys::TransactionIdDidCommit(segment_meta.xmin);

        //     if !invisible {
        //         alive_segments.push(segment_meta);
        //         opstamp = opstamp.max(entry.opstamp);
        //         max_xmin = max_xmin.max(entry.xmin);
        //     }
        // }

        todo!()
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;

    #[pg_test]
    fn test_list_managed_files() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let directory = BlockingDirectory { relation_oid };
        let listed_files = directory.list_managed_files().unwrap();
        assert_eq!(listed_files.len(), 6);
    }
}
