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

use super::block::{
    BM25PageSpecialData, LinkedList, LinkedListData, MVCCEntry, PgItem, SCHEMA_START,
};
use super::buffer::{BufferManager, BufferMut, Page};
use super::utils::vacuum_get_freeze_limit;
use anyhow::Result;
use pgrx::pg_sys;
use pgrx::pg_sys::BlockNumber;
use rustc_hash::{FxHashMap, FxHashSet};
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
    pub header_blockno: pg_sys::BlockNumber,
    bman: BufferManager,
    _marker: std::marker::PhantomData<T>,
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> LinkedList for LinkedItemList<T> {
    fn get_header_blockno(&self) -> pg_sys::BlockNumber {
        self.header_blockno
    }

    fn block_for_ord(&self, ord: usize) -> Option<BlockNumber> {
        unimplemented!(
            "block_for_ord is not implemented for LinkedItemList.  caller requested ord: {ord}"
        )
    }

    unsafe fn get_linked_list_data(&self) -> LinkedListData {
        let header_buffer = self.bman.get_buffer(self.get_header_blockno());
        let page = header_buffer.page();
        page.contents::<LinkedListData>()
    }
}

impl<T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> LinkedItemList<T> {
    pub fn open(relation_oid: pg_sys::Oid, header_blockno: pg_sys::BlockNumber) -> Self {
        Self {
            header_blockno,
            bman: BufferManager::new(relation_oid),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn bman(&self) -> &BufferManager {
        &self.bman
    }

    pub fn bman_mut(&mut self) -> &mut BufferManager {
        &mut self.bman
    }

    pub unsafe fn create(relation_oid: pg_sys::Oid) -> Self {
        let mut bman = BufferManager::new(relation_oid);

        let mut header_buffer = bman.new_buffer();
        let header_blockno = header_buffer.number();
        let mut start_buffer = bman.new_buffer();
        let start_blockno = start_buffer.number();

        let mut header_page = header_buffer.init_page();
        start_buffer.init_page();

        // TODO: `LinkedItemsList` currently never modifies this metadata
        // after creation, but `LinkedBytesList` does. Do we actually need it?
        let metadata = header_page.contents_mut::<LinkedListData>();
        metadata.start_blockno = start_blockno;
        metadata.last_blockno = start_blockno;
        metadata.npages = 0;

        Self {
            header_blockno,
            bman,
            _marker: std::marker::PhantomData,
        }
    }

    /// Return a Vec of all the items in this linked list
    pub unsafe fn list(&self) -> Vec<T> {
        let mut items = vec![];
        let mut blockno = self.get_start_blockno();

        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = self.bman.get_buffer(blockno);
            let page = buffer.page();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();
            while offsetno <= max_offset {
                if let Some((deserialized, _)) = page.read_item::<T>(offsetno) {
                    items.push(deserialized);
                }
                offsetno += 1;
            }
            blockno = page.next_blockno();
        }

        items
    }

    pub unsafe fn is_empty(&self) -> bool {
        let mut blockno = self.get_start_blockno();

        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = self.bman.get_buffer(blockno);
            let page = buffer.page();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();
            while offsetno <= max_offset {
                if let Some((_, _)) = page.read_item::<T>(offsetno) {
                    return false;
                }
                offsetno += 1;
            }
            blockno = page.next_blockno();
        }

        true
    }

