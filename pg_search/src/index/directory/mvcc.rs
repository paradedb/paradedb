// Copyright (c) 2023-2026 ParadeDB, Inc.
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
use crate::api::HashMap;
use crate::index::reader::segment_component::SegmentComponentReader;
use crate::index::writer::segment_component::SegmentComponentWriter;
use crate::postgres::heap::{ExpressionState, HeapFetchState};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{
    bm25_max_free_space, FileEntry, MVCCEntry, SegmentMetaEntry, SegmentMetaEntryContent,
    SegmentMetaEntryImmutable, SegmentMetaEntryMutable,
};
use crate::postgres::storage::buffer::{BufferManager, PinnedBuffer};
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::storage::MAX_BUFFERS_TO_EXTEND_BY;
use parking_lot::Mutex;
use pgrx::pg_sys;
use std::any::Any;
use std::collections::hash_map::Entry;
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
    DirectoryLock, DirectoryPanicHandler, FileHandle, InnerWritePtr, Lock, RamDirectory,
    WatchCallback, WatchHandle,
};
use tantivy::index::{SegmentComponent, SegmentId, SegmentMetaInventory};
use tantivy::{Directory, IndexMeta, SegmentMeta, TantivyError};

/// By default Tantivy writes 8192 bytes at a time (the `BufWriter` default).
/// We want to write more at a time so we can allocate chunks of blocks all at once,
/// which creates less lock contention than allocating one block at a time.
pub const BUFWRITER_CAPACITY: usize = bm25_max_free_space() * MAX_BUFFERS_TO_EXTEND_BY;

/// The `(max_doc, num_deleted_docs)` pair of a mutable segment's meta entry as one reader
/// loaded it. A mutable segment is materialized from the heap on open, from the prefix of its
/// add/remove log these two counts bound, so two opens agree on the segment's `DocId` space
/// only if they use the same pair. Distinct from [`SegmentMetaEntry::mutable_snapshot`], which
/// returns the ctid set a bound selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MutableSegmentBound {
    pub max_doc: u32,
    pub num_deleted_docs: u32,
}

impl MutableSegmentBound {
    /// Docs the materialized segment exposes: Adds minus Removes.
    pub fn live_docs(self) -> u32 {
        self.max_doc - self.num_deleted_docs
    }
}

/// One segment of a [`SegmentView`], in the origin reader's ordinal position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentViewEntry {
    pub id: SegmentId,
    pub docs: SegmentViewDocs,
}

/// What the origin reader knew about the segment's documents. Mirrors the
/// [`SegmentMetaEntryContent`] split: immutable content is fixed on disk, mutable content is
/// materialized per open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentViewDocs {
    /// Fixed on disk, so a replaying reader gets the same documents from the segment id alone.
    /// The counts ride along only because `EXPLAIN ANALYZE` renders per-segment doc counts out
    /// of the shared state rather than out of a reader.
    Immutable { max_doc: u32, num_deleted_docs: u32 },
    /// Materialized per open from the log prefix the bound selects; every replaying reader
    /// must use the same bound.
    Mutable(MutableSegmentBound),
    /// Nothing known: the view only pins the segment set. See
    /// [`SegmentView::from_unordered_ids`].
    IdOnly,
}

impl SegmentViewDocs {
    /// What `SegmentReader::max_doc()` reports for this segment under this view.
    pub fn max_doc(self) -> u32 {
        match self {
            SegmentViewDocs::Immutable { max_doc, .. } => max_doc,
            SegmentViewDocs::Mutable(bound) => bound.live_docs(),
            SegmentViewDocs::IdOnly => 0,
        }
    }

    /// What `SegmentReader::num_deleted_docs()` reports for this segment under this view.
    /// Zero for a mutable segment: materialization skips removed ctids instead of carrying
    /// a delete bitset.
    pub fn num_deleted_docs(self) -> u32 {
        match self {
            SegmentViewDocs::Immutable {
                num_deleted_docs, ..
            } => num_deleted_docs,
            SegmentViewDocs::Mutable(_) | SegmentViewDocs::IdOnly => 0,
        }
    }

