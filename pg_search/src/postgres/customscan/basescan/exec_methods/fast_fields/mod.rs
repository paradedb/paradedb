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

pub mod mixed;

use crate::api::FieldName;
use crate::api::HashSet;
use crate::gucs;
use crate::index::fast_fields_helper::{FastFieldType, WhichFastField};
use crate::nodecast;
use crate::postgres::customscan::basescan::privdat::PrivateData;
use crate::postgres::customscan::basescan::projections::score::{is_score_func, uses_scores};
use crate::postgres::customscan::basescan::BaseScan;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::score_funcoids;

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::strip_tokenizer_cast;
use crate::postgres::var::{find_one_var, find_one_var_and_fieldname, find_vars, VarContext};
use crate::schema::{FieldSource, SearchIndexSchema};

use pgrx::{pg_sys, PgList, PgTupleDesc};

/// Returns true if all variables in the expression belong to the current relation.
///
/// If an expression contains variables from other relations, it cannot be evaluated
/// by the current scan and must be evaluated by an upper node.
unsafe fn can_scan_evaluate_expr(rti: pg_sys::Index, expr: *mut pg_sys::Expr) -> bool {
    let vars = find_vars(expr as *mut pg_sys::Node);
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
    rti: pg_sys::Index,
    heaprel: &PgSearchRelation,
    index: &PgSearchRelation,
    is_execution_time: bool,
) -> Vec<WhichFastField> {
    pullup_fast_fields(
        target_list,
        referenced_columns,
        heaprel,
        index,
        rti,
        is_execution_time,
    )
    .filter(|fields| !fields.is_empty())
    .unwrap_or_default()
}

fn fast_field_type_for_pullup(base_oid: pg_sys::Oid, is_array: bool) -> Option<FastFieldType> {
    if is_array {
        return None;
    }
    match base_oid {
        pg_sys::TEXTOID | pg_sys::VARCHAROID | pg_sys::UUIDOID => Some(FastFieldType::String),
        pg_sys::BOOLOID => Some(FastFieldType::Bool),
        pg_sys::DATEOID
        | pg_sys::TIMEOID
        | pg_sys::TIMESTAMPOID
        | pg_sys::TIMESTAMPTZOID
        | pg_sys::TIMETZOID => Some(FastFieldType::Date),
        pg_sys::FLOAT4OID | pg_sys::FLOAT8OID => Some(FastFieldType::Float64),
        pg_sys::INT2OID | pg_sys::INT4OID | pg_sys::INT8OID => Some(FastFieldType::Int64),
        _ => {
            // This fast field type is supported for pushdown of queries, but not for
            // rendering via fast field execution.
            //
            // JSON/JSONB are excluded because fast fields do not contain the
            // full content of the JSON in a way that we can easily render:
            // rather, the individual fields are exploded out into dynamic
            // columns.
            //
            // NUMERIC is excluded because we do not store the original
            // precision/scale in the index, so we cannot safely reconstruct the
            // value without potentially losing precision. See:
            // https://github.com/paradedb/paradedb/issues/2968
            None
        }
    }
}

