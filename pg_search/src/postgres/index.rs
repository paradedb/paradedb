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

use crate::index::{SearchIndex, SearchIndexError, WriterDirectory};
use pgrx::{pg_sys, PgRelation};

/// Open the underlying [`SearchIndex`] for the specified Postgres index relation
pub fn open_search_index(
    index_relation: &PgRelation,
) -> anyhow::Result<SearchIndex, SearchIndexError> {
    let database_oid = unsafe { pg_sys::MyDatabaseId };
    let index_oid = index_relation.oid();
    let relfilenode = relfilenode_from_pg_relation(index_relation);
    let directory = WriterDirectory::from_oids(
        database_oid.as_u32(),
        index_oid.as_u32(),
        relfilenode.as_u32(),
    );
    SearchIndex::from_disk(&directory)
}

/// Retrieves the `relfilenode` from a `PgRelation`, handling PostgreSQL version differences.
#[inline(always)]
pub fn relfilenode_from_pg_relation(index_relation: &PgRelation) -> pg_sys::Oid {
    #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
    {
        index_relation.rd_node.relNode
    }
    #[cfg(any(feature = "pg16", feature = "pg17"))]
    {
        index_relation.rd_locator.relNumber
    }
}
