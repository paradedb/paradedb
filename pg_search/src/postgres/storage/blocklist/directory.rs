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

use super::chunk_size;
use crate::immutable_tree::{Entry, Node};
use crate::postgres::storage::block::{
    BM25PageSpecialData, LinkedListData, block_number_is_valid, bm25_max_free_space,
};
use crate::postgres::storage::buffer::BufferManager;
use pgrx::pg_sys;
use std::sync::Arc;

const MAGIC: &[u8; 4] = b"BDIR";
const VERSION: u32 = 3;
const HEADER_SIZE: usize = 12;
const ENTRY_SIZE: usize = 8;
pub(super) const ROOT_CAPACITY: usize =
    (bm25_max_free_space() - size_of::<LinkedListData>() - HEADER_SIZE) / ENTRY_SIZE;
const PAGE_CAPACITY: usize = (bm25_max_free_space() - HEADER_SIZE) / ENTRY_SIZE;

/// Version 3: BDIR, version, level, then (logical start, block) pairs; all integers are LE u32.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    pub(super) node: Arc<Node<u32, pg_sys::BlockNumber>>,
    pub(super) legacy_end: Option<u32>,
}

impl Directory {
    pub fn build(
        bman: &mut BufferManager,
        mut block: pg_sys::BlockNumber,
    ) -> Option<(Self, pg_sys::BlockNumber)> {
        let mut entries = Vec::new();
        let mut ordinal = 0u32;
        while block != pg_sys::InvalidBlockNumber {
            pgrx::check_for_interrupts!();
            entries.push(Entry {
                start: ordinal,
                address: block,
            });
            let buffer = bman.get_buffer(block);
            let page = buffer.page();
            let mut bytes = page.as_slice();
            while !bytes.is_empty() {
                let (count, len) = chunk_size(bytes);
                ordinal = ordinal.checked_add(count.try_into().ok()?)?;
                bytes = &bytes[len..];
            }
            block = page.next_blockno();
        }
        if entries.is_empty() {
            return None;
        }
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
        Some((
            Self {
                node: Arc::new(node),
                legacy_end: None,
            },
            overflow,
        ))
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
        if bytes.len() < HEADER_SIZE || bytes.len() > bm25_max_free_space() || &bytes[..4] != MAGIC
        {
            return None;
        }
        let word = |offset| {
            bytes
                .get(offset..offset + 4)
                .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
        };
        let (level, header_size, legacy_end) = match word(4)? {
            version @ (1 | 2) => {
                let header_size = if version == 1 { 16 } else { 20 };
                let count = word(8)? as usize;
                if count == 0
                    || count > (bm25_max_free_space() - header_size) / ENTRY_SIZE
                    || bytes.len() != header_size + count * ENTRY_SIZE
                {
                    return None;
                }
                (
                    if version == 1 { 0 } else { word(16)? },
                    header_size,
                    Some(word(12)?),
                )
            }
            VERSION => (word(8)?, HEADER_SIZE, None),
            _ => return None,
        };
        let encoded = bytes.get(header_size..)?;
        if level > 3 || encoded.is_empty() || !encoded.len().is_multiple_of(ENTRY_SIZE) {
            return None;
        }
        let entries: Vec<_> = encoded
            .chunks_exact(ENTRY_SIZE)
            .map(|entry| Entry {
                start: u32::from_le_bytes(entry[..4].try_into().unwrap()),
                address: u32::from_le_bytes(entry[4..].try_into().unwrap()),
            })
            .collect();
        if entries.iter().any(|entry| {
            !block_number_is_valid(entry.address)
                || legacy_end.is_some_and(|end| entry.start >= end)
        }) || entries
            .windows(2)
            .any(|pair| pair[0].start >= pair[1].start)
        {
            return None;
        }
        Some(Self {
            node: Arc::new(Node { entries, level }),
            legacy_end,
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
