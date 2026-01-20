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

pub mod build;
pub mod executors;
pub mod privdat;
pub mod scan_state;

use self::build::{
    ExecutionHints, HeapConditionInfo, JoinAlgorithmHint, JoinCSClause, JoinKeyPair, JoinSideInfo,
    SerializableJoinLevelExpr, SerializableJoinSide, SerializableJoinType,
};
use self::privdat::PrivateData;
use self::scan_state::{JoinLevelExpr, JoinScanState, JoinSide};
use crate::api::operator::anyelement_query_input_opoid;
// Note: MvccSatisfies, SearchIndexReader moved to execution commit
use crate::nodecast;
use crate::postgres::customscan::basescan::projections::score::{is_score_func, uses_scores};
use crate::postgres::customscan::builders::custom_path::{
    CustomPathBuilder, Flags, OrderByStyle, RestrictInfoType,
};
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::customscan::range_table::{bms_iter, get_plain_relation_relid};
use crate::postgres::customscan::score_funcoids;
use crate::postgres::customscan::{CustomScan, ExecMethod, JoinPathlistHookArgs, PlainExecCapable};
// Note: OwnedVisibilityChecker, VisibilityChecker moved to execution commit
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::utils::{expr_collect_rtis, expr_contains_any_operator};
use crate::query::SearchQueryInput;
use crate::DEFAULT_STARTUP_COST;
use pgrx::{pg_sys, PgList};
use std::ffi::CStr;

#[derive(Default)]
pub struct JoinScan;

/// Result of extracting join conditions from the restrict list.
struct JoinConditions {
    /// Equi-join keys with type info for composite key extraction.
    equi_keys: Vec<JoinKeyPair>,
    /// Other join conditions (non-equijoin) that need to be evaluated after hash lookup.
    /// These are the RestrictInfo nodes themselves.
    other_conditions: Vec<*mut pg_sys::RestrictInfo>,
    /// Whether any join-level condition contains our @@@ operator.
    has_search_predicate: bool,
}

impl CustomScan for JoinScan {
    const NAME: &'static CStr = c"ParadeDB Join Scan";
    type Args = JoinPathlistHookArgs;
    type State = JoinScanState;
    type PrivateData = PrivateData;

