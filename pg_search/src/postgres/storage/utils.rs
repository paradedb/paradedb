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

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{BM25PageSpecialData, PgItem};
use crate::postgres::storage::buffer::BufferMutVec;
use pgrx::pg_sys::OffsetNumber;
use pgrx::{check_for_interrupts, pg_sys, PgMemoryContexts};
use std::cell::LazyCell;
use std::fmt::Debug;
use std::sync::LazyLock;

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
}

struct BufferAccessStrategyHolder(pg_sys::BufferAccessStrategy);
unsafe impl Send for BufferAccessStrategyHolder {}
unsafe impl Sync for BufferAccessStrategyHolder {}

static BAS_BULKWRITE: LazyLock<BufferAccessStrategyHolder> = LazyLock::new(|| {
    BufferAccessStrategyHolder(unsafe {
        // SAFETY:  Allocated in `TopMemoryContext`, once, so that it's always available
        PgMemoryContexts::TopMemoryContext.switch_to(|_| {
            pg_sys::GetAccessStrategy(pg_sys::BufferAccessStrategyType::BAS_BULKWRITE)
        })
    })
});

#[derive(Clone, Debug)]
pub struct BM25BufferCache {
    rel: PgSearchRelation,
}

unsafe impl Send for BM25BufferCache {}
unsafe impl Sync for BM25BufferCache {}

impl BM25BufferCache {
    pub fn open(rel: &PgSearchRelation) -> Self {
        Self { rel: rel.clone() }
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
                        BAS_BULKWRITE.0,
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

    pub unsafe fn new_buffer(&self) -> pg_sys::Buffer {
        let buffer = self.bulk_extend_relation(1)[0];
        pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as i32);
        buffer
    }

    pub unsafe fn new_buffers(&self, npages: usize) -> BufferMutVec {
        if npages == 0 {
            return BufferMutVec::empty();
        }

        let mut buffers = [pg_sys::InvalidBuffer as pg_sys::Buffer; MAX_BUFFERS_TO_EXTEND_BY];
        // bulk_extend_relation() returns `pg_sys::Buffer` instances that are not locked
        let extended_buffers = self.bulk_extend_relation(npages);
        for (i, buffer) in extended_buffers.iter().take(npages).enumerate() {
            buffers[i] = *buffer;
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
}
