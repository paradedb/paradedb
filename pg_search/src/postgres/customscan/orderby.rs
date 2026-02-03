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

//! Shared utilities for analyzing sort expressions in `ORDER BY` clauses.
//!
//! This module is shared between custom scans (BaseScan, AggregateScan) and the planner hook
//! to ensure that TopN compatibility validation logic is consistent across the codebase.
//!
//! This sharing is required to workaround <https://github.com/paradedb/paradedb/issues/3455>,
//! ensuring that we only replace window functions with ParadeDB placeholders
//! when we are certain that the query can be executed as a TopN query.

use crate::api::FieldName;
use crate::index::reader::index::MAX_TOPN_FEATURES;
use crate::nodecast;
use crate::postgres::customscan::builders::custom_path::OrderByStyle;
use crate::postgres::customscan::score_funcoids;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::var::{fieldname_from_var, find_one_var_and_fieldname, VarContext};
use crate::schema::{SearchField, SearchIndexSchema};
use pgrx::{direct_function_call, pg_sys, IntoDatum, PgList};

/// The type of sort expression found in an ORDER BY clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortExpressionType {
    /// Sorting by search score: `ORDER BY pdb.score(...)`
    Score,
    /// Sorting by a lowercased field: `ORDER BY lower(col)`
    Lower,
    /// Sorting by a raw field: `ORDER BY col`
    Raw,
}

/// Reason why pathkeys cannot be used for TopN execution
///
/// Note: These fields are currently unused on this branch but are kept to maintain
/// structural compatibility with the upstream commit being backported.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum UnusableReason {
    /// ORDER BY has too many columns (more than MAX_TOPN_FEATURES)
    TooManyColumns { count: usize, max: usize },
    /// Only a prefix of the ORDER BY columns can be pushed down
    PrefixOnly { matched: usize },
    /// Columns are not indexed with fast=true or not sortable
    NotSortable,
}

#[derive(Debug, Clone)]
pub enum PathKeyInfo {
    /// There are no PathKeys at all.
    None,
    /// There were PathKeys, but we cannot execute them.
    ///
    /// Note: The `UnusableReason` field is currently unused on this branch but is kept
    /// for compatibility with the upstream commit.
    #[allow(dead_code)]
    Unusable(UnusableReason),
    /// There were PathKeys, but we can only execute a non-empty prefix of them.
    UsablePrefix(Vec<OrderByStyle>),
    /// There are some PathKeys, and we can execute all of them.
    UsableAll(Vec<OrderByStyle>),
}

impl PathKeyInfo {
    pub fn is_usable(&self) -> bool {
        match self {
            PathKeyInfo::UsablePrefix(_) | PathKeyInfo::UsableAll(_) => true,
            PathKeyInfo::None | PathKeyInfo::Unusable(_) => false,
        }
    }

    pub fn pathkeys(&self) -> Option<&Vec<OrderByStyle>> {
        match self {
            PathKeyInfo::UsablePrefix(pathkeys) | PathKeyInfo::UsableAll(pathkeys) => {
                Some(pathkeys)
            }
            PathKeyInfo::None | PathKeyInfo::Unusable(_) => None,
        }
    }
}

/// Helper function to get the OID of the text lower function
pub fn text_lower_funcoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"pg_catalog.lower(text)".into_datum()],
        )
        .expect("the `pg_catalog.lower(text)` function should exist")
    }
}

unsafe fn extract_score_var(node: *mut pg_sys::Node) -> Option<*mut pg_sys::Var> {
    if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
        if score_funcoids().contains(&(*funcexpr).funcid) {
            let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
            if args.len() == 1 {
                return nodecast!(Var, T_Var, args.get_ptr(0).unwrap());
            }
        }
    }
    None
}

unsafe fn extract_lower_var(node: *mut pg_sys::Node) -> Option<*mut pg_sys::Var> {
    let funcexpr = nodecast!(FuncExpr, T_FuncExpr, node)?;
    if (*funcexpr).funcid == text_lower_funcoid() {
        let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
        if args.len() == 1 {
            if let Some(var) = nodecast!(Var, T_Var, args.get_ptr(0).unwrap()) {
                return Some(var);
            } else if let Some(relabel) =
                nodecast!(RelabelType, T_RelabelType, args.get_ptr(0).unwrap())
            {
                return nodecast!(Var, T_Var, (*relabel).arg);
            }
        }
    }
    None
}