    fn create_custom_path(builder: CustomPathBuilder<Self>) -> Option<pg_sys::CustomPath> {
        unsafe {
            let args = builder.args();
            let root = args.root;
            let jointype = args.jointype;
            let outerrel = args.outerrel;
            let innerrel = args.innerrel;
            let extra = args.extra;

            // TODO(join-types): Currently only INNER JOIN is supported.
            // Future work should add:
            // - LEFT JOIN: Return NULL for non-matching build rows; track matched driving rows
            // - RIGHT JOIN: Swap driving/build sides, then use LEFT logic
            // - FULL OUTER JOIN: Track unmatched rows on both sides; two-pass or marking approach
            // - SEMI JOIN: Stop after first match per driving row (benefits EXISTS queries)
            // - ANTI JOIN: Return only driving rows with no matches (benefits NOT EXISTS)
            if jointype != pg_sys::JoinType::JOIN_INNER {
                return None;
            }

            // TODO(no-limit): Currently requires LIMIT for JoinScan to be proposed.
            // This is overly restrictive. We should allow no-limit joins when:
            // 1. Both sides have search predicates (Aggregate Score pattern), OR
            // 2. Join-level predicates exist that benefit from index
            //
            // JoinScan currently requires a LIMIT clause. This restriction exists because
            // without a limit, scanning the entire index may not be more efficient than
            // PostgreSQL's native join execution.
            let limit = if (*root).limit_tuples > -1.0 {
                Some((*root).limit_tuples as usize)
            } else {
                return None;
            };

            // Extract information from both sides of the join
            let mut outer_side = extract_join_side_info(root, outerrel)?;
            let mut inner_side = extract_join_side_info(root, innerrel)?;

            // Extract join conditions from the restrict list
            let outer_rti = outer_side.heap_rti.unwrap_or(0);
            let inner_rti = inner_side.heap_rti.unwrap_or(0);
            let join_conditions = extract_join_conditions(extra, outer_rti, inner_rti);

            // Don't propose JoinScan for non-equijoin conditions without equi-join keys.
            // Without equi-join keys, we'd need to do an O(N*M) cross-product scan
            // checking every row pair against the conditions. PostgreSQL's native join
            // handles this more efficiently with its optimized cartesian product logic.
            let has_equi_join_keys = !join_conditions.equi_keys.is_empty();
            let has_non_equijoin_conditions = !join_conditions.other_conditions.is_empty();
            if !has_equi_join_keys && has_non_equijoin_conditions {
                return None;
            }

            // Determine driving side: the side with a search predicate is the driving side
            // (it streams results from Tantivy while the other side builds a hash table)
            let driving_side_is_outer = outer_side.has_search_predicate;
            let driving_side_rti = if driving_side_is_outer {
                outer_rti
            } else {
                inner_rti
            };

            // Check if paradedb.score() is used anywhere in the query for the driving side.
            // This includes ORDER BY, SELECT list, or any other expression.
            let funcoids = score_funcoids();
            let score_pathkey = extract_score_pathkey(root, driving_side_rti as pg_sys::Index);
            let score_in_tlist =
                uses_scores((*root).processed_tlist.cast(), funcoids, driving_side_rti);
            let score_needed = score_pathkey.is_some() || score_in_tlist;

            // Record score_needed in the plan for the executor
            if driving_side_is_outer {
                outer_side = outer_side.with_score_needed(score_needed);
            } else {
                inner_side = inner_side.with_score_needed(score_needed);
            }

            // Build the join clause with join keys
            let mut join_clause = JoinCSClause::new()
                .with_outer_side(outer_side.clone())
                .with_inner_side(inner_side.clone())
                .with_join_type(SerializableJoinType::from(jointype))
                .with_limit(limit);

            // Add extracted equi-join keys with type info
            for jk in join_conditions.equi_keys {
                join_clause = join_clause.add_join_key(
                    jk.outer_attno,
                    jk.inner_attno,
                    jk.type_oid,
                    jk.typlen,
                    jk.typbyval,
                );
            }

            // Extract join-level predicates (search predicates and heap conditions)
            // This builds an expression tree that can reference:
            // - Predicate nodes: Tantivy search queries
            // - HeapCondition nodes: PostgreSQL expressions
            // Returns the updated join_clause and a list of heap condition clause pointers
            let heap_condition_clauses: Vec<*mut pg_sys::Expr>;
            let Ok((mut join_clause, heap_condition_clauses)) = Self::extract_join_level_conditions(
                root,
                extra,
                &outer_side,
                &inner_side,
                &join_conditions.other_conditions,
                join_clause,
            ) else {
                return None;
            };

            // Check if this is a valid join for JoinScan
            // We need at least one side with a BM25 index AND a search predicate,
            // OR successfully extracted join-level predicates.
            // Note: Heap conditions alone don't justify using JoinScan (no search advantage).
            let has_side_predicate = (outer_side.has_bm25_index && outer_side.has_search_predicate)
                || (inner_side.has_bm25_index && inner_side.has_search_predicate);
            let has_join_level_predicates = !join_clause.join_level_predicates.is_empty();

            if !has_side_predicate && !has_join_level_predicates {
                return None;
            }

            // Get the cheapest total paths from outer and inner relations
            // These are needed so PostgreSQL can resolve Vars in custom_scan_tlist
            let outer_path = (*outerrel).cheapest_total_path;
            let inner_path = (*innerrel).cheapest_total_path;

            // Cost model for JoinScan
            // We estimate costs based on:
            // - Driving side: rows after search filtering, limited by LIMIT
            // - Build side: full scan to build hash table
            // - Hash table build cost: sequential scan + hashing
            // - Probe cost: per driving row, hash lookup + heap fetch
            let (startup_cost, total_cost, result_rows, estimated_build_rows) =
                estimate_joinscan_cost(&join_clause, outerrel, innerrel, limit);

            // Compute execution hints based on planning information
            let hints = compute_execution_hints(estimated_build_rows, limit);
            join_clause = join_clause.with_hints(hints);

            // Create the private data with hints included
            let private_data = PrivateData::new(join_clause.clone());

            // Force the path to be chosen when we have a valid join opportunity.
            // TODO: Once cost model is well-tuned, consider removing Flags::Force
            // to let PostgreSQL make cost-based decisions.
            let mut builder = builder
                .set_flag(Flags::Force)
                .set_startup_cost(startup_cost)
                .set_total_cost(total_cost)
                .set_rows(result_rows)
                .add_custom_path(outer_path)
                .add_custom_path(inner_path);

            // Add pathkey if ORDER BY score detected for driving side
            if let Some(ref pathkey) = score_pathkey {
                builder = builder.add_path_key(pathkey);
            }

            let mut custom_path = builder.build(private_data);

            // Store the restrictlist and heap condition clauses in custom_private
            // Structure: [PrivateData JSON, restrictlist, heap_cond_1, heap_cond_2, ...]
            let mut private_list = PgList::<pg_sys::Node>::from_pg(custom_path.custom_private);

            // Add the restrictlist as the second element
            let restrictlist = (*extra).restrictlist;
            if !restrictlist.is_null() {
                private_list.push(restrictlist.cast());
            } else {
                private_list.push(std::ptr::null_mut());
            }

            // Add heap condition clauses as subsequent elements
            for clause in heap_condition_clauses {
                private_list.push(clause.cast());
            }

            custom_path.custom_private = private_list.into_pg();

            Some(custom_path)
        }
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
        // For joins, scanrelid must be 0 (it's not scanning a single relation)
        builder.set_scanrelid(0);

        // Get best_path before builder is consumed
        let best_path = builder.args().best_path;

        let mut node = builder.build();

        unsafe {
            // For joins, we need to set custom_scan_tlist to describe the output columns.
            // Create a fresh copy of the target list to avoid corrupting the original
            let original_tlist = node.scan.plan.targetlist;
            let copied_tlist = pg_sys::copyObjectImpl(original_tlist.cast()).cast::<pg_sys::List>();
            let tlist = PgList::<pg_sys::TargetEntry>::from_pg(copied_tlist);

            // For join custom scans, PostgreSQL doesn't pass clauses via the usual parameter.
            // We stored the restrictlist in custom_private during create_custom_path
            let private_list = PgList::<pg_sys::Node>::from_pg((*best_path).custom_private);

            // Second element (if present) is the restrictlist we stored during create_custom_path
            // Note: We do NOT add restrictlist clauses to custom_exprs because setrefs would try
            // to resolve their Vars using the child plans' target lists, which may not have all
            // the needed columns. Instead, we keep the restrictlist in custom_private and handle
            // join condition evaluation manually during execution using the original Var references.

            // Extract the column mappings from the ORIGINAL targetlist (before we add restrictlist Vars).
            // The original_tlist has the SELECT's output columns, which is what ps_ResultTupleSlot is based on.
            // We store this mapping in PrivateData so build_result_tuple can use it during execution.
            let mut output_columns = Vec::new();

            // Get the outer and inner RTIs from PrivateData
            // Note: custom_private may have [PrivateData JSON, restrictlist]
            // We need to preserve the restrictlist when updating
            let current_private = PgList::<pg_sys::Node>::from_pg(node.custom_private);
            let restrictlist_node = if current_private.len() > 1 {
                current_private.get_ptr(1)
            } else {
                None
            };

            let mut private_data = PrivateData::from(node.custom_private);
            let outer_rti = private_data.join_clause.outer_side.heap_rti.unwrap_or(0);
            let inner_rti = private_data.join_clause.inner_side.heap_rti.unwrap_or(0);

            // Use the ORIGINAL targetlist to extract output_columns, NOT the extended tlist.
            // The original_tlist matches what ps_ResultTupleSlot is built from.
            let original_entries = PgList::<pg_sys::TargetEntry>::from_pg(original_tlist);
            let funcoids = score_funcoids();

            // Determine which RTI has the search predicate (score comes from that side)
            let driving_side_rti = if private_data.join_clause.outer_side.query.is_some() {
                outer_rti
            } else {
                inner_rti
            };

            for te in original_entries.iter_ptr() {
                if (*(*te).expr).type_ == pg_sys::NodeTag::T_Var {
                    let var = (*te).expr as *mut pg_sys::Var;
                    let varno = (*var).varno as pg_sys::Index;
                    let varattno = (*var).varattno;

                    // Determine if this column comes from outer or inner relation
                    let is_outer = varno == outer_rti;
                    output_columns.push(privdat::OutputColumnInfo {
                        is_outer,
                        original_attno: varattno,
                        is_score: false,
                    });
                } else if uses_scores((*te).expr.cast(), funcoids, driving_side_rti) {
                    // This expression contains paradedb.score() for the driving side
                    output_columns.push(privdat::OutputColumnInfo {
                        is_outer: driving_side_rti == outer_rti,
                        original_attno: 0,
                        is_score: true,
                    });
                } else {
                    // Non-Var, non-score expression - mark as null (attno = 0)
                    output_columns.push(privdat::OutputColumnInfo {
                        is_outer: false,
                        original_attno: 0,
                        is_score: false,
                    });
                }
            }

            // Update PrivateData with the output column mapping
            private_data.output_columns = output_columns;

            // Add heap condition clauses to custom_exprs so they get transformed by set_customscan_references.
            // The Vars in these expressions will be converted to INDEX_VAR references into custom_scan_tlist.
            // Heap condition clauses are stored in best_path.custom_private starting at index 2
            // (after PrivateData and restrictlist). Note: we read from best_path, not node, because
            // the builder only copies the PrivateData JSON to node.custom_private, not the full list.
            let path_private = PgList::<pg_sys::Node>::from_pg((*best_path).custom_private);
            let mut custom_exprs = PgList::<pg_sys::Node>::new();
            let num_heap_conditions = private_data.join_clause.heap_conditions.len();

            for i in 0..num_heap_conditions {
                // Index 0 = PrivateData, Index 1 = restrictlist, Index 2+ = heap condition clauses
                let clause_idx = 2 + i;
                if clause_idx < path_private.len() {
                    if let Some(clause_node) = path_private.get_ptr(clause_idx) {
                        if !clause_node.is_null() {
                            // Copy the clause to avoid modifying the original
                            let clause_copy = pg_sys::copyObjectImpl(clause_node.cast()).cast();
                            custom_exprs.push(clause_copy);
                        }
                    }
                }
            }
            node.custom_exprs = custom_exprs.into_pg();

            // Convert PrivateData back to a list and preserve the restrictlist
            let private_data_list: *mut pg_sys::List = private_data.into();
            let mut new_private = PgList::<pg_sys::Node>::from_pg(private_data_list);
            if let Some(rl) = restrictlist_node {
                new_private.push(rl);
            }
            node.custom_private = new_private.into_pg();

            // Set custom_scan_tlist with all needed columns
            node.custom_scan_tlist = tlist.into_pg();
        }

        node
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        // Transfer join clause and output column mapping to scan state
        builder.custom_state().join_clause = builder.custom_private().join_clause.clone();
        builder.custom_state().output_columns = builder.custom_private().output_columns.clone();
        builder.build()
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        _ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    ) {
        let join_clause = &state.custom_state().join_clause;

        // Show join type
        let join_type_str = match join_clause.join_type {
            SerializableJoinType::Inner => "Inner",
            SerializableJoinType::Left => "Left",
            SerializableJoinType::Right => "Right",
            SerializableJoinType::Full => "Full",
            SerializableJoinType::Semi => "Semi",
            SerializableJoinType::Anti => "Anti",
        };
        explainer.add_text("Join Type", join_type_str);

        // Get relation names and aliases for display
        let outer_rel_name = join_clause
            .outer_side
            .heaprelid
            .map(|oid| PgSearchRelation::open(oid).name().to_string())
            .unwrap_or_else(|| "?".to_string());
        let inner_rel_name = join_clause
            .inner_side
            .heaprelid
            .map(|oid| PgSearchRelation::open(oid).name().to_string())
            .unwrap_or_else(|| "?".to_string());

        // Get aliases (use alias if available, otherwise use table name)
        let outer_alias = join_clause
            .outer_side
            .alias
            .as_ref()
            .cloned()
            .unwrap_or_else(|| outer_rel_name.clone());
        let inner_alias = join_clause
            .inner_side
            .alias
            .as_ref()
            .cloned()
            .unwrap_or_else(|| inner_rel_name.clone());

        // Show relation info for both sides (with alias in parentheses if different)
        let outer_display = if join_clause.outer_side.alias.is_some() {
            format!("{} ({})", outer_rel_name, outer_alias)
        } else {
            outer_rel_name.clone()
        };
        let inner_display = if join_clause.inner_side.alias.is_some() {
            format!("{} ({})", inner_rel_name, inner_alias)
        } else {
            inner_rel_name.clone()
        };
        explainer.add_text("Outer Relation", outer_display);
        explainer.add_text("Inner Relation", inner_display);

        // Show join keys (equi-join condition) with column names using aliases
        if !join_clause.join_keys.is_empty() {
            let keys_str: Vec<String> = join_clause
                .join_keys
                .iter()
                .map(|k| {
                    let outer_col = get_attname_safe(
                        join_clause.outer_side.heaprelid,
                        k.outer_attno,
                        &outer_alias,
                    );
                    let inner_col = get_attname_safe(
                        join_clause.inner_side.heaprelid,
                        k.inner_attno,
                        &inner_alias,
                    );
                    format!("{} = {}", outer_col, inner_col)
                })
                .collect();
            explainer.add_text("Join Cond", keys_str.join(", "));
        } else {
            explainer.add_text("Join Cond", "cross join");
        }

        // Show if there are heap conditions (cross-relation filters)
        if join_clause.has_heap_conditions() {
            explainer.add_text(
                "Heap Conditions",
                join_clause.heap_conditions.len().to_string(),
            );
        }

        // Show side-level search predicates with clear labeling
        if join_clause.outer_side.has_search_predicate {
            if let Some(ref query) = join_clause.outer_side.query {
                explainer.add_explainable("Outer Tantivy Query", query);
            }
        }
        if join_clause.inner_side.has_search_predicate {
            if let Some(ref query) = join_clause.inner_side.query {
                explainer.add_explainable("Inner Tantivy Query", query);
            }
        }

        // Show join-level expression tree if present
        if let Some(ref expr) = join_clause.join_level_expr {
            let expr_str = format_join_level_expr(
                expr,
                &join_clause.join_level_predicates,
                &join_clause.heap_conditions,
            );
            explainer.add_text("Join Predicate", expr_str);
        }

        // Show limit if present
        if let Some(limit) = join_clause.limit {
            explainer.add_text("Limit", limit.to_string());
        }

        // Show execution hints from planner
        let hint_str = match join_clause.hints.algorithm {
            JoinAlgorithmHint::Auto => "Auto",
            JoinAlgorithmHint::PreferHash => "Prefer Hash",
        };
        explainer.add_text("Algorithm Hint", hint_str);

        // Show estimated build rows if available
        if let Some(est_rows) = join_clause.hints.estimated_build_rows {
            explainer.add_text("Est. Build Rows", format!("{:.0}", est_rows));
        }

        // For EXPLAIN ANALYZE, show the actual execution method used
        if explainer.is_analyze() {
            let exec_method = if state.custom_state().using_nested_loop {
                "Nested Loop"
            } else {
                "Hash Join"
            };
            explainer.add_text("Exec Method", exec_method);
        }
    }

