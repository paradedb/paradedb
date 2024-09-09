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
    let path = path.as_mut().expect("`path` must not be null`");
    let indexinfo = path
        .indexinfo
        .as_ref()
        .expect("`indexinfo` must not be null`");
    let index_relation = unsafe {
        PgRelation::with_lock(
            indexinfo.indexoid,
            pg_sys::AccessShareLock as pg_sys::LOCKMODE,
        )
    };
    let heap_relation = index_relation
        .heap_relation()
        .expect("index relation must have a valid corresponding heap relation");

    *index_correlation = 0.0;
    *index_startup_cost = 0.0;
    *index_pages = 0.0;
    *index_total_cost = 0.0;
    *index_selectivity = 0.0;

    let index_clauses = PgList::<pg_sys::IndexClause>::from_pg(path.indexclauses);

    for clause in index_clauses.iter_ptr() {
        let ri = (*clause).rinfo;

        if *index_selectivity == 0.0 {
            *index_selectivity = (*ri).norm_selec;
        } else {
            *index_selectivity = (*ri).norm_selec.min(*index_selectivity);
        }
    }

    let reltuples = heap_relation.reltuples().unwrap_or(1f32) as f64;
    *index_total_cost += *index_selectivity * reltuples * pg_sys::cpu_index_tuple_cost;

    path.path.rows = *index_selectivity * reltuples;
}
