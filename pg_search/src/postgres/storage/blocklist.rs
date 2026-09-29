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

mod directory;
mod tree;
pub use directory::Directory;

type ChunkBlockCount = usize;
type ChunkByteLength = usize;

/// Reads a chunk header to get its mapped block count and encoded byte length.
fn chunk_size(bytes: &[u8]) -> (ChunkBlockCount, ChunkByteLength) {
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
    use super::tree::Entry;
    use super::{Directory, chunk_size};
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

        pub fn finish(
            &mut self,
            bman: &mut BufferManager,
        ) -> Option<(pg_sys::BlockNumber, Directory, pg_sys::BlockNumber)> {
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
            let mut ordinal = 0u32;
            let mut entries = vec![Entry {
                start: ordinal,
                address: starting_blockno,
            }];
            loop {
                let mut page = block.page_mut();

                if page.can_fit(chunk.byte_len()) {
                    // this chunk fits on this page, so write it there
                    // TODO:  can probably write directly to the slice rather than going through a Vec<u8>
                    let bytes = chunk.into_bytes();
                    page.append_bytes(&bytes);
                    ordinal = ordinal
                        .checked_add(chunk_size(&bytes).0 as u32)
                        .expect("too many blocks in component");

                    chunk = match chunks.next() {
                        Some(chunk) => chunk,
                        None => break,
                    }
                } else {
                    pgrx::check_for_interrupts!();
                    // this chunk doesn't fit on this page, so allocate another page
                    let mut next_block = bman.new_buffer();
                    next_block.init_page();

                    // and link it to this one
                    page.special_mut::<BM25PageSpecialData>().next_blockno = next_block.number();
                    entries.push(Entry {
                        start: ordinal,
                        address: next_block.number(),
                    });

                    // and loop back around to write this chunk to the new page
                    block = next_block;
                }
            }

            drop(block);
            let (directory, overflow) = Directory::build(bman, entries);
            Some((starting_blockno, directory, overflow))
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
    use crate::postgres::storage::blocklist::tree::{
        Bounds, InvalidNode, Node, PageReader, TreeReader,
    };
    use crate::postgres::storage::buffer::BufferManager;
    use bitpacking::{BitPacker, BitPacker1x, BitPacker4x, BitPacker8x};
    use pgrx::pg_sys;
    use std::ops::Range;
    use std::sync::Arc;

    #[derive(Debug)]
    pub struct BlockList {
        blocks: Vec<pg_sys::BlockNumber>,
        next_blockno: pg_sys::BlockNumber,
        directory: Option<TreeReader<u32, u32, MappingPage>>,
    }

    impl BlockList {
        pub fn new(starting_block: pg_sys::BlockNumber) -> Self {
            Self {
                blocks: Vec::new(),
                next_blockno: starting_block,
                directory: None,
            }
        }

        pub fn with_directory(mut self, directory: Option<Directory>, end: Option<usize>) -> Self {
            self.directory = directory.and_then(|directory| {
                let end = end.and_then(|end| u32::try_from(end).ok());
                TreeReader::new(directory.node, end).ok()
            });
            self
        }

        fn read_next_page(&mut self, bman: &BufferManager) {
            let block = read_map_page(bman, self.next_blockno);
            let page = block.page();
            let mut bytes = page.as_slice();
            while !bytes.is_empty() {
                let consumed = decode_chunk(bytes, &mut self.blocks);
                bytes = &bytes[consumed..];
            }
            self.next_blockno = page.next_blockno();
        }

        pub fn get(&mut self, bman: &BufferManager, i: usize) -> Option<pg_sys::BlockNumber> {
            if let Some(directory) = &mut self.directory {
                let ordinal = u32::try_from(i).ok()?;
                match directory.get(&MappingReader(bman), ordinal) {
                    Ok(page) => return page.and_then(|page| page.get(i)),
                    Err(InvalidNode) => self.directory = None,
                }
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
            self.directory = None;
            while self.next_blockno != pg_sys::InvalidBlockNumber {
                self.read_next_page(bman);
            }
            self.blocks.into_iter()
        }
    }

    fn read_map_page(
        bman: &BufferManager,
        block: pg_sys::BlockNumber,
    ) -> crate::postgres::storage::buffer::Buffer {
        #[cfg(feature = "io_stats")]
        let _scope = crate::index::reader::io_stats::trace::block_map();
        bman.get_buffer(block)
    }

    struct MappingReader<'a>(&'a BufferManager);

    impl PageReader<u32, u32> for MappingReader<'_> {
        type Leaf = MappingPage;

        fn read_node(
            &self,
            address: u32,
            _bounds: Bounds<u32>,
        ) -> Result<Arc<Node<u32, u32>>, InvalidNode> {
            let buffer = read_map_page(self.0, address);
            let directory = Directory::decode(buffer.page().as_slice()).ok_or(InvalidNode)?;
            Ok(directory.node)
        }

        fn read_leaf(&self, address: u32, bounds: Bounds<u32>) -> Result<MappingPage, InvalidNode> {
            let buffer = read_map_page(self.0, address);
            let mapping =
                MappingPage::new(bounds.start as usize, buffer.page().as_slice().to_vec());
            if bounds
                .end
                .is_some_and(|end| mapping.ordinals.end != end as usize)
            {
                return Err(InvalidNode);
            }
            Ok(mapping)
        }
    }

    #[derive(Debug)]
    struct MappingPage {
        ordinals: Range<usize>,
        bytes: Vec<u8>,
        chunks: Vec<MappingChunk>,
        current_chunk: Option<usize>,
        #[cfg(any(test, feature = "pg_test"))]
        chunk_decodes: usize,
    }

    #[derive(Debug)]
    struct MappingChunk {
        start: usize,
        offset: usize,
        decoded: Option<Box<[pg_sys::BlockNumber]>>,
    }

    impl MappingPage {
        fn new(first_ordinal: usize, bytes: Vec<u8>) -> Self {
            let mut chunks = Vec::new();
            let mut ordinal = first_ordinal;
            let mut offset = 0;
            while offset < bytes.len() {
                chunks.push(MappingChunk {
                    start: ordinal,
                    offset,
                    decoded: None,
                });
                let (count, len) = chunk_size(&bytes[offset..]);
                ordinal += count;
                offset += len;
            }
            Self {
                ordinals: first_ordinal..ordinal,
                bytes,
                chunks,
                current_chunk: None,
                #[cfg(any(test, feature = "pg_test"))]
                chunk_decodes: 0,
            }
        }

        fn get(&mut self, ordinal: usize) -> Option<pg_sys::BlockNumber> {
            if !self.ordinals.contains(&ordinal) {
                return None;
            }
            if let Some(chunk) = self.current_chunk {
                let chunk = &self.chunks[chunk];
                if ordinal >= chunk.start
                    && let Some(block) = chunk.decoded.as_ref()?.get(ordinal - chunk.start)
                {
                    return Some(*block);
                }
            }
            let chunk = self
                .chunks
                .partition_point(|chunk| chunk.start <= ordinal)
                .checked_sub(1)?;
            let entry = &mut self.chunks[chunk];
            let decoded = entry.decoded.get_or_insert_with(|| {
                let bytes = &self.bytes[entry.offset..];
                let mut decoded = Vec::with_capacity(chunk_size(bytes).0);
                decode_chunk(bytes, &mut decoded);
                #[cfg(any(test, feature = "pg_test"))]
                {
                    self.chunk_decodes += 1;
                }
                decoded.into_boxed_slice()
            });
            self.current_chunk = Some(chunk);
            decoded.get(ordinal - entry.start).copied()
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
            assert!(builder::BlockList::default().finish(&mut bman).is_none());
            let blocks: Vec<u32> = (1u32..30_074)
                .map(|i| i.wrapping_mul(2_654_435_761))
                .collect();
            let mut builder = builder::BlockList::default();
            for &block in &blocks {
                builder.push(block);
            }
            let (start, directory, overflow) = builder.finish(&mut bman).unwrap();
            assert_eq!(overflow, pg_sys::InvalidBlockNumber);
            assert_eq!(directory.node.level, 0);
            assert!(directory.node.entries.len() > 4);
            let entries = directory.node.entries.clone();
            let mut reader = BlockList::new(start).with_directory(Some(directory), None);
            let before = unsafe {
                pg_sys::pgBufferUsage.shared_blks_hit + pg_sys::pgBufferUsage.shared_blks_read
            };
            assert_eq!(reader.get(&bman, blocks.len() - 1), blocks.last().copied());
            let after = unsafe {
                pg_sys::pgBufferUsage.shared_blks_hit + pg_sys::pgBufferUsage.shared_blks_read
            };
            assert_eq!(after - before, 1);
            assert!(reader.blocks.is_empty());
            for entry in &entries {
                let i = entry.start as usize;
                for i in [i.saturating_sub(1), i, (i + 1).min(blocks.len() - 1)] {
                    assert_eq!(reader.get(&bman, i), Some(blocks[i]));
                }
            }
            for step in 0..blocks.len() {
                let i = step.wrapping_mul(7919) % blocks.len();
                assert_eq!(reader.get(&bman, i), Some(blocks[i]));
            }
            let after = unsafe {
                pg_sys::pgBufferUsage.shared_blks_hit + pg_sys::pgBufferUsage.shared_blks_read
            };
            assert_eq!(after - before, entries.len() as i64);
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
            use crate::postgres::storage::LinkedBytesList;
            use crate::postgres::storage::block::{
                BM25PageSpecialData, LinkedList, LinkedListData,
            };
            use crate::postgres::storage::blocklist::directory::ROOT_CAPACITY;
            use crate::postgres::storage::blocklist::tree::Entry;
            Spi::run("CREATE TABLE directory_overflow (id SERIAL, data TEXT)").unwrap();
            Spi::run("CREATE INDEX directory_overflow_idx ON directory_overflow USING bm25 (id, data) WITH (key_field='id')").unwrap();
            let oid = Spi::get_one::<pg_sys::Oid>("SELECT 'directory_overflow_idx'::regclass::oid")
                .unwrap()
                .unwrap();
            let rel = PgSearchRelation::open(oid);
            let mut bman = BufferManager::new(&rel);
            let count = ROOT_CAPACITY as u32 * 2048 + 17;
            let blocks: Vec<_> = (1..=count).map(|i| i.wrapping_mul(2_654_435_761)).collect();
            let mut builder = builder::BlockList::default();
            for &block in &blocks {
                builder.push(block);
            }
            let (start, mut directory, mut overflow) = builder.finish(&mut bman).unwrap();
            assert_eq!(directory.node.level, 1);
            assert!(directory.node.entries.len() > 1);
            let mut reader =
                BlockList::new(start).with_directory(Some(directory.clone()), Some(count as usize));
            let before = unsafe {
                pg_sys::pgBufferUsage.shared_blks_hit + pg_sys::pgBufferUsage.shared_blks_read
            };
            assert_eq!(reader.get(&bman, blocks.len() - 1), blocks.last().copied());
            assert_eq!(reader.get(&bman, blocks.len() - 1), blocks.last().copied());
            let after = unsafe {
                pg_sys::pgBufferUsage.shared_blks_hit + pg_sys::pgBufferUsage.shared_blks_read
            };
            assert_eq!(after - before, 2);
            for entry in &directory.node.entries {
                let i = entry.start as usize;
                for i in [i.saturating_sub(1), i, (i + 1).min(blocks.len() - 1)] {
                    assert_eq!(reader.get(&bman, i), Some(blocks[i]));
                }
            }
            for step in 0usize..1000 {
                let i = step.wrapping_mul(7919) % blocks.len();
                assert_eq!(reader.get(&bman, i), Some(blocks[i]));
            }
            assert_eq!(reader.get(&bman, blocks.len()), None);
            assert_eq!(reader.into_blocks(&bman).collect::<Vec<_>>(), blocks);
            assert_eq!(
                BlockList::new(start).into_blocks(&bman).collect::<Vec<_>>(),
                blocks
            );

            let leaf_block = directory.node.entries.last().unwrap().address;
            let buffer = bman.get_buffer(leaf_block);
            let original = buffer.page().as_slice().to_vec();
            let next = buffer.page().next_blockno();
            drop(buffer);
            let mut invalid = original.clone();
            invalid[4..8].copy_from_slice(&99u32.to_le_bytes());
            {
                let mut buffer = bman.get_buffer_mut(leaf_block);
                let mut page = buffer.init_page();
                assert!(page.append_bytes(&invalid));
                page.special_mut::<BM25PageSpecialData>().next_blockno = next;
            }
            let mut reader =
                BlockList::new(start).with_directory(Some(directory.clone()), Some(count as usize));
            assert_eq!(reader.get(&bman, blocks.len() - 1), blocks.last().copied());
            assert!(reader.directory.is_none());
            {
                let mut buffer = bman.get_buffer_mut(leaf_block);
                let mut page = buffer.init_page();
                assert!(page.append_bytes(&original));
                page.special_mut::<BM25PageSpecialData>().next_blockno = next;
            }

            for level in 2..=3 {
                let mut buffer = bman.new_buffer();
                let block = buffer.number();
                let mut page = buffer.init_page();
                assert!(page.append_bytes(&directory.encode()));
                page.special_mut::<BM25PageSpecialData>().next_blockno = overflow;
                overflow = block;
                directory = Directory {
                    node: Arc::new(Node {
                        entries: vec![Entry {
                            start: 0,
                            address: block,
                        }],
                        level,
                    }),
                };
            }
            let mut reader =
                BlockList::new(start).with_directory(Some(directory.clone()), Some(count as usize));
            let before = unsafe {
                pg_sys::pgBufferUsage.shared_blks_hit + pg_sys::pgBufferUsage.shared_blks_read
            };
            assert_eq!(reader.get(&bman, blocks.len() - 1), blocks.last().copied());
            let after = unsafe {
                pg_sys::pgBufferUsage.shared_blks_hit + pg_sys::pgBufferUsage.shared_blks_read
            };
            assert_eq!(after - before, 4);

            let list = LinkedBytesList::create_with_fsm(&rel);
            {
                let mut buffer = bman.get_buffer_mut(list.header_blockno);
                let mut page = buffer.page_mut();
                let metadata = page.contents_mut::<LinkedListData>();
                metadata.blocklist_start = start;
                assert!(page.append_bytes(&directory.encode()));
                page.special_mut::<BM25PageSpecialData>().next_blockno = overflow;
            }
            let mut expected = blocks;
            expected.push(list.header_blockno);
            for mut block in [overflow, start] {
                while block != pg_sys::InvalidBlockNumber {
                    expected.push(block);
                    block = bman.get_buffer(block).page().next_blockno();
                }
            }
            assert_eq!(
                list.block_for_ord(count as usize - 1),
                expected.get(count as usize - 1).copied()
            );
            assert_eq!(list.freeable_blocks().collect::<Vec<_>>(), expected);
        }

        #[pg_test]
        fn test_blocklist_directory_format() {
            use crate::postgres::storage::block::{LinkedListData, bm25_max_free_space};
            use crate::postgres::storage::blocklist::directory::{MAX_LEVEL, ROOT_CAPACITY};
            use crate::postgres::storage::blocklist::tree::Entry;
            let directory = Directory {
                node: Arc::new(Node {
                    entries: vec![
                        Entry {
                            start: 0,
                            address: 900,
                        },
                        Entry {
                            start: 100,
                            address: 2700,
                        },
                    ],
                    level: 0,
                }),
            };
            let prefix = vec![0u8; size_of::<LinkedListData>()];
            assert!(Directory::read(&prefix, 900).is_none());
            let mut expected = b"BDIR".to_vec();
            for word in [2u32, 0, 0, 900, 100, 2700] {
                expected.extend_from_slice(&word.to_le_bytes());
            }
            assert_eq!(directory.encode(), expected);
            for version in [0u32, 1, 3, u32::MAX] {
                let mut bytes = prefix.clone();
                bytes.extend_from_slice(&expected);
                let version_offset = prefix.len() + 4;
                bytes[version_offset..version_offset + 4].copy_from_slice(&version.to_le_bytes());
                assert!(Directory::read(&bytes, 900).is_none());
            }

            let largest = Directory {
                node: Arc::new(Node {
                    entries: (0..ROOT_CAPACITY as u32)
                        .map(|i| Entry {
                            start: i,
                            address: i + 1,
                        })
                        .collect(),
                    level: 1,
                }),
            };
            let mut bytes = prefix.clone();
            bytes.extend_from_slice(&largest.encode());
            assert!(bytes.len() <= bm25_max_free_space());
            assert!(bm25_max_free_space() - bytes.len() < 8);
            assert_eq!(Directory::read(&bytes, 1), Some(largest));
            for level in 0..=MAX_LEVEL {
                let node = Directory {
                    node: Arc::new(Node {
                        level,
                        ..(*directory.node).clone()
                    }),
                };
                let mut bytes = prefix.clone();
                bytes.extend_from_slice(&node.encode());
                assert_eq!(Directory::read(&bytes, 900), Some(node));
                if level == 0 {
                    assert!(Directory::read(&bytes, 901).is_none());
                }
                for len in prefix.len()..bytes.len() {
                    // Whole-entry prefixes are valid; pd_lower supplies the entry count.
                    if len < prefix.len() + 20 || !(len - prefix.len() - 12).is_multiple_of(8) {
                        assert!(Directory::read(&bytes[..len], 900).is_none());
                    }
                }
                for (offset, value) in [
                    (16, 0),
                    (20, 99),
                    (24, MAX_LEVEL + 1),
                    (28, 1),
                    (32, 0),
                    (36, 0),
                ] {
                    let mut invalid = bytes.clone();
                    invalid[offset..offset + 4].copy_from_slice(&u32::to_le_bytes(value));
                    assert!(Directory::read(&invalid, 900).is_none());
                }
            }
        }

        #[pg_test]
        fn test_blocklist_interleaved_chunks() {
            const STREAMS: usize = 32;
            const ROUNDS: usize = 100;
            const FIRST: usize = 10_000;
            let packer = BitPacker4x::new();
            let mut bytes = Vec::new();
            for stream in 0..STREAMS {
                let initial = (stream * BitPacker4x::BLOCK_LEN * 3) as u32;
                let values: Vec<_> = (1..=BitPacker4x::BLOCK_LEN)
                    .map(|i| initial + i as u32 * 3)
                    .collect();
                let previous = (initial != 0).then_some(initial);
                let bits = packer.num_bits_strictly_sorted(previous, &values);
                let mut encoded = vec![0; values.len() * bits as usize / 8];
                packer.compress_strictly_sorted(previous, &values, &mut encoded, bits);
                bytes.extend_from_slice(&[ChunkStyleTag::StrictlySorted4x as u8, bits]);
                bytes.extend_from_slice(&initial.to_le_bytes());
                bytes.extend_from_slice(&encoded);
            }
            assert!(bytes.len() <= crate::postgres::storage::block::bm25_max_free_space());
            let mut page = MappingPage::new(FIRST, bytes.clone());
            assert_eq!(page.chunk_decodes, 0);
            for round in 0..ROUNDS {
                for stream in 0..STREAMS {
                    let offset = stream * BitPacker4x::BLOCK_LEN + round;
                    assert_eq!(page.get(FIRST + offset), Some((offset as u32 + 1) * 3));
                }
                assert_eq!(page.chunk_decodes, STREAMS);
            }
            pgrx::notice!(
                "{} interleaved lookups, {} chunk decodes",
                STREAMS * ROUNDS,
                page.chunk_decodes
            );

            let mut sparse = MappingPage::new(FIRST, bytes);
            let last = STREAMS * BitPacker4x::BLOCK_LEN - 1;
            for offset in [last, 0, last, 1, last - 1] {
                assert_eq!(sparse.get(FIRST + offset), Some((offset as u32 + 1) * 3));
            }
            assert_eq!(sparse.get(FIRST - 1), None);
            assert_eq!(sparse.get(FIRST + last + 1), None);
            assert_eq!(sparse.chunk_decodes, 2);
            assert_eq!(
                sparse
                    .chunks
                    .iter()
                    .filter(|chunk| chunk.decoded.is_some())
                    .count(),
                2
            );
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
                            let mut page = MappingPage::new(1000, bytes);
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
            let mut page = MappingPage::new(0, bytes);
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
            let (start, _, _) = builder.finish(&mut bman).unwrap();

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
