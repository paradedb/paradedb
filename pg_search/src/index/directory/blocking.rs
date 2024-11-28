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
    error::TantivyError,
};
use tantivy::{index::SegmentMetaInventory, Directory, IndexMeta};

use crate::index::reader::segment_component::SegmentComponentReader;
use crate::index::writer::segment_component::SegmentComponentWriter;
use crate::postgres::storage::block::{
    bm25_max_free_space, bm25_metadata, DirectoryEntry, PgItem, SegmentComponentId,
    SegmentComponentPath, SegmentMetaEntry,
};
use crate::postgres::storage::linked_list::{LinkedBytesList, LinkedItemList};
use crate::postgres::storage::utils::BM25BufferCache;

use itertools::Itertools;

/// Tantivy Directory trait implementation over block storage
#[derive(Clone, Debug)]
pub struct BlockingDirectory {
    relation_oid: pg_sys::Oid,
}

impl BlockingDirectory {
    pub fn new(relation_oid: pg_sys::Oid) -> Self {
        Self { relation_oid }
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

    // We have done the work to ensure that Tantivy locks are not needed, only Postgres locks
    fn acquire_lock(&self, lock: &Lock) -> result::Result<DirectoryLock, LockError> {
        Ok(DirectoryLock::from(Box::new(Lock {
            filepath: lock.filepath.clone(),
            is_blocking: true,
        })))
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

    fn save_metas(&self, meta: &IndexMeta) -> tantivy::Result<()> {
        let cache = unsafe { BM25BufferCache::open(self.relation_oid) };
        let metadata = unsafe { bm25_metadata(self.relation_oid) };

        // Save Tantivy Schema if this is the first commit
        {
            let mut schema = LinkedBytesList::open_with_lock(
                self.relation_oid,
                metadata.schema_start,
                Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
            );

            if schema.is_empty() {
                let bytes =
                    serde_json::to_vec(&meta.schema).expect("expected to serialize valid Schema");
                unsafe { schema.write(&bytes).expect("write schema should succeed") };
            }
        }

        // Save Tantivy IndexSettings if this is the first commit
        {
            let mut settings = LinkedBytesList::open_with_lock(
                self.relation_oid,
                metadata.settings_start,
                Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
            );

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

        // Update SegmentMeta entries
        let opstamp = meta.opstamp;
        let current_xid = unsafe { pg_sys::GetCurrentTransactionId() };

        let mut new_segments = meta.segments.clone();
        let mut segment_metas = LinkedItemList::<SegmentMetaEntry>::open_with_lock(
            self.relation_oid,
            metadata.segment_metas_start,
            Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
        );

        crate::log_message(&format!(
            "-- SAVING {} {:?}",
            current_xid,
            new_segments
                .clone()
                .into_iter()
                .map(|s| s.id().short_uuid_string())
                .collect::<Vec<_>>()
                .into_iter()
                .sorted()
        ));

        // Mark old SegmentMeta entries as deleted
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
                .write(entries)
                .expect("save new metas should succeed");
        }

        // Mark old DirectoryEntry entries as deleted
        let directory = LinkedItemList::<DirectoryEntry>::open_with_lock(
            self.relation_oid,
            metadata.directory_start,
            Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
        );
        let segment_ids = meta.segments.iter().map(|s| s.id()).collect::<HashSet<_>>();

        unsafe {
            for (entry, blockno, offsetno) in directory.list_all_items().unwrap() {
                let SegmentComponentId(entry_segment_id) =
                    SegmentComponentPath(entry.path.clone()).try_into().unwrap();
                if !segment_ids.contains(&entry_segment_id) {
                    let entry_with_xmax = DirectoryEntry {
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
                        entry_segment_id
                    );

                    pg_sys::MarkBufferDirty(buffer);
                    pg_sys::UnlockReleaseBuffer(buffer);
                }
            }
        }

        Ok(())
    }

    fn load_metas(&self, inventory: &SegmentMetaInventory) -> tantivy::Result<IndexMeta> {
        let metadata = unsafe { bm25_metadata(self.relation_oid) };
        let segment_metas = LinkedItemList::<SegmentMetaEntry>::open_with_lock(
            self.relation_oid,
            metadata.segment_metas_start,
            Some(pg_sys::BUFFER_LOCK_SHARE),
        );

        let mut alive_segments = vec![];
        let mut opstamp = 0;
        let mut max_xmin = 0;
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };

        for (entry, _, _) in unsafe { segment_metas.list_all_items().unwrap() } {
            unsafe {
                crate::log_message(&format!(
                    "-- CHECKING {} {:?}: VISIBLE {}, ENTRY XMIN {}, ENTRY XMAX {}, SNAPSHOT XMIN {}, SNAPSHOT XMAX {}",
                    unsafe { pg_sys::GetCurrentTransactionId() },
                    entry.meta.segment_id.short_uuid_string(),
                    unsafe { is_entry_visible(entry.xmin, entry.xmax, snapshot) },
                    entry.xmin,
                    entry.xmax,
                    (*snapshot).xmin,
                    (*snapshot).xmax
                ));
            }

            let visible = unsafe { is_entry_visible(entry.xmin, entry.xmax, snapshot) };
            if visible {
                let segment_meta = entry.meta.clone();
                alive_segments.push(segment_meta.track(inventory));

                // TODO: Store optstamps separately
                if entry.xmin > max_xmin {
                    max_xmin = entry.xmin;
                    opstamp = entry.opstamp;
                } else if entry.xmin == max_xmin {
                    opstamp = entry.opstamp.max(opstamp);
                }
            }
        }

        crate::log_message(&format!(
            "-- LOADED {} {:?}",
            unsafe { pg_sys::GetCurrentTransactionId() },
            alive_segments
                .clone()
                .into_iter()
                .map(|s| s.id().short_uuid_string())
                .collect::<Vec<_>>()
                .into_iter()
                .sorted()
        ));

        let schema = LinkedBytesList::open_with_lock(
            self.relation_oid,
            metadata.schema_start,
            Some(pg_sys::BUFFER_LOCK_SHARE),
        );
        let deserialized_schema = serde_json::from_slice(unsafe { &schema.read_all() })
            .expect("expected to deserialize valid Schema");

        let settings = LinkedBytesList::open_with_lock(
            self.relation_oid,
            metadata.settings_start,
            Some(pg_sys::BUFFER_LOCK_SHARE),
        );
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

unsafe fn is_entry_visible(
    xmin: pg_sys::TransactionId,
    xmax: pg_sys::TransactionId,
    snapshot: pg_sys::Snapshot,
) -> bool {
    let xmin_visible =
        !pg_sys::XidInMVCCSnapshot(xmin, snapshot) && pg_sys::TransactionIdDidCommit(xmin);
    let deleted = xmax != pg_sys::InvalidTransactionId
        && !pg_sys::XidInMVCCSnapshot(xmax, snapshot)
        && pg_sys::TransactionIdDidCommit(xmax);
    xmin_visible && !deleted
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
