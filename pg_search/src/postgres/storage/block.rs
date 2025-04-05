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

use crate::postgres::storage::buffer::BufferManager;
use pgrx::pg_sys::BlockNumber;
use pgrx::*;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::mem::{offset_of, size_of};
use std::path::{Path, PathBuf};
use std::slice::from_raw_parts;
use tantivy::index::{SegmentComponent, SegmentId};
use tantivy::Opstamp;

pub const MERGE_LOCK: pg_sys::BlockNumber = 0;
pub const CLEANUP_LOCK: pg_sys::BlockNumber = 1;
pub const SCHEMA_START: pg_sys::BlockNumber = 2;
pub const SETTINGS_START: pg_sys::BlockNumber = 4;
pub const SEGMENT_METAS_START: pg_sys::BlockNumber = 6;

// keep this sorted, please
// and update it if a new hardcoded block number is added in the future
pub const FIXED_BLOCK_NUMBERS: [pg_sys::BlockNumber; 5] = [
    MERGE_LOCK,
    CLEANUP_LOCK,
    SCHEMA_START,
    SETTINGS_START,
    SEGMENT_METAS_START,
];

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

    /// Counts the total number of data pages in the linked list (excludes the header page)
    pub npages: u32,

    /// Indicates where the BlockList for this linked list starts;
    pub blocklist_start: pg_sys::BlockNumber,
}

impl Debug for LinkedListData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkedListData")
            .field("start_blockno", &{ self.start_blockno })
            .field("last_blockno", &{ self.last_blockno })
            .field("npages", &{ self.npages })
            .field("blocklist_start", &{ self.blocklist_start })
            .finish()
    }
}

/// Every linked list must implement this trait
pub trait LinkedList {
    // Required methods
    fn get_header_blockno(&self) -> pg_sys::BlockNumber;

    // Provided methods
    fn get_start_blockno(&self) -> pg_sys::BlockNumber {
        let metadata = unsafe { self.get_linked_list_data() };
        let start_blockno = metadata.start_blockno;
        assert!(start_blockno != 0);
        assert!(start_blockno != pg_sys::InvalidBlockNumber);
        start_blockno
    }

    fn get_last_blockno(&self) -> pg_sys::BlockNumber {
        let metadata = unsafe { self.get_linked_list_data() };
        let last_blockno = metadata.last_blockno;
        assert!(last_blockno != 0);
        assert!(last_blockno != pg_sys::InvalidBlockNumber);
        last_blockno
    }

    fn npages(&self) -> u32 {
        let metadata = unsafe { self.get_linked_list_data() };
        metadata.npages - 1
    }

    fn block_for_ord(&self, ord: usize) -> Option<pg_sys::BlockNumber>;

    unsafe fn get_linked_list_data(&self) -> LinkedListData;
}

// ---------------------------------------------------------
// Linked list entry structs
// ---------------------------------------------------------

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

/// Metadata for tracking alive segments
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SegmentMetaEntry {
    pub segment_id: SegmentId,
    pub max_doc: u32,
    pub xmin: pg_sys::TransactionId,
    pub xmax: pg_sys::TransactionId,

    pub postings: Option<FileEntry>,
    pub positions: Option<FileEntry>,
    pub fast_fields: Option<FileEntry>,
    pub field_norms: Option<FileEntry>,
    pub terms: Option<FileEntry>,
    pub store: Option<FileEntry>,
    pub temp_store: Option<FileEntry>,
    pub delete: Option<DeleteEntry>,
}

#[cfg(any(test, feature = "pg_test"))]
impl Default for SegmentMetaEntry {
    fn default() -> Self {
        Self {
            segment_id: SegmentId::generate_random(),
            max_doc: Default::default(),
            xmin: pg_sys::InvalidTransactionId,
            xmax: pg_sys::InvalidTransactionId,
            postings: None,
            positions: None,
            fast_fields: None,
            field_norms: None,
            terms: None,
            store: None,
            temp_store: None,
            delete: None,
        }
    }
}

