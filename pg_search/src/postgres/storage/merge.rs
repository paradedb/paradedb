// Copyright (c) 2023-2025 Retake, Inc.
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

use crate::postgres::storage::block::{
    bm25_max_free_space, BM25PageSpecialData, LinkedList, MVCCEntry, PgItem, SegmentFileDetails,
    MERGE_LOCK,
};
use crate::postgres::storage::buffer::{BufferManager, BufferMut, PinnedBuffer};
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use pgrx::pg_sys;
use pgrx::pg_sys::TransactionId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ops::Deref;
use std::slice::from_raw_parts;
use tantivy::index::SegmentId;

/// The metadata stored on the [`MergeLock`] page
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct MergeLockData {
    pub last_merge: pg_sys::TransactionId,

    /// This space was once used but no longer is.  As such, it needs to remain dead forever
    #[allow(dead_code)]
    pub _dead_space: u32,

    /// Contains the [`pg_sys::BlockNumber`] of the active merge list
    active_vacuum_list: pg_sys::BlockNumber,

    /// A block for which is pin is held during `ambulkdelete()`
    pub ambulkdelete_sentinel: pg_sys::BlockNumber,

    /// The starting block for a [`LinkedItemsList<MergeEntry>]`
    pub merge_list: pg_sys::BlockNumber,
}

#[repr(transparent)]
pub struct VacuumSentinel(PinnedBuffer);

/// Only one merge can happen at a time, so we need to lock the merge process
#[derive(Debug)]
pub struct MergeLock {
    // NB:  Rust's struct drop order is how the fields are defined in the source code
    // and while it _probably_ doesn't matter, we'd prefer to have `buffer`'s drop impl
    // run before the `bman` from which it originated
    buffer: BufferMut,
    bman: BufferManager,
    save_xid: bool,
}

impl MergeLock {
    pub unsafe fn is_merging(relation_oid: pg_sys::Oid) -> bool {
        if !crate::postgres::utils::IsTransactionState() {
            return false;
        }

        // a merge is happening if we're unable to obtain the MERGE_LOCK
        // means some other backend has it
        let mut bman = BufferManager::new(relation_oid);
        bman.get_buffer_conditional(MERGE_LOCK).is_none()
    }

    /// This is a blocking operation to acquire the MERGE_LOCK.  
    pub unsafe fn acquire(relation_oid: pg_sys::Oid) -> Self {
        let mut bman = BufferManager::new(relation_oid);
        let merge_lock = bman.get_buffer_mut(MERGE_LOCK);
        MergeLock {
            buffer: merge_lock,
            bman,
            save_xid: false,
        }
    }

    pub unsafe fn init(relation_id: pg_sys::Oid) {
        let mut bman = BufferManager::new(relation_id);
        let mut merge_lock = bman.get_buffer_mut(MERGE_LOCK);
        let mut page = merge_lock.page_mut();
        let metadata = page.contents_mut::<MergeLockData>();

        metadata.last_merge = pg_sys::InvalidTransactionId;
        metadata.active_vacuum_list = pg_sys::InvalidBlockNumber;
        metadata.ambulkdelete_sentinel = pg_sys::InvalidBlockNumber;
    }

    pub fn metadata(&self) -> MergeLockData {
        let page = self.buffer.page();
        page.contents::<MergeLockData>()
    }

    pub fn vacuum_list(mut self) -> VacuumList {
        let mut page = self.buffer.page_mut();
        let metadata = page.contents_mut::<MergeLockData>();

        // if the `active_vacuum_list` block number appears to be uninitialized, which in our
        // case will be zero if this is from an index that existed prior to adding the `active_vacuum_list`
        // field, or pg_sys::InvalidBlockNumber if the index was created after adding the
        // `active_vacuum_list` field.
        let relation_oid = self.bman.relation_oid();
        if metadata.active_vacuum_list == 0
            || metadata.active_vacuum_list == pg_sys::InvalidBlockNumber
        {
            // create a new VacuumList for this index and assign its starting block number
            metadata.active_vacuum_list = VacuumList::create(relation_oid);
        }

        // open the VacuumList
        let start_page = metadata.active_vacuum_list;
        let merge_lock = self;
        VacuumList::open(Some(merge_lock), relation_oid, start_page)
    }