    fn begin_custom_scan(
        _state: &mut CustomScanStateWrapper<Self>,
        _estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        // For EXPLAIN-only (without ANALYZE), we don't need to do anything
        if eflags & (pg_sys::EXEC_FLAG_EXPLAIN_ONLY as i32) != 0 {
            return;
        }
        unimplemented!("JoinScan execution not implemented - see next commit")
    }

    fn rescan_custom_scan(_state: &mut CustomScanStateWrapper<Self>) {
        unimplemented!("JoinScan execution not implemented - see next commit")
    }

    fn exec_custom_scan(_state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        unimplemented!("JoinScan execution not implemented - see next commit")
    }

    fn shutdown_custom_scan(_state: &mut CustomScanStateWrapper<Self>) {}

    fn end_custom_scan(_state: &mut CustomScanStateWrapper<Self>) {}
}

impl JoinScan {
    /// Extract join-level predicates from the restrict list.
    ///
    /// This handles:
    /// - Simple search predicates (single table): Converted to Predicate nodes
    /// - Cross-relation predicates without search: Converted to HeapCondition nodes
    /// - Boolean expressions (AND/OR/NOT): Recursively transformed
    ///
    /// Returns the updated JoinCSClause and a list of heap condition clause pointers
    /// (in the same order as heap_conditions in the clause) for adding to custom_exprs.
    unsafe fn extract_join_level_conditions(
        root: *mut pg_sys::PlannerInfo,
        extra: *mut pg_sys::JoinPathExtraData,
        outer_side: &JoinSideInfo,
        inner_side: &JoinSideInfo,
        other_conditions: &[*mut pg_sys::RestrictInfo],
        mut join_clause: JoinCSClause,
    ) -> Result<(JoinCSClause, Vec<*mut pg_sys::Expr>), String> {
        let mut heap_condition_clauses: Vec<*mut pg_sys::Expr> = Vec::new();

        if extra.is_null() {
            return Ok((join_clause, heap_condition_clauses));
        }

        let restrictlist = (*extra).restrictlist;
        if restrictlist.is_null() {
            return Ok((join_clause, heap_condition_clauses));
        }

        let outer_rti = outer_side.heap_rti.unwrap_or(0);
        let inner_rti = inner_side.heap_rti.unwrap_or(0);
        let search_op = anyelement_query_input_opoid();

        let restrict_infos = PgList::<pg_sys::RestrictInfo>::from_pg(restrictlist);

        // Collect all expressions into the expression tree
        let mut expr_trees: Vec<SerializableJoinLevelExpr> = Vec::new();

        // Track which RestrictInfos are heap conditions (by pointer) for index lookup
        let other_cond_set: std::collections::HashSet<usize> =
            other_conditions.iter().map(|&ri| ri as usize).collect();

        for ri in restrict_infos.iter_ptr() {
            if ri.is_null() || (*ri).clause.is_null() {
                continue;
            }

            let clause = (*ri).clause;
            let has_search_op = expr_contains_any_operator(clause.cast(), &[search_op]);

            if has_search_op {
                // Transform search predicate into expression tree
                // This also collects sub-expression heap conditions
                if let Some(expr) = Self::transform_to_search_expr(
                    root,
                    clause.cast(),
                    outer_rti,
                    inner_rti,
                    outer_side,
                    inner_side,
                    &mut join_clause,
                    &mut heap_condition_clauses,
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
                // Create a HeapCondition leaf node
                let description = format_expr_for_explain(clause.cast());
                let condition_idx =
                    join_clause.add_heap_condition(description, heap_condition_clauses.len());
                heap_condition_clauses.push(clause);
                expr_trees.push(SerializableJoinLevelExpr::HeapCondition { condition_idx });
            }
        }

        // Combine all expressions with AND (since they're separate RestrictInfos)
        if !expr_trees.is_empty() {
            let final_expr = if expr_trees.len() == 1 {
                expr_trees.pop().unwrap()
            } else {
                SerializableJoinLevelExpr::And(expr_trees)
            };
            join_clause = join_clause.with_join_level_expr(final_expr);
        }

        Ok((join_clause, heap_condition_clauses))
    }

    /// Recursively transform a PostgreSQL expression with search predicates into a SerializableJoinLevelExpr.
    ///
    /// - For single-table sub-trees with search predicates: extract as a Predicate leaf
    /// - For cross-relation sub-trees without search predicates: extract as a HeapCondition leaf
    /// - For BoolExpr (AND/OR/NOT): recursively transform children
    ///
    /// Also collects heap condition clause pointers into `heap_condition_clauses` for adding
    /// to custom_exprs during plan_custom_path.
    #[allow(clippy::too_many_arguments)]
    unsafe fn transform_to_search_expr(
        root: *mut pg_sys::PlannerInfo,
        node: *mut pg_sys::Node,
        outer_rti: pg_sys::Index,
        inner_rti: pg_sys::Index,
        outer_side: &JoinSideInfo,
        inner_side: &JoinSideInfo,
        join_clause: &mut JoinCSClause,
        heap_condition_clauses: &mut Vec<*mut pg_sys::Expr>,
    ) -> Option<SerializableJoinLevelExpr> {
        if node.is_null() {
            return None;
        }

        let search_op = anyelement_query_input_opoid();
        let has_search_op = expr_contains_any_operator(node, &[search_op]);

        // Check which tables this expression references
        let rtis = expr_collect_rtis(node);
        let refs_outer = rtis.contains(&outer_rti);
        let refs_inner = rtis.contains(&inner_rti);

        // If this is a single-table expression with search predicate, extract as Predicate
        if has_search_op && rtis.len() == 1 && (refs_outer || refs_inner) {
            let (rti, side, join_side) = if refs_outer {
                (outer_rti, outer_side, SerializableJoinSide::Outer)
            } else {
                (inner_rti, inner_side, SerializableJoinSide::Inner)
            };

            // Extract the Tantivy query for this expression
            if let Some(predicate_idx) =
                Self::extract_single_table_predicate(root, rti, side, node, join_clause)
            {
                return Some(SerializableJoinLevelExpr::Predicate {
                    side: join_side,
                    predicate_idx,
                });
            }
            return None;
        }

        // If this is a cross-relation expression WITHOUT search predicate, create HeapCondition
        if !has_search_op && refs_outer && refs_inner {
            // Create a HeapCondition and store the clause pointer for custom_exprs
            let description = format_expr_for_explain(node);
            let condition_idx =
                join_clause.add_heap_condition(description, heap_condition_clauses.len());
            heap_condition_clauses.push(node as *mut pg_sys::Expr);
            return Some(SerializableJoinLevelExpr::HeapCondition { condition_idx });
        }

        // Handle BoolExpr (AND/OR/NOT) by recursively processing children
        let node_type = (*node).type_;

        if node_type == pg_sys::NodeTag::T_BoolExpr {
            let boolexpr = node as *mut pg_sys::BoolExpr;
            let boolop = (*boolexpr).boolop;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);

            match boolop {
                pg_sys::BoolExprType::AND_EXPR => {
                    let mut children = Vec::new();
                    let mut all_transformed = true;
                    for arg in args.iter_ptr() {
                        if let Some(child_expr) = Self::transform_to_search_expr(
                            root,
                            arg,
                            outer_rti,
                            inner_rti,
                            outer_side,
                            inner_side,
                            join_clause,
                            heap_condition_clauses,
                        ) {
                            children.push(child_expr);
                        } else {
                            all_transformed = false;
                            break;
                        }
                    }
                    if !all_transformed || children.is_empty() {
                        None
                    } else if children.len() == 1 {
                        Some(children.pop().unwrap())
                    } else {
                        Some(SerializableJoinLevelExpr::And(children))
                    }
                }
                pg_sys::BoolExprType::OR_EXPR => {
                    // For OR, we need ALL children to be transformable
                    // Otherwise we can't correctly evaluate the OR
                    let mut children = Vec::new();
                    let mut all_transformed = true;
                    for arg in args.iter_ptr() {
                        if let Some(child_expr) = Self::transform_to_search_expr(
                            root,
                            arg,
                            outer_rti,
                            inner_rti,
                            outer_side,
                            inner_side,
                            join_clause,
                            heap_condition_clauses,
                        ) {
                            children.push(child_expr);
                        } else {
                            all_transformed = false;
                            break;
                        }
                    }
                    if !all_transformed || children.is_empty() {
                        None
                    } else if children.len() == 1 {
                        Some(children.pop().unwrap())
                    } else {
                        Some(SerializableJoinLevelExpr::Or(children))
                    }
                }
                pg_sys::BoolExprType::NOT_EXPR => {
                    // NOT has exactly one argument
                    if let Some(arg) = args.iter_ptr().next() {
                        if let Some(child_expr) = Self::transform_to_search_expr(
                            root,
                            arg,
                            outer_rti,
                            inner_rti,
                            outer_side,
                            inner_side,
                            join_clause,
                            heap_condition_clauses,
                        ) {
                            return Some(SerializableJoinLevelExpr::Not(Box::new(child_expr)));
                        }
                    }
                    None
                }
                _ => None,
            }
        } else {
            // Not a BoolExpr and not handled above - can't transform
            None
        }
    }

