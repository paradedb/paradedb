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

//! Predicate extraction functions for JoinScan.
//!
//! See the [JoinScan README](README.md) for the full architecture overview.
//!
//! This module handles the transformation of PostgreSQL expressions containing
//! search predicates into `JoinLevelExpr` trees that can be evaluated
//! during join execution. It supports:
//!
//! - Single-table search predicates (converted to Tantivy queries)
//! - Cross-relation heap conditions (evaluated by PostgreSQL)
//! - Boolean expression trees (AND/OR/NOT)

use super::build::{JoinLevelExpr, JoinSource, RelNode, ScanInfo};
use crate::api::operator::anyelement_query_input_opoid;
use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
use crate::postgres::customscan::datafusion::translator::PredicateTranslator;
use crate::postgres::customscan::pullup::resolve_fast_field;
use crate::postgres::customscan::qual_inspect::{
    PlannerContext, QualExtractState, contains_exec_param, extract_quals,
};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::utils::{expr_collect_rtis, expr_collect_vars, expr_contains_any_operator};
use crate::query::SearchQueryInput;
use pgrx::{PgList, pg_sys};

/// Recursively transform a PostgreSQL expression tree into a `JoinLevelExpr`.
///
/// Handles:
/// - Single-table search predicates: extracted into a single Tantivy query (preserving NOT/AND/OR)
/// - Cross-relation sub-trees without search predicates: extracted as a MultiTablePredicate leaf
///   (in the same order as multi_table_predicates in the clause) for adding to custom_exprs
/// - Cross-relation boolean expressions (AND/OR/NOT): recursively preserved in the `JoinLevelExpr` tree
pub unsafe fn transform_to_search_expr(
    root: *mut pg_sys::PlannerInfo,
    node: *mut pg_sys::Node,
    sources: &[&JoinSource],
    plan: Option<&RelNode>,
    multi_table_predicate_clauses: &mut Vec<*mut pg_sys::Expr>,
) -> Option<JoinLevelExpr> {
    if node.is_null() {
        return None;
    }

    // A List is an implicit conjunction container, not one expression. It
    // must be decomposed before relation classification: collecting RTIs from
    // the container can otherwise group unrelated single-table predicates into
    // one apparent cross-table predicate.
    let node_type = (*node).type_;
    if node_type == pg_sys::NodeTag::T_List {
        let list = PgList::<pg_sys::Node>::from_pg(node as *mut pg_sys::List);
        let mut children = Vec::new();
        for item in list.iter_ptr() {
            let child_expr =
                transform_to_search_expr(root, item, sources, plan, multi_table_predicate_clauses)?;
            children.push(child_expr);
        }
        return if children.is_empty() {
            None
        } else if children.len() == 1 {
            Some(children.pop().unwrap())
        } else {
            Some(JoinLevelExpr::And(children))
        };
    }

    let search_op = anyelement_query_input_opoid();
    let has_search_op = expr_contains_any_operator(node, &[search_op]);

    // Check which tables this expression references
    let rtis = expr_collect_rtis(node);
    let mut referenced_source_indices = Vec::new();

    for (i, source) in sources.iter().enumerate() {
        if rtis.iter().any(|&rti| source.contains_rti(rti)) {
            referenced_source_indices.push(i);
        }
    }

    // If this is a single-table expression with search predicate, extract as a single
    // Tantivy search predicate so that table-local negation (with NULL-preserving exists guards),
    // conjunctions, and disjunctions are evaluated natively by Tantivy.
    if has_search_op && rtis.len() == 1 && referenced_source_indices.len() == 1 {
        let rti = *rtis.iter().next().unwrap();
        let source = &sources[referenced_source_indices[0]];
        let plan_position = source.plan_position;

        // Extract the Tantivy query for this expression
        if let Some(base_info) = find_base_info_recursive(source, rti)
            && let Some(pred) = extract_single_table_predicate(root, rti, &base_info, node)
        {
            return Some(JoinLevelExpr::SingleTablePredicate {
                plan_position,
                predicate: Box::new(pred),
            });
        }
        return None;
    }

    // If this is a cross-relation expression or unnest filter WITHOUT search predicate, create MultiTablePredicate
    let references_unnest =
        plan.is_some_and(|p| rtis.iter().any(|&rti| p.find_lateral_unnest(rti).is_some()));

    if !has_search_op && (referenced_source_indices.len() > 1 || references_unnest) {
        if !all_vars_are_fast_fields_recursive(node, sources, plan) {
            return None;
        }

        if !PredicateTranslator::can_translate(Some(root), sources, node, plan) {
            return None;
        }

        let pg_node_string = crate::postgres::deparse::node_to_string_owned(node.cast());
        multi_table_predicate_clauses.push(node as *mut pg_sys::Expr);
        return Some(JoinLevelExpr::MultiTablePredicate {
            predicate: Box::new(
                crate::postgres::customscan::joinscan::build::MultiTablePredicateInfo {
                    pg_node_string,
                },
            ),
        });
    }

    // If this is a cross-table BoolExpr, preserve its boolean structure (AND, OR, NOT)
    // in JoinLevelExpr so it can be translated into DataFusion's boolean expressions.
    if node_type == pg_sys::NodeTag::T_BoolExpr {
        let boolexpr = node as *mut pg_sys::BoolExpr;
        let boolop = (*boolexpr).boolop;
        let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);

        match boolop {
            pg_sys::BoolExprType::AND_EXPR | pg_sys::BoolExprType::OR_EXPR => {
                let mut children = Vec::new();
                for arg in args.iter_ptr() {
                    let child_expr = transform_to_search_expr(
                        root,
                        arg,
                        sources,
                        plan,
                        multi_table_predicate_clauses,
                    )?;
                    children.push(child_expr);
                }
                if children.is_empty() {
                    None
                } else if children.len() == 1 {
                    Some(children.pop().unwrap())
                } else if boolop == pg_sys::BoolExprType::AND_EXPR {
                    Some(JoinLevelExpr::And(children))
                } else {
                    Some(JoinLevelExpr::Or(children))
                }
            }
            pg_sys::BoolExprType::NOT_EXPR => {
                if let Some(arg) = args.iter_ptr().next()
                    && let Some(child_expr) = transform_to_search_expr(
                        root,
                        arg,
                        sources,
                        plan,
                        multi_table_predicate_clauses,
                    )
                {
                    return Some(JoinLevelExpr::Not(Box::new(child_expr)));
                }
                None
            }
            _ => None,
        }
    } else {
        None
    }
}

