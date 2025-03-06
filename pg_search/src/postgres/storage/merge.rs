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

use crate::postgres::storage::block::{bm25_max_free_space, BM25PageSpecialData, MERGE_LOCK};
use crate::postgres::storage::buffer::{BufferManager, BufferMut, PinnedBuffer};
use pgrx::pg_sys;
use std::collections::HashSet;
use std::mem::ManuallyDrop;
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
    ambulkdelete_sentinel: pg_sys::BlockNumber,
}

struct VacuumSentinel(PinnedBuffer);

/// Only one merge can happen at a time, so we need to lock the merge process
#[derive(Debug)]
pub struct MergeLock {
    bman: BufferManager,
    buffer: BufferMut,
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

    /// This lock is acquired by inserts that attempt to merge segments
    /// Merges should only happen if there is no other merge in progress
    /// AND the effects of the previous merge are visible
    pub unsafe fn acquire_for_merge(relation_oid: pg_sys::Oid) -> Option<Self> {
        if !crate::postgres::utils::IsTransactionState() {
            return None;
        }

        let mut bman = BufferManager::new(relation_oid);
        let merge_lock = bman.get_buffer_conditional(MERGE_LOCK)?;
        let page = merge_lock.page();
        let metadata = page.contents::<MergeLockData>();
        let last_merge = metadata.last_merge;

        // in order to return the MergeLock we need to make sure we can see the effects of the
        // last merge that ran.
        //
        // We already know we're the only backend with the Buffer-level lock, because the
        // `.get_buffer_conditional()` call above gave us the Buffer, so now we need to ensure
        // we're allowed to touch the segments that may have been modified by the last merge
        let last_merge_visible =
            // the last_merge value is zero b/c we've never done a merge
            last_merge == pg_sys::InvalidTransactionId

                // or it is from this transaction
                || pg_sys::TransactionIdIsCurrentTransactionId(last_merge)

                // or the last_merge transaction's effects are known to be visible by all
                // current/future transactions
                || {
                #[cfg(feature = "pg13")]
                {
                    let oldest_xmin = pg_sys::TransactionIdLimitedForOldSnapshots(
                        pg_sys::GetOldestXmin(bman.bm25cache().heaprel(), pg_sys::PROCARRAY_FLAGS_VACUUM as i32), bman.bm25cache().heaprel(),
                    );
                    pg_sys::TransactionIdPrecedes(last_merge, oldest_xmin)
                }
                #[cfg(any(
                    feature = "pg14",
                    feature = "pg15",
                    feature = "pg16",
                    feature = "pg17"
                ))]
                {
                    let oldest_xmin = pg_sys::GetOldestNonRemovableTransactionId(bman.bm25cache().heaprel());
                    pg_sys::TransactionIdPrecedes(last_merge, oldest_xmin)
                }
            };

        if last_merge_visible {
            Some(MergeLock {
                bman,
                buffer: merge_lock,
                save_xid: true,
            })
        } else {
            None
        }
    }

    /// This is a blocking operation to acquire the MERGE_LOCK.  
    ///
    /// It should only be called from [`ambulkdelete`] and will block
    /// until concurrent merges in other backends complete.
    pub unsafe fn acquire_for_ambulkdelete(relation_oid: pg_sys::Oid) -> Self {
        let mut bman = BufferManager::new(relation_oid);
        let merge_lock = bman.get_buffer_mut(MERGE_LOCK);
        MergeLock {
            bman,
            buffer: merge_lock,
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
        VacuumList::open(
            move || merge_lock.pin_ambulkdelete_sentinel(),
            relation_oid,
            start_page,
        )
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

        VacuumList::open(
            || panic!("cannot acquire VacuumList sentinel in this context"),
            self.bman.relation_oid(),
            metadata.active_vacuum_list,
        )
        .list()
    }

    fn pin_ambulkdelete_sentinel(mut self) -> VacuumSentinel {
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
}

impl Drop for MergeLock {
    fn drop(&mut self) {
        unsafe {
            if self.save_xid {
                if crate::postgres::utils::IsTransactionState() {
                    let mut current_xid = pg_sys::GetCurrentTransactionIdIfAny();

                    // if we don't have a transaction id (typically from a parallel vacuum)...
                    if current_xid == pg_sys::InvalidTransactionId {
                        // ... then use the next transaction id as ours
                        #[cfg(feature = "pg13")]
                        {
                            current_xid = pg_sys::ReadNewTransactionId()
                        }

                        #[cfg(not(feature = "pg13"))]
                        {
                            current_xid = pg_sys::ReadNextTransactionId()
                        }
                    }

                    let mut page = self.buffer.page_mut();
                    let metadata = page.contents_mut::<MergeLockData>();
                    metadata.last_merge = current_xid;
                }
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
    bman: BufferManager,
    start_block_number: pg_sys::BlockNumber,
    sentinel: Box<dyn FnOnce() -> VacuumSentinel>,
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
        sentinel: impl FnOnce() -> VacuumSentinel + 'static,
        relation_oid: pg_sys::Oid,
        start_block_number: pg_sys::BlockNumber,
    ) -> VacuumList {
        Self {
            bman: BufferManager::new(relation_oid),
            start_block_number,
            sentinel: Box::new(sentinel),
        }
    }

    pub fn write_active_list<'s>(
        mut self,
        segment_ids: impl Iterator<Item = &'s SegmentId>,
    ) -> impl Drop + 'static {
        let mut segment_ids = segment_ids.collect::<Vec<_>>();
        segment_ids.sort();

        let mut buffer = self.bman.get_buffer_mut(self.start_block_number);
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
                    buffer = self.bman.get_buffer_mut(page.next_blockno());
                    page = buffer.page_mut();
                } else {
                    // make a new next block and link it in
                    let next_buffer = self.bman.new_buffer();
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

        (self.sentinel)().0
    }

    pub fn list(&self) -> HashSet<SegmentId> {
        let mut segment_ids = HashSet::new();
        let mut buffer = self.bman.get_buffer(self.start_block_number);
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
                    .into_iter()
                    .map(|bytes| SegmentId::from_bytes(*bytes)),
            );

            if page.next_blockno() == pg_sys::InvalidBlockNumber {
                // we don't have another block
                break;
            }

            buffer = self.bman.get_buffer(page.next_blockno());
        }

        segment_ids
    }
}
