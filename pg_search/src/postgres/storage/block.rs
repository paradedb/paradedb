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

// ----------------------------------------------------------------
// 						Block storage layout
// ----------------------------------------------------------------

// # metadata

// ...LP_LOWER
// [directory_start BlockNumber]
// ...LP_UPPER
// LP_SPECIAL: <nothing>

// TODO: DOCUMENT META BLOCKS
// TODO: Get rid of deleted flag

// # directory entries

// ...LP_LOWER
// Item: [<PathBuf>, BlockNumber, ssize_t, TransactionId]
// Item: [<PathBuf>, BlockNumber, ssize_t, TransactionId]
// Item: [<PathBuf>, BlockNumber, ssize_t, TransactionId]
// ...
// Item: [<PathBuf>, BlockNumber, ssize_t, TransactionId]
// ...LP_UPPER
// LP_SPECIAL: [next_page BlockNumber, delete_xid TransactionId]

// # segment file blocknumber list

// ...LP_LOWER
// [BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber]
// [BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber]
// [BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber]
// [BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber]
// ...LP_UPPER
// LP_SPECIAL: [next_page BlockNumber, delete_xid TransactionId]

// # segment file data

// ...LP_LOWER
// [u8 byte data]
// ...LP_UPPER
// LP_SPECIAL: [next_page BlockNumber, delete_xid TransactionId]

use super::linked_list::PgItem;
use super::utils::BM25BufferCache;
use pgrx::*;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Write};
use std::mem::{offset_of, size_of};
use std::path::PathBuf;
use std::slice::from_raw_parts;
use tantivy::index::InnerSegmentMeta;

pub const METADATA_BLOCKNO: pg_sys::BlockNumber = 0; // Stores metadata for the entire index
pub const INDEX_WRITER_LOCK_BLOCKNO: pg_sys::BlockNumber = 1; // Used for Tantivy's INDEX_WRITER_LOCK
pub const META_LOCK_BLOCKNO: pg_sys::BlockNumber = 2; // Used for Tantivy's META_LOCK
pub const MANAGED_LOCK_BLOCKNO: pg_sys::BlockNumber = 3; // Used for Tantivy's MANAGED_LOCK

/// Special data struct for the metadata page, located at METADATA_BLOCKNO
#[derive(Debug)]
pub struct MetaPageData {
    pub directory_start: pg_sys::BlockNumber,
    pub segment_metas_start: pg_sys::BlockNumber,
    pub schema_start: pg_sys::BlockNumber,
    pub settings_start: pg_sys::BlockNumber,
}

/// Special data struct for all other pages except the metadata page and lock pages
#[derive(Debug)]
pub struct BM25PageSpecialData {
    pub next_blockno: pg_sys::BlockNumber,
    pub delete_xid: pg_sys::FullTransactionId,
}

/// Every linked list should start with a page that holds metadata about the linked list
#[derive(Debug)]
pub struct LinkedListData {
    pub start_blockno: pg_sys::BlockNumber,
    pub last_blockno: pg_sys::BlockNumber,
}

/// Metadata for tracking segment components
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DirectoryEntry {
    pub path: PathBuf,
    pub start: pg_sys::BlockNumber,
    pub total_bytes: usize,
    // This is the transaction ID that created this entry
    // An entry is a candidate for being marked as deleted if this transaction ID has been committed or aborted
    // which guarantees that we don't mark in-progress entries as deleted
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
    pub xmax: pg_sys::TransactionId,
}

/// Defined in `src/include/c.h`
const fn typealign_down(align_val: usize, len: usize) -> usize {
    // #define TYPEALIGN_DOWN(ALIGNVAL,LEN)  \
    // (((uintptr_t) (LEN)) & ~((uintptr_t) ((ALIGNVAL) - 1)))
    len & !(align_val - 1)
}

/// Defined in `src/include/c.h`
const fn maxalign_down(len: usize) -> usize {
    // #define MAXALIGN_DOWN(LEN)
    // TYPEALIGN_DOWN(MAXIMUM_ALIGNOF, (LEN))
    typealign_down(pg_sys::MAXIMUM_ALIGNOF as usize, len)
}

pub const unsafe fn bm25_max_item_size() -> usize {
    maxalign_down(
        pg_sys::BLCKSZ as usize
            - pg_sys::MAXALIGN(
                offset_of!(pg_sys::PageHeaderData, pd_linp) + size_of::<pg_sys::ItemIdData>(),
            )
            - pg_sys::MAXALIGN(size_of::<BM25PageSpecialData>()),
    )
}

pub const unsafe fn bm25_max_free_space() -> usize {
    (pg_sys::BLCKSZ as usize)
        - pg_sys::MAXALIGN(size_of::<BM25PageSpecialData>())
        - pg_sys::MAXALIGN(offset_of!(pg_sys::PageHeaderData, pd_linp))
}