/// Analyzes an ORDER BY expression to determine its type and extract the underlying variable.
///
/// This function unifies the logic for identifying sort keys across the planner hook (validation)
/// and the custom scan planner (execution).
///
/// Returns:
/// - `Some((type, var, field_name))` if the expression is a supported sort key.
/// - `None` if the expression is not supported or the variable/field could not be resolved.
pub unsafe fn analyze_sort_expression(
    node: *mut pg_sys::Node,
    context: VarContext,
) -> Option<(SortExpressionType, *mut pg_sys::Var, Option<FieldName>)> {
    if let Some(var) = extract_score_var(node) {
        return Some((SortExpressionType::Score, var, None));
    }

    if let Some(var) = extract_lower_var(node) {
        let (relid, attno) = context.var_relation(var);
        let field_name = fieldname_from_var(relid, var, attno);
        return Some((SortExpressionType::Lower, var, field_name));
    }

    if let Some((var, field_name)) = find_one_var_and_fieldname(context, node) {
        return Some((SortExpressionType::Raw, var, Some(field_name)));
    }

    None
}

/// Extract FuncExpr from PlaceHolderVar node
unsafe fn extract_funcexpr_from_placeholder(
    phv: *mut pg_sys::PlaceHolderVar,
) -> Option<*mut pg_sys::FuncExpr> {
    if phv.is_null() || (*phv).phexpr.is_null() {
        return None;
    }

    // The phexpr should contain our FuncExpr
    if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, (*phv).phexpr) {
        return Some(funcexpr);
    }

    None
}

/// Check if a varno is valid for the current relation (rti).
///
/// Returns true if:
/// 1. varno matches rti (exact match)
/// 2. varno is the parent of rti (partitioned/inheritance case)
unsafe fn is_varno_valid_for_relation(
    root: *mut pg_sys::PlannerInfo,
    varno: pg_sys::Index,
    current_rti: pg_sys::Index,
) -> bool {
    // 1. Exact match
    if varno == current_rti {
        return true;
    }

    // 2. Check if varno is parent of current_rti
    if !(*root).append_rel_list.is_null() {
        let append_rels = PgList::<pg_sys::AppendRelInfo>::from_pg((*root).append_rel_list);
        for appinfo in append_rels.iter_ptr() {
            if (*appinfo).parent_relid == varno && (*appinfo).child_relid == current_rti {
                return true;
            }
        }
    }

    false
}

/// Find TargetEntry by ressortgroupref
unsafe fn find_target_entry_by_ref(
    target_list: &PgList<pg_sys::TargetEntry>,
    ref_id: pg_sys::Index,
) -> Option<*mut pg_sys::TargetEntry> {
    target_list
        .iter_ptr()
        .find(|&te| (*te).ressortgroupref == ref_id)
}

