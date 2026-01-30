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

//! EXPLAIN output helpers for JoinScan.
//!
//! This module provides formatting utilities for displaying JoinScan plans
//! in PostgreSQL's EXPLAIN output, including expression tree formatting
//! and column name resolution.

use super::build::{JoinCSClause, JoinLevelExpr};

use crate::postgres::customscan::explain::ExplainFormat;
use crate::postgres::deparse::node_to_string_fallback;
use pgrx::pg_sys;

/// Format a PostgreSQL expression node for EXPLAIN output.
/// Returns a human-readable description of the expression.
///
/// Note: This uses nodeToString which produces a debug representation.
/// For proper SQL deparsing with table/column names, we'd need a PlannerContext
/// which isn't available during EXPLAIN. The debug output is still useful for
/// understanding the expression structure.
pub(super) unsafe fn format_expr_for_explain(node: *mut pg_sys::Node) -> String {
    node_to_string_fallback(node)
}

/// Get the column name for an attribute, with fallback to "relname.attno" if lookup fails.
pub(super) fn get_attname_safe(
    heaprelid: Option<pg_sys::Oid>,
    attno: pg_sys::AttrNumber,
    rel_name: &str,
) -> String {
    let Some(oid) = heaprelid else {
        return format!("{}.{}", rel_name, attno);
    };

    unsafe {
        let attname_ptr = pg_sys::get_attname(oid, attno, true); // missing_ok = true
        if attname_ptr.is_null() {
            format!("{}.{}", rel_name, attno)
        } else {
            let attname = std::ffi::CStr::from_ptr(attname_ptr)
                .to_str()
                .unwrap_or("?");
            format!("{}.{}", rel_name, attname)
        }
    }
}

/// Format a join-level expression tree for EXPLAIN output.
pub(super) fn format_join_level_expr(expr: &JoinLevelExpr, join_clause: &JoinCSClause) -> String {
    match expr {
        JoinLevelExpr::SingleTablePredicate {
            source_idx,
            predicate_idx,
        } => {
            let label = if *source_idx == 0 { "outer" } else { "inner" };
            if let Some(pred) = join_clause.join_level_predicates.get(*predicate_idx) {
                format!("{}:{}", label, pred.query.explain_format())
            } else {
                format!("{}:?", label)
            }
        }
        JoinLevelExpr::MultiTablePredicate { predicate_idx } => {
            if let Some(cond) = join_clause.multi_table_predicates.get(*predicate_idx) {
                format!("heap:{}", cond.description)
            } else {
                "heap:?".to_string()
            }
        }
        JoinLevelExpr::And(children) => {
            let parts: Vec<_> = children
                .iter()
                .map(|c| format_join_level_expr(c, join_clause))
                .collect();
            if parts.len() == 1 {
                parts.into_iter().next().unwrap()
            } else {
                format!("({})", parts.join(" AND "))
            }
        }
        JoinLevelExpr::Or(children) => {
            let parts: Vec<_> = children
                .iter()
                .map(|c| format_join_level_expr(c, join_clause))
                .collect();
            if parts.len() == 1 {
                parts.into_iter().next().unwrap()
            } else {
                format!("({})", parts.join(" OR "))
            }
        }
        JoinLevelExpr::Not(child) => {
            format!("NOT {}", format_join_level_expr(child, join_clause))
        }
    }
}