// Helper function to process an attribute number and add a fast field if appropriate
fn collect_fast_field_try_for_attno(
    attno: i32,
    matches: &mut Vec<WhichFastField>,
    tupdesc: &PgTupleDesc<'_>,
    heaprel: &PgSearchRelation,
    index: &PgSearchRelation,
    fieldname: Option<&FieldName>,
) -> bool {
    match attno {
        // any of these mean we can't use fast fields
        pg_sys::MinTransactionIdAttributeNumber
        | pg_sys::MaxTransactionIdAttributeNumber
        | pg_sys::MinCommandIdAttributeNumber
        | pg_sys::MaxCommandIdAttributeNumber => return false,

        // these aren't _exactly_ fast fields, but we do have the information
        // readily available during the scan, so we'll pretend
        pg_sys::SelfItemPointerAttributeNumber => {
            // okay, "ctid" is a fast field but it's secret
            matches.push(WhichFastField::Ctid);
        }

        pg_sys::TableOidAttributeNumber => {
            matches.push(WhichFastField::TableOid);
        }

        attno => {
            // Handle attno <= 0 - this can happen in materialized views and FULL JOINs
            if attno <= 0 {
                // Just mark it as processed and continue
                return true;
            }

            // Get attribute info - use if let to handle missing attributes gracefully
            if let Some(att) = tupdesc.get((attno - 1) as usize) {
                let schema = index
                    .schema()
                    .expect("pullup_fast_fields: should have a schema");
                if let Some(search_field) = schema.search_field(att.name()) {
                    let categorized_fields = schema.categorized_fields();
                    let field_data = categorized_fields
                        .iter()
                        .find(|(sf, _)| sf == &search_field)
                        .map(|(_, data)| data);

                    if let Some(data) = field_data {
                        // Ensure that the expression used to index the value exactly matches the
                        // expression used in the target list (which we know is a Var, because
                        // that is the only thing that calls this function with attno > 0).
                        //
                        // Expression indices where target list references original column are not supported.
                        // See: https://github.com/paradedb/paradedb/issues/3978
                        if !matches!(data.source, FieldSource::Heap { attno: source_attno } if source_attno == (attno - 1) as usize)
                        {
                            return true;
                        }

                        if search_field.is_fast() {
                            if let Some(ff_type) =
                                fast_field_type_for_pullup(data.base_oid.value(), data.is_array)
                            {
                                matches
                                    .push(WhichFastField::Named(att.name().to_string(), ff_type));
                            } else {
                                // If the field is fast but the type is not supported (e.g. array, numeric, json),
                                // we treat it as not fast field capable.
                                return false;
                            }
                        }
                    }
                }
            }
            // If the attribute doesn't exist in this relation, just continue
            // This can happen in JOIN queries or materialized views
        }
    }
    true
}

unsafe fn fix_varno_list(list: *mut pg_sys::List, old_varno: i32, new_varno: i32) {
    if list.is_null() {
        return;
    }
    let list = PgList::<pg_sys::Node>::from_pg(list);
    for node in list.iter_ptr() {
        fix_varno_in_place(node, old_varno, new_varno);
    }
}

unsafe fn fix_varno_in_place(node: *mut pg_sys::Node, old_varno: i32, new_varno: i32) {
    if node.is_null() {
        return;
    }
    if let Some(var) = nodecast!(Var, T_Var, node) {
        if (*var).varno as i32 == old_varno {
            (*var).varno = new_varno as _;
        }
        if (*var).varnosyn as i32 == old_varno {
            (*var).varnosyn = new_varno as _;
        }
    } else if let Some(expr) = nodecast!(OpExpr, T_OpExpr, node) {
        fix_varno_list((*expr).args, old_varno, new_varno);
    } else if let Some(expr) = nodecast!(FuncExpr, T_FuncExpr, node) {
        fix_varno_list((*expr).args, old_varno, new_varno);
    } else if let Some(expr) = nodecast!(BoolExpr, T_BoolExpr, node) {
        fix_varno_list((*expr).args, old_varno, new_varno);
    } else if let Some(expr) = nodecast!(RelabelType, T_RelabelType, node) {
        fix_varno_in_place((*expr).arg.cast(), old_varno, new_varno);
    } else if let Some(expr) = nodecast!(CoerceToDomain, T_CoerceToDomain, node) {
        fix_varno_in_place((*expr).arg.cast(), old_varno, new_varno);
    } else if let Some(expr) = nodecast!(CoerceViaIO, T_CoerceViaIO, node) {
        fix_varno_in_place((*expr).arg.cast(), old_varno, new_varno);
    }
}

