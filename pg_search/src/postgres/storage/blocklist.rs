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

use crate::postgres::storage::block::{LinkedListData, block_number_is_valid, bm25_max_free_space};
use crate::postgres::storage::buffer::BufferManager;
use pgrx::pg_sys;

#[derive(Debug)]
#[repr(u8)]
enum ChunkStyleTag {
    Sorted1x = 0,
    Sorted4x = 1,
    Sorted8x = 2,
    StrictlySorted1x = 3,
    StrictlySorted4x = 4,
    StrictlySorted8x = 5,
    Uncompressed = 6,
}

impl From<u8> for ChunkStyleTag {
    fn from(value: u8) -> Self {
        match value {
            0 => ChunkStyleTag::Sorted1x,
            1 => ChunkStyleTag::Sorted4x,
            2 => ChunkStyleTag::Sorted8x,
            3 => ChunkStyleTag::StrictlySorted1x,
            4 => ChunkStyleTag::StrictlySorted4x,
            5 => ChunkStyleTag::StrictlySorted8x,
            6 => ChunkStyleTag::Uncompressed,
            other => panic!("invalid chunk style tag: {other}"),
        }
    }
}

impl From<ChunkStyleTag> for u8 {
    fn from(value: ChunkStyleTag) -> Self {
        value as u8
    }
}

const DIRECTORY_MAGIC: &[u8; 4] = b"BDIR";
const DIRECTORY_VERSION: u32 = 1;
const DIRECTORY_HEADER_SIZE: usize = 16;
const DIRECTORY_ENTRY_SIZE: usize = 8;
const MAX_DIRECTORY_ENTRIES: usize =
    (bm25_max_free_space() - size_of::<LinkedListData>() - DIRECTORY_HEADER_SIZE)
        / DIRECTORY_ENTRY_SIZE;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DirectoryEntry {
    first_ordinal: u32,
    block: pg_sys::BlockNumber,
}

/// Optional component-header suffix: magic, version, entry count, address count, then
/// (first logical ordinal, physical map block) pairs. All integers are little-endian u32s.
/// The original linked map is retained for older readers and directories that do not fit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    entries: Vec<DirectoryEntry>,
    total_entries: u32,
}

impl Directory {
    pub fn build(bman: &BufferManager, mut block: pg_sys::BlockNumber) -> Option<Self> {
        let mut entries = Vec::new();
        let mut total_entries = 0u32;
        while block != pg_sys::InvalidBlockNumber {
            if entries.len() == MAX_DIRECTORY_ENTRIES {
                return None;
            }
            pgrx::check_for_interrupts!();
            entries.push(DirectoryEntry {
                first_ordinal: total_entries,
                block,
            });
            let buffer = bman.get_buffer(block);
            let page = buffer.page();
            let mut bytes = page.as_slice();
            while !bytes.is_empty() {
                let (count, len) = chunk_size(bytes);
                total_entries = total_entries.checked_add(count.try_into().ok()?)?;
                bytes = &bytes[len..];
            }
            block = page.next_blockno();
        }
        (!entries.is_empty()).then_some(Self {
            entries,
            total_entries,
        })
    }