    pub unsafe fn garbage_collect(&mut self) -> Vec<T> {
        // concurrent changes to the items list itself is not possible and should interlock with
        // other backends that are reading/modifying the list
        let _schema_lock = self.bman_mut().get_buffer_mut(SCHEMA_START);

        // Delete all items that are definitely dead
        let heap_relation = self.bman().bm25cache().heaprel();
        let freeze_limit = vacuum_get_freeze_limit(heap_relation);
        let start_blockno = self.get_start_blockno();
        let mut blockno = start_blockno;
        let mut last_filled_blockno = start_blockno;

        let mut recycled_entries = Vec::new();

        while blockno != pg_sys::InvalidBlockNumber {
            let mut buffer = self.bman.get_buffer_mut(blockno);
            let mut page = buffer.page_mut();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();
            let mut delete_offsets = vec![];

            while offsetno <= max_offset {
                if let Some((entry, _)) = page.read_item::<T>(offsetno) {
                    if entry.recyclable(self.bman_mut()) {
                        page.mark_item_dead(offsetno);

                        recycled_entries.push(entry);
                        delete_offsets.push(offsetno);
                    } else {
                        let xmin_needs_freeze = entry.xmin_needs_freeze(freeze_limit);
                        let xmax_needs_freeze = entry.xmax_needs_freeze(freeze_limit);

                        if xmin_needs_freeze || xmax_needs_freeze {
                            let frozen_entry =
                                entry.into_frozen(xmin_needs_freeze, xmax_needs_freeze);
                            let PgItem(item, size) = frozen_entry.clone().into();
                            let did_replace = page.replace_item(offsetno, item, size);
                            assert!(did_replace);
                        }
                    }
                }
                offsetno += 1;
            }

            if !delete_offsets.is_empty() {
                page.delete_items(&mut delete_offsets);
            }

            let new_max_offset = page.max_offset_number();
            let current_blockno = blockno;
            blockno = page.next_blockno();

            // Compaction step: If the page is entirely empty, mark as deleted
            // Adjust the pointer from the last known non-empty node to point to the next non-empty node
            if new_max_offset == pg_sys::InvalidOffsetNumber && current_blockno != start_blockno {
                page.mark_deleted(pg_sys::GetCurrentTransactionId());
                // this page is no longer useful to us so go ahead and return it to the FSM.  Doing
                // so will also drop the lock
                self.bman.return_to_fsm_mut(buffer);

                // We've reached the end of the list, which means the last filled block is now the
                // last entry in the list
                if blockno == pg_sys::InvalidBlockNumber {
                    let mut last_filled_buffer = self.bman.get_buffer_mut(last_filled_blockno);
                    let mut last_filled_page = last_filled_buffer.page_mut();
                    let special = last_filled_page.special_mut::<BM25PageSpecialData>();
                    special.next_blockno = pg_sys::InvalidBlockNumber;
                }
            } else if new_max_offset != pg_sys::InvalidOffsetNumber
                && current_blockno != start_blockno
            {
                // drop the buffer were holding onto as we might acquire an exclusive lock on another
                // one below and we don't want concurrent backends that are otherwise racing through
                // this linked list to end up causing a lock inversion where they've locked the page
                // we want while we have this page locked
                drop(buffer);

                let mut last_filled_buffer = self.bman.get_buffer_mut(last_filled_blockno);
                let mut last_filled_page = last_filled_buffer.page_mut();
                if last_filled_page.next_blockno() != current_blockno {
                    let special = last_filled_page.special_mut::<BM25PageSpecialData>();
                    special.next_blockno = current_blockno;
                }
                last_filled_blockno = current_blockno;
            }
        }

        recycled_entries
    }

    pub unsafe fn add_items(&mut self, items: &[T], buffer: Option<BufferMut>) -> Result<()> {
        let need_hold = buffer.is_some();
        let mut buffer =
            buffer.unwrap_or_else(|| self.bman.get_buffer_mut(self.get_start_blockno()));

        // this will get set to the `buffer` argument above and remain open (ie, exclusive locked)
        // until we're done adding all the items, so long as the original `buffer` argument was
        // Some(BufferMut)
        let mut hold_open = None;

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
                    if need_hold && hold_open.is_none() {
                        hold_open = Some(buffer);
                    }
                    buffer = self.bman.get_buffer_mut(next_blockno);
                } else {
                    // need to create new block and link it to this one
                    let mut new_page = self.bman.new_buffer();
                    let new_blockno = new_page.number();
                    new_page.init_page();

                    let special = page.special_mut::<BM25PageSpecialData>();
                    special.next_blockno = new_blockno;

                    if need_hold && hold_open.is_none() {
                        hold_open = Some(buffer);
                    }
                    buffer = new_page;
                }
            }
        }

        Ok(())
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

    ///
    /// TODO: Rename to `lookup_ex_mut` to parallel `atomically`.
    ///
    pub unsafe fn lookup_ex<Cmp: Fn(&T) -> bool>(
        &self,
        cmp: Cmp,
    ) -> Result<(T, pg_sys::BlockNumber, pg_sys::OffsetNumber)> {
        let mut blockno = self.get_start_blockno();
        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = self.bman.get_buffer(blockno);
            let page = buffer.page();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();

            while offsetno <= max_offset {
                if let Some((deserialized, _)) = page.read_item::<T>(offsetno) {
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
    /// Allows for atomically mutating multiple entries.
    ///
    /// TODO: currently nothing prevents multiple concurrent `atomically` calls:
    /// external locking is necessary to ensure that only one `atomically` block
    /// is running at a time.
    ///
    /// The given Accessor allows for adding, removing, and mutating entries.
    /// Modified pages are copied, and are made visible atomically by replacing
    /// a single inbound pointer once the Accessor is committed.
    ///
    /// If the Accessor is dropped without commit, any copied pages are garbage
    /// collected instead.
    ///
    pub fn atomically<'s>(&'s mut self) -> LinkedItemListAccessor<'s, T> {
        LinkedItemListAccessor {
            list: self,
            writable_mapping: FxHashMap::default(),
            writable: FxHashSet::default(),
        }
    }
}

