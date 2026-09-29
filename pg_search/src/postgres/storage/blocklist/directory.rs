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

use super::tree::{Entry, Node};
use crate::postgres::storage::block::{
    BM25PageSpecialData, LinkedListData, block_number_is_valid, bm25_max_free_space,
};
use crate::postgres::storage::buffer::BufferManager;
use pgrx::pg_sys;
use std::sync::Arc;

const MAGIC: &[u8; 4] = b"BDIR";
const VERSION: u32 = 2;
const WORD_SIZE: usize = size_of::<u32>();
const HEADER_SIZE: usize = MAGIC.len() + 2 * WORD_SIZE;
const ENTRY_SIZE: usize = 2 * WORD_SIZE;
// The root shares the component header page, leaving less room for entries
// than a standalone directory page.
pub(super) const ROOT_CAPACITY: usize =
    (bm25_max_free_space() - size_of::<LinkedListData>() - HEADER_SIZE) / ENTRY_SIZE;
const PAGE_CAPACITY: usize = (bm25_max_free_space() - HEADER_SIZE) / ENTRY_SIZE;
/// Maximum root level needed to cover all `u32` logical page numbers.
/// A level-0 root covers `ROOT_CAPACITY` map pages; each extra level multiplies
/// coverage by `PAGE_CAPACITY` (level 3 is enough with 8 KiB pages).
pub(super) const MAX_LEVEL: u32 = {
    let mut covered_pages = ROOT_CAPACITY as u64;
    let mut level = 0;
    while covered_pages <= u32::MAX as u64 {
        covered_pages *= PAGE_CAPACITY as u64;
        level += 1;
    }
    level
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    pub(super) node: Arc<Node<u32, pg_sys::BlockNumber>>,
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
        (
            Self {
                node: Arc::new(node),
            },
            overflow,
        )
    }

    pub fn read(bytes: &[u8], first_block: pg_sys::BlockNumber) -> Option<Self> {
        if bytes.len() > bm25_max_free_space() {
            return None;
        }
        let directory = Self::decode(bytes.get(size_of::<LinkedListData>()..)?)?;
        if directory.node.entries[0].start != 0
            || (directory.node.level == 0 && directory.node.entries[0].address != first_block)
        {
            return None;
        }
        Some(directory)
    }

    pub(super) fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > bm25_max_free_space() {
            return None;
        }
        let mut encoded = bytes.strip_prefix(MAGIC)?;
        let next_word = |bytes: &mut &[u8]| {
            let (word, remaining) = bytes.split_first_chunk::<WORD_SIZE>()?;
            *bytes = remaining;
            Some(u32::from_le_bytes(*word))
        };
        let version = next_word(&mut encoded)?;
        let level = next_word(&mut encoded)?;
        if version != VERSION || level > MAX_LEVEL {
            return None;
        }
        if encoded.is_empty() || !encoded.len().is_multiple_of(ENTRY_SIZE) {
            return None;
        }
        let entries: Vec<_> = encoded
            .chunks_exact(ENTRY_SIZE)
            .map(|entry| Entry {
                start: u32::from_le_bytes(entry[..WORD_SIZE].try_into().unwrap()),
                address: u32::from_le_bytes(entry[WORD_SIZE..].try_into().unwrap()),
            })
            .collect();
        if entries
            .iter()
            .any(|entry| !block_number_is_valid(entry.address))
            || entries
                .windows(2)
                .any(|pair| pair[0].start >= pair[1].start)
        {
            return None;
        }
        Some(Self {
            node: Arc::new(Node { entries, level }),
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        Self::encode_node(&self.node)
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
