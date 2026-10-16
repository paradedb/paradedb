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

use crate::postgres::search_operator_relations::top_parent;
use pgrx::pg_sys;

/// Check if an RTE represents a plain table (not a view, subquery, etc.)
/// Returns the relid if it's a plain relation (RELKIND_RELATION or RELKIND_MATVIEW).
pub fn get_plain_relation_relid(rte: *mut pg_sys::RangeTblEntry) -> Option<pg_sys::Oid> {
    if rte.is_null() {
        return None;
    }
    let rte = unsafe { &*rte };

    // Must be a plain relation RTE
    if rte.rtekind != pg_sys::RTEKind::RTE_RELATION {
        return None;
    }

    // An inheritance parent stands for itself plus its children, but its index
    // covers only its own rows. Scanning it would drop every child row.
    // `FROM ONLY parent` clears `inh` and stays eligible.
    if rte.inh {
        return None;
    }

    let relid = rte.relid;
    let relkind = unsafe { pg_sys::get_rel_relkind(relid) } as u8;

    // Only support regular tables and materialized views
    if relkind != pg_sys::RELKIND_RELATION && relkind != pg_sys::RELKIND_MATVIEW {
        return None;
    }

    Some(relid)
}

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
pub unsafe fn bms_iter(bms: *mut pg_sys::Bitmapset) -> impl Iterator<Item = pg_sys::Index> {
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

/// Whether the relation is a partition, at any depth, of a partitioned table in `baserels`.
///
/// Partitioning expands one level at a time, so a table's `all_partrels` holds its direct
/// partitions only: the leaves under a partitioned partition are reached through their parents.
pub unsafe fn is_partitioned_table_setup(
    root: *mut pg_sys::PlannerInfo,
    rel_relids: *mut pg_sys::Bitmapset,
    baserels: *mut pg_sys::Bitmapset,
) -> bool {
    bms_iter(rel_relids).any(|rti| {
        top_parent(root, rti).is_some_and(|(table, _)| {
            table != rti
                && rte_is_partitioned(root, table)
                && pg_sys::bms_is_member(table as i32, baserels)
        })
    })
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

pub unsafe fn rte_is_partitioned(root: *mut pg_sys::PlannerInfo, rti: pg_sys::Index) -> bool {
    let rtable = (*(*root).parse).rtable;
    let rte = pg_sys::rt_fetch(rti, rtable);
    (*rte).relkind as u8 == pg_sys::RELKIND_PARTITIONED_TABLE
}

pub unsafe fn rte_is_parent(
    root: *mut pg_sys::PlannerInfo,
    parent: pg_sys::Index,
    child: pg_sys::Index,
) -> bool {
    if (*root).simple_rel_array.is_null() || child > (*root).simple_rel_array_size as pg_sys::Index
    {
        return false;
    }

    let parent_rel_info_ptr = *(*root).simple_rel_array.add(child as usize);
    if parent_rel_info_ptr.is_null() {
        return false;
    }

    let parent_rel_info = &*parent_rel_info_ptr;
    if parent_rel_info.all_partrels.is_null() {
        return false;
    }

    pg_sys::bms_is_member(parent as i32, parent_rel_info.all_partrels)
}
