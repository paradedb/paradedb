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

use pgrx::pg_sys;
use serde::{Deserialize, Serialize};
use std::mem::{offset_of, size_of};
use std::path::PathBuf;

pub const METADATA_BLOCKNO: pg_sys::BlockNumber = 0; // Stores metadata for the entire index
pub const INDEX_WRITER_LOCK_BLOCKNO: pg_sys::BlockNumber = 1; // Used for Tantivy's INDEX_WRITER_LOCK
pub const META_LOCK_BLOCKNO: pg_sys::BlockNumber = 2; // Used for Tantivy's META_LOCK
pub const MANAGED_LOCK_BLOCKNO: pg_sys::BlockNumber = 3; // Used for Tantivy's MANAGED_LOCK
pub const TANTIVY_META_BLOCKNO: pg_sys::BlockNumber = 4; // Used for Tantivy's meta.json

/// Special data struct for the metadata page, located at METADATA_BLOCKNO
pub struct MetaPageData {
    pub segment_component_first_blockno: pg_sys::BlockNumber,
    pub segment_component_last_blockno: pg_sys::BlockNumber,
    pub tantivy_meta_last_blockno: pg_sys::BlockNumber,
    pub tantivy_managed_first_blockno: pg_sys::BlockNumber,
    pub tantivy_managed_last_blockno: pg_sys::BlockNumber,
}

/// Special data struct for all other pages except the metadata page and lock pages
pub struct BM25PageSpecialData {
    pub next_blockno: pg_sys::BlockNumber,
    pub deleted: bool,
    pub delete_xid: pg_sys::FullTransactionId,
}

/// Metadata for tracking segment components
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SegmentComponentOpaque {
    pub path: PathBuf,
    pub blocks: Vec<pg_sys::BlockNumber>,
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
