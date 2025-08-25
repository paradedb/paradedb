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

use super::block::{bm25_max_free_space, BM25PageSpecialData, LinkedList, LinkedListData};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::blocklist;
use crate::postgres::storage::buffer::{init_new_buffer, BufferManager, PageHeaderMethods};
use anyhow::Result;
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

        let new_buffers_needed = {
            let buffer = self.list.bman.get_buffer(self.last_blockno);
            let page = buffer.page();
            let free_space = page.header().free_space();
            ((bytes.len().saturating_sub(free_space)) as f64 / bm25_max_free_space() as f64).ceil()
                as usize
        };
        let mut new_buffers = self.list.bman.new_buffers(new_buffers_needed);

        while bytes_written < bytes.len() {
            check_for_interrupts!();
            self.blocklist_builder.push(self.last_blockno);

            let mut buffer = self.list.bman.get_buffer_mut(self.last_blockno);
            let mut page = buffer.page_mut();
            let free_space = page.header().free_space();
            assert!(free_space <= bm25_max_free_space());

            let bytes_to_write = min(free_space, bytes.len() - bytes_written);
            if bytes_to_write == 0 {
                let mut new_buffer = new_buffers.next().unwrap_or_else(|| {
                    panic!(
                        "{} buffers was not enough for {} bytes",
                        new_buffers_needed,
                        bytes.len()
                    )
                });

                // Set next blockno
                let new_blockno = new_buffer.number();
                let special = page.special_mut::<BM25PageSpecialData>();
                special.next_blockno = new_blockno;

                // Initialize new page
                new_buffer.init_page();

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

    /// Write the pending [`BlockList`] data to disk.  This must be called once, as soon as the caller
    /// is positive this [`LinkedBytesListWriter`] is itself complete and fully written to disk.
    pub fn finalize_and_write(mut self) -> std::io::Result<LinkedBytesList> {
        // now that we're being finalized we can set the `last_blockno` of our metadata page
        // to the one we've internally tracked during .write()
        let mut header_buffer = self
            .list
            .bman
            .get_buffer_mut(self.list.get_header_blockno());

        let mut header_page = header_buffer.page_mut();
        let metadata = header_page.contents_mut::<LinkedListData>();
        metadata.last_blockno = self.last_blockno;

        if let Some(blockno) = self.blocklist_builder.finish(&mut self.list.bman) {
            metadata.blocklist_start = blockno;
        }
        Ok(self.list)
    }
}

impl Write for LinkedBytesListWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        unsafe {
            self.write(buf).map_err(std::io::Error::other)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // we don't do any buffering so there's nothing to flush
        Ok(())
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

    fn block_for_ord(&self, ord: usize) -> Option<pg_sys::BlockNumber> {
        self.blocklist_reader
            .get_or_init(|| {
                blocklist::reader::BlockList::new(
                    &self.bman,
                    self.get_linked_list_data().blocklist_start,
                )
            })
            .get(ord)
    }
}

#[derive(Debug)]
pub enum RangeData {
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
            RangeData::MultiPage(vec) => vec.len(),
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        match self {
            RangeData::MultiPage(vec) => vec.as_ptr(),
        }
    }
}

impl Deref for RangeData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            RangeData::MultiPage(vec) => &vec[..],
        }
    }
}

impl LinkedBytesList {
    pub fn open(rel: &PgSearchRelation, header_blockno: pg_sys::BlockNumber) -> Self {
        Self {
            bman: BufferManager::new(rel),
            header_blockno,
            blocklist_reader: Default::default(),
        }
    }

    /// Create a new [`LinkedBytesList`] in the specified `indexrel`'s block storage.  This method
    /// creates the necessary initial block structure without trying to use recycled pages from
    /// the [`FreeSpaceManager`].
    ///
    /// This is required if this object is created during `CREATE INDEX`/`REINDEX` as part of the
    /// initial index structure and the FSM hasn't been initialized yet.
    pub unsafe fn create_without_fsm(rel: &PgSearchRelation) -> pg_sys::BlockNumber {
        let (mut header_buffer, start_buffer) = (init_new_buffer(rel), init_new_buffer(rel));
        let header_blockno = header_buffer.number();
        let start_blockno = start_buffer.number();

        let mut header_page = header_buffer.page_mut();
        let metadata = header_page.contents_mut::<LinkedListData>();
        metadata.start_blockno = start_blockno;
        metadata.last_blockno = start_blockno;
        metadata.blocklist_start = pg_sys::InvalidBlockNumber;

        header_blockno
    }

