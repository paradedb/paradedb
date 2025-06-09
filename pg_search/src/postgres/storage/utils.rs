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
use crate::postgres::storage::block::PgItem;
use crate::postgres::storage::metadata::MetaPage;
use parking_lot::Mutex;
use pgrx::pg_sys;
use pgrx::pg_sys::OffsetNumber;

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

    pub unsafe fn extend_relation(&self) -> pg_sys::Buffer {
        pg_sys::LockRelationForExtension(self.indexrel, pg_sys::ExclusiveLock as i32);

        let buffer = self.get_buffer(
            pg_sys::InvalidBlockNumber,
            Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
        );

        pg_sys::UnlockRelationForExtension(self.indexrel, pg_sys::ExclusiveLock as i32);
        buffer
    }

    pub unsafe fn new_buffer(&self) -> pg_sys::Buffer {
        // It is imperative that the metadata page itself does not create any pages,
        // because those would call new_buffer and cause a loop.

        let metadata = MetaPage::open((*self.indexrel).rd_id);
        if let Some(mut fsm) = metadata.fsm_opt() {
            if let Some(blockno) = fsm.pop() {
                // Try to reuse a recyclable page
                return self.get_buffer(blockno.into(), Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
            }
        }

        // No recyclable pages found, create a new page
        // Postgres requires an exclusive lock on the relation to create a new page
        self.extend_relation()
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
