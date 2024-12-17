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
use crate::index::channel::NeedWal;
use crate::postgres::storage::buffer::BufferManager;
use anyhow::{bail, Result};
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

    fn get_relation_oid(&self) -> pg_sys::Oid {
        self.relation_oid
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

    pub unsafe fn garbage_collect(&mut self) -> Result<()> {
        // let cache = &self.cache;

        // Delete all items that are definitely dead
        let snapshot = pg_sys::GetActiveSnapshot();
        let heap_oid = unsafe { pg_sys::IndexGetRelation(self.relation_oid, false) };
        let heap_relation = unsafe { pg_sys::RelationIdGetRelation(heap_oid) };
        let start_blockno = self.get_start_blockno();
        let mut blockno = start_blockno;

        while blockno != pg_sys::InvalidBlockNumber {
            let mut buffer = self.bman.get_buffer_mut(blockno);
            let mut page = buffer.page_mut();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();
            let mut delete_offsets = vec![];

            while offsetno <= max_offset {
                let entry = page.read_item::<T>(offsetno);

                if entry.recyclable(snapshot, heap_relation) {
                    delete_offsets.push(offsetno);
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

    pub unsafe fn add_items(&mut self, items: Vec<T>) -> Result<()> {
        // let cache = &self.cache;

        for item in items {
            let PgItem(pg_item, size) = item.into();

            // Find a page with free space and lock it
            // TODO: Do we need to start from the beginning every time?
            'insert_loop: loop {
                let mut blockno = self.get_start_blockno();
                let insert_buffer = loop {
                    if blockno == pg_sys::InvalidBlockNumber {
                        break None;
                        // break pg_sys::InvalidBuffer as i32;
                    }

                    let buffer = self.bman.get_buffer_mut(blockno);
                    let free_space = buffer.page().free_space();
                    if free_space >= size + size_of::<pg_sys::ItemIdData>() {
                        break Some(buffer);
                    } else {
                        blockno = buffer.next_blockno();
                    }
                };

                if let Some(mut insert_buffer) = insert_buffer {
                    // We have found an existing page with free space -- append to it
                    let mut page = insert_buffer.page_mut();
                    let offsetno = page.append_item(pg_item, size, 0);

                    assert_ne!(
                        offsetno,
                        pg_sys::InvalidOffsetNumber,
                        "failed to add item to existing page {}",
                        blockno
                    );
                    break 'insert_loop;
                } else {
                    // There are no more pages with free space, we need to create a new one
                    // First, validate that another process has not already created a new page
                    let last_blockno = self.get_last_blockno();
                    let mut last_buffer = self.bman.get_buffer_mut(last_blockno);

                    // If another process has already created a new page, restart the insert loop
                    if last_buffer.next_blockno() != pg_sys::InvalidBlockNumber {
                        continue 'insert_loop;
                    }

                    // We indeed have the last page, create a new one
                    let mut new_buffer = self.bman.new_buffer();
                    let new_blockno = new_buffer.number();
                    let mut new_page = new_buffer.init_page();

                    // Update the last page to point to the new page
                    let mut last_page = last_buffer.page_mut();
                    let special = last_page.special_mut::<BM25PageSpecialData>();
                    special.next_blockno = new_blockno;

                    // Update the header to point to the new last page
                    let mut header_buffer = self.bman.get_buffer_mut(self.header_blockno);
                    let mut page = header_buffer.page_mut();
                    let metadata = page.contents_mut::<LinkedListData>();
                    metadata.inner.last_blockno = new_blockno;
                    metadata.inner.npages += 1;

                    // Add the item to the new page
                    let offsetno = new_page.append_item(pg_item, size, 0);
                    assert_ne!(
                        offsetno,
                        pg_sys::InvalidOffsetNumber,
                        "failed to add item to new page",
                    );

                    break 'insert_loop;
                }
            }
        }

        Ok(())
    }

    pub unsafe fn lookup<K>(
        &self,
        target: K,
        cmp: fn(&T, &K) -> bool,
    ) -> Result<(T, pg_sys::BlockNumber, pg_sys::OffsetNumber)>
    where
        K: Debug,
    {
        let mut blockno = self.get_start_blockno();

        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = self.bman.get_buffer(blockno);
            let page = buffer.page();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();

            while offsetno <= max_offset {
                let deserialized = page.read_item::<T>(offsetno);

                if cmp(&deserialized, &target) {
                    return Ok((deserialized, blockno, offsetno));
                }
                offsetno += 1;
            }

            blockno = buffer.next_blockno();
        }

        bail!(
            "transaction {} failed to find {:?}",
            pg_sys::GetCurrentTransactionId(),
            target
        );
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    use crate::postgres::storage::block::DirectoryEntry;

    #[pg_test]
    unsafe fn test_linked_items_garbage_collect_single_page() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let snapshot = pg_sys::GetActiveSnapshot();
        let delete_xid = (*snapshot).xmin - 1;

        let mut list = LinkedItemList::<DirectoryEntry>::create(relation_oid, true);
        let entries_to_delete = vec![
            DirectoryEntry {
                path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
                start: 10,
                total_bytes: 100_usize,
                xmin: delete_xid,
                xmax: delete_xid,
            },
            DirectoryEntry {
                path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
                start: 12,
                total_bytes: 200_usize,
                xmin: delete_xid,
                xmax: delete_xid,
            },
        ];
        let entries_to_keep = vec![DirectoryEntry {
            path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
            start: 10,
            total_bytes: 100_usize,
            xmin: (*snapshot).xmin - 1,
            xmax: pg_sys::InvalidTransactionId,
        }];

        list.add_items(entries_to_delete.clone()).unwrap();
        list.add_items(entries_to_keep.clone()).unwrap();
        list.garbage_collect().unwrap();

        assert!(list
            .lookup(entries_to_delete[0].clone(), |a, b| a.path == b.path)
            .is_err());
        assert!(list
            .lookup(entries_to_delete[1].clone(), |a, b| a.path == b.path)
            .is_err());
        assert!(list
            .lookup(entries_to_keep[0].clone(), |a, b| a.path == b.path)
            .is_ok());
    }

    #[pg_test]
    unsafe fn test_linked_items_garbage_collect_multiple_pages() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let snapshot = pg_sys::GetActiveSnapshot();
        let deleted_xid = (*snapshot).xmin - 1;
        let not_deleted_xid = pg_sys::InvalidTransactionId;
        let xmin = (*snapshot).xmin - 1;

        // Add 2000 entries, delete every 10th entry
        {
            let mut list = LinkedItemList::<DirectoryEntry>::create(relation_oid, true);
            let entries = (1..2000)
                .map(|i| DirectoryEntry {
                    path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
                    start: i,
                    total_bytes: 100_usize,
                    xmin,
                    xmax: if i % 10 == 0 {
                        deleted_xid
                    } else {
                        not_deleted_xid
                    },
                })
                .collect::<Vec<_>>();

            list.add_items(entries.clone()).unwrap();
            list.garbage_collect().unwrap();

            for entry in entries {
                if entry.xmax == not_deleted_xid {
                    assert!(list.lookup(entry.clone(), |a, b| a.path == b.path).is_ok());
                } else {
                    assert!(list.lookup(entry.clone(), |a, b| a.path == b.path).is_err());
                }
            }
        }

        // First n pages are full, next m pages need to be compacted, next n are full
        {
            let mut list = LinkedItemList::<DirectoryEntry>::create(relation_oid, true);
            let entries_1 = (1..500)
                .map(|i| DirectoryEntry {
                    path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
                    start: i,
                    total_bytes: 100_usize,
                    xmin,
                    xmax: not_deleted_xid,
                })
                .collect::<Vec<_>>();
            list.add_items(entries_1.clone()).unwrap();

            let entries_2 = (1..1000)
                .map(|i| DirectoryEntry {
                    path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
                    start: i,
                    total_bytes: 100_usize,
                    xmin,
                    xmax: if i % 10 == 0 {
                        not_deleted_xid
                    } else {
                        deleted_xid
                    },
                })
                .collect::<Vec<_>>();
            list.add_items(entries_2.clone()).unwrap();

            let entries_3 = (1..500)
                .map(|i| DirectoryEntry {
                    path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
                    start: i,
                    total_bytes: 100_usize,
                    xmin,
                    xmax: not_deleted_xid,
                })
                .collect::<Vec<_>>();
            list.add_items(entries_3.clone()).unwrap();

            list.garbage_collect().unwrap();

            for entries in [entries_1, entries_2, entries_3] {
                for entry in entries {
                    if entry.xmax == not_deleted_xid {
                        assert!(list.lookup(entry.clone(), |a, b| a.path == b.path).is_ok());
                    } else {
                        assert!(list.lookup(entry.clone(), |a, b| a.path == b.path).is_err());
                    }
                }
            }
        }
    }
}
