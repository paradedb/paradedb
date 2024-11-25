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

use super::block::BM25PageSpecialData;
use super::utils::{BM25BufferCache, BM25Page};
use crate::postgres::storage::block::bm25_max_free_space;
use anyhow::{bail, Result};
use pgrx::pg_sys;
use std::cmp::min;
use std::fmt::Debug;
use std::io::{Cursor, Read};
use std::slice::{from_raw_parts, from_raw_parts_mut};

pub struct PgItem(pub pg_sys::Item, pub pg_sys::Size);

/// Linked list implementation over block storage,
/// where each node in the list is a pg_sys::Item
pub struct LinkedItemList<T: From<PgItem> + Into<PgItem> + Debug + Clone> {
    relation_oid: pg_sys::Oid,
    pub start: pg_sys::BlockNumber,
    _marker: std::marker::PhantomData<T>,
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone> LinkedItemList<T> {
    pub fn open(relation_oid: pg_sys::Oid, start: pg_sys::BlockNumber) -> Self {
        Self {
            relation_oid,
            start,
            _marker: std::marker::PhantomData,
        }
    }

    pub unsafe fn create(relation_oid: pg_sys::Oid) -> Self {
        let cache = BM25BufferCache::open(relation_oid);
        let start_buffer = cache.new_buffer();
        let start_blockno = pg_sys::BufferGetBlockNumber(start_buffer);
        let page = pg_sys::BufferGetPage(start_buffer);
        let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
        (*special).last_blockno = start_blockno;

        pg_sys::UnlockReleaseBuffer(start_buffer);

        Self {
            relation_oid,
            start: start_blockno,
            _marker: std::marker::PhantomData,
        }
    }

    pub unsafe fn delete(&self) {
        let cache = BM25BufferCache::open(self.relation_oid);
        let mut blockno = self.start;
        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
            let page = pg_sys::BufferGetPage(buffer);
            let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
            blockno = (*special).next_blockno;
            page.mark_deleted();

            pg_sys::MarkBufferDirty(buffer);
            pg_sys::UnlockReleaseBuffer(buffer);
        }
    }

