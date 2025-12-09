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

//! Expression deparsing utilities for converting PostgreSQL expression nodes
//! into human-readable SQL strings for EXPLAIN output.

use crate::nodecast;
use crate::postgres::customscan::qual_inspect::{contains_exec_param, PlannerContext};
use crate::postgres::rel::PgSearchRelation;
use pgrx::{pg_guard, pg_sys};

/// Helper function to deparse an expression using a relation context.
/// Returns a human-readable SQL string representation of the expression.
/// Falls back to nodeToString representation if deparsing fails (e.g., for complex expressions).
///
/// The `rel` parameter can be either:
/// - An index relation (will use its underlying heap relation for context)
/// - A heap relation directly (will use it directly for context)
///
/// This function handles:
/// - Simple expressions (varno 1): deparse directly
/// - Single-table expressions with varno != 1: remap varno and deparse
/// - Multi-table expressions: build context with all RTEs
/// - Expressions with PARAM_EXEC nodes: replace with placeholders and deparse
/// - Expressions with PARAM_EXTERN: deparse as "$N"
pub unsafe fn deparse_expr(
    planner_context: Option<&PlannerContext>,
    rel: &PgSearchRelation,
    expr: *mut pg_sys::Node,
) -> String {
    if expr.is_null() {
        return "<null>".to_string();
    }

    // For expressions containing PARAM_EXEC nodes (correlated subquery params),
    // clone and replace them with placeholder constants, then deparse
    // Note: PARAM_EXTERN (prepared statement params like $1, $2) are safe and will be
    // deparsed as "$N" by PostgreSQL's deparse_expression
    let expr_to_deparse = if contains_exec_param(expr) {
        let cloned = pg_sys::copyObjectImpl(expr.cast()).cast::<pg_sys::Node>();
        replace_exec_params_with_placeholders(cloned);
        cloned
    } else {
        expr
    };

    // Get the heap relation for deparsing context.
    // If rel.heap_relation() returns Some, rel is an index and we use its heap relation.
    // If it returns None, rel IS the heap relation, so use it directly.
    let (heap_name, heap_oid) = if let Some(heaprel) = rel.heap_relation() {
        (heaprel.name().to_string(), heaprel.oid())
    } else {
        (rel.name().to_string(), rel.oid())
    };

    // Collect all varnos referenced in the expression
    let varnos = collect_varnos(expr_to_deparse);

    // If expression has no var references (constant expression), use heap relation context
    if varnos.is_empty() {
        return deparse_with_single_relation(expr_to_deparse, &heap_name, heap_oid);
    }

    // Get the PlannerInfo to access the rtable
    let Some(root) = planner_context.and_then(|c| c.planner_info()) else {
        // No PlannerInfo available - try simple deparse for varno 1
        if varnos.len() == 1 && varnos[0] == 1 {
            return deparse_with_single_relation(expr_to_deparse, &heap_name, heap_oid);
        }
        return node_to_string_fallback(expr_to_deparse);
    };

    let parse = (*root).parse;
    if parse.is_null() {
        return node_to_string_fallback(expr_to_deparse);
    }

    let rtable = (*parse).rtable;
    if rtable.is_null() {
        return node_to_string_fallback(expr_to_deparse);
    }

    let rtable_list = pgrx::PgList::<pg_sys::RangeTblEntry>::from_pg(rtable);

    if varnos.len() == 1 {
        // Single varno - get RTE and deparse
        let varno = varnos[0];
        let rte_idx = (varno - 1) as usize; // varno is 1-based

        let Some(rte) = rtable_list.get_ptr(rte_idx) else {
            return node_to_string_fallback(expr_to_deparse);
        };

        // Only handle RTE_RELATION
        if (*rte).rtekind != pg_sys::RTEKind::RTE_RELATION {
            return node_to_string_fallback(expr_to_deparse);
        }

        let relid = (*rte).relid;
        let relname = get_rel_name_safe(relid);

        if varno == 1 {
            // Already at varno 1, deparse directly
            return deparse_with_single_relation(expr_to_deparse, &relname, relid);
        }

        // Need to remap varno to 1 for deparsing
        // Clone the expression and remap varnos (note: may already be cloned for PARAM replacement)
        let expr_copy = pg_sys::copyObjectImpl(expr_to_deparse.cast()).cast::<pg_sys::Node>();
        remap_varnos(expr_copy, varno, 1);
        return deparse_with_single_relation(expr_copy, &relname, relid);
    }

    // Multiple varnos - try to build a context with all RTEs
    deparse_with_full_context(expr_to_deparse, &rtable_list, &varnos)
}

