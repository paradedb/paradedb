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

//! Discover dependencies introduced by native SQL-function inlining without
//! replacing the real function call or changing PostgreSQL's optimization.

use super::{PermissionInfo, ReadSet, annotate_query, declare_relation_reads, permission_info};
use crate::api::HashMap;
use crate::nodecast;
use crate::postgres::catalog::is_sql_function;
use crate::postgres::rel::PgSearchRelation;
use pgrx::{PgBox, PgList, pg_guard, pg_sys};

pub(super) unsafe fn is_scalar_sql_call(node: *mut pg_sys::Node) -> bool {
    nodecast!(FuncExpr, T_FuncExpr, node)
        .is_some_and(|call| !(*call).funcretset && is_sql_function((*call).funcid))
}

pub(super) unsafe fn annotate_srf(
    query: *mut pg_sys::Query,
    rte: *mut pg_sys::RangeTblEntry,
    parents: &[*mut pg_sys::Query],
) -> bool {
    let copy = pg_sys::copyObjectImpl(rte.cast()).cast::<pg_sys::RangeTblEntry>();
    (*copy).functions =
        pg_sys::eval_const_expressions(std::ptr::null_mut(), (*copy).functions.cast()).cast();
    // The native expander needs a global for its analysis dependencies. The
    // real planner still sees the original call and records its own dependency.
    let global = PgBox::<pg_sys::PlannerGlobal>::alloc_node(pg_sys::NodeTag::T_PlannerGlobal);
    let mut root = PgBox::<pg_sys::PlannerInfo>::alloc_node(pg_sys::NodeTag::T_PlannerInfo);
    root.glob = global.as_ptr();
    root.parse = query;
    let expanded = pg_sys::inline_set_returning_function(root.as_ptr(), copy);
    if expanded.is_null() {
        return false;
    }
    let before: HashMap<_, _> = entries(expanded)
        .into_iter()
        .map(|info| (info, pg_sys::bms_copy((*info).selectedCols)))
        .collect();
    let depends_on_role =
        annotate_query(expanded, parents) || global.dependsOnRole || (*expanded).hasRowSecurity;
    for info in entries(expanded) {
        let added = pg_sys::bms_difference(
            (*info).selectedCols,
            before.get(&info).copied().unwrap_or(std::ptr::null_mut()),
        );
        if pg_sys::bms_num_members(added) != 0 {
            let relation = PgSearchRelation::with_lock((*info).relid, pg_sys::AccessShareLock as _);
            declare_relation_reads(
                query,
                &relation,
                (*info).checkAsUser,
                &ReadSet { attributes: added },
            );
        }
        pg_sys::bms_free(added);
    }
    for columns in before.into_values() {
        pg_sys::bms_free(columns);
    }
    depends_on_role
}

unsafe fn entries(query: *mut pg_sys::Query) -> Vec<*mut PermissionInfo> {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }
        if let Some(query) = nodecast!(Query, T_Query, node) {
            let entries = &mut *context.cast::<Vec<*mut PermissionInfo>>();
            for rte in PgList::<pg_sys::RangeTblEntry>::from_pg((*query).rtable).iter_ptr() {
                if let Some(info) = permission_info(query, rte)
                    && (*info).relid != pg_sys::InvalidOid
                {
                    entries.push(info);
                }
            }
            pg_sys::query_tree_walker(query, Some(walker), context, 0)
        } else {
            pg_sys::expression_tree_walker(node, Some(walker), context)
        }
    }
    let mut entries = Vec::<*mut PermissionInfo>::new();
    walker(query.cast(), std::ptr::from_mut(&mut entries).cast());
    entries
}
