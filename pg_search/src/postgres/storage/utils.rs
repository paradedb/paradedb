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

use crate::api::HashMap;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{bm25_max_free_space, BM25PageSpecialData, PgItem};
use parking_lot::Mutex;
use pgrx::pg_sys::OffsetNumber;
use pgrx::{check_for_interrupts, pg_sys, PgMemoryContexts};
use std::fmt::Debug;

/// Matches Postgres's [`MAX_BUFFERS_TO_EXTEND_BY`]
pub const MAX_BUFFERS_TO_EXTEND_BY: usize = 64;

pub trait BM25Page {
    /// Read the opaque, non-decoded [`PgItem`] at `offno`.
    unsafe fn read_item(&self, offno: OffsetNumber) -> Option<PgItem>;

    /// Read the fully decoded/deserialized item at `offno` into an instance of `T`
    unsafe fn deserialize_item<T: From<PgItem>>(
        &self,
        offsetno: pg_sys::OffsetNumber,
    ) -> Option<(T, pg_sys::Size)>;

    unsafe fn xmax(&self) -> pg_sys::TransactionId;

    /// Return true if the page is able to reused as if it were a new page
    unsafe fn is_reusable(&self) -> bool;
}

impl BM25Page for pg_sys::Page {
    unsafe fn deserialize_item<T: From<PgItem>>(
        &self,
        offno: OffsetNumber,
    ) -> Option<(T, pg_sys::Size)> {
        let pg_item = self.read_item(offno)?;
        let size = pg_item.1;
        Some((T::from(pg_item), size))
    }

    unsafe fn read_item(&self, offno: OffsetNumber) -> Option<PgItem> {
        let item_id = pg_sys::PageGetItemId(*self, offno);

        if (*item_id).lp_flags() != pg_sys::LP_NORMAL {
            return None;
        }

        let item = pg_sys::PageGetItem(*self, item_id);
        let size = (*item_id).lp_len() as pg_sys::Size;
        Some(PgItem(item, size))
    }

    unsafe fn xmax(&self) -> pg_sys::TransactionId {
        let special = pg_sys::PageGetSpecialPointer(*self) as *mut BM25PageSpecialData;
        (*special).xmax
    }

    unsafe fn is_reusable(&self) -> bool {
        if pg_sys::PageIsNew(*self) {
            return true;
        }

        // technically we're only called on pages given to us by the FSM, and in that case the page
        // can be immediately reused if our internal `xmax` tracking is set to frozen, indicating
        // that it's been deleted
        self.xmax() == pg_sys::FrozenTransactionId
    }
}

#[derive(Debug)]
pub struct BM25BufferCache {
    rel: PgSearchRelation,
    cache: Mutex<HashMap<pg_sys::BlockNumber, Vec<u8>>>,
    bulkwrite_bas: pg_sys::BufferAccessStrategy,
}

unsafe impl Send for BM25BufferCache {}
unsafe impl Sync for BM25BufferCache {}

impl BM25BufferCache {
    pub fn open(rel: &PgSearchRelation) -> Self {
        unsafe {
            Self {
                rel: Clone::clone(rel),
                cache: Default::default(),
                bulkwrite_bas: PgMemoryContexts::TopTransactionContext.switch_to(|_| {
                    pg_sys::GetAccessStrategy(pg_sys::BufferAccessStrategyType::BAS_BULKWRITE)
                }),
            }
        }
    }

    pub fn rel(&self) -> &PgSearchRelation {
        &self.rel
    }

