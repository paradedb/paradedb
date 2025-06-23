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

use crate::postgres::rel::PgSearchRelation;
use anyhow::{anyhow, Result};
use pgrx::{pg_sys, PgRelation, Spi};
use std::sync::Arc;

pub enum IndexKind {
    Index(crate::postgres::rel::PgSearchRelation),
    PartitionedIndex(Vec<crate::postgres::rel::PgSearchRelation>),
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
            pg_sys::RELKIND_PARTITIONED_INDEX => {
                // Locate the child index Oids, and open them.
                let child_array: Vec<pg_sys::Oid> = Spi::get_one_with_args(
                    "SELECT ARRAY_AGG(c.oid)
                     FROM pg_inherits i
                     JOIN pg_class c ON i.inhrelid = c.oid
                     WHERE i.inhparent = $1;",
                    &[index_relation.oid().into()],
                )
                .expect("failed to lookup child partitions")
                .unwrap();
                let child_relations = child_array
                    .into_iter()
                    .map(|oid| {
                        // TODO: Do these acquisitions need to be sorted?
                        PgSearchRelation::with_lock(oid, pg_sys::AccessShareLock as _)
                    })
                    .collect();
                Ok(IndexKind::PartitionedIndex(child_relations))
            }
            _ => Err(anyhow!("Expected to receive an index argument.")),
        }
    }

    ///
    /// Return an iterator over the partitions of this index, which might be
    /// of length 1 if it is not partitioned.
    ///
    pub fn partitions(self) -> Box<dyn Iterator<Item = crate::postgres::rel::PgSearchRelation>> {
        match self {
            Self::Index(rel) => Box::new(std::iter::once(rel)),
            Self::PartitionedIndex(rel) => Box::new(rel.into_iter()),
        }
    }
}
