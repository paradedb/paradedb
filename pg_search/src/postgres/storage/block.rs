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
// [directory_entries_start BlockNumber]
// ...LP_UPPER
// LP_SPECIAL: <nothing>

// # directory entries

// ...LP_LOWER
// Item: [<PathBuf>, BlockNumber, ssize_t, TransactionId]
// Item: [<PathBuf>, BlockNumber, ssize_t, TransactionId]
// Item: [<PathBuf>, BlockNumber, ssize_t, TransactionId]
// ...
// Item: [<PathBuf>, BlockNumber, ssize_t, TransactionId]
// ...LP_UPPER
// LP_SPECIAL: [next_page BlockNumber, deleted bool, delete_xid TransactionId]

// # segment file blocknumber list

// ...LP_LOWER
// [BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber]
// [BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber]
// [BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber]
// [BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber][BlockNumber]
// ...LP_UPPER
// LP_SPECIAL: [next_page BlockNumber, deleted bool, delete_xid TransactionId]

// # segment file data

// ...LP_LOWER
// [u8 byte data]
// ...LP_UPPER
// LP_SPECIAL: [next_page BlockNumber, deleted bool, delete_xid TransactionId]

use super::linked_list::PgItem;
use pgrx::*;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};
use std::mem::{offset_of, size_of};
use std::path::PathBuf;
use std::slice::from_raw_parts;
use super::utils::BM25BufferCache;

pub const METADATA_BLOCKNO: pg_sys::BlockNumber = 0; // Stores metadata for the entire index
pub const INDEX_WRITER_LOCK_BLOCKNO: pg_sys::BlockNumber = 1; // Used for Tantivy's INDEX_WRITER_LOCK
pub const META_LOCK_BLOCKNO: pg_sys::BlockNumber = 2; // Used for Tantivy's META_LOCK
pub const MANAGED_LOCK_BLOCKNO: pg_sys::BlockNumber = 3; // Used for Tantivy's MANAGED_LOCK
pub const TANTIVY_META_BLOCKNO: pg_sys::BlockNumber = 4; // Used for Tantivy's meta.json

/// Special data struct for the metadata page, located at METADATA_BLOCKNO
pub struct MetaPageData {
    pub segment_component_first_blockno: pg_sys::BlockNumber,
    pub tantivy_meta_last_blockno: pg_sys::BlockNumber,
    pub tantivy_managed_first_blockno: pg_sys::BlockNumber,
}

/// Special data struct for all other pages except the metadata page and lock pages
pub struct BM25PageSpecialData {
    pub next_blockno: pg_sys::BlockNumber,
    pub last_blockno: pg_sys::BlockNumber,
    pub deleted: bool,
    pub delete_xid: pg_sys::FullTransactionId,
}

/// Metadata for tracking segment components
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SegmentComponentOpaque {
    pub path: PathBuf,
    pub start: pg_sys::BlockNumber,
    pub total_bytes: usize,
    pub xid: u32,
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

impl Into<Vec<u8>> for BlockNumberList {
    fn into(self) -> Vec<u8> {
        let mut bytes = vec![];
        for blockno in self.0 {
            bytes.extend_from_slice(&blockno.to_le_bytes());
        }
        bytes
    }
}

impl From<PgItem> for SegmentComponentOpaque {
    fn from(pg_item: PgItem) -> Self {
        let PgItem(item, size) = pg_item;
        let opaque: SegmentComponentOpaque = unsafe {
            serde_json::from_slice(from_raw_parts(item as *const u8, size))
                .expect("expected to deserialize valid SegmentComponent")
        };
        opaque
    }
}

impl Into<PgItem> for SegmentComponentOpaque {
    fn into(self) -> PgItem {
        let bytes: Vec<u8> =
            serde_json::to_vec(&self).expect("expected to serialize valid SegmentComponent");
        let pg_bytes = unsafe { pg_sys::palloc(bytes.len()) as *mut u8 };
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pg_bytes, bytes.len());
        }
        PgItem(pg_bytes as pg_sys::Item, bytes.len() as pg_sys::Size)
    }
}

pub unsafe fn bm25_metadata(relation_oid: pg_sys::Oid) -> MetaPageData {
    let cache = BM25BufferCache::open(relation_oid);
    let metadata_buffer = cache.get_buffer(METADATA_BLOCKNO, Some(pg_sys::BUFFER_LOCK_SHARE));
    let metadata_page = pg_sys::BufferGetPage(metadata_buffer);
    let metadata = pg_sys::PageGetContents(metadata_page) as *mut MetaPageData;
    let data = MetaPageData {
        segment_component_first_blockno: (*metadata).segment_component_first_blockno,
        tantivy_meta_last_blockno: (*metadata).tantivy_meta_last_blockno,
        tantivy_managed_first_blockno: (*metadata).tantivy_managed_first_blockno,
    };
    pg_sys::UnlockReleaseBuffer(metadata_buffer);
    data
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
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
    fn test_segment_component_opaque_into() {
        let segment = SegmentComponentOpaque {
            path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
            start: 0,
            total_bytes: 100 as usize,
            xid: 0,
        };
        let pg_item: PgItem = segment.clone().into();
        let segment_from_pg_item: SegmentComponentOpaque = pg_item.into();
        assert_eq!(segment, segment_from_pg_item);
    }
}
