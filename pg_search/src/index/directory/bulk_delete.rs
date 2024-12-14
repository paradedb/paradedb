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
use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::{io, result};
use tantivy::directory::error::{DeleteError, LockError, OpenReadError, OpenWriteError};
use tantivy::directory::{DirectoryLock, FileHandle, Lock, WatchCallback, WatchHandle, WritePtr};
use tantivy::{index::SegmentMetaInventory, Directory, IndexMeta};

use super::utils::{
    delete_unused_delete_metas, delete_unused_directory_entries, delete_unused_metas,
    get_deleted_ids, list_managed_files, load_metas, save_delete_metas, DirectoryLookup,
};
use crate::index::reader::segment_component::SegmentComponentReader;
use crate::index::writer::segment_component::SegmentComponentWriter;
use crate::postgres::storage::block::bm25_max_free_space;

/// The sole purpose of the BulkDeleteDirectory is to propagate deleted terms to the Tantivy index
/// It is meant to be called by ambulkdelete and should **never** be used for any other purpose
/// because it does not respect Postgres MVCC visibility rules
#[derive(Clone, Debug)]
pub struct BulkDeleteDirectory {
    relation_oid: pg_sys::Oid,
    readers: Arc<Mutex<FxHashMap<PathBuf, Arc<dyn FileHandle>>>>,
}

impl BulkDeleteDirectory {
    pub fn new(relation_oid: pg_sys::Oid) -> Self {
        Self {
            relation_oid,
            readers: Arc::new(Mutex::new(FxHashMap::default())),
        }
    }
}

impl DirectoryLookup for BulkDeleteDirectory {
    fn relation_oid(&self) -> pg_sys::Oid {
        self.relation_oid
    }
}

impl Directory for BulkDeleteDirectory {
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

    fn delete(&self, _path: &Path) -> result::Result<(), DeleteError> {
        Ok(())
    }

    fn exists(&self, _path: &Path) -> Result<bool, OpenReadError> {
        Ok(true)
    }

    fn open_write(&self, path: &Path) -> result::Result<WritePtr, OpenWriteError> {
        let result = unsafe { SegmentComponentWriter::new(self.relation_oid, path) };
        Ok(io::BufWriter::with_capacity(
            bm25_max_free_space(),
            Box::new(result),
        ))
    }

    fn atomic_read(&self, path: &Path) -> result::Result<Vec<u8>, OpenReadError> {
        unimplemented!("atomic_read should not be called for {:?}", path);
    }

    fn atomic_write(&self, path: &Path, _data: &[u8]) -> io::Result<()> {
        unimplemented!("atomic_write should not be called for {:?}", path);
    }

    fn sync_directory(&self) -> io::Result<()> {
        Ok(())
    }

    fn acquire_lock(&self, lock: &Lock) -> result::Result<DirectoryLock, LockError> {
        Ok(DirectoryLock::from(Box::new(Lock {
            filepath: lock.filepath.clone(),
            is_blocking: true,
        })))
    }

    fn watch(&self, _watch_callback: WatchCallback) -> tantivy::Result<WatchHandle> {
        unimplemented!("OnCommitWithDelay ReloadPolicy not supported");
    }

    fn list_managed_files(&self) -> tantivy::Result<HashSet<PathBuf>> {
        unsafe { list_managed_files(self.relation_oid) }
    }

    fn register_files_as_managed(
        &self,
        _files: Vec<PathBuf>,
        _overwrite: bool,
    ) -> tantivy::Result<()> {
        Ok(())
    }

    fn save_metas(&self, meta: &IndexMeta, previous_meta: &IndexMeta) -> tantivy::Result<()> {
        let opstamp = meta.opstamp;
        let current_xid = unsafe { pg_sys::GetCurrentTransactionId() };

        if meta.segments.is_empty() {
            return Ok(());
        }

        let deleted_ids = get_deleted_ids(meta, previous_meta);
        unsafe {
            save_delete_metas(self.relation_oid, meta, opstamp)
                .map_err(|err| tantivy::TantivyError::InternalError(err.to_string()))?;
            delete_unused_metas(self.relation_oid, &deleted_ids, current_xid);
            delete_unused_directory_entries(self.relation_oid, &deleted_ids, current_xid);
            delete_unused_delete_metas(self.relation_oid, &deleted_ids, current_xid);
        }

        Ok(())
    }

    fn load_metas(&self, inventory: &SegmentMetaInventory) -> tantivy::Result<IndexMeta> {
        let solve_mvcc = false;
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
