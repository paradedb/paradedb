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

//! Planning-related functions for JoinScan.
//!
//! This module contains functions used during the planning phase to:
//! - Extract and analyze join conditions
//! - Gather information about join sides (tables, indexes, predicates)
//! - Collect required fields to ensure availability during execution
//! - Handle ORDER BY score pathkeys

use super::build::{JoinCSClause, JoinKeyPair, ScanInfo};
use super::privdat::{OutputColumnInfo, INNER_SCORE_ALIAS, OUTER_SCORE_ALIAS};
use crate::api::operator::anyelement_query_input_opoid;
use crate::api::{HashMap, OrderByFeature, OrderByInfo, SortDirection};
use crate::index::fast_fields_helper::{FastFieldType, WhichFastField};
use crate::nodecast;
use crate::postgres::customscan::basescan::projections::score::is_score_func;
use crate::postgres::customscan::builders::custom_path::OrderByStyle;
use crate::postgres::customscan::opexpr::{
    initialize_equality_operator_lookup, OperatorAccepts, PostgresOperatorOid, TantivyOperator,
};
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::customscan::range_table::{bms_iter, get_plain_relation_relid};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::utils::{expr_collect_vars, expr_contains_any_operator};
use crate::postgres::var::fieldname_from_var;
use crate::query::SearchQueryInput;
use pgrx::{pg_sys, PgList};
use std::sync::OnceLock;

/// Cache for operator OID lookups.
static OPERATOR_LOOKUP: OnceLock<HashMap<PostgresOperatorOid, TantivyOperator>> = OnceLock::new();
pub(super) struct JoinConditions {
    /// Equi-join keys with type info for composite key extraction.
    pub equi_keys: Vec<JoinKeyPair>,
    /// Other join conditions (non-equijoin) that need to be evaluated after join.
    /// These are the RestrictInfo nodes themselves.
    pub other_conditions: Vec<*mut pg_sys::RestrictInfo>,
    /// Whether any join-level condition contains our @@@ operator.
    pub has_search_predicate: bool,
}

/// Extract join conditions from the restrict list.
///
/// Analyzes the join's restrict list to identify:
/// - Equi-join conditions (e.g., `a.id = b.id`) for joining
/// - Other conditions that need post-join evaluation
/// - Whether any condition contains our @@@ search operator
pub(super) unsafe fn extract_join_conditions(
    extra: *mut pg_sys::JoinPathExtraData,
    outer_rti: pg_sys::Index,
    inner_rti: pg_sys::Index,
) -> JoinConditions {
    let mut result = JoinConditions {
        equi_keys: Vec::new(),
        other_conditions: Vec::new(),
        has_search_predicate: false,
    };

    if extra.is_null() {
        return result;
    }

    let restrictlist = (*extra).restrictlist;
    if restrictlist.is_null() {
        return result;
    }

    let restrict_infos = PgList::<pg_sys::RestrictInfo>::from_pg(restrictlist);

    for ri in restrict_infos.iter_ptr() {
        if ri.is_null() {
            continue;
        }

        let clause = (*ri).clause;
        if clause.is_null() {
            continue;
        }

        // Check if this clause contains our @@@ operator
        let search_op = anyelement_query_input_opoid();
        if expr_contains_any_operator(clause.cast(), &[search_op]) {
            result.has_search_predicate = true;
        }

        // Try to identify equi-join conditions (OpExpr with Var = Var using equality operator)
        let mut is_equi_join = false;

        if (*clause).type_ == pg_sys::NodeTag::T_OpExpr {
            let opexpr = clause as *mut pg_sys::OpExpr;
            let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);

            // Equi-join: should have exactly 2 args, both Var nodes, AND use equality operator
            if args.len() == 2 {
                let arg0 = args.get_ptr(0).unwrap();
                let arg1 = args.get_ptr(1).unwrap();

                // Check if operator is an equality operator
                let opno = (*opexpr).opno;
                let is_equality_op = lookup_operator(opno) == Some("=");

                if is_equality_op
                    && (*arg0).type_ == pg_sys::NodeTag::T_Var
                    && (*arg1).type_ == pg_sys::NodeTag::T_Var
                {
                    let var0 = arg0 as *mut pg_sys::Var;
                    let var1 = arg1 as *mut pg_sys::Var;

                    let varno0 = (*var0).varno as pg_sys::Index;
                    let varno1 = (*var1).varno as pg_sys::Index;
                    let attno0 = (*var0).varattno;
                    let attno1 = (*var1).varattno;

                    // Check if this is an equi-join between outer and inner
                    if varno0 == outer_rti && varno1 == inner_rti {
                        // Get type info from the Var
                        let type_oid = (*var0).vartype;
                        let (typlen, typbyval) = get_type_info(type_oid);
                        result.equi_keys.push(JoinKeyPair {
                            outer_attno: attno0,
                            inner_attno: attno1,
                            type_oid,
                            typlen,
                            typbyval,
                        });
                        is_equi_join = true;
                    } else if varno0 == inner_rti && varno1 == outer_rti {
                        // Get type info from the Var
                        let type_oid = (*var1).vartype;
                        let (typlen, typbyval) = get_type_info(type_oid);
                        result.equi_keys.push(JoinKeyPair {
                            outer_attno: attno1,
                            inner_attno: attno0,
                            type_oid,
                            typlen,
                            typbyval,
                        });
                        is_equi_join = true;
                    }
                }
            }
        }

        // If it's not an equi-join, it's an "other" condition
        // BUT: Skip conditions that contain our @@@ search operator, as these
        // will be handled separately via join-level predicate evaluation
        if !is_equi_join {
            let search_op = anyelement_query_input_opoid();
            let has_search_op = expr_contains_any_operator(clause.cast(), &[search_op]);
            if !has_search_op {
                result.other_conditions.push(ri);
            }
        }
    }

    result
}

