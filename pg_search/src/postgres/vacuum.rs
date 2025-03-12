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

use crate::postgres::storage::block::{FIXED_BLOCK_NUMBERS, MERGE_LOCK};
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::merge::MergeLockData;
use pgrx::*;

#[pg_guard]
pub extern "C" fn amvacuumcleanup(
    info: *mut pg_sys::IndexVacuumInfo,
    stats: *mut pg_sys::IndexBulkDeleteResult,
) -> *mut pg_sys::IndexBulkDeleteResult {
    let info = unsafe { PgBox::from_pg(info) };
    if info.analyze_only {
        return stats;
    }

    unsafe {
        let index_relation = PgRelation::from_pg(info.index);
        let index_oid = index_relation.oid();
        let mut bman = BufferManager::new(index_oid);

        // If we got a lock, truncate index to the last non-recyclable block
        let lock_acquired =
            pg_sys::ConditionalLockRelation(info.heaprel, pg_sys::ExclusiveLock as i32);
        let mut nblocks =
            pg_sys::RelationGetNumberOfBlocksInFork(info.index, pg_sys::ForkNumber::MAIN_FORKNUM);

        if lock_acquired {
            let first_vacuumable_blockno = FIXED_BLOCK_NUMBERS.last().unwrap() + 1;
            let last_blockno = nblocks - 1;
            assert!(
                nblocks >= first_vacuumable_blockno,
                "the index has fewer blocks than should be possible"
            );

            for blockno in (first_vacuumable_blockno..nblocks).rev() {
                if blockno % 100 == 0 {
                    pg_sys::vacuum_delay_point();
                }

                if let Some(buffer) = bman.get_buffer_conditional(blockno) {
                    // If this block is not recyclable but the previous ones were, truncate up to this block
                    let page = buffer.page();
                    if !page.is_recyclable(info.heaprel) {
                        if blockno != last_blockno && pg_sys::CritSectionCount == 0 {
                            nblocks = blockno + 1;
                            pg_sys::RelationTruncate(info.index, nblocks);
                        }
                        break;
                    }
                } else {
                    // If we couldn't get a conditional lock on this block, that means another backend is
                    // processing this block and it's not recyclable. However, if it got this far, then the previous
                    // blocks must have been recyclable, so we can truncate up to the previous block.
                    if (blockno + 1) != last_blockno && pg_sys::CritSectionCount == 0 {
                        nblocks = blockno + 2;
                        pg_sys::RelationTruncate(info.index, nblocks);
                    }
                    break;
                }
            }
        }

        // Return the rest to the free space map, if recyclable
        let vacuum_sentinel_blockno = {
            let merge_lock = bman.get_buffer(MERGE_LOCK);
            let page = merge_lock.page();
            let metadata = page.contents::<MergeLockData>();
            metadata.ambulkdelete_sentinel
        };

        for blockno in FIXED_BLOCK_NUMBERS.last().unwrap() + 1..nblocks {
            if blockno == vacuum_sentinel_blockno {
                // don't try to open the vacuum_sentinel_blockno block -- only `ambulkdelete` should ever
                // have a pin on it.
                continue;
            }
            if blockno % 100 == 0 {
                pg_sys::vacuum_delay_point();
            }
            let buffer = bman.get_buffer(blockno);
            let page = buffer.page();

            if page.is_recyclable(info.heaprel) {
                bman.record_free_index_page(buffer);
            }
        }
        pg_sys::IndexFreeSpaceMapVacuum(info.index);
    }

    // TODO: Update stats
    stats
}
