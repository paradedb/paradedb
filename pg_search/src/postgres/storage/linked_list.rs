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

use super::block::{BM25PageSpecialData, MetaPageData, METADATA_BLOCKNO};
use super::utils::{BM25BufferCache, BM25Page};
use anyhow::{bail, Result};
use pgrx::pg_sys;
use std::cmp::min;
use std::fmt::Debug;
use std::io::{Cursor, Read};
use std::slice::{from_raw_parts, from_raw_parts_mut};

use crate::postgres::storage::block::bm25_max_free_space;

pub trait LinkedItem {
    fn from_pg_item(item: pg_sys::Item, size: pg_sys::Size) -> Self;
    fn as_pg_item(&self) -> (pg_sys::Item, pg_sys::Size);
}

/// Linked list implementation over block storage,
/// where each node in the list is a pg_sys::Item
pub struct LinkedItemList<T: LinkedItem + Debug> {
    relation_oid: pg_sys::Oid,
    get_first_blockno: fn(*mut MetaPageData) -> pg_sys::BlockNumber,
    get_last_blockno: fn(*mut MetaPageData) -> pg_sys::BlockNumber,
    set_first_blockno: fn(*mut MetaPageData, pg_sys::BlockNumber),
    set_last_blockno: fn(*mut MetaPageData, pg_sys::BlockNumber),
    _marker: std::marker::PhantomData<T>,
}

impl<T: LinkedItem + Debug> LinkedItemList<T> {
    pub fn new(
        relation_oid: pg_sys::Oid,
        get_first_blockno: fn(*mut MetaPageData) -> pg_sys::BlockNumber,
        get_last_blockno: fn(*mut MetaPageData) -> pg_sys::BlockNumber,
        set_first_blockno: fn(*mut MetaPageData, pg_sys::BlockNumber),
        set_last_blockno: fn(*mut MetaPageData, pg_sys::BlockNumber),
    ) -> Self {
        Self {
            relation_oid,
            get_first_blockno,
            get_last_blockno,
            set_first_blockno,
            set_last_blockno,
            _marker: std::marker::PhantomData,
        }
    }