    pub unsafe fn write(&mut self, items: Vec<T>) -> Result<()> {
        let cache = BM25BufferCache::open(self.relation_oid);

        let start_buffer = cache.get_buffer(self.start, Some(pg_sys::BUFFER_LOCK_SHARE));
        let page = pg_sys::BufferGetPage(start_buffer);
        let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
        let insert_blockno = if (*special).last_blockno == pg_sys::InvalidBlockNumber {
            self.start
        } else {
            (*special).last_blockno
        };
        pg_sys::UnlockReleaseBuffer(start_buffer);

        let mut insert_buffer =
            cache.get_buffer(insert_blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
        let mut insert_page = pg_sys::BufferGetPage(insert_buffer);

        for item in items {
            let PgItem(pg_item, size) = item.clone().into();
            eprintln!("inserting item: {:?} size {}", item, size);
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
                (*special).next_blockno = new_blockno;

                pg_sys::MarkBufferDirty(insert_buffer);
                pg_sys::UnlockReleaseBuffer(insert_buffer);

                insert_buffer = new_buffer;
                insert_page = pg_sys::BufferGetPage(insert_buffer);

                let start_buffer =
                    cache.get_buffer(self.start, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
                let page = pg_sys::BufferGetPage(start_buffer);
                let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
                (*special).last_blockno = new_blockno;
                pg_sys::UnlockReleaseBuffer(start_buffer);

                offsetno = pg_sys::PageAddItemExtended(
                    insert_page,
                    pg_item,
                    size,
                    pg_sys::InvalidOffsetNumber,
                    0,
                );

                if offsetno == pg_sys::InvalidOffsetNumber {
                    pg_sys::UnlockReleaseBuffer(insert_buffer);
                    bail!("Failed to add item");
                }
            }
        }

        pg_sys::MarkBufferDirty(insert_buffer);
        pg_sys::UnlockReleaseBuffer(insert_buffer);

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
        let mut blockno = self.start;

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

    pub unsafe fn list_all_items(&self) -> Result<Vec<T>> {
        let mut items = Vec::new();
        let cache = BM25BufferCache::open(self.relation_oid);
        let mut blockno = self.start;

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
    start: pg_sys::BlockNumber,
}

impl LinkedBytesList {
    pub fn open(relation_oid: pg_sys::Oid, start: pg_sys::BlockNumber) -> Self {
        Self {
            relation_oid,
            start,
        }
    }

    pub unsafe fn write(&mut self, bytes: &[u8]) -> Result<Vec<pg_sys::BlockNumber>> {
        let cache = BM25BufferCache::open(self.relation_oid);
        let mut blocks_created = vec![];

        let start_buffer = cache.get_buffer(self.start, Some(pg_sys::BUFFER_LOCK_SHARE));
        let page = pg_sys::BufferGetPage(start_buffer);
        let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
        let insert_blockno = if (*special).last_blockno == pg_sys::InvalidBlockNumber {
            self.start
        } else {
            (*special).last_blockno
        };
        pg_sys::UnlockReleaseBuffer(start_buffer);

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

                    let start_buffer =
                        cache.get_buffer(self.start, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
                    let page = pg_sys::BufferGetPage(start_buffer);
                    let special = pg_sys::PageGetSpecialPointer(page) as *mut BM25PageSpecialData;
                    (*special).last_blockno = new_blockno;
                    pg_sys::UnlockReleaseBuffer(start_buffer);

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
        let mut blockno = self.start;
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
    use pgrx::pg_sys::AsPgCStr;
    use pgrx::prelude::*;
    use std::collections::HashSet;
    use std::path::PathBuf;
    use uuid::Uuid;

    impl From<PgItem> for PathBuf {
        fn from(pg_item: PgItem) -> Self {
            let PgItem(item, size) = pg_item;
            let path_str = unsafe {
                std::str::from_utf8(from_raw_parts(item as *const u8, size))
                    .expect("expected valid Utf-8")
            };
            PathBuf::from(path_str)
        }
    }

    impl Into<PgItem> for PathBuf {
        fn into(self) -> PgItem {
            let path_str = self.to_str().expect("file path is not valid UTF-8");
            PgItem(
                path_str.as_pg_cstr() as pg_sys::Item,
                path_str.as_bytes().len() as pg_sys::Size,
            )
        }
    }

    #[pg_test]
    unsafe fn test_pathbuf_linked_list() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();

        let cache = BM25BufferCache::open(relation_oid);
        let start_buffer = cache.new_buffer();
        let start_blockno = pg_sys::BufferGetBlockNumber(start_buffer);
        pg_sys::UnlockReleaseBuffer(start_buffer);

        let mut linked_list = LinkedItemList::<PathBuf>::open(relation_oid, start_blockno);

        let files: Vec<PathBuf> = (0..10000)
            .map(|_| {
                let uuid = Uuid::new_v4();
                let mut path = PathBuf::new();
                path.set_file_name(format!("{}.ext", uuid));
                path
            })
            .collect();

        for file in &files {
            linked_list.write(vec![file.clone()]).unwrap();
        }

        let listed_files = linked_list.list_all_items().unwrap();
        let superset = listed_files.iter().collect::<HashSet<_>>();
        let subset = files.iter().collect::<HashSet<_>>();
        assert!(superset.is_superset(&subset));

        for i in (0..10000).step_by(100) {
            let target = files.get(i).expect("expected file");
            let (found, _, _) = linked_list
                .lookup(target.clone(), |target, path| *target == *path)
                .unwrap();
            assert_eq!(found, *target);
        }

        let invalid_file = PathBuf::from("invalid_file.ext");
        assert!(linked_list
            .lookup(invalid_file, |target, path| *target == *path)
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
        let start_buffer = cache.new_buffer();
        let start_blockno = pg_sys::BufferGetBlockNumber(start_buffer);
        pg_sys::UnlockReleaseBuffer(start_buffer);

        let mut linked_list = LinkedBytesList::open(relation_oid, start_blockno);
        let bytes: Vec<u8> = (1..=255).cycle().take(100_000).collect();
        linked_list.write(&bytes).unwrap();
        let read_bytes = linked_list.read_all();
        assert_eq!(bytes, read_bytes);
    }
}
