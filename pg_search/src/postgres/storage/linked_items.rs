// Copyright (c) 2023-2024 Retake, Inc.
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

use super::block::{BM25PageSpecialData, LinkedList, LinkedListData, MVCCEntry, PgItem};
use crate::postgres::storage::buffer::{BufferManager, BufferMut};
use crate::postgres::NeedWal;
use anyhow::Result;
use pgrx::pg_sys;
use std::fmt::Debug;

// ---------------------------------------------------------------
// Linked list implementation over block storage,
// where each node in the list is a pg_sys::Item
// ---------------------------------------------------------------

// +-------------------------------------------------------------+
// |                       Header Buffer                         |
// +-------------------------------------------------------------+
// | LinkedListData                                              |
// +-------------------------------------------------------------+
// | LP_SPECIAL                                                  |
// | [next_blockno: BlockNumber, xmax: TransactionId]            |
// +-------------------------------------------------------------+

// +-------------------------------------------------------------+
// |                        Start Buffer                         |
// +-------------------------------------------------------------+
// | [Item] [Item] [Item] ...                                    |
// +-------------------------------------------------------------+
// | LP_SPECIAL                                                  |
// | [next_blockno: BlockNumber, xmax: TransactionId]            |
// +-------------------------------------------------------------+

// ... repeat until the last block

// +-------------------------------------------------------------+
// |                        Last Buffer                          |
// +-------------------------------------------------------------+
// | [Item] [Item] [Item] ...                                    |
// +-------------------------------------------------------------+
// | LP_SPECIAL                                                  |
// | [next_blockno: BlockNumber, xmax: TransactionId]            |
// +-------------------------------------------------------------+