    pub fn list_vacuuming_segments(&mut self) -> HashSet<SegmentId> {
        if !self.is_ambulkdelete_running() {
            // there's no ambulkdelete running, so the contents of the VacuumList are immaterial to us
            return Default::default();
        }

        let page = self.buffer.page();
        let metadata = page.contents::<MergeLockData>();
        if metadata.active_vacuum_list == 0
            || metadata.active_vacuum_list == pg_sys::InvalidBlockNumber
        {
            // the VacuumList has never been initialized
            return Default::default();
        }

        VacuumList::open(None, self.bman.relation_oid(), metadata.active_vacuum_list).read_list()
    }

    fn pin_ambulkdelete_sentinel(&mut self) -> VacuumSentinel {
        let mut page = self.buffer.page_mut();
        let metadata = page.contents_mut::<MergeLockData>();
        if metadata.ambulkdelete_sentinel == 0
            || metadata.ambulkdelete_sentinel == pg_sys::InvalidBlockNumber
        {
            // initialize the sentinel page, if necessary
            let mut sentinal_buffer = self.bman.new_buffer();
            sentinal_buffer.init_page();
            metadata.ambulkdelete_sentinel = sentinal_buffer.number();
        }

        let sentinel = self.bman.pinned_buffer(metadata.ambulkdelete_sentinel);
        VacuumSentinel(sentinel)
    }

    pub fn is_ambulkdelete_running(&mut self) -> bool {
        let page = self.buffer.page();
        let metadata = page.contents::<MergeLockData>();
        if metadata.ambulkdelete_sentinel == 0
            || metadata.ambulkdelete_sentinel == pg_sys::InvalidBlockNumber
        {
            // sentinel page was never initialized
            return false;
        }

        // an `ambulkdelete()` is running if we can't acquire the sentinel block for cleanup
        // it means ambulkdelete() is holding a pin on that buffer
        self.bman
            .get_buffer_for_cleanup_conditional(metadata.ambulkdelete_sentinel)
            .is_none()
    }

    pub unsafe fn in_progress_segment_ids(&self) -> impl Iterator<Item = SegmentId> {
        let metadata = self.metadata();
        if metadata.merge_list == 0 || metadata.merge_list == pg_sys::InvalidBlockNumber {
            // our merge_list has never been initialized
            let iter: Box<dyn Iterator<Item = SegmentId>> = Box::new(std::iter::empty());
            return iter;
        }

        let relation_id = (*self.bman.bm25cache().indexrel()).rd_id;
        let entries = LinkedItemList::<MergeEntry>::open(relation_id, metadata.merge_list);
        Box::new(
            entries
                .list()
                .into_iter()
                .map(move |merge_entry| merge_entry.segment_ids(relation_id).into_iter())
                .flatten(),
        )
    }

    pub unsafe fn in_progress_merge_entries(
        &self,
    ) -> impl Iterator<Item = (MergeEntry, SegmentId)> {
        let metadata = self.metadata();
        if metadata.merge_list == 0 || metadata.merge_list == pg_sys::InvalidBlockNumber {
            // our merge_list has never been initialized
            let iter: Box<dyn Iterator<Item = (MergeEntry, SegmentId)>> =
                Box::new(std::iter::empty());
            return iter;
        }
        let relation_id = (*self.bman.bm25cache().indexrel()).rd_id;
        let entries = LinkedItemList::<MergeEntry>::open(relation_id, metadata.merge_list);
        Box::new(
            entries
                .list()
                .into_iter()
                .map(move |merge_entry| {
                    merge_entry
                        .segment_ids(relation_id)
                        .into_iter()
                        .map(move |segment_id| (merge_entry, segment_id))
                })
                .flatten(),
        )
    }

    pub unsafe fn is_merge_in_progress(&self) -> bool {
        let metadata = self.metadata();
        if metadata.merge_list == 0 || metadata.merge_list == pg_sys::InvalidBlockNumber {
            return false;
        }
        let relation_id = (*self.bman.bm25cache().indexrel()).rd_id;
        !LinkedItemList::<MergeEntry>::open(relation_id, metadata.merge_list).is_empty()
    }

