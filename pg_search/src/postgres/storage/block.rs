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

use super::utils::BM25BufferCache;
use crate::postgres::storage::SKIPLIST_FREQ;
use pgrx::*;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::mem::{offset_of, size_of};
use std::path::PathBuf;
use std::slice::from_raw_parts;
use tantivy::index::SegmentId;

pub const MERGE_LOCK: pg_sys::BlockNumber = 0;
pub const SCHEMA_START: pg_sys::BlockNumber = 1;
pub const SETTINGS_START: pg_sys::BlockNumber = 3;
pub const DIRECTORY_START: pg_sys::BlockNumber = 5;
pub const SEGMENT_METAS_START: pg_sys::BlockNumber = 7;
pub const DELETE_METAS_START: pg_sys::BlockNumber = 9;

// ---------------------------------------------------------
// BM25 page special data
// ---------------------------------------------------------

// Struct for all page's LP_SPECIAL data
#[derive(Debug)]
pub struct BM25PageSpecialData {
    pub next_blockno: pg_sys::BlockNumber,
    pub xmax: pg_sys::TransactionId,
}

// ---------------------------------------------------------
// Merge lock
// ---------------------------------------------------------

#[derive(Debug)]
pub struct MergeLockData {
    pub last_merge: pg_sys::TransactionId,
}

// ---------------------------------------------------------
// Linked lists
// ---------------------------------------------------------

/// Struct held in the first buffer of every linked list's content area
#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct LinkedListData {
    pub inner: Inner,

    /// contains every [`SKIPLIST_FREQ`]th BlockNumber in the list, and
    /// element zero is always the first block number
    pub skip_list: [pg_sys::BlockNumber; {
        (bm25_max_free_space() - size_of::<Inner>()) / size_of::<pg_sys::BlockNumber>()
    }],
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct Inner {
    /// Indicates the last BlockNumber of the linked list.
    pub last_blockno: pg_sys::BlockNumber,

    /// Counts the total number of data pages in the linked list (excludes the header page)
    pub npages: u32,
}

impl Debug for LinkedListData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkedListData")
            .field("inner", &{ self.inner })
            .field("skip_list", &{ self.skip_list })
            .finish()
    }
}

/// Every linked list must implement this trait
pub trait LinkedList {
    // Required methods
    fn get_header_blockno(&self) -> pg_sys::BlockNumber;
    fn get_relation_oid(&self) -> pg_sys::Oid;

    // Provided methods
    fn get_start_blockno(&self) -> pg_sys::BlockNumber {
        let metadata = unsafe { self.get_linked_list_data() };
        let start_blockno = metadata.skip_list[0];
        assert!(start_blockno != 0);
        assert!(start_blockno != pg_sys::InvalidBlockNumber);
        start_blockno
    }

    fn get_last_blockno(&self) -> pg_sys::BlockNumber {
        let metadata = unsafe { self.get_linked_list_data() };
        let last_blockno = metadata.inner.last_blockno;
        assert!(last_blockno != 0);
        assert!(last_blockno != pg_sys::InvalidBlockNumber);
        last_blockno
    }

    fn npages(&self) -> u32 {
        let metadata = unsafe { self.get_linked_list_data() };
        metadata.inner.npages - 1
    }

    fn nearest_block_by_ord(&self, ord: usize) -> (pg_sys::BlockNumber, usize) {
        let index = ord / SKIPLIST_FREQ;
        let metadata = unsafe { self.get_linked_list_data() };
        (metadata.skip_list[index], index * SKIPLIST_FREQ)
    }

    unsafe fn get_linked_list_data(&self) -> LinkedListData {
        let cache = BM25BufferCache::open(self.get_relation_oid());
        let header_buffer =
            cache.get_buffer(self.get_header_blockno(), Some(pg_sys::BUFFER_LOCK_SHARE));
        let page = pg_sys::BufferGetPage(header_buffer);
        let metadata = pg_sys::PageGetContents(page) as *mut LinkedListData;
        let data = metadata.read_unaligned();

        pg_sys::UnlockReleaseBuffer(header_buffer);
        data
    }
}

// ---------------------------------------------------------
// Linked list entry structs
// ---------------------------------------------------------

/// Metadata for tracking segment components
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DirectoryEntry {
    pub path: PathBuf,
    pub start: pg_sys::BlockNumber,
    pub total_bytes: usize,
    pub xmin: pg_sys::TransactionId,
    pub xmax: pg_sys::TransactionId,
}

/// Metadata for tracking alive segments
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SegmentMetaEntry {
    pub segment_id: SegmentId,
    pub max_doc: u32,
    pub opstamp: tantivy::Opstamp,
    pub xmin: pg_sys::TransactionId,
    pub xmax: pg_sys::TransactionId,
}

/// Metadata for tracking segment deletes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeleteMetaEntry {
    pub segment_id: SegmentId,
    pub num_deleted_docs: u32,
    pub opstamp: tantivy::Opstamp,
    pub xmax: pg_sys::TransactionId,
}