    /// Extract a single-table predicate and add it to the join clause.
    /// Returns the index of the predicate in join_level_predicates, or None if extraction fails.
    unsafe fn extract_single_table_predicate(
        root: *mut pg_sys::PlannerInfo,
        rti: pg_sys::Index,
        side: &JoinSideInfo,
        expr: *mut pg_sys::Node,
        join_clause: &mut JoinCSClause,
    ) -> Option<usize> {
        let indexrelid = side.indexrelid?;
        let heaprelid = side.heaprelid?;

        let (_, bm25_idx) = rel_get_bm25_index(heaprelid)?;

        // Create a RestrictInfo wrapping the expression for extract_quals
        let mut ri_list = PgList::<pg_sys::RestrictInfo>::new();
        let fake_ri = pg_sys::palloc0(std::mem::size_of::<pg_sys::RestrictInfo>())
            as *mut pg_sys::RestrictInfo;
        (*fake_ri).type_ = pg_sys::NodeTag::T_RestrictInfo;
        (*fake_ri).clause = expr.cast();
        ri_list.push(fake_ri);

        let context = PlannerContext::from_planner(root);
        let mut state = QualExtractState::default();

        // extract_quals handles BoolExpr (AND/OR/NOT) recursively, preserving the structure
        let qual = extract_quals(
            &context,
            rti,
            ri_list.as_ptr().cast(),
            anyelement_query_input_opoid(),
            RestrictInfoType::BaseRelation,
            &bm25_idx,
            false,
            &mut state,
            false, // Don't attempt pushdown for join-level predicates
        )?;

        let query = SearchQueryInput::from(&qual);
        let idx = join_clause.add_join_level_predicate(indexrelid, heaprelid, query);
        Some(idx)
    }
}

