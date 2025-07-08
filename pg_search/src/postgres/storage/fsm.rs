// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{bm25_max_free_space, BM25PageSpecialData};
use crate::postgres::storage::buffer::{init_new_buffer, BufferManager};
use crate::postgres::storage::metadata::MetaPage;
use pgrx::iter::TableIterator;
use pgrx::{name, pg_extern, pg_sys, AnyNumeric, PgRelation};

// NB:  As of the initial implementation, we only have "Uncompressed" but I (@eeeebbbbrrrr) could
//      imagine bit-packed blocks, variable-width blocks, etc
/// Denotes what the data on an FSM block looks like
#[derive(Debug, Copy, Clone)]
#[repr(u32)]
enum FSMBlockKind {
    Uncompressed = 0,
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct FSMBlockHeader {
    /// Denotes how the block data is stored on this page
    kind: FSMBlockKind,

    /// Specifies the number of *free* blocks managed by this page
    len: u32,
}

impl Default for FSMBlockHeader {
    fn default() -> Self {
        Self {
            kind: FSMBlockKind::Uncompressed,
            len: 0,
        }
    }
}

const UNCOMPRESSED_MAX_BLOCKS_PER_PAGE: usize =
    (bm25_max_free_space() / size_of::<pg_sys::BlockNumber>()) - size_of::<FSMBlockHeader>();

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct FSMBlock {
    header: FSMBlockHeader,

    // with different [`FSMBlockKind`]s this might need to become a `[u8; bm25_max_free_space() - size_of::<FSMBlockInner>()]`
    // and treated differently by each kind
    blocks: [pg_sys::BlockNumber; UNCOMPRESSED_MAX_BLOCKS_PER_PAGE],
}

impl Default for FSMBlock {
    fn default() -> Self {
        Self {
            header: Default::default(),
            blocks: [pg_sys::InvalidBlockNumber; UNCOMPRESSED_MAX_BLOCKS_PER_PAGE],
        }
    }
}

/// The [`FreeSpaceManager`] our version of Postgres' "free space map".  We need to track free space
/// as whole blocks and we'd prefer to not have to mark pages we return to a FSM as deleted.  Our
/// own implementation allows us to do that.
///
/// The design of this structure is simply a linked list of blocks where each block, a [`FSMBlock`],
/// is (currently) a fixed-sized array of [`pg_sys::BlockNumber`]s.  Each block contains a small
/// [`FSMBlockHeader`] that stores the list of free blocks it contains along with bookkeeping data
/// and a [`FSMBlockKind`] flag indicating how the blocks are stored on that page.
///
/// Outside of per-page exclusive locking when mutating a page, no special locking requirements exist
/// to manage concurrency.  The intent is that the [`FreeSpaceManager`]'s linked block list can grow
/// unbounded, with the hope that it actually won't grow to be very large in practice.
///
/// Any other kind of structure will likely need a more sophisticated approach to concurrency control.
///
/// The user-facing API is meant to _kinda_ mimic a `Vec` in that the [`FreeSpaceManager`] can be
/// popped and extended.  From where in the FSM popped blocks come, or where in the FSM new blocks
/// are added, is an implementation detail.
#[derive(Debug)]
pub struct FreeSpaceManager {
    start_blockno: pg_sys::BlockNumber,
}

impl FreeSpaceManager {
    /// Create a new [`FreeSpaceManager`] in the block storage of the specified `indexrel`.
    pub unsafe fn create(indexrel: &PgSearchRelation) -> pg_sys::BlockNumber {
        let mut new_buffer = init_new_buffer(indexrel);
        let mut page = new_buffer.page_mut();
        *page.contents_mut::<FSMBlock>() = FSMBlock::default();
        new_buffer.number()
    }

    /// Open an existing [`FreeSpaceManager`] which is rooted at the specified starting block number.
    pub fn open(start_blockno: pg_sys::BlockNumber) -> Self {
        Self { start_blockno }
    }

    /// Retrieve a single free [`pg_sys::BlockNumber`], which can be acquired and re-initialized.
    ///
    /// If no free blocks are available, this returns `None`.
    ///
    /// Upon return, the block is removed from the [`FreeSpaceManager`]'s control.  It is the caller's
    /// responsibility to ensure the block is soon properly used or else it will be lost forever as
    /// dead space in the underlying relation.
    ///
    /// # Implementation Note
    ///
    /// The FSM is traversed from head to tail.  The requested block is "popped" from the first page
    /// that has a free block.  This is done by decrementing its header `len` property by one.
    pub fn pop(&self, bman: &mut BufferManager) -> Option<pg_sys::BlockNumber> {
        let mut blockno = self.start_blockno;

        loop {
            if blockno == pg_sys::InvalidBlockNumber {
                // the FSM is empty
                return None;
            }
            let mut buffer = bman.get_buffer_mut(blockno);
            let mut page = buffer.page_mut();
            let block = page.contents::<FSMBlock>();
            if block.header.len == 0 {
                // go to the next block
                blockno = page.special::<BM25PageSpecialData>().next_blockno;
                continue;
            }

            // found a block to return
            let block = page.contents_mut::<FSMBlock>();
            block.header.len -= 1;
            return Some(block.blocks[block.header.len as usize]);
        }
    }