    pub fn read(bytes: &[u8], first_block: pg_sys::BlockNumber) -> Option<Self> {
        let bytes = bytes.get(size_of::<LinkedListData>()..)?;
        if bytes.len() < DIRECTORY_HEADER_SIZE || &bytes[..4] != DIRECTORY_MAGIC {
            return None;
        }
        let word = |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        if word(4) != DIRECTORY_VERSION {
            return None;
        }
        let count = word(8) as usize;
        let total_entries = word(12);
        if count == 0 || count > MAX_DIRECTORY_ENTRIES || total_entries == 0 {
            return None;
        }
        let encoded = bytes
            .get(DIRECTORY_HEADER_SIZE..DIRECTORY_HEADER_SIZE + count * DIRECTORY_ENTRY_SIZE)?;
        let entries: Vec<_> = encoded
            .chunks_exact(DIRECTORY_ENTRY_SIZE)
            .map(|entry| DirectoryEntry {
                first_ordinal: u32::from_le_bytes(entry[..4].try_into().unwrap()),
                block: u32::from_le_bytes(entry[4..].try_into().unwrap()),
            })
            .collect();
        if entries[0].first_ordinal != 0
            || entries[0].block != first_block
            || entries.iter().any(|entry| {
                !block_number_is_valid(entry.block) || entry.first_ordinal >= total_entries
            })
            || entries
                .windows(2)
                .any(|pair| pair[0].first_ordinal >= pair[1].first_ordinal)
        {
            return None;
        }
        Some(Self {
            entries,
            total_entries,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut bytes =
            Vec::with_capacity(DIRECTORY_HEADER_SIZE + self.entries.len() * DIRECTORY_ENTRY_SIZE);
        bytes.extend_from_slice(DIRECTORY_MAGIC);
        bytes.extend_from_slice(&DIRECTORY_VERSION.to_le_bytes());
        bytes.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.total_entries.to_le_bytes());
        for entry in &self.entries {
            bytes.extend_from_slice(&entry.first_ordinal.to_le_bytes());
            bytes.extend_from_slice(&entry.block.to_le_bytes());
        }
        bytes
    }
}

fn chunk_size(bytes: &[u8]) -> (usize, usize) {
    use bitpacking::{BitPacker, BitPacker1x, BitPacker4x, BitPacker8x};
    let tag = ChunkStyleTag::from(bytes[0]);
    let (count, len) = match tag {
        ChunkStyleTag::Uncompressed => {
            let count = bytes[1] as usize;
            (count, 2 + count * size_of::<pg_sys::BlockNumber>())
        }
        _ => {
            let count = match tag {
                ChunkStyleTag::Sorted1x | ChunkStyleTag::StrictlySorted1x => BitPacker1x::BLOCK_LEN,
                ChunkStyleTag::Sorted4x | ChunkStyleTag::StrictlySorted4x => BitPacker4x::BLOCK_LEN,
                ChunkStyleTag::Sorted8x | ChunkStyleTag::StrictlySorted8x => BitPacker8x::BLOCK_LEN,
                _ => unreachable!(),
            };
            let bits = bytes[1] as usize;
            assert!(bits <= 32, "invalid block map bit width");
            (count, 6 + count * bits / 8)
        }
    };
    assert!(count > 0 && len <= bytes.len(), "truncated block map chunk");
    (count, len)
}

pub mod builder {
    use crate::postgres::storage::block::BM25PageSpecialData;
    use crate::postgres::storage::blocklist::ChunkStyleTag;
    use crate::postgres::storage::buffer::BufferManager;
    use bitpacking::{BitPacker, BitPacker1x, BitPacker4x, BitPacker8x};
    use pgrx::pg_sys;
    use std::fmt::{Debug, Formatter};

    #[rustfmt::skip]
    enum ChunkStyle {
        Sorted1x { num_bits: u8, initial: pg_sys::BlockNumber, bytes: Vec<u8> },
        Sorted4x { num_bits: u8, initial: pg_sys::BlockNumber, bytes: Vec<u8> },
        Sorted8x { num_bits: u8, initial: pg_sys::BlockNumber, bytes: Vec<u8> },
        StrictlySorted1x { num_bits: u8, initial: pg_sys::BlockNumber, bytes: Vec<u8> },
        StrictlySorted4x { num_bits: u8, initial: pg_sys::BlockNumber, bytes: Vec<u8> },
        StrictlySorted8x { num_bits: u8, initial: pg_sys::BlockNumber, bytes: Vec<u8> },
        Uncompressed(Vec<pg_sys::BlockNumber>),
    }

    impl Debug for ChunkStyle {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("ChunkStyle")
                .field("tag", &self.tag())
                .field("num_bits", &self.num_bits())
                .field("byte_len", &self.byte_len())
                .finish()
        }
    }

    impl ChunkStyle {
        pub fn tag(&self) -> ChunkStyleTag {
            match self {
                ChunkStyle::Sorted1x { .. } => ChunkStyleTag::Sorted1x,
                ChunkStyle::Sorted4x { .. } => ChunkStyleTag::Sorted4x,
                ChunkStyle::Sorted8x { .. } => ChunkStyleTag::Sorted8x,
                ChunkStyle::StrictlySorted1x { .. } => ChunkStyleTag::StrictlySorted1x,
                ChunkStyle::StrictlySorted4x { .. } => ChunkStyleTag::StrictlySorted4x,
                ChunkStyle::StrictlySorted8x { .. } => ChunkStyleTag::StrictlySorted8x,
                ChunkStyle::Uncompressed(_) => ChunkStyleTag::Uncompressed,
            }
        }

        pub fn byte_len(&self) -> usize {
            match self {
                ChunkStyle::Sorted1x { bytes, .. }
                | ChunkStyle::Sorted4x { bytes, .. }
                | ChunkStyle::Sorted8x { bytes, .. }
                | ChunkStyle::StrictlySorted1x { bytes, .. }
                | ChunkStyle::StrictlySorted4x { bytes, .. }
                | ChunkStyle::StrictlySorted8x { bytes, .. } => {
                    size_of::<u8>() // tag
                        + size_of::<u8>()   // num_bits
                        + size_of::<pg_sys::BlockNumber>()   // initial
                        + bytes.len()
                }
                ChunkStyle::Uncompressed(values) => {
                    size_of::<u8>() // tag
                        + size_of::<u8>() // len
                        + values.len() * size_of::<pg_sys::BlockNumber>()
                }
            }
        }