pub unsafe fn find_base_info_recursive(
    source: &JoinSource,
    rti: pg_sys::Index,
) -> Option<ScanInfo> {
    if source.contains_rti(rti) {
        Some(source.scan_info.clone())
    } else {
        None
    }
}

/// Extract a single-table predicate from an expression.
pub unsafe fn extract_single_table_predicate(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    side: &ScanInfo,
    expr: *mut pg_sys::Node,
) -> Option<crate::postgres::customscan::joinscan::build::JoinLevelSearchPredicate> {
    let indexrelid = side.indexrelid;
    let heaprelid = side.heaprelid;
    let (_, bm25_idx) = rel_get_bm25_index(heaprelid)?;

    // Create a RestrictInfo wrapping the expression for extract_quals
    let mut ri_list = PgList::<pg_sys::RestrictInfo>::new();
    let fake_ri =
        pg_sys::palloc0(std::mem::size_of::<pg_sys::RestrictInfo>()) as *mut pg_sys::RestrictInfo;
    (*fake_ri).type_ = pg_sys::NodeTag::T_RestrictInfo;
    (*fake_ri).clause = expr.cast();
    ri_list.push(fake_ri);

    let context = PlannerContext::from_planner(root);
    let mut state = QualExtractState::default();

    let qual = extract_quals(
        &context,
        rti,
        ri_list.as_ptr().cast(),
        RestrictInfoType::BaseRelation,
        &bm25_idx,
        false,
        &mut state,
        false,
    )?;

    let query = SearchQueryInput::from(&qual);
    Some(
        crate::postgres::customscan::joinscan::build::JoinLevelSearchPredicate {
            rti,
            indexrelid,
            heaprelid,
            query,
        },
    )
}

