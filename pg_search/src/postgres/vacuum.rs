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

use crate::postgres::storage::merge::MergeLock;
use pgrx::*;

#[pg_guard]
pub unsafe extern "C" fn amvacuumcleanup(
    info: *mut pg_sys::IndexVacuumInfo,
    stats: *mut pg_sys::IndexBulkDeleteResult,
) -> *mut pg_sys::IndexBulkDeleteResult {
    // garbage collect the MERGE_LOCK
    // we keep it pruned during normal operations, but a cleanup could be necessary
    // in the event of a panic/crash between creating a MergeEntry and removing it after merging
    let mut merge_lock = MergeLock::acquire((*(*info).index).rd_id);
    merge_lock.garbage_collect();
    pg_sys::IndexFreeSpaceMapVacuum((*info).index);
    drop(merge_lock);

    stats
}