    pub fn mutable_bound(self) -> Option<MutableSegmentBound> {
        match self {
            SegmentViewDocs::Mutable(bound) => Some(bound),
            _ => None,
        }
    }
}

impl SegmentViewEntry {
    pub fn max_doc(&self) -> u32 {
        self.docs.max_doc()
    }

    pub fn num_deleted_docs(&self) -> u32 {
        self.docs.num_deleted_docs()
    }

    pub fn mutable_bound(&self) -> Option<MutableSegmentBound> {
        self.docs.mutable_bound()
    }
}

/// A reader's segment view, frozen so other readers of the same index can replay it.
///
/// Packed `(segment_ord, doc_id)` addresses and per-segment term ordinals travel between
/// readers opened in different processes (a JoinScan leader, its MPP workers, the readers a
/// worker rebuilds for a scan that sits behind a network boundary). Those addresses are only
/// meaningful against the reader that produced them, so every reader of one source in one query
/// must expose the same ordinal order and, for mutable segments, materialize the same document
/// set. Opening with [`MvccSatisfies::ParallelWorker`] over this view gives that: the listed
/// segments in the listed order, mutable segments bounded by the origin's view.
///
/// The data sits behind an `Arc`, so clones are cheap even at thousands of segments.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SegmentView {
    inner: Arc<SegmentViewInner>,
}

#[derive(Debug, PartialEq, Eq, Default)]
struct SegmentViewInner {
    entries: Vec<SegmentViewEntry>,
    ordinals: HashMap<SegmentId, usize>,
}

impl SegmentView {
    pub fn new(entries: Vec<SegmentViewEntry>) -> Self {
        let ordinals = entries
            .iter()
            .enumerate()
            .map(|(ord, e)| (e.id, ord))
            .collect();
        Self {
            inner: Arc::new(SegmentViewInner { entries, ordinals }),
        }
    }

    /// Capture the view of `segment_readers` (in ordinal order), taking mutable bounds from
    /// `directory`. Both must come from the same open.
    pub fn capture(segment_readers: &[tantivy::SegmentReader], directory: &MVCCDirectory) -> Self {
        let mutable = directory.mutable_bounds();
        Self::new(
            segment_readers
                .iter()
                .map(|reader| {
                    let id = reader.segment_id();
                    let docs = match mutable.get(&id) {
                        Some(bound) => SegmentViewDocs::Mutable(*bound),
                        None => SegmentViewDocs::Immutable {
                            max_doc: reader.max_doc(),
                            num_deleted_docs: reader.num_deleted_docs(),
                        },
                    };
                    SegmentViewEntry { id, docs }
                })
                .collect(),
        )
    }

    /// A view that only pins the segment set. Ordinal order is the iterator's (arbitrary for a
    /// set), and it carries no replay data, so it is only for readers whose addresses never
    /// leave the process (a `paradedb.aggregate` worker).
    pub fn from_unordered_ids(ids: impl IntoIterator<Item = SegmentId>) -> Self {
        Self::new(
            ids.into_iter()
                .map(|id| SegmentViewEntry {
                    id,
                    docs: SegmentViewDocs::IdOnly,
                })
                .collect(),
        )
    }

    pub fn entries(&self) -> &[SegmentViewEntry] {
        &self.inner.entries
    }

    pub fn len(&self) -> usize {
        self.inner.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.entries.is_empty()
    }

    pub fn contains(&self, id: &SegmentId) -> bool {
        self.inner.ordinals.contains_key(id)
    }

    pub fn ids(&self) -> impl Iterator<Item = SegmentId> + '_ {
        self.inner.entries.iter().map(|e| e.id)
    }

    /// The origin reader's ordinal for `id`.
    pub fn ordinal_of(&self, id: &SegmentId) -> Option<usize> {
        self.inner.ordinals.get(id).copied()
    }

    pub fn mutable_bound(&self, id: &SegmentId) -> Option<MutableSegmentBound> {
        self.ordinal_of(id)
            .and_then(|ord| self.inner.entries[ord].mutable_bound())
    }
}