/// Check if all Var references in an expression are fast fields.
pub unsafe fn all_vars_are_fast_fields_recursive(
    node: *mut pg_sys::Node,
    sources: &[&JoinSource],
    plan: Option<&crate::postgres::customscan::joinscan::build::RelNode>,
) -> bool {
    let vars = expr_collect_vars(node, false);

    for var_ref in vars {
        let mut source_found = false;
        for source in sources {
            if source.contains_rti(var_ref.rti) {
                if let Some(base_info) = find_base_info_recursive(source, var_ref.rti) {
                    let heaprel = PgSearchRelation::open(base_info.heaprelid);
                    let indexrel = PgSearchRelation::open(base_info.indexrelid);
                    if resolve_fast_field(var_ref.attno as i32, &heaprel.tuple_desc(), &indexrel)
                        .is_none()
                    {
                        return false;
                    }
                } else {
                    return false;
                }
                source_found = true;
                break;
            }
        }
        if !source_found
            && let Some(plan) = plan
            && let Some(unnest_info) = plan.find_lateral_unnest(var_ref.rti)
            && sources
                .iter()
                .any(|s| s.contains_rti(unnest_info.source_rti.0))
        {
            source_found = true;
        }
        if !source_found {
            return false;
        }
    }

    true
}

/// The outcome of validating and absorbing conditions for a join node.
pub struct ResolvedJoinConditions {
    /// Serialized expression absorbed into `JoinNode.filter` to be evaluated
    /// during the join (e.g. non-equi join conditions, outer-join ON conditions,
    /// or semi/anti join conditions).
    pub filter: Option<JoinLevelExpr>,
    /// Conditions that could not be absorbed into the join filter but are safe
    /// to evaluate post-join (e.g. search operators `@@@` or WHERE-clause predicates
    /// on inner/outer joins).
    pub post_join_conditions: Vec<*mut pg_sys::RestrictInfo>,
}

