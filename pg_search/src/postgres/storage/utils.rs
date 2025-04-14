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

use crate::postgres::storage::block::{bm25_max_free_space, BM25PageSpecialData, PgItem};
use parking_lot::Mutex;
use pgrx::pg_sys;
use pgrx::pg_sys::OffsetNumber;
use rustc_hash::FxHashMap;

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
    indexrel: pg_sys::Relation,
    heaprel: pg_sys::Relation,
    cache: Mutex<FxHashMap<pg_sys::BlockNumber, Vec<u8>>>,
}

unsafe impl Send for BM25BufferCache {}
unsafe impl Sync for BM25BufferCache {}

impl BM25BufferCache {
    pub fn open(indexrelid: pg_sys::Oid) -> Self {
        unsafe {
            let indexrel = pg_sys::RelationIdGetRelation(indexrelid);
            let heaprelid = pg_sys::IndexGetRelation(indexrelid, false);
            let heaprel = pg_sys::RelationIdGetRelation(heaprelid);
            Self {
                indexrel,
                heaprel,
                cache: Default::default(),
            }
        }
    }

    pub unsafe fn heaprel(&self) -> *mut pg_sys::RelationData {
        self.heaprel
    }

    pub unsafe fn indexrel(&self) -> *mut pg_sys::RelationData {
        self.indexrel
    }

    pub unsafe fn new_buffer(&self) -> pg_sys::Buffer {
        // Try to find a recyclable page
        loop {
            // ask for a page with at least `bm25_max_free_space()` -- that's how much we need to do our things
            let blockno = pg_sys::GetPageWithFreeSpace(self.indexrel, bm25_max_free_space() as _);
            if blockno == pg_sys::InvalidBlockNumber {
                break;
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
                    return buffer;
                }

                pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_UNLOCK as i32);
            }

            pg_sys::ReleaseBuffer(buffer);
        }

        // No recyclable pages found, create a new page
        // Postgres requires an exclusive lock on the relation to create a new page
        pg_sys::LockRelationForExtension(self.indexrel, pg_sys::ExclusiveLock as i32);

        let buffer = self.get_buffer(
            pg_sys::InvalidBlockNumber,
            Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
        );

        pg_sys::UnlockRelationForExtension(self.indexrel, pg_sys::ExclusiveLock as i32);
        buffer
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
                pg_sys::RelationClose(self.heaprel);
            }
        }
    }
}

/// Get the freeze limit for marking XIDs as frozen
/// Inspired by vacuum_get_cutoffs in backend/commands/vacuum.c
pub unsafe fn vacuum_get_freeze_limit(heap_relation: pg_sys::Relation) -> pg_sys::TransactionId {
    extern "C" {
        pub static mut autovacuum_freeze_max_age: ::std::os::raw::c_int;
    }

    let oldest_xmin = pg_sys::GetOldestNonRemovableTransactionId(heap_relation);

    assert!(pg_sys::TransactionIdIsNormal(oldest_xmin));
    assert!(pg_sys::vacuum_freeze_min_age >= 0);

    let next_xid = pg_sys::ReadNextFullTransactionId().value as pg_sys::TransactionId;
    let freeze_min_age =
        std::cmp::min(pg_sys::vacuum_freeze_min_age, autovacuum_freeze_max_age / 2);
    if freeze_min_age > (next_xid as i32) {
        return pg_sys::FirstNormalTransactionId;
    }

    let mut freeze_limit = next_xid - (freeze_min_age as u32);
    // ensure that freeze_limit is a normal transaction ID
    if !pg_sys::TransactionIdIsNormal(freeze_limit) {
        freeze_limit = pg_sys::FirstNormalTransactionId;
    }
    // freeze_limit must always be <= oldest_xmin
    if pg_sys::TransactionIdPrecedes(oldest_xmin, freeze_limit) {
        freeze_limit = oldest_xmin;
    }
    freeze_limit
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;

    #[pg_test]
    unsafe fn test_freeze_limit_relaxed() {
        let vacuum_freeze_min_age = 50_000_000;

        Spi::run(&format!(
            "SET vacuum_freeze_min_age = {};",
            vacuum_freeze_min_age
        ))
        .unwrap();
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();

        let heap_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't' AND relnamespace = current_schema()::regnamespace;")
                .expect("spi should succeed")
                .unwrap();
        let heap_relation = pg_sys::RelationIdGetRelation(heap_oid);

        if pg_sys::ReadNextFullTransactionId().value <= vacuum_freeze_min_age as u64 {
            assert_eq!(
                vacuum_get_freeze_limit(heap_relation),
                pg_sys::FirstNormalTransactionId
            );
        } else {
            assert!(vacuum_get_freeze_limit(heap_relation) > pg_sys::FirstNormalTransactionId);
        }

        pg_sys::RelationClose(heap_relation);
    }

    #[pg_test]
    unsafe fn test_freeze_limit_aggressive() {
        let vacuum_freeze_min_age = 0;

        Spi::run(&format!(
            "SET vacuum_freeze_min_age = {};",
            vacuum_freeze_min_age
        ))
        .unwrap();
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("INSERT INTO t (data) VALUES ('test')").unwrap();

        let heap_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't' AND relnamespace = current_schema()::regnamespace;")
                .expect("spi should succeed")
                .unwrap();
        let heap_relation = pg_sys::RelationIdGetRelation(heap_oid);

        assert_eq!(
            vacuum_get_freeze_limit(heap_relation),
            pg_sys::GetOldestNonRemovableTransactionId(heap_relation),
        );

        pg_sys::RelationClose(heap_relation);
    }
}