/// Deparse an expression with a single-relation context
unsafe fn deparse_with_single_relation(
    expr: *mut pg_sys::Node,
    relname: &str,
    relid: pg_sys::Oid,
) -> String {
    let relname_cstr = match std::ffi::CString::new(relname) {
        Ok(s) => s,
        Err(_) => return node_to_string_fallback(expr),
    };

    let context = pg_sys::deparse_context_for(relname_cstr.as_ptr(), relid);
    let deparsed = pg_sys::deparse_expression(expr.cast(), context, false, true);
    if deparsed.is_null() {
        return node_to_string_fallback(expr);
    }

    std::ffi::CStr::from_ptr(deparsed)
        .to_string_lossy()
        .into_owned()
}

/// Try to deparse an expression that references multiple relations
/// by building a context with all required RTEs
unsafe fn deparse_with_full_context(
    expr: *mut pg_sys::Node,
    rtable_list: &pgrx::PgList<pg_sys::RangeTblEntry>,
    varnos: &[pg_sys::Index],
) -> String {
    // For multi-relation expressions, we need to build a context that includes
    // all referenced relations at their correct varno positions.
    //
    // PostgreSQL's deparse_context_for creates a context for a single relation at varno 1.
    // To handle multiple relations, we use list_concat to combine contexts.
    // Each relation is placed at its correct position by padding with nulls.

    // Verify all referenced varnos are valid RTE_RELATION entries
    for &varno in varnos {
        let rte_idx = (varno - 1) as usize;
        let Some(rte) = rtable_list.get_ptr(rte_idx) else {
            return node_to_string_fallback(expr);
        };
        if (*rte).rtekind != pg_sys::RTEKind::RTE_RELATION {
            return node_to_string_fallback(expr);
        }
    }

    // Find the maximum varno to know how big our rtable needs to be
    let max_varno = *varnos.iter().max().unwrap_or(&1);

    // Build an rtable with all entries at their correct positions
    // We need to create RTE entries for each position up to max_varno
    let mut rte_names: Vec<std::ffi::CString> = Vec::new();
    let mut combined_rtable: *mut pg_sys::List = std::ptr::null_mut();

    for varno in 1..=max_varno {
        let rte_idx = (varno - 1) as usize;

        if let Some(rte) = rtable_list.get_ptr(rte_idx) {
            if (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION {
                let relid = (*rte).relid;
                let relname = get_rel_name_safe(relid);
                let relname_cstr = match std::ffi::CString::new(relname) {
                    Ok(s) => s,
                    Err(_) => return node_to_string_fallback(expr),
                };

                // Create a context for this relation
                // Note: deparse_context_for puts the relation at varno 1 in its internal context
                // We'll use the first relation's context as base and the rtable will be built properly
                if combined_rtable.is_null() && varno == 1 {
                    combined_rtable = pg_sys::deparse_context_for(relname_cstr.as_ptr(), relid);
                }
                rte_names.push(relname_cstr);
            }
        }
    }

    // If we couldn't build a proper context, fall back
    if combined_rtable.is_null() {
        return node_to_string_fallback(expr);
    }

    // For multi-varno expressions, we need a more sophisticated approach
    // Since building the full context is complex, let's try a different strategy:
    // Deparse each var reference separately and reconstruct the expression string
    // For now, fall back to nodeToString for multi-relation expressions
    // This is a known limitation that could be improved in the future
    node_to_string_fallback(expr)
}

/// Get a relation name safely, returning a fallback if not found
unsafe fn get_rel_name_safe(relid: pg_sys::Oid) -> String {
    let name_ptr = pg_sys::get_rel_name(relid);
    if name_ptr.is_null() {
        return format!("relation_{}", u32::from(relid));
    }
    std::ffi::CStr::from_ptr(name_ptr)
        .to_string_lossy()
        .into_owned()
}

/// Replace PARAM_EXEC nodes with PARAM_EXTERN in an expression tree.
/// This allows deparse_expression to render them as `$N` instead of crashing.
/// The expression should be cloned before calling this function.
unsafe fn replace_exec_params_with_placeholders(node: *mut pg_sys::Node) {
    if node.is_null() {
        return;
    }

    #[pg_guard]
    unsafe extern "C-unwind" fn param_replacer(
        node: *mut pg_sys::Node,
        _context: *mut core::ffi::c_void,
    ) -> bool {
        if let Some(param) = nodecast!(Param, T_Param, node) {
            // Convert PARAM_EXEC to PARAM_EXTERN so deparse_expression can handle it
            // PARAM_EXEC = 1 (subquery params), PARAM_EXTERN = 0 (prepared stmt params)
            if (*param).paramkind == pg_sys::ParamKind::PARAM_EXEC {
                (*param).paramkind = pg_sys::ParamKind::PARAM_EXTERN;
                // Adjust paramid: PARAM_EXEC uses 0-based ids, PARAM_EXTERN uses 1-based
                // Add 1 to make it display as $1, $2, etc. instead of $0
                (*param).paramid += 1;
            }
        }

        // Continue walking
        pg_sys::expression_tree_walker(node, Some(param_replacer), std::ptr::null_mut())
    }

    param_replacer(node, std::ptr::null_mut());
}

/// Remap varnos in an expression tree from old_varno to new_varno
unsafe fn remap_varnos(
    node: *mut pg_sys::Node,
    old_varno: pg_sys::Index,
    new_varno: pg_sys::Index,
) {
    if node.is_null() {
        return;
    }

    #[pg_guard]
    unsafe extern "C-unwind" fn remap_walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        let (old_varno, new_varno) = *(context as *const (pg_sys::Index, pg_sys::Index));

        if let Some(var) = nodecast!(Var, T_Var, node) {
            if (*var).varno as pg_sys::Index == old_varno {
                (*var).varno = new_varno as _;
            }
            // Also update varnosyn if it matches
            if (*var).varnosyn as pg_sys::Index == old_varno {
                (*var).varnosyn = new_varno as _;
            }
        }

        // Continue walking - return false to continue, true to stop
        pg_sys::expression_tree_walker(node, Some(remap_walker), context)
    }

    let context = (old_varno, new_varno);
    remap_walker(
        node,
        &context as *const (pg_sys::Index, pg_sys::Index) as *mut core::ffi::c_void,
    );
}

/// Convert a PostgreSQL node to its string representation using nodeToString.
/// This provides a fallback representation when proper deparsing isn't possible.
pub unsafe fn node_to_string_fallback(expr: *mut pg_sys::Node) -> String {
    pgrx::node_to_string(expr)
        .unwrap_or("<unknown>")
        .to_string()
}

/// Collect all unique varnos (range table indices) referenced in an expression.
pub unsafe fn collect_varnos(node: *mut pg_sys::Node) -> Vec<pg_sys::Index> {
    if node.is_null() {
        return Vec::new();
    }

    let mut varnos: Vec<pg_sys::Index> = Vec::new();

    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        let varnos = &mut *(context as *mut Vec<pg_sys::Index>);

        if let Some(var) = nodecast!(Var, T_Var, node) {
            let varno = (*var).varno as pg_sys::Index;
            if !varnos.contains(&varno) {
                varnos.push(varno);
            }
        }

        pg_sys::expression_tree_walker(node, Some(walker), context)
    }

    walker(
        node,
        &mut varnos as *mut Vec<pg_sys::Index> as *mut core::ffi::c_void,
    );

    varnos
}