    pub unsafe fn add_items<CacheFn>(
        &mut self,
        items: Vec<T>,
        overwrite: bool,
        update_cache: CacheFn,
    ) -> Result<()>
    where
        CacheFn: Fn(&T, pg_sys::BlockNumber, pg_sys::OffsetNumber),
    {
        let cache = BM25BufferCache::open(self.relation_oid);
        // It's important that we hold this lock for the duration of the function because we may be overwriting the list
        let metadata_buffer =
            cache.get_buffer(METADATA_BLOCKNO, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
        let metadata_page = pg_sys::BufferGetPage(metadata_buffer);
        let metadata = pg_sys::PageGetContents(metadata_page) as *mut MetaPageData;

        if overwrite {
            let mut blockno = (self.get_first_blockno)(metadata);

            while blockno != pg_sys::InvalidBlockNumber {
                let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
                let page = pg_sys::BufferGetPage(buffer);
                let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
                blockno = (*special).next_blockno;
                page.mark_deleted();

                pg_sys::MarkBufferDirty(buffer);
                pg_sys::UnlockReleaseBuffer(buffer);
            }

            let new_buffer = cache.new_buffer();
            let new_blockno = pg_sys::BufferGetBlockNumber(new_buffer);
            (self.set_first_blockno)(metadata, new_blockno);
            (self.set_last_blockno)(metadata, new_blockno);

            pg_sys::MarkBufferDirty(new_buffer);
            pg_sys::UnlockReleaseBuffer(new_buffer);
        }

        let mut insert_blockno = (self.get_last_blockno)(metadata);
        let mut insert_buffer =
            cache.get_buffer(insert_blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
        let mut insert_page = pg_sys::BufferGetPage(insert_buffer);

        for item in &items {
            let (pg_item, size) = item.as_pg_item();
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
                let new_blockno = pg_sys::BufferGetBlockNumber(new_buffer);
                (self.set_last_blockno)(metadata, new_blockno);
                (*special).next_blockno = new_blockno;

                pg_sys::MarkBufferDirty(metadata_buffer);
                pg_sys::MarkBufferDirty(insert_buffer);
                pg_sys::UnlockReleaseBuffer(insert_buffer);

                insert_buffer = new_buffer;
                insert_blockno = new_blockno;
                insert_page = pg_sys::BufferGetPage(insert_buffer);

                offsetno = pg_sys::PageAddItemExtended(
                    insert_page,
                    pg_item,
                    size,
                    pg_sys::InvalidOffsetNumber,
                    0,
                );

                if offsetno == pg_sys::InvalidOffsetNumber {
                    pg_sys::UnlockReleaseBuffer(insert_buffer);
                    pg_sys::UnlockReleaseBuffer(metadata_buffer);
                    bail!("Failed to add item {:?}", item);
                }
            }

            update_cache(item, insert_blockno, offsetno);
        }

        pg_sys::MarkBufferDirty(insert_buffer);
        pg_sys::UnlockReleaseBuffer(insert_buffer);
        pg_sys::UnlockReleaseBuffer(metadata_buffer);

        Ok(())
    }

    pub unsafe fn lookup<EqFn, CacheFn>(
        &self,
        eq: EqFn,
        update_cache: CacheFn,
    ) -> Result<(T, pg_sys::BlockNumber, pg_sys::OffsetNumber)>
    where
        EqFn: Fn(&T) -> bool,
        CacheFn: Fn(&T, pg_sys::BlockNumber, pg_sys::OffsetNumber),
    {
        let cache = BM25BufferCache::open(self.relation_oid);
        let metadata_buffer = cache.get_buffer(METADATA_BLOCKNO, Some(pg_sys::BUFFER_LOCK_SHARE));
        let metadata_page = pg_sys::BufferGetPage(metadata_buffer);
        let metadata = pg_sys::PageGetContents(metadata_page) as *mut MetaPageData;
        let mut blockno = (self.get_first_blockno)(metadata);

        pg_sys::UnlockReleaseBuffer(metadata_buffer);

        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
            let page = pg_sys::BufferGetPage(buffer);
            let mut offsetno = pg_sys::FirstOffsetNumber;

            while offsetno <= pg_sys::PageGetMaxOffsetNumber(page) {
                let item_id = pg_sys::PageGetItemId(page, offsetno);
                let deserialized = T::from_pg_item(
                    pg_sys::PageGetItem(page, item_id),
                    (*item_id).lp_len() as pg_sys::Size,
                );

                update_cache(&deserialized, blockno, offsetno);

                if eq(&deserialized) {
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

        bail!("failed to find item in linked list");
    }

    pub unsafe fn list_all_items(&self) -> Result<Vec<T>> {
        let mut items = Vec::new();
        let cache = BM25BufferCache::open(self.relation_oid);
        let metadata_buffer = cache.get_buffer(METADATA_BLOCKNO, Some(pg_sys::BUFFER_LOCK_SHARE));
        let metadata_page = pg_sys::BufferGetPage(metadata_buffer);
        let metadata = pg_sys::PageGetContents(metadata_page) as *mut MetaPageData;
        let mut blockno = (self.get_first_blockno)(metadata);

        pg_sys::UnlockReleaseBuffer(metadata_buffer);

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
                let item = T::from_pg_item(
                    pg_sys::PageGetItem(page, item_id),
                    (*item_id).lp_len() as pg_sys::Size,
                );
                items.push(item);
            }

            let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
            blockno = (*special).next_blockno;
            pg_sys::UnlockReleaseBuffer(buffer);
        }

        Ok(items)
    }
}

/// Linked list implementation over block storage,
/// where each node is a page filled with bytes (with the potential exception of the last page)
pub struct LinkedBytesList {
    relation_oid: pg_sys::Oid,
    first_blockno: pg_sys::BlockNumber,
    last_blockno: pg_sys::BlockNumber,
}

impl LinkedBytesList {
    pub fn new(
        relation_oid: pg_sys::Oid,
        first_blockno: pg_sys::BlockNumber,
        last_blockno: pg_sys::BlockNumber,
    ) -> Self {
        Self {
            relation_oid,
            first_blockno,
            last_blockno,
        }
    }

