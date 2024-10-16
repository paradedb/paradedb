use super::block::{BM25PageSpecialData, MetaPageData, METADATA_BLOCKNO};
use super::utils::{BM25BufferCache, BM25Page};
use anyhow::{bail, Result};
use pgrx::pg_sys;
use std::fmt::Debug;

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

                // eprintln!("lookup found item {:?}", deserialized);

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

        // linked_list.add_items(vec![], true).unwrap();
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
}
