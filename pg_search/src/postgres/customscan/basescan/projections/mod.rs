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

pub mod score;
pub mod snippet;
pub mod window_agg;

use crate::api::operator::is_anyelement_search_opoid;
use crate::gucs;
use crate::nodecast;
use pgrx::pg_sys::expression_tree_walker;
use pgrx::{PgList, pg_guard, pg_sys};
use std::ffi::CStr;
use std::ptr::addr_of_mut;

const REPORT_URL: &str = "https://github.com/paradedb/paradedb/issues/new/choose";

/// `pdb.score` / `pdb.snippet*` are placeholders rewritten by a ParadeDB custom
/// scan. Reaching here means the scan did not take over the query. The cause is
/// not always a missing search operator — join/aggregate scans may be disabled,
/// or the query shape may be unsupported (for example a top-level join without
/// LIMIT).
pub(crate) fn placeholder_not_rewritten() -> ! {
    if !gucs::enable_custom_scan() {
        pgrx::error!(
            "pdb.score() / pdb.snippet() cannot be evaluated because paradedb.enable_custom_scan is off. \
             SET paradedb.enable_custom_scan = on"
        );
    }

    let has_operator = unsafe { query_has_search_operator() };

    if has_operator && !gucs::enable_join_custom_scan() {
        pgrx::error!(
            "pdb.score() / pdb.snippet() cannot be evaluated because paradedb.enable_join_custom_scan is off. \
             SET paradedb.enable_join_custom_scan = on"
        );
    }

    if has_operator && !gucs::enable_aggregate_custom_scan() {
        pgrx::error!(
            "pdb.score() / pdb.snippet() cannot be evaluated because paradedb.enable_aggregate_custom_scan is off. \
             SET paradedb.enable_aggregate_custom_scan = on"
        );
    }

    if has_operator {
        pgrx::error!(
            "pdb.score() / pdb.snippet() cannot be evaluated for this query shape. \
             A common cause is a top-level join without LIMIT, which prevents JoinScan. \
             If this looks like a bug, please report it at {REPORT_URL}"
        );
    }

    pgrx::error!(
        "pdb.score() / pdb.snippet() require a ParadeDB search operator (@@@, |||, &&&, ===, or ###) in the WHERE clause. \
         If this query already has one, please report it at {REPORT_URL}"
    )
}

unsafe fn query_has_search_operator() -> bool {
    if source_text_has_search_operator(debug_query_text())
        || source_text_has_search_operator(active_portal_source_text())
    {
        return true;
    }
    planned_stmt_has_search_operator()
}

fn source_text_has_search_operator(sql: Option<&str>) -> bool {
    let Some(sql) = sql else {
        return false;
    };
    sql.contains("@@@")
        || sql.contains("|||")
        || sql.contains("&&&")
        || sql.contains("===")
        || sql.contains("###")
}

unsafe fn debug_query_text() -> Option<&'static str> {
    let ptr = pg_sys::debug_query_string;
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

unsafe fn active_portal_source_text() -> Option<&'static str> {
    let portal = pg_sys::ActivePortal;
    if portal.is_null() {
        return None;
    }
    let ptr = (*portal).sourceText;
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

unsafe fn planned_stmt_has_search_operator() -> bool {
    let portal = pg_sys::ActivePortal;
    if portal.is_null() {
        return false;
    }
    let query_desc = (*portal).queryDesc;
    if query_desc.is_null() {
        return false;
    }
    let pstmt = (*query_desc).plannedstmt;
    if pstmt.is_null() {
        return false;
    }
    if plan_has_search_operator((*pstmt).planTree) {
        return true;
    }
    for plan in PgList::<pg_sys::Plan>::from_pg((*pstmt).subplans).iter_ptr() {
        if plan_has_search_operator(plan) {
            return true;
        }
    }
    false
}

unsafe fn plan_has_search_operator(plan: *mut pg_sys::Plan) -> bool {
    if plan.is_null() {
        return false;
    }
    if list_has_search_operator((*plan).targetlist) || list_has_search_operator((*plan).qual) {
        return true;
    }

    match (*plan).type_ {
        pg_sys::NodeTag::T_NestLoop
        | pg_sys::NodeTag::T_MergeJoin
        | pg_sys::NodeTag::T_HashJoin => {
            let join = plan.cast::<pg_sys::Join>();
            if list_has_search_operator((*join).joinqual) {
                return true;
            }
        }
        _ => {}
    }

    plan_has_search_operator((*plan).lefttree) || plan_has_search_operator((*plan).righttree)
}

unsafe fn list_has_search_operator(list: *mut pg_sys::List) -> bool {
    for node in PgList::<pg_sys::Node>::from_pg(list).iter_ptr() {
        if expr_has_search_operator(node) {
            return true;
        }
    }
    false
}

unsafe fn expr_has_search_operator(node: *mut pg_sys::Node) -> bool {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }
        if let Some(opexpr) = nodecast!(OpExpr, T_OpExpr, node)
            && is_anyelement_search_opoid((*opexpr).opno)
        {
            return true;
        }
        if let Some(saop) = nodecast!(ScalarArrayOpExpr, T_ScalarArrayOpExpr, node)
            && is_anyelement_search_opoid((*saop).opno)
        {
            return true;
        }
        if let Some(ri) = nodecast!(RestrictInfo, T_RestrictInfo, node) {
            return expr_has_search_operator((*ri).clause.cast());
        }
        expression_tree_walker(node, Some(walker), data)
    }

    if node.is_null() {
        return false;
    }
    let mut dummy = ();
    walker(node, addr_of_mut!(dummy).cast())
}
