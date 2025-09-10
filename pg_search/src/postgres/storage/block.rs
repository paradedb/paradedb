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

use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::mem::{offset_of, size_of};
use std::path::{Path, PathBuf};
use std::slice::from_raw_parts;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::postgres::storage::buffer::{Buffer, BufferManager, BufferMut};
use crate::postgres::storage::LinkedBytesList;
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

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum SegmentMetaEntryContent {
    Immutable(SegmentMetaEntryImmutable),
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
        // TODO: Stop using this value for mutable segments.
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
        }
    }

    pub fn as_tantivy(&self) -> tantivy::index::InnerSegmentMeta {
        let deletes = match self.content {
            SegmentMetaEntryContent::Immutable(content) => {
                content
                    .delete
                    .map(|delete_entry| tantivy::index::DeleteMeta {
                        num_deleted_docs: delete_entry.num_deleted_docs,
                        opstamp: 0, // hardcode zero as the entry's opstamp as it's not used
                    })
            }
        };

        tantivy::index::InnerSegmentMeta {
            segment_id: self.segment_id(),
            max_doc: self.max_doc(),
            deletes,
            include_temp_doc_store: Arc::new(AtomicBool::new(false)),
        }
    }

    /// If this entry already has DeleteEntry, clone the entire entry and return it. Finally,
    /// replace the deletes with the given DeleteEntry.
    pub fn replace_deletes(&mut self, delete: DeleteEntry) -> Option<Self> {
        let cloned = *self;
        match &mut self.content {
            SegmentMetaEntryContent::Immutable(content) => {
                let result = if content.delete.is_some() {
                    Some(cloned)
                } else {
                    None
                };
                content.delete = Some(delete);
                result
            }
        }
    }

    pub fn file_entry(&self, path: &Path) -> Option<FileEntry> {
        for (file_path, (file_entry, _)) in self.get_component_paths().zip(self.file_entries()) {
            if path == file_path {
                return Some(*file_entry);
            }
        }
        None
    }

    pub fn file_entries(&self) -> impl Iterator<Item = (&FileEntry, SegmentComponent)> {
        let SegmentMetaEntryContent::Immutable(ref content) = self.content;

        content
            .postings
            .iter()
            .map(|fe| (fe, SegmentComponent::Postings))
            .chain(
                content
                    .positions
                    .iter()
                    .map(|fe| (fe, SegmentComponent::Positions)),
            )
            .chain(
                content
                    .fast_fields
                    .iter()
                    .map(|fe| (fe, SegmentComponent::FastFields)),
            )
            .chain(
                content
                    .field_norms
                    .iter()
                    .map(|fe| (fe, SegmentComponent::FieldNorms)),
            )
            .chain(content.terms.iter().map(|fe| (fe, SegmentComponent::Terms)))
            .chain(
                content
                    .temp_store
                    .iter()
                    .map(|fe| (fe, SegmentComponent::TempStore)),
            )
            .chain(
                content
                    .delete
                    .as_ref()
                    .map(|d| (&d.file_entry, SegmentComponent::Delete)),
            )
    }

    pub fn byte_size(&self) -> u64 {
        let SegmentMetaEntryContent::Immutable(ref content) = self.content;

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
        let uuid = self.segment_id().uuid_string();
        self.file_entries().map(move |(_, component)| {
            if matches!(component, SegmentComponent::Delete) {
                PathBuf::from(format!(
                    "{}.0.{}", // we can hardcode zero as the opstamp component of the path as it's not used by anyone
                    uuid,
                    SegmentComponent::Delete
                ))
            } else {
                PathBuf::from(format!("{uuid}.{component}"))
            }
        })
    }

    pub fn freeable_blocks<'a>(
        &'a self,
        indexrel: &'a PgSearchRelation,
    ) -> impl Iterator<Item = pg_sys::BlockNumber> + 'a {
        let iter: Box<dyn Iterator<Item = pg_sys::BlockNumber>> = if self.is_orphaned_delete() {
            match self.content {
                SegmentMetaEntryContent::Immutable(content) => {
                    // This is a "fake" `DeleteEntry`: free the blocks for the old `DeleteEntry` only
                    let block = content.delete.as_ref().unwrap().file_entry.starting_block;
                    Box::new(LinkedBytesList::open(indexrel, block).freeable_blocks())
                }
            }
        } else {
            // Otherwise, we need to free the blocks for all the files.
            Box::new(self.file_entries().flat_map(move |(file_entry, _)| {
                LinkedBytesList::open(indexrel, file_entry.starting_block).freeable_blocks()
            }))
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
                bincode::serde::encode_into_std_write(content, &mut buf, bincode::config::legacy())
                    .expect("expected to serialize valid SegmentMetaEntryContent")
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

    unsafe fn mergeable(&self) -> bool;
}

impl MVCCEntry for SegmentMetaEntry {
    fn pintest_blockno(&self) -> pg_sys::BlockNumber {
        match self.file_entries().next() {
            None => panic!("SegmentMetaEntry for `{}` has no files", self.segment_id()),
            Some((file_entry, _)) => file_entry.starting_block,
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

    unsafe fn mergeable(&self) -> bool {
        // mergeable if we haven't deleted it
        !self.is_deleted()
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
