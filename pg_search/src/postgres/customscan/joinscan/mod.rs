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
mod explain;
mod planning;
mod predicate;
pub mod privdat;
pub mod scan_state;

use self::build::{
    JoinAlgorithmHint, JoinCSClause, SerializableJoinLevelExpr, SerializableJoinSide,
    SerializableJoinType,
};
use self::explain::{format_join_level_expr, get_attname_safe};
use self::planning::{
    compute_execution_hints, estimate_joinscan_cost, extract_join_conditions,
    extract_join_side_info, extract_score_pathkey,
};
use self::predicate::extract_join_level_conditions;
use self::privdat::PrivateData;
use self::scan_state::{
    CompositeKey, InnerRow, JoinKeyInfo, JoinLevelEvalContext, JoinLevelExpr, JoinScanState,
    JoinSide, KeyValue,
};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::basescan::projections::score::uses_scores;
use crate::postgres::customscan::builders::custom_path::{CustomPathBuilder, Flags};
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::score_funcoids;
use crate::postgres::customscan::{CustomScan, ExecMethod, JoinPathlistHookArgs, PlainExecCapable};
use crate::postgres::heap::{OwnedVisibilityChecker, VisibilityChecker};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::item_pointer_to_u64;
use pgrx::{pg_sys, PgList};
use std::ffi::CStr;