    /// The returned Vec will contain as many free blocks as possible, up to `npages`.
    ///
    /// Upon return, these blocks are now removed from the [`FreeSpaceManager`]'s control.  If the caller
    /// loses these blocks then they're lost forever as dead space in the underlying relation.
    ///
    /// # Implementation Note
    ///
    /// The FSM is traversed from head to tail.  During traversal, if the current page has at least
    /// `npages` free blocks, they're consumed (and queued for return) by decrementing the page's
    /// `len` property by the number of blocks consumed from that page.
    pub fn pop_many(&self, bman: &mut BufferManager, npages: usize) -> Vec<pg_sys::BlockNumber> {
        if npages == 0 {
            return Vec::new();
        }
        let mut result = Vec::with_capacity(npages);
        let mut remaining = npages;
        let mut blockno = self.start_blockno;

        while remaining > 0 && blockno != pg_sys::InvalidBlockNumber {
            let mut buffer = bman.get_buffer_mut(blockno);
            let mut page = buffer.page_mut();
            let block = page.contents::<FSMBlock>();

            if block.header.len > 0 {
                let chunk_size = remaining.min(block.header.len as usize);
                let new_len = block.header.len - chunk_size as u32;

                result
                    .extend_from_slice(&block.blocks[new_len as usize..block.header.len as usize]);
                page.contents_mut::<FSMBlock>().header.len = new_len;
                remaining -= chunk_size;
            }

            // this block is now empty
            // go to the next page
            blockno = page.special::<BM25PageSpecialData>().next_blockno
        }

        // TODO:  it might make sense to sort this
        // result.sort_unstable();

        result
    }

    /// Add the specified `extend_with` iterator of [`pg_sys::BlockNumber`]s to this [`FreeSpaceManager`].
    ///
    /// # Implementation Note
    ///
    /// The FSM is traversed head to tail and empty space on each page is filled with the results
    /// of the provided `extend_with` iterator.  If the whole FSM is full then a new block is allocated
    /// and linked to the end as the new tail.
    pub fn extend(
        &self,
        bman: &mut BufferManager,
        extend_with: impl Iterator<Item = pg_sys::BlockNumber>,
    ) {
        let mut extend_with = extend_with.peekable();
        let mut blockno = self.start_blockno;
        loop {
            let mut buffer = bman.get_buffer_mut(blockno);
            let mut page = buffer.page_mut();

            let mut len = page.contents::<FSMBlock>().header.len as usize;
            while len < UNCOMPRESSED_MAX_BLOCKS_PER_PAGE {
                match extend_with.peek() {
                    // we've added every block from the iterator
                    None => {
                        return;
                    }

                    // add the next block
                    Some(blockno) => {
                        let block = page.contents_mut::<FSMBlock>();
                        block.blocks[block.header.len as usize] = *blockno;
                        block.header.len += 1;
                        len = block.header.len as usize;
                        extend_with.next(); // burn it
                    }
                }
            }

            // TODO:  it might make sense to sort `block.blocks` in reverse order
            //        so that when blocks are popped off they're returned smallest-to-largest

            // we still have blocks to apply
            // move to the next block and apply them there
            blockno = page.special::<BM25PageSpecialData>().next_blockno;

            // however, if there is no next block we need to make one and link it in
            if blockno == pg_sys::InvalidBlockNumber {
                let mut new_buffer = init_new_buffer(bman.buffer_access().rel());
                let mut new_page = new_buffer.page_mut();

                // initialize the new page with a default FSMBlock
                *new_page.contents_mut::<FSMBlock>() = FSMBlock::default();

                // move to this new block
                let new_blockno = new_buffer.number();
                page.special_mut::<BM25PageSpecialData>().next_blockno = new_blockno;
                blockno = new_blockno;
            }
        }
    }
}

#[pg_extern]
unsafe fn fsm_info(
    index: PgRelation,
) -> TableIterator<
    'static,
    (
        name!(fsm_blockno, AnyNumeric),
        name!(free_blockno, AnyNumeric),
    ),
> {
    let index = PgSearchRelation::from_pg(index.as_ptr());

    let meta = MetaPage::open(&index);
    let fsm_start = meta.fsm();
    let bman = BufferManager::new(&index);
    let mut mapping = Vec::<(pg_sys::BlockNumber, Vec<pg_sys::BlockNumber>)>::default();

    let mut blockno = fsm_start;

    while blockno != pg_sys::InvalidBlockNumber {
        let buffer = bman.get_buffer(blockno);
        let page = buffer.page();
        let block = page.contents::<FSMBlock>();
        let free_blocks = block.blocks[..block.header.len as usize].to_vec();
        mapping.push((blockno, free_blocks));
        blockno = page.special::<BM25PageSpecialData>().next_blockno;
    }

    TableIterator::new(mapping.into_iter().flat_map(|(fsm_blockno, blocks)| {
        blocks
            .into_iter()
            .map(move |blockno| (fsm_blockno.into(), blockno.into()))
    }))
}
