#[derive(Debug)]
#[repr(u8)]
enum ChunkSyleTag {
    Sorted1x = 0,
    Sorted4x = 1,
    Sorted8x = 2,
    StrictlySorted1x = 3,
    StrictlySorted4x = 4,
    StrictlySorted8x = 5,
    Uncompressed = 6,
}

impl From<u8> for ChunkSyleTag {
    fn from(value: u8) -> Self {
        match value {
            0 => ChunkSyleTag::Sorted1x,
            1 => ChunkSyleTag::Sorted4x,
            2 => ChunkSyleTag::Sorted8x,
            3 => ChunkSyleTag::StrictlySorted1x,
            4 => ChunkSyleTag::StrictlySorted4x,
            5 => ChunkSyleTag::StrictlySorted8x,
            6 => ChunkSyleTag::Uncompressed,
            other => panic!("invalid chunk style tag: {}", other),
        }
    }
}

impl From<ChunkSyleTag> for u8 {
    fn from(value: ChunkSyleTag) -> Self {
        value as u8
    }
}

pub mod builder {
    use crate::postgres::storage::block::BM25PageSpecialData;
    use crate::postgres::storage::blocklist::ChunkSyleTag;
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
        pub fn tag(&self) -> ChunkSyleTag {
            match self {
                ChunkStyle::Sorted1x { .. } => ChunkSyleTag::Sorted1x,
                ChunkStyle::Sorted4x { .. } => ChunkSyleTag::Sorted4x,
                ChunkStyle::Sorted8x { .. } => ChunkSyleTag::Sorted8x,
                ChunkStyle::StrictlySorted1x { .. } => ChunkSyleTag::StrictlySorted1x,
                ChunkStyle::StrictlySorted4x { .. } => ChunkSyleTag::StrictlySorted4x,
                ChunkStyle::StrictlySorted8x { .. } => ChunkSyleTag::StrictlySorted8x,
                ChunkStyle::Uncompressed(_) => ChunkSyleTag::Uncompressed,
            }
        }