impl SegmentMetaEntry {
    pub fn num_docs(&self) -> usize {
        self.max_doc as usize - self.num_deleted_docs()
    }

    pub fn num_deleted_docs(&self) -> usize {
        self.delete
            .map(|entry| entry.num_deleted_docs as usize)
            .unwrap_or(0)
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
            .chain(self.store.iter().map(|fe| (fe, SegmentComponent::Store)))
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

    pub fn byte_size(&self) -> u64 {
        let mut size = 0;

        size += self
            .postings
            .as_ref()
            .map(|entry| entry.total_bytes as u64)
            .unwrap_or(0);
        size += self
            .positions
            .as_ref()
            .map(|entry| entry.total_bytes as u64)
            .unwrap_or(0);
        size += self
            .fast_fields
            .as_ref()
            .map(|entry| entry.total_bytes as u64)
            .unwrap_or(0);
        size += self
            .field_norms
            .as_ref()
            .map(|entry| entry.total_bytes as u64)
            .unwrap_or(0);
        size += self
            .terms
            .as_ref()
            .map(|entry| entry.total_bytes as u64)
            .unwrap_or(0);
        size += self
            .store
            .as_ref()
            .map(|entry| entry.total_bytes as u64)
            .unwrap_or(0);
        size += self
            .temp_store
            .as_ref()
            .map(|entry| entry.total_bytes as u64)
            .unwrap_or(0);
        size += self
            .delete
            .as_ref()
            .map(|entry| entry.file_entry.total_bytes as u64)
            .unwrap_or(0);
        size
    }

    pub fn get_component_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        let uuid = self.segment_id.uuid_string();
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
}

// ---------------------------------------------------------
// Linked list entry <-> PgItem
// ---------------------------------------------------------

/// Wrapper for pg_sys::Item that also stores its size
#[derive(Clone)]
pub struct PgItem(pub pg_sys::Item, pub pg_sys::Size);

impl From<SegmentMetaEntry> for PgItem {
    fn from(val: SegmentMetaEntry) -> Self {
        let bytes: Vec<u8> =
            bincode::serialize(&val).expect("expected to serialize valid SegmentMetaEntry");
        let pg_bytes = unsafe { pg_sys::palloc(bytes.len()) as *mut u8 };
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pg_bytes, bytes.len());
        }
        PgItem(pg_bytes as pg_sys::Item, bytes.len() as pg_sys::Size)
    }
}

impl From<PgItem> for SegmentMetaEntry {
    fn from(pg_item: PgItem) -> Self {
        let PgItem(item, size) = pg_item;
        let decoded: SegmentMetaEntry = unsafe {
            bincode::deserialize(from_raw_parts(item as *const u8, size))
                .expect("expected to deserialize valid SegmentMetaEntry")
        };
        decoded
    }
}

