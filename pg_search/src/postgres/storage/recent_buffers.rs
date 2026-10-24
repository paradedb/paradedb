// Copyright (c) 2023-2026 ParadeDB, Inc.
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

use pgrx::pg_sys;
use std::cell::RefCell;

#[derive(Default)]
struct RecentBuffers {
    key: Option<[u32; 4]>,
    buffers: Vec<pg_sys::Buffer>,
}

thread_local! {
    static RECENT: RefCell<RecentBuffers> = RefCell::new(RecentBuffers::default());
}

pub(super) unsafe fn read(
    rel: pg_sys::Relation,
    fork: pg_sys::ForkNumber::Type,
    block: pg_sys::BlockNumber,
) -> pg_sys::Buffer {
    #[cfg(feature = "pg15")]
    let locator = (*rel).rd_node;
    #[cfg(not(feature = "pg15"))]
    let locator = (*rel).rd_locator;
    #[cfg(feature = "pg15")]
    let file = locator.relNode.to_u32();
    #[cfg(not(feature = "pg15"))]
    let file = locator.relNumber.to_u32();
    let key = [
        locator.spcOid.to_u32(),
        locator.dbOid.to_u32(),
        file,
        fork as u32,
    ];

    RECENT.with_borrow_mut(|cache| {
        if cache.key != Some(key) {
            let blocks = pg_sys::RelationGetNumberOfBlocksInFork(rel, fork) as usize;
            // Bound backend-local hints to 16 MiB; larger block numbers use normal reads.
            let capacity = (blocks + blocks / 8 + 1024).min(4 * 1024 * 1024);
            cache.buffers.clear();
            cache.buffers.resize(capacity, pg_sys::InvalidBuffer as _);
            cache.key = Some(key);
        }
        let slot = cache.buffers.get_mut(block as usize);
        if let Some(buffer) = slot.as_deref()
            && *buffer != pg_sys::InvalidBuffer as pg_sys::Buffer
            && pg_sys::ReadRecentBuffer(locator, fork, block, *buffer)
        {
            if (*rel).pgstat_enabled {
                if (*rel).pgstat_info.is_null() {
                    pg_sys::pgstat_assoc_relation(rel);
                }
                if !(*rel).pgstat_info.is_null() {
                    #[cfg(feature = "pg15")]
                    {
                        (*(*rel).pgstat_info).t_counts.t_blocks_hit += 1;
                    }
                    #[cfg(not(feature = "pg15"))]
                    {
                        (*(*rel).pgstat_info).counts.blocks_hit += 1;
                    }
                }
            }
            return *buffer;
        }
        let buffer = pg_sys::ReadBufferExtended(
            rel,
            fork,
            block,
            pg_sys::ReadBufferMode::RBM_NORMAL,
            std::ptr::null_mut(),
        );
        if let Some(slot) = slot {
            *slot = buffer;
        }
        buffer
    })
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::postgres::rel::PgSearchRelation;
    use pgrx::prelude::*;

    #[pg_test]
    unsafe fn test_segment_component_recent_buffer_hints() {
        Spi::run("CREATE TABLE recent_hints (id INT)").unwrap();
        Spi::run("CREATE INDEX recent_hints_a ON recent_hints USING paradedb (id)").unwrap();
        Spi::run("CREATE TABLE recent_other (id INT)").unwrap();
        Spi::run("CREATE INDEX recent_hints_b ON recent_other USING paradedb (id)").unwrap();
        let oid = Spi::get_one::<pg_sys::Oid>("SELECT 'recent_hints_a'::regclass::oid")
            .unwrap()
            .unwrap();
        let a = PgSearchRelation::open(oid);
        let first = read(a.as_ptr(), a.fork_number(), 0);
        pg_sys::ReleaseBuffer(first);
        let before = pg_sys::pgBufferUsage.shared_blks_hit;
        let again = read(a.as_ptr(), a.fork_number(), 0);
        assert_eq!(again, first);
        let after = pg_sys::pgBufferUsage.shared_blks_hit;
        assert_eq!(after, before + 1);
        pg_sys::ReleaseBuffer(again);
        let different = read(a.as_ptr(), a.fork_number(), 1);
        pg_sys::ReleaseBuffer(different);
        RECENT.with_borrow_mut(|cache| cache.buffers[0] = different);
        let repaired = read(a.as_ptr(), a.fork_number(), 0);
        assert_eq!(pg_sys::BufferGetBlockNumber(repaired), 0);
        assert_eq!(repaired, first);
        pg_sys::ReleaseBuffer(repaired);
        let key = RECENT.with_borrow(|cache| cache.key);
        let oid = Spi::get_one::<pg_sys::Oid>("SELECT 'recent_hints_b'::regclass::oid")
            .unwrap()
            .unwrap();
        let b = PgSearchRelation::open(oid);
        let other = read(b.as_ptr(), b.fork_number(), 0);
        assert_ne!(RECENT.with_borrow(|cache| cache.key), key);
        pg_sys::ReleaseBuffer(other);
    }
}
