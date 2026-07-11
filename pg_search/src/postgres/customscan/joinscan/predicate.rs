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

use super::build::{
    FilterNode, JoinCSClause, JoinLevelExpr, JoinNode, JoinSource, RelNode, ScanInfo,
};
use crate::api::operator::anyelement_query_input_opoid;
use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
use crate::postgres::customscan::datafusion::explain::format_expr_for_explain;
use crate::postgres::customscan::datafusion::translator::PredicateTranslator;
use crate::postgres::customscan::pullup::resolve_fast_field;
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::deparse::deparse_expr;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::utils::{expr_collect_rtis, expr_collect_vars, expr_contains_any_operator};
use crate::query::SearchQueryInput;
use pgrx::{pg_sys, PgList};

/// Extract join-level conditions from the restrict list and transform them into
/// a `JoinLevelExpr` tree.
///
/// This function processes the join's restrict list to identify:
/// - Search predicates (@@@ operator): transformed into Predicate nodes
/// - Cross-relation conditions: transformed into MultiTablePredicate nodes
/// - Boolean expressions: recursively processed to preserve structure
///
/// `JoinNode.absorbed_search_clauses` carries `@@@` `RestrictInfo`s parked
/// during sub-join reconstruction. This is the first point a `JoinCSClause`
/// exists to receive the interned predicates, so we drain them here before
/// the regular `extra->restrictlist` walk.
///
/// Returns the updated JoinCSClause and a list of heap condition clause pointers
/// (in the same order as multi_table_predicates in the clause) for adding to custom_exprs.
pub unsafe fn extract_join_level_conditions(
    root: *mut pg_sys::PlannerInfo,
    extra: *mut pg_sys::JoinPathExtraData,
    sources: &[&JoinSource],
    other_conditions: &[*mut pg_sys::RestrictInfo],
    mut join_clause: JoinCSClause,
) -> Result<(JoinCSClause, Vec<*mut pg_sys::Expr>), String> {
    let mut multi_table_predicate_clauses: Vec<*mut pg_sys::Expr> = Vec::new();

    if sources.is_empty() {
        return Ok((join_clause, multi_table_predicate_clauses));
    }

    // The absorbed-clause walk is independent of `extra`: it mutates
    // `join_clause.plan` and `join_clause.join_level_predicates` from each
    // sub-join's `joinrestrictinfo`. PG places each clause at its lowest
    // applicable join, and the absorbed path only runs on Inner sub-joins
    // (the Inner-only gate in `collect_join_sources_join_rel`), so in
    // practice the two passes process disjoint clause sets.
    let new_plan = lower_absorbed_search_clauses(
        root,
        std::mem::take(&mut join_clause.plan),
        &mut join_clause,
        &mut multi_table_predicate_clauses,
    )?;
    join_clause.plan = new_plan;

    if extra.is_null() {
        return Ok((join_clause, multi_table_predicate_clauses));
    }

    let restrictlist = (*extra).restrictlist;
    if restrictlist.is_null() {
        return Ok((join_clause, multi_table_predicate_clauses));
    }

    let search_op = anyelement_query_input_opoid();
    let restrict_infos = PgList::<pg_sys::RestrictInfo>::from_pg(restrictlist);

    // Collect all expressions into the expression tree
    let mut expr_trees: Vec<JoinLevelExpr> = Vec::new();

    // Track which RestrictInfos are heap conditions (by pointer) for index lookup
    let other_cond_set: crate::api::HashSet<usize> =
        other_conditions.iter().map(|&ri| ri as usize).collect();

    for ri in restrict_infos.iter_ptr() {
        if ri.is_null() || (*ri).clause.is_null() {
            continue;
        }

        let clause = (*ri).clause;
        let has_search_op = expr_contains_any_operator(clause.cast(), &[search_op]);

        if has_search_op {
            if let Some(expr) = transform_to_search_expr(
                root,
                clause.cast(),
                sources,
                &mut join_clause,
                &mut multi_table_predicate_clauses,
            ) {
                expr_trees.push(expr);
            } else {
                return Err(format!(
                    "Failed to transform search predicate into expression tree: {}",
                    format_expr_for_explain(clause.cast()).as_str()
                ));
            }
        } else if other_cond_set.contains(&(ri as usize)) {
            // This is a top-level heap condition (cross-relation, no search operator)
            // Only accept if all referenced columns are fast fields
            if !all_vars_are_fast_fields_recursive(clause.cast(), sources) {
                return Err(format!(
                    "Multi-table predicate '{}' references non-fast-field columns",
                    format_expr_for_explain(clause.cast())
                ));
            }

            // Check if the predicate can be translated to DataFusion
            if !PredicateTranslator::can_translate(sources, clause.cast()) {
                return Err(format!(
                    "Multi-table predicate '{}' cannot be executed by DataFusion (unsupported operator or type)",
                    format_expr_for_explain(clause.cast())
                ));
            }

            // Create a MultiTablePredicate leaf node
            let description = format_expr_for_explain(clause.cast());
            let predicate_idx = join_clause
                .add_multi_table_predicate(description, multi_table_predicate_clauses.len());
            multi_table_predicate_clauses.push(clause);
            expr_trees.push(JoinLevelExpr::MultiTablePredicate { predicate_idx });
        }
    }

    // Combine all expressions with AND
    if !expr_trees.is_empty() {
        let final_expr = if expr_trees.len() == 1 {
            expr_trees.pop().unwrap()
        } else {
            JoinLevelExpr::And(expr_trees)
        };
        join_clause = join_clause.with_join_level_expr(final_expr);
    }

    Ok((join_clause, multi_table_predicate_clauses))
}

