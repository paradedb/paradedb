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
    FilterNode, JoinCSClause, JoinLevelExpr, JoinNode, JoinSource, RelNode, ScanInfo, UnnestNode,
};
use crate::api::operator::anyelement_query_input_opoid;
use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
use crate::postgres::customscan::datafusion::translator::PredicateTranslator;
use crate::postgres::customscan::pullup::resolve_fast_field;
use crate::postgres::customscan::qual_inspect::{
    contains_exec_param, extract_quals, PlannerContext, QualExtractState,
};
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
        &mut multi_table_predicate_clauses,
    )?;
    join_clause.plan = new_plan;

    let search_op = anyelement_query_input_opoid();
    let mut all_restrict_infos: Vec<*mut pg_sys::RestrictInfo> = Vec::new();
    if !extra.is_null() && !(*extra).restrictlist.is_null() {
        let restrict_infos = PgList::<pg_sys::RestrictInfo>::from_pg((*extra).restrictlist);
        for ri in restrict_infos.iter_ptr() {
            if !ri.is_null() && !(*ri).clause.is_null() {
                all_restrict_infos.push(ri);
            }
        }
    }
    for ri in other_conditions {
        if !ri.is_null() && !(**ri).clause.is_null() && !all_restrict_infos.contains(ri) {
            all_restrict_infos.push(*ri);
        }
    }

    // Collect all expressions into the expression tree
    let mut expr_trees: Vec<JoinLevelExpr> = Vec::new();

    // Track which RestrictInfos are heap conditions (by pointer) for index lookup
    let other_cond_set: crate::api::HashSet<usize> =
        other_conditions.iter().map(|&ri| ri as usize).collect();

    for ri in &all_restrict_infos {
        let clause = (**ri).clause;
        let has_search_op = expr_contains_any_operator(clause.cast(), &[search_op]);

        if has_search_op {
            if let Some(expr) = transform_to_search_expr(
                root,
                clause.cast(),
                sources,
                &mut multi_table_predicate_clauses,
            ) {
                expr_trees.push(expr);
            } else {
                let formatted =
                    crate::postgres::deparse::deparse_planner_expr_or_raw(root, clause.cast());
                return Err(format!(
                    "Failed to transform search predicate into expression tree: {}",
                    formatted
                ));
            }
        } else if other_cond_set.contains(&(*ri as usize)) {
            // This is a top-level heap condition (cross-relation or unnest filter, no search operator)
            // Only accept if all referenced columns are fast fields
            if !all_vars_are_fast_fields_recursive(clause.cast(), sources, Some(&join_clause.plan))
            {
                let formatted =
                    crate::postgres::deparse::deparse_planner_expr_or_raw(root, clause.cast());
                return Err(format!(
                    "Multi-table predicate '{}' references non-columnar columns",
                    formatted
                ));
            }

            // Check if the predicate can be translated to DataFusion
            if !PredicateTranslator::can_translate(
                Some(root),
                sources,
                clause.cast(),
                Some(&join_clause.plan),
            ) {
                let formatted =
                    crate::postgres::deparse::deparse_planner_expr_or_raw(root, clause.cast());
                return Err(format!(
                    "Multi-table predicate '{}' cannot be executed by DataFusion (unsupported operator or type)",
                    formatted
                ));
            }

            // Create a MultiTablePredicate leaf node
            let pg_node_string = crate::postgres::deparse::node_to_string_owned(clause.cast());
            multi_table_predicate_clauses.push(clause);
            expr_trees.push(JoinLevelExpr::MultiTablePredicate {
                predicate: Box::new(
                    crate::postgres::customscan::joinscan::build::MultiTablePredicateInfo {
                        pg_node_string,
                    },
                ),
            });
        }
    }

    // Fallback: parse-tree `(*jointree).quals` - PG sometimes leaves cross-table
    // predicates here when outer joins (e.g., LATERAL unnest) prevent them from
    // being pushed into joinrestrictinfo.
    let parse = (*root).parse;
    if !parse.is_null() && !(*parse).jointree.is_null() && !(*(*parse).jointree).quals.is_null() {
        let mut conjuncts = Vec::new();
        crate::postgres::customscan::qual_inspect::collect_implicit_and_conjuncts(
            (*(*parse).jointree).quals,
            &mut conjuncts,
        );
        let valid_rtis: Vec<pg_sys::Index> = sources.iter().map(|s| s.scan_info.heap_rti).collect();
        for conjunct in conjuncts {
            let rtis = expr_collect_rtis(conjunct);
            // Only process cross-table predicates whose referenced RTIs are all in `sources`
            if rtis.len() > 1 && rtis.iter().all(|rti| join_clause.plan.contains_rti(*rti)) {
                // Check if this conjunct was already extracted as an equi-key
                if super::build::try_extract_equi_key(conjunct.cast(), &valid_rtis).is_some() {
                    continue;
                }
                // Check if this conjunct is already present in all_restrict_infos or multi_table_predicate_clauses
                let already_present = all_restrict_infos.iter().any(|ri| {
                    std::ptr::eq((**ri).clause.cast::<pg_sys::Node>(), conjunct)
                        || pg_sys::equal((**ri).clause.cast(), conjunct.cast())
                }) || multi_table_predicate_clauses.iter().any(|c| {
                    std::ptr::eq((*c).cast::<pg_sys::Node>(), conjunct)
                        || pg_sys::equal((*c).cast(), conjunct.cast())
                });
                if already_present {
                    continue;
                }

                let has_search_op = expr_contains_any_operator(conjunct.cast(), &[search_op]);
                if has_search_op {
                    if let Some(expr) = transform_to_search_expr(
                        root,
                        conjunct.cast(),
                        sources,
                        &mut multi_table_predicate_clauses,
                    ) {
                        expr_trees.push(expr);
                    } else {
                        let formatted =
                            crate::postgres::deparse::deparse_planner_expr(root, conjunct.cast())
                                .unwrap_or_else(|| {
                                    crate::postgres::deparse::node_to_string_without_context(
                                        conjunct.cast(),
                                    )
                                });
                        return Err(format!(
                            "Failed to transform search predicate into expression tree: {}",
                            formatted
                        ));
                    }
                } else {
                    if !all_vars_are_fast_fields_recursive(
                        conjunct.cast(),
                        sources,
                        Some(&join_clause.plan),
                    ) {
                        let formatted =
                            crate::postgres::deparse::deparse_planner_expr(root, conjunct.cast())
                                .unwrap_or_else(|| {
                                    crate::postgres::deparse::node_to_string_without_context(
                                        conjunct.cast(),
                                    )
                                });
                        return Err(format!(
                            "Multi-table predicate '{}' references non-fast-field columns",
                            formatted
                        ));
                    }

                    if !PredicateTranslator::can_translate(
                        Some(root),
                        sources,
                        conjunct.cast(),
                        Some(&join_clause.plan),
                    ) {
                        let formatted =
                            crate::postgres::deparse::deparse_planner_expr(root, conjunct.cast())
                                .unwrap_or_else(|| {
                                    crate::postgres::deparse::node_to_string_without_context(
                                        conjunct.cast(),
                                    )
                                });
                        return Err(format!(
                            "Multi-table predicate '{}' cannot be executed by DataFusion (unsupported operator or type)",
                            formatted
                        ));
                    }

                    let pg_node_string = {
                        let node_str = pg_sys::nodeToString(conjunct.cast());
                        std::ffi::CStr::from_ptr(node_str)
                            .to_string_lossy()
                            .into_owned()
                    };
                    multi_table_predicate_clauses.push(conjunct.cast());
                    expr_trees.push(JoinLevelExpr::MultiTablePredicate {
                        predicate: Box::new(
                            crate::postgres::customscan::joinscan::build::MultiTablePredicateInfo {
                                pg_node_string,
                            },
                        ),
                    });
                }
            }
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

    join_clause.assign_tagged_queries();

    Ok((join_clause, multi_table_predicate_clauses))
}

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
                transform_to_search_expr(root, item, sources, multi_table_predicate_clauses)?;
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
        if let Some(base_info) = find_base_info_recursive(source, rti) {
            if let Some(pred) = extract_single_table_predicate(root, rti, &base_info, node) {
                return Some(JoinLevelExpr::SingleTablePredicate {
                    plan_position,
                    predicate: Box::new(pred),
                });
            }
        }
        return None;
    }

    // If this is a cross-relation expression WITHOUT search predicate, create MultiTablePredicate
    if !has_search_op && referenced_source_indices.len() > 1 {
        if !all_vars_are_fast_fields_recursive(node, sources, None) {
            return None;
        }

        let translator = PredicateTranslator::new(sources).with_planner_info(root);
        translator.translate(node)?;

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
                    if let Some(child_expr) =
                        transform_to_search_expr(root, arg, sources, multi_table_predicate_clauses)
                    {
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

/// Sub-join reconstruction stashes `@@@` `RestrictInfo`s onto
/// `JoinNode.absorbed_search_clauses` without lowering them, because no
/// `JoinCSClause` exists yet to receive interned `plan_position`s. Once one
/// does, walking the tree converts each entry into a `RelNode::Filter`
/// wrapping the absorbing `JoinNode`.
pub(super) unsafe fn lower_absorbed_search_clauses(
    root: *mut pg_sys::PlannerInfo,
    node: RelNode,
    multi_table_predicate_clauses: &mut Vec<*mut pg_sys::Expr>,
) -> Result<RelNode, String> {
    match node {
        RelNode::Scan(s) => Ok(RelNode::Scan(s)),
        RelNode::Filter(f) => {
            let FilterNode { input, predicate } = *f;
            let input = lower_absorbed_search_clauses(root, input, multi_table_predicate_clauses)?;
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
            let left = lower_absorbed_search_clauses(root, left, multi_table_predicate_clauses)?;
            let right = lower_absorbed_search_clauses(root, right, multi_table_predicate_clauses)?;

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
        RelNode::Unnest(u) => {
            let UnnestNode {
                input,
                unnest_info,
                absorbed_clauses,
            } = *u;
            let input = lower_absorbed_search_clauses(root, input, multi_table_predicate_clauses)?;
            let unnest_node = RelNode::Unnest(Box::new(UnnestNode {
                input,
                unnest_info,
                absorbed_clauses: Vec::new(),
            }));
            if absorbed_clauses.is_empty() {
                return Ok(unnest_node);
            }
            let sub_sources = unnest_node.sources();
            let predicate = build_absorbed_filter(
                root,
                &sub_sources,
                &absorbed_clauses,
                multi_table_predicate_clauses,
            )?;
            Ok(RelNode::Filter(Box::new(FilterNode {
                input: unnest_node,
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
                multi_table_predicate_clauses,
            )
            .ok_or_else(|| {
                let formatted = crate::postgres::deparse::deparse_planner_expr(root, clause.cast())
                    .unwrap_or_else(|| {
                        crate::postgres::deparse::node_to_string_without_context(clause.cast())
                    });
                format!("Failed to lower absorbed search clause: {}", formatted)
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