fn lookup_operator(opno: pg_sys::Oid) -> Option<&'static str> {
    let lookup = OPERATOR_LOOKUP
        .get_or_init(|| unsafe { initialize_equality_operator_lookup(OperatorAccepts::All) });
    lookup.get(&opno).copied()
}

/// Get type length and pass-by-value info for a given type OID.
pub(super) unsafe fn get_type_info(type_oid: pg_sys::Oid) -> (i16, bool) {
    let mut typlen: i16 = 0;
    let mut typbyval: bool = false;
    pg_sys::get_typlenbyval(type_oid, &mut typlen, &mut typbyval);
    (typlen, typbyval)
}

/// Try to extract join side information from a RelOptInfo.
/// Returns ScanInfo if we find a base relation (possibly with a BM25 index).
pub(super) unsafe fn extract_join_side_info(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
) -> Option<ScanInfo> {
    if rel.is_null() {
        return None;
    }

    let relids = (*rel).relids;
    if relids.is_null() {
        return None;
    }

    // TODO(multi-relation-sides): Currently we only handle single base relations on
    // each side. This means queries like:
    //   SELECT * FROM A JOIN B ON ... JOIN C ON ... WHERE A.text @@@ 'x' LIMIT 10
    // won't use JoinScan because one "side" of the outer join is itself a join result.
    //
    // Supporting this would require:
    // 1. Recursive analysis of join trees to find the relation with search predicate
    // 2. Propagating search predicates through the join tree
    // 3. Handling parameterized paths for inner relations
    let mut rti_iter = bms_iter(relids);
    let rti = rti_iter.next()?;

    if rti_iter.next().is_some() {
        return None;
    }

    // Get the RTE and verify it's a plain relation
    let rtable = (*(*root).parse).rtable;
    if rtable.is_null() {
        return None;
    }

    let rte = pg_sys::rt_fetch(rti, rtable);
    let relid = get_plain_relation_relid(rte)?;

    let mut side_info = ScanInfo::new().with_heap_rti(rti).with_heaprelid(relid);

    // Extract the alias from the RTE if present
    // The eref->aliasname contains the alias (or table name if no alias was specified)
    if !(*rte).eref.is_null() {
        let eref = (*rte).eref;
        if !(*eref).aliasname.is_null() {
            let alias_cstr = std::ffi::CStr::from_ptr((*eref).aliasname);
            if let Ok(alias) = alias_cstr.to_str() {
                // Always set alias to the eref name, which is unique in the query context
                side_info = side_info.with_alias(alias.to_string());
            }
        }
    }

    // Check if this relation has a BM25 index
    if let Some((_, bm25_index)) = rel_get_bm25_index(relid) {
        side_info = side_info.with_indexrelid(bm25_index.oid());

        // Try to extract quals for this relation
        let baserestrictinfo = PgList::<pg_sys::RestrictInfo>::from_pg((*rel).baserestrictinfo);
        if !baserestrictinfo.is_empty() {
            let context = PlannerContext::from_planner(root);
            let mut state = QualExtractState::default();

            if let Some(qual) = extract_quals(
                &context,
                rti,
                baserestrictinfo.as_ptr().cast(),
                anyelement_query_input_opoid(),
                crate::postgres::customscan::builders::custom_path::RestrictInfoType::BaseRelation,
                &bm25_index,
                false, // Don't convert external to special qual
                &mut state,
                true, // Attempt pushdown
            ) {
                if state.uses_our_operator {
                    let query = SearchQueryInput::from(&qual);
                    side_info = side_info.with_query(query);
                }
            }
        }
    }

    Some(side_info)
}