    pub unsafe fn record_in_progress_segment_ids<'a>(
        &mut self,
        segment_ids: impl IntoIterator<Item = &'a &'a SegmentId>,
    ) -> anyhow::Result<pg_sys::TransactionId> {
        assert!(pg_sys::IsTransactionState());

        let relation_id = (*self.bman.bm25cache().indexrel()).rd_id;
        let merge_list_blockno = {
            let mut page = self.buffer.page_mut();
            let metadata = page.contents_mut::<MergeLockData>();

            if metadata.merge_list == 0 || metadata.merge_list == pg_sys::InvalidBlockNumber {
                let merge_list = LinkedItemList::<MergeEntry>::create(relation_id);
                metadata.merge_list = merge_list.get_header_blockno();
            }

            metadata.merge_list
        };

        // write the SegmentIds to disk
        let mut segment_ids_list = LinkedBytesList::create(relation_id);
        for segment_id in segment_ids {
            segment_ids_list.write(segment_id.uuid_bytes())?;
        }

        // fabricate and write the [`MergeEntry`] itself
        let xid = pg_sys::GetCurrentTransactionId();
        let merge_entry = MergeEntry {
            pid: pg_sys::MyProcPid,
            xmin: xid, // the entry is transient
            xmax: xid, // so it will be considered deleted by this transaction
            entries_block: segment_ids_list.get_header_blockno(),
        };

        let mut entries_list = LinkedItemList::<MergeEntry>::open(relation_id, merge_list_blockno);
        entries_list.add_items(&[merge_entry], None)?;
        Ok(xid)
    }

    pub unsafe fn remove_entry(
        &mut self,
        xmax: pg_sys::TransactionId,
    ) -> anyhow::Result<MergeEntry> {
        let page = self.buffer.page();
        let metadata = page.contents::<MergeLockData>();
        if metadata.merge_list == 0 || metadata.merge_list == pg_sys::InvalidBlockNumber {
            panic!("merge_list should have been initialized by now");
        }

        let relation_id = (*self.bman.bm25cache().indexrel()).rd_id;
        let mut entries_list = LinkedItemList::<MergeEntry>::open(relation_id, metadata.merge_list);
        let removed_entry = entries_list.remove_item(|merge_entry| merge_entry.xmax == xmax)?;

        LinkedBytesList::open(relation_id, removed_entry.entries_block)
            .return_to_fsm_unchecked(&removed_entry, None);
        Ok(removed_entry)
    }

    pub unsafe fn garbage_collect(&mut self) {
        let page = self.buffer.page();
        let metadata = page.contents::<MergeLockData>();
        if metadata.merge_list == 0 || metadata.merge_list == pg_sys::InvalidBlockNumber {
            return;
        }

        let relation_id = (*self.bman.bm25cache().indexrel()).rd_id;
        let mut entries_list = LinkedItemList::<MergeEntry>::open(relation_id, metadata.merge_list);
        let recycled_entries = entries_list.garbage_collect();
        for recycled_entry in recycled_entries {
            let mut bytes_list = LinkedBytesList::open(relation_id, recycled_entry.entries_block);
            bytes_list.mark_deleted(recycled_entry.xmax);
            bytes_list.return_to_fsm(&recycled_entry, None);
        }
    }
}

impl Drop for MergeLock {
    fn drop(&mut self) {
        unsafe {
            if self.save_xid && crate::postgres::utils::IsTransactionState() {
                let mut current_xid = pg_sys::GetCurrentTransactionIdIfAny();

                // if we don't have a transaction id (typically from a parallel vacuum)...
                if current_xid == pg_sys::InvalidTransactionId {
                    // ... then use the next transaction id as ours
                    current_xid = pg_sys::ReadNextTransactionId()
                }

                let mut page = self.buffer.page_mut();
                let metadata = page.contents_mut::<MergeLockData>();
                metadata.last_merge = current_xid;
            }
        }
    }
}

type SegmentIdBytes = [u8; 16];
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct VacuumListData {
    segment_ids:
        [SegmentIdBytes; (bm25_max_free_space() - size_of::<u16>()) / size_of::<SegmentIdBytes>()],
    nentries: u16,
}

pub struct VacuumList {
    relation_oid: pg_sys::Oid,
    start_block_number: pg_sys::BlockNumber,
    merge_lock: Option<MergeLock>,
}

impl VacuumList {
    fn create(relation_oid: pg_sys::Oid) -> pg_sys::BlockNumber {
        let mut bman = BufferManager::new(relation_oid);
        let mut start_buffer = bman.new_buffer();
        let mut start_page = start_buffer.init_page();

        let special = start_page.special_mut::<BM25PageSpecialData>();
        special.next_blockno = pg_sys::InvalidBlockNumber;

        start_buffer.number()
    }

    fn open(
        merge_lock: Option<MergeLock>,
        relation_oid: pg_sys::Oid,
        start_block_number: pg_sys::BlockNumber,
    ) -> VacuumList {
        Self {
            relation_oid,
            start_block_number,
            merge_lock,
        }
    }

