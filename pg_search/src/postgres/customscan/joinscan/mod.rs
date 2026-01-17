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
    JoinCSClause, JoinKeyPair, JoinSideInfo, SerializableJoinLevelExpr, SerializableJoinSide,
    SerializableJoinType,
};
use self::privdat::PrivateData;
use self::scan_state::{
    CompositeKey, InnerRow, JoinKeyInfo, JoinLevelExpr, JoinScanState, JoinSide, KeyValue,
};
use crate::api::operator::anyelement_query_input_opoid;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
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
use crate::postgres::heap::{OwnedVisibilityChecker, VisibilityChecker};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::utils::{expr_collect_rtis, expr_contains_any_operator};
use crate::query::SearchQueryInput;
use crate::DEFAULT_STARTUP_COST;
use pgrx::itemptr::item_pointer_to_u64;
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
            // Use FastField executor for both sides in unlimited mode.
            // The current restriction exists because TopN executor needs a limit for
            // incremental fetching with dead tuple scaling.
            let limit = if (*root).limit_tuples > -1.0 {
                Some((*root).limit_tuples as usize)
            } else {
                return None;
            };

            // Extract information from both sides of the join
            let outer_side = extract_join_side_info(root, outerrel)?;
            let inner_side = extract_join_side_info(root, innerrel)?;

            // Extract join conditions from the restrict list
            let outer_rti = outer_side.heap_rti.unwrap_or(0);
            let inner_rti = inner_side.heap_rti.unwrap_or(0);
            let join_conditions = extract_join_conditions(extra, outer_rti, inner_rti);

            // If there are no equi-join keys but there ARE non-equijoin conditions,
            // don't propose JoinScan. The non-equijoin conditions can't be evaluated
            // properly in our current implementation (Var references use RTIs instead
            // of OUTER_VAR/INNER_VAR). Let PostgreSQL's native join handle it.
            let has_equi_join_keys = !join_conditions.equi_keys.is_empty();
            let has_non_equijoin_conditions = !join_conditions.other_conditions.is_empty();
            if !has_equi_join_keys && has_non_equijoin_conditions {
                return None;
            }

            // Build the join clause with join keys
            let mut join_clause = JoinCSClause::new()
                .with_outer_side(outer_side.clone())
                .with_inner_side(inner_side.clone())
                .with_join_type(SerializableJoinType::from(jointype))
                .with_limit(limit)
                .with_has_other_conditions(!join_conditions.other_conditions.is_empty());

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

            // If restrictlist contains @@@ operators, try to extract join-level predicates
            if join_conditions.has_search_predicate {
                join_clause = Self::extract_join_level_search_predicates(
                    root,
                    extra,
                    &outer_side,
                    &inner_side,
                    join_clause,
                );
            }

            // Check if this is a valid join for JoinScan
            // We need at least one side with a BM25 index AND a search predicate,
            // OR successfully extracted join-level predicates.
            let has_side_predicate = (outer_side.has_bm25_index && outer_side.has_search_predicate)
                || (inner_side.has_bm25_index && inner_side.has_search_predicate);
            let has_join_level_predicates = !join_clause.join_level_predicates.is_empty();

            if !has_side_predicate && !has_join_level_predicates {
                return None;
            }

            // Create the private data
            let private_data = PrivateData::new(join_clause.clone());

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
            let (startup_cost, total_cost, result_rows) =
                estimate_joinscan_cost(&join_clause, outerrel, innerrel, limit);

            // ORDER BY score pushdown: Check if query has ORDER BY paradedb.score()
            // for the driving side. When TopN executor is used (which is always the case
            // when there's a LIMIT), results are returned in score order, so we can
            // declare pathkeys to eliminate the Sort node PostgreSQL would otherwise add.
            let driving_side_rti = if join_clause.driving_side_is_outer() {
                outer_rti
            } else {
                inner_rti
            };
            let score_pathkey = extract_score_pathkey(root, driving_side_rti as pg_sys::Index);

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

            // Store the restrictlist in custom_private as a second list element
            // This is needed because PostgreSQL doesn't pass join clauses to PlanCustomPath
            // Note: This is stored for future use when join-level expression filtering is implemented
            let restrictlist = (*extra).restrictlist;

            if !restrictlist.is_null() {
                // Add the restrictlist to custom_private
                let mut private_list = PgList::<pg_sys::Node>::from_pg(custom_path.custom_private);
                // Add the restrictlist as the second element
                private_list.push(restrictlist.cast());
                custom_path.custom_private = private_list.into_pg();
            }

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

        // Show if there are additional filter conditions
        if join_clause.has_other_conditions {
            explainer.add_text("Has Filter", "true");
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
            let expr_str = format_join_level_expr(expr, &join_clause.join_level_predicates);
            explainer.add_text("Join Predicate", expr_str);
        }

        // Show limit if present
        if let Some(limit) = join_clause.limit {
            explainer.add_text("Limit", limit.to_string());
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
                    // Use TopN executor if we have a limit, otherwise use FastField
                    let executor = if let Some(limit) = join_clause.limit {
                        executors::JoinSideExecutor::new_topn(
                            limit,
                            &heaprel,
                            indexrelid,
                            query.clone(),
                            snapshot,
                        )
                    } else {
                        // FastField executor with scores enabled for paradedb.score() support
                        executors::JoinSideExecutor::new_fast_field(
                            &heaprel,
                            indexrelid,
                            query.clone(),
                            snapshot,
                            true, // need_scores for driving side
                        )
                    };
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
                // predicate, we pre-materialize ALL matching ctids into a HashSet upfront.
                // This defeats the incremental fetching benefit for large result sets.
                //
                // A better approach would be to use JoinSideExecutor for build side too:
                // - During hash table build, filter rows lazily using the executor
                // - This would allow early termination if hash table exceeds work_mem
                // - Could also enable build side ordering for merge-join style execution
                //
                // The collect_all_ctids() method in executors.rs was intended for this
                // but isn't currently used. Consider integrating it here.
                if let (Some(indexrelid), Some(ref query)) =
                    (build_side.indexrelid, &build_side.query)
                {
                    let indexrel = PgSearchRelation::open(indexrelid);
                    let search_reader = SearchIndexReader::open_with_context(
                        &indexrel,
                        query.clone(),
                        false, // don't need scores for build side
                        MvccSatisfies::Snapshot,
                        None,
                        None,
                    );

                    if let Ok(reader) = search_reader {
                        let mut build_ctids = std::collections::HashSet::new();

                        // Create a visibility checker to resolve stale ctids
                        let mut vis_checker = OwnedVisibilityChecker::new(&heaprel, snapshot);

                        let results = reader.search();
                        for (scored, _doc_address) in results {
                            // Verify tuple visibility and get its current ctid
                            if let Some(current_ctid) = vis_checker.get_current_ctid(scored.ctid) {
                                build_ctids.insert(current_ctid);
                            }
                        }

                        state.custom_state_mut().build_matching_ctids = Some(build_ctids);
                    }
                }

                state.custom_state_mut().build_heaprel = Some(heaprel);
            }

            // Initialize join qual evaluation if we have non-equijoin conditions
            if join_clause.has_other_conditions {
                // Get the restrictlist from custom_private (second element)
                let cscan = state.csstate.ss.ps.plan as *mut pg_sys::CustomScan;
                let private_list = PgList::<pg_sys::Node>::from_pg((*cscan).custom_private);

                if private_list.len() > 1 {
                    if let Some(restrictlist_node) = private_list.get_ptr(1) {
                        let restrictlist = restrictlist_node as *mut pg_sys::List;

                        // Build qual expression from non-equijoin RestrictInfos
                        let quals = extract_non_equijoin_quals(restrictlist, &join_clause);
                        if !quals.is_null() {
                            // Create expression context
                            let econtext = pg_sys::CreateExprContext(estate);

                            // Initialize expression state
                            let qual_state = pg_sys::ExecInitQual(quals, &mut state.csstate.ss.ps);

                            state.custom_state_mut().join_qual_state = Some(qual_state);
                            state.custom_state_mut().join_qual_econtext = Some(econtext);
                        }
                    }
                }
            }

            // Initialize join-level predicate evaluation if we have a join-level expression
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
    }

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        unsafe {
            // Check if we've reached the limit
            if state.custom_state().reached_limit() {
                return std::ptr::null_mut();
            }

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
                if let Some(build_ctid) = state.custom_state_mut().pending_build_ctids.pop_front() {
                    if let Some(slot) = Self::build_result_tuple(state, build_ctid) {
                        // Check join-level predicate using the expression tree
                        if let Some(ref expr) = state.custom_state().join_level_expr {
                            let driving_is_outer = state.custom_state().driving_is_outer;
                            let driving_slot = state.custom_state().driving_fetch_slot;
                            let build_slot = state.custom_state().build_scan_slot;

                            // Map driving/build to outer/inner based on driving_is_outer
                            let (outer_slot, inner_slot) = if driving_is_outer {
                                (driving_slot, build_slot)
                            } else {
                                (build_slot, driving_slot)
                            };

                            // Convert heap ctids to u64 using the SAME encoding as the BM25 index
                            let outer_ctid_u64 = outer_slot
                                .map(|s| crate::postgres::utils::item_pointer_to_u64((*s).tts_tid))
                                .unwrap_or(0);

                            let inner_ctid_u64 = inner_slot
                                .map(|s| crate::postgres::utils::item_pointer_to_u64((*s).tts_tid))
                                .unwrap_or(0);

                            // Evaluate the full boolean expression tree
                            let ctid_sets = &state.custom_state().join_level_ctid_sets;
                            if !expr.evaluate(outer_ctid_u64, inner_ctid_u64, ctid_sets) {
                                // Predicate not satisfied, try next match
                                continue;
                            }
                        }

                        // Evaluate join qual if we have one
                        // TODO(expr-context): This expression context setup may not handle all edge
                        // cases correctly. Potential issues to test:
                        // - Expressions referencing system columns (ctid, tableoid, xmin, etc.)
                        // - Expressions with correlated subqueries
                        // - Expressions with volatile functions (should be re-evaluated each time)
                        // - Expressions after setrefs that use INDEX_VAR vs original RTI references
                        //
                        // The current approach sets ecxt_scantuple to our result slot and
                        // ecxt_outertuple/ecxt_innertuple to the source slots for backwards
                        // compatibility. This may need refinement based on how PostgreSQL's
                        // set_customscan_references transforms the qual expressions.
                        if let (Some(qual_state), Some(econtext)) = (
                            state.custom_state().join_qual_state,
                            state.custom_state().join_qual_econtext,
                        ) {
                            // Set up the expression context with both tuple slots
                            // After fix_scan_list, Vars in custom_exprs are transformed to INDEX_VAR
                            // referencing positions in custom_scan_tlist (our result slot)
                            (*econtext).ecxt_scantuple = slot;

                            // Also set outer/inner tuples for backwards compatibility
                            // In case any Vars weren't transformed
                            let driving_slot_ptr = state.custom_state().driving_fetch_slot;
                            let build_slot_ptr = state.custom_state().build_scan_slot;
                            let driving_is_outer = state.custom_state().driving_is_outer;

                            if let (Some(ds), Some(bs)) = (driving_slot_ptr, build_slot_ptr) {
                                if driving_is_outer {
                                    (*econtext).ecxt_outertuple = ds;
                                    (*econtext).ecxt_innertuple = bs;
                                } else {
                                    (*econtext).ecxt_outertuple = bs;
                                    (*econtext).ecxt_innertuple = ds;
                                }
                            }

                            // Evaluate the qual - returns true if tuple passes
                            let passes = pg_sys::ExecQual(qual_state, econtext);
                            if !passes {
                                // Qual failed, try next match
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

                        // Check if we've reached the limit
                        if executor.reached_limit() {
                            return std::ptr::null_mut();
                        }

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
                // Clone the ctids to avoid borrow issues
                let build_ctids: Vec<u64> = state
                    .custom_state()
                    .hash_table
                    .get(&key)
                    .map(|rows| rows.iter().map(|r| r.ctid).collect())
                    .unwrap_or_default();

                for ctid in build_ctids {
                    state.custom_state_mut().pending_build_ctids.push_back(ctid);
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
            if let Some(ref matching) = build_matching_ctids {
                if !matching.contains(&ctid) {
                    continue; // Skip rows that don't match the build side predicate
                }
            }

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

            // Add to hash table
            state
                .custom_state_mut()
                .hash_table
                .entry(key)
                .or_default()
                .push(InnerRow { ctid });
        }

        // Store whether we're doing a cross join for later use
        state.custom_state_mut().is_cross_join = !has_equi_join_keys;
    }

    /// Execute nested loop join when hash join exceeds memory limit.
    /// This is a fallback that uses less memory but has O(N*M) complexity.
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
            // Check limit
            if state.custom_state().reached_limit() {
                return std::ptr::null_mut();
            }

            // If we have pending matches, return one
            if let Some(build_ctid) = state.custom_state_mut().pending_build_ctids.pop_front() {
                if let Some(slot) = Self::build_result_tuple(state, build_ctid) {
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

                        // Check if we've reached the limit
                        if executor.reached_limit() {
                            return std::ptr::null_mut();
                        }

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
                if let Some(ref matching) = build_matching_ctids {
                    if !matching.contains(&build_ctid) {
                        continue; // Skip rows that don't match the build side predicate
                    }
                }

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
                        // Add to pending matches
                        state
                            .custom_state_mut()
                            .pending_build_ctids
                            .push_back(build_ctid);
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
    unsafe fn build_result_tuple(
        state: &mut CustomScanStateWrapper<Self>,
        build_ctid: u64,
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

        // Fetch build tuple
        let build_vis = state.custom_state_mut().build_visibility_checker.as_mut()?;
        build_vis.exec_if_visible(build_ctid, build_slot, |_| ())?;

        // Get the result tuple descriptor from the result slot
        let result_tupdesc = (*result_slot).tts_tupleDescriptor;
        let natts = (*result_tupdesc).natts as usize;

        // Clear the result slot
        pg_sys::ExecClearTuple(result_slot);

        // Make sure slots have all attributes deformed
        pg_sys::slot_getallattrs(driving_slot);
        pg_sys::slot_getallattrs(build_slot);

        // Map slots based on driving_is_outer:
        // - If driving_is_outer: driving_slot=outer, build_slot=inner
        // - If driving_is_inner: driving_slot=inner, build_slot=outer
        let (outer_slot, inner_slot) = if driving_is_outer {
            (driving_slot, build_slot)
        } else {
            (build_slot, driving_slot)
        };

        // Use the stored output_columns mapping to build the result tuple.
        // This was populated during planning (before setrefs transformed the Vars),
        // so it contains the original attribute numbers that work with our heap tuples.
        let output_columns = &state.custom_state().output_columns;

        // Fill the result slot based on the output column mapping
        let datums = (*result_slot).tts_values;
        let nulls = (*result_slot).tts_isnull;

        // Get the driving side score for score columns
        let driving_score = state.custom_state().current_driving_score;

        for (i, col_info) in output_columns.iter().enumerate() {
            if i >= natts {
                break;
            }

            // Handle score columns specially
            if col_info.is_score {
                // Score comes from the driving side's search results
                use pgrx::IntoDatum;
                if let Some(datum) = driving_score.into_datum() {
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

    /// Extract join-level search predicates from the restrict list and build an expression tree.
    ///
    /// This recursively transforms the PostgreSQL expression tree into a `SerializableJoinLevelExpr`
    /// that preserves the full AND/OR/NOT structure. Each single-table sub-expression becomes a
    /// leaf predicate that references a Tantivy query.
    ///
    /// For example, `(p.desc @@@ 'wireless' AND NOT p.desc @@@ 'mouse') OR s.info @@@ 'shipping'`:
    /// - Creates an OR expression with two children
    /// - Left child: a leaf predicate for products with query `'wireless' AND NOT 'mouse'`
    /// - Right child: a leaf predicate for suppliers with query `'shipping'`
    unsafe fn extract_join_level_search_predicates(
        root: *mut pg_sys::PlannerInfo,
        extra: *mut pg_sys::JoinPathExtraData,
        outer_side: &JoinSideInfo,
        inner_side: &JoinSideInfo,
        mut join_clause: JoinCSClause,
    ) -> JoinCSClause {
        if extra.is_null() {
            return join_clause;
        }

        let restrictlist = (*extra).restrictlist;
        if restrictlist.is_null() {
            return join_clause;
        }

        let outer_rti = outer_side.heap_rti.unwrap_or(0);
        let inner_rti = inner_side.heap_rti.unwrap_or(0);
        let search_op = anyelement_query_input_opoid();

        let restrict_infos = PgList::<pg_sys::RestrictInfo>::from_pg(restrictlist);

        // Collect all expressions that have search predicates
        let mut expr_trees: Vec<SerializableJoinLevelExpr> = Vec::new();

        for ri in restrict_infos.iter_ptr() {
            if ri.is_null() || (*ri).clause.is_null() {
                continue;
            }

            let clause = (*ri).clause;

            // Check if this clause contains any search predicates
            if !expr_contains_any_operator(clause.cast(), &[search_op]) {
                continue;
            }

            // Transform this clause into an expression tree
            if let Some(expr) = Self::transform_to_expr(
                root,
                clause.cast(),
                outer_rti,
                inner_rti,
                outer_side,
                inner_side,
                &mut join_clause,
            ) {
                expr_trees.push(expr);
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

        join_clause
    }

    /// Recursively transform a PostgreSQL expression node into a SerializableJoinLevelExpr.
    ///
    /// - For single-table sub-trees with search predicates: extract as a leaf predicate
    /// - For BoolExpr (AND/OR/NOT): recursively transform children
    /// - For expressions without search predicates or referencing both tables: return None
    unsafe fn transform_to_expr(
        root: *mut pg_sys::PlannerInfo,
        node: *mut pg_sys::Node,
        outer_rti: pg_sys::Index,
        inner_rti: pg_sys::Index,
        outer_side: &JoinSideInfo,
        inner_side: &JoinSideInfo,
        join_clause: &mut JoinCSClause,
    ) -> Option<SerializableJoinLevelExpr> {
        if node.is_null() {
            return None;
        }

        let search_op = anyelement_query_input_opoid();

        // First, check if this node contains any search predicates
        if !expr_contains_any_operator(node, &[search_op]) {
            return None;
        }

        // Check which tables this expression references
        let rtis = expr_collect_rtis(node);
        let refs_outer = rtis.contains(&outer_rti);
        let refs_inner = rtis.contains(&inner_rti);

        // If this is a single-table expression, extract it as a leaf predicate
        if rtis.len() == 1 && (refs_outer || refs_inner) {
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

        // If this expression references both tables (or neither of ours),
        // we need to look inside if it's a BoolExpr
        let node_type = (*node).type_;

        if node_type == pg_sys::NodeTag::T_BoolExpr {
            let boolexpr = node as *mut pg_sys::BoolExpr;
            let boolop = (*boolexpr).boolop;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);

            match boolop {
                pg_sys::BoolExprType::AND_EXPR => {
                    let mut children = Vec::new();
                    for arg in args.iter_ptr() {
                        if let Some(child_expr) = Self::transform_to_expr(
                            root,
                            arg,
                            outer_rti,
                            inner_rti,
                            outer_side,
                            inner_side,
                            join_clause,
                        ) {
                            children.push(child_expr);
                        }
                    }
                    if children.is_empty() {
                        None
                    } else if children.len() == 1 {
                        Some(children.pop().unwrap())
                    } else {
                        Some(SerializableJoinLevelExpr::And(children))
                    }
                }
                pg_sys::BoolExprType::OR_EXPR => {
                    let mut children = Vec::new();
                    for arg in args.iter_ptr() {
                        if let Some(child_expr) = Self::transform_to_expr(
                            root,
                            arg,
                            outer_rti,
                            inner_rti,
                            outer_side,
                            inner_side,
                            join_clause,
                        ) {
                            children.push(child_expr);
                        }
                    }
                    if children.is_empty() {
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
                        if let Some(child_expr) = Self::transform_to_expr(
                            root,
                            arg,
                            outer_rti,
                            inner_rti,
                            outer_side,
                            inner_side,
                            join_clause,
                        ) {
                            return Some(SerializableJoinLevelExpr::Not(Box::new(child_expr)));
                        }
                    }
                    None
                }
                _ => None,
            }
        } else {
            // Not a BoolExpr and not single-table - can't handle
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

/// Get type length and pass-by-value info for a given type OID.
unsafe fn get_type_info(type_oid: pg_sys::Oid) -> (i16, bool) {
    let mut typlen: i16 = 0;
    let mut typbyval: bool = false;
    pg_sys::get_typlenbyval(type_oid, &mut typlen, &mut typbyval);
    (typlen, typbyval)
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
unsafe fn extract_non_equijoin_quals(
    restrictlist: *mut pg_sys::List,
    join_clause: &JoinCSClause,
) -> *mut pg_sys::List {
    if restrictlist.is_null() {
        return std::ptr::null_mut();
    }

    let mut quals = PgList::<pg_sys::Expr>::new();
    let restrict_infos = PgList::<pg_sys::RestrictInfo>::from_pg(restrictlist);
    let outer_rti = join_clause.outer_side.heap_rti.unwrap_or(0);
    let inner_rti = join_clause.inner_side.heap_rti.unwrap_or(0);

    for ri in restrict_infos.iter_ptr() {
        if ri.is_null() || (*ri).clause.is_null() {
            continue;
        }

        let clause = (*ri).clause;

        // Skip if this is an equi-join condition (Var = Var using equality operator between outer and inner)
        let mut is_equi_join = false;
        if (*clause).type_ == pg_sys::NodeTag::T_OpExpr {
            let opexpr = clause as *mut pg_sys::OpExpr;
            let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);

            if args.len() == 2 {
                let arg0 = args.get_ptr(0).unwrap();
                let arg1 = args.get_ptr(1).unwrap();

                // Check if operator is an equality operator
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

                    // Check if it's an equi-join between outer and inner
                    if (varno0 == outer_rti && varno1 == inner_rti)
                        || (varno0 == inner_rti && varno1 == outer_rti)
                    {
                        is_equi_join = true;
                    }
                }
            }
        }

        if !is_equi_join {
            quals.push(clause);
        }
    }

    if quals.is_empty() {
        std::ptr::null_mut()
    } else {
        // Convert list of Exprs to implicit AND using make_ands_explicit
        if quals.len() == 1 {
            let single = quals.get_ptr(0).unwrap();
            let mut result = PgList::<pg_sys::Expr>::new();
            result.push(single);
            result.into_pg()
        } else {
            // Create a BoolExpr with AND
            quals.into_pg()
        }
    }
}

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
/// Returns (startup_cost, total_cost, result_rows).
///
/// Cost model components:
/// - Startup cost: Build side sequential scan + hash table construction
/// - Per-tuple cost: Hash lookup + heap fetch for driving side
/// - Result rows: Min(limit, estimated_matches)
unsafe fn estimate_joinscan_cost(
    join_clause: &JoinCSClause,
    outerrel: *mut pg_sys::RelOptInfo,
    innerrel: *mut pg_sys::RelOptInfo,
    limit: Option<usize>,
) -> (f64, f64, f64) {
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

    (startup_cost, total_cost, result_rows)
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
        SerializableJoinLevelExpr::And(children) => {
            let parts: Vec<_> = children
                .iter()
                .map(|c| format_join_level_expr(c, predicates))
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
                .map(|c| format_join_level_expr(c, predicates))
                .collect();
            if parts.len() == 1 {
                parts.into_iter().next().unwrap()
            } else {
                format!("({})", parts.join(" OR "))
            }
        }
        SerializableJoinLevelExpr::Not(child) => {
            format!("NOT {}", format_join_level_expr(child, predicates))
        }
    }
}