unsafe fn find_matching_fast_field(
    node: *mut pg_sys::Node,
    index_expressions: &PgList<pg_sys::Expr>,
    schema: SearchIndexSchema,
    rti: pg_sys::Index,
) -> Option<WhichFastField> {
    for (i, expr) in index_expressions.iter_ptr().enumerate() {
        let expr = expr as *mut pg_sys::Node;
        // Check if the unwrapped index expression matches the target node
        let unwrapped_index_expr = strip_tokenizer_cast(expr);

        // Adjust varno in index expression to match query rti
        fix_varno_in_place(unwrapped_index_expr, 1, rti as i32);

        if pg_sys::equal(
            node as *const core::ffi::c_void,
            unwrapped_index_expr as *const core::ffi::c_void,
        ) {
            // Find the search field corresponding to this expression index
            let categorized_fields = schema.categorized_fields();
            let field_data = categorized_fields.iter().find(|(sf, data)| {
                matches!(data.source, FieldSource::Expression { att_idx } if att_idx == i)
            });

            if let Some((search_field, data)) = field_data {
                if search_field.is_fast() {
                    if let Some(ff_type) =
                        fast_field_type_for_pullup(data.base_oid.value(), data.is_array)
                    {
                        return Some(WhichFastField::Named(
                            search_field.field_name().to_string(),
                            ff_type,
                        ));
                    }
                }
            }
        }
    }
    None
}

