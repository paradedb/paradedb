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

use anyhow::Result;
use parking_lot::Mutex;
use pgrx::pg_sys;
use rustc_hash::FxHashMap;
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::{io, result};
use tantivy::directory::error::{DeleteError, LockError, OpenReadError, OpenWriteError};
use tantivy::directory::{DirectoryLock, FileHandle, Lock, WatchCallback, WatchHandle, WritePtr};
use tantivy::{index::SegmentMetaInventory, Directory, IndexMeta};

use super::utils::{SegmentComponentId, SegmentComponentPath};
use crate::index::reader::segment_component::SegmentComponentReader;
use crate::index::writer::segment_component::SegmentComponentWriter;
use crate::postgres::storage::block::{
    bm25_max_free_space, DirectoryEntry, LinkedList, MVCCEntry, PgItem, SegmentMetaEntry,
    DIRECTORY_START, SCHEMA_START, SEGMENT_METAS_START, SETTINGS_START,
};
use crate::postgres::storage::utils::{BM25Buffer, BM25BufferCache};
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};

/// Tantivy Directory trait implementation over block storage
#[derive(Clone, Debug)]
pub struct BlockingDirectory {
    relation_oid: pg_sys::Oid,

    // keep a cache of readers behind an Arc<Mutex<_>> so that if/when this BlockingDirectory is
    // cloned, we don't lose all the work we did originally creating the FileHandler impls.  And
    // it's cloned a lot!
    readers: Arc<Mutex<FxHashMap<PathBuf, Arc<dyn FileHandle>>>>,
}

impl BlockingDirectory {
    pub fn new(relation_oid: pg_sys::Oid) -> Self {
        Self {
            relation_oid,
            readers: Arc::new(Mutex::new(FxHashMap::default())),
        }
    }

    pub unsafe fn directory_lookup(
        &self,
        path: &Path,
    ) -> Result<(DirectoryEntry, pg_sys::BlockNumber, pg_sys::OffsetNumber)> {
        let directory = LinkedItemList::<DirectoryEntry>::open(self.relation_oid, DIRECTORY_START);
        let result = directory.lookup(path, |opaque, path| opaque.path == *path)?;
        Ok(result)
    }
}

