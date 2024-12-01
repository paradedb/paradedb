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
use super::utils::{BM25BufferCache, BM25Page};
use anyhow::{bail, Result};
use pgrx::pg_sys;
use std::fmt::Debug;

// ---------------------------------------------------------------
// Linked list implementation over block storage,
// where each node in the list is a pg_sys::Item
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
    /// Open an existing linked list and holds the specified lock on it until the linked list is dropped
    pub fn open(relation_oid: pg_sys::Oid, header_blockno: pg_sys::BlockNumber) -> Self {
        Self {
            relation_oid,
            header_blockno,
            _marker: std::marker::PhantomData,
        }
    }

    /// Create a new linked list and holds an exclusive lock on it until the linked list is dropped
    pub unsafe fn create(relation_oid: pg_sys::Oid) -> Self {
        let cache = BM25BufferCache::open(relation_oid);
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
        (*metadata).start_blockno = start_blockno;
        (*metadata).last_blockno = start_blockno;

        // Set pd_lower to the end of the metadata
        // Without doing so, metadata will be lost if xlog.c compresses the page
        let page_header = header_page as *mut pg_sys::PageHeaderData;
        (*page_header).pd_lower = (metadata.add(1) as usize - header_page as usize) as u16;

        pg_sys::GenericXLogFinish(state);
        pg_sys::UnlockReleaseBuffer(header_buffer);
        pg_sys::UnlockReleaseBuffer(start_buffer);

        Self {
            relation_oid,
            header_blockno,
            _marker: std::marker::PhantomData,
        }
    }

    pub unsafe fn garbage_collect(&mut self) -> Result<()> {
        // Collect all entries that are not definitely deleted
        let mut entries_to_keep = vec![];
        let snapshot = pg_sys::GetActiveSnapshot();
        let heap_oid = unsafe { pg_sys::IndexGetRelation(self.relation_oid, false) };
        let heap_relation = unsafe { pg_sys::RelationIdGetRelation(heap_oid) };

        for (entry, _, _) in self.list_all_items()? {
            let definitely_deleted = entry.is_deleted()
                && !pg_sys::XidInMVCCSnapshot(entry.get_xmax(), snapshot)
                && pg_sys::GlobalVisCheckRemovableXid(heap_relation, entry.get_xmax());
            if !definitely_deleted {
                entries_to_keep.push(entry);
            }
        }

        // Mark all buffer as deleted besides the lock buffer
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
        let start_buffer = cache.new_buffer();
        let start_page = pg_sys::GenericXLogRegisterBuffer(
            state,
            start_buffer,
            pg_sys::GENERIC_XLOG_FULL_IMAGE as i32,
        );
        start_page.init(pg_sys::BufferGetPageSize(start_buffer));

        let start_blockno = pg_sys::BufferGetBlockNumber(start_buffer);
        let header_buffer =
            cache.get_buffer(self.header_blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
        let header_page = pg_sys::GenericXLogRegisterBuffer(state, header_buffer, 0);
        let metadata = pg_sys::PageGetContents(header_page) as *mut LinkedListData;
        (*metadata).start_blockno = start_blockno;
        (*metadata).last_blockno = start_blockno;

        pg_sys::GenericXLogFinish(state);
        pg_sys::UnlockReleaseBuffer(header_buffer);
        pg_sys::UnlockReleaseBuffer(start_buffer);

        // Write garbage collected entries
        self.add_items(entries_to_keep)
    }

    pub unsafe fn add_items(&mut self, items: Vec<T>) -> Result<()> {
        let cache = BM25BufferCache::open(self.relation_oid);

        for item in items {
            let insert_blockno = self.get_last_blockno();
            let insert_buffer =
                cache.get_buffer(insert_blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
            let state = cache.start_xlog();
            let insert_page = pg_sys::GenericXLogRegisterBuffer(state, insert_buffer, 0);
            let PgItem(pg_item, size) = item.clone().into();

            let mut offsetno = pg_sys::PageAddItemExtended(
                insert_page,
                pg_item,
                size,
                pg_sys::InvalidOffsetNumber,
                0,
            );
            if offsetno == pg_sys::InvalidOffsetNumber {
                let special =
                    pg_sys::PageGetSpecialPointer(insert_page) as *mut BM25PageSpecialData;
                let new_buffer = cache.new_buffer();
                let new_page = pg_sys::GenericXLogRegisterBuffer(
                    state,
                    new_buffer,
                    pg_sys::GENERIC_XLOG_FULL_IMAGE as i32,
                );
                new_page.init(pg_sys::BufferGetPageSize(new_buffer));

                let new_blockno = pg_sys::BufferGetBlockNumber(new_buffer);
                (*special).next_blockno = new_blockno;

                let header_buffer = cache.get_buffer(
                    self.get_header_blockno(),
                    Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
                );

                let page = pg_sys::GenericXLogRegisterBuffer(state, header_buffer, 0);
                let metadata = pg_sys::PageGetContents(page) as *mut LinkedListData;
                (*metadata).last_blockno = new_blockno;

                offsetno = pg_sys::PageAddItemExtended(
                    new_page,
                    pg_item,
                    size,
                    pg_sys::InvalidOffsetNumber,
                    0,
                );

                if offsetno == pg_sys::InvalidOffsetNumber {
                    pg_sys::GenericXLogAbort(state);
                    pg_sys::UnlockReleaseBuffer(new_buffer);
                    pg_sys::UnlockReleaseBuffer(insert_buffer);
                    pg_sys::UnlockReleaseBuffer(header_buffer);
                    bail!("Failed to add item");
                }

                pg_sys::GenericXLogFinish(state);
                pg_sys::UnlockReleaseBuffer(new_buffer);
                pg_sys::UnlockReleaseBuffer(insert_buffer);
                pg_sys::UnlockReleaseBuffer(header_buffer);
            } else {
                pg_sys::GenericXLogFinish(state);
                pg_sys::UnlockReleaseBuffer(insert_buffer);
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
        let cache = BM25BufferCache::open(self.relation_oid);
        let mut blockno = self.get_start_blockno();

        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
            let page = pg_sys::BufferGetPage(buffer);
            let mut offsetno = pg_sys::FirstOffsetNumber;

            while offsetno <= pg_sys::PageGetMaxOffsetNumber(page) {
                let item_id = pg_sys::PageGetItemId(page, offsetno);
                let deserialized = T::from(PgItem(
                    pg_sys::PageGetItem(page, item_id),
                    (*item_id).lp_len() as pg_sys::Size,
                ));

                if cmp(&deserialized, &target) {
                    pg_sys::UnlockReleaseBuffer(buffer);
                    return Ok((deserialized, blockno, offsetno));
                }
                offsetno += 1;
            }

            blockno = {
                let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
                (*special).next_blockno
            };
            pg_sys::UnlockReleaseBuffer(buffer);
        }

        bail!("failed to find {:?}", target);
    }

    pub unsafe fn list_all_items(
        &self,
    ) -> Result<Vec<(T, pg_sys::BlockNumber, pg_sys::OffsetNumber)>> {
        let mut items = Vec::new();
        let cache = BM25BufferCache::open(self.relation_oid);
        let mut blockno = self.get_start_blockno();

        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
            let page = pg_sys::BufferGetPage(buffer);
            let max_offset = pg_sys::PageGetMaxOffsetNumber(page);

            if max_offset == pg_sys::InvalidOffsetNumber {
                pg_sys::UnlockReleaseBuffer(buffer);
                break;
            }

            for offsetno in 1..=max_offset {
                let item_id = pg_sys::PageGetItemId(page, offsetno);
                let item = T::from(PgItem(
                    pg_sys::PageGetItem(page, item_id),
                    (*item_id).lp_len() as pg_sys::Size,
                ));
                items.push((item, blockno, offsetno));
            }

            let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
            blockno = (*special).next_blockno;
            pg_sys::UnlockReleaseBuffer(buffer);
        }

        Ok(items)
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
    unsafe fn test_linked_items_garbage_collect() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let snapshot = pg_sys::GetActiveSnapshot();
        let delete_xid = (*snapshot).xmin - 1;

        let mut list = LinkedItemList::<DirectoryEntry>::create(relation_oid);
        let entries_to_delete = vec![
            DirectoryEntry {
                path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
                start: 10,
                total_bytes: 100 as usize,
                xmin: delete_xid,
                xmax: delete_xid,
            },
            DirectoryEntry {
                path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
                start: 12,
                total_bytes: 200 as usize,
                xmin: delete_xid,
                xmax: delete_xid,
            },
        ];
        list.add_items(entries_to_delete.clone()).unwrap();
        list.garbage_collect().unwrap();

        assert!(list
            .lookup(entries_to_delete[0].clone(), |a, b| a.path == b.path)
            .is_err());
        assert!(list
            .lookup(entries_to_delete[1].clone(), |a, b| a.path == b.path)
            .is_err());
    }

    #[pg_test]
    unsafe fn test_linked_items_list_all_items() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let mut list = LinkedItemList::<DirectoryEntry>::create(relation_oid);
        let entries = vec![
            DirectoryEntry {
                path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
                start: 10,
                total_bytes: 100 as usize,
                xmin: 1,
                xmax: 2,
            },
            DirectoryEntry {
                path: PathBuf::from(format!("{}.ext", Uuid::new_v4())),
                start: 12,
                total_bytes: 200 as usize,
                xmin: 3,
                xmax: 4,
            },
        ];

        list.add_items(entries.clone()).unwrap();
        let items = list.list_all_items().unwrap();
        assert_eq!(
            items
                .into_iter()
                .map(|(entry, _, _)| entry)
                .collect::<Vec<_>>(),
            entries
        );
    }
}
