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

// ---------------------------------------------------------------
// BM25 block storage layout
// A block storage representation of a Tantivy index
// ---------------------------------------------------------------

// ---------------------------------------------------------------
// MetaPage
// Block 0 is always the MetaPage.  It stores the block numbers of
// everything described below, so those blocks are allocated at
// index build but their numbers are not fixed.  Indexes built
// before those block numbers moved into the MetaPage fall back to
// the hardcoded `LEGACY_*` numbers in `metadata.rs`.
// ---------------------------------------------------------------

// ---------------------------------------------------------------
// Lock blocks
// Two blocks are used as locks for merge and vacuum cleanup
// ---------------------------------------------------------------

// +-------------------------------------------------------------+
// |                          Merge Lock                         |
// +-------------------------------------------------------------+
// | Empty                                                       |
// +-------------------------------------------------------------+

// +-------------------------------------------------------------+
// |                         Cleanup Lock                        |
// +-------------------------------------------------------------+
// | Empty                                                       |
// +-------------------------------------------------------------+

// ---------------------------------------------------------------
// Metadata blocks
// These blocks are created at index build and their block numbers
// are recorded in the MetaPage
// ---------------------------------------------------------------

// +-------------------------------------------------------------+
// |                          Schema Block                       |
// +-------------------------------------------------------------+
// | Serialized Tantivy Schema                                   |
// +-------------------------------------------------------------+
// | LP_SPECIAL                                                  |
// | [next_blockno: BlockNumber, xmax: TransactionId]            |
// +-------------------------------------------------------------+

// +-------------------------------------------------------------+
// |                        Settings Block                       |
// +-------------------------------------------------------------+
// | Serialized Tantivy IndexSettings                            |
// +-------------------------------------------------------------+
// | LP_SPECIAL                                                  |
// | [next_blockno: BlockNumber, xmax: TransactionId]            |
// +-------------------------------------------------------------+

// +-------------------------------------------------------------+
// |                 Segment Meta Entries Block                  |
// +-------------------------------------------------------------+
// | Serialized SegmentMetaEntry Items                           |
// | [SegmentMetaEntry]                                          |
// | [SegmentMetaEntry]                                          |
// | [SegmentMetaEntry]                                          |
// | ...                                                         |
// +-------------------------------------------------------------+
// | LP_SPECIAL                                                  |
// | [next_blockno: BlockNumber, xmax: TransactionId]            |
// +-------------------------------------------------------------+

// ---------------------------------------------------------------
// Remaining blocks: Segment component blocks
// These blocks are created when Tantivy writes new segments
// Can be picked up by vacuum
// ---------------------------------------------------------------

// +-------------------------------------------------------------+
// |                 Segment Component <uuid.ext>                |
// +-------------------------------------------------------------+
// | Serialized SegmentComponent data                            |
// +-------------------------------------------------------------+
// | LP_SPECIAL                                                  |
// | [next_blockno: BlockNumber, xmax: TransactionId]            |
// +-------------------------------------------------------------+

mod avl;
pub mod block;
mod blocklist;
pub mod buffer;
pub mod custom_rmgr;
pub mod fsm;
pub mod linked_bytes;
pub mod linked_items;
pub mod merge;
pub mod metadata;
pub mod utils;
mod xlog;

pub use self::linked_bytes::{LinkedBytesList, LinkedBytesListWriter};
pub use self::linked_items::LinkedItemList;
pub use self::utils::MAX_BUFFERS_TO_EXTEND_BY;