#[derive(Default)]
pub struct JoinScan;

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

            // Check if paradedb.score() is used anywhere in the query for each side.
            // This includes ORDER BY, SELECT list, or any other expression.
            // We need to check BOTH sides, not just the driving side, because:
            // - Driving side: scores come from the streaming executor
            // - Build side: scores come from the pre-materialized search results
            let funcoids = score_funcoids();
            let score_pathkey = extract_score_pathkey(root, driving_side_rti as pg_sys::Index);

            // Check if outer side needs scores
            let outer_score_in_tlist =
                uses_scores((*root).processed_tlist.cast(), funcoids, outer_rti);
            let outer_score_needed = if driving_side_is_outer {
                score_pathkey.is_some() || outer_score_in_tlist
            } else {
                outer_score_in_tlist
            };

            // Check if inner side needs scores
            let inner_score_in_tlist =
                uses_scores((*root).processed_tlist.cast(), funcoids, inner_rti);
            let inner_score_needed = if !driving_side_is_outer {
                score_pathkey.is_some() || inner_score_in_tlist
            } else {
                inner_score_in_tlist
            };

            // Record score_needed for each side
            outer_side = outer_side.with_score_needed(outer_score_needed);
            inner_side = inner_side.with_score_needed(inner_score_needed);

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
            let Ok((mut join_clause, heap_condition_clauses)) = extract_join_level_conditions(
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
            let has_side_predicate = (outer_side.has_bm25_index()
                && outer_side.has_search_predicate)
                || (inner_side.has_bm25_index() && inner_side.has_search_predicate);
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
                } else if uses_scores((*te).expr.cast(), funcoids, outer_rti) {
                    // This expression contains paradedb.score() for the outer side
                    output_columns.push(privdat::OutputColumnInfo {
                        is_outer: true,
                        original_attno: 0,
                        is_score: true,
                    });
                } else if uses_scores((*te).expr.cast(), funcoids, inner_rti) {
                    // This expression contains paradedb.score() for the inner side
                    output_columns.push(privdat::OutputColumnInfo {
                        is_outer: false,
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
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        unsafe {
            // For EXPLAIN-only (without ANALYZE), we don't need to do much
            if eflags & (pg_sys::EXEC_FLAG_EXPLAIN_ONLY as i32) != 0 {
                return;
            }

            // Clone the join clause to avoid borrow issues
            let join_clause = state.custom_state().join_clause.clone();
            let snapshot = (*estate).es_snapshot;

            // Determine which side is driving (has search predicate) vs build
            let driving_is_outer = join_clause.driving_side_is_outer();
            state.custom_state_mut().driving_is_outer = driving_is_outer;

            let (driving_side, build_side) = if driving_is_outer {
                (&join_clause.outer_side, &join_clause.inner_side)
            } else {
                (&join_clause.inner_side, &join_clause.outer_side)
            };

            // Populate join key info for execution
            for jk in &join_clause.join_keys {
                let (build_attno, driving_attno) = if driving_is_outer {
                    (jk.inner_attno as i32, jk.outer_attno as i32)
                } else {
                    (jk.outer_attno as i32, jk.inner_attno as i32)
                };

                state.custom_state_mut().build_key_info.push(JoinKeyInfo {
                    attno: build_attno,
                    typlen: jk.typlen,
                    typbyval: jk.typbyval,
                });
                state.custom_state_mut().driving_key_info.push(JoinKeyInfo {
                    attno: driving_attno,
                    typlen: jk.typlen,
                    typbyval: jk.typbyval,
                });
            }

            // Initialize memory limit from work_mem (in KB, convert to bytes)
            let work_mem_bytes = (pg_sys::work_mem as usize) * 1024;
            state.custom_state_mut().max_hash_memory = work_mem_bytes;

            // Note: Execution hints (algorithm preference, memory estimates) are stored in
            // join_clause.hints and used during hash table building to make informed decisions.
            // We don't pre-set using_nested_loop here because build side state isn't initialized yet.
            // The hints are used in build_hash_table() to:
            // 1. Pre-size the hash table based on estimated_build_rows
            // 2. Inform early termination decisions based on estimated_hash_memory

            // Use PostgreSQL's already-initialized result tuple slot (ps_ResultTupleSlot).
            // PostgreSQL sets this up in ExecInitCustomScan based on custom_scan_tlist
            // and the projection info. Don't create our own slot - use the one PostgreSQL
            // provides to ensure compatibility with the query executor's expectations.
            //
            // Note: After set_customscan_references, custom_scan_tlist contains Vars with
            // OUTER_VAR/INNER_VAR varnos. Using ExecTypeFromTL on that would create a
            // corrupt tuple descriptor.
            let result_slot = state.csstate.ss.ps.ps_ResultTupleSlot;
            state.custom_state_mut().result_slot = Some(result_slot);

            // Open relations for the driving side
            // If driving side has a search predicate, use search scan; otherwise use heap scan
            if let Some(heaprelid) = driving_side.heaprelid {
                let heaprel = PgSearchRelation::open(heaprelid);

                // Create visibility checker for driving side
                let vis_checker = VisibilityChecker::with_rel_and_snap(&heaprel, snapshot);
                state.custom_state_mut().driving_visibility_checker = Some(vis_checker);

                // Create a slot for fetching driving tuples
                let driving_slot =
                    pg_sys::MakeTupleTableSlot(heaprel.rd_att, &pg_sys::TTSOpsBufferHeapTuple);
                state.custom_state_mut().driving_fetch_slot = Some(driving_slot);

                // If driving side has a search predicate, create an executor
                if let (Some(indexrelid), Some(ref query)) =
                    (driving_side.indexrelid, &driving_side.query)
                {
                    // Use FastField executor which iterates all matching results
                    // using batched ctid lookups for efficiency.
                    let executor = executors::JoinSideExecutor::new_fast_field(
                        indexrelid,
                        query.clone(),
                        driving_side.score_needed,
                    );
                    state.custom_state_mut().driving_executor = Some(executor);

                    // Create visibility checker for fetching tuples by ctid from executor results
                    let vis_checker = VisibilityChecker::with_rel_and_snap(&heaprel, snapshot);
                    state.custom_state_mut().driving_visibility_checker = Some(vis_checker);
                } else {
                    // No search predicate - use heap scan for driving side
                    let scan_desc = pg_sys::table_beginscan(
                        heaprel.as_ptr(),
                        snapshot,
                        0,
                        std::ptr::null_mut(),
                    );
                    state.custom_state_mut().driving_scan_desc = Some(scan_desc);
                    state.custom_state_mut().driving_uses_heap_scan = true;
                }

                state.custom_state_mut().driving_heaprel = Some(heaprel);
            }

            // Open relations for the build side (side we build hash table from)
            if let Some(heaprelid) = build_side.heaprelid {
                let heaprel = PgSearchRelation::open(heaprelid);

                // Create visibility checker for build side
                let vis_checker = VisibilityChecker::with_rel_and_snap(&heaprel, snapshot);
                state.custom_state_mut().build_visibility_checker = Some(vis_checker);

                // Create a slot for scanning build tuples
                let build_slot =
                    pg_sys::MakeTupleTableSlot(heaprel.rd_att, &pg_sys::TTSOpsBufferHeapTuple);
                state.custom_state_mut().build_scan_slot = Some(build_slot);

                // Start a sequential scan on the build relation for building hash table
                let scan_desc =
                    pg_sys::table_beginscan(heaprel.as_ptr(), snapshot, 0, std::ptr::null_mut());
                state.custom_state_mut().build_scan_desc = Some(scan_desc);

                // TODO(build-side-streaming): Currently, if the build side has a search
                // predicate, we pre-materialize ALL matching ctids into a HashMap upfront.
                // This defeats the incremental fetching benefit for large result sets.
                //
                // A better approach would be to use JoinSideExecutor for build side too:
                // - During hash table build, filter rows lazily using the executor
                // - This would allow early termination if hash table exceeds work_mem
                // - Could also enable build side ordering for merge-join style execution
                if let (Some(indexrelid), Some(ref query)) =
                    (build_side.indexrelid, &build_side.query)
                {
                    let indexrel = PgSearchRelation::open(indexrelid);
                    let search_reader = SearchIndexReader::open_with_context(
                        &indexrel,
                        query.clone(),
                        build_side.score_needed, // need scores if paradedb.score() references build side
                        MvccSatisfies::Snapshot,
                        None,
                        None,
                    );

                    if let Ok(reader) = search_reader {
                        // Store ctid -> score mapping (score is 0.0 if not needed)
                        let mut build_ctids = std::collections::HashMap::new();

                        // Create a visibility checker to resolve stale ctids
                        let mut vis_checker = OwnedVisibilityChecker::new(&heaprel, snapshot);

                        let results = reader.search();
                        for (scored, _doc_address) in results {
                            // Verify tuple visibility and get its current ctid
                            if let Some(current_ctid) = vis_checker.get_current_ctid(scored.ctid) {
                                build_ctids.insert(current_ctid, scored.bm25);
                            }
                        }

                        state.custom_state_mut().build_matching_ctids = Some(build_ctids);
                    }
                }

                state.custom_state_mut().build_heaprel = Some(heaprel);
            }

            // Initialize heap condition evaluation if we have any
            // The heap condition expressions were added to custom_exprs during plan_custom_path
            // and have been transformed by set_customscan_references to use INDEX_VAR references.
            if join_clause.has_heap_conditions() {
                let cscan = state.csstate.ss.ps.plan as *mut pg_sys::CustomScan;
                let custom_exprs = PgList::<pg_sys::Node>::from_pg((*cscan).custom_exprs);

                // Create expression context for all heap conditions
                let econtext = pg_sys::CreateExprContext(estate);
                state.custom_state_mut().heap_condition_econtext = Some(econtext);

                // Initialize expression state for each heap condition
                // The expressions in custom_exprs are in the same order as heap_conditions
                for (i, _heap_cond) in join_clause.heap_conditions.iter().enumerate() {
                    if i < custom_exprs.len() {
                        if let Some(expr_node) = custom_exprs.get_ptr(i) {
                            if !expr_node.is_null() {
                                // Create a single-element list for ExecInitQual
                                let mut qual_list = PgList::<pg_sys::Expr>::new();
                                qual_list.push(expr_node as *mut pg_sys::Expr);
                                let qual_state = pg_sys::ExecInitQual(
                                    qual_list.into_pg(),
                                    &mut state.csstate.ss.ps,
                                );
                                state
                                    .custom_state_mut()
                                    .heap_condition_states
                                    .push(qual_state);
                                continue;
                            }
                        }
                    }
                    // If we couldn't initialize this condition, push null
                    state
                        .custom_state_mut()
                        .heap_condition_states
                        .push(std::ptr::null_mut());
                }
            }

            // Initialize join-level predicate evaluation if we have a join-level expression
            //
            // TODO(memory-limit): All matching ctids are materialized into HashSets upfront
            // with no memory limit. A broad predicate like `content @@@ 'the'` could match
            // millions of rows, causing OOM. Consider:
            // - Applying work_mem limit and falling back to row-by-row evaluation
            // - Using lazy/streaming evaluation instead of full materialization
            // - Adding an upper bound on ctid_set size
            if let Some(ref serializable_expr) = join_clause.join_level_expr {
                let join_level_predicates = &join_clause.join_level_predicates;

                // Execute Tantivy queries for each predicate and collect matching ctids
                // The results are stored in join_level_ctid_sets, indexed by predicate position
                for predicate in join_level_predicates {
                    let indexrel = PgSearchRelation::open(predicate.indexrelid);

                    let search_reader = SearchIndexReader::open_with_context(
                        &indexrel,
                        predicate.query.clone(),
                        false, // don't need scores
                        MvccSatisfies::Snapshot,
                        None,
                        None,
                    );

                    let mut ctid_set = std::collections::HashSet::new();

                    if let Ok(reader) = search_reader {
                        // Create a visibility checker for this relation (owns its slot)
                        let heaprel = PgSearchRelation::open(predicate.heaprelid);
                        let mut vis_checker = OwnedVisibilityChecker::new(&heaprel, snapshot);

                        let results = reader.search();
                        for (scored, _doc_address) in results {
                            // Verify tuple visibility and get its current ctid
                            if let Some(ctid) = vis_checker.get_current_ctid(scored.ctid) {
                                ctid_set.insert(ctid);
                            }
                        }
                        // vis_checker is dropped here, cleaning up the slot
                    }

                    state.custom_state_mut().join_level_ctid_sets.push(ctid_set);
                }

                // Convert the serializable expression to the runtime expression
                let runtime_expr = convert_to_runtime_expr(serializable_expr);
                state.custom_state_mut().join_level_expr = Some(runtime_expr);
            }
        }
    }

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        // Reset state for rescanning
        state.custom_state_mut().reset();

        // Also reset the driving executor if present
        if let Some(ref mut executor) = state.custom_state_mut().driving_executor {
            executor.reset();
        }
    }

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        unsafe {
            // Note: We don't enforce LIMIT here because JoinScan might be nested inside
            // another join (e.g., 3-table join). PostgreSQL's Limit node handles limiting.
            // The limit value in join_clause is used for cost estimation and executor hints.

            // Phase 1: Build hash table from build side (first call only)
            if !state.custom_state().hash_table_built {
                Self::build_hash_table(state);
                state.custom_state_mut().hash_table_built = true;
            }

            // If we exceeded memory limit, use nested loop instead of hash join
            if state.custom_state().using_nested_loop {
                return Self::exec_nested_loop(state);
            }

            // Phase 2: Probe hash table with driving side search results
            loop {
                // If we have pending matches, return one
                if let Some((build_ctid, build_score)) =
                    state.custom_state_mut().pending_build_ctids.pop_front()
                {
                    if let Some(slot) = Self::build_result_tuple(state, build_ctid, build_score) {
                        // Check join-level predicate using the expression tree
                        if let Some(ref expr) = state.custom_state().join_level_expr {
                            // Map driving/build to outer/inner based on driving_is_outer
                            let (outer_slot, inner_slot) = state.custom_state().outer_inner_slots();

                            // Convert heap ctids to u64 using the SAME encoding as the BM25 index
                            let outer_ctid_u64 = outer_slot
                                .map(|s| crate::postgres::utils::item_pointer_to_u64((*s).tts_tid))
                                .unwrap_or(0);

                            let inner_ctid_u64 = inner_slot
                                .map(|s| crate::postgres::utils::item_pointer_to_u64((*s).tts_tid))
                                .unwrap_or(0);

                            // Set up expression context for heap condition evaluation (if any)
                            if let Some(econtext) = state.custom_state().heap_condition_econtext {
                                (*econtext).ecxt_scantuple = slot;

                                // Also set outer/inner tuple slots for the expression context
                                if let (Some(outer), Some(inner)) = (outer_slot, inner_slot) {
                                    (*econtext).ecxt_outertuple = outer;
                                    (*econtext).ecxt_innertuple = inner;
                                }
                            }

                            // Create evaluation context for the unified expression tree
                            let econtext = state
                                .custom_state()
                                .heap_condition_econtext
                                .unwrap_or(std::ptr::null_mut());
                            let eval_ctx = JoinLevelEvalContext {
                                ctid_sets: &state.custom_state().join_level_ctid_sets,
                                heap_condition_states: &state.custom_state().heap_condition_states,
                                econtext,
                            };

                            // Evaluate the full boolean expression tree (includes both
                            // Predicate nodes for Tantivy searches and HeapCondition nodes
                            // for PostgreSQL expressions)
                            if !expr.evaluate(outer_ctid_u64, inner_ctid_u64, &eval_ctx) {
                                // Expression not satisfied, try next match
                                continue;
                            }
                        }

                        state.custom_state_mut().rows_returned += 1;
                        return slot;
                    }
                    // Visibility check failed, try next match
                    continue;
                }

                // Get next driving row - either from executor or heap scan
                let (driving_ctid, driving_score) =
                    if state.custom_state().driving_executor.is_some() {
                        // Use the executor for incremental fetching
                        let executor = state.custom_state_mut().driving_executor.as_mut().unwrap();

                        // Note: We don't check executor.reached_limit() here because
                        // for joins the LIMIT applies to output rows (tracked by rows_returned),
                        // not input driving rows. The executor's limit is set very high.
                        match executor.next_visible() {
                            executors::JoinExecResult::Visible { ctid, score } => (ctid, score),
                            executors::JoinExecResult::Eof => return std::ptr::null_mut(),
                        }
                    } else if let Some(scan_desc) = state.custom_state().driving_scan_desc {
                        // Fallback to heap scan (no search predicate case)
                        let slot = state.custom_state().driving_fetch_slot;
                        let Some(slot) = slot else {
                            return std::ptr::null_mut();
                        };

                        let has_tuple = pg_sys::table_scan_getnextslot(
                            scan_desc,
                            pg_sys::ScanDirection::ForwardScanDirection,
                            slot,
                        );

                        if !has_tuple {
                            return std::ptr::null_mut(); // No more heap tuples
                        }

                        // Get ctid from the slot
                        let ctid = item_pointer_to_u64((*slot).tts_tid);
                        (ctid, 0.0) // No score for heap scan
                    } else {
                        return std::ptr::null_mut(); // No source for driving rows
                    };

                state.custom_state_mut().current_driving_ctid = Some(driving_ctid);
                state.custom_state_mut().current_driving_score = driving_score;

                // Extract join key from driving tuple
                let join_key = Self::extract_driving_join_key(state, driving_ctid);
                let Some(key) = join_key else {
                    // Couldn't extract join key (tuple not visible), try next driving row
                    continue;
                };

                // Look up matching build rows in hash table
                // Clone the (ctid, score) pairs to avoid borrow issues
                let build_rows: Vec<(u64, f32)> = state
                    .custom_state()
                    .hash_table
                    .get(&key)
                    .map(|rows| rows.iter().map(|r| (r.ctid, r.score)).collect())
                    .unwrap_or_default();

                for (ctid, score) in build_rows {
                    state
                        .custom_state_mut()
                        .pending_build_ctids
                        .push_back((ctid, score));
                }

                // Loop back to process pending matches
            }
        }
    }

    fn shutdown_custom_scan(_state: &mut CustomScanStateWrapper<Self>) {}

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        unsafe {
            // End build scan if active
            if let Some(scan_desc) = state.custom_state().build_scan_desc {
                pg_sys::table_endscan(scan_desc);
            }

            // End driving heap scan if active
            if let Some(scan_desc) = state.custom_state().driving_scan_desc {
                pg_sys::table_endscan(scan_desc);
            }

            // Drop tuple slots that we own.
            // Note: Don't drop result_slot - we borrowed it from PostgreSQL's ps_ResultTupleSlot.
            if let Some(slot) = state.custom_state().build_scan_slot {
                pg_sys::ExecDropSingleTupleTableSlot(slot);
            }
            if let Some(slot) = state.custom_state().driving_fetch_slot {
                pg_sys::ExecDropSingleTupleTableSlot(slot);
            }
        }

        // Clean up resources
        state.custom_state_mut().driving_heaprel = None;
        state.custom_state_mut().driving_executor = None;
        state.custom_state_mut().driving_scan_desc = None;
        state.custom_state_mut().build_heaprel = None;
        state.custom_state_mut().build_scan_desc = None;
        state.custom_state_mut().build_scan_slot = None;
        state.custom_state_mut().driving_fetch_slot = None;
        state.custom_state_mut().result_slot = None;
    }
}

