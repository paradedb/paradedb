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

//! TODO: This module should be moved up to `pg_search/src/postgres/customscan` and
//! `ColumnarExec` should be renamed, as the functionality is now shared
//! between the base scan and aggregate scan.

pub mod columnar;

use crate::api::HashSet;
use crate::gucs;
use crate::index::fast_fields_helper::WhichFastField;
use crate::nodecast;
use crate::postgres::customscan::basescan::BaseScan;
use crate::postgres::customscan::basescan::privdat::PrivateData;
use crate::postgres::customscan::basescan::projections::score::is_score_func;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::node::CustomScanNodeExt;
use crate::postgres::customscan::pullup::{Access, ResTarget, SourceContext};
use crate::postgres::customscan::score_funcoids;
use crate::postgres::node::NodeExt;

use pgrx::{PgList, pg_sys};

/// Returns true if all variables in the expression belong to the current relation.
///
/// If an expression contains variables from other relations, it cannot be evaluated
/// by the current scan and must be evaluated by an upper node.
unsafe fn can_scan_evaluate_expr(rti: pg_sys::Index, expr: *mut pg_sys::Expr) -> bool {
    let vars = expr.collect_nodes::<pg_sys::Var>();
    // It is evaluatable by the scan if ALL vars belong to this scan (rti).
    // Note: Constants (no vars) are also considered evaluatable by the scan (locally).
    vars.iter().all(|var| (**var).varno as pg_sys::Index == rti)
}

/// Find all the fields that can be used as "fast fields", categorize them as [`WhichFastField`]s,
/// and return the list. If there are none, or one or more of the fields can't be used as a
/// "fast field", we return [`None`].
pub unsafe fn collect_fast_fields(
    target_list: *mut pg_sys::List,
    referenced_columns: &HashSet<pg_sys::AttrNumber>,
    context: &SourceContext,
    is_execution_time: bool,
) -> Vec<WhichFastField> {
    pullup_fast_fields(target_list, referenced_columns, context, is_execution_time)
        .filter(|fields| !fields.is_empty())
        .unwrap_or_default()
}