    unsafe fn bulk_extend_relation(
        &self,
        npages: usize,
    ) -> [pg_sys::Buffer; MAX_BUFFERS_TO_EXTEND_BY] {
        let mut buffers = [pg_sys::InvalidBuffer as pg_sys::Buffer; MAX_BUFFERS_TO_EXTEND_BY];

        #[cfg(any(feature = "pg16", feature = "pg17"))]
        {
            // `ExtendBufferedRelBy` is only allowed from certain backends
            let can_use_extend_buffered_rel_by = npages > 1
                && (pg_sys::MyBackendType == pg_sys::BackendType::B_BG_WORKER
                    || pg_sys::MyBackendType == pg_sys::BackendType::B_BACKEND);

            if can_use_extend_buffered_rel_by {
                let mut filled = 0;
                let mut extended_by = 0;
                loop {
                    check_for_interrupts!();
                    let bmr = pg_sys::BufferManagerRelation {
                        rel: self.rel.as_ptr(),
                        ..Default::default()
                    };
                    pg_sys::ExtendBufferedRelBy(
                        bmr,
                        pg_sys::ForkNumber::MAIN_FORKNUM,
                        self.bulkwrite_bas,
                        0,
                        (npages - filled) as _,
                        buffers.as_mut_ptr().add(filled),
                        &mut extended_by,
                    );
                    filled += extended_by as usize;
                    extended_by = 0;
                    if filled == npages {
                        break;
                    }
                }

                return buffers;
            }
        }

        pg_sys::LockRelationForExtension(self.rel.as_ptr(), pg_sys::AccessExclusiveLock as i32);
        for buffer in buffers.iter_mut().take(npages) {
            *buffer = self.get_buffer(pg_sys::InvalidBlockNumber, None);
        }
        pg_sys::UnlockRelationForExtension(self.rel.as_ptr(), pg_sys::AccessExclusiveLock as i32);
        buffers
    }

    unsafe fn recycled_buffer(&self) -> Option<pg_sys::Buffer> {
        loop {
            check_for_interrupts!();
            // ask for a page with at least `bm25_max_free_space()` -- that's how much we need to do our things
            let blockno =
                pg_sys::GetPageWithFreeSpace(self.rel.as_ptr(), bm25_max_free_space() as _);
            if blockno == pg_sys::InvalidBlockNumber {
                return None;
            }
            // we got one, so let Postgres know so the FSM will stop considering it
            pg_sys::RecordUsedIndexPage(self.rel.as_ptr(), blockno);

            let buffer = self.get_buffer(blockno, None);
            if pg_sys::ConditionalLockBuffer(buffer) {
                let page = pg_sys::BufferGetPage(buffer);

                // the FSM would have returned a page to us that was previously known to be reusable,
                // but it may not still be reusable now that we have a lock.
                //
                // between then and now some other backend could have gotten this page too, locked it,
                // (re)initialized it, and released its lock, making it unusable by us
                if page.is_reusable() {
                    return Some(buffer);
                }

                pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_UNLOCK as i32);
            }

            pg_sys::ReleaseBuffer(buffer);
        }
    }

    pub unsafe fn new_buffer(&self) -> pg_sys::Buffer {
        self.recycled_buffer().unwrap_or_else(|| {
            let buffer = self.bulk_extend_relation(1)[0];
            pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as i32);
            buffer
        })
    }

    pub unsafe fn new_buffers(&self, npages: usize) -> BufferMutVec {
        let mut buffers =
            [(pg_sys::InvalidBuffer as pg_sys::Buffer, false); MAX_BUFFERS_TO_EXTEND_BY];
        let mut remaining = npages;
        let mut cursor = 0;

        while remaining > 0 {
            if let Some(buffer) = self.recycled_buffer() {
                // recycled_buffers() returns buffers that are already locked
                buffers[cursor] = (buffer, false);
                cursor += 1;
                remaining -= 1;
            } else {
                break;
            }
        }

        if remaining > 0 {
            let extended_buffers = self.bulk_extend_relation(remaining);
            // bulk_extend_relation() returns buffers that are not locked
            for buffer in extended_buffers.iter().take(remaining) {
                buffers[cursor] = (*buffer, true);
                cursor += 1;
            }
        }

        BufferMutVec::new(buffers)
    }

    pub unsafe fn get_buffer(
        &self,
        blockno: pg_sys::BlockNumber,
        lock: Option<u32>,
    ) -> pg_sys::Buffer {
        self.get_buffer_with_strategy(blockno, std::ptr::null_mut(), lock)
    }

    pub unsafe fn get_buffer_with_strategy(
        &self,
        blockno: pg_sys::BlockNumber,
        strategy: pg_sys::BufferAccessStrategy,
        lock: Option<u32>,
    ) -> pg_sys::Buffer {
        let buffer = pg_sys::ReadBufferExtended(
            self.rel.as_ptr(),
            pg_sys::ForkNumber::MAIN_FORKNUM,
            blockno,
            pg_sys::ReadBufferMode::RBM_NORMAL,
            strategy,
        );
        debug_assert!(buffer != pg_sys::InvalidBuffer as pg_sys::Buffer);
        if let Some(lock) = lock {
            pg_sys::LockBuffer(buffer, lock as i32);
        }
        buffer
    }

    pub unsafe fn get_page_slice(&self, blockno: pg_sys::BlockNumber, lock: Option<u32>) -> &[u8] {
        let mut cache = self.cache.lock();
        let slice = cache.entry(blockno).or_insert_with(|| {
            let buffer = self.get_buffer(blockno, lock);
            let page = pg_sys::BufferGetPage(buffer);
            let data =
                std::slice::from_raw_parts(page as *mut u8, pg_sys::BLCKSZ as usize).to_vec();
            pg_sys::UnlockReleaseBuffer(buffer);

            data
        });

        std::slice::from_raw_parts(slice.as_ptr(), slice.len())
    }
}