impl SegmentMetaEntry {
    /// Fake an opstamp value based on our internal `xmin` and `xmax` values
    pub fn opstamp(&self) -> Opstamp {
        self.xmin.max(self.xmax) as Opstamp // ((self.xmax as u64) << 32) | (self.xmin as u64)
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
    // Required methods
    fn get_xmin(&self) -> pg_sys::TransactionId;
    fn get_xmax(&self) -> pg_sys::TransactionId;
    fn into_frozen(self, should_freeze_xmin: bool, should_freeze_xmax: bool) -> Self;

    fn pintest_blockno(&self) -> pg_sys::BlockNumber;

    // Provided methods
    unsafe fn visible(&self, snapshot: pg_sys::Snapshot) -> bool {
        let xmin = self.get_xmin();
        let xmax = self.get_xmax();
        let xmin_visible = pg_sys::TransactionIdIsCurrentTransactionId(xmin)
            || !pg_sys::XidInMVCCSnapshot(xmin, snapshot);
        let deleted = xmax != pg_sys::InvalidTransactionId
            && (pg_sys::TransactionIdIsCurrentTransactionId(xmax)
                || !pg_sys::XidInMVCCSnapshot(xmax, snapshot));
        xmin_visible && !deleted
    }

    unsafe fn recyclable(&self, bman: &mut BufferManager) -> bool {
        let xmax = self.get_xmax();
        if xmax == pg_sys::InvalidTransactionId {
            return false;
        }

        // if the xmax transaction is no longer in progress
        !pg_sys::TransactionIdIsInProgress(xmax)

        // and there's no pin on our pintest buffer, assuming we have a valid buffer
        && {
            self.pintest_blockno() == pg_sys::InvalidBlockNumber
                || bman.get_buffer_for_cleanup_conditional(self.pintest_blockno()).is_some()
        }
    }

    unsafe fn mergeable(&self) -> bool {
        let xmin = self.get_xmin();
        let xmax = self.get_xmax();

        // mergeable if we haven't deleted it
        xmax == pg_sys::InvalidTransactionId

            // and it's from a transaction that is not in progress.  we can't merge segments created
            // by *this* transaction (ie, xmin == GetCurrentTransactionId()) because this transaction
            // is considered in progress
        && (!pg_sys::TransactionIdIsInProgress(xmin))
    }

    unsafe fn xmin_needs_freeze(&self, freeze_limit: pg_sys::TransactionId) -> bool {
        let xmin = self.get_xmin();
        pg_sys::TransactionIdIsNormal(xmin) && pg_sys::TransactionIdPrecedes(xmin, freeze_limit)
    }

    unsafe fn xmax_needs_freeze(&self, freeze_limit: pg_sys::TransactionId) -> bool {
        let xmax = self.get_xmax();
        pg_sys::TransactionIdIsNormal(xmax) && pg_sys::TransactionIdPrecedes(xmax, freeze_limit)
    }
}

impl MVCCEntry for SegmentMetaEntry {
    fn get_xmin(&self) -> pg_sys::TransactionId {
        self.xmin
    }
    fn get_xmax(&self) -> pg_sys::TransactionId {
        self.xmax
    }
    fn into_frozen(self, should_freeze_xmin: bool, should_freeze_xmax: bool) -> Self {
        SegmentMetaEntry {
            xmin: if should_freeze_xmin {
                pg_sys::FrozenTransactionId
            } else {
                self.xmin
            },
            xmax: if should_freeze_xmax {
                pg_sys::FrozenTransactionId
            } else {
                self.xmax
            },
            ..self
        }
    }

    fn pintest_blockno(&self) -> BlockNumber {
        match self.file_entries().next() {
            None => panic!("SegmentMetaEntry for `{}` has no files", self.segment_id),
            Some((file_entry, _)) => file_entry.starting_block,
        }
    }
}

pub const fn bm25_max_free_space() -> usize {
    unsafe {
        (pg_sys::BLCKSZ as usize)
            - pg_sys::MAXALIGN(size_of::<BM25PageSpecialData>())
            - pg_sys::MAXALIGN(offset_of!(pg_sys::PageHeaderData, pd_linp))
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;

    #[pg_test]
    unsafe fn test_needs_freeze() {
        let freeze_limit = 100;
        let segment = SegmentMetaEntry {
            xmin: 50,
            xmax: 150,
            ..Default::default()
        };

        let xmin_needs_freeze = segment.xmin_needs_freeze(freeze_limit);
        let xmax_needs_freeze = segment.xmax_needs_freeze(freeze_limit);

        assert!(xmin_needs_freeze);
        assert!(!xmax_needs_freeze);

        let frozen_segment = segment.into_frozen(xmin_needs_freeze, xmax_needs_freeze);

        assert_eq!(
            frozen_segment,
            SegmentMetaEntry {
                xmin: pg_sys::FrozenTransactionId,
                ..segment
            }
        );
    }
}