    pub unsafe fn write(
        &mut self,
        bytes: &[u8],
        overwrite: bool,
    ) -> Result<Vec<pg_sys::BlockNumber>> {
        let cache = BM25BufferCache::open(self.relation_oid);
        if overwrite {
            let mut blockno = self.first_blockno;

            while blockno != pg_sys::InvalidBlockNumber {
                let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
                let page = pg_sys::BufferGetPage(buffer);
                let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
                blockno = (*special).next_blockno;
                page.mark_deleted();

                pg_sys::MarkBufferDirty(buffer);
                pg_sys::UnlockReleaseBuffer(buffer);
            }

            let new_buffer = cache.new_buffer();
            let new_blockno = pg_sys::BufferGetBlockNumber(new_buffer);
            self.first_blockno = new_blockno;
            self.last_blockno = new_blockno;

            pg_sys::MarkBufferDirty(new_buffer);
            pg_sys::UnlockReleaseBuffer(new_buffer);
        }

        let mut insert_blockno = self.last_blockno;
        let mut insert_buffer =
            cache.get_buffer(insert_blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
        let mut insert_page = pg_sys::BufferGetPage(insert_buffer);

        let mut data_cursor = Cursor::new(bytes);
        let mut bytes_written = 0;
        let mut blocks_created = vec![];

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
                    self.last_blockno = new_blockno;
                    blocks_created.push(new_blockno);

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
        let mut blockno = self.first_blockno;
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
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;
    use std::collections::HashSet;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[pg_test]
    unsafe fn test_pathbuf_linked_list() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run(
            "CALL paradedb.create_bm25(
            index_name => 't_idx',
            table_name => 't',
            key_field => 'id',
            text_fields => paradedb.field('data')
        )",
        )
        .unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let mut linked_list = LinkedItemList::<PathBuf>::new(
            relation_oid,
            |metadata| unsafe { (*metadata).tantivy_managed_first_blockno },
            |metadata| unsafe { (*metadata).tantivy_managed_last_blockno },
            |metadata, blockno| unsafe { (*metadata).tantivy_managed_first_blockno = blockno },
            |metadata, blockno| unsafe { (*metadata).tantivy_managed_last_blockno = blockno },
        );

        let files: Vec<PathBuf> = (0..10000)
            .map(|_| {
                let uuid = Uuid::new_v4();
                let mut path = PathBuf::new();
                path.set_file_name(format!("{}.ext", uuid));
                path
            })
            .collect();

        for file in &files {
            linked_list
                .add_items(vec![file.clone()], false, |_, _, _| {})
                .unwrap();
        }

        let listed_files = linked_list.list_all_items().unwrap();
        let superset = listed_files.iter().collect::<HashSet<_>>();
        let subset = files.iter().collect::<HashSet<_>>();
        assert!(superset.is_superset(&subset));

        for i in (0..10000).step_by(100) {
            let target = files.get(i).expect("expected file");
            let (found, _, _) = linked_list.lookup(|i| *i == *target, |_, _, _| {}).unwrap();
            assert_eq!(found, *target);
        }

        let invalid_file = PathBuf::from("invalid_file.ext");
        assert!(linked_list
            .lookup(|i| *i == invalid_file, |_, _, _| {})
            .is_err());
    }

    #[pg_test]
    unsafe fn test_linked_bytes() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let cache = BM25BufferCache::open(relation_oid);
        let buffer = cache.new_buffer();
        let blockno = pg_sys::BufferGetBlockNumber(buffer);
        pg_sys::UnlockReleaseBuffer(buffer);

        let mut linked_list = LinkedBytesList::new(relation_oid, blockno, blockno);
        let bytes: Vec<u8> = (1..=255).cycle().take(100_000).collect();
        let blocks = linked_list.write(&bytes, true);
        let read_bytes = linked_list.read_all();
        assert_eq!(bytes, read_bytes);
    }
}