/// If all referenced fields in the given node can be fetched from the index as "fast fields",
/// return WhichFastFields covering them.
///
/// There are inline comments explaining the restrictions on what is supported.
pub unsafe fn pullup_fast_fields(
    node: *mut pg_sys::List,
    referenced_columns: &HashSet<pg_sys::AttrNumber>,
    heaprel: &PgSearchRelation,
    index: &PgSearchRelation,
    rti: pg_sys::Index,
    is_execution_time: bool,
) -> Option<Vec<WhichFastField>> {
    let mut matches = Vec::new();

    let tupdesc = heaprel.tuple_desc();

    // Get index expressions to check for matching expressions
    let index_info = pg_sys::BuildIndexInfo(index.as_ptr());
    let index_expressions = PgList::<pg_sys::Expr>::from_pg((*index_info).ii_Expressions);

    // First collect all matches from the target list (standard behavior)
    let targetlist = PgList::<pg_sys::TargetEntry>::from_pg(node);

    // Process target list entries
    for te in targetlist.iter_ptr() {
        if (*te).resorigtbl != pg_sys::Oid::INVALID && (*te).resorigtbl != heaprel.oid() {
            continue;
        }

        let maybe_var = if pgrx::is_a((*te).expr.cast(), pg_sys::NodeTag::T_Var) {
            if let Some(var) = find_one_var((*te).expr.cast()) {
                if (*var).varno as i32 != rti as i32 {
                    // We expect all Vars in the target list to be from the same range table as the
                    // index we're searching, so if we see a Var from a different range table, we skip it.
                    if is_execution_time {
                        // This is a sanity check to ensure that the target list is consistent with the
                        // index we're searching. As we're not supporting JOINs and Projection, at
                        // execution time (not planning time), we expect all Vars in the target list to
                        // be from the same range table as the index we're searching.
                        debug_assert_eq!(
                            (*var).varno as i32,
                            rti as i32,
                            "Encountered a Var with a different range table index.",
                        );
                    }
                    continue;
                }
                find_one_var_and_fieldname(VarContext::from_exec(heaprel.oid()), (*te).expr.cast())
            } else {
                None
            }
        } else {
            None
        };

        if let Some((var, fieldname)) = maybe_var {
            let start_len = matches.len();
            if !collect_fast_field_try_for_attno(
                (*var).varattno as i32,
                &mut matches,
                &tupdesc,
                heaprel,
                index,
                Some(&fieldname),
            ) {
                return None;
            }
            // If the var was successfully added as a fast field, continue.
            // If not (e.g. source mismatch), fall through to expression matching.
            if matches.len() > start_len {
                continue;
            }
        }

        // Try to match complex expression (or Var with source mismatch) against index expressions
        if let Some(ff) = find_matching_fast_field(
            (*te).expr as *mut pg_sys::Node,
            &index_expressions,
            index.schema().ok()?,
            rti,
        ) {
            matches.push(ff);
            continue;
        }

        if uses_scores((*te).expr.cast(), score_funcoids(), rti) {
            // we can only pull up a score if the score is:
            // 1. directly a call to `pdb.score`, with no wrapping expression (i.e. `is_score_func`)
            // 2. a call to `pdb.score` inside of an expression which will be solved by a
            //    wrapping/outer scan because it contains vars from other relations.
            if is_score_func((*te).expr.cast(), rti) {
                matches.push(WhichFastField::Score);
                continue;
            } else if !can_scan_evaluate_expr(rti, (*te).expr.cast()) {
                // The expression depends on other relations, so it will be evaluated by an upper node.
                // We just need to provide the score.
                matches.push(WhichFastField::Score);
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
                unsafe {
                    std::ffi::CStr::from_ptr((*te).resname)
                        .to_str()
                        .unwrap_or(create_resname("INVALID_NAME_STRING", &*te).as_str())
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
        // Casts of key fields (e.g. `CAST(id AS TEXT)`) are not supported.
        // See: https://github.com/paradedb/paradedb/issues/3978
        return None;
    }

    // Now also consider all referenced columns from other parts of the query
    for &attno in referenced_columns {
        let start_len = matches.len();
        if !collect_fast_field_try_for_attno(
            attno as i32,
            &mut matches,
            &tupdesc,
            heaprel,
            index,
            None,
        ) {
            return None;
        }
        // If not added (e.g. because of source mismatch), try expression matching for this column.
        if matches.len() == start_len {
            // For columns referenced in other parts of the query (e.g. WHERE), we only have
            // the attribute number. To support cases where the column is indexed via an
            // expression (e.g. `col::pdb.literal`), we create a synthetic Var to match
            // against index expressions.
            let mut dummy_var = pg_sys::Var {
                xpr: pg_sys::Expr {
                    type_: pg_sys::NodeTag::T_Var,
                },
                varno: rti as _,
                varattno: attno as i16,
                vartype: pg_sys::InvalidOid,
                vartypmod: -1,
                varcollid: pg_sys::InvalidOid,
                varlevelsup: 0,
                varnosyn: 0,
                varattnosyn: 0,
                location: -1,
                #[cfg(not(feature = "pg15"))]
                varnullingrels: std::ptr::null_mut(),
                #[cfg(feature = "pg18")]
                varreturningtype: pg_sys::InvalidOid.into(),
            };
            if let Some(att) = tupdesc.get((attno - 1) as usize) {
                dummy_var.vartype = att.atttypid;
                dummy_var.vartypmod = att.atttypmod;
                dummy_var.varcollid = att.attcollation;

                if let Some(ff) = find_matching_fast_field(
                    &mut dummy_var as *mut _ as *mut pg_sys::Node,
                    &index_expressions,
                    index.schema().ok()?,
                    rti,
                ) {
                    matches.push(ff);
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
        .filter(|ff| matches!(ff, WhichFastField::Named(_, _)))
        .count();

    // If we're missing any columns, we can't use fast field execution
    if referenced_columns_count > fast_field_column_count {
        return false;
    }

    true
}

/// Check if we can use the Mixed fast field execution method
pub fn is_mixed_fast_field_capable(privdata: &PrivateData) -> bool {
    if !gucs::is_mixed_fast_field_exec_enabled() {
        return false;
    }

    if !fast_field_capable_prereqs(privdata) {
        return false;
    }

    // We should only use Mixed if there is at least one named fast field, but fewer than the
    // configured column threshold.
    let which_fast_fields = privdata.planned_which_fast_fields().as_ref().unwrap();
    let named_field_count = which_fast_fields
        .iter()
        .filter(|wff| matches!(wff, WhichFastField::Named(_, _)))
        .count();

    0 < named_field_count && named_field_count < gucs::mixed_fast_field_exec_column_threshold()
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
                | WhichFastField::Score
        )
    })
}

/// Add nodes to `EXPLAIN` output to describe the "fast fields" being used by the query, if any
pub fn explain(state: &CustomScanStateWrapper<BaseScan>, explainer: &mut Explainer) {
    use crate::postgres::customscan::builders::custom_path::ExecMethodType;

    if let ExecMethodType::FastFieldMixed {
        which_fast_fields, ..
    } = &state.custom_state().exec_method_type
    {
        // Get all fast fields used
        let fields: Vec<_> = which_fast_fields
            .iter()
            .filter(|ff| matches!(ff, WhichFastField::Named(_, _)))
            .map(|ff| ff.name())
            .collect();

        explainer.add_text("Fast Fields", fields.join(", "));
    }
}
