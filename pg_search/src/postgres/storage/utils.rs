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
use crate::postgres::storage::block::{bm25_max_free_space, BM25PageSpecialData, PgItem};
use parking_lot::Mutex;
use pgrx::pg_sys::OffsetNumber;
use pgrx::{check_for_interrupts, pg_sys};

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

#[repr(C)]
struct BufferManagerRelation {
    relation: pg_sys::Relation,
    smgr: *mut pg_sys::SMgrRelationData,
    relpersistence: i8,
}

impl Default for BufferManagerRelation {
    fn default() -> Self {
        Self {
            relation: std::ptr::null_mut(),
            smgr: std::ptr::null_mut(),
            relpersistence: 0,
        }
    }
}

#[derive(Debug)]
pub struct BM25BufferCache {
    indexrel: pg_sys::Relation,
    cache: Mutex<HashMap<pg_sys::BlockNumber, Vec<u8>>>,
}

unsafe impl Send for BM25BufferCache {}
unsafe impl Sync for BM25BufferCache {}

impl BM25BufferCache {
    pub fn open(indexrelid: pg_sys::Oid) -> Self {
        unsafe {
            let indexrel = pg_sys::RelationIdGetRelation(indexrelid);
            Self {
                indexrel,
                cache: Default::default(),
            }
        }
    }

    pub unsafe fn indexrel(&self) -> *mut pg_sys::RelationData {
        self.indexrel
    }

    unsafe fn bulk_extend_relation(&self, npages: usize) -> Vec<pg_sys::Buffer> {
        let mut buffers = vec![pg_sys::InvalidBuffer as pg_sys::Buffer; npages];
        let mut filled = 0;
        let mut extended_by = 0;

        pg_sys::ffi::pg_guard_ffi_boundary(|| {
            extern "C-unwind" {
                fn ExtendBufferedRelBy(
                    bmr: BufferManagerRelation,
                    fork: i32,
                    strategy: pg_sys::BufferAccessStrategy,
                    flags: pg_sys::uint32,
                    extend_by: pg_sys::uint32,
                    buffers: *mut pg_sys::Buffer,
                    extended_by: *mut pg_sys::uint32,
                ) -> pg_sys::BlockNumber;
            }

            loop {
                check_for_interrupts!();
                let bmr = BufferManagerRelation {
                    relation: self.indexrel,
                    ..Default::default()
                };
                ExtendBufferedRelBy(
                    bmr,
                    pg_sys::ForkNumber::MAIN_FORKNUM,
                    std::ptr::null_mut(),
                    0,
                    (npages - filled) as _,
                    buffers.as_mut_ptr().add(filled),
                    &mut extended_by,
                );
                filled += extended_by as usize;
                if filled == npages {
                    break;
                }
            }
        });

        buffers
    }

    unsafe fn recycled_buffer(&self) -> Option<pg_sys::Buffer> {
        loop {
            check_for_interrupts!();
            // ask for a page with at least `bm25_max_free_space()` -- that's how much we need to do our things
            let blockno = pg_sys::GetPageWithFreeSpace(self.indexrel, bm25_max_free_space() as _);
            if blockno == pg_sys::InvalidBlockNumber {
                return None;
            }
            // we got one, so let Postgres know so the FSM will stop considering it
            pg_sys::RecordUsedIndexPage(self.indexrel, blockno);

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
        let mut buffers = Vec::new();
        let mut remaining = npages;

        while remaining > 0 {
            if let Some(buffer) = self.recycled_buffer() {
                buffers.push((buffer, false));
                remaining -= 1;
            } else {
                break;
            }
        }

        if remaining > 0 {
            let extended_buffers = self.bulk_extend_relation(remaining);
            buffers.extend(extended_buffers.into_iter().map(|buffer| (buffer, true)));
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
            self.indexrel,
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
            if crate::postgres::utils::IsTransactionState() {
                pg_sys::RelationClose(self.indexrel);
            }
        }
    }
}

type NeedsLock = bool;
pub struct BufferMutVec {
    inner: Vec<(pg_sys::Buffer, NeedsLock)>,
}

impl BufferMutVec {
    pub fn new(buffers: Vec<(pg_sys::Buffer, NeedsLock)>) -> Self {
        Self { inner: buffers }
    }

    /// Claim a buffer from the start, which ensures that the buffers are in the same order as they were created.
    /// Typically this means in order of increasing block number.
    pub fn claim_buffer(&mut self) -> Option<pg_sys::Buffer> {
        if self.inner.is_empty() {
            None
        } else {
            let (buffer, needs_lock) = self.inner.remove(0);
            if needs_lock {
                unsafe {
                    pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as i32);
                }
            }
            Some(buffer)
        }
    }
}

impl Drop for BufferMutVec {
    fn drop(&mut self) {
        for (buffer, needs_lock) in self.inner.drain(..) {
            unsafe {
                if buffer != pg_sys::InvalidBuffer as pg_sys::Buffer
                    && pg_sys::InterruptHoldoffCount > 0
                    && crate::postgres::utils::IsTransactionState()
                {
                    if needs_lock {
                        pg_sys::ReleaseBuffer(buffer);
                    } else {
                        pg_sys::UnlockReleaseBuffer(buffer);
                    }
                }
            }
        }
    }
}