impl JoinScan {
    /// Build the hash table from the build side by scanning the heap.
    unsafe fn build_hash_table(state: &mut CustomScanStateWrapper<Self>) {
        let build_key_info = state.custom_state().build_key_info.clone();
        let has_equi_join_keys = !build_key_info.is_empty();
        let max_hash_memory = state.custom_state().max_hash_memory;

        let Some(scan_desc) = state.custom_state().build_scan_desc else {
            return;
        };
        let Some(slot) = state.custom_state().build_scan_slot else {
            return;
        };

        // Pre-size hash table based on execution hints from planner
        // This avoids reallocations as the hash table grows
        if let Some(est_rows) = state.custom_state().join_clause.hints.estimated_build_rows {
            let capacity = (est_rows as usize).min(100_000); // Cap at 100K to avoid huge allocations
            state.custom_state_mut().hash_table.reserve(capacity);
        }

        // Get build_matching_ctids reference (if build side has a search predicate)
        let build_matching_ctids = state.custom_state().build_matching_ctids.clone();

        // Scan all build side tuples
        while pg_sys::table_scan_getnextslot(
            scan_desc,
            pg_sys::ScanDirection::ForwardScanDirection,
            slot,
        ) {
            // Extract the ctid from the slot
            let ctid = item_pointer_to_u64((*slot).tts_tid);

            // If build side has a search predicate, filter to only matching rows
            // and get the BM25 score (if build side needs scores)
            let build_score = if let Some(ref matching) = build_matching_ctids {
                match matching.get(&ctid) {
                    Some(&score) => score,
                    None => continue, // Skip rows that don't match the build side predicate
                }
            } else {
                0.0 // No search predicate on build side
            };

            // Extract the composite key
            let key = match extract_composite_key(slot, &build_key_info) {
                Some(k) => k,
                None => continue, // Skip rows with NULL keys
            };

            // Estimate memory for this entry
            let entry_size = estimate_entry_size(&key);
            let new_memory = state.custom_state().hash_table_memory + entry_size;

            // Check memory limit
            if new_memory > max_hash_memory && max_hash_memory > 0 {
                // Exceeded memory limit - switch to nested loop
                state.custom_state_mut().hash_table.clear();
                state.custom_state_mut().hash_table_memory = 0;
                state.custom_state_mut().using_nested_loop = true;

                // Reset scan to beginning for nested loop
                pg_sys::table_rescan(scan_desc, std::ptr::null_mut());
                break;
            }

            state.custom_state_mut().hash_table_memory = new_memory;

            // Add to hash table with ctid and score
            state
                .custom_state_mut()
                .hash_table
                .entry(key)
                .or_default()
                .push(InnerRow {
                    ctid,
                    score: build_score,
                });
        }

        // Store whether we're doing a cross join for later use
        // TODO(cross-join-memory): For cross joins, all build rows map to CompositeKey::CrossJoin,
        // creating a single hash bucket with ALL build rows. During probe, this generates O(N*M)
        // row pairs which can cause memory exhaustion even though hash table build succeeded.
        // Consider: immediate nested loop fallback for cross joins, stricter memory limits,
        // or emitting a warning in EXPLAIN about cross-join performance.
        state.custom_state_mut().is_cross_join = !has_equi_join_keys;
    }