/// Describes how a `MvccDirectory` should resolve segment visibility.  Note that
/// this enum is purposely non-cloneable.  Wrap it with an [`Arc`] if you need that.  Because of
/// the [`MvccSatisfies::ParallelWorker`] variant, cloning could be incredibly expensive when
/// an index has many (thousands!) of segments.
#[derive(Debug, PartialEq, Eq)]
pub enum MvccSatisfies {
    /// Replay exactly the given view; see [`SegmentView`].
    ParallelWorker(SegmentView),
    LargestSegment,
    Snapshot,
    Vacuum,
    Mergeable,
}

impl MvccSatisfies {
    pub fn directory(self, index_relation: &PgSearchRelation) -> MVCCDirectory {
        MVCCDirectory::with_mvcc_style(index_relation, self)
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
enum LoadedSegmentMetaEntry {
    Persisted {
        meta: SegmentMetaEntry,
        entry: SegmentMetaEntryImmutable,
    },
    Memory {
        meta: SegmentMetaEntry,
        tantivy_meta: SegmentMeta,
        entry: SegmentMetaEntryMutable,
        // Created lazily on first read so that indexing occurs in whichever parallel worker is
        // responsible for this segment.
        directory: Arc<OnceLock<RamDirectory>>,
    },
}

impl LoadedSegmentMetaEntry {
    fn pintest_blockno(&self) -> pg_sys::BlockNumber {
        match self {
            Self::Persisted { entry, .. } => entry.pintest_blockno(),
            Self::Memory { entry, .. } => entry.pintest_blockno(),
        }
    }
}

type AtomicFileEntry = (FileEntry, Arc<AtomicUsize>);
/// Tantivy Directory trait implementation over block storage
/// This Directory implementation respects Postgres MVCC visibility rules
/// and should back all Tantivy Indexes used in insert and scan operations
#[derive(Debug, Clone)]
pub struct MVCCDirectory {
    //
    // NB:  Directories get cloned, **A LOT**, by tantivy.  As such, it should be cheap, especially
    // in terms of memory usage, to clone this struct.
    //
    indexrel: PgSearchRelation,
    mvcc_style: Arc<MvccSatisfies>,

    // keep a cache of readers behind an Arc<Mutex<_>> so that if/when this MVCCDirectory is
    // cloned, we don't lose all the work we did originally creating the FileHandler impls.  And
    // it's cloned a lot!
    readers: Arc<Mutex<HashMap<PathBuf, Arc<dyn FileHandle>>>>,
    new_files: Arc<Mutex<HashMap<PathBuf, AtomicFileEntry>>>,

    // a lazily loaded [`IndexMeta`], which is only created once per MVCCDirectory instance
    // we cannot tolerate tantivy calling `load_metas()` multiple times and giving it a different
    // answer
    loaded_metas: Arc<OnceLock<tantivy::Result<IndexMeta>>>,
    all_entries: Arc<Mutex<HashMap<SegmentId, LoadedSegmentMetaEntry>>>,
    pin_cushion: Arc<Mutex<Option<PinCushion>>>,
    total_segment_count: Arc<AtomicUsize>,
    total_docs: Arc<AtomicUsize>,
    heap_fetch_state: Arc<OnceLock<HeapFetchState>>,
    expression_state: Arc<OnceLock<ExpressionState>>,
}

unsafe impl Send for MVCCDirectory {}
unsafe impl Sync for MVCCDirectory {}

impl MVCCDirectory {
    pub fn parallel_worker(index_relation: &PgSearchRelation, view: SegmentView) -> Self {
        Self::with_mvcc_style(index_relation, MvccSatisfies::ParallelWorker(view))
    }

    pub fn with_mvcc_style(index_relation: &PgSearchRelation, mvcc_style: MvccSatisfies) -> Self {
        Self {
            indexrel: Clone::clone(index_relation),
            mvcc_style: Arc::new(mvcc_style),

            readers: Default::default(),
            new_files: Default::default(),
            loaded_metas: Default::default(),
            pin_cushion: Default::default(),
            all_entries: Default::default(),
            total_segment_count: Default::default(),
            total_docs: Default::default(),
            heap_fetch_state: Default::default(),
            expression_state: Default::default(),
        }
    }

    /// The bound each loaded mutable segment was materialized from.
    pub fn mutable_bounds(&self) -> HashMap<SegmentId, MutableSegmentBound> {
        self.all_entries
            .lock()
            .iter()
            .filter_map(|(id, lsme)| match lsme {
                LoadedSegmentMetaEntry::Persisted { .. } => None,
                LoadedSegmentMetaEntry::Memory { meta, .. } => Some((
                    *id,
                    MutableSegmentBound {
                        max_doc: meta.max_doc(),
                        num_deleted_docs: meta.num_deleted_docs() as u32,
                    },
                )),
            })
            .collect()
    }

    /// If the given SegmentId is a mutable segment, return true.
    pub fn is_mutable(&self, segment_id: &SegmentId) -> bool {
        self.all_entries
            .lock()
            .get(segment_id)
            .map(|lsme| match lsme {
                LoadedSegmentMetaEntry::Persisted { .. } => false,
                LoadedSegmentMetaEntry::Memory { .. } => true,
            })
            .unwrap_or(false)
    }

    fn file_entry(&self, path: &Path) -> tantivy::Result<Arc<dyn FileHandle>> {
        let file_name = path
            .file_name()
            .expect("path should have a filename")
            .to_str()
            .expect("path should be valid UTF8");
        let uuid_string = &file_name[..file_name.find('.').unwrap_or(file_name.len())];
        let segment_id = SegmentId::from_uuid_string(uuid_string)
            .map_err(|e| TantivyError::InvalidArgument(e.to_string()))?;

        let Some(meta_entry) = self.all_entries.lock().get(&segment_id).cloned() else {
            return Err(TantivyError::OpenDirectoryError(
                OpenDirectoryError::DoesNotExist(path.to_path_buf()),
            ));
        };

        match meta_entry {
            LoadedSegmentMetaEntry::Persisted { entry, .. } => {
                let Some(file_entry) = entry.file_entry(uuid_string, path) else {
                    return Err(TantivyError::OpenDirectoryError(
                        OpenDirectoryError::DoesNotExist(path.to_path_buf()),
                    ));
                };
                Ok(Arc::new(unsafe {
                    SegmentComponentReader::new(
                        &self.indexrel,
                        file_entry,
                        path.extension()
                            .and_then(|ext| ext.to_str())
                            .and_then(|ext| SegmentComponent::try_from(ext).ok()),
                    )
                }))
            }
            LoadedSegmentMetaEntry::Memory {
                meta,
                tantivy_meta,
                directory,
                ..
            } => {
                let file_handle = directory
                    .get_or_init(|| {
                        let heap_fetch_state = self.heap_fetch_state.get_or_init(|| {
                            let heaprel = self
                                .indexrel
                                .heap_relation()
                                .expect("Should have a heap relation.");
                            HeapFetchState::new(&heaprel)
                        });
                        let expression_state = self
                            .expression_state
                            .get_or_init(|| ExpressionState::new(&self.indexrel));
                        index_memory_segment(
                            &self.indexrel,
                            &tantivy_meta,
                            &meta,
                            heap_fetch_state,
                            expression_state,
                            &self.mvcc_style,
                        )
                        .expect("Failed to index mutable segment.")
                    })
                    .get_file_handle(path)
                    .map_err(|err| match err {
                        OpenReadError::FileDoesNotExist(missing) => {
                            TantivyError::OpenDirectoryError(OpenDirectoryError::DoesNotExist(
                                missing,
                            ))
                        }
                        other => other.into(),
                    })?;
                Ok(file_handle)
            }
        }
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

        let pintest_blockno = all_entries.get(segment_id)?.pintest_blockno();
        let mut pin_cushion = self.pin_cushion.lock();
        let pin_cushion = pin_cushion.as_mut()?;

        pin_cushion.remove(pintest_blockno);
        Some(())
    }

    pub(crate) fn all_entries(&self) -> HashMap<SegmentId, SegmentMetaEntry> {
        self.all_entries
            .lock()
            .iter()
            .map(|(id, entry)| match entry {
                LoadedSegmentMetaEntry::Persisted { meta, .. } => (*id, *meta),
                LoadedSegmentMetaEntry::Memory { meta, .. } => (*id, *meta),
            })
            .collect()
    }

    /// Return the [`SegmentMetaEntry`] for a single loaded segment.
    ///
    /// Unlike [`Self::all_entries`], this avoids cloning the entire metadata map when only one
    /// entry is needed.
    pub(crate) fn segment_meta_entry(&self, segment_id: &SegmentId) -> Option<SegmentMetaEntry> {
        self.all_entries
            .lock()
            .get(segment_id)
            .map(|entry| match entry {
                LoadedSegmentMetaEntry::Persisted { meta, .. }
                | LoadedSegmentMetaEntry::Memory { meta, .. } => *meta,
            })
    }

    /// Returns the [`AtomicUsize`] where the number of segments that survive [`load_metas()`]'
    /// visibility checking gets stored once [`load_metas()`] has actually been called.
    ///
    /// An implementation detail behind the value calculation is that there's special casing for
    /// [`MvccSatisfies::LargestSegment`] in that it will use the count of **all** "Snapshot"-visible
    /// segments rather than `1` (one).
    pub(crate) fn total_segment_count(&self) -> Arc<AtomicUsize> {
        self.total_segment_count.clone()
    }

    pub(crate) fn total_docs(&self) -> Arc<AtomicUsize> {
        self.total_docs.clone()
    }
}

impl Directory for MVCCDirectory {
    /// Returns a segment reader that implements std::io::Read
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        match self.readers.lock().entry(path.to_path_buf()) {
            Entry::Occupied(reader) => Ok(reader.get().clone()),
            Entry::Vacant(vacant) => match self.file_entry(path) {
                Ok(file_handle) => Ok(vacant.insert(file_handle).clone()),
                Err(err) => {
                    let file_entry =
                        if let Some((file_entry, total_bytes)) = self.new_files.lock().get(path) {
                            FileEntry {
                                starting_block: file_entry.starting_block,
                                total_bytes: total_bytes.load(Ordering::Relaxed),
                            }
                        } else if let TantivyError::OpenDirectoryError(
                            OpenDirectoryError::DoesNotExist(missing),
                        ) = err
                        {
                            // Not in the catalog and not an uncommitted write:
                            // surface tantivy's missing-file variant so probing
                            // readers (see `file_entry`) can take their absent
                            // path instead of failing on an opaque IoError.
                            return Err(OpenReadError::FileDoesNotExist(missing));
                        } else {
                            return Err(OpenReadError::IoError {
                                io_error: io::Error::other(err.to_string()).into(),
                                filepath: PathBuf::from(path),
                            });
                        };
                    Ok(vacant
                        .insert(Arc::new(unsafe {
                            SegmentComponentReader::new(
                                &self.indexrel,
                                file_entry,
                                path.extension()
                                    .and_then(|ext| ext.to_str())
                                    .and_then(|ext| SegmentComponent::try_from(ext).ok()),
                            )
                        }))
                        .clone())
                }
            },
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
    fn open_write_inner(&self, path: &Path) -> result::Result<InnerWritePtr, OpenWriteError> {
        let writer = unsafe { SegmentComponentWriter::new(&self.indexrel, path) };
        self.new_files.lock().insert(
            path.to_path_buf(),
            (writer.file_entry(), writer.total_bytes()),
        );
        Ok(Box::new(writer))
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
    /// identified by `<uuid>.<ext>` PathBufs
    fn list_managed_files(&self) -> tantivy::Result<std::collections::HashSet<PathBuf>> {
        unsafe {
            Ok(MetaPage::open(&self.indexrel)
                .segment_metas()
                .list(None)
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
        let mut file_entries = HashMap::default();
        let payload = if let Some(payload) = payload.downcast_mut::<HashMap<PathBuf, FileEntry>>() {
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
        save_schema(&self.indexrel, &meta.schema)
            .map_err(|err| tantivy::TantivyError::SchemaError(err.to_string()))?;

        save_settings(&self.indexrel, &meta.index_settings)
            .map_err(|err| tantivy::TantivyError::InternalError(err.to_string()))?;

        // If there were no new segments, skip the rest of the work
        if meta.segments.is_empty() {
            return Ok(());
        }

        unsafe {
            save_new_metas(&self.indexrel, meta, previous_meta, payload)
                .map_err(|err| tantivy::TantivyError::InternalError(err.to_string()))?;
        }

        Ok(())
    }

    fn load_metas(&self, inventory: &SegmentMetaInventory) -> tantivy::Result<IndexMeta> {
        let loaded_metas = self.loaded_metas.get_or_init(|| unsafe {
            match load_metas(
                &self.indexrel,
                inventory,
                &self.mvcc_style,
                self.indexrel
                    .schema()
                    .unwrap_or_else(|e| panic!("{e}"))
                    .tantivy_schema(),
            ) {
                Err(e) => Err(e),
                Ok(mut loaded) => {
                    let all_entries: HashMap<_, _> = loaded
                        .entries
                        .into_iter()
                        .map(|entry| {
                            let lsme = match entry.content {
                                SegmentMetaEntryContent::Immutable(content) => {
                                    LoadedSegmentMetaEntry::Persisted {
                                        meta: entry,
                                        entry: content,
                                    }
                                }
                                SegmentMetaEntryContent::Mutable(content) => {
                                    LoadedSegmentMetaEntry::Memory {
                                        meta: entry,
                                        tantivy_meta: entry.as_tantivy().track(inventory),
                                        entry: content,
                                        directory: Arc::new(OnceLock::default()),
                                    }
                                }
                            };
                            (entry.segment_id(), lsme)
                        })
                        .collect();

                    match &*self.mvcc_style {
                        // The view came from a reader that already ordered these segments, and
                        // the addresses in flight were packed against that order. Replay it: a
                        // doc-count sort would break ties in list order, and both the list order
                        // and the counts can have moved since the view was captured.
                        MvccSatisfies::ParallelWorker(view) => {
                            loaded.meta.segments.sort_unstable_by_key(|meta| {
                                view.ordinal_of(&meta.id())
                                    .expect("load_metas: segment not in the parallel view")
                            });
                        }
                        // Sort the segments in ascending order by how long we think they'll
                        // take to query.
                        // * for immutable, smallest to largest by document count
                        // * followed by mutable/in-memory, since they're indexed at read time,
                        //   so their doc count is not comparable.
                        //
                        // When segments are claimed by workers they're claimed from
                        // back-to-front and our goal is to have the most expensive segments
                        // claimed first to reduce stragglers.
                        //
                        // TODO: I don't love doing this here, but it's the last place where we
                        // can (easily) determine which segments are memory segments. If we found
                        // a better way to identify a `SegmentReader` as being in-memory, then we
                        // could put it back in parallel worker initialization.
                        _ => {
                            loaded.meta.segments.sort_unstable_by_key(|meta| {
                                match all_entries.get(&meta.id()).unwrap() {
                                    LoadedSegmentMetaEntry::Persisted { meta, .. } => {
                                        meta.num_docs()
                                    }
                                    LoadedSegmentMetaEntry::Memory { meta, .. } => {
                                        (u32::MAX as usize) + meta.num_docs()
                                    }
                                }
                            });
                        }
                    }

                    *self.all_entries.lock() = all_entries;

                    *self.pin_cushion.lock() = Some(loaded.pin_cushion);
                    self.total_segment_count
                        .store(loaded.total_segments, Ordering::Relaxed);
                    self.total_docs.store(loaded.total_docs, Ordering::Relaxed);
                    Ok(loaded.meta)
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

    fn bufwriter_capacity(&self) -> usize {
        BUFWRITER_CAPACITY
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

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// Index the mutable segment to create a RamDirectory with contents corresponding to the given
/// SegmentMeta entry.
///
/// Note that between `prepare` and `index`, additional documents may have been added. We limit the
/// number of contained docs to the `max_doc` value and ignore trailing values.
pub fn index_memory_segment(
    indexrel: &PgSearchRelation,
    segment_meta: &SegmentMeta,
    segment: &SegmentMetaEntry,
    heap_fetch_state: &HeapFetchState,
    expression_state: &ExpressionState,
    mvcc_style: &MvccSatisfies,
) -> anyhow::Result<RamDirectory> {
    use crate::index::writer::index::SerialIndexWriter;
    use crate::postgres::heap::HeapDocFetcher;
    use pgrx::PgTupleDesc;

    /// RAII guard that guarantees an active snapshot for the duration of the heap reads and
    /// detoasting performed while materializing a mutable segment.
    ///
    /// Most callers of [`index_memory_segment`] (ordinary DML, queries, VACUUM, background merge
    /// workers) already run with an active snapshot. The logical-replication apply worker, however,
    /// applies remote changes without pushing one, and `pg_detoast_datum` on an out-of-line TOAST
    /// value requires a snapshot (`get_toast_snapshot` errors otherwise). This guard pushes the
    /// transaction snapshot only when the caller lacks one, and pops it on normal scope exit.
    ///
    /// On a panic / PostgreSQL ERROR the surrounding transaction aborts, which itself resets the
    /// active-snapshot stack; popping again here would underflow it, so the drop implementation
    /// uses `impl_safe_drop!` to skip cleanup while unwinding.
    struct ActiveSnapshotGuard {
        pushed: bool,
    }

    impl ActiveSnapshotGuard {
        unsafe fn ensure() -> Self {
            let pushed = !pg_sys::ActiveSnapshotSet();
            if pushed {
                pg_sys::PushActiveSnapshot(pg_sys::GetTransactionSnapshot());
            }
            Self { pushed }
        }
    }

    crate::impl_safe_drop!(ActiveSnapshotGuard, |self| {
        if self.pushed {
            unsafe {
                pg_sys::PopActiveSnapshot();
            }
        }
    });

    // Ensure an active snapshot for the heap fetches, detoasting, and expression evaluation
    // below. Held until this function returns so it covers the entire materialization loop.
    let _snapshot_guard = unsafe { ActiveSnapshotGuard::ensure() };

    let directory = RamDirectory::create();
    let ctids = segment
        .mutable_snapshot(indexrel)
        .map_err(|e| anyhow::anyhow!("Could not snapshot mutable segment: {e}"))?;

    let mut writer = SerialIndexWriter::in_memory(
        indexrel,
        segment_meta.id(),
        directory.clone(),
        // TODO: Remove argument.
        1337,
    )?;

    let heaprel = indexrel
        .heap_relation()
        .expect("Should have a heap relation.");
    let heaptupdesc = unsafe { PgTupleDesc::from_pg_unchecked(heaprel.rd_att) };
    let search_schema = indexrel.schema()?;
    let categorized_fields = search_schema.categorized_fields();
    let created_by_version = indexrel.created_by_version();

    // Query-visible materialization (Snapshot / ParallelWorker / LargestSegment) fetches each ctid
    // with the active MVCC snapshot; maintenance materialization (e.g. Mergeable) must index every
    // live ctid regardless of any single snapshot. See the `HeapDocFetcher` docs for the full
    // reasoning. Because query-visible materialization happens lazily inside query planning or
    // execution, the active snapshot here is the snapshot that defines query-visible heap rows,
    // so indexing an invisible ctid as empty cannot change a query result or estimate.
    let query_visible = match mvcc_style {
        MvccSatisfies::Snapshot
        | MvccSatisfies::ParallelWorker(_)
        | MvccSatisfies::LargestSegment => true,
        MvccSatisfies::Vacuum | MvccSatisfies::Mergeable => false,
    };

    let mut fetcher = HeapDocFetcher::new(
        heap_fetch_state,
        expression_state,
        &heaprel,
        &heaptupdesc,
        categorized_fields.as_slice(),
        created_by_version,
        query_visible,
    );

    for ctid in ctids {
        // A ctid with nothing to index becomes an empty document: the segment must contain
        // every ctid in the mutable snapshot.
        let doc = unsafe { fetcher.fetch_doc(ctid) }.unwrap_or_else(tantivy::TantivyDocument::new);
        writer.insert(doc, ctid, || {
            unreachable!("No limits configured: should not finalize.")
        })?;
    }

    writer.finalize_nocommit()?.expect(
        "Segment should be non-empty because we created a `SegmentMeta` for it in the first place.",
    );

    Ok(directory)
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;

    use crate::postgres::rel::PgSearchRelation;
    use crate::postgres::storage::block::SegmentMetaEntryContent;

    use pgrx::prelude::*;

    #[pg_test]
    unsafe fn test_list_meta_entries() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("INSERT INTO t (data) VALUES ('test');").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();
        let indexrel = PgSearchRelation::open(relation_oid);
        let linked_list = MetaPage::open(&indexrel).segment_metas();
        let mut listed_files = unsafe { linked_list.list(None) };
        assert_eq!(listed_files.len(), 1);
        let entry = listed_files.pop().unwrap();
        let SegmentMetaEntryContent::Immutable(entry) = entry.content else {
            todo!("test_list_meta_entries");
        };
        assert!(entry.field_norms.is_some());
        assert!(entry.fast_fields.is_some());
        assert!(entry.postings.is_some());
        assert!(entry.positions.is_some());
        assert!(entry.terms.is_some());
        assert!(entry.delete.is_none());
    }

    /// A reader replaying a [`SegmentView`] must expose the view's ordinal order and, for a
    /// mutable segment, the document set the view's origin materialized, even after the
    /// segment's log has grown. This is what keeps packed `DocAddress`es comparable across
    /// the readers a JoinScan opens in different processes.
    #[pg_test]
    unsafe fn test_segment_view_replays_origin_reader() {
        use crate::index::reader::index::SearchIndexReader;

        Spi::run("CREATE TABLE t (id SERIAL8, data TEXT);").unwrap();
        // Two immutable segments of unequal size, then a small batch that lands in the
        // mutable segment.
        Spi::run("SET paradedb.global_mutable_segment_rows = 0;").unwrap();
        Spi::run(
            "CREATE INDEX t_idx ON t USING paradedb (id, data) \
             WITH (key_field = 'id', layer_sizes = '0', background_layer_sizes = '0')",
        )
        .unwrap();
        Spi::run("INSERT INTO t (data) SELECT 'a' FROM generate_series(1, 30);").unwrap();
        Spi::run("INSERT INTO t (data) SELECT 'b' FROM generate_series(1, 10);").unwrap();
        Spi::run("RESET paradedb.global_mutable_segment_rows;").unwrap();
        Spi::run("INSERT INTO t (data) SELECT 'c' FROM generate_series(1, 5);").unwrap();

        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();
        let indexrel = PgSearchRelation::open(relation_oid);

        let origin =
            SearchIndexReader::empty(&indexrel, MvccSatisfies::Snapshot).expect("open origin");
        let view = origin.segment_view();
        assert_eq!(view.len(), 3);
        let mutable: Vec<_> = view
            .entries()
            .iter()
            .filter(|e| e.mutable_bound().is_some())
            .collect();
        assert_eq!(mutable.len(), 1, "one mutable segment expected");
        assert_eq!(mutable[0].max_doc(), 5);
        let origin_ids: Vec<_> = origin
            .segment_readers()
            .iter()
            .map(|r| r.segment_id())
            .collect();

        // The mutable segment's log grows past the captured view.
        Spi::run("INSERT INTO t (data) SELECT 'd' FROM generate_series(1, 7);").unwrap();
        let fresh =
            SearchIndexReader::empty(&indexrel, MvccSatisfies::Snapshot).expect("open fresh");
        let fresh_mutable = fresh
            .segment_readers()
            .iter()
            .find(|r| r.segment_id() == mutable[0].id)
            .expect("mutable segment still there");
        assert_eq!(
            fresh_mutable.max_doc(),
            12,
            "a plain snapshot sees the new rows"
        );

        // A replaying reader sees what the origin saw.
        let replay = SearchIndexReader::empty(&indexrel, MvccSatisfies::ParallelWorker(view))
            .expect("open replay");
        let replay_ids: Vec<_> = replay
            .segment_readers()
            .iter()
            .map(|r| r.segment_id())
            .collect();
        assert_eq!(replay_ids, origin_ids, "ordinal order replays the origin");
        for (o, r) in origin
            .segment_readers()
            .iter()
            .zip(replay.segment_readers())
        {
            assert_eq!(
                o.max_doc(),
                r.max_doc(),
                "segment {} doc space",
                o.segment_id()
            );
        }
    }
}
