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

use crate::postgres::build::is_bm25_index;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::pg_search_extension_installed;
use pgrx::{PgList, pg_guard, pg_sys};
use std::ptr::null_mut;

pub(crate) fn register_hook() {
    static mut PREV_HOOK: pg_sys::set_rel_pathlist_hook_type = None;

    #[pg_guard]
    unsafe extern "C-unwind" fn callback(
        root: *mut pg_sys::PlannerInfo,
        rel: *mut pg_sys::RelOptInfo,
        rti: pg_sys::Index,
        rte: *mut pg_sys::RangeTblEntry,
    ) {
        unsafe {
            if let Some(previous_hook) = PREV_HOOK {
                previous_hook(root, rel, rti, rte);
            }
            if !pg_sys::enable_indexonlyscan || !pg_search_extension_installed() {
                return;
            }

            let mut candidates = Vec::new();
            for path in PgList::<pg_sys::Path>::from_pg((*rel).pathlist).iter_ptr() {
                if (*path).type_ != pg_sys::NodeTag::T_IndexPath
                    || (*path).pathtype != pg_sys::NodeTag::T_IndexScan
                    || !(*path).param_info.is_null()
                    || (*path).parallel_aware
                {
                    continue;
                }
                let path = &*path.cast::<pg_sys::IndexPath>();
                let index = &*path.indexinfo;
                if index.hypothetical
                    || !is_bm25_index(&PgSearchRelation::open(index.indexoid))
                    || !covers_required_attributes(path)
                {
                    continue;
                }

                candidates.push(pg_sys::create_index_path(
                    root,
                    path.indexinfo,
                    path.indexclauses,
                    path.indexorderbys,
                    path.indexorderbycols,
                    path.path.pathkeys,
                    path.indexscandir,
                    true,
                    null_mut(),
                    1.0,
                    false,
                ));
            }

            // add_path can remove existing paths, so finish reading the pathlist first.
            for candidate in candidates {
                pg_sys::add_path(rel, candidate.cast());
            }
        }
    }

    unsafe {
        PREV_HOOK = pg_sys::set_rel_pathlist_hook;
        pg_sys::set_rel_pathlist_hook = Some(callback);
    }
}

// Like PostgreSQL's check_index_only, but exclude filters enforced by exact index conditions.
unsafe fn covers_required_attributes(path: &pg_sys::IndexPath) -> bool {
    unsafe {
        let index = &*path.indexinfo;
        let rel = &*index.rel;
        let mut required = null_mut();
        pg_sys::pull_varattnos((*rel.reltarget).exprs.cast(), rel.relid, &mut required);

        for restriction in PgList::<pg_sys::RestrictInfo>::from_pg(index.indrestrictinfo).iter_ptr()
        {
            if !(*restriction).pseudoconstant
                && !pg_sys::is_redundant_with_indexclauses(restriction, path.indexclauses)
            {
                pg_sys::pull_varattnos((*restriction).clause.cast(), rel.relid, &mut required);
            }
        }

        // Index-only recheck expressions must also reference returnable columns.
        for clause in PgList::<pg_sys::IndexClause>::from_pg(path.indexclauses).iter_ptr() {
            for qual in PgList::<pg_sys::RestrictInfo>::from_pg((*clause).indexquals).iter_ptr() {
                pg_sys::pull_varattnos((*qual).clause.cast(), rel.relid, &mut required);
            }
        }

        let mut returnable = null_mut();
        for column in 0..index.ncolumns as usize {
            let attno = *index.indexkeys.add(column);
            if attno > 0 && *index.canreturn.add(column) {
                returnable = pg_sys::bms_add_member(
                    returnable,
                    attno - pg_sys::FirstLowInvalidHeapAttributeNumber,
                );
            }
        }

        let covered = pg_sys::bms_is_subset(required, returnable);
        pg_sys::bms_free(required);
        pg_sys::bms_free(returnable);
        covered
    }
}