pub struct LinkedItemListAccessor<'s, T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> {
    list: &'s mut LinkedItemList<T>,
    // A mapping from blocks in the base list to writable copies of those blocks. The writable copies
    // only exist within this accessor: during `commit` we atomically make all of them visible, and
    // during `drop` we garbage collect them.
    writable_mapping: FxHashMap<pg_sys::BlockNumber, pg_sys::BlockNumber>,
    // Blocks which are writable.
    writable: FxHashSet<pg_sys::BlockNumber>,
}

// TODO: Find a way to share common methods with `LinkedItemList`.
impl<'s, T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> LinkedItemListAccessor<'s, T> {
    ///
    /// Identical to `lookup_ex`, except that it returns a page that may be written to atomically
    /// within this Accessor.
    ///
    pub unsafe fn lookup_ex_mut<Cmp: Fn(&T) -> bool>(
        &mut self,
        cmp: Cmp,
    ) -> Result<(T, pg_sys::BlockNumber, pg_sys::OffsetNumber)> {
        let mut blockno = self.list.get_start_blockno();
        let mut previous_page = None;
        while blockno != pg_sys::InvalidBlockNumber {
            blockno = self.maybe_writable(blockno);
            let buffer = self.list.bman.get_buffer_mut(blockno);
            let page = buffer.page();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();

            while offsetno <= max_offset {
                if let Some((deserialized, _)) = page.read_item::<T>(offsetno) {
                    if cmp(&deserialized) {
                        blockno = self.ensure_writable(previous_page, page, blockno);
                        return Ok((deserialized, blockno, offsetno));
                    }
                }
                offsetno += 1;
            }

            let next_blockno = page.next_blockno();
            previous_page = Some((blockno, buffer));
            blockno = next_blockno;
        }

        // if we get here, we didn't find what we were looking for
        // but we should have -- how else could we have asked for something from the list?
        Err(anyhow::anyhow!(format!(
            "transaction {} failed to find the desired entry",
            pg_sys::GetCurrentTransactionIdIfAny()
        )))
    }

    pub fn commit(mut self) {
        if self.writable.is_empty() {
            // Nothing to do.
            return;
        }

        // Find the last clean page in the existing list, and adjust its next_blockno pointer
        // to point into the writable pages, making them visible to other readers.
        let mut blockno = self.list.get_start_blockno();
        let mut previous_buffer = None;
        while blockno != pg_sys::InvalidBlockNumber {
            let Some(writable_blockno) = self.writable_mapping.get(&blockno).copied() else {
                // Open this block to get the next pointer, and keep it open until we've
                // determined that we don't need to modify it.
                let buffer = self.list.bman.get_buffer_mut(blockno);
                blockno = buffer.page().next_blockno();
                previous_buffer = Some(buffer);
                continue;
            };

            // We've found the first writable block.
            if let Some(mut previous_buffer) = previous_buffer {
                // If we have a previous buffer, then modify it to point to the first
                // writable block.
                previous_buffer
                    .page_mut()
                    .special_mut::<BM25PageSpecialData>()
                    .next_blockno = writable_blockno;
                break;
            } else {
                // If we don't have a previous buffer, then the first block of the list
                // was writable, and so we need to go and modify our sentinel.
                todo!();
            }
        }

        // Clean up any pages that we unlinked.
        // TODO: Don't leak these pages.

        self.writable_mapping.clear();
        self.writable.clear();
    }

    ///
    /// Get a writable version of the given blockno, if one already exists. Otherwise, the readonly
    /// version.
    ///
    fn maybe_writable(&self, blockno: pg_sys::BlockNumber) -> pg_sys::BlockNumber {
        self.writable_mapping
            .get(&blockno)
            .copied()
            .unwrap_or(blockno)
    }

