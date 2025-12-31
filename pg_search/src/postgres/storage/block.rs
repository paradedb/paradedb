// Copyright (C) 2023-2026 ParadeDB, Inc.
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

use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::mem::{offset_of, size_of};
use std::path::{Path, PathBuf};
use std::slice::from_raw_parts;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::api::HashSet;
use crate::postgres::storage::buffer::{Buffer, BufferManager, BufferMut};
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use crate::postgres::PgSearchRelation;

use pgrx::*;
use serde::{Deserialize, Serialize};
use tantivy::index::{SegmentComponent, SegmentId};
use tantivy::Opstamp;

// ---------------------------------------------------------
// BM25 page special data
// ---------------------------------------------------------

// Struct for all page's LP_SPECIAL data
#[derive(Clone, Debug)]
pub struct BM25PageSpecialData {
    pub next_blockno: pg_sys::BlockNumber,
    pub xmax: pg_sys::TransactionId,
}

// ---------------------------------------------------------
// Linked lists
// ---------------------------------------------------------

/// Struct held in the first buffer of every linked list's content area
#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct LinkedListData {
    /// Indicates the first BlockNumber of the linked list.
    pub start_blockno: pg_sys::BlockNumber,

    /// Indicates the last BlockNumber of the linked list.
    pub last_blockno: pg_sys::BlockNumber,

    /// This once tracked the number of blocks in the linked list, but now it's just dead space
    #[doc(hidden)]
    _dead_space: u32,

    /// Indicates where the BlockList for this linked list starts;
    pub blocklist_start: pg_sys::BlockNumber,
}

impl Debug for LinkedListData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkedListData")
            .field("start_blockno", &{ self.start_blockno })
            .field("last_blockno", &{ self.last_blockno })
            .field("blocklist_start", &{ self.blocklist_start })
            .finish()
    }
}

/// Every linked list must implement this trait
pub trait LinkedList {
    fn get_header_blockno(&self) -> pg_sys::BlockNumber;

    fn bman(&self) -> &BufferManager;

    fn bman_mut(&mut self) -> &mut BufferManager;

    ///
    /// Get the start blockno of the LinkedList, and return a Buffer for the header block of
    /// the list, which must be held until the start blockno is actually dereferenced.
    ///
    fn get_start_blockno(&self) -> (pg_sys::BlockNumber, Buffer) {
        let buffer = self.bman().get_buffer(self.get_header_blockno());
        let metadata = buffer.page().contents::<LinkedListData>();
        let start_blockno = metadata.start_blockno;
        assert!(start_blockno != 0);
        assert!(start_blockno != pg_sys::InvalidBlockNumber);
        (start_blockno, buffer)
    }

    ///
    /// See `get_start_blockno`.
    ///
    fn get_start_blockno_mut(&mut self) -> (pg_sys::BlockNumber, BufferMut) {
        let header_blockno = self.get_header_blockno();
        let buffer = self.bman_mut().get_buffer_mut(header_blockno);
        let metadata = buffer.page().contents::<LinkedListData>();
        let start_blockno = metadata.start_blockno;
        assert!(start_blockno != 0);
        assert!(start_blockno != pg_sys::InvalidBlockNumber);
        (start_blockno, buffer)
    }

    fn get_last_blockno(&self) -> pg_sys::BlockNumber {
        // TODO: If concurrency is a concern for "append" cases, then we'd want to iterate from the
        // hand-over-hand from the head to the tail rather than jumping immediately to the tail.
        let buffer = self.bman().get_buffer(self.get_header_blockno());
        let metadata = buffer.page().contents::<LinkedListData>();
        let last_blockno = metadata.last_blockno;
        assert!(last_blockno != 0);
        assert!(last_blockno != pg_sys::InvalidBlockNumber);
        last_blockno
    }

    fn block_for_ord(&self, ord: usize) -> Option<pg_sys::BlockNumber>;

    ///
    /// Note: It is not safe to begin iteration of the list using this method, as the buffer for
    /// the metadata is released when it returns. Use `get_start_blockno` to begin iteration.
    fn get_linked_list_data(&self) -> LinkedListData {
        self.bman()
            .get_buffer(self.get_header_blockno())
            .page()
            .contents::<LinkedListData>()
    }
}