impl Drop for BM25BufferCache {
    fn drop(&mut self) {
        unsafe {
            pg_sys::FreeAccessStrategy(self.bulkwrite_bas);
        }
    }
}

/// Holds an array of buffers -- used for bulk allocating new buffers.
///
/// These buffers can either be locked (if retrieved from the FSM) or unlocked (if the relation was extended)
/// [`BufferMutVec`] locks/releases them appropriately when they are claimed/dropped.
type NeedsLock = bool;
pub struct BufferMutVec {
    inner: [(pg_sys::Buffer, NeedsLock); MAX_BUFFERS_TO_EXTEND_BY],
    cursor: usize,
}

impl BufferMutVec {
    pub fn new(buffers: [(pg_sys::Buffer, NeedsLock); MAX_BUFFERS_TO_EXTEND_BY]) -> Self {
        Self {
            inner: buffers,
            cursor: 0,
        }
    }

    /// Claim a buffer from the start, which ensures that the buffers are in the same order as they were created.
    /// Typically this means in order of increasing block number.
    pub fn next(&mut self) -> Option<pg_sys::Buffer> {
        if self.cursor >= MAX_BUFFERS_TO_EXTEND_BY {
            return None;
        }

        let (buffer, needs_lock) = self.inner[self.cursor];
        self.cursor += 1;

        if buffer == pg_sys::InvalidBuffer as pg_sys::Buffer {
            return None;
        }

        if needs_lock {
            unsafe {
                pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as i32);
            }
        }
        Some(buffer)
    }
}

impl Drop for BufferMutVec {
    fn drop(&mut self) {
        if unsafe { pg_sys::InterruptHoldoffCount > 0 }
            && unsafe { crate::postgres::utils::IsTransactionState() }
        {
            loop {
                if self.cursor >= MAX_BUFFERS_TO_EXTEND_BY {
                    break;
                }

                unsafe {
                    let (buffer, needs_lock) = self.inner[self.cursor];
                    if buffer == pg_sys::InvalidBuffer as pg_sys::Buffer {
                        break;
                    }

                    if needs_lock {
                        pg_sys::ReleaseBuffer(buffer);
                    } else {
                        pg_sys::UnlockReleaseBuffer(buffer);
                    }

                    self.cursor += 1;
                }
            }
        }
    }
}