        pub fn num_bits(&self) -> u8 {
            match self {
                ChunkStyle::Sorted1x { num_bits, .. } => *num_bits,
                ChunkStyle::Sorted4x { num_bits, .. } => *num_bits,
                ChunkStyle::Sorted8x { num_bits, .. } => *num_bits,
                ChunkStyle::StrictlySorted1x { num_bits, .. } => *num_bits,
                ChunkStyle::StrictlySorted4x { num_bits, .. } => *num_bits,
                ChunkStyle::StrictlySorted8x { num_bits, .. } => *num_bits,
                ChunkStyle::Uncompressed(_) => u8::MAX,
            }
        }

        pub fn into_bytes(self) -> Vec<u8> {
            let tag = self.tag();
            match self {
                ChunkStyle::Sorted1x {
                    num_bits,
                    initial,
                    bytes,
                }
                | ChunkStyle::Sorted4x {
                    num_bits,
                    initial,
                    bytes,
                }
                | ChunkStyle::Sorted8x {
                    num_bits,
                    initial,
                    bytes,
                }
                | ChunkStyle::StrictlySorted1x {
                    num_bits,
                    initial,
                    bytes,
                }
                | ChunkStyle::StrictlySorted4x {
                    num_bits,
                    initial,
                    bytes,
                }
                | ChunkStyle::StrictlySorted8x {
                    num_bits,
                    initial,
                    bytes,
                } => std::iter::once(tag as u8)
                    .chain(std::iter::once(num_bits))
                    .chain(initial.to_le_bytes())
                    .chain(bytes)
                    .collect(),
                ChunkStyle::Uncompressed(values) => std::iter::once(tag as u8)
                    .chain((values.len() as u8).to_le_bytes())
                    .chain(values.into_iter().flat_map(|bn| bn.to_le_bytes()))
                    .collect(),
            }
        }
    }

    pub struct BlockList {
        chunks: Vec<ChunkStyle>,
        queue: Vec<pg_sys::BlockNumber>,
        last_chunked_blockno: Option<pg_sys::BlockNumber>,
    }

    impl Default for BlockList {
        fn default() -> Self {
            Self {
                chunks: Default::default(),
                queue: Vec::with_capacity(BitPacker8x::BLOCK_LEN),
                last_chunked_blockno: None,
            }
        }
    }

