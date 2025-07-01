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
use crate::postgres::storage::block::bm25_max_free_space;
use crate::postgres::storage::buffer::{init_new_buffer, BufferManager};
use pgrx::{check_for_interrupts, pg_sys};

#[derive(Debug)]
#[repr(C, packed)]
struct FSMBlockInner {
    len: u16,
    prev_block: pg_sys::BlockNumber,
}

impl Default for FSMBlockInner {
    fn default() -> Self {
        Self {
            len: 0,
            prev_block: pg_sys::InvalidBlockNumber,
        }
    }
}

const FSM_BLOCK_SIZE: usize =
    (bm25_max_free_space() / size_of::<pg_sys::BlockNumber>()) - size_of::<FSMBlockInner>();
#[repr(C)]
#[derive(Debug)]
struct FSMBlock {
    meta: FSMBlockInner,
    blocks: [pg_sys::BlockNumber; FSM_BLOCK_SIZE],
}

impl Default for FSMBlock {
    fn default() -> Self {
        Self {
            meta: Default::default(),
            blocks: [pg_sys::InvalidBlockNumber; FSM_BLOCK_SIZE],
        }
    }
}

#[derive(Debug)]
pub struct FreeSpaceManager {
    last_block: pg_sys::BlockNumber,
}

impl FreeSpaceManager {
    pub unsafe fn init(indexrel: pg_sys::Relation) -> pg_sys::BlockNumber {
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
                // back up to previous block and start over
                blockno = block.meta.prev_block;
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
                let new_len = block.meta.len - chunk_size as u16;

                result.extend_from_slice(&block.blocks[new_len as usize..block.meta.len as usize]);
                block.meta.len = new_len;
                remaining -= chunk_size;
            }

            // this block is now empty
            // back up to the previous page
            blockno = block.meta.prev_block;
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

            while (block.meta.len as usize) < FSM_BLOCK_SIZE {
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
            // back up to the previous block and apply them there
            blockno = block.meta.prev_block;

            // however, if there is no previous block we need to make one and link it in
            if blockno == pg_sys::InvalidBlockNumber {
                let mut new_buffer = unsafe { init_new_buffer(bman.bm25cache().rel().as_ptr()) };
                let mut page = new_buffer.page_mut();
                *page.contents_mut::<FSMBlock>() = FSMBlock::default();
                block.meta.prev_block = new_buffer.number();
                blockno = block.meta.prev_block;
            }
        }
    }
}