/// Check if all ORDER BY columns are fast fields.
///
/// For JoinScan to be proposed, all columns used in ORDER BY must be fast fields
/// in their respective BM25 indexes (or be paradedb.score() which is handled separately).
///
/// Returns true if:
/// - No ORDER BY clause exists
/// - All ORDER BY columns are fast fields or score functions
///
/// Returns false if any ORDER BY column is not a fast field.
pub(super) unsafe fn order_by_columns_are_fast_fields(
    root: *mut pg_sys::PlannerInfo,
    outer_side: &ScanInfo,
    inner_side: &ScanInfo,
) -> bool {
    use super::predicate::is_column_fast_field;

    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);
    if pathkeys.is_empty() {
        return true; // No ORDER BY, nothing to check
    }

    let outer_rti = outer_side.heap_rti.unwrap_or(0);
    let inner_rti = inner_side.heap_rti.unwrap_or(0);

    for pathkey_ptr in pathkeys.iter_ptr() {
        let equivclass = (*pathkey_ptr).pk_eclass;
        let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

        // Check each member of the equivalence class
        for member in members.iter_ptr() {
            let expr = (*member).em_expr;

            // Skip if this is a score function (handled separately)
            if let Some(phv) = nodecast!(PlaceHolderVar, T_PlaceHolderVar, expr) {
                if !phv.is_null() && !(*phv).phexpr.is_null() {
                    if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, (*phv).phexpr) {
                        if is_score_func(funcexpr.cast(), outer_rti)
                            || is_score_func(funcexpr.cast(), inner_rti)
                        {
                            continue; // Score function, skip fast field check
                        }
                    }
                }
            }
            if is_score_func(expr.cast(), outer_rti) || is_score_func(expr.cast(), inner_rti) {
                continue; // Score function, skip fast field check
            }

            // Check if this is a Var (column reference)
            if let Some(var) = nodecast!(Var, T_Var, expr) {
                let varno = (*var).varno as pg_sys::Index;
                let varattno = (*var).varattno;

                // Determine which side this column belongs to
                let side = if varno == outer_rti {
                    outer_side
                } else if varno == inner_rti {
                    inner_side
                } else {
                    // Unknown relation - can't verify, reject
                    pgrx::debug1!(
                        "JoinScan: ORDER BY column (varno={}) not from join sides, rejecting",
                        varno
                    );
                    return false;
                };

                // Check if the column is a fast field
                if !is_column_fast_field(side, varattno) {
                    pgrx::debug1!(
                        "JoinScan: ORDER BY column (varno={}, attno={}) is not a fast field, rejecting",
                        varno,
                        varattno
                    );
                    return false;
                }
            }
        }
    }

    true // All ORDER BY columns are fast fields
}

