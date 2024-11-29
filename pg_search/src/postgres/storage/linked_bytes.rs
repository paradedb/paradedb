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
use anyhow::Result;
use pgrx::pg_sys;
use std::cmp::min;
use std::io::{Cursor, Read};
use std::slice::{from_raw_parts, from_raw_parts_mut};

// ---------------------------------------------------------------
// Linked list implementation over block storage,
// where each node is a page filled with bm25_max_free_space()
// bytes (with the possible exception of the last page)
// ---------------------------------------------------------------

// +-------------------------------------------------------------+
// |                        Lock Buffer                          |
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

pub struct LinkedBytesList {
    relation_oid: pg_sys::Oid,
    pub lock_buffer: pg_sys::Buffer,
    lock: Option<u32>,
}

impl LinkedList for LinkedBytesList {
    fn get_lock_buffer(&self) -> pg_sys::Buffer {
        self.lock_buffer
    }

    fn get_lock(&self) -> Option<u32> {
        self.lock
    }
}

impl LinkedBytesList {
    pub fn open_with_lock(
        relation_oid: pg_sys::Oid,
        lock_blockno: pg_sys::BlockNumber,
        lock: Option<u32>,
    ) -> Self {
        let cache = unsafe { BM25BufferCache::open(relation_oid) };
        let lock_buffer = unsafe { cache.get_buffer(lock_blockno, lock) };
        Self {
            relation_oid,
            lock_buffer,
            lock,
        }
    }

    /// Create a new linked list and holds an exclusive lock on it until the linked list is dropped
    pub unsafe fn create(relation_oid: pg_sys::Oid) -> Self {
        let cache = BM25BufferCache::open(relation_oid);
        let lock_buffer = cache.new_buffer();
        let start_buffer = cache.new_buffer();
        let start_blockno = pg_sys::BufferGetBlockNumber(start_buffer);

        let lock_page = pg_sys::BufferGetPage(lock_buffer);
        let metadata = pg_sys::PageGetContents(lock_page) as *mut LinkedListData;
        (*metadata).start_blockno = start_blockno;
        (*metadata).last_blockno = start_blockno;

        pg_sys::MarkBufferDirty(lock_buffer);
        pg_sys::MarkBufferDirty(start_buffer);
        pg_sys::UnlockReleaseBuffer(start_buffer);

        Self {
            relation_oid,
            lock_buffer,
            lock: Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
        }
    }

    pub unsafe fn write(&mut self, bytes: &[u8]) -> Result<Vec<pg_sys::BlockNumber>> {
        let cache = BM25BufferCache::open(self.relation_oid);
        let mut blocks_created = vec![];
        let insert_blockno = self.get_last_blockno();
        let mut insert_buffer =
            cache.get_buffer(insert_blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));

        let mut data_cursor = Cursor::new(bytes);
        let mut bytes_written = 0;

        while bytes_written < bytes.len() {
            unsafe {
                let page = pg_sys::BufferGetPage(insert_buffer);
                let header = page as *mut pg_sys::PageHeaderData;
                let free_space = ((*header).pd_upper - (*header).pd_lower) as usize;
                assert!(free_space <= bm25_max_free_space());

                let bytes_to_write = min(free_space, bytes.len() - bytes_written);
                if bytes_to_write == 0 {
                    let new_buffer = cache.new_buffer();
                    let new_blockno = pg_sys::BufferGetBlockNumber(new_buffer);
                    let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
                    (*special).next_blockno = new_blockno;

                    pg_sys::MarkBufferDirty(insert_buffer);
                    pg_sys::UnlockReleaseBuffer(insert_buffer);

                    insert_buffer = new_buffer;
                    blocks_created.push(new_blockno);
                    self.set_last_blockno(new_blockno);
                    continue;
                }

                let page_slice = from_raw_parts_mut(
                    (page as *mut u8).add((*header).pd_lower as usize),
                    bytes_to_write,
                );
                data_cursor.read_exact(page_slice)?;
                bytes_written += bytes_to_write;
                (*header).pd_lower += bytes_to_write as u16;
            }
        }

        unsafe {
            pg_sys::MarkBufferDirty(insert_buffer);
            pg_sys::UnlockReleaseBuffer(insert_buffer);
        };

        Ok(blocks_created)
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
            let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
            let page = pg_sys::BufferGetPage(buffer);
            let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
            blockno = (*special).next_blockno;
            page.mark_deleted();

            pg_sys::MarkBufferDirty(buffer);
            pg_sys::UnlockReleaseBuffer(buffer);
        }

        let lock_page = pg_sys::BufferGetPage(self.lock_buffer);
        lock_page.mark_deleted();
        pg_sys::MarkBufferDirty(self.lock_buffer);
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

    pub unsafe fn get_all_blocks(&self) -> Vec<pg_sys::BlockNumber> {
        let cache = BM25BufferCache::open(self.relation_oid);
        let mut blockno = self.get_start_blockno();
        let mut blocks = vec![];

        while blockno != pg_sys::InvalidBlockNumber {
            blocks.push(blockno);
            let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
            let page = pg_sys::BufferGetPage(buffer);
            let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
            blockno = (*special).next_blockno;
            pg_sys::UnlockReleaseBuffer(buffer);
        }

        blocks
    }
}

impl Drop for LinkedBytesList {
    fn drop(&mut self) {
        unsafe {
            if pg_sys::IsTransactionState() {
                if self.lock.is_some() {
                    pg_sys::UnlockReleaseBuffer(self.lock_buffer);
                } else {
                    pg_sys::ReleaseBuffer(self.lock_buffer);
                }
            }
        };
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;

    // TODO: Add tests for LinkedItemList
    // TODO: Test all functions above

    #[pg_test]
    unsafe fn test_linked_bytes() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        // TODO: Swap back to bm25 index once this works
        Spi::run("CREATE INDEX t_idx ON t(id, data)").unwrap();
        // Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        // Test read/write from newly created linked list
        let bytes: Vec<u8> = (1..=255).cycle().take(100_000).collect();
        let start_blockno = {
            let mut linked_list = LinkedBytesList::create(relation_oid);
            let blocks_created = linked_list.write(&bytes).unwrap();
            let read_bytes = linked_list.read_all();
            assert_eq!(bytes, read_bytes);
            assert!(blocks_created.len() > 0);

            pg_sys::BufferGetBlockNumber(linked_list.lock_buffer)
        };

        // Test read from already created linked list
        let linked_list = LinkedBytesList::open_with_lock(
            relation_oid,
            start_blockno,
            Some(pg_sys::BUFFER_LOCK_SHARE),
        );
        let read_bytes = linked_list.read_all();
        assert_eq!(bytes, read_bytes);
    }
}