    ///
    /// If the given blockno is not already writable, create a writable copy (in the context of
    /// this Accessor), and return the writable blockno.
    ///
    fn ensure_writable(
        &mut self,
        previous_page: Option<(pg_sys::BlockNumber, BufferMut)>,
        page: Page,
        blockno: pg_sys::BlockNumber,
    ) -> pg_sys::BlockNumber {
        if self.writable.contains(&blockno) {
            // Block was already writable.
            return blockno;
        }

        // Copy the page, and add it to `writable_mapping`.
        let mut new_buffer = self.list.bman.new_buffer();
        let new_blockno = new_buffer.number();
        new_buffer.init_page();
        let mut new_page = new_buffer.page_mut();

        // TODO: It looks like this copy should use something similar to
        // PageGetTempPageCopySpecial: essentially memcopy'ing the page.
        let mut offsetno = pg_sys::FirstOffsetNumber;
        let max_offset = page.max_offset_number();
        while offsetno <= max_offset {
            if let Some((deserialized, _)) = page.read_item::<T>(offsetno) {
                let PgItem(item, size) = deserialized.into();
                let new_offsetno = new_page.append_item(item, size, 0);
                assert!(new_offsetno != pg_sys::InvalidOffsetNumber);
            }
            offsetno += 1;
        }

        new_page.special_mut::<BM25PageSpecialData>().next_blockno =
            self.maybe_writable(page.next_blockno());

        // If the previous page was already writable, then we eagerly update it to point
        // to this new writable page.
        if let Some((previous_blockno, mut previous_buffer)) = previous_page {
            if self.writable.contains(&previous_blockno) {
                previous_buffer
                    .page_mut()
                    .special_mut::<BM25PageSpecialData>()
                    .next_blockno = new_blockno;
            }
        }

        self.writable_mapping.insert(blockno, new_blockno);
        self.writable.insert(new_blockno);
        new_blockno
    }
}