    pub fn write_list<'s>(
        mut self,
        segment_ids: impl Iterator<Item = &'s SegmentId>,
    ) -> VacuumSentinel {
        let mut segment_ids = segment_ids.collect::<Vec<_>>();
        segment_ids.sort();

        let mut bman = BufferManager::new(self.relation_oid);
        let mut buffer = bman.get_buffer_mut(self.start_block_number);
        let mut page = buffer.page_mut();
        let mut contents = page.contents_mut::<VacuumListData>();
        contents.nentries = 0;

        for segment_id in segment_ids {
            let segment_id = segment_id.uuid_bytes();
            if contents.nentries as usize >= contents.segment_ids.len() {
                // switch to the next page, either using the one that's already linked
                // or by creating a new page
                if page.next_blockno() != pg_sys::InvalidBlockNumber {
                    // we want to reuse the next block if we have one
                    buffer = bman.get_buffer_mut(page.next_blockno());
                    page = buffer.page_mut();
                } else {
                    // make a new next block and link it in
                    let next_buffer = bman.new_buffer();
                    let special = page.special_mut::<BM25PageSpecialData>();
                    special.next_blockno = next_buffer.number();

                    // switch to it
                    buffer = next_buffer;
                    page = buffer.init_page();
                }

                contents = page.contents_mut::<VacuumListData>();
                contents.nentries = 0;
            }

            contents.segment_ids[contents.nentries as usize].copy_from_slice(segment_id);
            contents.nentries += 1;
        }

        let vacuum_sentinel = self
            .merge_lock
            .as_mut()
            .expect("VacuumList should own the MergeLock in this context")
            .pin_ambulkdelete_sentinel();

        // yes, I know, but this makes it clear that our intention is to obtain the vacuum_sentinel
        // before we (and our contained MergeLock) are dropped
        drop(self);
        vacuum_sentinel
    }

    pub fn read_list(&self) -> HashSet<SegmentId> {
        let mut segment_ids = HashSet::new();

        let bman = BufferManager::new(self.relation_oid);
        let mut buffer = bman.get_buffer(self.start_block_number);
        loop {
            let page = buffer.page();
            let contents = page.contents::<VacuumListData>();
            if contents.nentries == 0 {
                // no entries on this page
                break;
            }

            // add all the entries from this page
            segment_ids.extend(
                contents.segment_ids[..contents.nentries as usize]
                    .iter()
                    .map(|bytes| SegmentId::from_bytes(*bytes)),
            );

            if page.next_blockno() == pg_sys::InvalidBlockNumber {
                // we don't have another block
                break;
            }

            buffer = bman.get_buffer(page.next_blockno());
        }

        segment_ids
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct MergeEntry {
    pub pid: i32,
    pub xmin: pg_sys::TransactionId,
    pub xmax: pg_sys::TransactionId,
    pub entries_block: pg_sys::BlockNumber,
}

impl From<PgItem> for MergeEntry {
    fn from(value: PgItem) -> Self {
        let PgItem(item, size) = value;
        let decoded: MergeEntry = unsafe {
            bincode::deserialize(from_raw_parts(item as *const u8, size))
                .expect("expected to deserialize valid MergeEntry")
        };
        decoded
    }
}

impl From<MergeEntry> for PgItem {
    fn from(value: MergeEntry) -> Self {
        let bytes: Vec<u8> =
            bincode::serialize(&value).expect("expected to serialize valid MergeEntry");
        let pg_bytes = unsafe { pg_sys::palloc(bytes.len()) as *mut u8 };
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pg_bytes, bytes.len());
        }
        PgItem(pg_bytes as pg_sys::Item, bytes.len() as pg_sys::Size)
    }
}

impl MVCCEntry for MergeEntry {
    fn get_xmin(&self) -> TransactionId {
        self.xmin
    }

    fn get_xmax(&self) -> TransactionId {
        self.xmax
    }

    fn into_frozen(self, should_freeze_xmin: bool, should_freeze_xmax: bool) -> Self {
        Self {
            xmin: if should_freeze_xmin {
                pg_sys::FrozenTransactionId
            } else {
                self.xmin
            },
            xmax: if should_freeze_xmax {
                pg_sys::FrozenTransactionId
            } else {
                self.xmax
            },
            ..self
        }
    }
}

impl MergeEntry {
    pub unsafe fn segment_ids(&self, relation_id: pg_sys::Oid) -> Vec<SegmentId> {
        let bytes = LinkedBytesList::open(relation_id, self.entries_block);
        let bytes = bytes.read_all();
        bytes
            .chunks(size_of::<SegmentIdBytes>())
            .map(|entry| {
                SegmentId::from_bytes(entry.try_into().expect("malformed SegmentId entry"))
            })
            .collect()
    }
}
