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

use super::block::{
    bm25_max_free_space, BM25PageSpecialData, LinkedList, LinkedListData, FIXED_BLOCK_NUMBERS,
};
use crate::postgres::storage::blocklist;
use crate::postgres::storage::buffer::{BufferManager, PageHeaderMethods};
use anyhow::Result;
use pgrx::pg_sys::BlockNumber;
use pgrx::{check_for_interrupts, pg_sys};
use std::cmp::min;
use std::fmt::Debug;
use std::io::{Cursor, Read, Write};
use std::ops::{Deref, Range};
use std::sync::OnceLock;

// ---------------------------------------------------------------
// Linked list implementation over block storage,
// where each node is a page filled with bm25_max_free_space()
// bytes (with the possible exception of the last page)
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
// | Vec<u8>                                                     |
// +-------------------------------------------------------------+
// | LP_SPECIAL                                                  |
// | [next_blockno: BlockNumber, xmax: TransactionId]            |
// +-------------------------------------------------------------+

// ... repeat until the last block

// +-------------------------------------------------------------+
// |                        Last Buffer                          |
// +-------------------------------------------------------------+
// | Vec<u8>                                                     |
// +-------------------------------------------------------------+
// | LP_SPECIAL                                                  |
// | [next_blockno: BlockNumber, xmax: TransactionId]            |
// +-------------------------------------------------------------+

#[derive(Debug)]
pub struct LinkedBytesList {
    bman: BufferManager,
    pub header_blockno: pg_sys::BlockNumber,
    blocklist_reader: OnceLock<blocklist::reader::BlockList>,
}

pub struct LinkedBytesListWriter {
    list: LinkedBytesList,
    blocklist_builder: blocklist::builder::BlockList,
    last_blockno: pg_sys::BlockNumber,
}

impl LinkedBytesListWriter {
    pub unsafe fn write(&mut self, bytes: &[u8]) -> Result<()> {
        let mut data_cursor = Cursor::new(bytes);
        let mut bytes_written = 0;

        while bytes_written < bytes.len() {
            check_for_interrupts!();
            self.blocklist_builder.push(self.last_blockno);

            let mut buffer = self.list.bman.get_buffer_mut(self.last_blockno);
            let mut page = buffer.page_mut();
            let free_space = page.header().free_space();
            assert!(free_space <= bm25_max_free_space());

            let bytes_to_write = min(free_space, bytes.len() - bytes_written);
            if bytes_to_write == 0 {
                let mut new_buffer = self.list.bman.new_buffer();

                // Set next blockno
                let new_blockno = new_buffer.number();
                let special = page.special_mut::<BM25PageSpecialData>();
                special.next_blockno = new_blockno;

                // Initialize new page
                new_buffer.init_page();

                // Set last blockno to new blockno
                let mut header_buffer = self
                    .list
                    .bman
                    .get_buffer_mut(self.list.get_header_blockno());

                let mut page = header_buffer.page_mut();
                let metadata = page.contents_mut::<LinkedListData>();
                metadata.last_blockno = new_blockno;
                metadata.npages += 1;

                self.last_blockno = new_blockno;
                continue;
            }

            let page_slice = page
                .free_space_slice_mut(bytes_to_write)
                .expect("page is full");
            data_cursor.read_exact(page_slice)?;
            bytes_written += bytes_to_write;

            page.header_mut().pd_lower += bytes_to_write as u16;
        }

        Ok(())
    }

    fn flush_inner(&mut self) -> Result<()> {
        // TODO: Do we need to flush the currently open block in `self.buffer`?

        // TODO: `finish` implies that this method should only be called once: rather than being in
        // `flush`, it should potentially only be in `finish`?
        if let Some(blockno) = self.blocklist_builder.finish(&mut self.list.bman) {
            let mut header_block = self.list.bman.get_buffer_mut(self.list.header_blockno);
            let mut page = header_block.page_mut();
            let metadata = page.contents_mut::<LinkedListData>();
            metadata.blocklist_start = blockno;
        }
        Ok(())
    }

    pub fn into_inner(mut self) -> Result<LinkedBytesList> {
        self.flush_inner()?;
        Ok(self.list)
    }
}

impl Write for LinkedBytesListWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        unsafe {
            self.write(buf)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.flush_inner()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}

impl LinkedList for LinkedBytesList {
    fn get_header_blockno(&self) -> pg_sys::BlockNumber {
        self.header_blockno
    }

    fn bman(&self) -> &BufferManager {
        &self.bman
    }