// ---------------------------------------------------------
// Linked list entry structs
// ---------------------------------------------------------

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum MutableSegmentEntry {
    Add(u64),
    Remove(u64),
}

impl From<MutableSegmentEntry> for PgItem {
    fn from(val: MutableSegmentEntry) -> Self {
        let mut buf = pgrx::StringInfo::new();
        let len = bincode::serde::encode_into_std_write(val, &mut buf, bincode::config::legacy())
            .expect("expected to serialize valid MutableSegmentEntry");
        PgItem(buf.into_char_ptr() as pg_sys::Item, len as pg_sys::Size)
    }
}

impl From<PgItem> for MutableSegmentEntry {
    fn from(pg_item: PgItem) -> Self {
        let PgItem(item, size) = pg_item;
        let (decoded, _) = bincode::serde::decode_from_slice(
            unsafe { from_raw_parts(item as *const u8, size) },
            bincode::config::legacy(),
        )
        .expect("expected to deserialize valid MutableSegmentEntry");
        decoded
    }
}

/// Prior to https://github.com/paradedb/paradedb/pull/2487, the field storing this tag contained
/// either A. FrozenTransactionId, or B. GetCurrentTransactionId(). After #2487 and before #3203,
/// the field contained `InvalidTransactionId`. We treat all such tags as representing an
/// `Immutable` segment in `impl From<u32> for Self`.
///
/// New tags added here must therefore _not_ be valid transaction ids. At some point in the future,
/// if we decide to drop compatibility with indexes created before #2487, then we could begin to
/// use valid transaction ids as tags.
#[repr(u32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(from = "u32", into = "u32")]
enum SegmentMetaEntryTag {
    /// Tag for a mutable segment (reserved due to `BootstrapTransactionId`).
    Mutable = 1,
    /// Tag for an immutable segment (reserved due to `FrozenTransactionId`).
    Immutable = 2,
}

impl From<u32> for SegmentMetaEntryTag {
    fn from(value: u32) -> Self {
        // See the `SegmentMetaEntryTag` docs.
        let txnid_value: pg_sys::TransactionId = value.into();
        if value == SegmentMetaEntryTag::Immutable as u32
            || txnid_value == pg_sys::InvalidTransactionId
            || txnid_value >= pg_sys::FirstNormalTransactionId
        {
            SegmentMetaEntryTag::Immutable
        } else if value == SegmentMetaEntryTag::Mutable as u32 {
            SegmentMetaEntryTag::Mutable
        } else {
            panic!("Expected a SegmentMetaEntryTag, got: {value}");
        }
    }
}