/// Validates join conditions and absorbs translatable non-equi conditions
/// into `JoinNode.filter`.
///
/// Returns `Ok(ResolvedJoinConditions)` if all join conditions are legal for the join type,
/// or `Err(JoinDeclineReason)` if any condition violates join semantics:
/// - Outer join ON conditions (`is_pushed_down == false`) that cannot be absorbed into `filter`
///   cannot be evaluated post-join without dropping null-extended rows.
/// - Semi and Anti join conditions cannot be evaluated post-join because the inner relation's
///   columns are not projected by the join.
/// - Keyless joins require at least one condition absorbed into `filter`.
pub unsafe fn resolve_join_conditions(
    root: *mut pg_sys::PlannerInfo,
    sources: &[&JoinSource],
    equi_keys: &[super::build::JoinKeyPair],
    other_conditions: &[*mut pg_sys::RestrictInfo],
    jointype: pg_sys::JoinType::Type,
) -> Result<ResolvedJoinConditions, super::JoinDeclineReason> {
    let is_outer = matches!(
        jointype,
        pg_sys::JoinType::JOIN_LEFT | pg_sys::JoinType::JOIN_RIGHT | pg_sys::JoinType::JOIN_FULL
    );
    let is_semi_or_anti = matches!(
        jointype,
        pg_sys::JoinType::JOIN_SEMI | pg_sys::JoinType::JOIN_ANTI
    );
    #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
    let is_semi_or_anti = is_semi_or_anti || jointype == pg_sys::JoinType::JOIN_RIGHT_ANTI;
    #[cfg(feature = "pg18")]
    let is_semi_or_anti = is_semi_or_anti || jointype == pg_sys::JoinType::JOIN_RIGHT_SEMI;

    let search_op = anyelement_query_input_opoid();
    let mut absorbed_clauses: Vec<*mut pg_sys::Node> = Vec::new();
    let mut unabsorbed: Vec<*mut pg_sys::RestrictInfo> = Vec::with_capacity(other_conditions.len());

    for &ri in other_conditions {
        let clause = (*ri).clause;
        // Skip `@@@` (and any search ops): search clauses pass through to
        // `extract_join_level_conditions`, where `transform_to_search_expr` handles them.
        if !clause.is_null() && expr_contains_any_operator(clause.cast(), &[search_op]) {
            unabsorbed.push(ri);
            continue;
        }
        // For outer joins, only absorb ON-clause conditions (is_pushed_down == false);
        // WHERE-clause conditions (is_pushed_down == true) stay as post-join filters.
        if is_outer && (*ri).is_pushed_down {
            unabsorbed.push(ri);
            continue;
        }
        if clause.is_null()
            || contains_exec_param(clause.cast())
            || !all_vars_are_fast_fields_recursive(clause.cast(), sources, None)
            || !PredicateTranslator::can_translate(Some(root), sources, clause.cast(), None)
        {
            unabsorbed.push(ri);
            continue;
        }
        absorbed_clauses.push(clause.cast());
    }

    let filter = match absorbed_clauses.len() {
        0 => None,
        _ => {
            // Combine multiple absorbed clauses into a single AND at
            // the PG node level so we serialize one expression tree.
            let combined_node: *mut pg_sys::Node = if absorbed_clauses.len() == 1 {
                absorbed_clauses[0]
            } else {
                let mut list = PgList::<pg_sys::Expr>::new();
                for n in &absorbed_clauses {
                    list.push((*n).cast());
                }
                pg_sys::make_andclause(list.into_pg()).cast()
            };
            let pg_node_string =
                crate::postgres::deparse::node_to_string_owned(combined_node.cast());
            let input_vars =
                crate::postgres::customscan::joinscan::collect_input_vars(combined_node);
            Some(JoinLevelExpr::PgExpression {
                pg_node_string,
                input_vars,
            })
        }
    };

    // Identify unabsorbed conditions that cannot be evaluated post-join:
    // - Outer joins: ON conditions (is_pushed_down == false) must be evaluated during the join.
    // - Semi / Anti joins: inner columns are not projected post-join, so all conditions must be evaluated during the join.
    // - Keyless joins: when equi_keys is empty, at least one condition must be absorbed as a join filter.
    let (illegal_residuals, context) = if is_outer {
        (
            unabsorbed
                .iter()
                .copied()
                .filter(|&ri| !(*ri).is_pushed_down)
                .collect::<Vec<_>>(),
            "outer join ON clauses",
        )
    } else if is_semi_or_anti {
        (unabsorbed.clone(), "semi/anti join conditions")
    } else if equi_keys.is_empty() && !other_conditions.is_empty() && filter.is_none() {
        (unabsorbed.clone(), "join conditions")
    } else {
        (Vec::new(), "")
    };

    let must_decline =
        !illegal_residuals.is_empty() || (equi_keys.is_empty() && is_outer && filter.is_none());

    if must_decline {
        if illegal_residuals.iter().any(|&ri| {
            let clause = (*ri).clause;
            !clause.is_null()
                && crate::postgres::utils::expr_contains_any_operator(clause.cast(), &[search_op])
        }) {
            return Err(super::JoinDeclineReason::new(format!(
                "JoinScan not used: search operators in {context} are not supported"
            )));
        }
        if illegal_residuals.iter().any(|&ri| {
            let clause = (*ri).clause;
            !clause.is_null()
                && crate::postgres::customscan::collation_semantics::expr_has_unsupported_collation(
                    clause.cast(),
                )
        }) {
            return Err(super::JoinDeclineReason::new(
                "JoinScan not used: join conditions on a nondeterministic collation are not supported",
            ));
        }
        return Err(super::JoinDeclineReason::new(
            "JoinScan not used: join conditions must reference columnar indexed fields",
        ));
    }

    Ok(ResolvedJoinConditions {
        filter,
        post_join_conditions: unabsorbed,
    })
}