    fn bman_mut(&mut self) -> &mut BufferManager {
        &mut self.bman
    }

    fn block_for_ord(&self, ord: usize) -> Option<BlockNumber> {
        self.blocklist_reader
            .get_or_init(|| {
                blocklist::reader::BlockList::new(&self.bman, unsafe {
                    self.get_linked_list_data().blocklist_start
                })
            })
            .get(ord)
    }
}

#[derive(Debug)]
pub enum RangeData {
    OnePage(*const u8, usize),
    MultiPage(Vec<u8>),
}

impl Default for RangeData {
    fn default() -> Self {
        RangeData::MultiPage(vec![])
    }
}

unsafe impl Sync for RangeData {}
unsafe impl Send for RangeData {}

impl RangeData {
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            RangeData::OnePage(_, len) => *len,
            RangeData::MultiPage(vec) => vec.len(),
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        match self {
            RangeData::OnePage(ptr, _) => *ptr,
            RangeData::MultiPage(vec) => vec.as_ptr(),
        }
    }
}

impl Deref for RangeData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            RangeData::OnePage(ptr, len) => unsafe { std::slice::from_raw_parts(*ptr, *len) },
            RangeData::MultiPage(vec) => &vec[..],
        }
    }
}

impl LinkedBytesList {
    pub fn open(relation_oid: pg_sys::Oid, header_blockno: pg_sys::BlockNumber) -> Self {
        Self {
            bman: BufferManager::new(relation_oid),
            header_blockno,
            blocklist_reader: Default::default(),
        }
    }

    pub unsafe fn create(relation_oid: pg_sys::Oid) -> Self {
        let mut bman = BufferManager::new(relation_oid);

        let mut header_buffer = bman.new_buffer();
        let header_blockno = header_buffer.number();
        let mut start_buffer = bman.new_buffer();
        let start_blockno = start_buffer.number();

        let mut header_page = header_buffer.init_page();
        start_buffer.init_page();

        let metadata = header_page.contents_mut::<LinkedListData>();
        metadata.start_blockno = start_blockno;
        metadata.last_blockno = start_blockno;
        metadata.npages = 1;
        metadata.blocklist_start = pg_sys::InvalidBlockNumber;

        Self {
            bman,
            header_blockno,
            blocklist_reader: Default::default(),
        }
    }

    pub fn writer(self) -> LinkedBytesListWriter {
        let last_blockno = self.get_last_blockno();
        LinkedBytesListWriter {
            list: self,
            blocklist_builder: Default::default(),
            last_blockno,
        }
    }

    pub unsafe fn read_all(&self) -> Vec<u8> {
        let (mut blockno, mut buffer) = self.get_start_blockno();
        let mut bytes: Vec<u8> = vec![];

        while blockno != pg_sys::InvalidBlockNumber {
            buffer = self.bman.get_buffer_exchange(blockno, buffer);
            let page = buffer.page();
            let special = page.special::<BM25PageSpecialData>();
            let slice = page.as_slice();

            bytes.extend_from_slice(slice);
            blockno = special.next_blockno;
        }

        bytes
    }

    /// Return all the allocated blocks used by this [`LinkedBytesList`] back to the
    /// Free Space Map behind this index.
    ///
    /// It's the caller's responsibility to later call [`pg_sys::IndexFreeSpaceMapVacuum`]
    /// if necessary.
    pub unsafe fn return_to_fsm(mut self) {
        // in addition to the list itself, we also have a secondary list of linked blocks (which
        // contain the blocknumbers of this list) that needs to be marked deleted too
        let metadata = self.get_linked_list_data();
        for starting_blockno in [metadata.start_blockno, metadata.blocklist_start] {
            let mut blockno = starting_blockno;
            while blockno != pg_sys::InvalidBlockNumber {
                debug_assert!(
                    FIXED_BLOCK_NUMBERS.iter().all(|fb| *fb != blockno),
                    "mark_deleted:  blockno {blockno} cannot ever be recycled"
                );
                let mut buffer = self.bman.get_buffer_mut(blockno);
                let page = buffer.page_mut();
                let special = page.special::<BM25PageSpecialData>();

                blockno = special.next_blockno;
                buffer.return_to_fsm(&mut self.bman);
            }
        }

        let header_buffer = self.bman.get_buffer_mut(self.header_blockno);
        header_buffer.return_to_fsm(&mut self.bman);
    }

    pub fn is_empty(&self) -> bool {
        self.bman.page_is_empty(self.get_start_blockno().0)
    }

