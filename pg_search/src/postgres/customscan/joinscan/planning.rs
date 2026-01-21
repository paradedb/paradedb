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
//! - Handle ORDER BY score pathkeys

use super::build::{ExecutionHints, JoinAlgorithmHint, JoinKeyPair, JoinSideInfo};
use crate::api::operator::anyelement_query_input_opoid;
use crate::nodecast;
use crate::postgres::customscan::basescan::projections::score::is_score_func;
use crate::postgres::customscan::builders::custom_path::OrderByStyle;
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::customscan::range_table::{bms_iter, get_plain_relation_relid};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::utils::expr_contains_any_operator;
use crate::query::SearchQueryInput;
use pgrx::{pg_sys, PgList};

/// Result of extracting join conditions from the restrict list.
pub(super) struct JoinConditions {
    /// Equi-join keys with type info for composite key extraction.
    pub equi_keys: Vec<JoinKeyPair>,
    /// Other join conditions (non-equijoin) that need to be evaluated after hash lookup.
    /// These are the RestrictInfo nodes themselves.
    pub other_conditions: Vec<*mut pg_sys::RestrictInfo>,
    /// Whether any join-level condition contains our @@@ operator.
    pub has_search_predicate: bool,
}

/// Extract join conditions from the restrict list.
///
/// Analyzes the join's restrict list to identify:
/// - Equi-join conditions (e.g., `a.id = b.id`) for hash table building
/// - Other conditions that need post-hash evaluation
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

                // Check if operator is an equality operator (hash-joinable)
                let opno = (*opexpr).opno;
                let is_equality_op = is_op_hash_joinable(opno);

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

/// Check if an operator is suitable for hash join (i.e., is an equality operator).
/// Uses PostgreSQL's op_hashjoinable to determine this.
pub(super) unsafe fn is_op_hash_joinable(opno: pg_sys::Oid) -> bool {
    // op_hashjoinable checks if the operator can be used for hash joins,
    // which requires it to be an equality operator with a hash function.
    // We pass InvalidOid as inputtype to accept any input type.
    pg_sys::op_hashjoinable(opno, pg_sys::InvalidOid)
}

/// Get type length and pass-by-value info for a given type OID.
pub(super) unsafe fn get_type_info(type_oid: pg_sys::Oid) -> (i16, bool) {
    let mut typlen: i16 = 0;
    let mut typbyval: bool = false;
    pg_sys::get_typlenbyval(type_oid, &mut typlen, &mut typbyval);
    (typlen, typbyval)
}

/// Try to extract join side information from a RelOptInfo.
/// Returns JoinSideInfo if we find a base relation (possibly with a BM25 index).
pub(super) unsafe fn extract_join_side_info(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
) -> Option<JoinSideInfo> {
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

    let mut side_info = JoinSideInfo::new().with_heap_rti(rti).with_heaprelid(relid);

    // Extract the alias from the RTE if present
    // The eref->aliasname contains the alias (or table name if no alias was specified)
    if !(*rte).eref.is_null() {
        let eref = (*rte).eref;
        if !(*eref).aliasname.is_null() {
            let alias_cstr = std::ffi::CStr::from_ptr((*eref).aliasname);
            if let Ok(alias) = alias_cstr.to_str() {
                // Get the actual table name to check if alias is different
                let rel = PgSearchRelation::open(relid);
                let table_name = rel.name();
                // Only set alias if it's different from the table name
                if alias != table_name {
                    side_info = side_info.with_alias(alias.to_string());
                }
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

/// Compute execution hints based on planning information.
///
/// Simplified version without cost estimation - just sets basic hints.
/// Detailed cost estimation is deferred to DataFusion integration.
pub(super) fn compute_execution_hints(_limit: Option<usize>) -> ExecutionHints {
    ExecutionHints::new().with_algorithm(JoinAlgorithmHint::Auto)
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