    impl Debug for BlockList {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("BlockList")
                .field("chunks", &self.chunks)
                .field("queue", &format!("len={}", self.queue.len()))
                .finish()
        }
    }

    impl BlockList {
        pub fn push(&mut self, block_number: pg_sys::BlockNumber) {
            assert!(block_number != 0, "cannot add block 0 to the blocklist");

            if let Some(last) = self.queue.last()
                && last == &block_number
            {
                // we just added this block
                return;
            }

            if self.queue.len() == BitPacker4x::BLOCK_LEN {
                self.chunks
                    .push(self.pack_4x(&self.queue, self.last_chunked_blockno));

                let last = self.queue.last().cloned();
                self.queue.clear();
                self.last_chunked_blockno = last;
            }

            self.queue.push(block_number);
        }

        pub fn finish(&mut self, bman: &mut BufferManager) -> Option<pg_sys::BlockNumber> {
            let mut queue = &self.queue[..];
            let mut last = self.last_chunked_blockno;
            while !queue.is_empty() {
                if queue.len() >= BitPacker8x::BLOCK_LEN {
                    let (head, tail) = queue.split_at(BitPacker8x::BLOCK_LEN);
                    self.chunks.push(self.pack_8x(head, last));

                    last = head.last().cloned();
                    queue = tail;
                } else if queue.len() >= BitPacker4x::BLOCK_LEN {
                    let (head, tail) = queue.split_at(BitPacker4x::BLOCK_LEN);
                    self.chunks.push(self.pack_4x(head, last));

                    last = head.last().cloned();
                    queue = tail;
                } else if queue.len() >= BitPacker1x::BLOCK_LEN {
                    let (head, tail) = queue.split_at(BitPacker1x::BLOCK_LEN);
                    self.chunks.push(self.pack_1x(head, last));

                    last = head.last().cloned();
                    queue = tail;
                } else {
                    self.chunks.push(ChunkStyle::Uncompressed(queue.to_vec()));
                    self.queue.clear();
                    break;
                }
            }

            let mut chunks = std::mem::take(&mut self.chunks).into_iter();
            let mut chunk = chunks.next()?;
            let mut block = bman.new_buffer();
            block.init_page();

            let starting_blockno = block.number();
            loop {
                let mut page = block.page_mut();

                if page.can_fit(chunk.byte_len()) {
                    // this chunk fits on this page, so write it there
                    // TODO:  can probably write directly to the slice rather than going through a Vec<u8>
                    let bytes = chunk.into_bytes();
                    page.append_bytes(&bytes);

                    chunk = match chunks.next() {
                        Some(chunk) => chunk,
                        None => break,
                    }
                } else {
                    // this chunk doesn't fit on this page, so allocate another page
                    let mut next_block = bman.new_buffer();
                    next_block.init_page();

                    // and link it to this one
                    page.special_mut::<BM25PageSpecialData>().next_blockno = next_block.number();

                    // and loop back around to write this chunk to the new page
                    block = next_block;
                }
            }

            Some(starting_blockno)
        }

        fn pack_8x(
            &self,
            slice: &[pg_sys::BlockNumber],
            initial: Option<pg_sys::BlockNumber>,
        ) -> ChunkStyle {
            let packer = BitPacker8x::new();
            if slice.is_sorted() {
                let num_bits = packer.num_bits_strictly_sorted(initial, slice);
                let mut bytes = vec![0u8; num_bits as usize * BitPacker8x::BLOCK_LEN / 8];
                packer.compress_strictly_sorted(initial, slice, &mut bytes, num_bits);
                ChunkStyle::StrictlySorted8x {
                    num_bits,
                    initial: initial.unwrap_or(0),
                    bytes,
                }
            } else {
                let num_bits = packer.num_bits_sorted(initial.unwrap_or(0), slice);
                let mut bytes = vec![0u8; num_bits as usize * BitPacker8x::BLOCK_LEN / 8];
                packer.compress_sorted(initial.unwrap_or(0), slice, &mut bytes, num_bits);
                ChunkStyle::Sorted8x {
                    num_bits,
                    initial: initial.unwrap_or(0),
                    bytes,
                }
            }
        }

        fn pack_4x(
            &self,
            slice: &[pg_sys::BlockNumber],
            initial: Option<pg_sys::BlockNumber>,
        ) -> ChunkStyle {
            let packer = BitPacker4x::new();
            if slice.is_sorted() {
                let num_bits = packer.num_bits_strictly_sorted(initial, slice);
                let mut bytes = vec![0u8; num_bits as usize * BitPacker4x::BLOCK_LEN / 8];
                packer.compress_strictly_sorted(initial, slice, &mut bytes, num_bits);
                ChunkStyle::StrictlySorted4x {
                    num_bits,
                    initial: initial.unwrap_or(0),
                    bytes,
                }
            } else {
                let num_bits = packer.num_bits_sorted(initial.unwrap_or(0), slice);
                let mut bytes = vec![0u8; num_bits as usize * BitPacker4x::BLOCK_LEN / 8];
                packer.compress_sorted(initial.unwrap_or(0), slice, &mut bytes, num_bits);
                ChunkStyle::Sorted4x {
                    num_bits,
                    initial: initial.unwrap_or(0),
                    bytes,
                }
            }
        }

        fn pack_1x(
            &self,
            slice: &[pg_sys::BlockNumber],
            initial: Option<pg_sys::BlockNumber>,
        ) -> ChunkStyle {
            let packer = BitPacker1x::new();
            if slice.is_sorted() {
                let num_bits = packer.num_bits_strictly_sorted(initial, slice);
                let mut bytes = vec![0u8; num_bits as usize * BitPacker1x::BLOCK_LEN / 8];
                packer.compress_strictly_sorted(initial, slice, &mut bytes, num_bits);
                ChunkStyle::StrictlySorted1x {
                    num_bits,
                    initial: initial.unwrap_or(0),
                    bytes,
                }
            } else {
                let num_bits = packer.num_bits_sorted(initial.unwrap_or(0), slice);
                let mut bytes = vec![0u8; num_bits as usize * BitPacker1x::BLOCK_LEN / 8];
                packer.compress_sorted(initial.unwrap_or(0), slice, &mut bytes, num_bits);
                ChunkStyle::Sorted1x {
                    num_bits,
                    initial: initial.unwrap_or(0),
                    bytes,
                }
            }
        }
    }
}