    /// Execute nested loop join when hash join exceeds memory limit.
    /// This is a fallback that uses less memory but has O(N*M) complexity.
    ///
    /// TODO(nested-loop-perf): This fallback rescans the entire build side for EACH driving
    /// row, resulting in O(D*B) I/O. For large tables this is severely worse than PostgreSQL's
    /// native join which would spill to disk. Consider alternatives:
    /// - Block nested loop (batch driving rows)
    /// - Spill hash table to disk like PostgreSQL
    /// - Fall back entirely to PostgreSQL's native join execution
    /// - At minimum, emit a WARNING when this fallback triggers
    unsafe fn exec_nested_loop(
        state: &mut CustomScanStateWrapper<Self>,
    ) -> *mut pg_sys::TupleTableSlot {
        let Some(scan_desc) = state.custom_state().build_scan_desc else {
            return std::ptr::null_mut();
        };
        let Some(build_slot) = state.custom_state().build_scan_slot else {
            return std::ptr::null_mut();
        };

        loop {
            // Note: We don't check reached_limit() here because JoinScan might be nested
            // inside another join. PostgreSQL's Limit node handles limiting.

            // If we have pending matches, return one
            if let Some((build_ctid, build_score)) =
                state.custom_state_mut().pending_build_ctids.pop_front()
            {
                if let Some(slot) = Self::build_result_tuple(state, build_ctid, build_score) {
                    state.custom_state_mut().rows_returned += 1;
                    return slot;
                }
                continue;
            }

            // Need a driving row. If we don't have one, get the next one.
            if state.custom_state().current_driving_ctid.is_none() {
                // Get next driving row - either from executor or heap scan
                let (driving_ctid, driving_score) =
                    if state.custom_state().driving_executor.is_some() {
                        // Use the executor for incremental fetching
                        let executor = state.custom_state_mut().driving_executor.as_mut().unwrap();

                        // Note: We don't check executor.reached_limit() here because
                        // for joins the LIMIT applies to output rows (tracked by rows_returned),
                        // not input driving rows.
                        match executor.next_visible() {
                            executors::JoinExecResult::Visible { ctid, score } => (ctid, score),
                            executors::JoinExecResult::Eof => return std::ptr::null_mut(),
                        }
                    } else if let Some(driving_scan_desc) = state.custom_state().driving_scan_desc {
                        // Fallback to heap scan (no search predicate case)
                        let slot = state.custom_state().driving_fetch_slot;
                        let Some(slot) = slot else {
                            return std::ptr::null_mut();
                        };

                        let has_tuple = pg_sys::table_scan_getnextslot(
                            driving_scan_desc,
                            pg_sys::ScanDirection::ForwardScanDirection,
                            slot,
                        );

                        if !has_tuple {
                            return std::ptr::null_mut();
                        }

                        let ctid = item_pointer_to_u64((*slot).tts_tid);
                        (ctid, 0.0)
                    } else {
                        return std::ptr::null_mut();
                    };

                state.custom_state_mut().current_driving_ctid = Some(driving_ctid);
                state.custom_state_mut().current_driving_score = driving_score;

                // Reset build side scan for this driving row
                pg_sys::table_rescan(scan_desc, std::ptr::null_mut());
            }

            // Get build_matching_ctids reference (if build side has a search predicate)
            let build_matching_ctids = state.custom_state().build_matching_ctids.clone();

            // Now scan the build side for this driving row
            while pg_sys::table_scan_getnextslot(
                scan_desc,
                pg_sys::ScanDirection::ForwardScanDirection,
                build_slot,
            ) {
                let build_ctid = item_pointer_to_u64((*build_slot).tts_tid);

                // If build side has a search predicate, filter to only matching rows
                // and get the BM25 score (if build side needs scores)
                let build_score = if let Some(ref matching) = build_matching_ctids {
                    match matching.get(&build_ctid) {
                        Some(&score) => score,
                        None => continue, // Skip rows that don't match the build side predicate
                    }
                } else {
                    0.0 // No search predicate on build side
                };

                // For nested loop, we compare keys on the fly
                let driving_key_info = state.custom_state().driving_key_info.clone();
                let build_key_info = state.custom_state().build_key_info.clone();
                let driving_slot = state.custom_state().driving_fetch_slot;

                // Extract keys and compare
                if let Some(driving_slot) = driving_slot {
                    let driving_key = extract_composite_key(driving_slot, &driving_key_info);
                    let build_key = extract_composite_key(build_slot, &build_key_info);

                    // Check if keys match (or if it's a cross join)
                    let keys_match = match (&driving_key, &build_key) {
                        (Some(CompositeKey::CrossJoin), Some(CompositeKey::CrossJoin)) => true,
                        (Some(dk), Some(bk)) => dk == bk,
                        _ => false, // NULL keys don't match
                    };

                    if keys_match {
                        // Add to pending matches with score
                        state
                            .custom_state_mut()
                            .pending_build_ctids
                            .push_back((build_ctid, build_score));
                    }
                }
            }

            // Done with this driving row, clear it to get the next one
            state.custom_state_mut().current_driving_ctid = None;
        }
    }

