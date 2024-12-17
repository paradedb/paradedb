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

use super::utils::{
    delete_unused_delete_metas, delete_unused_directory_entries, delete_unused_metas,
    get_deleted_ids, list_managed_files, load_metas, save_delete_metas, save_new_metas,
    save_schema, save_settings, DirectoryLookup,
};
use crate::index::channel::NeedWal;
use crate::index::reader::segment_component::SegmentComponentReader;
use crate::index::writer::segment_component::SegmentComponentWriter;
use crate::postgres::storage::block::bm25_max_free_space;
use anyhow::Result;
use parking_lot::Mutex;
use pgrx::pg_sys;
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::{io, result};
use tantivy::directory::error::{DeleteError, LockError, OpenReadError, OpenWriteError};
use tantivy::directory::{DirectoryLock, FileHandle, Lock, WatchCallback, WatchHandle, WritePtr};
use tantivy::{index::SegmentMetaInventory, Directory, IndexMeta};

/// Tantivy Directory trait implementation over block storage
/// This Directory implementation respects Postgres MVCC visibility rules
/// and should back all Tantivy Indexes used in insert and scan operations
#[derive(Clone, Debug)]
pub struct MVCCDirectory {
    relation_oid: pg_sys::Oid,
    need_wal: NeedWal,

    // keep a cache of readers behind an Arc<Mutex<_>> so that if/when this MVCCDirectory is
    // cloned, we don't lose all the work we did originally creating the FileHandler impls.  And
    // it's cloned a lot!
    readers: Arc<Mutex<FxHashMap<PathBuf, Arc<dyn FileHandle>>>>,
}

impl MVCCDirectory {
    pub fn new(relation_oid: pg_sys::Oid, need_wal: NeedWal) -> Self {
        Self {
            relation_oid,
            need_wal,
            readers: Arc::new(Mutex::new(FxHashMap::default())),
        }
    }
}

impl DirectoryLookup for MVCCDirectory {
    fn relation_oid(&self) -> pg_sys::Oid {
        self.relation_oid
    }

    fn need_wal(&self) -> NeedWal {
        self.need_wal
    }
}

impl Directory for MVCCDirectory {
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
                        SegmentComponentReader::new(self.relation_oid, opaque, self.need_wal)
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
        let result = unsafe { SegmentComponentWriter::new(self.relation_oid, path, self.need_wal) };
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
        unsafe { list_managed_files(self.relation_oid) }
    }

    // This is intentionally a no-op
    // This function is called by Tantivy in two places: during garbage collection and when a new segment is created
    // In the garbage collection case, we want to handle this ourselves because we need to do transaction visibility checks
    // In the new segment case, we want to handle this ourselves because we also store the segment's byte length and
    // block numbers alongside the path
    fn register_files_as_managed(
        &self,
        _files: Vec<PathBuf>,
        _overwrite: bool,
    ) -> tantivy::Result<()> {
        Ok(())
    }

    /// Saves a Tantivy IndexMeta to block storage
    fn save_metas(&self, meta: &IndexMeta, previous_meta: &IndexMeta) -> tantivy::Result<()> {
        let opstamp = meta.opstamp;
        let current_xid = unsafe { pg_sys::GetCurrentTransactionId() };

        // Save Schema and IndexSettings if this is the first time
        save_schema(self.relation_oid, &meta.schema, self.need_wal)
            .map_err(|err| tantivy::TantivyError::SchemaError(err.to_string()))?;

        save_settings(self.relation_oid, &meta.index_settings, self.need_wal)
            .map_err(|err| tantivy::TantivyError::InternalError(err.to_string()))?;

        // If there were no new segments, skip the rest of the work
        if meta.segments.is_empty() {
            return Ok(());
        }

        let deleted_ids = get_deleted_ids(meta, previous_meta);
        unsafe {
            save_delete_metas(self.relation_oid, meta, opstamp, self.need_wal)
                .map_err(|err| tantivy::TantivyError::InternalError(err.to_string()))?;
            save_new_metas(
                self.relation_oid,
                meta,
                previous_meta,
                current_xid,
                opstamp,
                self.need_wal,
            )
            .map_err(|err| tantivy::TantivyError::InternalError(err.to_string()))?;
            delete_unused_metas(self.relation_oid, &deleted_ids, current_xid, self.need_wal);
            delete_unused_directory_entries(
                self.relation_oid,
                &deleted_ids,
                current_xid,
                self.need_wal,
            );
            delete_unused_delete_metas(self.relation_oid, &deleted_ids, current_xid, self.need_wal);
        }

        Ok(())
    }

    fn load_metas(&self, inventory: &SegmentMetaInventory) -> tantivy::Result<IndexMeta> {
        let solve_mvcc = true;
        unsafe {
            load_metas(
                self.relation_oid,
                inventory,
                pg_sys::GetActiveSnapshot(),
                solve_mvcc,
            )
        }
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

        let directory = MVCCDirectory::new(relation_oid, true);
        let listed_files = directory.list_managed_files().unwrap();
        assert_eq!(listed_files.len(), 6);
    }
}