pub mod reader {
    use super::{ChunkStyleTag, Directory, chunk_size};
    use crate::postgres::storage::buffer::BufferManager;
    use bitpacking::{BitPacker, BitPacker1x, BitPacker4x, BitPacker8x};
    use pgrx::pg_sys;
    use std::collections::VecDeque;
    use std::ops::Range;
    use std::sync::Arc;

    const MAPPING_PAGE_CACHE_SIZE: usize = 4;

    #[derive(Debug)]
    pub struct BlockList {
        blocks: Vec<pg_sys::BlockNumber>,
        next_blockno: pg_sys::BlockNumber,
        directory: Option<Arc<Directory>>,
        pages: VecDeque<MappingPage>,
    }

    impl BlockList {
        pub fn new(starting_block: pg_sys::BlockNumber) -> Self {
            Self {
                blocks: Vec::new(),
                next_blockno: starting_block,
                directory: None,
                pages: VecDeque::new(),
            }
        }

        pub fn with_directory(mut self, directory: Option<Arc<Directory>>) -> Self {
            self.directory = directory;
            self
        }

        fn read_next_page(&mut self, bman: &BufferManager) {
            let block = bman.get_buffer(self.next_blockno);
            let page = block.page();
            let mut bytes = page.as_slice();
            while !bytes.is_empty() {
                let consumed = decode_chunk(bytes, &mut self.blocks);
                bytes = &bytes[consumed..];
            }
            self.next_blockno = page.next_blockno();
        }

        pub fn get(&mut self, bman: &BufferManager, i: usize) -> Option<pg_sys::BlockNumber> {
            if let Some(directory) = &self.directory {
                if i >= directory.total_entries as usize {
                    return None;
                }
                if let Some(page) = self.pages.back_mut()
                    && page.ordinals.contains(&i)
                {
                    return page.get(i);
                }
                let pos = directory
                    .entries
                    .partition_point(|entry| entry.first_ordinal as usize <= i)
                    - 1;
                let entry = &directory.entries[pos];
                if let Some(cached) = self.pages.iter().position(|page| page.block == entry.block) {
                    let page = self.pages.remove(cached).unwrap();
                    self.pages.push_back(page);
                } else {
                    let buffer = bman.get_buffer(entry.block);
                    let page = MappingPage::new(
                        entry.block,
                        entry.first_ordinal as usize,
                        buffer.page().as_slice().to_vec(),
                    );
                    if self.pages.len() == MAPPING_PAGE_CACHE_SIZE {
                        self.pages.pop_front();
                    }
                    self.pages.push_back(page);
                }
                return self.pages.back_mut().unwrap().get(i);
            }
            while self.blocks.len() <= i && self.next_blockno != pg_sys::InvalidBlockNumber {
                self.read_next_page(bman);
            }
            self.blocks.get(i).copied()
        }

        pub fn into_blocks(
            mut self,
            bman: &BufferManager,
        ) -> std::vec::IntoIter<pg_sys::BlockNumber> {
            while self.next_blockno != pg_sys::InvalidBlockNumber {
                self.read_next_page(bman);
            }
            self.blocks.into_iter()
        }
    }

    #[derive(Debug)]
    struct MappingPage {
        block: pg_sys::BlockNumber,
        ordinals: Range<usize>,
        bytes: Vec<u8>,
        chunks: Vec<(usize, usize)>,
        decoded_chunk: Option<usize>,
        decoded: Vec<pg_sys::BlockNumber>,
    }

    impl MappingPage {
        fn new(block: pg_sys::BlockNumber, first_ordinal: usize, bytes: Vec<u8>) -> Self {
            let mut chunks = Vec::new();
            let mut ordinal = first_ordinal;
            let mut offset = 0;
            while offset < bytes.len() {
                chunks.push((ordinal, offset));
                let (count, len) = chunk_size(&bytes[offset..]);
                ordinal += count;
                offset += len;
            }
            Self {
                block,
                ordinals: first_ordinal..ordinal,
                bytes,
                chunks,
                decoded_chunk: None,
                decoded: Vec::new(),
            }
        }

        fn get(&mut self, ordinal: usize) -> Option<pg_sys::BlockNumber> {
            if let Some(chunk) = self.decoded_chunk {
                let first = self.chunks[chunk].0;
                if ordinal >= first && ordinal - first < self.decoded.len() {
                    return Some(self.decoded[ordinal - first]);
                }
            }
            let chunk = self
                .chunks
                .partition_point(|&(start, _)| start <= ordinal)
                .checked_sub(1)?;
            let (first, offset) = self.chunks[chunk];
            if self.decoded_chunk != Some(chunk) {
                self.decoded.clear();
                decode_chunk(&self.bytes[offset..], &mut self.decoded);
                self.decoded_chunk = Some(chunk);
            }
            self.decoded.get(ordinal - first).copied()
        }
    }

