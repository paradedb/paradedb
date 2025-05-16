// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use super::utils::{load_metas, save_new_metas, save_schema, save_settings};
use crate::index::channel::{ChannelRequest, ChannelRequestHandler};
use crate::index::reader::segment_component::SegmentComponentReader;
use crate::index::writer::segment_component::SegmentComponentWriter;
use crate::postgres::storage::block::{
    bm25_max_free_space, FileEntry, MVCCEntry, SegmentMetaEntry, SEGMENT_METAS_START,
};
use crate::postgres::storage::buffer::{BufferManager, PinnedBuffer};
use crate::postgres::storage::LinkedItemList;
use crossbeam::channel::Receiver;
use parking_lot::Mutex;
use pgrx::{pg_sys, PgRelation};
use rustc_hash::FxHashMap;
use std::any::Any;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Debug, Display};
use std::panic::panic_any;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::{io, result};
use tantivy::directory::error::{
    DeleteError, LockError, OpenDirectoryError, OpenReadError, OpenWriteError,
};
use tantivy::directory::{
    DirectoryLock, DirectoryPanicHandler, FileHandle, Lock, WatchCallback, WatchHandle, WritePtr,
};
use tantivy::index::SegmentId;
use tantivy::{index::SegmentMetaInventory, Directory, IndexMeta, TantivyError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MvccSatisfies {
    ParallelWorker(HashSet<SegmentId>),
    Snapshot,
    Vacuum,
    Mergeable,
}

impl MvccSatisfies {
    pub fn directory(self, index_relation: &PgRelation) -> MVCCDirectory {
        match self {
            MvccSatisfies::ParallelWorker(segment_ids) => {
                MVCCDirectory::parallel_worker(index_relation.oid(), segment_ids)
            }
            MvccSatisfies::Snapshot => MVCCDirectory::snapshot(index_relation.oid()),
            MvccSatisfies::Vacuum => MVCCDirectory::vacuum(index_relation.oid()),
            MvccSatisfies::Mergeable => MVCCDirectory::mergeable(index_relation.oid()),
        }
    }
    pub fn channel_request_handler(
        self,
        index_relation: &PgRelation,
        receiver: Receiver<ChannelRequest>,
    ) -> ChannelRequestHandler {
        ChannelRequestHandler::open(
            self.directory(index_relation),
            index_relation.oid(),
            receiver,
        )
    }
}

type AtomicFileEntry = (FileEntry, Arc<AtomicUsize>);
/// Tantivy Directory trait implementation over block storage
/// This Directory implementation respects Postgres MVCC visibility rules
/// and should back all Tantivy Indexes used in insert and scan operations
#[derive(Clone, Debug)]
pub struct MVCCDirectory {
    relation_oid: pg_sys::Oid,
    mvcc_style: MvccSatisfies,

    // keep a cache of readers behind an Arc<Mutex<_>> so that if/when this MVCCDirectory is
    // cloned, we don't lose all the work we did originally creating the FileHandler impls.  And
    // it's cloned a lot!
    readers: Arc<Mutex<FxHashMap<PathBuf, Arc<dyn FileHandle>>>>,
    new_files: Arc<Mutex<FxHashMap<PathBuf, AtomicFileEntry>>>,

    // a lazily loaded [`IndexMeta`], which is only created once per MVCCDirectory instance
    // we cannot tolerate tantivy calling `load_metas()` multiple times and giving it a different
    // answer
    loaded_metas: OnceLock<Arc<tantivy::Result<IndexMeta>>>,
    all_entries: Arc<Mutex<HashMap<SegmentId, SegmentMetaEntry>>>,
    pin_cushion: Arc<Mutex<Option<PinCushion>>>,
}

unsafe impl Send for MVCCDirectory {}
unsafe impl Sync for MVCCDirectory {}

impl MVCCDirectory {
    pub fn parallel_worker(relation_oid: pg_sys::Oid, segment_ids: HashSet<SegmentId>) -> Self {
        Self::with_mvcc_style(relation_oid, MvccSatisfies::ParallelWorker(segment_ids))
    }

    pub fn snapshot(relation_oid: pg_sys::Oid) -> Self {
        Self::with_mvcc_style(relation_oid, MvccSatisfies::Snapshot)
    }

    pub fn vacuum(relation_oid: pg_sys::Oid) -> Self {
        Self::with_mvcc_style(relation_oid, MvccSatisfies::Vacuum)
    }

    pub fn mergeable(relation_oid: pg_sys::Oid) -> Self {
        Self::with_mvcc_style(relation_oid, MvccSatisfies::Mergeable)
    }

    fn with_mvcc_style(relation_oid: pg_sys::Oid, mvcc_style: MvccSatisfies) -> Self {
        Self {
            relation_oid,
            mvcc_style,
            readers: Default::default(),
            new_files: Default::default(),
            loaded_metas: Default::default(),
            pin_cushion: Default::default(),
            all_entries: Default::default(),
        }
    }

    pub unsafe fn directory_lookup(&self, path: &Path) -> tantivy::Result<FileEntry> {
        let file_name = path
            .file_name()
            .expect("path should have a filename")
            .to_str()
            .expect("path should be valid UTF8");
        let file_name = &file_name[..file_name.find('.').unwrap_or(file_name.len())];
        let segment_id = SegmentId::from_uuid_string(file_name)
            .map_err(|e| TantivyError::InvalidArgument(e.to_string()))?;

        if let Some(meta_entry) = self.all_entries.lock().get(&segment_id) {
            if let Some(file_entry) = meta_entry.file_entry(path) {
                return Ok(file_entry);
            }
        }

        Err(TantivyError::OpenDirectoryError(
            OpenDirectoryError::DoesNotExist(path.to_path_buf()),
        ))
    }

    /// Drop the pins that are held on the specified [`SegmentId`]s.
    ///
    /// # Safety
    ///
    /// This does not remove the segments themselves from being accessible by Tantivy, which means
    /// that attempts to use these segments after dropping their pins will likely lead to incorrect
    /// behavior.  It is the callers responsibility to ensure this does not happen.
    pub(crate) unsafe fn drop_pins(&mut self, segment_ids: &[SegmentId]) -> tantivy::Result<()> {
        let all_entries = self.all_entries.lock();
        let mut pin_cushion = self.pin_cushion.lock();
        let pin_cushion = pin_cushion
            .as_mut()
            .expect("pin_cushion should have been initialized by now");
        for segment_id in segment_ids {
            let entry = all_entries.get(segment_id).ok_or_else(|| {
                TantivyError::SystemError(format!("segment {segment_id} not found in pin cushion"))
            })?;
            pin_cushion.remove(entry.pintest_blockno());
        }

        Ok(())
    }

    pub(crate) unsafe fn drop_pin(&mut self, segment_id: &SegmentId) -> Option<()> {
        let all_entries = self.all_entries.lock();
        let segment_meta_entry = all_entries.get(segment_id)?;
        let mut pin_cushion = self.pin_cushion.lock();
        let pin_cushion = pin_cushion.as_mut()?;

        pin_cushion.remove(segment_meta_entry.pintest_blockno());
        Some(())
    }

    pub(crate) fn all_entries(&self) -> HashMap<SegmentId, SegmentMetaEntry> {
        self.all_entries.lock().clone()
    }
}

impl Directory for MVCCDirectory {
    /// Returns a segment reader that implements std::io::Read
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        match self.readers.lock().entry(path.to_path_buf()) {
            Entry::Occupied(reader) => Ok(reader.get().clone()),
            Entry::Vacant(vacant) => {
                let file_entry = unsafe {
                    match self.directory_lookup(path) {
                        Ok(file_entry) => file_entry,
                        Err(err) => {
                            if let Some((file_entry, total_bytes)) = self.new_files.lock().get(path)
                            {
                                FileEntry {
                                    starting_block: file_entry.starting_block,
                                    total_bytes: total_bytes.load(Ordering::Relaxed),
                                }
                            } else {
                                return Err(OpenReadError::IoError {
                                    io_error: io::Error::new(io::ErrorKind::Other, err.to_string())
                                        .into(),
                                    filepath: PathBuf::from(path),
                                });
                            }
                        }
                    }
                };
                Ok(vacant
                    .insert(Arc::new(unsafe {
                        SegmentComponentReader::new(self.relation_oid, file_entry)
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
    /// Our [`ChannelDirectory`] is what gets called for doing writes, not this impl
    fn open_write(&self, path: &Path) -> result::Result<WritePtr, OpenWriteError> {
        let writer = unsafe { SegmentComponentWriter::new(self.relation_oid, path) };
        self.new_files.lock().insert(
            path.to_path_buf(),
            (writer.file_entry(), writer.total_bytes()),
        );
        Ok(io::BufWriter::with_capacity(
            bm25_max_free_space(),
            Box::new(writer),
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
            let segment_metas =
                LinkedItemList::<SegmentMetaEntry>::open(self.relation_oid, SEGMENT_METAS_START);
            Ok(segment_metas
                .list()
                .iter()
                .flat_map(|entry| entry.get_component_paths())
                .collect())
        }
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
    fn save_metas(
        &self,
        meta: &IndexMeta,
        previous_meta: &IndexMeta,
        payload: &mut (dyn Any + '_),
    ) -> tantivy::Result<()> {
        let mut file_entries = FxHashMap::default();
        let payload = if let Some(payload) = payload.downcast_mut::<FxHashMap<PathBuf, FileEntry>>()
        {
            payload
        } else {
            for (path, (file_entry, total_bytes)) in self.new_files.lock().iter() {
                file_entries.insert(
                    path.clone(),
                    FileEntry {
                        starting_block: file_entry.starting_block,
                        total_bytes: total_bytes.load(Ordering::Relaxed),
                    },
                );
            }
            &mut file_entries
        };

        // Save Schema and IndexSettings if this is the first time
        save_schema(self.relation_oid, &meta.schema)
            .map_err(|err| tantivy::TantivyError::SchemaError(err.to_string()))?;

        save_settings(self.relation_oid, &meta.index_settings)
            .map_err(|err| tantivy::TantivyError::InternalError(err.to_string()))?;

        // If there were no new segments, skip the rest of the work
        if meta.segments.is_empty() {
            return Ok(());
        }

        unsafe {
            save_new_metas(self.relation_oid, meta, previous_meta, payload)
                .map_err(|err| tantivy::TantivyError::InternalError(err.to_string()))?;
        }

        Ok(())
    }

    fn load_metas(&self, inventory: &SegmentMetaInventory) -> tantivy::Result<IndexMeta> {
        let loaded_metas = self.loaded_metas.get_or_init(|| unsafe {
            match load_metas(self.relation_oid, inventory, &self.mvcc_style) {
                Err(e) => Arc::new(Err(e)),
                Ok((all_entries, index_meta, pin_cushion)) => {
                    *self.all_entries.lock() = all_entries
                        .into_iter()
                        .map(|entry| (entry.segment_id, entry))
                        .collect();
                    *self.pin_cushion.lock() = Some(pin_cushion);
                    Arc::new(Ok(index_meta))
                }
            }
        });

        Clone::clone(loaded_metas)
    }

    fn supports_garbage_collection(&self) -> bool {
        false
    }

    fn panic_handler(&self) -> Option<DirectoryPanicHandler> {
        let panic_handler = move |any: Box<dyn Any + Send>| {
            fn downcast_to_panic(any: Box<dyn Any + Send>, depth: usize) {
                // NB:  the `any` error could be other types too, but lord knows what they might be

                if let Some(message) = any.downcast_ref::<String>() {
                    pgrx::warning!("{message}");
                } else if let Some(message) = any.downcast_ref::<&str>() {
                    pgrx::warning!("{message}");
                } else if let Some(message) = any.downcast_ref::<tantivy::TantivyError>() {
                    pgrx::warning!("{message:?}");
                } else if let Some(message) = any.downcast_ref::<&dyn Display>() {
                    pgrx::warning!("{message}");
                } else if let Some(message) = any.downcast_ref::<&dyn Debug>() {
                    pgrx::warning!("{message:?}")
                } else if let Some(message) = any.downcast_ref::<&dyn Error>() {
                    pgrx::warning!("{message}");
                } else {
                    if depth >= 10 {
                        // just to avoid recursing forever if we always end up downcasting to another
                        // `[Box<dyn Any + Send>]`
                        panic_any(any);
                    }
                    match any.downcast::<Box<dyn Any + Send>>() {
                        // The actual error might be hidden behind another Box<dyn Any + Send>
                        // so recurse with this boxed version
                        Ok(any) => downcast_to_panic(*any, depth + 1),

                        // this will likely just panic with a message that says:  Any { .. }
                        // completely unhelpful, but it is better than also having Postgres crash
                        Err(unknown) => panic_any(unknown),
                    }
                }
            }

            downcast_to_panic(any, 0);
        };
        Some(Arc::new(panic_handler))
    }

    fn wants_cancel(&self) -> bool {
        unsafe {
            pg_sys::QueryCancelPending != 0
                || !pg_sys::IsTransactionState()
                || pg_sys::IsAbortedTransactionBlockState()
        }
    }

    fn log(&self, message: &str) {
        pgrx::debug1!("{message}");
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct PinCushion(HashMap<pg_sys::BlockNumber, PinnedBuffer>);

impl PinCushion {
    pub fn push(&mut self, bman: &BufferManager, entry: &SegmentMetaEntry) {
        let blockno = entry.pintest_blockno();
        self.0.insert(blockno, bman.pinned_buffer(blockno));
    }

    pub fn remove(&mut self, blockno: pg_sys::BlockNumber) {
        self.0.remove(&blockno);
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;

    #[pg_test]
    fn test_list_meta_entries() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("INSERT INTO t (data) VALUES ('test');").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let linked_list =
            LinkedItemList::<SegmentMetaEntry>::open(relation_oid, SEGMENT_METAS_START);
        let mut listed_files = unsafe { linked_list.list() };
        assert_eq!(listed_files.len(), 1);
        let entry = listed_files.pop().unwrap();
        assert!(entry.store.is_some());
        assert!(entry.field_norms.is_some());
        assert!(entry.fast_fields.is_some());
        assert!(entry.postings.is_some());
        assert!(entry.positions.is_some());
        assert!(entry.terms.is_some());
        assert!(entry.delete.is_none());
    }
}
