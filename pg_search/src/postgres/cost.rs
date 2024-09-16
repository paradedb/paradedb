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

use crate::index::SearchIndex;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::postgres::utils::relfilenode_from_index_oid;
use crate::writer::WriterDirectory;
use crate::{DEFAULT_STARTUP_COST, UNKNOWN_SELECTIVITY};
use pgrx::*;

#[allow(clippy::too_many_arguments)]
#[pg_guard(immutable, parallel_safe)]
pub unsafe extern "C" fn amcostestimate(
    _root: *mut pg_sys::PlannerInfo,
    path: *mut pg_sys::IndexPath,
    _loop_count: f64,
    index_startup_cost: *mut pg_sys::Cost,
    index_total_cost: *mut pg_sys::Cost,
    index_selectivity: *mut pg_sys::Selectivity,
    index_correlation: *mut f64,
    index_pages: *mut f64,
) {
    assert!(!path.is_null());
    assert!(!(*path).indexinfo.is_null());

    let indexrel = unsafe {
        PgRelation::with_lock(
            (*(*path).indexinfo).indexoid,
            pg_sys::AccessShareLock as pg_sys::LOCKMODE,
        )
    };
    let index_clauses = PgList::<pg_sys::IndexClause>::from_pg((*path).indexclauses);
    let reltuples = indexrel
        .heap_relation()
        .expect("index relation must have a valid corresponding heap relation")
        .reltuples()
        .unwrap_or(1.0) as f64;
    let page_estimate = {
        assert!(!indexrel.rd_options.is_null());
        let options = indexrel.rd_options as *mut SearchIndexCreateOptions;
        let database_oid = crate::MyDatabaseId();
        let index_oid = indexrel.oid().as_u32();
        let relfilenode = relfilenode_from_index_oid(index_oid).as_u32();
        let directory = WriterDirectory::from_oids(database_oid, index_oid, relfilenode);
        let search_index = SearchIndex::from_cache(
            &directory,
            &(*options)
                .get_uuid()
                .expect("`SearchIndexCreateOptions` must have a `uuid` property"),
        )
        .expect("should be able to retrieve a SearchIndex from internal cache");
        search_index.byte_size().unwrap_or(0) / pg_sys::BLCKSZ as u64
    };
    drop(indexrel);

    // start these at zero
    *index_selectivity = 0.0;
    *index_pages = 0.0;
    *index_total_cost = 0.0;

    // we output rows in random order relative to the heap's ctid ordering
    *index_correlation = 0.0;

    // it does cost a little bit for us to startup, which is spawning the tantivy query
    *index_startup_cost = DEFAULT_STARTUP_COST;

    // choose the smallest selectivity from the RestrictInfo clauses that have already done their estimations
    *index_selectivity = index_clauses
        .iter_ptr()
        .map(|clause| (*(*clause).rinfo).norm_selec)
        .filter(|norm| *norm > 0.0)
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Greater))
        .unwrap_or(UNKNOWN_SELECTIVITY);

    // use the selectivity to further estimate how many postgres pages we'd read,
    // if in fact we were based on Postgres' block storage
    *index_pages = *index_selectivity * page_estimate as f64;

    // total cost is just a hardcoded value of the cost to read a tuple from an index times the
    // estimated number of rows we expect to return
    *index_total_cost =
        *index_startup_cost + *index_selectivity * reltuples * pg_sys::cpu_index_tuple_cost;
}