pub unsafe fn bm25_metadata(relation_oid: pg_sys::Oid) -> MetaPageData {
    let cache = BM25BufferCache::open(relation_oid);
    let metadata_buffer = cache.get_buffer(METADATA_BLOCKNO, Some(pg_sys::BUFFER_LOCK_SHARE));
    let metadata_page = pg_sys::BufferGetPage(metadata_buffer);
    let metadata = pg_sys::PageGetContents(metadata_page) as *mut MetaPageData;

    let header = metadata_page as *const pg_sys::PageHeaderData;

    let data = MetaPageData {
        directory_start: (*metadata).directory_start,
        segment_metas_start: (*metadata).segment_metas_start,
        schema_start: (*metadata).schema_start,
        settings_start: (*metadata).settings_start,
    };
    pg_sys::UnlockReleaseBuffer(metadata_buffer);
    data
}

pub struct BlockNumberList(pub Vec<pg_sys::BlockNumber>);

impl From<&[u8]> for BlockNumberList {
    fn from(bytes: &[u8]) -> Self {
        let mut blocks = vec![];
        let mut cursor = Cursor::new(bytes);
        while cursor.position() < bytes.len() as u64 {
            let mut block_bytes = [0u8; std::mem::size_of::<pg_sys::BlockNumber>()];
            cursor.read_exact(&mut block_bytes).unwrap();
            blocks.push(u32::from_le_bytes(block_bytes) as pg_sys::BlockNumber);
        }
        BlockNumberList(blocks)
    }
}

impl From<Vec<u8>> for BlockNumberList {
    fn from(bytes: Vec<u8>) -> Self {
        BlockNumberList::from(&bytes[..])
    }
}

impl From<BlockNumberList> for Vec<u8> {
    fn from(val: BlockNumberList) -> Self {
        let mut bytes = vec![];
        for blockno in val.0 {
            bytes.extend_from_slice(&blockno.to_le_bytes());
        }
        bytes
    }
}

impl From<PgItem> for DirectoryEntry {
    fn from(pg_item: PgItem) -> Self {
        let PgItem(item, size) = pg_item;
        let decoded: DirectoryEntry = unsafe {
            bincode::deserialize(from_raw_parts(item as *const u8, size))
                .expect("expected to deserialize valid SegmentComponent")
        };
        decoded
    }
}

impl From<DirectoryEntry> for PgItem {
    fn from(val: DirectoryEntry) -> Self {
        let bytes: Vec<u8> =
            bincode::serialize(&val).expect("expected to serialize valid SegmentComponent");
        let pg_bytes = unsafe { pg_sys::palloc(bytes.len()) as *mut u8 };
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pg_bytes, bytes.len());
        }
        PgItem(pg_bytes as pg_sys::Item, bytes.len() as pg_sys::Size)
    }
}

impl From<PgItem> for SegmentMetaEntry {
    fn from(pg_item: PgItem) -> Self {
        let data =
            unsafe { std::slice::from_raw_parts(pg_item.0 as *const u8, pg_item.1 as usize) };

        assert!(
            data.len() >= 16,
            "PgItem data is too small to contain xmin and xmax"
        );

        let xmin = u32::from_le_bytes(data[0..4].try_into().expect("Failed to read xmin"));
        let xmax = u32::from_le_bytes(data[4..8].try_into().expect("Failed to read xmax"));
        let opstamp = u64::from_le_bytes(data[8..16].try_into().expect("Failed to read opstamp"));

        let meta_str = String::from_utf8(data[16..].to_vec()).expect("Failed to read meta");
        let meta: InnerSegmentMeta =
            serde_json::from_str(&meta_str).expect("Failed to deserialize InnerSegmentMeta");

        SegmentMetaEntry {
            meta,
            opstamp,
            xmin,
            xmax,
        }
    }
}

impl From<SegmentMetaEntry> for PgItem {
    fn from(val: SegmentMetaEntry) -> Self {
        // Serialize only the `meta` and `opstamp` fields of the SegmentMetaEntry as JSON
        let mut buffer = serde_json::to_vec_pretty(&val.meta).unwrap();
        writeln!(&mut buffer).unwrap();

        // First 16 bytes for xmin, xmax, and opstamp
        let mut bytes = Vec::with_capacity(16 + buffer.len());
        bytes.extend_from_slice(&val.xmin.to_le_bytes());
        bytes.extend_from_slice(&val.xmax.to_le_bytes());
        bytes.extend_from_slice(&val.opstamp.to_le_bytes());
        bytes.extend_from_slice(&buffer[..]);

        let pg_bytes = unsafe { pg_sys::palloc(bytes.len()) as *mut u8 };
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pg_bytes, bytes.len());
        }
        PgItem(pg_bytes as pg_sys::Item, bytes.len() as pg_sys::Size)
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use tantivy::index::SegmentId;
    use uuid::Uuid;

    #[pg_test]
    unsafe fn test_block_number_list() {
        let blocknos = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let blockno_list = BlockNumberList(blocknos.clone());
        let bytes: Vec<u8> = blockno_list.into();
        let blockno_list_from_bytes = BlockNumberList::from(&bytes[..]);
        assert_eq!(blocknos, blockno_list_from_bytes.0);
    }

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
