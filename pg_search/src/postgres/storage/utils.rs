// Copyright (c) 2023-2024 Retake, Inc.
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

use crate::postgres::storage::block::BM25PageSpecialData;
use pgrx::pg_sys;
use pgrx::PgBox;

pub trait BM25Page {
    unsafe fn init(self, page_size: pg_sys::Size);
    unsafe fn mark_deleted(self);
    unsafe fn recyclable(self, heap_relation: pg_sys::Relation) -> bool;
}

impl BM25Page for pg_sys::Page {
    unsafe fn init(self, page_size: pg_sys::Size) {
        pg_sys::PageInit(self, page_size, size_of::<BM25PageSpecialData>());

        let special = pg_sys::PageGetSpecialPointer(self) as *mut BM25PageSpecialData;
        (*special).next_blockno = pg_sys::InvalidBlockNumber;
        (*special).deleted = false;
        (*special).delete_xid = pg_sys::FullTransactionId::default();
    }

    unsafe fn mark_deleted(self) {
        let special = pg_sys::PageGetSpecialPointer(self) as *mut BM25PageSpecialData;
        (*special).next_blockno = pg_sys::InvalidBlockNumber;
        (*special).deleted = true;
        (*special).delete_xid = pg_sys::ReadNextFullTransactionId();
    }

    unsafe fn recyclable(self, heap_relation: pg_sys::Relation) -> bool {
        if pg_sys::PageIsNew(self) {
            return true;
        }

        let special = pg_sys::PageGetSpecialPointer(self) as *mut BM25PageSpecialData;
        if !(*special).deleted {
            return false;
        }

        pg_sys::GlobalVisCheckRemovableFullXid(heap_relation, (*special).delete_xid)
    }
}

#[derive(Clone, Debug)]
pub struct BM25BufferCache {
    indexrel: PgBox<pg_sys::RelationData>,
    heaprel: PgBox<pg_sys::RelationData>,
}

unsafe impl Send for BM25BufferCache {}
unsafe impl Sync for BM25BufferCache {}

impl BM25BufferCache {
    pub unsafe fn open(indexrelid: pg_sys::Oid) -> Self {
        let indexrel = pg_sys::RelationIdGetRelation(indexrelid);
        let heaprelid = pg_sys::IndexGetRelation(indexrelid, false);
        let heaprel = pg_sys::RelationIdGetRelation(heaprelid);
        Self {
            indexrel: PgBox::from_pg(indexrel),
            heaprel: PgBox::from_pg(heaprel),
        }
    }

    pub unsafe fn new_buffer(&self) -> pg_sys::Buffer {
        // Try to find a recyclable page
        loop {
            let blockno = pg_sys::GetFreeIndexPage(self.indexrel.as_ptr());
            if blockno == pg_sys::InvalidBlockNumber {
                break;
            }

            let buffer = self.get_buffer(blockno, None);

            if pg_sys::ConditionalLockBuffer(buffer) {
                let page = pg_sys::BufferGetPage(buffer);
                if page.recyclable(self.heaprel.as_ptr()) {
                    page.init(pg_sys::BufferGetPageSize(buffer));
                    pg_sys::RelationClose(self.heaprel.as_ptr());
                    return buffer;
                }

                pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_UNLOCK as i32);
            }

            pg_sys::ReleaseBuffer(buffer);
        }

        // No recyclable pages found, create a new page
        // Postgres requires an exclusive lock on the relation to create a new page
        pg_sys::LockRelationForExtension(self.indexrel.as_ptr(), pg_sys::ExclusiveLock as i32);

        let buffer = self.get_buffer(
            pg_sys::InvalidBlockNumber,
            Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
        );
        let page = pg_sys::BufferGetPage(buffer);
        page.init(pg_sys::BufferGetPageSize(buffer));

        pg_sys::UnlockRelationForExtension(self.indexrel.as_ptr(), pg_sys::ExclusiveLock as i32);

        buffer
    }

    pub unsafe fn get_buffer(
        &self,
        blockno: pg_sys::BlockNumber,
        lock: Option<u32>,
    ) -> pg_sys::Buffer {
        let buffer = pg_sys::ReadBufferExtended(
            self.indexrel.as_ptr(),
            pg_sys::ForkNumber::MAIN_FORKNUM,
            blockno,
            pg_sys::ReadBufferMode::RBM_NORMAL,
            std::ptr::null_mut(),
        );
        if let Some(lock) = lock {
            pg_sys::LockBuffer(buffer, lock as i32);
        }
        buffer
    }

    pub unsafe fn record_free_index_page(&self, blockno: pg_sys::BlockNumber) {
        pg_sys::RecordFreeIndexPage(self.indexrel.as_ptr(), blockno);
    }
}

impl Drop for BM25BufferCache {
    fn drop(&mut self) {
        unsafe {
            if pg_sys::IsTransactionState() {
                pg_sys::RelationClose(self.indexrel.as_ptr());
                pg_sys::RelationClose(self.heaprel.as_ptr());
            }
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;

    #[pg_test]
    unsafe fn test_new_buffer() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run(
            "CALL paradedb.create_bm25(
            index_name => 't_idx',
            table_name => 't',
            key_field => 'id',
            text_fields => paradedb.field('data')
        )",
        )
        .unwrap();
        let relation_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();
        let index_relation = pg_sys::RelationIdGetRelation(relation_oid);
        let cache = BM25BufferCache::open(relation_oid);
        let nblocks = pg_sys::RelationGetNumberOfBlocksInFork(
            index_relation,
            pg_sys::ForkNumber::MAIN_FORKNUM,
        );
        let buffer = cache.new_buffer();

        assert_eq!(pg_sys::BufferGetBlockNumber(buffer), nblocks);
        pg_sys::RelationClose(index_relation);
    }
}