    fn decode_chunk(slice: &[u8], blocks: &mut Vec<pg_sys::BlockNumber>) -> usize {
        let (_, expected) = chunk_size(slice);
        let mut offset = 0;
        let tag = ChunkStyleTag::from(slice[offset]);
        offset += 1;

        match tag {
            tag @ ChunkStyleTag::Sorted1x
            | tag @ ChunkStyleTag::Sorted4x
            | tag @ ChunkStyleTag::Sorted8x
            | tag @ ChunkStyleTag::StrictlySorted1x
            | tag @ ChunkStyleTag::StrictlySorted4x
            | tag @ ChunkStyleTag::StrictlySorted8x => {
                let num_bits = slice[offset];
                offset += 1;
                let initial = u32::from_le_bytes(
                    slice[offset..offset + size_of::<pg_sys::BlockNumber>()]
                        .try_into()
                        .unwrap(),
                );
                offset += size_of::<pg_sys::BlockNumber>();
                let end = blocks.len();
                match tag {
                    ChunkStyleTag::Sorted1x => {
                        blocks.extend_from_slice(&[0; BitPacker1x::BLOCK_LEN]);
                        offset += BitPacker1x::new().decompress_sorted(
                            initial,
                            &slice[offset..],
                            &mut blocks[end..],
                            num_bits,
                        );
                    }
                    ChunkStyleTag::Sorted4x => {
                        blocks.extend_from_slice(&[0; BitPacker4x::BLOCK_LEN]);
                        offset += BitPacker4x::new().decompress_sorted(
                            initial,
                            &slice[offset..],
                            &mut blocks[end..],
                            num_bits,
                        );
                    }
                    ChunkStyleTag::Sorted8x => {
                        blocks.extend_from_slice(&[0; BitPacker8x::BLOCK_LEN]);
                        offset += BitPacker8x::new().decompress_sorted(
                            initial,
                            &slice[offset..],
                            &mut blocks[end..],
                            num_bits,
                        );
                    }
                    ChunkStyleTag::StrictlySorted1x => {
                        blocks.extend_from_slice(&[0; BitPacker1x::BLOCK_LEN]);
                        offset += BitPacker1x::new().decompress_strictly_sorted(
                            (initial != 0).then_some(initial),
                            &slice[offset..],
                            &mut blocks[end..],
                            num_bits,
                        );
                    }
                    ChunkStyleTag::StrictlySorted4x => {
                        blocks.extend_from_slice(&[0; BitPacker4x::BLOCK_LEN]);
                        offset += BitPacker4x::new().decompress_strictly_sorted(
                            (initial != 0).then_some(initial),
                            &slice[offset..],
                            &mut blocks[end..],
                            num_bits,
                        );
                    }
                    ChunkStyleTag::StrictlySorted8x => {
                        blocks.extend_from_slice(&[0; BitPacker8x::BLOCK_LEN]);
                        offset += BitPacker8x::new().decompress_strictly_sorted(
                            (initial != 0).then_some(initial),
                            &slice[offset..],
                            &mut blocks[end..],
                            num_bits,
                        );
                    }
                    _ => unreachable!(),
                }
            }
            ChunkStyleTag::Uncompressed => {
                let len = slice[offset] as usize;
                offset += 1;
                let mut tmp = [0u8; size_of::<pg_sys::BlockNumber>()];
                for _ in 0..len {
                    tmp.copy_from_slice(&slice[offset..offset + size_of::<pg_sys::BlockNumber>()]);
                    offset += size_of::<pg_sys::BlockNumber>();
                    let value = u32::from_le_bytes(tmp);
                    blocks.push(value);
                }
            }
        }
        assert_eq!(offset, expected);
        offset
    }

    #[cfg(any(test, feature = "pg_test"))]
    #[pgrx::pg_schema]
    mod tests {
        use super::*;
        use crate::postgres::rel::PgSearchRelation;
        use crate::postgres::storage::blocklist::builder;
        use pgrx::prelude::*;