/// Extract ORDER BY score pathkey for the driving side.
///
/// This checks if the query has an ORDER BY clause with paradedb.score()
/// referencing the driving side relation. If found, returns the OrderByStyle
/// that can be used to declare pathkeys on the CustomPath, eliminating the
/// need for PostgreSQL to add a separate Sort node.
///
/// Returns None if:
/// - No ORDER BY clause exists
/// - ORDER BY doesn't use paradedb.score()
/// - Score function references a different relation
pub(super) unsafe fn extract_score_pathkey(
    root: *mut pg_sys::PlannerInfo,
    driving_side_rti: pg_sys::Index,
) -> Option<OrderByStyle> {
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);
    if pathkeys.is_empty() {
        return None;
    }

    // We only support a single score-based ORDER BY for now
    // (first pathkey must be score for the driving side)
    let pathkey_ptr = pathkeys.iter_ptr().next()?;
    let pathkey = pathkey_ptr;
    let equivclass = (*pathkey).pk_eclass;
    let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

    for member in members.iter_ptr() {
        let expr = (*member).em_expr;

        // Check if this is a PlaceHolderVar containing a score function
        if let Some(phv) = nodecast!(PlaceHolderVar, T_PlaceHolderVar, expr) {
            if !phv.is_null() && !(*phv).phexpr.is_null() {
                if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, (*phv).phexpr) {
                    if is_score_func(funcexpr.cast(), driving_side_rti) {
                        return Some(OrderByStyle::Score(pathkey));
                    }
                }
            }
        }
        // Check if this is a direct score function call
        else if is_score_func(expr.cast(), driving_side_rti) {
            return Some(OrderByStyle::Score(pathkey));
        }
    }

    None
}