/// Recursively transform a PostgreSQL expression with search predicates into a JoinLevelExpr.
///
/// - For single-table sub-trees with search predicates: extract as a Predicate leaf
/// - For cross-relation sub-trees without search predicates: extract as a MultiTablePredicate leaf
/// - For BoolExpr (AND/OR/NOT): recursively transform children
///
/// Also collects heap condition clause pointers into `multi_table_predicate_clauses` for adding
/// to custom_exprs during plan_custom_path.
#[allow(clippy::too_many_arguments)]
pub unsafe fn transform_to_search_expr(
    root: *mut pg_sys::PlannerInfo,
    node: *mut pg_sys::Node,
    sources: &[&JoinSource],
    join_clause: &mut JoinCSClause,
    multi_table_predicate_clauses: &mut Vec<*mut pg_sys::Expr>,
) -> Option<JoinLevelExpr> {
    if node.is_null() {
        return None;
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

    // If this is a single-table expression with search predicate, extract as Predicate
    if has_search_op && rtis.len() == 1 && referenced_source_indices.len() == 1 {
        let rti = *rtis.iter().next().unwrap();
        let source = &sources[referenced_source_indices[0]];
        let plan_position = source.plan_position;

        // Extract the Tantivy query for this expression
        if let Some(base_info) = find_base_info_recursive(source, rti) {
            if let Some(predicate_idx) =
                extract_single_table_predicate(root, rti, &base_info, node, join_clause)
            {
                return Some(JoinLevelExpr::SingleTablePredicate {
                    plan_position,
                    predicate_idx,
                });
            }
        }
        return None;
    }

    // If this is a cross-relation expression WITHOUT search predicate, create MultiTablePredicate
    if !has_search_op && referenced_source_indices.len() > 1 {
        if !all_vars_are_fast_fields_recursive(node, sources) {
            return None;
        }

        let translator = PredicateTranslator::new(sources);
        translator.translate(node)?;

        let description = format_expr_for_explain(node);
        let predicate_idx =
            join_clause.add_multi_table_predicate(description, multi_table_predicate_clauses.len());
        multi_table_predicate_clauses.push(node as *mut pg_sys::Expr);
        return Some(JoinLevelExpr::MultiTablePredicate { predicate_idx });
    }

    // Handle List nodes: Postgres may wrap quals in a List (common on PG18).
    // Treat as an implicit AND of the list elements.
    let node_type = (*node).type_;
    if node_type == pg_sys::NodeTag::T_List {
        let list = PgList::<pg_sys::Node>::from_pg(node as *mut pg_sys::List);
        let mut children = Vec::new();
        for item in list.iter_ptr() {
            let child_expr = transform_to_search_expr(
                root,
                item,
                sources,
                join_clause,
                multi_table_predicate_clauses,
            )?;
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

    // Handle BoolExpr
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
                        join_clause,
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
                if let Some(arg) = args.iter_ptr().next() {
                    if let Some(child_expr) = transform_to_search_expr(
                        root,
                        arg,
                        sources,
                        join_clause,
                        multi_table_predicate_clauses,
                    ) {
                        return Some(JoinLevelExpr::Not(Box::new(child_expr)));
                    }
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

/// Extract a single-table predicate and add it to the join clause.
/// Returns the index of the predicate in join_level_predicates, or None if extraction fails.
pub unsafe fn extract_single_table_predicate(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    side: &ScanInfo,
    expr: *mut pg_sys::Node,
    join_clause: &mut JoinCSClause,
) -> Option<usize> {
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

    // Eagerly deparse the expression for EXPLAIN output while planner pointers are valid
    let display_str = {
        let context = PlannerContext::from_planner(root);
        let index_rel = PgSearchRelation::open(indexrelid);
        deparse_expr(Some(&context), &index_rel, expr)
    };

    let idx = join_clause.add_join_level_predicate(rti, indexrelid, heaprelid, query, display_str);
    Some(idx)
}

/// Sub-join reconstruction stashes `@@@` `RestrictInfo`s onto
/// `JoinNode.absorbed_search_clauses` without lowering them, because no
/// `JoinCSClause` exists yet to receive interned `plan_position`s. Once one
/// does, walking the tree converts each entry into a `RelNode::Filter`
/// wrapping the absorbing `JoinNode`.
pub(super) unsafe fn lower_absorbed_search_clauses(
    root: *mut pg_sys::PlannerInfo,
    node: RelNode,
    join_clause: &mut JoinCSClause,
    multi_table_predicate_clauses: &mut Vec<*mut pg_sys::Expr>,
) -> Result<RelNode, String> {
    match node {
        RelNode::Scan(s) => Ok(RelNode::Scan(s)),
        RelNode::Filter(f) => {
            let FilterNode { input, predicate } = *f;
            let input = lower_absorbed_search_clauses(
                root,
                input,
                join_clause,
                multi_table_predicate_clauses,
            )?;
            Ok(RelNode::Filter(Box::new(FilterNode { input, predicate })))
        }
        RelNode::Join(j) => {
            let JoinNode {
                join_type,
                left,
                right,
                equi_keys,
                filter,
                subplan_id,
                absorbed_search_clauses,
            } = *j;
            let left = lower_absorbed_search_clauses(
                root,
                left,
                join_clause,
                multi_table_predicate_clauses,
            )?;
            let right = lower_absorbed_search_clauses(
                root,
                right,
                join_clause,
                multi_table_predicate_clauses,
            )?;

            if absorbed_search_clauses.is_empty() {
                return Ok(RelNode::Join(Box::new(JoinNode {
                    join_type,
                    left,
                    right,
                    equi_keys,
                    filter,
                    subplan_id,
                    absorbed_search_clauses: Vec::new(),
                })));
            }

            // PG anchors `RestrictInfo`s against RTIs from the sub-tree, so
            // resolve against everything reachable below this join.
            let mut sub_sources = left.sources();
            sub_sources.extend(right.sources());

            let predicate = build_absorbed_filter(
                root,
                &sub_sources,
                &absorbed_search_clauses,
                join_clause,
                multi_table_predicate_clauses,
            )?;
            Ok(RelNode::Filter(Box::new(FilterNode {
                input: RelNode::Join(Box::new(JoinNode {
                    join_type,
                    left,
                    right,
                    equi_keys,
                    filter,
                    subplan_id,
                    absorbed_search_clauses: Vec::new(),
                })),
                predicate,
            })))
        }
    }
}

/// `absorbed` was populated from a live `joinrestrictinfo` earlier in the
/// same planning pass, so every entry should still translate. We error
/// rather than skip so a future refactor that drops a clause on the floor
/// blows up the test suite instead of producing wrong rows.
unsafe fn build_absorbed_filter(
    root: *mut pg_sys::PlannerInfo,
    sub_sources: &[&JoinSource],
    absorbed: &[*mut pg_sys::RestrictInfo],
    join_clause: &mut JoinCSClause,
    multi_table_predicate_clauses: &mut Vec<*mut pg_sys::Expr>,
) -> Result<JoinLevelExpr, String> {
    let expr_trees: Vec<JoinLevelExpr> = absorbed
        .iter()
        .copied()
        .map(|ri| {
            if ri.is_null() {
                return Err("absorbed search clause is a null RestrictInfo".to_string());
            }
            let clause = (*ri).clause;
            if clause.is_null() {
                return Err("absorbed search clause has a null clause".to_string());
            }
            transform_to_search_expr(
                root,
                clause.cast(),
                sub_sources,
                join_clause,
                multi_table_predicate_clauses,
            )
            .ok_or_else(|| {
                format!(
                    "Failed to lower absorbed search clause: {}",
                    format_expr_for_explain(clause.cast()).as_str()
                )
            })
        })
        .collect::<Result<_, _>>()?;

    // Caller guarantees `absorbed` is non-empty; `collect` either yielded
    // N entries or short-circuited with `Err`. An empty result here would
    // otherwise lower to `And(vec![])`, which evaluates to TRUE and
    // silently wipes the WHERE.
    match expr_trees.len() {
        0 => Err("absorbed clause set lowered to empty expr tree".to_string()),
        1 => Ok(expr_trees.into_iter().next().unwrap()),
        _ => Ok(JoinLevelExpr::And(expr_trees)),
    }
}

/// Check if all Var references in an expression are fast fields.
pub unsafe fn all_vars_are_fast_fields_recursive(
    node: *mut pg_sys::Node,
    sources: &[&JoinSource],
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
        if !source_found {
            return false;
        }
    }

    true
}
