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

use super::block::{BM25PageSpecialData, LinkedList, LinkedListData, MVCCEntry, PgItem};
use super::buffer::{init_new_buffer, BufferManager, BufferMut};
use crate::postgres::rel::PgSearchRelation;
use anyhow::Result;
use pgrx::pg_sys;
use pgrx::pg_sys::BlockNumber;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
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
    pub header_blockno: pg_sys::BlockNumber,
    bman: BufferManager,
    _marker: std::marker::PhantomData<T>,
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> LinkedList for LinkedItemList<T> {
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
        unimplemented!(
            "block_for_ord is not implemented for LinkedItemList.  caller requested ord: {ord}"
        )
    }
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> LinkedItemList<T> {
    pub fn open(indexrel: &PgSearchRelation, header_blockno: pg_sys::BlockNumber) -> Self {
        Self {
            header_blockno,
            bman: BufferManager::new(indexrel),
            _marker: std::marker::PhantomData,
        }
    }

    pub unsafe fn create_direct(indexrel: &PgSearchRelation) -> pg_sys::BlockNumber {
        let mut header_buffer = init_new_buffer(indexrel);
        let start_buffer = init_new_buffer(indexrel);

        let header_blockno = header_buffer.number();
        let start_blockno = start_buffer.number();

        let mut header_page = header_buffer.page_mut();
        let metadata = header_page.contents_mut::<LinkedListData>();
        metadata.start_blockno = start_blockno;
        metadata.last_blockno = start_blockno;
        metadata.npages = 0;

        header_blockno
    }

    pub fn create(indexrel: &PgSearchRelation) -> Self {
        let (mut _self, mut header_buffer) = Self::create_without_start_page(indexrel);

        let mut start_buffer = _self.bman.new_buffer();
        let start_blockno = start_buffer.number();
        start_buffer.init_page();

        let mut header_page = header_buffer.page_mut();
        let metadata = header_page.contents_mut::<LinkedListData>();
        metadata.start_blockno = start_blockno;
        metadata.last_blockno = start_blockno;
        metadata.npages = 0;

        _self
    }

    fn create_without_start_page(indexrel: &PgSearchRelation) -> (Self, BufferMut) {
        let mut bman = BufferManager::new(indexrel);

        let mut header_buffer = bman.new_buffer();
        let header_blockno = header_buffer.number();
        header_buffer.init_page();

        (
            Self {
                header_blockno,
                bman,
                _marker: std::marker::PhantomData,
            },
            header_buffer,
        )
    }

    /// Return a Vec of all the items in this linked list
    pub unsafe fn list(&self) -> Vec<T> {
        let mut items = vec![];
        let (mut blockno, mut buffer) = self.get_start_blockno();

        while blockno != pg_sys::InvalidBlockNumber {
            buffer = self.bman.get_buffer_exchange(blockno, buffer);
            let page = buffer.page();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();
            while offsetno <= max_offset {
                if let Some((deserialized, _)) = page.deserialize_item::<T>(offsetno) {
                    items.push(deserialized);
                }
                offsetno += 1;
            }
            blockno = page.next_blockno();
        }

        items
    }

    pub unsafe fn is_empty(&self) -> bool {
        let (mut blockno, mut buffer) = self.get_start_blockno();

        while blockno != pg_sys::InvalidBlockNumber {
            buffer = self.bman.get_buffer_exchange(blockno, buffer);
            let page = buffer.page();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();
            while offsetno <= max_offset {
                if let Some((_, _)) = page.deserialize_item::<T>(offsetno) {
                    return false;
                }
                offsetno += 1;
            }
            blockno = page.next_blockno();
        }

        true
    }

    pub unsafe fn garbage_collect(&mut self) -> Vec<T> {
        // Delete all items that are definitely dead
        self.retain(|bman, entry| {
            if entry.recyclable(bman) {
                RetainItem::Remove(entry)
            } else {
                RetainItem::Retain
            }
        })
    }

    ///
    /// Mutate the list in-place by optionally removing, replacing, or retaining each entry. Returns
    /// the list of removed entries.
    ///
    /// Note that this method will necessarily write WAL entries for every buffer in the list,
    /// because it must acquire each buffer as mutable.
    ///
    pub unsafe fn retain(
        &mut self,
        mut f: impl FnMut(&mut BufferManager, T) -> RetainItem<T>,
    ) -> Vec<T> {
        let (mut blockno, mut previous_buffer) = self.get_start_blockno_mut();
        let mut is_first_block = true;

        let mut recycled_entries = Vec::new();
        let mut delete_offsets = vec![];
        while blockno != pg_sys::InvalidBlockNumber {
            let mut buffer = self.bman.get_buffer_mut(blockno);
            let mut page = buffer.page_mut();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();

            while offsetno <= max_offset {
                if let Some((entry, _)) = page.deserialize_item::<T>(offsetno) {
                    match f(self.bman_mut(), entry) {
                        RetainItem::Remove(entry) => {
                            page.mark_item_dead(offsetno);

                            recycled_entries.push(entry);
                            delete_offsets.push(offsetno);
                        }
                        RetainItem::Retain => {}
                    }
                }
                offsetno += 1;
            }

            if !delete_offsets.is_empty() {
                page.delete_items(&mut delete_offsets);
                delete_offsets.clear();
            }

            let new_max_offset = page.max_offset_number();
            blockno = page.next_blockno();

            // Compaction step: If the page is entirely empty, mark as deleted
            // Adjust the pointer from the last known non-empty node to point to the next non-empty node
            if new_max_offset == pg_sys::InvalidOffsetNumber && !is_first_block {
                // this page is no longer useful to us: remove it from the linked list by adjusting
                // the previous page to point at the next page.
                previous_buffer
                    .page_mut()
                    .special_mut::<BM25PageSpecialData>()
                    .next_blockno = blockno;

                // return it to the FSM. Doing so will also drop the lock, but we are still
                // holding the lock on the previous page, so hand-over-hand is ensured.
                buffer.return_to_fsm(&mut self.bman);
            } else {
                // this is either the start page, or a page containing valid data. move its buffer
                // into previous_buffer to ensure that it is held hand-over-hand until we decide
                // that we might need to free some future buffer.
                previous_buffer = buffer;
            }

            is_first_block = false;
        }

        recycled_entries
    }

    ///
    /// Visit each entry, without mutating entries or the list structure.
    ///
    pub unsafe fn for_each(&mut self, mut f: impl FnMut(&mut BufferManager, T)) {
        let (mut blockno, mut buffer) = self.get_start_blockno();
        while blockno != pg_sys::InvalidBlockNumber {
            buffer = self.bman().get_buffer_exchange(blockno, buffer);
            let page = buffer.page();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();
            while offsetno <= max_offset {
                if let Some((deserialized, _)) = page.deserialize_item(offsetno) {
                    f(self.bman_mut(), deserialized);
                }
                offsetno += 1;
            }
            blockno = page.next_blockno();
        }
    }

    pub unsafe fn add_items(&mut self, items: &[T], buffer: Option<BufferMut>) {
        let mut buffer = if let Some(buffer) = buffer {
            buffer
        } else {
            let (start_blockno, buffer) = self.get_start_blockno_mut();
            self.bman.get_buffer_exchange_mut(start_blockno, buffer)
        };

        for item in items {
            let PgItem(pg_item, size) = item.clone().into();

            'append_loop: loop {
                let mut page = buffer.page_mut();
                let offsetno = page.append_item(pg_item, size, 0);
                if offsetno != pg_sys::InvalidOffsetNumber {
                    // it added to this block
                    break 'append_loop;
                } else if page.next_blockno() != pg_sys::InvalidBlockNumber {
                    // go to the next block
                    let next_blockno = page.next_blockno();
                    buffer = self.bman.get_buffer_mut(next_blockno);
                } else {
                    // need to create new block and link it to this one
                    let mut new_page = self.bman.new_buffer();
                    let new_blockno = new_page.number();
                    new_page.init_page();

                    let special = page.special_mut::<BM25PageSpecialData>();
                    special.next_blockno = new_blockno;

                    buffer = new_page;
                }
            }
        }
    }

    pub unsafe fn remove_item<Cmp: Fn(&T) -> bool>(&mut self, cmp: Cmp) -> Result<T> {
        let (entry, blockno, offsetno) = self.lookup_ex(cmp)?;

        let mut buffer = self.bman.get_buffer_for_cleanup(blockno);
        let mut page = buffer.page_mut();
        page.delete_item(offsetno);

        Ok(entry)
    }

    #[allow(dead_code)]
    pub unsafe fn lookup<Cmp: Fn(&T) -> bool>(&self, cmp: Cmp) -> Result<T> {
        self.lookup_ex(cmp).map(|(t, _, _)| t)
    }

    pub unsafe fn lookup_ex<Cmp: Fn(&T) -> bool>(
        &self,
        cmp: Cmp,
    ) -> Result<(T, pg_sys::BlockNumber, pg_sys::OffsetNumber)> {
        let (mut blockno, mut buffer) = self.get_start_blockno();
        while blockno != pg_sys::InvalidBlockNumber {
            buffer = self.bman.get_buffer_exchange(blockno, buffer);
            let page = buffer.page();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();

            while offsetno <= max_offset {
                if let Some((deserialized, _)) = page.deserialize_item::<T>(offsetno) {
                    if cmp(&deserialized) {
                        return Ok((deserialized, blockno, offsetno));
                    }
                }
                offsetno += 1;
            }

            blockno = page.next_blockno();
        }

        // if we get here, we didn't find what we were looking for
        // but we should have -- how else could we have asked for something from the list?
        Err(anyhow::anyhow!(format!(
            "transaction {} failed to find the desired entry",
            pg_sys::GetCurrentTransactionIdIfAny()
        )))
    }

    ///
    /// Make a temporary copy of this list and all its backing buffers to allow for atomic
    /// mutation. When the given guard is `commit()`ed, all changes will become visible atomically
    /// in this list.
    ///
    /// TODO: For small lists which mutate large portions of this collection, we would likely be
    /// better off converting it into a Rust collection (which would be easier to manipulate), and
    /// then re-storing it from scratch to commit.
    ///
    pub unsafe fn atomically(&mut self) -> AtomicGuard<'_, T> {
        // While we're operating on the List atomically, hold an exclusive lock on its header.
        let original_header_lock = {
            let header_blockno = self.header_blockno;
            self.bman_mut().get_buffer_mut(header_blockno)
        };

        // We create the duplicate without a start page: it will be filled in in the first
        // iteration of the loop below.
        let (mut cloned, mut previous_buffer) =
            LinkedItemList::create_without_start_page(self.bman.bm25cache().rel());

        // TODO: This code could either:
        // * switch to compacting pages as it goes.
        // * switch to using memcpy
        // ... but not both.
        let mut blockno = original_header_lock
            .page()
            .contents::<LinkedListData>()
            .start_blockno;
        assert!(
            blockno != pg_sys::InvalidBlockNumber,
            "Must contain at least one block."
        );
        let mut previous_new_buffer: Option<BufferMut> = None;
        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = self.bman.get_buffer_mut(blockno);
            let page = buffer.page();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();

            let mut new_buffer = cloned.bman.new_buffer();
            let new_blockno = new_buffer.number();
            let mut new_page = new_buffer.init_page();

            // Link the new block in with the previous block, or with the header.
            if let Some(mut previous_new_buffer) = previous_new_buffer {
                previous_new_buffer
                    .page_mut()
                    .special_mut::<BM25PageSpecialData>()
                    .next_blockno = new_blockno;
            } else {
                // In the case of the first block, our previous buffer points to the header.
                previous_buffer
                    .page_mut()
                    .contents_mut::<LinkedListData>()
                    .start_blockno = new_blockno;
            }

            while offsetno <= max_offset {
                if let Some(PgItem(item, size)) = page.read_item(offsetno) {
                    let new_offsetno = new_page.append_item(item, size, 0);
                    assert!(new_offsetno != pg_sys::InvalidOffsetNumber);
                }
                offsetno += 1;
            }

            *new_page.special_mut::<BM25PageSpecialData>() =
                (*page.special::<BM25PageSpecialData>()).clone();
            blockno = page.next_blockno();

            // We hold the previous_buffer in order to ensure hand-over-hand locking on the
            // original list.
            previous_buffer = buffer;
            previous_new_buffer = Some(new_buffer);
        }

        AtomicGuard {
            original: Some((self, original_header_lock)),
            cloned,
        }
    }
}

