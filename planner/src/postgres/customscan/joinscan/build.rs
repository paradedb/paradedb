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
#![allow(clippy::missing_safety_doc, unsafe_op_in_unsafe_fn)]

use crate::postgres::customscan::range_table::{get_plain_relation_relid, get_rte};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use pgrx::pg_sys;

/// Look up a base relation's OID, alias, and optional BM25 index by RTI.
///
/// Shared between JoinScan's `collect_join_sources_base_rel` and AggregateScan's
/// `collect_join_agg_sources`. Returns `None` if the RTI doesn't point to a
/// plain relation (e.g., subquery, CTE).
pub unsafe fn lookup_base_rel_info(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
) -> Option<(pg_sys::Oid, Option<String>, Option<PgSearchRelation>)> {
    let rte = get_rte(
        (*root).simple_rel_array_size as usize,
        (*root).simple_rte_array,
        rti,
    )?;

    let relid = get_plain_relation_relid(rte)?;

    let alias = if !(*rte).eref.is_null() && !(*(*rte).eref).aliasname.is_null() {
        std::ffi::CStr::from_ptr((*(*rte).eref).aliasname)
            .to_str()
            .ok()
            .map(|s| s.to_string())
    } else {
        None
    };

    let bm25_index = rel_get_bm25_index(relid).map(|(_, idx)| idx);

    Some((relid, alias, bm25_index))
}