    #[inline]
    pub unsafe fn get_cached_page_slice(&self, blockno: pg_sys::BlockNumber) -> &[u8] {
        self.bman
            .bm25cache()
            .get_page_slice(blockno, Some(pg_sys::BUFFER_LOCK_SHARE))
    }

    #[inline]
    pub unsafe fn get_cached_range(
        &self,
        blockno: pg_sys::BlockNumber,
        range: Range<usize>,
    ) -> RangeData {
        const ITEM_SIZE: usize = bm25_max_free_space();
        let page = self.get_cached_page_slice(blockno);
        let slice_start = range.start % ITEM_SIZE;
        let slice_len = range.len();
        let header_size = std::mem::offset_of!(pg_sys::PageHeaderData, pd_linp);
        let slice_start = slice_start + header_size;
        let slice_end = slice_start + slice_len;
        let slice = &page[slice_start..slice_end];

        // it's all on one page
        RangeData::OnePage(slice.as_ptr(), slice_len)
    }

    pub unsafe fn get_bytes_range(&self, range: Range<usize>) -> RangeData {
        const ITEM_SIZE: usize = bm25_max_free_space();

        // find the closest block in the linked list to where `range` begins
        let start_block_ord = range.start / ITEM_SIZE;
        let mut blockno = self
            .block_for_ord(start_block_ord)
            .expect("block not found");

        if range.start % ITEM_SIZE + range.len() < ITEM_SIZE {
            // fits on one page -- use our page cache.  many individual pages are read multiple
            // times, and using a cache avoids copying the same data
            self.get_cached_range(blockno, range)
        } else {
            // finally, read in the bytes from the blocks that contain the range -- these are specifically not cached
            let mut data = Vec::with_capacity(range.len());
            let mut remaining = range.len();
            while data.len() != range.len() && blockno != pg_sys::InvalidBlockNumber {
                let buffer = self.bman.get_buffer(blockno);
                let page = buffer.page();
                let special = page.special::<BM25PageSpecialData>();
                let slice_start = if data.is_empty() {
                    range.start % ITEM_SIZE
                } else {
                    0
                };
                let slice_len = (ITEM_SIZE - slice_start).min(remaining);
                let slice = page.as_slice_range(slice_start, slice_len);

                data.extend_from_slice(slice);
                blockno = special.next_blockno;
                remaining -= slice_len;
            }

            RangeData::MultiPage(data)
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::postgres::storage::block::BM25PageSpecialData;
    use crate::postgres::storage::utils::BM25BufferCache;
    use pgrx::prelude::*;

    #[pg_test]
    unsafe fn test_linked_bytes_read_write() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        // Test read/write from newly created linked list
        let bytes: Vec<u8> = (1..=255).cycle().take(100_000).collect();
        let start_blockno = {
            let linked_list = LinkedBytesList::create(relation_oid);
            let mut writer = linked_list.writer();
            writer.write(&bytes).unwrap();
            let linked_list = writer.into_inner().unwrap();
            let read_bytes = linked_list.read_all();
            assert_eq!(bytes, read_bytes);

            linked_list.header_blockno
        };

        // Test read from already created linked list
        let linked_list = LinkedBytesList::open(relation_oid, start_blockno);
        let read_bytes = linked_list.read_all();
        assert_eq!(bytes, read_bytes);
    }

    #[pg_test]
    unsafe fn test_linked_bytes_is_empty() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let linked_list = LinkedBytesList::create(relation_oid);
        assert!(linked_list.is_empty());

        let bytes: Vec<u8> = (1..=255).cycle().take(100_000).collect();
        let mut writer = linked_list.writer();
        writer.write(&bytes).unwrap();
        let linked_list = writer.into_inner().unwrap();
        assert!(!linked_list.is_empty());
    }

    #[pg_test]
    unsafe fn test_linked_bytes_mark_deleted() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let linked_list = LinkedBytesList::create(relation_oid);
        let bytes: Vec<u8> = (1..=255).cycle().take(100_000).collect();
        let mut writer = linked_list.writer();
        writer.write(&bytes).unwrap();
        let linked_list = writer.into_inner().unwrap();
        let (mut blockno, _) = linked_list.get_start_blockno();
        linked_list.return_to_fsm();

        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = BM25BufferCache::open(relation_oid)
                .get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
            let page = pg_sys::BufferGetPage(buffer);
            let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
            assert!((*special).xmax != pg_sys::InvalidTransactionId);
            blockno = (*special).next_blockno;
            pg_sys::UnlockReleaseBuffer(buffer);
        }
    }
}