/// Extract pathkeys from ORDER BY clauses using comprehensive expression handling
/// This function handles score functions, lower functions, relabel types, and regular variables
///
/// Returns PathKeyInfo indicating whether any PathKeys existed at all, and if so, whether they
/// might be usable via fast fields.
///
/// This function must be kept in sync with `validate_topn_compatibility` below.
pub unsafe fn extract_pathkey_styles_with_sortability_check<F1, F2>(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    schema: &SearchIndexSchema,
    regular_sortability_check: F1,
    lower_sortability_check: F2,
) -> PathKeyInfo
where
    F1: Fn(&SearchField) -> bool,
    F2: Fn(&SearchField) -> bool,
{
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);
    if pathkeys.is_empty() {
        return PathKeyInfo::None;
    }

    let mut pathkey_styles = Vec::new();
    for pathkey_ptr in pathkeys.iter_ptr() {
        let pathkey = pathkey_ptr;
        let equivclass = (*pathkey).pk_eclass;
        let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

        let mut found_valid_member = false;

        for member in members.iter_ptr() {
            let expr = (*member).em_expr;

            // Handle PlaceHolderVar: unwrap to check if it contains a sortable expression.
            // We support any valid sort expression (Score, Lower, Raw) that might be wrapped.
            let mut expr_to_analyze = expr;
            if let Some(phv) = nodecast!(PlaceHolderVar, T_PlaceHolderVar, expr) {
                if let Some(funcexpr) = extract_funcexpr_from_placeholder(phv) {
                    expr_to_analyze = funcexpr.cast();
                }
            }

            if let Some((sort_type, var, field_name_opt)) =
                analyze_sort_expression(expr_to_analyze.cast(), VarContext::from_planner(root))
            {
                // Verify the var belongs to the correct relation (either the relation itself or its parent)
                if !is_varno_valid_for_relation(root, (*var).varno as pg_sys::Index, rti) {
                    continue;
                }

                match sort_type {
                    SortExpressionType::Score => {
                        pathkey_styles.push(OrderByStyle::Score(pathkey));
                        found_valid_member = true;
                        break;
                    }
                    SortExpressionType::Lower => {
                        if let Some(field_name) = field_name_opt {
                            if let Some(search_field) = schema.search_field(field_name.root()) {
                                if lower_sortability_check(&search_field) {
                                    pathkey_styles.push(OrderByStyle::Field(pathkey, field_name));
                                    found_valid_member = true;
                                    break;
                                }
                            }
                        }
                    }
                    SortExpressionType::Raw => {
                        if let Some(field_name) = field_name_opt {
                            if let Some(search_field) = schema.search_field(field_name.root()) {
                                if regular_sortability_check(&search_field) {
                                    pathkey_styles.push(OrderByStyle::Field(pathkey, field_name));
                                    found_valid_member = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // If we couldn't find any valid member for this pathkey, then we can't handle this series
        // of pathkeys.
        if !found_valid_member {
            if pathkey_styles.is_empty() {
                return PathKeyInfo::Unusable(UnusableReason::NotSortable);
            } else {
                return PathKeyInfo::UsablePrefix(pathkey_styles);
            }
        }
    }

    PathKeyInfo::UsableAll(pathkey_styles)
}

/// Check if the query is a valid TopN query compatible with ParadeDB execution.
///
/// Ensures that:
/// 1. The query has both ORDER BY and LIMIT clauses.
/// 2. There are not too many sort columns.
/// 3. All sort columns belong to the same relation.
/// 4. That relation has a BM25 index.
/// 5. All sort columns are sortable in the index (fast fields).
///
/// This function must be kept in sync with [`extract_pathkey_styles_with_sortability_check`]
/// above to ensure that queries accepted here can be executed by the custom scan.
pub unsafe fn validate_topn_compatibility(parse: *mut pg_sys::Query) -> bool {
    if parse.is_null() || (*parse).sortClause.is_null() || (*parse).limitCount.is_null() {
        return false;
    }

    let sort_list = PgList::<pg_sys::SortGroupClause>::from_pg((*parse).sortClause);
    if sort_list.len() > MAX_TOPN_FEATURES {
        return false;
    }

    let target_list = PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList);

    // We need to identify the single relation that this TopN query targets
    // Tuple: (varno, relid, schema)
    let mut target_relation_info: Option<(pg_sys::Index, pg_sys::Oid, SearchIndexSchema)> = None;

    for sort_clause in sort_list.iter_ptr() {
        let tle_ref = (*sort_clause).tleSortGroupRef;
        let Some(te) = find_target_entry_by_ref(&target_list, tle_ref) else {
            return false;
        };

        let expr = (*te).expr as *mut pg_sys::Node;

        // Use analyze_sort_expression to identify the sort key type and underlying variable
        let Some((sort_type, var, field_name_opt)) =
            analyze_sort_expression(expr, VarContext::from_query(parse))
        else {
            return false;
        };

        // Identify relation
        let varno = (*var).varno as pg_sys::Index;
        if varno == 0 {
            return false;
        }

        if let Some((expected_varno, _, _)) = &target_relation_info {
            if varno != *expected_varno {
                // Sorting by different relations
                return false;
            }
        } else {
            // Initialize target relation info
            let (relid, _) = VarContext::from_query(parse).var_relation(var);
            if relid == pg_sys::InvalidOid {
                return false;
            }

            // Check if has BM25
            let (_, bm25_index) = match rel_get_bm25_index(relid) {
                Some(res) => res,
                None => return false,
            };

            let schema = match SearchIndexSchema::open(&bm25_index) {
                Ok(s) => s,
                Err(_) => return false,
            };

            target_relation_info = Some((varno, relid, schema));
        }

        // Validate sortability
        let (_, _, schema) = target_relation_info.as_ref().unwrap();

        match sort_type {
            SortExpressionType::Score => {
                // Score is always sortable
                continue;
            }
            SortExpressionType::Lower => {
                let Some(field_name) = field_name_opt else {
                    return false;
                };
                let Some(search_field) = schema.search_field(field_name.root()) else {
                    return false;
                };
                if !search_field.is_lower_sortable() {
                    return false;
                }
            }
            SortExpressionType::Raw => {
                let Some(field_name) = field_name_opt else {
                    return false;
                };
                let Some(search_field) = schema.search_field(field_name.root()) else {
                    return false;
                };
                if !search_field.is_raw_sortable() {
                    return false;
                }
            }
        }
    }

    target_relation_info.is_some()
}