impl ExecMethod for JoinScan {
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        <JoinScan as PlainExecCapable>::exec_methods()
    }
}

impl PlainExecCapable for JoinScan {}
/// Extract join conditions from the restrict list.
/// Separates equi-join keys (for hash lookup) from other conditions (for filtering).
unsafe fn extract_join_conditions(
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
unsafe fn is_op_hash_joinable(opno: pg_sys::Oid) -> bool {
    // op_hashjoinable checks if the operator can be used for hash joins,
    // which requires it to be an equality operator with a hash function.
    // We pass InvalidOid as inputtype to accept any input type.
    pg_sys::op_hashjoinable(opno, pg_sys::InvalidOid)
}

/// Get type length and pass-by-value info for a given type OID.
unsafe fn get_type_info(type_oid: pg_sys::Oid) -> (i16, bool) {
    let mut typlen: i16 = 0;
    let mut typbyval: bool = false;
    pg_sys::get_typlenbyval(type_oid, &mut typlen, &mut typbyval);
    (typlen, typbyval)
}

/// Try to extract join side information from a RelOptInfo.
/// Returns JoinSideInfo if we find a base relation (possibly with a BM25 index).
unsafe fn extract_join_side_info(
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

/// Estimate the cost of a JoinScan operation.
///
/// Returns (startup_cost, total_cost, result_rows, estimated_build_rows).
///
/// Cost model components:
/// - Startup cost: Build side sequential scan + hash table construction
/// - Per-tuple cost: Hash lookup + heap fetch for driving side
/// - Result rows: Min(limit, estimated_matches)
/// - estimated_build_rows: Estimated rows in build side (for execution hints)
unsafe fn estimate_joinscan_cost(
    join_clause: &JoinCSClause,
    outerrel: *mut pg_sys::RelOptInfo,
    innerrel: *mut pg_sys::RelOptInfo,
    limit: Option<usize>,
) -> (f64, f64, f64, f64) {
    // Get row estimates from PostgreSQL's statistics
    let outer_rows = if !outerrel.is_null() {
        (*outerrel).rows.max(1.0)
    } else {
        1000.0
    };
    let inner_rows = if !innerrel.is_null() {
        (*innerrel).rows.max(1.0)
    } else {
        1000.0
    };

    // Determine driving and build side rows
    let driving_is_outer = join_clause.driving_side_is_outer();
    let (driving_rows, build_rows) = if driving_is_outer {
        (outer_rows, inner_rows)
    } else {
        (inner_rows, outer_rows)
    };

    // If driving side has a search predicate, assume selectivity reduces rows
    // This is a rough estimate - ideally we'd get stats from Tantivy
    let driving_selectivity = if join_clause.driving_side().has_search_predicate {
        0.1 // Assume search predicate selects ~10% of rows
    } else {
        1.0
    };
    let estimated_driving_rows = (driving_rows * driving_selectivity).max(1.0);

    // If build side has a search predicate, reduce build side estimate too
    let build_selectivity = if join_clause.build_side().has_search_predicate {
        0.1
    } else {
        1.0
    };
    let estimated_build_rows = (build_rows * build_selectivity).max(1.0);

    // Apply limit to result estimate
    let result_rows = match limit {
        Some(lim) => (estimated_driving_rows * 0.5).min(lim as f64).max(1.0), // Assume ~50% join selectivity
        None => estimated_driving_rows * 0.5,
    };

    // Cost components using PostgreSQL's cost constants
    let seq_page_cost = pg_sys::seq_page_cost;
    let cpu_tuple_cost = pg_sys::cpu_tuple_cost;
    let cpu_operator_cost = pg_sys::cpu_operator_cost;

    // Startup cost: Build the hash table from build side
    // - Sequential scan of build side
    // - Hashing each tuple
    let build_scan_cost = estimated_build_rows * cpu_tuple_cost;
    let build_hash_cost = estimated_build_rows * cpu_operator_cost;
    let startup_cost = DEFAULT_STARTUP_COST + build_scan_cost + build_hash_cost;

    // Per-tuple cost for driving side:
    // - Fetch from index (via Tantivy)
    // - Heap fetch for driving tuple
    // - Hash lookup
    // - Heap fetch for matching build tuples (if any)
    let index_fetch_cost = cpu_operator_cost; // Tantivy lookup
    let heap_fetch_cost = cpu_tuple_cost; // Heap access for driving tuple
    let hash_lookup_cost = cpu_operator_cost; // Hash table probe
    let per_driving_tuple_cost = index_fetch_cost + heap_fetch_cost + hash_lookup_cost;

    // Total run cost: process estimated driving rows
    // But we may short-circuit due to LIMIT
    let driving_rows_to_process = match limit {
        Some(lim) => {
            // We might need to process more driving rows than the limit
            // due to join selectivity (not all driving rows will have matches)
            (lim as f64 * 2.0).min(estimated_driving_rows)
        }
        None => estimated_driving_rows,
    };
    let run_cost = driving_rows_to_process * per_driving_tuple_cost;

    let total_cost = startup_cost + run_cost;

    (startup_cost, total_cost, result_rows, estimated_build_rows)
}

/// Compute execution hints based on planning information.
///
/// These hints help the executor make better decisions about algorithm selection,
/// memory allocation, and batch sizing.
fn compute_execution_hints(estimated_build_rows: f64, limit: Option<usize>) -> ExecutionHints {
    // Determine algorithm hint based on build side size
    let algorithm = determine_algorithm_hint(estimated_build_rows, limit);

    // Estimate hash table memory: each entry is roughly 64-128 bytes
    // (CompositeKey + Vec overhead + InnerRow + HashMap entry overhead)
    // Use a conservative estimate of 128 bytes per entry
    const ESTIMATED_ENTRY_SIZE: usize = 128;
    let estimated_hash_memory = (estimated_build_rows as usize) * ESTIMATED_ENTRY_SIZE;

    ExecutionHints::new()
        .with_algorithm(algorithm)
        .with_estimated_build_rows(estimated_build_rows)
        .with_estimated_hash_memory(estimated_hash_memory)
}

/// Determine preferred join algorithm based on build side size and limit.
fn determine_algorithm_hint(build_rows: f64, limit: Option<usize>) -> JoinAlgorithmHint {
    // No limit with large build side: hash is better for full scans
    if limit.is_none() && build_rows > 100.0 {
        return JoinAlgorithmHint::PreferHash;
    }

    // Default: let executor decide based on runtime conditions
    JoinAlgorithmHint::Auto
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
unsafe fn extract_score_pathkey(
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

/// Format a PostgreSQL expression node for EXPLAIN output.
/// Returns a human-readable description of the expression.
unsafe fn format_expr_for_explain(node: *mut pg_sys::Node) -> String {
    if node.is_null() {
        return "?".to_string();
    }

    // Use PostgreSQL's nodeToString for a basic representation
    // This gives us something like "(p.price > s.min_value)"
    let node_str = pg_sys::nodeToString(node.cast());
    if !node_str.is_null() {
        let c_str = std::ffi::CStr::from_ptr(node_str);
        let result = c_str.to_string_lossy().to_string();
        pg_sys::pfree(node_str.cast());
        // Truncate if too long for readability
        if result.len() > 50 {
            format!("{}...", &result[..47])
        } else {
            result
        }
    } else {
        "expr".to_string()
    }
}

/// Convert a serializable join-level expression to a runtime expression.
/// Note: Used in begin_custom_scan which is stubbed in this commit.
#[allow(dead_code)]
fn convert_to_runtime_expr(expr: &SerializableJoinLevelExpr) -> JoinLevelExpr {
    match expr {
        SerializableJoinLevelExpr::Predicate {
            side,
            predicate_idx,
        } => {
            let runtime_side = match side {
                SerializableJoinSide::Outer => JoinSide::Outer,
                SerializableJoinSide::Inner => JoinSide::Inner,
            };
            JoinLevelExpr::Predicate {
                side: runtime_side,
                ctid_set_idx: *predicate_idx,
            }
        }
        SerializableJoinLevelExpr::HeapCondition { condition_idx } => {
            JoinLevelExpr::HeapCondition {
                condition_idx: *condition_idx,
            }
        }
        SerializableJoinLevelExpr::And(children) => {
            JoinLevelExpr::And(children.iter().map(convert_to_runtime_expr).collect())
        }
        SerializableJoinLevelExpr::Or(children) => {
            JoinLevelExpr::Or(children.iter().map(convert_to_runtime_expr).collect())
        }
        SerializableJoinLevelExpr::Not(child) => {
            JoinLevelExpr::Not(Box::new(convert_to_runtime_expr(child)))
        }
    }
}

/// Get the column name for an attribute, with fallback to "relname.attno" if lookup fails.
fn get_attname_safe(
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
fn format_join_level_expr(
    expr: &SerializableJoinLevelExpr,
    predicates: &[build::JoinLevelSearchPredicate],
    heap_conditions: &[HeapConditionInfo],
) -> String {
    use crate::postgres::customscan::explain::ExplainFormat;

    match expr {
        SerializableJoinLevelExpr::Predicate {
            side,
            predicate_idx,
        } => {
            let side_str = match side {
                SerializableJoinSide::Outer => "outer",
                SerializableJoinSide::Inner => "inner",
            };
            if let Some(pred) = predicates.get(*predicate_idx) {
                format!("{}:{}", side_str, pred.query.explain_format())
            } else {
                format!("{}:?", side_str)
            }
        }
        SerializableJoinLevelExpr::HeapCondition { condition_idx } => {
            if let Some(cond) = heap_conditions.get(*condition_idx) {
                format!("heap:{}", cond.description)
            } else {
                "heap:?".to_string()
            }
        }
        SerializableJoinLevelExpr::And(children) => {
            let parts: Vec<_> = children
                .iter()
                .map(|c| format_join_level_expr(c, predicates, heap_conditions))
                .collect();
            if parts.len() == 1 {
                parts.into_iter().next().unwrap()
            } else {
                format!("({})", parts.join(" AND "))
            }
        }
        SerializableJoinLevelExpr::Or(children) => {
            let parts: Vec<_> = children
                .iter()
                .map(|c| format_join_level_expr(c, predicates, heap_conditions))
                .collect();
            if parts.len() == 1 {
                parts.into_iter().next().unwrap()
            } else {
                format!("({})", parts.join(" OR "))
            }
        }
        SerializableJoinLevelExpr::Not(child) => {
            format!(
                "NOT {}",
                format_join_level_expr(child, predicates, heap_conditions)
            )
        }
    }
}