impl Directory for BlockingDirectory {
    /// Returns a segment reader that implements std::io::Read
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        match self.readers.lock().entry(path.to_path_buf()) {
            Entry::Occupied(reader) => Ok(reader.get().clone()),
            Entry::Vacant(vacant) => {
                let (opaque, _, _) = unsafe {
                    self.directory_lookup(path)
                        .map_err(|err| OpenReadError::IoError {
                            io_error: io::Error::new(io::ErrorKind::Other, err.to_string()).into(),
                            filepath: PathBuf::from(path),
                        })?
                };
                Ok(vacant
                    .insert(Arc::new(unsafe {
                        SegmentComponentReader::new(self.relation_oid, opaque)
                    }))
                    .clone())
            }
        }
    }
    /// delete is called by Tantivy's garbage collection
    /// We handle this ourselves in amvacuumcleanup
    fn delete(&self, _path: &Path) -> result::Result<(), DeleteError> {
        Ok(())
    }

    // Internally, Tantivy only uses this for meta.json, which should always exist
    fn exists(&self, _path: &Path) -> Result<bool, OpenReadError> {
        Ok(true)
    }

    /// Returns a segment writer that implements std::io::Write
    fn open_write(&self, path: &Path) -> result::Result<WritePtr, OpenWriteError> {
        let result = unsafe { SegmentComponentWriter::new(self.relation_oid, path) };
        Ok(io::BufWriter::with_capacity(
            bm25_max_free_space(),
            Box::new(result),
        ))
    }

    /// atomic_read is used by Tantivy to read from managed.json and meta.json
    /// This function should never be called by our Tantivy fork because we read from them ourselves
    fn atomic_read(&self, path: &Path) -> result::Result<Vec<u8>, OpenReadError> {
        unimplemented!("atomic_read should not be called for {:?}", path);
    }

    /// atomic_write is used by Tantivy to write to managed.json, meta.json, and create .lock files
    /// This function should never be called by our Tantivy fork because we write to managed.json and meta.json ourselves
    fn atomic_write(&self, path: &Path, _data: &[u8]) -> io::Result<()> {
        unimplemented!("atomic_write should not be called for {:?}", path);
    }

    /// Postgres block storage handles flushing to disk for us
    /// We do not need to and should not implement this ourselves
    fn sync_directory(&self) -> io::Result<()> {
        Ok(())
    }

    // We have done the work to ensure that Tantivy locks are not needed, only Postgres locks
    // This is a no-op, returning a lock doesn't actually lock anything
    fn acquire_lock(&self, lock: &Lock) -> result::Result<DirectoryLock, LockError> {
        Ok(DirectoryLock::from(Box::new(Lock {
            filepath: lock.filepath.clone(),
            is_blocking: true,
        })))
    }

    // Tantivy only uses this API to detect new commits to implement the
    // `OnCommitWithDelay` `ReloadPolicy`. We do not want this reload policy in Postgres.
    fn watch(&self, _watch_callback: WatchCallback) -> tantivy::Result<WatchHandle> {
        unimplemented!("OnCommitWithDelay ReloadPolicy not supported");
    }

    /// Returns a list of all segment components to Tantivy,
    /// identified by <uuid>.<ext> PathBufs
    fn list_managed_files(&self) -> tantivy::Result<HashSet<PathBuf>> {
        unsafe {
            let cache = BM25BufferCache::open(self.relation_oid);
            let segment_components =
                LinkedItemList::<DirectoryEntry>::open(self.relation_oid, DIRECTORY_START);
            let mut blockno = segment_components.get_start_blockno();
            let mut files = HashSet::new();

            while blockno != pg_sys::InvalidBlockNumber {
                let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
                let page = pg_sys::BufferGetPage(buffer);
                let mut offsetno = pg_sys::FirstOffsetNumber;
                let max_offset = pg_sys::PageGetMaxOffsetNumber(page);

                while offsetno <= max_offset {
                    let item_id = pg_sys::PageGetItemId(page, offsetno);
                    let item = DirectoryEntry::from(PgItem(
                        pg_sys::PageGetItem(page, item_id),
                        (*item_id).lp_len() as pg_sys::Size,
                    ));
                    files.insert(item.path.clone());
                    offsetno += 1;
                }

                blockno = buffer.next_blockno();
                pg_sys::UnlockReleaseBuffer(buffer);
            }

            Ok(files)
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

    /// Saves a Tantivy IndexMeta to block storage
    fn save_metas(&self, meta: &IndexMeta) -> tantivy::Result<()> {
        let cache = unsafe { BM25BufferCache::open(self.relation_oid) };

        // Save Tantivy Schema if this is the first commit
        {
            let mut schema = LinkedBytesList::open(self.relation_oid, SCHEMA_START);

            if schema.is_empty() {
                let bytes =
                    serde_json::to_vec(&meta.schema).expect("expected to serialize valid Schema");
                unsafe { schema.write(&bytes).expect("write schema should succeed") };
            }
        }

        // Save Tantivy IndexSettings if this is the first commit
        {
            let mut settings = LinkedBytesList::open(self.relation_oid, SETTINGS_START);

            if settings.is_empty() {
                let bytes = serde_json::to_vec(&meta.index_settings)
                    .expect("expected to serialize valid IndexSettings");
                unsafe {
                    settings
                        .write(&bytes)
                        .expect("write settings should succeed")
                };
            }
        }

        // If there were no new segments, skip the rest of the work
        if meta.segments.is_empty() {
            return Ok(());
        }

        // Update SegmentMeta entries
        let opstamp = meta.opstamp;
        let current_xid = unsafe { pg_sys::GetCurrentTransactionId() };

        let mut new_segments = meta.segments.clone();
        let mut segment_metas =
            LinkedItemList::<SegmentMetaEntry>::open(self.relation_oid, SEGMENT_METAS_START);

        // Mark old SegmentMeta entries as deleted
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let mut deleted_segment_ids = vec![];
        unsafe {
            let mut blockno = segment_metas.get_start_blockno();
            while blockno != pg_sys::InvalidBlockNumber {
                let state = cache.start_xlog();
                let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
                let page = pg_sys::GenericXLogRegisterBuffer(state, buffer, 0);
                let max_offset = pg_sys::PageGetMaxOffsetNumber(page);
                let mut offsetno = pg_sys::FirstOffsetNumber;
                let mut needs_wal = false;

                while offsetno <= max_offset {
                    let item_id = pg_sys::PageGetItemId(page, offsetno);
                    let entry = SegmentMetaEntry::from(PgItem(
                        pg_sys::PageGetItem(page, item_id),
                        (*item_id).lp_len() as pg_sys::Size,
                    ));
                    if let Some(index) = new_segments
                        .iter()
                        .position(|segment| segment.id() == entry.meta.segment_id)
                    {
                        new_segments.remove(index);
                    } else if !entry.is_deleted() && entry.is_visible(snapshot) {
                        let entry_with_xmax = SegmentMetaEntry {
                            xmax: current_xid,
                            ..entry.clone()
                        };
                        let PgItem(item, size) = entry_with_xmax.clone().into();
                        let overwrite = pg_sys::PageIndexTupleOverwrite(page, offsetno, item, size);
                        assert!(
                            overwrite,
                            "setting xmax for {:?} should succeed",
                            entry.meta.segment_id
                        );
                        deleted_segment_ids.push(entry.meta.segment_id);
                        needs_wal = true;
                    }
                    offsetno += 1;
                }

                blockno = buffer.next_blockno();
                if needs_wal {
                    pg_sys::GenericXLogFinish(state);
                } else {
                    pg_sys::GenericXLogAbort(state);
                }
                pg_sys::UnlockReleaseBuffer(buffer);
            }
        }

        // Save new SegmentMeta entries
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
                .add_items(entries)
                .expect("save new metas should succeed");
        }

        // Mark old DirectoryEntry entries as deleted
        let directory = LinkedItemList::<DirectoryEntry>::open(self.relation_oid, DIRECTORY_START);
        unsafe {
            let mut blockno = directory.get_start_blockno();
            while blockno != pg_sys::InvalidBlockNumber {
                let state = cache.start_xlog();
                let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
                let page = pg_sys::GenericXLogRegisterBuffer(state, buffer, 0);
                let max_offset = pg_sys::PageGetMaxOffsetNumber(page);
                let mut offsetno = pg_sys::FirstOffsetNumber;
                let mut needs_wal = false;

                while offsetno <= max_offset {
                    let item_id = pg_sys::PageGetItemId(page, offsetno);
                    let entry = DirectoryEntry::from(PgItem(
                        pg_sys::PageGetItem(page, item_id),
                        (*item_id).lp_len() as pg_sys::Size,
                    ));
                    let SegmentComponentId(entry_segment_id) =
                        SegmentComponentPath(entry.path.clone())
                            .try_into()
                            .unwrap_or_else(|_| panic!("{:?} should be valid", entry.path.clone()));
                    if deleted_segment_ids.contains(&entry_segment_id) {
                        let entry_with_xmax = DirectoryEntry {
                            xmax: current_xid,
                            ..entry.clone()
                        };
                        let PgItem(item, size) = entry_with_xmax.clone().into();
                        let overwrite = pg_sys::PageIndexTupleOverwrite(page, offsetno, item, size);
                        assert!(
                            overwrite,
                            "setting xmax for {:?} should succeed",
                            entry_segment_id
                        );

                        // Delete the corresponding segment component
                        let segment_component =
                            LinkedBytesList::open(self.relation_oid, entry.start);
                        segment_component.mark_deleted();
                        needs_wal = true;
                    }
                    offsetno += 1;
                }

                blockno = buffer.next_blockno();
                if needs_wal {
                    pg_sys::GenericXLogFinish(state);
                } else {
                    pg_sys::GenericXLogAbort(state);
                }
                pg_sys::UnlockReleaseBuffer(buffer);
            }
        }

        Ok(())
    }

    fn load_metas(&self, inventory: &SegmentMetaInventory) -> tantivy::Result<IndexMeta> {
        let segment_metas =
            LinkedItemList::<SegmentMetaEntry>::open(self.relation_oid, SEGMENT_METAS_START);

        let cache = unsafe { BM25BufferCache::open(self.relation_oid) };
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };

        let mut alive_segments = vec![];
        let mut opstamp = 0;
        let mut max_xmin = 0;
        let mut blockno = segment_metas.get_start_blockno();

        unsafe {
            while blockno != pg_sys::InvalidBlockNumber {
                let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
                let page = pg_sys::BufferGetPage(buffer);
                let mut offsetno = pg_sys::FirstOffsetNumber;
                let max_offset = pg_sys::PageGetMaxOffsetNumber(page);

                while offsetno <= max_offset {
                    let item_id = pg_sys::PageGetItemId(page, offsetno);
                    let entry = SegmentMetaEntry::from(PgItem(
                        pg_sys::PageGetItem(page, item_id),
                        (*item_id).lp_len() as pg_sys::Size,
                    ));
                    if entry.is_visible(snapshot) {
                        let segment_meta = entry.meta.clone();
                        alive_segments.push(segment_meta.track(inventory));
                        if entry.xmin.cmp(&max_xmin) == Ordering::Greater {
                            max_xmin = entry.xmin;
                            opstamp = entry.opstamp;
                        }
                    }
                    offsetno += 1;
                }

                blockno = buffer.next_blockno();
                pg_sys::UnlockReleaseBuffer(buffer);
            }
        }

        let schema = LinkedBytesList::open(self.relation_oid, SCHEMA_START);
        let deserialized_schema = serde_json::from_slice(unsafe { &schema.read_all() })
            .expect("expected to deserialize valid Schema");

        let settings = LinkedBytesList::open(self.relation_oid, SETTINGS_START);
        let deserialized_settings = serde_json::from_slice(unsafe { &settings.read_all() })
            .expect("expected to deserialize valid IndexSettings");

        Ok(IndexMeta {
            segments: alive_segments,
            schema: deserialized_schema,
            index_settings: deserialized_settings,
            opstamp,
            payload: None,
        })
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
        Spi::run("INSERT INTO t (data) VALUES ('test');").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let directory = BlockingDirectory::new(relation_oid);
        let listed_files = directory.list_managed_files().unwrap();
        assert_eq!(listed_files.len(), 6);
    }
}
