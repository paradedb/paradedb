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

use crate::postgres::rel::PgSearchRelation;
use anyhow::{Result, anyhow};
use pgrx::{PgList, pg_sys};

pub enum IndexKind {
    Index(PgSearchRelation),
    PartitionedIndex(Vec<PgSearchRelation>),
}

impl IndexKind {
    ///
    /// Get the IndexKind for the given relation, or an error if it is not an index.
    ///
    pub fn for_index(index_relation: PgSearchRelation) -> Result<IndexKind> {
        let index_relkind = unsafe { pg_sys::get_rel_relkind(index_relation.oid()) as u8 };
        match index_relkind {
            pg_sys::RELKIND_INDEX => {
                // The index is not partitioned.
                Ok(IndexKind::Index(index_relation))
            }
            pg_sys::RELKIND_PARTITIONED_INDEX => Ok(IndexKind::PartitionedIndex(
                leaf_partition_indexes(&index_relation).collect(),
            )),
            _ => Err(anyhow!("Expected to receive an index argument.")),
        }
    }

    ///
    /// Return an iterator over the partitions of this index, which might be
    /// of length 1 if it is not partitioned.
    ///
    pub fn partitions(self) -> Box<dyn Iterator<Item = PgSearchRelation>> {
        match self {
            Self::Index(rel) => Box::new(std::iter::once(rel)),
            Self::PartitionedIndex(rel) => Box::new(rel.into_iter()),
        }
    }
}

#[allow(improper_ctypes)]
#[rustfmt::skip]
unsafe extern "C-unwind" {
    /// Core partition catalog walkers (`catalog/pg_inherits.h` and `catalog/partition.h`)
    /// that pgrx does not bind. Both can raise an `ERROR`, so every call runs inside
    /// `pg_guard_ffi_boundary`: the error then unwinds through the Rust frames above it
    /// instead of longjmp'ing over their `Drop`s.
    fn find_all_inheritors(parent_rel_id: pg_sys::Oid, lockmode: pg_sys::LOCKMODE, numparents: *mut *mut pg_sys::List) -> *mut pg_sys::List;
    fn get_partition_ancestors(relid: pg_sys::Oid) -> *mut pg_sys::List;
}

/// Whether `index_oid` names a partitioned index, which has no storage of its own.
pub fn is_partitioned_index(index_oid: pg_sys::Oid) -> bool {
    (unsafe { pg_sys::get_rel_relkind(index_oid) as u8 }) == pg_sys::RELKIND_PARTITIONED_INDEX
}

/// The leaf partition indexes under `parent`, at any nesting depth. The parent itself and
/// any intermediate partitioned index have no storage of their own, so only the leaves
/// are returned, and a member left invalid by a failed `CREATE INDEX` is left out.
pub fn leaf_partition_indexes(
    parent: &PgSearchRelation,
) -> impl Iterator<Item = PgSearchRelation> + use<> {
    let parent_oid = parent.oid();
    let inheritors = unsafe {
        PgList::<pg_sys::Oid>::from_pg(pg_sys::submodules::ffi::pg_guard_ffi_boundary(|| {
            find_all_inheritors(
                parent_oid,
                pg_sys::AccessShareLock as pg_sys::LOCKMODE,
                std::ptr::null_mut(),
            )
        }))
    };
    inheritors
        .iter_oid()
        .filter(|&oid| {
            (unsafe { pg_sys::get_rel_relkind(oid) as u8 }) == pg_sys::RELKIND_INDEX
                && unsafe { pg_sys::get_index_isvalid(oid) }
        })
        .map(|oid| PgSearchRelation::with_lock(oid, pg_sys::AccessShareLock as _))
        .collect::<Vec<_>>()
        .into_iter()
}

/// The member of `parent_index_oid` attached to the partition `child_heap_oid`, however
/// deeply the partition is nested. `None` if the partition has no valid member of that
/// index (e.g. one left invalid by a failed `CREATE INDEX`).
pub fn partition_member_index(
    child_heap_oid: pg_sys::Oid,
    parent_index_oid: pg_sys::Oid,
) -> Option<PgSearchRelation> {
    let child_heap = PgSearchRelation::with_lock(child_heap_oid, pg_sys::AccessShareLock as _);
    child_heap
        .indices(pg_sys::AccessShareLock as _)
        .find(|index| {
            if !unsafe { pg_sys::get_index_isvalid(index.oid()) } {
                return false;
            }
            let index_oid = index.oid();
            let ancestors = unsafe {
                PgList::<pg_sys::Oid>::from_pg(pg_sys::submodules::ffi::pg_guard_ffi_boundary(
                    || get_partition_ancestors(index_oid),
                ))
            };
            ancestors
                .iter_oid()
                .any(|ancestor| ancestor == parent_index_oid)
        })
}
