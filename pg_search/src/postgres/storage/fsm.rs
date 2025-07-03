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
#[repr(C, packed)]
struct FSMBlockInner {
    /// What type of block is this?
    kind: FSMBlockKind,

    /// How many physical blocks does it contain?
    len: u32,
}

impl Default for FSMBlockInner {
    fn default() -> Self {
        Self {
            kind: FSMBlockKind::Uncompressed,
            len: 0,
        }
    }
}

const UNCOMPRESSED_MAX_BLOCKS_PER_PAGE: usize =
    (bm25_max_free_space() / size_of::<pg_sys::BlockNumber>()) - size_of::<FSMBlockInner>();

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct FSMBlock {
    meta: FSMBlockInner,

    // with different [`FSMBlockKind`]s this might need to become a `[u8; bm25_max_free_space() - size_of::<FSMBlockInner>()]`
    // and treated differently by each kind
    blocks: [pg_sys::BlockNumber; UNCOMPRESSED_MAX_BLOCKS_PER_PAGE],
}

impl Default for FSMBlock {
    fn default() -> Self {
        Self {
            meta: Default::default(),
            blocks: [pg_sys::InvalidBlockNumber; UNCOMPRESSED_MAX_BLOCKS_PER_PAGE],
        }
    }
}

#[derive(Debug)]
pub struct FreeSpaceManager {
    last_block: pg_sys::BlockNumber,
}

impl FreeSpaceManager {
    pub unsafe fn init(indexrel: &PgSearchRelation) -> pg_sys::BlockNumber {
        let mut new_buffer = init_new_buffer(indexrel);
        let mut page = new_buffer.page_mut();
        *page.contents_mut::<FSMBlock>() = FSMBlock::default();
        new_buffer.number()
    }

    pub fn open(last_block: pg_sys::BlockNumber) -> Self {
        Self { last_block }
    }

    pub fn pop(&self, bman: &mut BufferManager) -> Option<pg_sys::BlockNumber> {
        let mut blockno = self.last_block;

        loop {
            if blockno == pg_sys::InvalidBlockNumber {
                // the FSM is empty
                return None;
            }
            let mut buffer = bman.get_buffer_mut(blockno);
            let mut page = buffer.page_mut();
            let block = page.contents_mut::<FSMBlock>();
            if block.meta.len == 0 {
                // go to the next block
                blockno = page.special::<BM25PageSpecialData>().next_blockno;
                continue;
            }

            // found a block to return
            block.meta.len -= 1;
            return Some(block.blocks[block.meta.len as usize]);
        }
    }

    pub fn pop_many(&self, bman: &mut BufferManager, npages: usize) -> Vec<pg_sys::BlockNumber> {
        if npages == 0 {
            return Vec::new();
        }
        let mut result = Vec::with_capacity(npages);
        let mut remaining = npages;
        let mut blockno = self.last_block;

        while remaining > 0 && blockno != pg_sys::InvalidBlockNumber {
            let mut buffer = bman.get_buffer_mut(blockno);
            let mut page = buffer.page_mut();
            let block = page.contents_mut::<FSMBlock>();

            if block.meta.len > 0 {
                let chunk_size = remaining.min(block.meta.len as usize);
                let new_len = block.meta.len - chunk_size as u32;

                result.extend_from_slice(&block.blocks[new_len as usize..block.meta.len as usize]);
                block.meta.len = new_len;
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

    pub fn extend(
        &self,
        bman: &mut BufferManager,
        blocks: impl Iterator<Item = pg_sys::BlockNumber>,
    ) {
        let mut blocks = blocks.peekable();
        let mut blockno = self.last_block;
        loop {
            let mut buffer = bman.get_buffer_mut(blockno);
            let mut page = buffer.page_mut();
            let block = page.contents_mut::<FSMBlock>();

            while (block.meta.len as usize) < UNCOMPRESSED_MAX_BLOCKS_PER_PAGE {
                match blocks.peek() {
                    // we've added every block from the iterator
                    None => {
                        return;
                    }

                    // add the next block
                    Some(blockno) => {
                        block.blocks[block.meta.len as usize] = *blockno;
                        block.meta.len += 1;
                        blocks.next(); // burn it
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
                let mut new_buffer = init_new_buffer(bman.bm25cache().rel());
                let mut new_page = new_buffer.page_mut();
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
        let free_blocks = block.blocks[..block.meta.len as usize].to_vec();
        mapping.push((blockno, free_blocks));
        blockno = page.special::<BM25PageSpecialData>().next_blockno;
    }

    TableIterator::new(mapping.into_iter().flat_map(|(fsm_blockno, blocks)| {
        blocks
            .into_iter()
            .map(move |blockno| (fsm_blockno.into(), blockno.into()))
    }))
}