impl<'s, T: From<PgItem> + Into<PgItem> + Debug + Clone + MVCCEntry> Drop
    for LinkedItemListAccessor<'s, T>
{
    fn drop(&mut self) {
        if !self.writable.is_empty() {
            // TODO: Don't leak these pages.
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;
    use std::collections::HashSet;
    use tantivy::index::SegmentId;
    use uuid::Uuid;

    use crate::postgres::storage::block::{FileEntry, SegmentMetaEntry};

    fn random_segment_id() -> SegmentId {
        SegmentId::from_uuid_string(&Uuid::new_v4().to_string()).unwrap()
    }

    fn linked_list_block_numbers(
        list: &LinkedItemList<SegmentMetaEntry>,
    ) -> HashSet<pg_sys::BlockNumber> {
        let mut blockno = list.get_start_blockno();
        let mut block_numbers = HashSet::new();

        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = list.bman().get_buffer(blockno);
            let page = buffer.page();
            blockno = page.next_blockno();
            block_numbers.insert(blockno);
        }

        block_numbers
    }

    #[pg_test]
    unsafe fn test_linked_items_garbage_collect_single_page() {
        let relation_oid = init_bm25_index();

        let snapshot = pg_sys::GetActiveSnapshot();
        let delete_xid = (*snapshot).xmin - 1;

        let mut list = LinkedItemList::<SegmentMetaEntry>::create(relation_oid);
        let entries_to_delete = vec![SegmentMetaEntry {
            segment_id: random_segment_id(),
            xmin: delete_xid,
            xmax: delete_xid,
            postings: Some(make_fake_postings(relation_oid)),
            ..Default::default()
        }];
        let entries_to_keep = vec![SegmentMetaEntry {
            segment_id: random_segment_id(),
            xmin: (*snapshot).xmin - 1,
            xmax: pg_sys::InvalidTransactionId,
            postings: Some(make_fake_postings(relation_oid)),
            ..Default::default()
        }];

        list.add_items(&entries_to_delete, None).unwrap();
        list.add_items(&entries_to_keep, None).unwrap();
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

        let snapshot = pg_sys::GetActiveSnapshot();
        let deleted_xid = (*snapshot).xmin - 1;
        let not_deleted_xid = pg_sys::InvalidTransactionId;
        let xmin = (*snapshot).xmin - 1;

        // Add 2000 entries, delete every 10th entry
        {
            let mut list = LinkedItemList::<SegmentMetaEntry>::create(relation_oid);
            let entries = (1..2000)
                .map(|i| SegmentMetaEntry {
                    segment_id: random_segment_id(),
                    xmin,
                    xmax: if i % 10 == 0 {
                        deleted_xid
                    } else {
                        not_deleted_xid
                    },
                    postings: Some(make_fake_postings(relation_oid)),
                    ..Default::default()
                })
                .collect::<Vec<_>>();

            list.add_items(&entries, None).unwrap();
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
            let mut list = LinkedItemList::<SegmentMetaEntry>::create(relation_oid);
            let entries_1 = (1..500)
                .map(|_| SegmentMetaEntry {
                    segment_id: random_segment_id(),
                    xmin,
                    xmax: not_deleted_xid,
                    postings: Some(make_fake_postings(relation_oid)),
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            list.add_items(&entries_1, None).unwrap();

            let entries_2 = (1..1000)
                .map(|_| SegmentMetaEntry {
                    segment_id: random_segment_id(),
                    xmin,
                    xmax: deleted_xid,
                    postings: Some(make_fake_postings(relation_oid)),
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            list.add_items(&entries_2, None).unwrap();

            let entries_3 = (1..500)
                .map(|_| SegmentMetaEntry {
                    segment_id: random_segment_id(),
                    xmin,
                    xmax: not_deleted_xid,
                    postings: Some(make_fake_postings(relation_oid)),
                    ..Default::default()
                })
                .collect::<Vec<_>>();

            list.add_items(&entries_3, None).unwrap();

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

            // Assert that compaction produced a smaller list with no new blocks
            let post_gc_blocks = linked_list_block_numbers(&list);
            assert!(pre_gc_blocks.len() > post_gc_blocks.len());
            assert!(post_gc_blocks.is_subset(&pre_gc_blocks));
        }
    }

    #[pg_test]
    unsafe fn test_linked_items_modify_atomically() {
        let relation_oid = init_bm25_index();

        let snapshot = pg_sys::GetActiveSnapshot();

        // Add 2000 entries.
        let mut list = LinkedItemList::<SegmentMetaEntry>::create(relation_oid);
        let entries = (1..2000)
            .map(|_| SegmentMetaEntry {
                segment_id: random_segment_id(),
                xmin: (*snapshot).xmin - 1,
                xmax: pg_sys::InvalidTransactionId,
                postings: Some(make_fake_postings(relation_oid)),
                ..Default::default()
            })
            .collect::<Vec<_>>();

        list.add_items(&entries, None).unwrap();

        // Collect the segment ids and positions of every entry.
        let get_positions = |list: &LinkedItemList<SegmentMetaEntry>| {
            entries
                .iter()
                .map(|entry| {
                    let (_, blockno, offsetno) = list
                        .lookup_ex(|el| el.segment_id == entry.segment_id)
                        .unwrap();
                    (entry.segment_id, blockno, offsetno)
                })
                .collect::<Vec<_>>()
        };
        let positions = get_positions(&list);

        // Grab one entry from each block, and validate that we have more four or more blocks.
        let unique_block_entries = {
            let mut dedupe = HashSet::new();
            positions
                .iter()
                .filter_map(|(segment_id, blockno, _)| {
                    if dedupe.insert(blockno) {
                        Some((segment_id, blockno))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        assert!(
            unique_block_entries.len() >= 4,
            "Got blocks: {unique_block_entries:?}"
        );

        // Atomically modify blocks 1 and 2.
        {
            let mut accessor = list.atomically();
            accessor
                .lookup_ex_mut(|el| el.segment_id == *unique_block_entries[1].0)
                .unwrap();
            accessor
                .lookup_ex_mut(|el| el.segment_id == *unique_block_entries[2].0)
                .unwrap();
            accessor.commit();
        }

        // Confirm that blocks 0 and 3 have not changed, but that blocks 1 and 2 have.
        for unchanged_block in [0, 3] {
            assert_eq!(
                list.lookup_ex(|el| el.segment_id == *unique_block_entries[unchanged_block].0)
                    .unwrap()
                    .1,
                *unique_block_entries[unchanged_block].1,
            );
        }
        for changed_block in [1, 2] {
            assert_ne!(
                list.lookup_ex(|el| el.segment_id == *unique_block_entries[changed_block].0)
                    .unwrap()
                    .1,
                *unique_block_entries[changed_block].1,
            );
        }

        // Confirm that all entries are still present, in order.
        let segments_before = positions.into_iter().map(|x| x.0).collect::<Vec<_>>();
        let segments_after = get_positions(&list)
            .into_iter()
            .map(|x| x.0)
            .collect::<Vec<_>>();
        assert_eq!(segments_before, segments_after);
    }

    fn init_bm25_index() -> pg_sys::Oid {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        return Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
            .expect("spi should succeed")
            .unwrap();
    }

    fn make_fake_postings(relation_oid: pg_sys::Oid) -> FileEntry {
        let mut postings_file_block = BufferManager::new(relation_oid).new_buffer();
        postings_file_block.init_page();
        FileEntry {
            starting_block: postings_file_block.number(),
            total_bytes: 0,
        }
    }
}
