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

use pgrx::pg_sys;

/// If the given Bitmapset has exactly one member, return it.
pub unsafe fn bms_exactly_one_member(bms: *mut pg_sys::Bitmapset) -> Option<pg_sys::Index> {
    let mut members = bms_iter(bms);
    let first_member = members.next()?;
    if members.next().is_some() {
        // There is more than one member.
        None
    } else {
        // There is only one member.
        Some(first_member)
    }
}

/// Helper function to create an iterator over Bitmapset members
unsafe fn bms_iter(bms: *mut pg_sys::Bitmapset) -> impl Iterator<Item = pg_sys::Index> {
    let mut set_bit: i32 = -1;
    std::iter::from_fn(move || {
        set_bit = pg_sys::bms_next_member(bms, set_bit);
        if set_bit < 0 {
            None
        } else {
            Some(set_bit as pg_sys::Index)
        }
    })
}

/// Helper function to check if a Bitmapset is empty
unsafe fn bms_is_empty(bms: *mut pg_sys::Bitmapset) -> bool {
    bms_iter(bms).next().is_none()
}

/// Helper function to determine if we're dealing with a partitioned table setup
pub unsafe fn is_partitioned_table_setup(
    root: *mut pg_sys::PlannerInfo,
    rel_relids: *mut pg_sys::Bitmapset,
    baserels: *mut pg_sys::Bitmapset,
) -> bool {
    // If the relation bitmap is empty, early return
    if bms_is_empty(rel_relids) {
        return false;
    }

    // Get the rtable for relkind checks
    let rtable = (*(*root).parse).rtable;

    // For each relation in baserels
    for baserel_idx in bms_iter(baserels) {
        // Skip invalid indices
        if baserel_idx == 0 || baserel_idx >= (*root).simple_rel_array_size as pg_sys::Index {
            continue;
        }

        // Get the RTE to check if this is a partitioned table
        let rte = pg_sys::rt_fetch(baserel_idx, rtable);
        if (*rte).relkind as u8 != pg_sys::RELKIND_PARTITIONED_TABLE {
            continue;
        }

        // Access RelOptInfo safely using offset and read
        if (*root).simple_rel_array.is_null() {
            continue;
        }

        // This is a partitioned table, get its RelOptInfo to find partitions
        let rel_info_ptr = *(*root).simple_rel_array.add(baserel_idx as usize);
        if rel_info_ptr.is_null() {
            continue;
        }

        let rel_info = &*rel_info_ptr;

        // Check if it has partitions
        if rel_info.all_partrels.is_null() {
            continue;
        }

        // Check if any relation in rel_relids is among the partitions
        if pg_sys::bms_overlap(rel_info.all_partrels, rel_relids) {
            return true;
        }
    }

    false
}

/// Get the RangeTblEntry for the given index from the given array.
///
/// Note that range tables are always 1-indexed, so the 0th element is wasted in the given array.
pub unsafe fn get_rte(
    rt_size: usize,
    rt: *mut *mut pg_sys::RangeTblEntry,
    index: pg_sys::Index,
) -> Option<*mut pg_sys::RangeTblEntry> {
    let index = index as usize;

    if index > rt_size {
        return None;
    }
    let ptr_to_nth_rte_ptr = rt.add(index);
    let nth_rte_ptr = *ptr_to_nth_rte_ptr;
    if nth_rte_ptr.is_null() {
        return None;
    }
    Some(nth_rte_ptr)
}
