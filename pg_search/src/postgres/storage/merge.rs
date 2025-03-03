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

use crate::postgres::storage::block::MERGE_LOCK;
use crate::postgres::storage::buffer::{BufferManager, BufferMut};
use pgrx::pg_sys;

/// The metadata stored on the [`MergeLock`] page
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct MergeLockData {
    pub last_merge: pg_sys::TransactionId,

    #[allow(dead_code)]
    pub _dead_space: u32,
}

/// Only one merge can happen at a time, so we need to lock the merge process
#[derive(Debug)]
pub struct MergeLock(BufferMut);

impl MergeLock {
    // This lock is acquired by inserts that attempt to merge segments
    // Merges should only happen if there is no other merge in progress
    // AND the effects of the previous merge are visible
    pub unsafe fn acquire_for_merge(relation_oid: pg_sys::Oid) -> Option<Self> {
        if !crate::postgres::utils::IsTransactionState() {
            return None;
        }

        let mut bman = BufferManager::new(relation_oid);
        let mut merge_lock = bman.get_buffer_conditional(MERGE_LOCK)?;
        let mut page = merge_lock.page_mut();
        let metadata = page.contents_mut::<MergeLockData>();
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
            metadata.last_merge = pg_sys::GetCurrentTransactionId();
            Some(MergeLock(merge_lock))
        } else {
            None
        }
    }

    // This lock must be acquired before ambulkdelete calls commit() on the index
    // We ask for an exclusive lock because ambulkdelete must delete all dead ctids
    pub unsafe fn acquire_for_delete(relation_oid: pg_sys::Oid) -> Self {
        let mut bman = BufferManager::new(relation_oid);
        let merge_lock = bman.get_buffer_mut(MERGE_LOCK);
        MergeLock(merge_lock)
    }

    pub unsafe fn init(relation_id: pg_sys::Oid) {
        let mut bman = BufferManager::new(relation_id);
        let mut merge_lock = bman.get_buffer_mut(MERGE_LOCK);
        let mut page = merge_lock.page_mut();
        let metadata = page.contents_mut::<MergeLockData>();
        metadata.last_merge = pg_sys::InvalidTransactionId;
    }
}

impl Drop for MergeLock {
    fn drop(&mut self) {
        unsafe {
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

                let mut page = self.0.page_mut();
                let metadata = page.contents_mut::<MergeLockData>();
                metadata.last_merge = current_xid;
            }
        }
    }
}
