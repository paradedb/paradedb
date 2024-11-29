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

use pgrx::*;
use serde::{Deserialize, Serialize};
use std::mem::{offset_of, size_of};
use std::path::PathBuf;
use std::slice::from_raw_parts;
use tantivy::index::InnerSegmentMeta;

pub const SCHEMA_START: pg_sys::BlockNumber = 0;
pub const SETTINGS_START: pg_sys::BlockNumber = 2;
pub const DIRECTORY_START: pg_sys::BlockNumber = 4;
pub const SEGMENT_METAS_START: pg_sys::BlockNumber = 6;

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
// Linked lists
// ---------------------------------------------------------

/// Struct held in the first buffer of every linked list's content area
#[derive(Debug)]
pub struct LinkedListData {
    pub start_blockno: pg_sys::BlockNumber,
    pub last_blockno: pg_sys::BlockNumber,
}

/// Every linked list must implement this trait
pub trait LinkedList {
    // Required methods
    fn get_lock_buffer(&self) -> pg_sys::Buffer;
    fn get_lock(&self) -> Option<u32>;

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

    unsafe fn set_last_blockno(&self, blockno: pg_sys::BlockNumber) {
        assert!(
            self.get_lock() == Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
            "an exclusive lock is required to write to linked list"
        );

        let lock_buffer = self.get_lock_buffer();
        let page = pg_sys::BufferGetPage(lock_buffer);
        let metadata = pg_sys::PageGetContents(page) as *mut LinkedListData;
        (*metadata).last_blockno = blockno;
        pg_sys::MarkBufferDirty(lock_buffer);
    }

    unsafe fn get_linked_list_data(&self) -> LinkedListData {
        let page = pg_sys::BufferGetPage(self.get_lock_buffer());
        let metadata = pg_sys::PageGetContents(page) as *mut LinkedListData;
        LinkedListData {
            start_blockno: (*metadata).start_blockno,
            last_blockno: (*metadata).last_blockno,
        }
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
    // This is the transaction ID that created this entry
    pub xmin: pg_sys::TransactionId,
    // The transaction ID that marks this entry as deleted
    // Vacuum will physically delete this entry if this transaction ID is no longer visible to any existing transactions
    pub xmax: pg_sys::TransactionId,
}

/// Metadata for tracking alive segments
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SegmentMetaEntry {
    pub meta: InnerSegmentMeta,
    pub opstamp: tantivy::Opstamp,
    // The transaction ID that created this entry
    pub xmin: pg_sys::TransactionId,
    // The transaction ID that marks this entry as deleted
    // Vacuum will physically delete this entry if this transaction ID is no longer visible to any existing transactions
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

// ---------------------------------------------------------
// Linked list entry MVCC methods
// ---------------------------------------------------------

pub trait MVCCEntry {
    // Required methods
    fn get_xmin(&self) -> pg_sys::TransactionId;
    fn get_xmax(&self) -> pg_sys::TransactionId;

    // Provided methods
    unsafe fn is_visible(&self, snapshot: pg_sys::Snapshot) -> bool {
        let xmin = self.get_xmin();
        let xmax = self.get_xmax();
        let xmin_visible =
            !pg_sys::XidInMVCCSnapshot(xmin, snapshot) && pg_sys::TransactionIdDidCommit(xmin);
        let deleted = xmax != pg_sys::InvalidTransactionId
            && !pg_sys::XidInMVCCSnapshot(xmax, snapshot)
            && pg_sys::TransactionIdDidCommit(xmax);
        xmin_visible && !deleted
    }

    fn is_deleted(&self) -> bool {
        self.get_xmax() != pg_sys::InvalidTransactionId
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

pub const unsafe fn bm25_max_free_space() -> usize {
    (pg_sys::BLCKSZ as usize)
        - pg_sys::MAXALIGN(size_of::<BM25PageSpecialData>())
        - pg_sys::MAXALIGN(offset_of!(pg_sys::PageHeaderData, pd_linp))
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
            total_bytes: 100 as usize,
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
            total_bytes: 100 as usize,
            xmin: pg_sys::GetCurrentTransactionId(),
            xmax: pg_sys::InvalidTransactionId,
        };
        let segment2 = DirectoryEntry {
            path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
            start: 1000,
            total_bytes: 100 as usize,
            xmin: pg_sys::GetCurrentTransactionId(),
            xmax: pg_sys::GetCurrentTransactionId(),
        };
        let PgItem(_, size1) = segment1.into();
        let PgItem(_, size2) = segment2.into();
        assert_eq!(size1, size2);
    }
}