// ---------------------------------------------------------
// Linked list entry <-> PgItem
// ---------------------------------------------------------

/// Wrapper for pg_sys::Item that also stores its size
pub struct PgItem(pub pg_sys::Item, pub pg_sys::Size);

impl From<DirectoryEntry> for PgItem {
    fn from(val: DirectoryEntry) -> Self {
        let bytes: Vec<u8> =
            bincode::serialize(&val).expect("expected to serialize valid DirectoryEntry");
        let pg_bytes = unsafe { pg_sys::palloc(bytes.len()) as *mut u8 };
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pg_bytes, bytes.len());
        }
        PgItem(pg_bytes as pg_sys::Item, bytes.len() as pg_sys::Size)
    }
}

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

impl From<DeleteMetaEntry> for PgItem {
    fn from(val: DeleteMetaEntry) -> Self {
        let bytes: Vec<u8> =
            bincode::serialize(&val).expect("expected to serialize valid DeleteMetaEntry");
        let pg_bytes = unsafe { pg_sys::palloc(bytes.len()) as *mut u8 };
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pg_bytes, bytes.len());
        }
        PgItem(pg_bytes as pg_sys::Item, bytes.len() as pg_sys::Size)
    }
}

impl From<PgItem> for DirectoryEntry {
    fn from(pg_item: PgItem) -> Self {
        let PgItem(item, size) = pg_item;
        let decoded: DirectoryEntry = unsafe {
            bincode::deserialize(from_raw_parts(item as *const u8, size))
                .expect("expected to deserialize valid DirectoryEntry")
        };
        decoded
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

impl From<PgItem> for DeleteMetaEntry {
    fn from(pg_item: PgItem) -> Self {
        let PgItem(item, size) = pg_item;
        let decoded: DeleteMetaEntry = unsafe {
            bincode::deserialize(from_raw_parts(item as *const u8, size))
                .expect("expected to deserialize valid DeleteMetaEntry")
        };
        decoded
    }
}

// ---------------------------------------------------------
// Linked list entry MVCC methods
// ---------------------------------------------------------

pub trait MVCCEntry {
    // Required methods
    fn get_xmin(&self) -> pg_sys::TransactionId;
    fn get_xmax(&self) -> pg_sys::TransactionId;

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

    fn deleted(&self) -> bool {
        self.get_xmax() != pg_sys::InvalidTransactionId
    }

    unsafe fn recyclable(
        &self,
        snapshot: pg_sys::Snapshot,
        heap_relation: pg_sys::Relation,
    ) -> bool {
        let xmax = self.get_xmax();
        if xmax == pg_sys::InvalidTransactionId {
            return false;
        }

        if pg_sys::XidInMVCCSnapshot(xmax, snapshot) {
            return false;
        }

        pg_sys::GlobalVisCheckRemovableXid(heap_relation, xmax)
    }
}

impl MVCCEntry for DirectoryEntry {
    fn get_xmin(&self) -> pg_sys::TransactionId {
        self.xmin
    }
    fn get_xmax(&self) -> pg_sys::TransactionId {
        self.xmax
    }
}

impl MVCCEntry for SegmentMetaEntry {
    fn get_xmin(&self) -> pg_sys::TransactionId {
        self.xmin
    }
    fn get_xmax(&self) -> pg_sys::TransactionId {
        self.xmax
    }
}

impl MVCCEntry for DeleteMetaEntry {
    fn get_xmax(&self) -> pg_sys::TransactionId {
        self.xmax
    }
    // We want DeleteMetaEntry to be visible to all transactions immediately
    // when it's written because ambulkdelete is atomic
    fn get_xmin(&self) -> pg_sys::TransactionId {
        pg_sys::FrozenTransactionId
    }
    unsafe fn visible(&self, _snapshot: pg_sys::Snapshot) -> bool {
        true
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
    use uuid::Uuid;

    #[pg_test]
    unsafe fn test_directory_entry_into() {
        let segment = DirectoryEntry {
            path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
            start: 0,
            total_bytes: 100_usize,
            xmin: pg_sys::GetCurrentTransactionId(),
            xmax: pg_sys::InvalidTransactionId,
        };
        let pg_item: PgItem = segment.clone().into();
        let segment_from_pg_item: DirectoryEntry = pg_item.into();
        assert_eq!(segment, segment_from_pg_item);
    }

    #[pg_test]
    unsafe fn test_serialized_size() {
        let segment1 = DirectoryEntry {
            path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
            start: 0,
            total_bytes: 100_usize,
            xmin: pg_sys::GetCurrentTransactionId(),
            xmax: pg_sys::InvalidTransactionId,
        };
        let segment2 = DirectoryEntry {
            path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
            start: 1000,
            total_bytes: 100_usize,
            xmin: pg_sys::GetCurrentTransactionId(),
            xmax: pg_sys::GetCurrentTransactionId(),
        };
        let PgItem(_, size1) = segment1.into();
        let PgItem(_, size2) = segment2.into();
        assert_eq!(size1, size2);
    }
}
