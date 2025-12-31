// Copyright (C) 2023-2026 ParadeDB, Inc.
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
use crate::postgres::storage::fsm::FreeSpaceManager;
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

pub struct LinkedItemList<T: From<PgItem> + Into<PgItem> + Debug + Clone> {
    pub header_blockno: pg_sys::BlockNumber,
    bman: BufferManager,
    _marker: std::marker::PhantomData<T>,
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone> LinkedList for LinkedItemList<T> {
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

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone> LinkedItemList<T> {
    pub fn open(indexrel: &PgSearchRelation, header_blockno: pg_sys::BlockNumber) -> Self {
        Self {
            header_blockno,
            bman: BufferManager::new(indexrel),
            _marker: std::marker::PhantomData,
        }
    }

    /// Create a new [`LinkedItemList`] in the specified `indexrel`'s block storage.  This method
    /// creates the necessary initial block structure without trying to use recycled pages from
    /// the [`FreeSpaceManager`].
    ///
    /// This is required if this object is created during `CREATE INDEX`/`REINDEX` as part of the
    /// initial index structure and the FSM hasn't been initialized yet.
    pub unsafe fn create_without_fsm(indexrel: &PgSearchRelation) -> pg_sys::BlockNumber {
        let mut header_buffer = init_new_buffer(indexrel);
        let start_buffer = init_new_buffer(indexrel);

        let header_blockno = header_buffer.number();
        let start_blockno = start_buffer.number();

        let mut header_page = header_buffer.page_mut();
        let metadata = header_page.contents_mut::<LinkedListData>();
        metadata.start_blockno = start_blockno;
        metadata.last_blockno = start_blockno;

        header_blockno
    }

    /// Create a new [`LinkedItemList`] in the specified `indexrel`'s block storage.  This method
    /// will attempt to create the initial block structure using recycled blocks from the [`FreeSpaceManager`].
    pub fn create_with_fsm(indexrel: &PgSearchRelation) -> Self {
        let (mut _self, mut header_buffer) = Self::create_without_start_page(indexrel);

        let mut start_buffer = _self.bman.new_buffer();
        let start_blockno = start_buffer.number();
        start_buffer.init_page();

        let mut header_page = header_buffer.page_mut();
        let metadata = header_page.contents_mut::<LinkedListData>();
        metadata.start_blockno = start_blockno;
        metadata.last_blockno = start_blockno;

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
    pub unsafe fn list(&self, many: Option<usize>) -> Vec<T> {
        let mut items = vec![];
        let (mut blockno, mut buffer) = self.get_start_blockno();
        let mut found = 0;

        'outer: while blockno != pg_sys::InvalidBlockNumber {
            buffer = self.bman.get_buffer_exchange(blockno, buffer);
            let page = buffer.page();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();
            while offsetno <= max_offset {
                if let Some(many) = many {
                    if found >= many {
                        break 'outer;
                    }
                }

                if let Some((deserialized, _)) = page.deserialize_item::<T>(offsetno) {
                    items.push(deserialized);
                }
                offsetno += 1;
                found += 1;
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

    pub unsafe fn garbage_collect(&mut self, when_recyclable: pg_sys::FullTransactionId) -> Vec<T>
    where
        T: MVCCEntry,
    {
        // Delete all items that are definitely dead
        self.retain(when_recyclable, |bman, entry| {
            if entry.recyclable(bman) {
                RetainItem::Remove(entry)
            } else {
                RetainItem::Retain
            }
        })
    }

    /// Return the freeable blocks of this list, including the header block.
    pub fn freeable_blocks(self) -> impl Iterator<Item = pg_sys::BlockNumber> {
        unsafe {
            let (blockno, _) = self.get_start_blockno();
            std::iter::once(self.header_blockno)
                .chain(Self::freeable_blocks_without_header(self.bman, blockno))
        }
    }

    unsafe fn freeable_blocks_without_header(
        bman: BufferManager,
        start_blockno: pg_sys::BlockNumber,
    ) -> impl Iterator<Item = pg_sys::BlockNumber> {
        let mut blockno = start_blockno;
        std::iter::from_fn(move || {
            if blockno == pg_sys::InvalidBlockNumber {
                return None;
            }
            let freeable_blockno = blockno;
            let buffer = bman.get_buffer(blockno);
            blockno = buffer.page().next_blockno();
            Some(freeable_blockno)
        })
    }

    /// Mutate the list in-place by optionally removing, replacing, or retaining each entry. Returns
    /// the list of removed entries.
    pub unsafe fn retain(
        &mut self,
        when_recyclable: pg_sys::FullTransactionId,
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
                if page.item_is_dead(offsetno) {
                    offsetno += 1;
                    continue;
                }

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
                buffer.return_to_fsm_with_when_recyclable(&mut self.bman, when_recyclable);
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

    /// Visit each entry, without mutating entries or the list structure.
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

    /// Adds the given items to the list.
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

    /// Removes the first item which matches the given comparison function.
    pub unsafe fn remove_item<Cmp: Fn(&T) -> bool>(&mut self, cmp: Cmp) -> Result<T> {
        // Acquire and hold a shared lock on the header for the entire operation, preventing the
        // list from being swapped out from under us by atomically between our read locks and
        // our write locks.
        let header_lock = self.bman.get_buffer(self.header_blockno);
        let start_blockno = header_lock
            .page()
            .contents::<LinkedListData>()
            .start_blockno;

        loop {
            // Search while holding read locks.
            let (_, blockno, offsetno) = self.lookup_ex(&cmp, Some(start_blockno))?;

            // Acquire a write lock (a cleanup lock in particular, because we're shortening the
            // page), and double check that we're still looking at the correct item.
            let mut buffer = self.bman.get_buffer_for_cleanup(blockno);
            let mut page = buffer.page_mut();
            match page.deserialize_item::<T>(offsetno) {
                Some((item, _)) if cmp(&item) => {
                    page.delete_item(offsetno);

                    return Ok(item);
                }
                _ => {
                    // The page was mutated before we could acquire our cleanup lock. Continue to
                    // retry.
                    continue;
                }
            }
        }
    }

    /// Updates the first item which matches the given comparison function.
    ///
    /// The update function must result in an item which serializes to the same size as the input.
    /// TODO: We don't have any use cases for supporting changed sizes, but it's possible to add
    /// it.
    pub unsafe fn update_item<Cmp: Fn(&T) -> bool, Update: FnOnce(&mut T)>(
        &mut self,
        cmp: Cmp,
        update: Update,
    ) -> Result<()> {
        // Acquire and hold a shared lock on the header for the entire operation, preventing the
        // list from being swapped out from under us by atomically between our read locks and
        // our write locks.
        let header_lock = self.bman.get_buffer(self.header_blockno);
        let start_blockno = header_lock
            .page()
            .contents::<LinkedListData>()
            .start_blockno;

        loop {
            // Search while holding read locks.
            let (_, blockno, offsetno) = self.lookup_ex(&cmp, Some(start_blockno))?;

            // Acquire a write lock, and double check that we're still looking at the correct item.
            let mut buffer = self.bman.get_buffer_mut(blockno);
            let mut page = buffer.page_mut();
            match page.deserialize_item::<T>(offsetno) {
                Some((mut item, old_size)) if cmp(&item) => {
                    // We've confirmed that we've found the right item: now update it.
                    update(&mut item);

                    let PgItem(pg_item, size) = item.into();
                    assert_eq!(old_size, size, "`update_item` does not support updating items in ways which change their sizes.");
                    let replaced = page.replace_item(offsetno, pg_item, size);
                    // This should not be possible because we checked that the size did not change,
                    // but confirm it anyway.
                    assert!(replaced, "Failed to replace item.");
                    return Ok(());
                }
                _ => {
                    // The page was mutated before we could acquire our cleanup lock. Continue to
                    // retry.
                    continue;
                }
            }
        }
    }

    pub unsafe fn lookup<Cmp: Fn(&T) -> bool>(&self, cmp: Cmp) -> Result<T> {
        self.lookup_ex(cmp, None).map(|(t, _, _)| t)
    }

    /// NOTE: It is not safe to make a mutation based on the result of this method without
    /// double checking that it is still accurate under a write lock.
    pub unsafe fn lookup_ex<Cmp: Fn(&T) -> bool>(
        &self,
        cmp: Cmp,
        blockno: Option<pg_sys::BlockNumber>,
    ) -> Result<(T, pg_sys::BlockNumber, pg_sys::OffsetNumber)> {
        let mut blockno = blockno.unwrap_or_else(|| self.get_start_blockno().0);
        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = self.bman.get_buffer(blockno);
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
            LinkedItemList::create_without_start_page(self.bman.buffer_access().rel());

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

pub struct AtomicGuard<'s, T: From<PgItem> + Into<PgItem> + Debug + Clone> {
    original: Option<(&'s mut LinkedItemList<T>, BufferMut)>,
    cloned: LinkedItemList<T>,
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone> Deref for AtomicGuard<'_, T> {
    type Target = LinkedItemList<T>;

    fn deref(&self) -> &Self::Target {
        &self.cloned
    }
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone> DerefMut for AtomicGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cloned
    }
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone> AtomicGuard<'_, T> {
    pub fn commit(mut self) {
        let (original, mut original_header_lock) =
            self.original.take().expect("Cannot commit twice!");

        // Update our header page to point to the new start block, and return the old one.
        let start_blockno = {
            // The metadata from the cloned header is the one we want to become the new header for everyone
            let cloned_header_metadata = self
                .cloned
                .bman
                .get_buffer(self.cloned.header_blockno)
                .page()
                .contents::<LinkedListData>();

            // Capture our old start page, then overwrite our metadata.
            let mut original_header_page = original_header_lock.page_mut();
            let original_metadata = original_header_page.contents_mut::<LinkedListData>();
            let original_start_blockno = original_metadata.start_blockno;

            // Here we replace the original header with the cloned header and drop the original header lock
            // ensuring it is written to disk, which includes the WAL.
            *original_metadata = cloned_header_metadata;
            std::mem::drop(original_header_lock);

            original_start_blockno
        };

        // And then collect our old contents, which are no longer reachable.
        let recyclable_blocks = unsafe {
            LinkedItemList::<T>::freeable_blocks_without_header(
                original.bman.clone(),
                start_blockno,
            )
            .chain(std::iter::once(self.cloned.header_blockno))
        };
        self.bman.fsm().extend_with_when_recyclable(
            &mut self.bman,
            unsafe { pg_sys::ReadNextFullTransactionId() },
            recyclable_blocks,
        );
    }
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone> Drop for AtomicGuard<'_, T> {
    fn drop(&mut self) {
        if self.original.is_none() {
            // We committed successfully.
            return;
        };

        unsafe {
            if !pg_sys::IsTransactionState() || std::thread::panicking() {
                // we are not in a transaction, so we can't release buffers
                return;
            }
        }

        panic!("internal error: failed to call `AtomicGuard::commit()`");
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::api::HashSet;
    use pgrx::prelude::*;
    use tantivy::index::SegmentId;

    use crate::postgres::rel::PgSearchRelation;
    use crate::postgres::storage::block::{FileEntry, SegmentMetaEntry, SegmentMetaEntryImmutable};

    fn random_segment_id() -> SegmentId {
        SegmentId::generate_random()
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

        let mut list = LinkedItemList::<SegmentMetaEntry>::create_with_fsm(&indexrel);
        let entries_to_delete = vec![SegmentMetaEntry::new_immutable(
            random_segment_id(),
            0,
            delete_xid,
            SegmentMetaEntryImmutable {
                postings: Some(make_fake_postings(&indexrel)),
                ..Default::default()
            },
        )];
        let entries_to_keep = vec![SegmentMetaEntry::new_immutable(
            random_segment_id(),
            0,
            pg_sys::InvalidTransactionId,
            SegmentMetaEntryImmutable {
                postings: Some(make_fake_postings(&indexrel)),
                ..Default::default()
            },
        )];

        list.add_items(&entries_to_delete, None);
        list.add_items(&entries_to_keep, None);
        list.garbage_collect(pg_sys::FullTransactionId {
            value: delete_xid.into_inner() as u64,
        });

        assert!(list
            .lookup(|entry| entry.segment_id() == entries_to_delete[0].segment_id())
            .is_err());
        assert!(list
            .lookup(|entry| entry.segment_id() == entries_to_keep[0].segment_id())
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
            let mut list = LinkedItemList::<SegmentMetaEntry>::create_with_fsm(&indexrel);
            let entries = (1..2000)
                .map(|i| {
                    SegmentMetaEntry::new_immutable(
                        random_segment_id(),
                        0,
                        if i % 10 == 0 {
                            deleted_xid
                        } else {
                            not_deleted_xid
                        },
                        SegmentMetaEntryImmutable {
                            postings: Some(make_fake_postings(&indexrel)),
                            ..Default::default()
                        },
                    )
                })
                .collect::<Vec<_>>();

            list.add_items(&entries, None);
            list.garbage_collect(pg_sys::FullTransactionId {
                value: deleted_xid.into_inner() as u64,
            });

            for entry in entries {
                if entry.xmax() == not_deleted_xid {
                    assert!(list
                        .lookup(|el| el.segment_id() == entry.segment_id())
                        .is_ok());
                } else {
                    assert!(list
                        .lookup_ex(|el| el.segment_id() == entry.segment_id(), None)
                        .is_err());
                }
            }
        }
        // First n pages are full, next m pages need to be compacted, next n are full
        {
            let mut list = LinkedItemList::<SegmentMetaEntry>::create_with_fsm(&indexrel);
            let entries_1 = (1..500)
                .map(|_| {
                    SegmentMetaEntry::new_immutable(
                        random_segment_id(),
                        0,
                        not_deleted_xid,
                        SegmentMetaEntryImmutable {
                            postings: Some(make_fake_postings(&indexrel)),
                            ..Default::default()
                        },
                    )
                })
                .collect::<Vec<_>>();
            list.add_items(&entries_1, None);

            let entries_2 = (1..1000)
                .map(|_| {
                    SegmentMetaEntry::new_immutable(
                        random_segment_id(),
                        0,
                        deleted_xid,
                        SegmentMetaEntryImmutable {
                            postings: Some(make_fake_postings(&indexrel)),
                            ..Default::default()
                        },
                    )
                })
                .collect::<Vec<_>>();
            list.add_items(&entries_2, None);

            let entries_3 = (1..500)
                .map(|_| {
                    SegmentMetaEntry::new_immutable(
                        random_segment_id(),
                        0,
                        not_deleted_xid,
                        SegmentMetaEntryImmutable {
                            postings: Some(make_fake_postings(&indexrel)),
                            ..Default::default()
                        },
                    )
                })
                .collect::<Vec<_>>();

            list.add_items(&entries_3, None);

            let pre_gc_blocks = linked_list_block_numbers(&list);
            list.garbage_collect(pg_sys::FullTransactionId {
                value: deleted_xid.into_inner() as u64,
            });

            for entries in [entries_1, entries_2, entries_3] {
                for entry in entries {
                    if entry.xmax() == not_deleted_xid {
                        assert!(list
                            .lookup_ex(|el| el.segment_id() == entry.segment_id(), None)
                            .is_ok());
                    } else {
                        assert!(list
                            .lookup_ex(|el| el.segment_id() == entry.segment_id(), None)
                            .is_err());
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
        let mut list = LinkedItemList::<SegmentMetaEntry>::create_with_fsm(&indexrel);
        let entries = (1..2000)
            .map(|_| {
                SegmentMetaEntry::new_immutable(
                    random_segment_id(),
                    0,
                    pg_sys::InvalidTransactionId,
                    SegmentMetaEntryImmutable {
                        postings: Some(make_fake_postings(&indexrel)),
                        ..Default::default()
                    },
                )
            })
            .collect::<Vec<_>>();

        list.add_items(&entries, None);

        // Atomically modify the list, and then confirm that it contains unique blocks, and the
        // same contents.
        let list_block_numbers = linked_list_block_numbers(&list);
        let list_contents = list.list(None);
        let guard = list.atomically();
        let duplicate_block_numbers = linked_list_block_numbers(&guard);
        let duplicate_contents = guard.list(None);
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
        assert_eq!(list.list(None), duplicate_contents);
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