impl From<SegmentMetaEntryTag> for u32 {
    fn from(value: SegmentMetaEntryTag) -> Self {
        value as u32
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
struct SegmentMetaEntryHeader {
    segment_id: SegmentId,
    max_doc: u32,

    /// Previously known as `xmin` and `_unused`: now used to store a tag which differentiates the
    /// type of SegmentMetaEntry this is. See the method doc.
    tag: SegmentMetaEntryTag,

    /// If set to [`pg_sys::FrozenTransactionId`] then this entry has been deleted via a Tantivy merge
    /// and a) is no longer visible to any transaction and b) is subject to being garbage collected
    xmax: pg_sys::TransactionId,
}

/// Metadata for tracking where to find a file
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct FileEntry {
    pub starting_block: pg_sys::BlockNumber,
    pub total_bytes: usize,
}

/// Metadata for tracking where to find a ".del" file
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeleteEntry {
    pub file_entry: FileEntry,
    pub num_deleted_docs: u32,
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct SegmentMetaEntryMutable {
    pub header_block: pg_sys::BlockNumber,
    pub num_deleted_docs: u32,
    // Once a mutable segment reaches a configurable size threshold, it is frozen, and becomes
    // mergeable.
    pub frozen: bool,
}

impl SegmentMetaEntryMutable {
    pub fn pintest_blockno(&self) -> pg_sys::BlockNumber {
        // NOTE: Using the header blockno here rather than the starting blockno. Should be
        // fine?
        self.header_block
    }

    pub fn create(indexrel: &PgSearchRelation) -> (Self, LinkedItemList<MutableSegmentEntry>) {
        let items = LinkedItemList::create_with_fsm(indexrel);
        let self_ = Self {
            header_block: items.get_header_blockno(),
            num_deleted_docs: 0,
            frozen: false,
        };
        (self_, items)
    }

    pub fn open(&self, indexrel: &PgSearchRelation) -> LinkedItemList<MutableSegmentEntry> {
        LinkedItemList::open(indexrel, self.header_block)
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct SegmentMetaEntryImmutable {
    pub postings: Option<FileEntry>,
    pub positions: Option<FileEntry>,
    pub fast_fields: Option<FileEntry>,
    pub field_norms: Option<FileEntry>,
    pub terms: Option<FileEntry>,
    pub store: Option<FileEntry>,
    pub temp_store: Option<FileEntry>,
    pub delete: Option<DeleteEntry>,
}

impl SegmentMetaEntryImmutable {
    pub fn path(uuid: &str, component: SegmentComponent) -> PathBuf {
        if matches!(component, SegmentComponent::Delete) {
            PathBuf::from(format!(
                "{}.0.{}", // we can hardcode zero as the opstamp component of the path as it's not used by anyone
                uuid,
                SegmentComponent::Delete
            ))
        } else {
            PathBuf::from(format!("{uuid}.{component}"))
        }
    }

    pub fn pintest_blockno(&self) -> pg_sys::BlockNumber {
        match self.file_entries().next() {
            None => panic!("SegmentMetaEntry has no files"),
            Some((file_entry, _)) => file_entry.starting_block,
        }
    }

    pub fn file_entry(&self, uuid: &str, path: &Path) -> Option<FileEntry> {
        for (file_entry, component) in self.file_entries() {
            if path == Self::path(uuid, component) {
                return Some(*file_entry);
            }
        }
        None
    }

    pub fn file_entries(&self) -> impl Iterator<Item = (&FileEntry, SegmentComponent)> {
        self.postings
            .iter()
            .map(|fe| (fe, SegmentComponent::Postings))
            .chain(
                self.positions
                    .iter()
                    .map(|fe| (fe, SegmentComponent::Positions)),
            )
            .chain(
                self.fast_fields
                    .iter()
                    .map(|fe| (fe, SegmentComponent::FastFields)),
            )
            .chain(
                self.field_norms
                    .iter()
                    .map(|fe| (fe, SegmentComponent::FieldNorms)),
            )
            .chain(self.terms.iter().map(|fe| (fe, SegmentComponent::Terms)))
            .chain(
                self.temp_store
                    .iter()
                    .map(|fe| (fe, SegmentComponent::TempStore)),
            )
            .chain(
                self.delete
                    .as_ref()
                    .map(|d| (&d.file_entry, SegmentComponent::Delete)),
            )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum SegmentMetaEntryContent {
    Immutable(SegmentMetaEntryImmutable),
    Mutable(SegmentMetaEntryMutable),
}

/// Metadata for tracking alive segments
///
/// NOTE: Implements Serialize for output-only use in the admin APIs: for internal storage, is
/// serialized/deserialized via `Into/From<PgItem>`.
#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub struct SegmentMetaEntry {
    header: SegmentMetaEntryHeader,
    pub content: SegmentMetaEntryContent,
}

impl SegmentMetaEntry {
    pub fn new_mutable(
        segment_id: SegmentId,
        max_doc: u32,
        xmax: pg_sys::TransactionId,
        content: SegmentMetaEntryMutable,
    ) -> Self {
        Self {
            header: SegmentMetaEntryHeader {
                segment_id,
                max_doc,
                tag: SegmentMetaEntryTag::Mutable,
                xmax,
            },
            content: SegmentMetaEntryContent::Mutable(content),
        }
    }

    pub fn new_immutable(
        segment_id: SegmentId,
        max_doc: u32,
        xmax: pg_sys::TransactionId,
        content: SegmentMetaEntryImmutable,
    ) -> Self {
        Self {
            header: SegmentMetaEntryHeader {
                segment_id,
                max_doc,
                tag: SegmentMetaEntryTag::Immutable,
                xmax,
            },
            content: SegmentMetaEntryContent::Immutable(content),
        }
    }

    pub fn segment_id(&self) -> SegmentId {
        self.header.segment_id
    }

    pub fn max_doc(&self) -> u32 {
        self.header.max_doc
    }

    pub fn is_mutable(&self) -> bool {
        matches!(&self.content, SegmentMetaEntryContent::Mutable(_))
    }

    /// If this is a mutable segment which is not frozen, add the given items; otherwise, return an
    /// error.
    pub fn mutable_add_items(
        &mut self,
        indexrel: &PgSearchRelation,
        items: &[MutableSegmentEntry],
    ) -> Result<(), &str> {
        let items_len: u32 = items.len().try_into().unwrap();
        let new_max_doc = self.header.max_doc + items_len;
        match &mut self.content {
            SegmentMetaEntryContent::Mutable(content) if !content.frozen => {
                unsafe { content.open(indexrel).add_items(items, None) };
                let row_limit = indexrel
                    .options()
                    .mutable_segment_rows()
                    .map(|v| v.get())
                    .unwrap_or(0);
                if new_max_doc as usize >= row_limit {
                    content.frozen = true;
                }
            }
            _ => return Err("Cannot add items to a non-mutable segment"),
        }
        self.header.max_doc = new_max_doc;
        Ok(())
    }

    /// If this is a mutable segment which is not frozen, delete the given items; otherwise, return an
    /// error.
    pub fn mutable_delete_items(
        &mut self,
        indexrel: &PgSearchRelation,
        ctids: Vec<u64>,
    ) -> Result<(), &str> {
        let SegmentMetaEntryContent::Mutable(ref mut content) = &mut self.content else {
            return Err("Cannot delete items from a non-mutable segment");
        };

        let entries = ctids
            .into_iter()
            .map(MutableSegmentEntry::Remove)
            .collect::<Vec<_>>();

        unsafe {
            content.open(indexrel).add_items(&entries, None);
        }
        let deleted: u32 = entries.len().try_into().unwrap();
        content.num_deleted_docs += deleted;

        Ok(())
    }

    /// Return a snapshot of the ctids which were valid when this SegmentMetaEntry was opened.
    pub fn mutable_snapshot(&self, indexrel: &PgSearchRelation) -> Result<Vec<u64>, &str> {
        let SegmentMetaEntryContent::Mutable(ref content) = &self.content else {
            return Err("Cannot snapshot a non-mutable segment");
        };

        let entries = unsafe {
            content
                .open(indexrel)
                .list(Some(self.max_doc() as usize + self.num_deleted_docs()))
        };

        // The mutable segment is composed of Adds and Removes in some order: in order to align with
        // the snapshot of the SegmentMeta entry as it existed when we opened it, we should only
        // consume a prefix of all entries which might now exist (since we have not been holding a lock
        // on it).
        let snapshot_entries = self.header.max_doc as usize + content.num_deleted_docs as usize;
        let expected_ctids = self.header.max_doc - content.num_deleted_docs;

        let mut ctid_set =
            HashSet::with_capacity_and_hasher(expected_ctids as usize, rustc_hash::FxBuildHasher);
        for entry in entries.into_iter().take(snapshot_entries) {
            match entry {
                MutableSegmentEntry::Add(ctid) => ctid_set.insert(ctid),
                MutableSegmentEntry::Remove(ctid) => ctid_set.remove(&ctid),
            };
        }
        assert_eq!(ctid_set.len(), expected_ctids as usize);

        let mut ctids: Vec<_> = ctid_set.into_iter().collect();
        ctids.sort_unstable();
        Ok(ctids)
    }

    pub fn xmax(&self) -> pg_sys::TransactionId {
        self.header.xmax
    }

    pub fn set_xmax(&mut self, xmax: pg_sys::TransactionId) {
        self.header.xmax = xmax;
    }

    pub fn is_deleted(&self) -> bool {
        self.xmax() == pg_sys::FrozenTransactionId
    }

    /// In `save_new_metas`, if a new `DeleteEntry` is created and there was already a `DeleteEntry`,
    /// we create a "fake" `SegmentMetaEntry` that stores the old `DeleteEntry` so it can be garbage collected
    /// later.
    ///
    /// This function returns true if the `SegmentMetaEntry` is a "fake" `DeleteEntry`
    pub fn is_orphaned_delete(&self) -> bool {
        self.segment_id() == SegmentId::from_bytes([0; 16])
    }

    /// Fake an `Opstamp` that's always zero
    pub fn opstamp(&self) -> Opstamp {
        0
    }

    pub fn num_docs(&self) -> usize {
        self.max_doc() as usize - self.num_deleted_docs()
    }

    pub fn num_deleted_docs(&self) -> usize {
        match self.content {
            SegmentMetaEntryContent::Immutable(content) => content
                .delete
                .map(|entry| entry.num_deleted_docs as usize)
                .unwrap_or(0),
            SegmentMetaEntryContent::Mutable(content) => content.num_deleted_docs as usize,
        }
    }

    pub unsafe fn is_mergeable(&self, indexrel: &PgSearchRelation) -> bool {
        // mergeable if we haven't deleted it, and if it isn't a mutable segment which is still
        // receiving writes.
        let immutable_or_mergable = |content| match content {
            SegmentMetaEntryContent::Mutable(content) => {
                content.frozen || indexrel.options().mutable_segment_rows().is_none()
            }
            SegmentMetaEntryContent::Immutable(_) => true,
        };

        !self.is_deleted() && immutable_or_mergable(self.content)
    }

    pub fn as_tantivy(&self) -> tantivy::index::InnerSegmentMeta {
        let (max_doc, deletes) = match self.content {
            SegmentMetaEntryContent::Immutable(content) => {
                let deletes = content
                    .delete
                    .map(|delete_entry| tantivy::index::DeleteMeta {
                        num_deleted_docs: delete_entry.num_deleted_docs,
                        opstamp: 0, // hardcode zero as the entry's opstamp as it's not used
                    });
                (self.header.max_doc, deletes)
            }
            SegmentMetaEntryContent::Mutable(content) => {
                // NOTE: Rather than ever claiming to contain deletes, we reduce the max doc count.
                // At indexing time, we skip deleted entries.
                (self.header.max_doc - content.num_deleted_docs, None)
            }
        };

        tantivy::index::InnerSegmentMeta {
            segment_id: self.segment_id(),
            max_doc,
            deletes,
            include_temp_doc_store: Arc::new(AtomicBool::new(false)),
        }
    }

    /// If this entry already has DeleteEntry, return an "orphaned deletes" clone. Finally,
    /// replace the deletes with the given DeleteEntry.
    pub fn replace_deletes(&mut self, delete: DeleteEntry) -> Option<Self> {
        let max_doc = self.max_doc();
        match &mut self.content {
            SegmentMetaEntryContent::Immutable(content) => {
                let result = if content.delete.is_some() {
                    // Create an "orphaned deletes" clone of the segment.
                    Some(SegmentMetaEntry::new_immutable(
                        SegmentId::from_bytes([0; 16]), // all zeros
                        max_doc,
                        pg_sys::FrozenTransactionId, // immediately recyclable
                        *content,
                    ))
                } else {
                    None
                };
                content.delete = Some(delete);
                result
            }
            SegmentMetaEntryContent::Mutable(_) => {
                // FIXME: See `delete.rs`: this codepath is not reachable, but we should adjust the
                // interface of `save_new_metas` to do the delete replacement more directly
                // to improve type safety.
                unreachable!("replace_deletes for a mutable segment");
            }
        }
    }

    pub fn byte_size(&self) -> u64 {
        let content = match &self.content {
            SegmentMetaEntryContent::Immutable(content) => content,
            SegmentMetaEntryContent::Mutable(content) => {
                // TODO: Guesstimate. Most likely the byte_size should be made optional so that
                // merging is forced to consider mutable segments separately.
                return (self.header.max_doc as u64 * 1000)
                    + (content.num_deleted_docs as u64 * 10);
            }
        };

        let mut size = 0;

        size += content
            .postings
            .as_ref()
            .map(|entry| entry.total_bytes as u64)
            .unwrap_or(0);
        size += content
            .positions
            .as_ref()
            .map(|entry| entry.total_bytes as u64)
            .unwrap_or(0);
        size += content
            .fast_fields
            .as_ref()
            .map(|entry| entry.total_bytes as u64)
            .unwrap_or(0);
        size += content
            .field_norms
            .as_ref()
            .map(|entry| entry.total_bytes as u64)
            .unwrap_or(0);
        size += content
            .terms
            .as_ref()
            .map(|entry| entry.total_bytes as u64)
            .unwrap_or(0);
        size += content
            .store
            .as_ref()
            .map(|entry| entry.total_bytes as u64)
            .unwrap_or(0);
        size += content
            .temp_store
            .as_ref()
            .map(|entry| entry.total_bytes as u64)
            .unwrap_or(0);
        size += content
            .delete
            .as_ref()
            .map(|entry| entry.file_entry.total_bytes as u64)
            .unwrap_or(0);
        size
    }

    pub fn get_component_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        let iter: Box<dyn Iterator<Item = PathBuf>> =
            match self.content {
                SegmentMetaEntryContent::Immutable(ref content) => {
                    let uuid = self.segment_id().uuid_string();
                    Box::new(content.file_entries().map(move |(_, component)| {
                        SegmentMetaEntryImmutable::path(&uuid, component)
                    }))
                }
                SegmentMetaEntryContent::Mutable(_) => Box::new(std::iter::empty()),
            };
        iter
    }

    pub fn freeable_blocks<'a>(
        &'a self,
        indexrel: &'a PgSearchRelation,
    ) -> impl Iterator<Item = pg_sys::BlockNumber> + 'a {
        let iter: Box<dyn Iterator<Item = pg_sys::BlockNumber>> = match self.content {
            SegmentMetaEntryContent::Immutable(ref content) if self.is_orphaned_delete() => {
                // This is a "fake" `DeleteEntry`: free the blocks for the old `DeleteEntry` only
                let block = content.delete.as_ref().unwrap().file_entry.starting_block;
                Box::new(LinkedBytesList::open(indexrel, block).freeable_blocks())
            }
            SegmentMetaEntryContent::Immutable(ref content) => {
                // Free all files.
                Box::new(content.file_entries().flat_map(move |(file_entry, _)| {
                    LinkedBytesList::open(indexrel, file_entry.starting_block).freeable_blocks()
                }))
            }
            SegmentMetaEntryContent::Mutable(ref content) => {
                // Free the content list.
                Box::new(content.open(indexrel).freeable_blocks())
            }
        };
        iter
    }
}

// ---------------------------------------------------------
// Linked list entry <-> PgItem
// ---------------------------------------------------------

/// Wrapper for pg_sys::Item that also stores its size
#[derive(Clone)]
pub struct PgItem(pub pg_sys::Item, pub pg_sys::Size);

impl From<SegmentMetaEntry> for PgItem {
    fn from(val: SegmentMetaEntry) -> Self {
        let mut buf = pgrx::StringInfo::new();

        let mut len =
            bincode::serde::encode_into_std_write(val.header, &mut buf, bincode::config::legacy())
                .expect("expected to serialize valid SegmentMetaEntryHeader");

        len += match &val.content {
            SegmentMetaEntryContent::Immutable(content) => {
                debug_assert!(val.header.tag == SegmentMetaEntryTag::Immutable);
                bincode::serde::encode_into_std_write(content, &mut buf, bincode::config::legacy())
                    .expect("expected to serialize valid SegmentMetaEntryContent::Immutable")
            }
            SegmentMetaEntryContent::Mutable(content) => {
                debug_assert!(val.header.tag == SegmentMetaEntryTag::Mutable);
                bincode::serde::encode_into_std_write(content, &mut buf, bincode::config::legacy())
                    .expect("expected to serialize valid SegmentMetaEntryContent::Mutable")
            }
        };

        PgItem(buf.into_char_ptr() as pg_sys::Item, len as pg_sys::Size)
    }
}

impl From<PgItem> for SegmentMetaEntry {
    fn from(pg_item: PgItem) -> Self {
        let PgItem(item, size) = pg_item;
        let bytes = unsafe { from_raw_parts(item as *const u8, size) };

        let (header, bytes_read): (SegmentMetaEntryHeader, _) =
            bincode::serde::decode_from_slice(bytes, bincode::config::legacy())
                .expect("expected to deserialize valid SegmentMetaEntryHeader");

        let content = match header.tag {
            SegmentMetaEntryTag::Immutable => {
                let (content, _): (SegmentMetaEntryImmutable, _) =
                    bincode::serde::decode_from_slice(
                        &bytes[bytes_read..],
                        bincode::config::legacy(),
                    )
                    .expect("expected to deserialize valid SegmentMetaEntryContent");
                SegmentMetaEntryContent::Immutable(content)
            }
            SegmentMetaEntryTag::Mutable => {
                let (content, _): (SegmentMetaEntryMutable, _) = bincode::serde::decode_from_slice(
                    &bytes[bytes_read..],
                    bincode::config::legacy(),
                )
                .expect("expected to deserialize valid SegmentMetaEntryContent");
                SegmentMetaEntryContent::Mutable(content)
            }
        };

        SegmentMetaEntry { header, content }
    }
}

pub trait SegmentFileDetails {
    fn segment_id(&self) -> Option<SegmentId>;
    fn component_type(&self) -> Option<SegmentComponent>;
}

impl<T: AsRef<Path>> SegmentFileDetails for T {
    fn segment_id(&self) -> Option<SegmentId> {
        let mut parts = self.as_ref().file_name()?.to_str()?.split('.');
        SegmentId::from_uuid_string(parts.next()?).ok()
    }

    fn component_type(&self) -> Option<SegmentComponent> {
        let mut parts = self.as_ref().file_name()?.to_str()?.split('.');
        let _ = parts.next()?; // skip segment id
        let mut extension = parts.next()?;
        if let Some(last) = parts.next() {
            // it has three parts, so the extension is instead the last part
            extension = last;
        }
        SegmentComponent::try_from(extension).ok()
    }
}

// ---------------------------------------------------------
// Linked list entry MVCC methods
// ---------------------------------------------------------

pub trait MVCCEntry {
    fn pintest_blockno(&self) -> pg_sys::BlockNumber;

    // Provided methods
    unsafe fn visible(&self) -> bool;

    unsafe fn recyclable(&self, bman: &mut BufferManager) -> bool;
}

impl MVCCEntry for SegmentMetaEntry {
    fn pintest_blockno(&self) -> pg_sys::BlockNumber {
        match self.content {
            SegmentMetaEntryContent::Immutable(ref content) => content.pintest_blockno(),
            SegmentMetaEntryContent::Mutable(ref content) => content.pintest_blockno(),
        }
    }

    unsafe fn visible(&self) -> bool {
        // visible if we haven't deleted it
        !self.is_deleted()
    }

    unsafe fn recyclable(&self, bman: &mut BufferManager) -> bool {
        // recyclable if we've deleted it
        self.is_deleted()

        // and there's no pin on our pintest buffer, assuming we have a valid buffer
        && (self.pintest_blockno() == pg_sys::InvalidBlockNumber || bman.get_buffer_for_cleanup_conditional(self.pintest_blockno()).is_some())
    }
}

pub const fn bm25_max_free_space() -> usize {
    unsafe {
        (pg_sys::BLCKSZ as usize)
            - pg_sys::MAXALIGN(size_of::<BM25PageSpecialData>())
            - pg_sys::MAXALIGN(offset_of!(pg_sys::PageHeaderData, pd_linp))
    }
}

#[inline(always)]
pub fn block_number_is_valid(block_number: pg_sys::BlockNumber) -> bool {
    block_number != 0 && block_number != pg_sys::InvalidBlockNumber
}