/// Quote an identifier only if it contains non-lowercase characters or special characters.
///
/// This ensures that mixed-case identifiers are preserved when parsed by DataFusion's `col()`
/// while avoiding unnecessary quotes for standard lowercase identifiers.
fn quote_identifier_if_needed(name: &str) -> String {
    let needs_quoting = name.is_empty()
        || name.starts_with(|c: char| !c.is_ascii_lowercase() && c != '_')
        || name
            .chars()
            .any(|c| !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '_');

    if needs_quoting {
        format!(r#""{}""#, name)
    } else {
        name.to_string()
    }
}

/// Extract ORDER BY information for DataFusion execution.
// TODO: Audit this to see whether any of it can be shared with TopN.
// It mostly can't because the implementation of the sort is so different, but should still try to unify a bit.
pub(super) unsafe fn extract_orderby(
    root: *mut pg_sys::PlannerInfo,
    outer_side: &ScanInfo,
    inner_side: &ScanInfo,
    driving_side_is_outer: bool,
) -> Vec<OrderByInfo> {
    let mut result = Vec::new();
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);

    if pathkeys.is_empty() {
        return result;
    }

    let outer_rti = outer_side.heap_rti.unwrap_or(0);
    let inner_rti = inner_side.heap_rti.unwrap_or(0);

    for pathkey_ptr in pathkeys.iter_ptr() {
        let pathkey = pathkey_ptr;
        let equivclass = (*pathkey).pk_eclass;
        let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

        // Determine direction
        let nulls_first = (*pathkey).pk_nulls_first;
        #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
        let is_asc = match (*pathkey).pk_strategy as u32 {
            pg_sys::BTLessStrategyNumber => true,
            pg_sys::BTGreaterStrategyNumber => false,
            _ => true,
        };
        #[cfg(feature = "pg18")]
        let is_asc = match (*pathkey).pk_cmptype {
            pg_sys::CompareType::COMPARE_LT => true,
            pg_sys::CompareType::COMPARE_GT => false,
            _ => true,
        };

        let direction = match (is_asc, nulls_first) {
            (true, true) => SortDirection::AscNullsFirst,
            (true, false) => SortDirection::AscNullsLast,
            (false, true) => SortDirection::DescNullsFirst,
            (false, false) => SortDirection::DescNullsLast,
        };

        // Find the expression that matches one of our relations
        for member in members.iter_ptr() {
            let expr = (*member).em_expr;

            // Handle PlaceHolderVar (common for score functions in joins)
            let mut check_expr = expr;
            if let Some(phv) = nodecast!(PlaceHolderVar, T_PlaceHolderVar, expr) {
                if !phv.is_null() && !(*phv).phexpr.is_null() {
                    check_expr = (*phv).phexpr;
                }
            }

            // Check score
            if is_score_func(check_expr.cast(), outer_rti) {
                if driving_side_is_outer {
                    result.push(OrderByInfo {
                        feature: OrderByFeature::Score,
                        direction,
                    });
                } else {
                    let alias = outer_side
                        .alias
                        .as_ref()
                        .expect("outer alias should be set");
                    result.push(OrderByInfo {
                        feature: OrderByFeature::Field(
                            format!("{}.{}", alias, OUTER_SCORE_ALIAS).into(),
                        ),
                        direction,
                    });
                }
                break;
            }
            if is_score_func(check_expr.cast(), inner_rti) {
                if !driving_side_is_outer {
                    result.push(OrderByInfo {
                        feature: OrderByFeature::Score,
                        direction,
                    });
                } else {
                    let alias = inner_side
                        .alias
                        .as_ref()
                        .expect("inner alias should be set");
                    result.push(OrderByInfo {
                        feature: OrderByFeature::Field(
                            format!("{}.{}", alias, INNER_SCORE_ALIAS).into(),
                        ),
                        direction,
                    });
                }
                break;
            }

            // Check Var
            if let Some(var) = nodecast!(Var, T_Var, expr) {
                let varno = (*var).varno as pg_sys::Index;
                let varattno = (*var).varattno;

                if varno == outer_rti {
                    if let Some(relid) = outer_side.heaprelid {
                        let fieldname = fieldname_from_var(relid, var, varattno)
                            .map(|f| f.to_string())
                            .unwrap_or_else(|| "?".to_string());
                        let alias = outer_side
                            .alias
                            .as_ref()
                            .expect("outer alias should be set");
                        let quoted_fieldname = quote_identifier_if_needed(&fieldname);
                        result.push(OrderByInfo {
                            feature: OrderByFeature::Field(
                                format!("{}.{}", alias, quoted_fieldname).into(),
                            ),
                            direction,
                        });
                        break;
                    }
                } else if varno == inner_rti {
                    if let Some(relid) = inner_side.heaprelid {
                        let fieldname = fieldname_from_var(relid, var, varattno)
                            .map(|f| f.to_string())
                            .unwrap_or_else(|| "?".to_string());
                        let alias = inner_side
                            .alias
                            .as_ref()
                            .expect("inner alias should be set");
                        let quoted_fieldname = quote_identifier_if_needed(&fieldname);
                        result.push(OrderByInfo {
                            feature: OrderByFeature::Field(
                                format!("{}.{}", alias, quoted_fieldname).into(),
                            ),
                            direction,
                        });
                        break;
                    }
                }
            }
        }
    }

    result
}