    /// Extract the join key from the driving tuple.
    /// For cross joins (no equi-join keys), returns CompositeKey::CrossJoin.
    unsafe fn extract_driving_join_key(
        state: &mut CustomScanStateWrapper<Self>,
        driving_ctid: u64,
    ) -> Option<CompositeKey> {
        let driving_slot = state.custom_state().driving_fetch_slot?;
        let uses_heap_scan = state.custom_state().driving_uses_heap_scan;
        let is_cross_join = state.custom_state().is_cross_join;
        let driving_key_info = state.custom_state().driving_key_info.clone();

        // For cross joins, just return the cross join key
        if is_cross_join {
            if uses_heap_scan {
                // Tuple already in slot from heap scan, no visibility check needed
                return Some(CompositeKey::CrossJoin);
            } else {
                // Fetch tuple by ctid and verify visibility
                let vis_checker = state
                    .custom_state_mut()
                    .driving_visibility_checker
                    .as_mut()?;
                return vis_checker.exec_if_visible(driving_ctid, driving_slot, |_rel| {
                    Some(CompositeKey::CrossJoin)
                })?;
            }
        }

        if uses_heap_scan {
            // Tuple already in slot from heap scan
            extract_composite_key(driving_slot, &driving_key_info)
        } else {
            // Fetch tuple by ctid and verify visibility
            let vis_checker = state
                .custom_state_mut()
                .driving_visibility_checker
                .as_mut()?;

            vis_checker.exec_if_visible(driving_ctid, driving_slot, |_rel| {
                extract_composite_key(driving_slot, &driving_key_info)
            })?
        }
    }