    /// Create a new [`LinkedBytesList`] in the specified `indexrel`'s block storage.  This method
    /// will attempt to create the initial block structure using recycled blocks from the [`FreeSpaceManager`].
    pub fn create_with_fsm(rel: &PgSearchRelation) -> Self {
        let mut bman = BufferManager::new(rel);
        let mut buffers = bman.new_buffers(2);

        let mut header_buffer = buffers.next().unwrap();
        let header_blockno = header_buffer.number();
        let mut start_buffer = buffers.next().unwrap();
        let start_blockno = start_buffer.number();

        let mut header_page = header_buffer.init_page();
        start_buffer.init_page();

        let metadata = header_page.contents_mut::<LinkedListData>();
        metadata.start_blockno = start_blockno;
        metadata.last_blockno = start_blockno;
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

    /// Returns a lazily-evaluated iterator of all the [`pg_sys::BlockNumber`]s used by this [`LinkedBytesList`].
    ///
    /// There's no locking per-se that happens while the returned Iterator emits block numbers.  It's
    /// possible the place where block numbers are found happen to themselves live on a block and in
    /// reading that block it will be locked with a share lock, but none of this provides any sort of
    /// consistency guarantees around this [`LinkedBytesList`]'s physical representation in the face
    /// of concurrency.
    ///
    /// [`LinkedBytesList`]s are immutable, up until the point where they're being reclaimed
    /// as free space and added to the [`FreeSpaceManager`].
    ///
    /// Which is the only time this function should be called.  That is, when it's otherwise known that
    /// no other concurrent Postgres backend would have open any block that will be returned from
    /// this function.
    ///
    /// We take care of this, elsewhere, through our constructs like the [`PinCushion`], the [`MergeLock`],
    /// and atomically managing the segment entries list through an atomic copy-on-write approach.
    pub fn freeable_blocks(mut self) -> impl Iterator<Item = pg_sys::BlockNumber> {
        // in addition to the list itself, we also have a secondary list of linked blocks (which
        // contain the blocknumbers of this list) that needs to be marked deleted too

        let mut blocklist_blockno = self.get_linked_list_data().blocklist_start;
        // iterate the BlockList contents -- this is every block used by this LinkedBytesList
        self.blocklist_reader
            .take()
            .unwrap_or_else(|| blocklist::reader::BlockList::new(&self.bman, blocklist_blockno))
            .into_iter()
            // include our header page
            .chain(std::iter::once(self.header_blockno))
            // the BlockList itself consumes one or more blocks -- make sure to include them too
            .chain(std::iter::from_fn(move || {
                if blocklist_blockno == pg_sys::InvalidBlockNumber {
                    return None;
                }
                let blockno = blocklist_blockno;
                let buffer = self.bman.get_buffer(blockno);
                blocklist_blockno = buffer.page().next_blockno();
                Some(blockno)
            }))
    }

    /// Return all the allocated blocks used by this [`LinkedBytesList`] back to the
    /// Free Space Map behind this index.
    pub unsafe fn return_to_fsm(self) {
        let mut bman = self.bman().clone();
        let fsm = bman.fsm();
        fsm.extend(&mut bman, self.freeable_blocks());
    }

    pub fn is_empty(&self) -> bool {
        self.bman.page_is_empty(self.get_start_blockno().0)
    }

    pub unsafe fn get_bytes_range(&self, range: Range<usize>) -> RangeData {
        const ITEM_SIZE: usize = bm25_max_free_space();

        // find the closest block in the linked list to where `range` begins
        let start_block_ord = range.start / ITEM_SIZE;
        let mut blockno = self
            .block_for_ord(start_block_ord)
            .expect("block not found");

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

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::postgres::rel::PgSearchRelation;
    use crate::postgres::storage::block::BM25PageSpecialData;
    use crate::postgres::storage::utils::RelationBufferAccess;
    use pgrx::prelude::*;

    #[pg_test]
    unsafe fn test_linked_bytes_read_write() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();
        let indexrel = PgSearchRelation::open(relation_oid);

        // Test read/write from newly created linked list
        let bytes: Vec<u8> = (1..=255).cycle().take(100_000).collect();
        let start_blockno = {
            let linked_list = LinkedBytesList::create_with_fsm(&indexrel);
            let mut writer = linked_list.writer();
            writer.write(&bytes).unwrap();
            let linked_list = writer.finalize_and_write().unwrap();
            let read_bytes = linked_list.read_all();
            assert_eq!(bytes, read_bytes);

            linked_list.header_blockno
        };

        // Test read from already created linked list
        let linked_list = LinkedBytesList::open(&indexrel, start_blockno);
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
        let indexrel = PgSearchRelation::open(relation_oid);

        let linked_list = LinkedBytesList::create_with_fsm(&indexrel);
        assert!(linked_list.is_empty());

        let bytes: Vec<u8> = (1..=255).cycle().take(100_000).collect();
        let mut writer = linked_list.writer();
        writer.write(&bytes).unwrap();
        let linked_list = writer.finalize_and_write().unwrap();
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
        let indexrel = PgSearchRelation::open(relation_oid);

        let linked_list = LinkedBytesList::create_with_fsm(&indexrel);
        let bytes: Vec<u8> = (1..=255).cycle().take(100_000).collect();
        let mut writer = linked_list.writer();
        writer.write(&bytes).unwrap();
        let linked_list = writer.finalize_and_write().unwrap();
        let (mut blockno, _) = linked_list.get_start_blockno();
        linked_list.return_to_fsm();

        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = RelationBufferAccess::open(&indexrel)
                .get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
            let page = pg_sys::BufferGetPage(buffer);
            let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;

            // NB:  There was a time when the call to `linked_list.returm_to_fsm()` above would
            // update every page in the list, setting the `xmax` in the special data to the transaction id
            // of the transaction that deleted it.
            //
            // Our custom FSM does not do this, and so now we assert that the xmax value is still invalid
            // it's actually no longer used anywhere.
            assert!((*special).xmax == pg_sys::InvalidTransactionId);
            blockno = (*special).next_blockno;
            pg_sys::UnlockReleaseBuffer(buffer);
        }
    }
}