/// If all referenced fields in the given node can be fetched from the index as "fast fields",
/// return WhichFastFields covering them.
///
/// There are inline comments explaining the restrictions on what is supported.
pub unsafe fn pullup_fast_fields(
    node: *mut pg_sys::List,
    referenced_columns: &HashSet<pg_sys::AttrNumber>,
    context: &SourceContext,
    is_execution_time: bool,
) -> Option<Vec<WhichFastField>> {
    let mut matches = Vec::new();

    let tupdesc = context.tupdesc();

    // First collect all matches from the target list (standard behavior)
    let targetlist = PgList::<pg_sys::TargetEntry>::from_pg(node);

    // Process target list entries
    for te in targetlist.iter_ptr() {
        if (*te).resorigtbl != pg_sys::Oid::INVALID && (*te).resorigtbl != context.heaprel.oid() {
            continue;
        }

        let maybe_var = if pgrx::is_a((*te).expr.cast(), pg_sys::NodeTag::T_Var) {
            if let Some(var) = (*te).expr.find_single_node::<pg_sys::Var>() {
                if (*var).varno as i32 != context.heap_rti as i32 {
                    // We expect all Vars in the target list to be from the same range table as the
                    // index we're searching, so if we see a Var from a different range table, we skip it.
                    if is_execution_time {
                        // This is a sanity check to ensure that the target list is consistent with the
                        // index we're searching. As we're not supporting JOINs and Projection, at
                        // execution time (not planning time), we expect all Vars in the target list to
                        // be from the same range table as the index we're searching.
                        debug_assert_eq!(
                            (*var).varno as i32,
                            context.heap_rti as i32,
                            "Encountered a Var with a different range table index.",
                        );
                    }
                    continue;
                }
                Some(var)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(var) = maybe_var {
            let start_len = matches.len();
            if let Some(resolved) =
                context.resolve(ResTarget::Attno((*var).varattno), Access::Scalar)
            {
                matches.push(resolved.field);
            }
            // If the var was successfully added as a fast field, continue.
            // If not (e.g. source mismatch), fall through to expression matching.
            if matches.len() > start_len {
                continue;
            }
        }

        // Try to match complex expression (or Var with source mismatch) against index expressions
        if let Some(resolved) = context.resolve(
            ResTarget::Expr((*te).expr as *mut pg_sys::Node),
            Access::Scalar,
        ) {
            matches.push(resolved.field);
            continue;
        }

        if (*te)
            .expr
            .contains_score_for_relation(score_funcoids(), context.heap_rti)
        {
            // we can only pull up a score if the score is:
            // 1. directly a call to `pdb.score`, with no wrapping expression (i.e. `is_score_func`)
            // 2. a call to `pdb.score` inside of an expression which will be solved by a
            //    wrapping/outer scan because it contains vars from other relations.
            if is_score_func((*te).expr.cast(), context.heap_rti) {
                matches.push(WhichFastField::score());
                continue;
            } else if !can_scan_evaluate_expr(context.heap_rti, (*te).expr.cast()) {
                // The expression depends on other relations, so it will be evaluated by an upper node.
                // We just need to provide the score.
                matches.push(WhichFastField::score());
                continue;
            }
            // Fallthrough: expression is local but complex -> cannot use fast fields
            //
            // Complex expressions involving score which are not going to be solved in upper nodes are not supported.
            // See: https://github.com/paradedb/paradedb/issues/3978
        } else if pgrx::is_a((*te).expr.cast(), pg_sys::NodeTag::T_Aggref)
            || nodecast!(Const, T_Const, (*te).expr).is_some()
            || nodecast!(WindowFunc, T_WindowFunc, (*te).expr).is_some()
        {
            let create_resname = |base: &str, te: &pg_sys::TargetEntry| {
                let restype = (*te.expr).type_;
                let resno = te.resno;
                let isjunk = te.resjunk;
                format!("{base}(resno={resno}, restype={restype:?}, resjunk={isjunk})")
            };
            let resname = if (*te).resname.is_null() {
                create_resname("NONAME", &*te)
            } else {
                let fallback = create_resname("INVALID_NAME_STRING", &*te);
                unsafe {
                    std::ffi::CStr::from_ptr((*te).resname)
                        .to_str()
                        .unwrap_or(&fallback)
                }
                .to_string()
            };

            matches.push(WhichFastField::Junk(resname));
            continue;
        }
        // we only support Vars or our score function in the target list
        // Other nodes (e.g., T_SubPlan, T_FuncExpr, T_OpExpr, T_CaseExpr, T_PlaceHolderVar, etc.)
        // are not supported in FastFields yet.
        //
        // Casts of fields (e.g. `CAST(id AS TEXT)`) are not supported.
        // See: https://github.com/paradedb/paradedb/issues/3978
        return None;
    }

    // Now also consider all referenced columns from other parts of the query
    for &attno in referenced_columns {
        let start_len = matches.len();
        if let Some(resolved) = context.resolve(ResTarget::Attno(attno), Access::Scalar) {
            matches.push(resolved.field);
        }
        // If not added (e.g. because of source mismatch), try expression matching for this column.
        if matches.len() == start_len {
            // For columns referenced in other parts of the query (e.g. WHERE), we only have
            // the attribute number. To support cases where the column is indexed via an
            // expression (e.g. `col::pdb.literal`), we create a synthetic Var to match
            // against index expressions.
            if let Some(att) = tupdesc.get((attno - 1) as usize) {
                let dummy_var = pg_sys::makeVar(
                    context.heap_rti as _,
                    attno as pg_sys::AttrNumber,
                    att.atttypid,
                    att.atttypmod,
                    att.attcollation,
                    0,
                );

                if let Some(resolved) = context.resolve(
                    ResTarget::Expr(dummy_var as *mut pg_sys::Node),
                    Access::Scalar,
                ) {
                    matches.push(resolved.field);
                }
            }
        }
    }

    Some(matches)
}

fn fast_field_capable_prereqs(privdata: &PrivateData) -> bool {
    if privdata.referenced_columns_count() == 0 && privdata.target_list_len().unwrap_or(0) == 0 {
        return false;
    }

    let which_fast_fields = privdata.planned_which_fast_fields().as_ref().unwrap();

    if is_all_special_or_junk_fields(which_fast_fields) {
        // if all the fast fields we have are Junk fields, then we're not actually
        // projecting fast fields, and we're better off using a Normal scan.
        return false;
    }

    // Make sure all referenced columns are fast fields
    let referenced_columns_count = privdata.referenced_columns_count();

    // Count columns that we have fast fields for (excluding system/junk fields)
    let fast_field_column_count = which_fast_fields
        .iter()
        .filter(|ff| matches!(ff, WhichFastField::Named { .. }))
        .count();

    // If we're missing any columns, we can't use fast field execution
    if referenced_columns_count > fast_field_column_count {
        return false;
    }

    true
}

/// Check if we can use the Columnar execution method
pub fn is_columnar_capable(privdata: &PrivateData) -> bool {
    if !gucs::is_columnar_exec_enabled() {
        return false;
    }

    if !fast_field_capable_prereqs(privdata) {
        return false;
    }

    // We should only use Columnar if there is at least one named fast field, but fewer than the
    // configured column threshold.
    let which_fast_fields = privdata.planned_which_fast_fields().as_ref().unwrap();
    let named_field_count = which_fast_fields
        .iter()
        .filter(|wff| matches!(wff, WhichFastField::Named { .. }))
        .count();

    0 < named_field_count && named_field_count < gucs::columnar_exec_column_threshold()
}

pub fn is_all_special_or_junk_fields<'a>(
    which_fast_fields: impl IntoIterator<Item = &'a WhichFastField>,
) -> bool {
    which_fast_fields.into_iter().all(|ff| {
        matches!(
            ff,
            WhichFastField::Junk(_)
                | WhichFastField::TableOid
                | WhichFastField::Ctid
                | WhichFastField::Score(_)
        )
    })
}

/// Add nodes to `EXPLAIN` output to describe the "fast fields" being used by the query, if any
pub fn explain(state: &CustomScanStateWrapper<BaseScan>, explainer: &mut Explainer) {
    use crate::postgres::customscan::builders::custom_path::ExecMethodType;

    if let ExecMethodType::Columnar {
        which_fast_fields, ..
    } = &state.custom_state().exec_method_type
    {
        // Get all fast fields used, sorted for deterministic output
        let mut fields: Vec<_> = which_fast_fields
            .iter()
            .filter(|ff| matches!(ff, WhichFastField::Named { .. }))
            .map(|ff| ff.name())
            .collect();
        fields.sort();

        explainer.add_text("Columnar", fields.join(", "));
    }
}