    /// Build a result tuple from the current driving row and a build row.
    ///
    /// # Arguments
    /// * `state` - The custom scan state
    /// * `build_ctid` - The ctid of the build row to include in the result
    /// * `build_score` - The BM25 score of the build row (used if paradedb.score() references build side)
    unsafe fn build_result_tuple(
        state: &mut CustomScanStateWrapper<Self>,
        build_ctid: u64,
        build_score: f32,
    ) -> Option<*mut pg_sys::TupleTableSlot> {
        let result_slot = state.custom_state().result_slot?;
        let driving_slot = state.custom_state().driving_fetch_slot?;
        let build_slot = state.custom_state().build_scan_slot?;
        let driving_ctid = state.custom_state().current_driving_ctid?;
        let driving_is_outer = state.custom_state().driving_is_outer;
        let uses_heap_scan = state.custom_state().driving_uses_heap_scan;

        // Fetch driving tuple if not using heap scan
        // (with heap scan, tuple is already in the slot)
        if !uses_heap_scan {
            let driving_vis = state
                .custom_state_mut()
                .driving_visibility_checker
                .as_mut()?;
            driving_vis.exec_if_visible(driving_ctid, driving_slot, |_| ())?;
        }

        // Fetch build tuple using direct tuple fetch.
        // The build side ctids come from a sequential scan (hash table building), not from an index,
        // so we use fetch_tuple_direct which uses table_tuple_fetch_slot instead of
        // table_index_fetch_tuple. The latter is designed for index-derived ctids and may incorrectly
        // report tuples as "all_dead" when used with sequential scan ctids.
        let build_vis = match state.custom_state().build_visibility_checker.as_ref() {
            Some(vis) => vis,
            None => {
                return None;
            }
        };

        if !build_vis.fetch_tuple_direct(build_ctid, build_slot) {
            return None;
        }

        // Get the result tuple descriptor from the result slot
        let result_tupdesc = (*result_slot).tts_tupleDescriptor;
        let natts = (*result_tupdesc).natts as usize;

        // Clear the result slot
        pg_sys::ExecClearTuple(result_slot);

        // Make sure slots have all attributes deformed
        pg_sys::slot_getallattrs(driving_slot);
        pg_sys::slot_getallattrs(build_slot);

        // Map driving/build to outer/inner based on driving_is_outer
        let (outer_slot, inner_slot) = state.custom_state().outer_inner_slots();
        let outer_slot = outer_slot?;
        let inner_slot = inner_slot?;

        // Use the stored output_columns mapping to build the result tuple.
        // This was populated during planning (before setrefs transformed the Vars),
        // so it contains the original attribute numbers that work with our heap tuples.
        let output_columns = &state.custom_state().output_columns;

        // Fill the result slot based on the output column mapping
        let datums = (*result_slot).tts_values;
        let nulls = (*result_slot).tts_isnull;

        for (i, col_info) in output_columns.iter().enumerate() {
            if i >= natts {
                break;
            }

            // Handle score columns specially
            if col_info.is_score {
                // Use helper to determine which score (driving vs build) based on column's side
                let score = state
                    .custom_state()
                    .score_for_column(col_info.is_outer, build_score);

                use pgrx::IntoDatum;
                if let Some(datum) = score.into_datum() {
                    *datums.add(i) = datum;
                    *nulls.add(i) = false;
                } else {
                    *nulls.add(i) = true;
                }
                continue;
            }

            // Determine which slot to read from based on is_outer
            let source_slot = if col_info.is_outer {
                outer_slot
            } else {
                inner_slot
            };
            let original_attno = col_info.original_attno;

            // Get the attribute value from the source slot using the original attribute number
            if original_attno <= 0 {
                // System attribute, whole-row reference, or non-Var expression - set null
                *nulls.add(i) = true;
                continue;
            }

            let source_natts = (*(*source_slot).tts_tupleDescriptor).natts as i16;
            if original_attno > source_natts {
                *nulls.add(i) = true;
                continue;
            }

            let mut is_null = false;
            let value = pg_sys::slot_getattr(source_slot, original_attno as i32, &mut is_null);
            *datums.add(i) = value;
            *nulls.add(i) = is_null;
        }

        // Use ExecStoreVirtualTuple to properly mark the slot as containing a virtual tuple
        // This is safer than manually setting tts_flags
        pg_sys::ExecStoreVirtualTuple(result_slot);

        Some(result_slot)
    }
}