        #[allow(dead_code)]
        pub fn len(&self) -> usize {
            match self {
                ChunkStyle::Sorted1x { .. } => BitPacker1x::BLOCK_LEN,
                ChunkStyle::Sorted4x { .. } => BitPacker4x::BLOCK_LEN,
                ChunkStyle::Sorted8x { .. } => BitPacker8x::BLOCK_LEN,
                ChunkStyle::StrictlySorted1x { .. } => BitPacker1x::BLOCK_LEN,
                ChunkStyle::StrictlySorted4x { .. } => BitPacker4x::BLOCK_LEN,
                ChunkStyle::StrictlySorted8x { .. } => BitPacker8x::BLOCK_LEN,
                ChunkStyle::Uncompressed(values) => values.len(),
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
                    .chain(values.into_iter().map(|bn| bn.to_le_bytes()).flatten())
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
        #[allow(dead_code)]
        pub fn is_empty(&self) -> bool {
            self.chunks.is_empty() && self.queue.is_empty()
        }

        pub fn push(&mut self, block_number: pg_sys::BlockNumber) {
            assert!(block_number != 0, "cannot add block 0 to the blocklist");

            if let Some(last) = self.queue.last() {
                if last == &block_number {
                    // we just added this block
                    return;
                }
            }

            if self.queue.len() == BitPacker8x::BLOCK_LEN {
                self.chunks
                    .push(self.pack_8x(&self.queue, self.last_chunked_blockno));

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
                    self.chunks.push(self.pack_8x(&head, last));

                    last = head.last().cloned();
                    queue = tail;
                } else if queue.len() >= BitPacker4x::BLOCK_LEN {
                    let (head, tail) = queue.split_at(BitPacker4x::BLOCK_LEN);
                    self.chunks.push(self.pack_4x(&head, last));

                    last = head.last().cloned();
                    queue = tail;
                } else if queue.len() >= BitPacker1x::BLOCK_LEN {
                    let (head, tail) = queue.split_at(BitPacker1x::BLOCK_LEN);
                    self.chunks.push(self.pack_1x(&head, last));

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
                packer.compress_strictly_sorted(initial, &slice, &mut bytes, num_bits);
                ChunkStyle::StrictlySorted8x {
                    num_bits,
                    initial: initial.unwrap_or(0),
                    bytes,
                }
            } else {
                let num_bits = packer.num_bits_sorted(initial.unwrap_or(0), slice);
                let mut bytes = vec![0u8; num_bits as usize * BitPacker8x::BLOCK_LEN / 8];
                packer.compress_sorted(initial.unwrap_or(0), &slice, &mut bytes, num_bits);
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
                packer.compress_strictly_sorted(initial, &slice, &mut bytes, num_bits);
                ChunkStyle::StrictlySorted4x {
                    num_bits,
                    initial: initial.unwrap_or(0),
                    bytes,
                }
            } else {
                let num_bits = packer.num_bits_sorted(initial.unwrap_or(0), slice);
                let mut bytes = vec![0u8; num_bits as usize * BitPacker4x::BLOCK_LEN / 8];
                packer.compress_sorted(initial.unwrap_or(0), &slice, &mut bytes, num_bits);
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
                packer.compress_strictly_sorted(initial, &slice, &mut bytes, num_bits);
                ChunkStyle::StrictlySorted1x {
                    num_bits,
                    initial: initial.unwrap_or(0),
                    bytes,
                }
            } else {
                let num_bits = packer.num_bits_sorted(initial.unwrap_or(0), slice);
                let mut bytes = vec![0u8; num_bits as usize * BitPacker1x::BLOCK_LEN / 8];
                packer.compress_sorted(initial.unwrap_or(0), &slice, &mut bytes, num_bits);
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
    use crate::postgres::storage::block::BM25PageSpecialData;
    use crate::postgres::storage::blocklist::ChunkSyleTag;
    use crate::postgres::storage::buffer::BufferManager;
    use bitpacking::{BitPacker, BitPacker1x, BitPacker4x, BitPacker8x};
    use pgrx::pg_sys;

    #[derive(Default, Debug)]
    pub struct BlockList {
        blocks: Vec<pg_sys::BlockNumber>,
    }

    impl BlockList {
        pub fn new(bman: &BufferManager, starting_block: pg_sys::BlockNumber) -> Self {
            if starting_block == pg_sys::InvalidBlockNumber {
                return Self::default();
            }

            let mut blocks = Vec::new();
            let mut blockno = starting_block;
            loop {
                let block = bman.get_buffer(blockno);
                let page = block.page();

                let mut offset = 0;
                let slice = page.as_slice();

                loop {
                    let tag = ChunkSyleTag::from(slice[offset]);
                    offset += 1;

                    match tag {
                        tag @ ChunkSyleTag::Sorted1x
                        | tag @ ChunkSyleTag::Sorted4x
                        | tag @ ChunkSyleTag::Sorted8x
                        | tag @ ChunkSyleTag::StrictlySorted1x
                        | tag @ ChunkSyleTag::StrictlySorted4x
                        | tag @ ChunkSyleTag::StrictlySorted8x => {
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
                                ChunkSyleTag::Sorted1x => {
                                    blocks.extend_from_slice(&[0; BitPacker1x::BLOCK_LEN]);
                                    offset += BitPacker1x::new().decompress_sorted(
                                        initial,
                                        &slice[offset..],
                                        &mut blocks[end..],
                                        num_bits,
                                    );
                                }
                                ChunkSyleTag::Sorted4x => {
                                    blocks.extend_from_slice(&[0; BitPacker4x::BLOCK_LEN]);
                                    offset += BitPacker4x::new().decompress_sorted(
                                        initial,
                                        &slice[offset..],
                                        &mut blocks[end..],
                                        num_bits,
                                    );
                                }
                                ChunkSyleTag::Sorted8x => {
                                    blocks.extend_from_slice(&[0; BitPacker8x::BLOCK_LEN]);
                                    offset += BitPacker8x::new().decompress_sorted(
                                        initial,
                                        &slice[offset..],
                                        &mut blocks[end..],
                                        num_bits,
                                    );
                                }
                                ChunkSyleTag::StrictlySorted1x => {
                                    blocks.extend_from_slice(&[0; BitPacker1x::BLOCK_LEN]);
                                    offset += BitPacker1x::new().decompress_strictly_sorted(
                                        (initial != 0).then_some(initial),
                                        &slice[offset..],
                                        &mut blocks[end..],
                                        num_bits,
                                    );
                                }
                                ChunkSyleTag::StrictlySorted4x => {
                                    blocks.extend_from_slice(&[0; BitPacker4x::BLOCK_LEN]);
                                    offset += BitPacker4x::new().decompress_strictly_sorted(
                                        (initial != 0).then_some(initial),
                                        &slice[offset..],
                                        &mut blocks[end..],
                                        num_bits,
                                    );
                                }
                                ChunkSyleTag::StrictlySorted8x => {
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
                        ChunkSyleTag::Uncompressed => {
                            let len = slice[offset] as usize;
                            offset += 1;
                            let mut tmp = [0u8; size_of::<pg_sys::BlockNumber>()];
                            for _ in 0..len {
                                tmp.copy_from_slice(
                                    &slice[offset..offset + size_of::<pg_sys::BlockNumber>()],
                                );
                                offset += size_of::<pg_sys::BlockNumber>();
                                let value = u32::from_le_bytes(tmp);
                                blocks.push(value);
                            }
                        }
                    }

                    if offset >= slice.len() {
                        break;
                    }
                }

                blockno = page.special::<BM25PageSpecialData>().next_blockno;
                if blockno == pg_sys::InvalidBlockNumber {
                    break;
                }
            }

            Self { blocks }
        }

        #[allow(dead_code)]
        pub fn len(&self) -> usize {
            self.blocks.len()
        }

        pub fn get(&self, i: usize) -> Option<pg_sys::BlockNumber> {
            self.blocks.get(i).cloned()
        }
    }
}
