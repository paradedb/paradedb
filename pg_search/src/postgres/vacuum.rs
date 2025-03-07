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

use crate::postgres::storage::buffer::BufferManager;
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

    // return all recyclable pages to the free space map
    unsafe {
        let index_relation = PgRelation::from_pg(info.index);
        let index_oid = index_relation.oid();
        let nblocks =
            pg_sys::RelationGetNumberOfBlocksInFork(info.index, pg_sys::ForkNumber::MAIN_FORKNUM);
        let mut bman = BufferManager::new(index_oid);
        let heap_oid = pg_sys::IndexGetRelation(index_oid, false);
        let heap_relation = pg_sys::RelationIdGetRelation(heap_oid);

        for blockno in 0..nblocks {
            if blockno % 100 == 0 {
                pg_sys::vacuum_delay_point();
            }
            let buffer = bman.get_buffer(blockno);
            let page = buffer.page();

            if page.is_recyclable(heap_relation) {
                bman.record_free_index_page(buffer);
            }
        }
        pg_sys::RelationClose(heap_relation);
        pg_sys::IndexFreeSpaceMapVacuum(info.index);
    }

    // TODO: Update stats
    stats
}
