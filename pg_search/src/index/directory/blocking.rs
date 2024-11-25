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
use std::sync::Arc;
use std::{io, result};
use tantivy::directory::{DirectoryLock, FileHandle, Lock, WatchCallback, WatchHandle, WritePtr};
use tantivy::Directory;
use tantivy::{
    directory::error::{DeleteError, LockError, OpenReadError, OpenWriteError},
    directory::{INDEX_WRITER_LOCK, MANAGED_LOCK, META_LOCK},
    error::TantivyError,
};

use super::lock::BlockingLock;
use crate::index::reader::segment_component::SegmentComponentReader;
use crate::index::writer::segment_component::SegmentComponentWriter;
use crate::postgres::storage::block::{
    bm25_max_free_space, bm25_max_item_size, bm25_metadata, BM25PageSpecialData, BlockNumberList,
    DirectoryEntry, INDEX_WRITER_LOCK_BLOCKNO, MANAGED_LOCK_BLOCKNO, META_LOCK_BLOCKNO,
    TANTIVY_META_BLOCKNO,
};
use crate::postgres::storage::linked_list::{LinkedBytesList, LinkedItemList};
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
        let (entry, _, _) = unsafe { self.directory_lookup(path)? };

        if unsafe {
            pg_sys::TransactionIdDidCommit(entry.xid) || pg_sys::TransactionIdDidAbort(entry.xid)
        } {
            let block_list = LinkedBytesList::open(self.relation_oid, entry.start);
            let BlockNumberList(blocks) = unsafe { block_list.read_all().into() };
            let cache = unsafe { BM25BufferCache::open(self.relation_oid) };

            // Mark pages as deleted, but don't actually free them
            // It's important that only VACUUM frees pages, because pages might still be used by other transactions
            for blockno in blocks {
                unsafe {
                    let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
                    let page = pg_sys::BufferGetPage(buffer);
                    page.mark_deleted();

                    pg_sys::MarkBufferDirty(buffer);
                    pg_sys::UnlockReleaseBuffer(buffer);
                }
            }

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
        let directory =
            LinkedItemList::<DirectoryEntry>::open(self.relation_oid, metadata.directory_start);
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

    fn atomic_write(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        // Atomic write should only ever be used for writing meta.json
        if path.to_path_buf() != *META_FILEPATH {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("atomic_write unexpected path: {:?}", path),
            ));
        }

        unsafe {
            const ITEM_SIZE: usize = unsafe { bm25_max_item_size() };
            let cache = BM25BufferCache::open(self.relation_oid);
            let mut buffer =
                cache.get_buffer(TANTIVY_META_BLOCKNO, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
            let mut page = pg_sys::BufferGetPage(buffer);

            for (i, chunk) in data.chunks(ITEM_SIZE).enumerate() {
                if i > 0 {
                    let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
                    if (*special).next_blockno == pg_sys::InvalidBlockNumber {
                        let new_buffer = cache.new_buffer();
                        (*special).next_blockno = pg_sys::BufferGetBlockNumber(new_buffer);
                        pg_sys::MarkBufferDirty(buffer);
                        pg_sys::UnlockReleaseBuffer(buffer);
                        buffer = new_buffer;
                        page = pg_sys::BufferGetPage(buffer);
                    } else {
                        let next_blockno = (*special).next_blockno;
                        pg_sys::MarkBufferDirty(buffer);
                        pg_sys::UnlockReleaseBuffer(buffer);
                        buffer =
                            cache.get_buffer(next_blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
                        page = pg_sys::BufferGetPage(buffer);
                    }
                }

                if pg_sys::PageGetMaxOffsetNumber(page) == pg_sys::InvalidOffsetNumber {
                    pg_sys::PageAddItemExtended(
                        page,
                        chunk.as_ptr() as pg_sys::Item,
                        chunk.len(),
                        pg_sys::FirstOffsetNumber,
                        0,
                    );
                } else {
                    let overwrite = pg_sys::PageIndexTupleOverwrite(
                        page,
                        pg_sys::FirstOffsetNumber,
                        chunk.as_ptr() as pg_sys::Item,
                        chunk.len(),
                    );
                    assert!(overwrite);
                }
            }

            let last_blockno = pg_sys::BufferGetBlockNumber(buffer);
            pg_sys::MarkBufferDirty(buffer);
            pg_sys::UnlockReleaseBuffer(buffer);

            // Update the last blockno in the metadata page
            let buffer =
                cache.get_buffer(TANTIVY_META_BLOCKNO, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
            let page = pg_sys::BufferGetPage(buffer);
            let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
            (*special).last_blockno = last_blockno;
            pg_sys::MarkBufferDirty(buffer);
            pg_sys::UnlockReleaseBuffer(buffer);
        }

        Ok(())
    }

    fn atomic_read(&self, path: &Path) -> result::Result<Vec<u8>, OpenReadError> {
        // Atomic read should only ever be used for reading .meta.json
        if path.to_path_buf() != *META_FILEPATH {
            return Err(OpenReadError::FileDoesNotExist(PathBuf::from(path)));
        }

        let bytes = unsafe {
            let cache = BM25BufferCache::open(self.relation_oid);
            let buffer = cache.get_buffer(TANTIVY_META_BLOCKNO, Some(pg_sys::BUFFER_LOCK_SHARE));
            let page = pg_sys::BufferGetPage(buffer);
            let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
            let last_blockno = (*special).last_blockno;

            pg_sys::UnlockReleaseBuffer(buffer);
            let mut bytes = Vec::new();
            let mut current_blockno = TANTIVY_META_BLOCKNO;

            loop {
                let buffer = cache.get_buffer(current_blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
                let page = pg_sys::BufferGetPage(buffer);
                let item_id = pg_sys::PageGetItemId(page, pg_sys::FirstOffsetNumber);
                let item = pg_sys::PageGetItem(page, item_id);
                let len = (*item_id).lp_len() as usize;

                bytes.extend(std::slice::from_raw_parts(item as *const u8, len));

                let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
                let next_blockno = (*special).next_blockno;

                if current_blockno == last_blockno || next_blockno == pg_sys::InvalidBlockNumber {
                    pg_sys::UnlockReleaseBuffer(buffer);
                    break;
                } else {
                    current_blockno = next_blockno;
                    pg_sys::UnlockReleaseBuffer(buffer);
                }
            }

            bytes
        };

        if bytes.is_empty() {
            return Err(OpenReadError::FileDoesNotExist(PathBuf::from(path)));
        }

        Ok(bytes)
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
            let segment_components =
                LinkedItemList::<DirectoryEntry>::open(self.relation_oid, metadata.directory_start);

            Ok(segment_components
                .list_all_items()
                .map_err(|err| TantivyError::InternalError(err.to_string()))?
                .into_iter()
                .map(|opaque| opaque.path)
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