        #[pg_test]
        fn test_blocklist_directory_lookup() {
            Spi::run("CREATE TABLE directory_lookup (id SERIAL, data TEXT)").unwrap();
            Spi::run("CREATE INDEX directory_lookup_idx ON directory_lookup USING bm25 (id, data) WITH (key_field='id')").unwrap();
            let oid = Spi::get_one::<pg_sys::Oid>("SELECT 'directory_lookup_idx'::regclass::oid")
                .unwrap()
                .unwrap();
            let indexrel = PgSearchRelation::open(oid);
            let mut bman = BufferManager::new(&indexrel);
            assert!(Directory::build(&bman, pg_sys::InvalidBlockNumber).is_none());
            let blocks: Vec<u32> = (1u32..30_074)
                .map(|i| i.wrapping_mul(2_654_435_761))
                .collect();
            let mut builder = builder::BlockList::default();
            for &block in &blocks {
                builder.push(block);
            }
            let start = builder.finish(&mut bman).unwrap();
            let directory = Arc::new(Directory::build(&bman, start).unwrap());
            assert!(directory.entries.len() > 4);
            let mut reader = BlockList::new(start).with_directory(Some(directory.clone()));
            let before = unsafe {
                pg_sys::pgBufferUsage.shared_blks_hit + pg_sys::pgBufferUsage.shared_blks_read
            };
            assert_eq!(reader.get(&bman, blocks.len() - 1), blocks.last().copied());
            let after = unsafe {
                pg_sys::pgBufferUsage.shared_blks_hit + pg_sys::pgBufferUsage.shared_blks_read
            };
            assert_eq!(after - before, 1);
            assert!(reader.blocks.is_empty());
            assert!(reader.pages.back().unwrap().decoded.len() <= BitPacker8x::BLOCK_LEN);
            for entry in &directory.entries {
                let i = entry.first_ordinal as usize;
                for i in [i.saturating_sub(1), i, (i + 1).min(blocks.len() - 1)] {
                    assert_eq!(reader.get(&bman, i), Some(blocks[i]));
                }
            }
            for step in 0..blocks.len() {
                let i = step.wrapping_mul(7919) % blocks.len();
                assert_eq!(reader.get(&bman, i), Some(blocks[i]));
                assert!(reader.pages.len() <= 4);
            }
            assert_eq!(reader.get(&bman, blocks.len()), None);
            assert_eq!(reader.get(&bman, usize::MAX), None);
            assert_eq!(reader.into_blocks(&bman).collect::<Vec<_>>(), blocks);
            assert_eq!(
                BlockList::new(start).into_blocks(&bman).collect::<Vec<_>>(),
                blocks
            );
        }

        #[pg_test]
        fn test_blocklist_directory_overflow() {
            use crate::postgres::storage::blocklist::MAX_DIRECTORY_ENTRIES;
            Spi::run("CREATE TABLE directory_overflow (id SERIAL, data TEXT)").unwrap();
            Spi::run("CREATE INDEX directory_overflow_idx ON directory_overflow USING bm25 (id, data) WITH (key_field='id')").unwrap();
            let oid = Spi::get_one::<pg_sys::Oid>("SELECT 'directory_overflow_idx'::regclass::oid")
                .unwrap()
                .unwrap();
            let rel = PgSearchRelation::open(oid);
            let mut bman = BufferManager::new(&rel);
            let count = MAX_DIRECTORY_ENTRIES as u32 * 2048 + 17;
            let mut builder = builder::BlockList::default();
            for i in 1..=count {
                builder.push(i.wrapping_mul(2_654_435_761));
            }
            let start = builder.finish(&mut bman).unwrap();
            assert!(Directory::build(&bman, start).is_none());
            let mut reader = BlockList::new(start);
            assert_eq!(
                reader.get(&bman, count as usize - 1),
                Some(count.wrapping_mul(2_654_435_761))
            );
        }