impl ExecMethod for JoinScan {
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        <JoinScan as PlainExecCapable>::exec_methods()
    }
}

impl PlainExecCapable for JoinScan {}

/// Estimate the memory size of a hash table entry.
fn estimate_entry_size(key: &CompositeKey) -> usize {
    let key_size = match key {
        CompositeKey::CrossJoin => 0,
        CompositeKey::Values(values) => {
            values
                .iter()
                .map(|v| v.data.len() + std::mem::size_of::<Vec<u8>>())
                .sum::<usize>()
                + std::mem::size_of::<Vec<KeyValue>>()
        }
    };
    // InnerRow (ctid: u64) + Vec overhead + HashMap entry overhead
    std::mem::size_of::<InnerRow>() + key_size + 64
}

/// Extract a composite key from a tuple slot using the given key info.
/// Returns None if any key column is NULL.
unsafe fn extract_composite_key(
    slot: *mut pg_sys::TupleTableSlot,
    key_info: &[JoinKeyInfo],
) -> Option<CompositeKey> {
    if key_info.is_empty() {
        return Some(CompositeKey::CrossJoin);
    }

    let mut values = Vec::with_capacity(key_info.len());
    for info in key_info {
        let mut is_null = false;
        let datum = pg_sys::slot_getattr(slot, info.attno, &mut is_null);
        if is_null {
            return None; // Skip rows with NULL keys (standard SQL behavior)
        }

        // Copy the datum value to owned bytes
        let key_value = copy_datum_to_key_value(datum, info.typlen, info.typbyval);
        values.push(key_value);
    }
    Some(CompositeKey::Values(values))
}

