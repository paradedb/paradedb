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

//! Stores the tree beside a component's existing compressed page map.
//!
//! ```text
//! Header page
//!   original 16-byte metadata -> data chain + compressed map chain
//!   directory root: BDIR | version=2 | level | (logical start, block)...
//!
//! Small: header root -> compressed map -> data
//! Large: header root -> directory page -> ... -> compressed map -> data
//! ```
//!
//! For example, a level-1 entry (100, 72) leads to directory block 72. Its level-0
//! entry (120, 900) leads to map block 900. Looking up logical page 130 follows
//! those two blocks, then decodes page 130's data address from the map.
//!
//! Extra directory pages use the same format as the root. Numbers are 4-byte
//! little-endian integers. Used page bytes determine entry count; neighboring
//! entries, parent ranges, or component length determine range ends.
//!
//! The header's next-page link chains all extra directory pages for cleanup.
//! Existing data/map chains stay unchanged. Old formats remain readable, and a
//! missing or unsupported directory falls back to walking the original map.

use super::tree::{Bounds, Entry, Node, ReadNode};
use crate::postgres::storage::block::{
    BM25PageSpecialData, LinkedListData, block_number_is_valid, bm25_max_free_space,
};
use crate::postgres::storage::buffer::BufferManager;
use bytemuck::{Pod, Zeroable};
use pgrx::pg_sys;
use tantivy::directory::OwnedBytes;

const MAGIC: &[u8; 4] = b"BDIR";
const VERSION: u32 = 2;
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct DirectoryHeader {
    magic: [u8; 4],
    version: u32,
    level: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct DirectoryEntry {
    start: u32,
    block: pg_sys::BlockNumber,
}

/// Borrows the encoded entries directly from the pinned page.
struct DirectoryNode<'a> {
    header: &'a DirectoryHeader,
    entries: &'a [DirectoryEntry],
}

const HEADER_SIZE: usize = size_of::<DirectoryHeader>();
const ENTRY_SIZE: usize = size_of::<DirectoryEntry>();
// The root shares the component header page, leaving less room for entries
// than a standalone directory page.
pub(super) const ROOT_CAPACITY: usize =
    (bm25_max_free_space() - size_of::<LinkedListData>() - HEADER_SIZE) / ENTRY_SIZE;
const PAGE_CAPACITY: usize = (bm25_max_free_space() - HEADER_SIZE) / ENTRY_SIZE;
/// Maximum root level for `u32` logical page numbers. With 8 KiB pages, level 2
/// covers about 1 billion map pages; level 3 exceeds the 4.3 billion addressable
/// pages. Level 0 already points to map pages, so level 3 means four directory levels.
pub(super) const MAX_LEVEL: u32 = 3;

/// A shared directory root for one component's compressed page map.
/// Level-0 entries point to map pages; higher levels point to directory pages.
/// The root is stored beside the component header on the same Postgres page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    bytes: OwnedBytes,
}

impl From<Node<u32, pg_sys::BlockNumber>> for Directory {
    fn from(node: Node<u32, pg_sys::BlockNumber>) -> Self {
        Self::decode(OwnedBytes::new(Self::encode_node(&node)))
            .expect("invalid block map directory")
    }
}

impl Directory {
    pub(super) fn build(
        bman: &mut BufferManager,
        entries: Vec<Entry<u32, pg_sys::BlockNumber>>,
    ) -> (Self, pg_sys::BlockNumber) {
        let mut overflow = pg_sys::InvalidBlockNumber;
        let node = Node::build(entries, ROOT_CAPACITY, PAGE_CAPACITY, |node| {
            pgrx::check_for_interrupts!();
            let mut buffer = bman.new_buffer();
            let block = buffer.number();
            let mut page = buffer.init_page();
            assert!(page.append_bytes(&Self::encode_node(node)));
            page.special_mut::<BM25PageSpecialData>().next_blockno = overflow;
            overflow = block;
            block
        });
        assert!(
            node.level <= MAX_LEVEL,
            "block map directory exceeds addressable depth"
        );
        (Self::from(node), overflow)
    }

    pub fn read(bytes: OwnedBytes, first_block: pg_sys::BlockNumber) -> Option<Self> {
        if bytes.len() > bm25_max_free_space() {
            return None;
        }
        if bytes.len() < size_of::<LinkedListData>() {
            return None;
        }
        let directory = Self::decode(bytes.slice(size_of::<LinkedListData>()..bytes.len()))?;
        if directory.entry(0).start != 0
            || (directory.level() == 0 && directory.entry(0).address != first_block)
        {
            return None;
        }
        Some(directory)
    }

    pub(super) fn decode(bytes: OwnedBytes) -> Option<Self> {
        if bytes.len() > bm25_max_free_space() || bytes.len() < HEADER_SIZE + ENTRY_SIZE {
            return None;
        }
        let header: &DirectoryHeader = bytemuck::try_from_bytes(&bytes[..HEADER_SIZE]).ok()?;
        if &header.magic != MAGIC
            || u32::from_le(header.version) != VERSION
            || u32::from_le(header.level) > MAX_LEVEL
        {
            return None;
        }
        let entries: &[DirectoryEntry] = bytemuck::try_cast_slice(&bytes[HEADER_SIZE..]).ok()?;
        if entries
            .iter()
            .any(|entry| !block_number_is_valid(u32::from_le(entry.block)))
            || entries
                .windows(2)
                .any(|pair| u32::from_le(pair[0].start) >= u32::from_le(pair[1].start))
        {
            return None;
        }
        Some(Self { bytes })
    }

    fn view(&self) -> DirectoryNode<'_> {
        DirectoryNode {
            header: bytemuck::from_bytes(&self.bytes[..HEADER_SIZE]),
            entries: bytemuck::cast_slice(&self.bytes[HEADER_SIZE..]),
        }
    }

    pub fn encode(&self) -> &[u8] {
        &self.bytes
    }

    fn encode_node(node: &Node<u32, pg_sys::BlockNumber>) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_SIZE + node.entries.len() * ENTRY_SIZE);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&node.level.to_le_bytes());
        for entry in &node.entries {
            bytes.extend_from_slice(&entry.start.to_le_bytes());
            bytes.extend_from_slice(&entry.address.to_le_bytes());
        }
        bytes
    }
}

impl ReadNode<u32, pg_sys::BlockNumber> for Directory {
    fn level(&self) -> u32 {
        u32::from_le(self.view().header.level)
    }
    fn len(&self) -> usize {
        self.view().entries.len()
    }
    fn entry(&self, index: usize) -> Entry<u32, pg_sys::BlockNumber> {
        let entry = &self.view().entries[index];
        Entry {
            start: u32::from_le(entry.start),
            address: u32::from_le(entry.block),
        }
    }
    fn valid_for(&self, bounds: Bounds<u32>) -> bool {
        self.entry(0).start == bounds.start
            && bounds
                .end
                .is_none_or(|end| self.entry(self.len() - 1).start < end)
    }
    fn partition_point(&self, key: u32) -> usize {
        self.view()
            .entries
            .partition_point(|entry| u32::from_le(entry.start) <= key)
    }
}