        #[pg_test]
        fn test_blocklist_directory_format() {
            use crate::postgres::storage::block::LinkedListData;
            use crate::postgres::storage::blocklist::{DirectoryEntry, MAX_DIRECTORY_ENTRIES};
            let directory = Directory {
                entries: vec![
                    DirectoryEntry {
                        first_ordinal: 0,
                        block: 900,
                    },
                    DirectoryEntry {
                        first_ordinal: 100,
                        block: 2700,
                    },
                ],
                total_entries: 150,
            };
            let prefix = vec![0u8; size_of::<LinkedListData>()];
            assert!(Directory::read(&prefix, 900).is_none());
            let mut bytes = prefix.clone();
            bytes.extend_from_slice(&directory.encode());
            assert_eq!(Directory::read(&bytes, 900), Some(directory.clone()));
            for len in prefix.len()..bytes.len() {
                assert!(Directory::read(&bytes[..len], 900).is_none());
            }
            for (offset, value) in [
                (16, 0),
                (20, 2),
                (24, u32::MAX),
                (28, 0),
                (32, 1),
                (36, 0),
                (40, 0),
            ] {
                let mut invalid = bytes.clone();
                invalid[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
                assert!(Directory::read(&invalid, 900).is_none());
            }
            assert!(Directory::read(&bytes, 901).is_none());
            let largest = Directory {
                entries: (0..MAX_DIRECTORY_ENTRIES as u32)
                    .map(|i| DirectoryEntry {
                        first_ordinal: i,
                        block: i + 1,
                    })
                    .collect(),
                total_entries: MAX_DIRECTORY_ENTRIES as u32,
            };
            let mut bytes = prefix;
            bytes.extend_from_slice(&largest.encode());
            assert_eq!(
                bytes.len(),
                crate::postgres::storage::block::bm25_max_free_space()
            );
            assert_eq!(Directory::read(&bytes, 1), Some(largest));
        }

        #[pg_test]
        fn test_blocklist_chunk_encodings() {
            macro_rules! check {
                ($packer:ty, $sorted:expr, $strict:expr) => {
                    for strict in [false, true] {
                        let packer = <$packer>::new();
                        for (initial, first, step) in [
                            (Some(90), 90 + u32::from(strict), u32::from(strict)),
                            (Some(90), 100, 3),
                            (Some(90), u32::MAX - 300, 1),
                            (None, 1, 1),
                        ] {
                            let values: Vec<u32> = (0..<$packer>::BLOCK_LEN)
                                .map(|i| first + i as u32 * step)
                                .collect();
                            let bits = if strict {
                                packer.num_bits_strictly_sorted(initial, &values)
                            } else {
                                packer.num_bits_sorted(initial.unwrap_or(0), &values)
                            };
                            let mut encoded = vec![0; values.len() * bits as usize / 8];
                            if strict {
                                packer.compress_strictly_sorted(
                                    initial,
                                    &values,
                                    &mut encoded,
                                    bits,
                                );
                            } else {
                                packer.compress_sorted(
                                    initial.unwrap_or(0),
                                    &values,
                                    &mut encoded,
                                    bits,
                                );
                            }
                            let mut bytes = vec![if strict { $strict } else { $sorted }, bits];
                            bytes.extend_from_slice(&initial.unwrap_or(0).to_le_bytes());
                            bytes.extend_from_slice(&encoded);
                            let mut page = MappingPage::new(1, 1000, bytes);
                            for (i, value) in values.iter().enumerate().rev() {
                                assert_eq!(page.get(1000 + i), Some(*value));
                            }
                            assert_eq!(page.get(999), None);
                            assert_eq!(page.get(1000 + values.len()), None);
                        }
                    }
                };
            }
            check!(BitPacker1x, 0, 3);
            check!(BitPacker4x, 1, 4);
            check!(BitPacker8x, 2, 5);
            let mut bytes = vec![6, 3];
            for n in [19u32, 2, 73] {
                bytes.extend_from_slice(&n.to_le_bytes());
            }
            let mut page = MappingPage::new(1, 0, bytes);
            assert_eq!(page.get(2), Some(73));
            assert_eq!(page.get(0), Some(19));
            assert_eq!(page.get(3), None);
        }

        #[pg_test]
        fn test_blocklist_lazy_pages() {
            Spi::run("CREATE TABLE t (id SERIAL, data TEXT)").unwrap();
            Spi::run("CREATE INDEX t_idx ON t USING paradedb (id, data)").unwrap();
            let oid = Spi::get_one::<pg_sys::Oid>("SELECT 't_idx'::regclass::oid")
                .unwrap()
                .unwrap();
            let indexrel = PgSearchRelation::open(oid);
            let mut bman = BufferManager::new(&indexrel);
            let blocks: Vec<u32> = (1u32..10_074)
                .map(|i| i.wrapping_mul(2_654_435_761))
                .collect();
            let mut builder = builder::BlockList::default();
            for &block in &blocks {
                builder.push(block);
            }
            let start = builder.finish(&mut bman).unwrap();

            let mut reader = BlockList::new(start);
            assert_eq!(reader.get(&bman, 0), Some(blocks[0]));
            assert!(reader.blocks.len() < blocks.len());
            assert_ne!(reader.next_blockno, pg_sys::InvalidBlockNumber);
            assert_eq!(reader.into_blocks(&bman).collect::<Vec<_>>(), blocks);

            let mut reader = BlockList::new(start);
            for i in [0, 32, 1024, 5000, 4096, blocks.len() - 1, 1] {
                assert_eq!(reader.get(&bman, i), Some(blocks[i]));
            }
            assert_eq!(reader.get(&bman, blocks.len()), None);
            assert_eq!(reader.get(&bman, usize::MAX), None);
            assert_eq!(reader.into_blocks(&bman).collect::<Vec<_>>(), blocks);
            assert_eq!(
                BlockList::new(pg_sys::InvalidBlockNumber).get(&bman, 0),
                None
            );
        }
    }
}