/// Extract non-equijoin predicates from a restrictlist.
/// These are the predicates that need to be evaluated after the hash join lookup.
/// Copy a datum to an owned KeyValue, handling pass-by-reference types.
unsafe fn copy_datum_to_key_value(datum: pg_sys::Datum, typlen: i16, typbyval: bool) -> KeyValue {
    if typbyval {
        // Pass-by-value: datum IS the value, copy its bytes directly
        let len = (typlen as usize).min(8);
        let bytes = datum.value().to_ne_bytes();
        KeyValue {
            data: bytes[..len].to_vec(),
        }
    } else if typlen == -1 {
        // Varlena type (TEXT, BYTEA, etc.)
        let varlena = datum.cast_mut_ptr::<pg_sys::varlena>();
        let detoasted = pg_sys::pg_detoast_datum_packed(varlena);
        // Use pgrx's varsize_any_exhdr and vardata_any macros
        let len = pgrx::varsize_any_exhdr(detoasted);
        let ptr = pgrx::vardata_any(detoasted) as *const u8;
        let data = std::slice::from_raw_parts(ptr, len).to_vec();
        KeyValue { data }
    } else if typlen == -2 {
        // Cstring
        let cstr = datum.cast_mut_ptr::<std::ffi::c_char>();
        let c_str = std::ffi::CStr::from_ptr(cstr);
        let data = c_str.to_bytes().to_vec();
        KeyValue { data }
    } else {
        // Fixed-length pass-by-reference
        let len = typlen as usize;
        let ptr = datum.cast_mut_ptr::<u8>();
        let data = std::slice::from_raw_parts(ptr, len).to_vec();
        KeyValue { data }
    }
}

/// Convert a serializable join-level expression to a runtime expression.
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