/// Collect all required fields for both sides of the join.
pub(super) unsafe fn collect_required_fields(
    join_clause: &mut JoinCSClause,
    output_columns: &[OutputColumnInfo],
    custom_exprs: *mut pg_sys::List,
) {
    let outer_rti = join_clause.outer_side.heap_rti.unwrap_or(0);
    let inner_rti = join_clause.inner_side.heap_rti.unwrap_or(0);
    let outer_alias = join_clause
        .outer_side
        .alias
        .clone()
        .expect("outer alias set");
    let inner_alias = join_clause
        .inner_side
        .alias
        .clone()
        .expect("inner alias set");

    // Add Ctid by default to both sides (needed for join/filters/output)
    join_clause.outer_side.add_field(
        pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber,
        WhichFastField::Ctid,
    );
    join_clause.inner_side.add_field(
        pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber,
        WhichFastField::Ctid,
    );

    // 1. Join Keys
    for jk in &join_clause.join_keys {
        ensure_field(&mut join_clause.outer_side, jk.outer_attno);
        ensure_field(&mut join_clause.inner_side, jk.inner_attno);
    }

    // 2. Filters (custom_exprs)
    let expr_list = PgList::<pg_sys::Node>::from_pg(custom_exprs);
    for expr_node in expr_list.iter_ptr() {
        let vars = expr_collect_vars(expr_node, true);
        for var in vars {
            if var.rti == pg_sys::INDEX_VAR as pg_sys::Index {
                // Resolve INDEX_VAR to original table column
                let idx = (var.attno - 1) as usize;
                if let Some(info) = output_columns.get(idx) {
                    if info.original_attno > 0 {
                        let rti = if info.is_outer { outer_rti } else { inner_rti };
                        if rti == outer_rti {
                            ensure_field(&mut join_clause.outer_side, info.original_attno);
                        } else if rti == inner_rti {
                            ensure_field(&mut join_clause.inner_side, info.original_attno);
                        }
                    }
                }
            } else if var.rti == outer_rti {
                ensure_field(&mut join_clause.outer_side, var.attno);
            } else if var.rti == inner_rti {
                ensure_field(&mut join_clause.inner_side, var.attno);
            }
        }
    }

    // 3. Order By
    for info in &join_clause.order_by {
        if let OrderByFeature::Field(name_wrapper) = &info.feature {
            let name = name_wrapper.as_ref();
            if let Some((alias, col_name)) = name.split_once('.') {
                let raw_col_name = col_name.trim_matches('"');
                if alias == outer_alias {
                    if let Some(attno) = get_attno_by_name(&join_clause.outer_side, raw_col_name) {
                        ensure_field(&mut join_clause.outer_side, attno);
                    }
                } else if alias == inner_alias {
                    if let Some(attno) = get_attno_by_name(&join_clause.inner_side, raw_col_name) {
                        ensure_field(&mut join_clause.inner_side, attno);
                    }
                }
            }
        }
    }

    // 4. Scores
    if join_clause.outer_side.score_needed {
        join_clause.outer_side.add_field(0, WhichFastField::Score);
    }

    if join_clause.inner_side.score_needed {
        join_clause.inner_side.add_field(0, WhichFastField::Score);
    }
}

/// Ensure a specific field is present in the ScanInfo's required fields list.
unsafe fn ensure_field(side: &mut ScanInfo, attno: pg_sys::AttrNumber) {
    if side.fields.iter().any(|f| f.attno == attno) {
        return;
    }
    if let Some(field) = get_fast_field(side, attno) {
        side.add_field(attno, field);
    }
}

/// Check if an attribute is a fast field and return its type/info.
unsafe fn get_fast_field(side: &ScanInfo, attno: pg_sys::AttrNumber) -> Option<WhichFastField> {
    if attno == 0 {
        return None;
    }
    // Handle system columns: for now only ctid is handled by default addition.
    if attno <= 0 {
        return None;
    }

    let heaprelid = side.heaprelid?;
    let heaprel = PgSearchRelation::open(heaprelid);
    let tupdesc = heaprel.tuple_desc();

    // attno is 1-based
    if attno as usize > tupdesc.len() {
        return None;
    }

    let att = tupdesc.get((attno - 1) as usize)?;
    let att_name = att.name();

    let indexrelid = side.indexrelid?;
    let indexrel = PgSearchRelation::open(indexrelid);
    let schema = indexrel.schema().ok()?;

    if let Some(search_field) = schema.search_field(att_name) {
        Some(WhichFastField::Named(
            att_name.to_string(),
            FastFieldType::from(search_field.field_type()),
        ))
    } else {
        Some(WhichFastField::Named(
            att_name.to_string(),
            FastFieldType::Int64,
        ))
    }
}

unsafe fn get_attno_by_name(side: &ScanInfo, name: &str) -> Option<pg_sys::AttrNumber> {
    let heaprelid = side.heaprelid?;
    let rel = PgSearchRelation::open(heaprelid);
    let tupdesc = rel.tuple_desc();
    for (i, att) in tupdesc.iter().enumerate() {
        if att.name() == name {
            return Some((i + 1) as pg_sys::AttrNumber);
        }
    }
    None
}
