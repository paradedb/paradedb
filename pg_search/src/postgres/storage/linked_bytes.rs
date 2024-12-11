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

use super::block::{bm25_max_free_space, BM25PageSpecialData, LinkedList, LinkedListData};
use super::utils::{BM25BufferCache, BM25Page};
use crate::postgres::storage::SKIPLIST_FREQ;
use anyhow::Result;
use parking_lot::Mutex;
use pgrx::pg_sys;
use rustc_hash::FxHashMap;
use std::cmp::min;
use std::collections::hash_map::Entry;
use std::io::{Cursor, Read};
use std::ops::{Deref, Range};
use std::slice::{from_raw_parts, from_raw_parts_mut};
use std::sync::Arc;

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

#[derive(Clone, Debug)]
pub struct LinkedBytesList {
    cache: Arc<BM25BufferCache>,
    relation_oid: pg_sys::Oid,
    pub header_blockno: pg_sys::BlockNumber,
    skipcache: Arc<Mutex<FxHashMap<usize, pg_sys::BlockNumber>>>,
}

impl LinkedList for LinkedBytesList {
    fn get_header_blockno(&self) -> pg_sys::BlockNumber {
        self.header_blockno
    }

    fn get_relation_oid(&self) -> pg_sys::Oid {
        self.relation_oid
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
            cache: unsafe { Arc::new(BM25BufferCache::open(relation_oid)) },
            relation_oid,
            header_blockno,
            skipcache: Default::default(),
        }
    }

    pub unsafe fn create(relation_oid: pg_sys::Oid) -> Self {
        let cache = Arc::new(BM25BufferCache::open(relation_oid));
        let state = cache.start_xlog();

        let header_buffer = cache.new_buffer();
        let header_blockno = pg_sys::BufferGetBlockNumber(header_buffer);
        let start_buffer = cache.new_buffer();
        let start_blockno = pg_sys::BufferGetBlockNumber(start_buffer);

        let header_page = pg_sys::GenericXLogRegisterBuffer(
            state,
            header_buffer,
            pg_sys::GENERIC_XLOG_FULL_IMAGE as i32,
        );
        header_page.init(pg_sys::BufferGetPageSize(header_buffer));

        let start_page = pg_sys::GenericXLogRegisterBuffer(
            state,
            start_buffer,
            pg_sys::GENERIC_XLOG_FULL_IMAGE as i32,
        );
        start_page.init(pg_sys::BufferGetPageSize(start_buffer));

        let metadata = pg_sys::PageGetContents(header_page) as *mut LinkedListData;
        (*metadata).skip_list[0] = start_blockno;
        (*metadata).inner.last_blockno = start_blockno;
        (*metadata).inner.npages = 1;

        // Set pd_lower to the end of the metadata
        // Without doing so, metadata will be lost if xlog.c compresses the page
        let page_header = header_page as *mut pg_sys::PageHeaderData;
        (*page_header).pd_lower = (metadata.add(1) as usize - header_page as usize) as u16;

        pg_sys::GenericXLogFinish(state);
        pg_sys::UnlockReleaseBuffer(header_buffer);
        pg_sys::UnlockReleaseBuffer(start_buffer);

        Self {
            cache,
            relation_oid,
            header_blockno,
            skipcache: Default::default(),
        }
    }

    pub unsafe fn write(&mut self, bytes: &[u8]) -> Result<()> {
        let cache = &self.cache;
        let mut data_cursor = Cursor::new(bytes);
        let mut bytes_written = 0;

        while bytes_written < bytes.len() {
            unsafe {
                let insert_blockno = self.get_last_blockno();
                let buffer = cache.get_buffer(insert_blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
                let state = cache.start_xlog();
                let page = pg_sys::GenericXLogRegisterBuffer(state, buffer, 0);
                let header = page as *mut pg_sys::PageHeaderData;
                let free_space = ((*header).pd_upper - (*header).pd_lower) as usize;
                assert!(free_space <= bm25_max_free_space());

                let bytes_to_write = min(free_space, bytes.len() - bytes_written);
                if bytes_to_write == 0 {
                    let new_buffer = cache.new_buffer();
                    // Set next blockno
                    let new_blockno = pg_sys::BufferGetBlockNumber(new_buffer);
                    let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
                    (*special).next_blockno = new_blockno;
                    // Initialize new page
                    let new_page = pg_sys::GenericXLogRegisterBuffer(
                        state,
                        new_buffer,
                        pg_sys::GENERIC_XLOG_FULL_IMAGE as i32,
                    );
                    new_page.init(pg_sys::BufferGetPageSize(new_buffer));
                    // Set last blockno to new blockno
                    let header_buffer = cache.get_buffer(
                        self.get_header_blockno(),
                        Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
                    );

                    let page = pg_sys::GenericXLogRegisterBuffer(state, header_buffer, 0);
                    let metadata = pg_sys::PageGetContents(page) as *mut LinkedListData;
                    (*metadata).inner.last_blockno = new_blockno;
                    (*metadata).inner.npages += 1;
                    if (*metadata).inner.npages as usize % SKIPLIST_FREQ == 0 {
                        let idx = (*metadata).inner.npages as usize / SKIPLIST_FREQ;
                        (*metadata).skip_list[idx] = new_blockno;
                    }

                    pg_sys::GenericXLogFinish(state);
                    pg_sys::UnlockReleaseBuffer(buffer);
                    pg_sys::UnlockReleaseBuffer(new_buffer);
                    pg_sys::UnlockReleaseBuffer(header_buffer);
                    continue;
                }

                let page_slice = from_raw_parts_mut(
                    (page as *mut u8).add((*header).pd_lower as usize),
                    bytes_to_write,
                );
                data_cursor.read_exact(page_slice)?;
                bytes_written += bytes_to_write;
                (*header).pd_lower += bytes_to_write as u16;
                pg_sys::GenericXLogFinish(state);
                pg_sys::UnlockReleaseBuffer(buffer);
            }
        }

        Ok(())
    }

    pub unsafe fn read_all(&self) -> Vec<u8> {
        let cache = BM25BufferCache::open(self.relation_oid);
        let mut blockno = self.get_start_blockno();
        let mut bytes: Vec<u8> = vec![];

        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
            let page = pg_sys::BufferGetPage(buffer);
            let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
            let header = page as *mut pg_sys::PageHeaderData;
            let header_size = std::mem::offset_of!(pg_sys::PageHeaderData, pd_linp) as u16;
            let slice_len = (*header).pd_lower - header_size;
            let slice = from_raw_parts((page as *mut u8).add(header_size.into()), slice_len.into());

            bytes.extend_from_slice(slice);
            blockno = (*special).next_blockno;
            pg_sys::UnlockReleaseBuffer(buffer);
        }

        bytes
    }

    pub unsafe fn mark_deleted(&self) {
        let cache = BM25BufferCache::open(self.relation_oid);
        let mut blockno = self.get_start_blockno();
        while blockno != pg_sys::InvalidBlockNumber {
            let state = cache.start_xlog();
            let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
            let page = pg_sys::GenericXLogRegisterBuffer(state, buffer, 0);
            let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
            blockno = (*special).next_blockno;
            page.mark_deleted();

            pg_sys::GenericXLogFinish(state);
            pg_sys::UnlockReleaseBuffer(buffer);
        }

        let state = cache.start_xlog();
        let header_buffer =
            cache.get_buffer(self.header_blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
        let lock_page = pg_sys::GenericXLogRegisterBuffer(state, header_buffer, 0);
        lock_page.mark_deleted();

        pg_sys::GenericXLogFinish(state);
        pg_sys::UnlockReleaseBuffer(header_buffer);
    }

    pub fn is_empty(&self) -> bool {
        unsafe {
            let cache = BM25BufferCache::open(self.relation_oid);
            let start_blockno = self.get_start_blockno();
            let start_buffer = cache.get_buffer(start_blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
            let start_page = pg_sys::BufferGetPage(start_buffer);
            let is_empty = pg_sys::PageIsEmpty(start_page);
            pg_sys::UnlockReleaseBuffer(start_buffer);
            is_empty
        }
    }

    #[inline]
    pub unsafe fn get_cached_page_slice(&self, blockno: pg_sys::BlockNumber) -> &[u8] {
        self.cache
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
        let cache = &self.cache;
        let start_block_ord = range.start / ITEM_SIZE;
        let (nearest_block, nearest_ord) = self.nearest_block_by_ord(start_block_ord);
        let mut blockno = nearest_block;

        match self.skipcache.lock().entry(start_block_ord) {
            Entry::Occupied(entry) => {
                blockno = *entry.get();
            }
            Entry::Vacant(entry) => {
                // scan forward from that block skipping those that appear before our range begins
                let mut skip = start_block_ord - nearest_ord.saturating_sub(1);
                while skip != 0 && blockno != pg_sys::InvalidBlockNumber {
                    let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
                    let page = pg_sys::BufferGetPage(buffer);
                    let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
                    blockno = (*special).next_blockno;
                    pg_sys::UnlockReleaseBuffer(buffer);
                    skip -= 1;
                }

                entry.insert(blockno);
            }
        }

        if range.start % ITEM_SIZE + range.len() < ITEM_SIZE {
            // fits on one page -- use our page cache.  many individual pages are read multiple
            // times, and using a cache avoids copying the same data
            self.get_cached_range(blockno, range)
        } else {
            // finally, read in the bytes from the blocks that contain the range -- these are specifically not cached
            let mut data = Vec::with_capacity(range.len());
            let mut remaining = range.len();
            while data.len() != range.len() && blockno != pg_sys::InvalidBlockNumber {
                let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
                let page = pg_sys::BufferGetPage(buffer);
                let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
                let slice_start = if data.is_empty() {
                    range.start % ITEM_SIZE
                } else {
                    0
                };
                let slice_len = (ITEM_SIZE - slice_start).min(remaining);
                let header_size = std::mem::offset_of!(pg_sys::PageHeaderData, pd_linp);
                let slice =
                    from_raw_parts((page as *mut u8).add(slice_start + header_size), slice_len);
                data.extend_from_slice(slice);

                blockno = (*special).next_blockno;
                pg_sys::UnlockReleaseBuffer(buffer);

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
            let mut linked_list = LinkedBytesList::create(relation_oid);
            linked_list.write(&bytes).unwrap();
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

        let mut linked_list = LinkedBytesList::create(relation_oid);
        assert!(linked_list.is_empty());

        let bytes: Vec<u8> = (1..=255).cycle().take(100_000).collect();
        linked_list.write(&bytes).unwrap();
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

        let mut linked_list = LinkedBytesList::create(relation_oid);
        let bytes: Vec<u8> = (1..=255).cycle().take(100_000).collect();
        linked_list.write(&bytes).unwrap();
        linked_list.mark_deleted();

        let mut blockno = linked_list.get_start_blockno();
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