pub enum RetainItem<T> {
    Remove(T),
    Retain,
}

pub struct AtomicGuard<'s, T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> {
    original: Option<(&'s mut LinkedItemList<T>, BufferMut)>,
    cloned: LinkedItemList<T>,
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> Deref for AtomicGuard<'_, T> {
    type Target = LinkedItemList<T>;

    fn deref(&self) -> &Self::Target {
        &self.cloned
    }
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> DerefMut for AtomicGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cloned
    }
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> AtomicGuard<'_, T> {
    pub fn commit(mut self) {
        let (original, mut original_header_lock) =
            self.original.take().expect("Cannot commit twice!");

        // Update our header page to point to the new start block, and return the old one.
        let mut blockno = {
            // Open both header pages.
            let mut original_header_page = original_header_lock.page_mut();
            let mut cloned_header_buffer =
                self.cloned.bman.get_buffer_mut(self.cloned.header_blockno);
            let mut cloned_header_page = cloned_header_buffer.page_mut();

            // Capture our old start page, then overwrite our metadata.
            let original_metadata = original_header_page.contents_mut::<LinkedListData>();
            let old_start_blockno = original_metadata.start_blockno;
            *original_metadata = *cloned_header_page.contents_mut::<LinkedListData>();

            // Finally, garbage collect the cloned header block.
            cloned_header_buffer.return_to_fsm(&mut self.cloned.bman);

            old_start_blockno
        };

        // Drop the header, to ensure that the new header content is written to the WAL before
        // we begin cleaning up its old pages.
        std::mem::drop(original_header_lock);

        // And then collect our old contents, which are no longer reachable.
        let recyclable_block = std::iter::from_fn(move || {
            if blockno == pg_sys::InvalidBlockNumber {
                return None;
            }
            let recyclable_blockno = blockno;
            let buffer = original.bman.get_buffer(blockno);
            blockno = buffer.page().next_blockno();
            Some(recyclable_blockno)
        });
        self.bman.fsm().extend(&mut self.bman, recyclable_block);
    }
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> Drop for AtomicGuard<'_, T> {
    fn drop(&mut self) {
        if self.original.is_none() {
            // We committed successfully.
            return;
        };

        unsafe {
            if !pg_sys::IsTransactionState() {
                // we are not in a transaction, so we can't release buffers
                return;
            }
        }

        // The guard was dropped without a call to commit: return its pages.
        let header_blockno = self.cloned.header_blockno;
        let bman = self.cloned.bman().clone();
        let header_buffer = bman.get_buffer(header_blockno);
        let mut blockno = header_buffer
            .page()
            .contents::<LinkedListData>()
            .start_blockno;
        drop(header_buffer);

        let recyclable_blocks =
            std::iter::once(header_blockno).chain(std::iter::from_fn(move || {
                if blockno == pg_sys::InvalidBlockNumber {
                    return None;
                }

                let recyable_blockno = blockno;
                let buffer = bman.get_buffer(blockno);
                blockno = buffer.page().next_blockno();
                Some(recyable_blockno)
            }));
        let fsm = self.bman.fsm();
        fsm.extend(&mut self.bman, recyclable_blocks);
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::api::HashSet;
    use pgrx::prelude::*;
    use tantivy::index::SegmentId;
    use uuid::Uuid;

    use crate::postgres::rel::PgSearchRelation;
    use crate::postgres::storage::block::{FileEntry, SegmentMetaEntry};

    fn random_segment_id() -> SegmentId {
        SegmentId::from_uuid_string(&Uuid::new_v4().to_string()).unwrap()
    }

    fn linked_list_block_numbers(
        list: &LinkedItemList<SegmentMetaEntry>,
    ) -> HashSet<pg_sys::BlockNumber> {
        let (mut blockno, _) = list.get_start_blockno();
        let mut block_numbers = HashSet::default();

        while blockno != pg_sys::InvalidBlockNumber {
            block_numbers.insert(blockno);
            let buffer = list.bman().get_buffer(blockno);
            let page = buffer.page();
            blockno = page.next_blockno();
        }

        block_numbers
    }

    #[pg_test]
    unsafe fn test_linked_items_garbage_collect_single_page() {
        let relation_oid = init_bm25_index();
        let indexrel = PgSearchRelation::open(relation_oid);
        let delete_xid = pg_sys::FrozenTransactionId;

        let mut list = LinkedItemList::<SegmentMetaEntry>::create(&indexrel);
        let entries_to_delete = vec![SegmentMetaEntry {
            segment_id: random_segment_id(),
            xmax: delete_xid,
            postings: Some(make_fake_postings(&indexrel)),
            ..Default::default()
        }];
        let entries_to_keep = vec![SegmentMetaEntry {
            segment_id: random_segment_id(),
            xmax: pg_sys::InvalidTransactionId,
            postings: Some(make_fake_postings(&indexrel)),
            ..Default::default()
        }];

        list.add_items(&entries_to_delete, None);
        list.add_items(&entries_to_keep, None);
        list.garbage_collect();

        assert!(list
            .lookup(|entry| entry.segment_id == entries_to_delete[0].segment_id)
            .is_err());
        assert!(list
            .lookup(|entry| entry.segment_id == entries_to_keep[0].segment_id)
            .is_ok());
    }

    #[pg_test]
    unsafe fn test_linked_items_garbage_collect_multiple_pages() {
        let relation_oid = init_bm25_index();
        let indexrel = PgSearchRelation::open(relation_oid);

        let deleted_xid = pg_sys::FrozenTransactionId;
        let not_deleted_xid = pg_sys::InvalidTransactionId;

        // Add 2000 entries, delete every 10th entry
        {
            let mut list = LinkedItemList::<SegmentMetaEntry>::create(&indexrel);
            let entries = (1..2000)
                .map(|i| SegmentMetaEntry {
                    segment_id: random_segment_id(),
                    xmax: if i % 10 == 0 {
                        deleted_xid
                    } else {
                        not_deleted_xid
                    },
                    postings: Some(make_fake_postings(&indexrel)),
                    ..Default::default()
                })
                .collect::<Vec<_>>();

            list.add_items(&entries, None);
            list.garbage_collect();

            for entry in entries {
                if entry.xmax == not_deleted_xid {
                    assert!(list.lookup(|el| el.segment_id == entry.segment_id).is_ok());
                } else {
                    assert!(list.lookup(|el| el.segment_id == entry.segment_id).is_err());
                }
            }
        }
        // First n pages are full, next m pages need to be compacted, next n are full
        {
            let mut list = LinkedItemList::<SegmentMetaEntry>::create(&indexrel);
            let entries_1 = (1..500)
                .map(|_| SegmentMetaEntry {
                    segment_id: random_segment_id(),
                    xmax: not_deleted_xid,
                    postings: Some(make_fake_postings(&indexrel)),
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            list.add_items(&entries_1, None);

            let entries_2 = (1..1000)
                .map(|_| SegmentMetaEntry {
                    segment_id: random_segment_id(),
                    xmax: deleted_xid,
                    postings: Some(make_fake_postings(&indexrel)),
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            list.add_items(&entries_2, None);

            let entries_3 = (1..500)
                .map(|_| SegmentMetaEntry {
                    segment_id: random_segment_id(),
                    xmax: not_deleted_xid,
                    postings: Some(make_fake_postings(&indexrel)),
                    ..Default::default()
                })
                .collect::<Vec<_>>();

            list.add_items(&entries_3, None);

            let pre_gc_blocks = linked_list_block_numbers(&list);
            list.garbage_collect();

            for entries in [entries_1, entries_2, entries_3] {
                for entry in entries {
                    if entry.xmax == not_deleted_xid {
                        assert!(list.lookup(|el| el.segment_id == entry.segment_id).is_ok());
                    } else {
                        assert!(list.lookup(|el| el.segment_id == entry.segment_id).is_err());
                    }
                }
            }

            // Assert that compaction produced a smaller list
            let post_gc_blocks = linked_list_block_numbers(&list);
            assert!(pre_gc_blocks.len() > post_gc_blocks.len());
        }
    }

    #[pg_test]
    unsafe fn test_linked_items_duplicate_then_replace() {
        let relation_oid = init_bm25_index();
        let indexrel = PgSearchRelation::open(relation_oid);

        // Add 2000 entries.
        let mut list = LinkedItemList::<SegmentMetaEntry>::create(&indexrel);
        let entries = (1..2000)
            .map(|_| SegmentMetaEntry {
                segment_id: random_segment_id(),
                xmax: pg_sys::InvalidTransactionId,
                postings: Some(make_fake_postings(&indexrel)),
                ..Default::default()
            })
            .collect::<Vec<_>>();

        list.add_items(&entries, None);

        // Atomically modify the list, and then confirm that it contains unique blocks, and the
        // same contents.
        let list_block_numbers = linked_list_block_numbers(&list);
        let list_contents = list.list();
        let guard = list.atomically();
        let duplicate_block_numbers = linked_list_block_numbers(&guard);
        let duplicate_contents = guard.list();
        assert_eq!(
            list_block_numbers
                .intersection(&duplicate_block_numbers)
                .cloned()
                .collect::<Vec<_>>(),
            Vec::<pg_sys::BlockNumber>::new(),
        );
        assert_eq!(list_contents, duplicate_contents);

        // Then `commit()` the guard, and confirm that the structure of the original list
        // matches afterwards.
        guard.commit();
        assert_eq!(linked_list_block_numbers(&list), duplicate_block_numbers);
        assert_eq!(list.list(), duplicate_contents);
    }

    fn init_bm25_index() -> pg_sys::Oid {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
            .expect("spi should succeed")
            .unwrap()
    }

    fn make_fake_postings(indexrel: &PgSearchRelation) -> FileEntry {
        let mut postings_file_block = BufferManager::new(indexrel).new_buffer();
        postings_file_block.init_page();
        FileEntry {
            starting_block: postings_file_block.number(),
            total_bytes: 0,
        }
    }
}