pub struct LinkedItemList<T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> {
    relation_oid: pg_sys::Oid,
    pub header_blockno: pg_sys::BlockNumber,
    bman: BufferManager,
    _marker: std::marker::PhantomData<T>,
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> LinkedList for LinkedItemList<T> {
    fn get_header_blockno(&self) -> pg_sys::BlockNumber {
        self.header_blockno
    }

    unsafe fn get_linked_list_data(&self) -> LinkedListData {
        let header_buffer = self.bman.get_buffer(self.get_header_blockno());
        let page = header_buffer.page();
        page.contents::<LinkedListData>()
    }
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> LinkedItemList<T> {
    pub fn open(
        relation_oid: pg_sys::Oid,
        header_blockno: pg_sys::BlockNumber,
        need_wal: NeedWal,
    ) -> Self {
        Self {
            relation_oid,
            header_blockno,
            bman: BufferManager::new(relation_oid, need_wal),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn bman(&self) -> &BufferManager {
        &self.bman
    }

    pub fn bman_mut(&mut self) -> &mut BufferManager {
        &mut self.bman
    }

    pub unsafe fn create(relation_oid: pg_sys::Oid, need_wal: NeedWal) -> Self {
        let mut bman = BufferManager::new(relation_oid, need_wal);

        let mut header_buffer = bman.new_buffer();
        let header_blockno = header_buffer.number();
        let mut start_buffer = bman.new_buffer();
        let start_blockno = start_buffer.number();

        let mut header_page = header_buffer.init_page();
        start_buffer.init_page();

        let metadata = header_page.contents_mut::<LinkedListData>();
        metadata.skip_list[0] = start_blockno;
        metadata.inner.last_blockno = start_blockno;
        metadata.inner.npages = 0;

        Self {
            relation_oid,
            header_blockno,
            bman,
            _marker: std::marker::PhantomData,
        }
    }

    pub unsafe fn garbage_collect(&mut self, strategy: pg_sys::BufferAccessStrategy) -> Result<()> {
        // Delete all items that are definitely dead
        let snapshot = pg_sys::GetActiveSnapshot();
        let heap_oid = unsafe { pg_sys::IndexGetRelation(self.relation_oid, false) };
        let heap_relation = unsafe { pg_sys::RelationIdGetRelation(heap_oid) };
        let start_blockno = self.get_start_blockno();
        let mut blockno = start_blockno;

        while blockno != pg_sys::InvalidBlockNumber {
            let mut buffer = self.bman.get_buffer_for_cleanup(blockno, strategy);
            let mut page = buffer.page_mut();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();
            let mut delete_offsets = vec![];

            while offsetno <= max_offset {
                if let Some(entry) = page.read_item::<T>(offsetno) {
                    if entry.recyclable(snapshot, heap_relation) {
                        delete_offsets.push(offsetno);
                    }
                }
                offsetno += 1;
            }

            if !delete_offsets.is_empty() {
                page.delete_items(&mut delete_offsets);
            }

            blockno = buffer.next_blockno();
        }

        pg_sys::RelationClose(heap_relation);
        Ok(())
    }

    pub unsafe fn add_items(&mut self, items: Vec<T>, buffer: Option<BufferMut>) -> Result<()> {
        let need_hold = buffer.is_some();
        let mut buffer =
            buffer.unwrap_or_else(|| self.bman.get_buffer_mut(self.get_start_blockno()));

        // this will get set to the `buffer` argument above and remain open (ie, exclusive locked)
        // until we're done adding all the items, so long as the original `buffer` argument was
        // Some(BufferMut)
        let mut hold_open = None;

        for item in items {
            let PgItem(pg_item, size) = item.into();

            'append_loop: loop {
                let mut page = buffer.page_mut();
                let offsetno = page.append_item(pg_item, size, 0);
                if offsetno != pg_sys::InvalidOffsetNumber {
                    // it added to this block
                    break 'append_loop;
                } else if buffer.next_blockno() != pg_sys::InvalidBlockNumber {
                    // go to the next block
                    let next_blockno = buffer.next_blockno();
                    if need_hold && hold_open.is_none() {
                        hold_open = Some(buffer);
                    }
                    buffer = self.bman.get_buffer_mut(next_blockno);
                } else {
                    // need to create new block and link it to this one
                    let mut new_page = self.bman.new_buffer();
                    let new_blockno = new_page.number();
                    new_page.init_page();

                    let mut page = buffer.page_mut();
                    let special = page.special_mut::<BM25PageSpecialData>();
                    special.next_blockno = new_blockno;

                    // Update the header to point to the new last page
                    let mut header_buffer = self.bman.get_buffer_mut(self.header_blockno);
                    let mut page = header_buffer.page_mut();
                    let metadata = page.contents_mut::<LinkedListData>();
                    metadata.inner.last_blockno = new_blockno;
                    metadata.inner.npages += 1;

                    if need_hold && hold_open.is_none() {
                        hold_open = Some(buffer);
                    }
                    buffer = new_page;
                }
            }
        }

        Ok(())
    }

    pub unsafe fn lookup<Cmp: Fn(&T) -> bool>(&self, cmp: Cmp) -> Result<T> {
        self.lookup_ex(cmp).map(|(t, _, _)| t)
    }

    pub unsafe fn lookup_ex<Cmp: Fn(&T) -> bool>(
        &self,
        cmp: Cmp,
    ) -> Result<(T, pg_sys::BlockNumber, pg_sys::OffsetNumber)> {
        let mut blockno = self.get_start_blockno();

        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = self.bman.get_buffer(blockno);
            let page = buffer.page();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();

            while offsetno <= max_offset {
                if let Some(deserialized) = page.read_item::<T>(offsetno) {
                    if cmp(&deserialized) {
                        return Ok((deserialized, blockno, offsetno));
                    }
                }
                offsetno += 1;
            }

            blockno = buffer.next_blockno();
        }

        // if we get here, we didn't find what we were looking for
        // but we should have -- how else could we have asked for something from the list?
        Err(anyhow::anyhow!(format!(
            "transaction {} failed to find the desired entry",
            pg_sys::GetCurrentTransactionId()
        )))
    }
}
