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
pub mod privdat;
pub mod scan_state;

use self::build::{JoinCSClause, JoinKeyPair, JoinSideInfo, SerializableJoinType};
use self::privdat::PrivateData;
use self::scan_state::{CompositeKey, InnerRow, JoinKeyInfo, JoinScanState, KeyValue};
use crate::api::operator::anyelement_query_input_opoid;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::builders::custom_path::{
    CustomPathBuilder, Flags, RestrictInfoType,
};
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::customscan::range_table::{bms_iter, get_plain_relation_relid};
use crate::postgres::customscan::{CustomScan, ExecMethod, JoinPathlistHookArgs, PlainExecCapable};
use crate::postgres::heap::{OwnedVisibilityChecker, VisibilityChecker};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::utils::{expr_contains_any_operator, expr_extract_search_opexprs};
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

            // For M1, we only handle INNER JOINs
            if jointype != pg_sys::JoinType::JOIN_INNER {
                return None;
            }

            // Check if there's a LIMIT in the query
            let limit = if (*root).limit_tuples > -1.0 {
                Some((*root).limit_tuples as usize)
            } else {
                // For M1, we require a LIMIT for Single Feature joins
                // (Join-level predicates for Aggregate Score joins are deferred to M3)
                return None;
            };

            // Extract information from both sides of the join
            let outer_side = extract_join_side_info(root, outerrel)?;
            let inner_side = extract_join_side_info(root, innerrel)?;

            // Extract join conditions from the restrict list
            let outer_rti = outer_side.heap_rti.unwrap_or(0);
            let inner_rti = inner_side.heap_rti.unwrap_or(0);
            let join_conditions = extract_join_conditions(extra, outer_rti, inner_rti);

            // Check if this is a valid join for JoinScan
            // We need at least one side with a BM25 index AND a search predicate,
            // OR a join-level search predicate (like OR across tables).
            let has_side_predicate = (outer_side.has_bm25_index && outer_side.has_search_predicate)
                || (inner_side.has_bm25_index && inner_side.has_search_predicate);
            let has_join_level_predicate = join_conditions.has_search_predicate;

            if !has_side_predicate && !has_join_level_predicate {
                return None;
            }

            // Build the join clause with join keys
            let mut join_clause = JoinCSClause::new()
                .with_outer_side(outer_side.clone())
                .with_inner_side(inner_side.clone())
                .with_join_type(SerializableJoinType::from(jointype))
                .with_limit(limit)
                .with_has_other_conditions(!join_conditions.other_conditions.is_empty())
                .with_has_join_level_search_predicate(join_conditions.has_search_predicate);

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

            // If we have join-level predicates, extract the search queries from them
            if has_join_level_predicate {
                join_clause = Self::extract_join_level_search_predicates(
                    root,
                    extra,
                    &outer_side,
                    &inner_side,
                    join_clause,
                );
            }

            // Create the private data
            let private_data = PrivateData::new(join_clause);

            // Build the CustomPath
            // For now, use simple cost estimates (will be improved later)
            let startup_cost = DEFAULT_STARTUP_COST;
            let total_cost = startup_cost + 1000.0; // Arbitrary cost for now

            // Get the cheapest total paths from outer and inner relations
            // These are needed so PostgreSQL can resolve Vars in custom_scan_tlist
            let outer_path = (*outerrel).cheapest_total_path;
            let inner_path = (*innerrel).cheapest_total_path;

            // Force the path to be chosen when we have a valid join opportunity
            // Add child paths so set_customscan_references can resolve Vars
            let builder = builder
                .set_flag(Flags::Force)
                .set_startup_cost(startup_cost)
                .set_total_cost(total_cost)
                .set_rows(limit.unwrap_or(1000) as f64)
                .add_custom_path(outer_path)
                .add_custom_path(inner_path);

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
                    });
                } else {
                    // Non-Var expression - mark as null (attno = 0)
                    output_columns.push(privdat::OutputColumnInfo {
                        is_outer: false,
                        original_attno: 0,
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

        // Show outer side info
        if let Some(rti) = join_clause.outer_side.heap_rti {
            explainer.add_text("Outer RTI", rti.to_string());
        }
        if join_clause.outer_side.has_search_predicate {
            if let Some(ref query) = join_clause.outer_side.query {
                explainer.add_query(query);
            }
        }

        // Show inner side info
        if let Some(rti) = join_clause.inner_side.heap_rti {
            explainer.add_text("Inner RTI", rti.to_string());
        }
        if join_clause.inner_side.has_search_predicate {
            if let Some(ref query) = join_clause.inner_side.query {
                explainer.add_query(query);
            }
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
                    type_oid: jk.type_oid,
                    typlen: jk.typlen,
                    typbyval: jk.typbyval,
                });
                state.custom_state_mut().driving_key_info.push(JoinKeyInfo {
                    attno: driving_attno,
                    type_oid: jk.type_oid,
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

                // Try to open search reader for driving side (if it has a search predicate)
                if let (Some(indexrelid), Some(ref query)) =
                    (driving_side.indexrelid, &driving_side.query)
                {
                    let indexrel = PgSearchRelation::open(indexrelid);
                    let search_reader = SearchIndexReader::open_with_context(
                        &indexrel,
                        query.clone(),
                        true, // need_scores for the driving side
                        MvccSatisfies::Snapshot,
                        None,
                        None,
                    );
                    if let Ok(reader) = search_reader {
                        state.custom_state_mut().driving_search_reader = Some(reader);
                    }
                    state.custom_state_mut().driving_indexrel = Some(indexrel);
                }

                // If no search reader, start a heap scan on driving side
                // This happens when we have join-level search predicates (OR across tables)
                // but no side-level predicates
                if state.custom_state().driving_search_reader.is_none() {
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

            // Initialize join-level predicate matching sets if we have join-level predicates
            let join_level_predicates = state
                .custom_state()
                .join_clause
                .join_level_predicates
                .clone();
            if !join_level_predicates.is_empty() {
                state.custom_state_mut().has_join_level_predicates = true;

                let outer_rti = join_clause.outer_side.heap_rti.unwrap_or(0);
                let inner_rti = join_clause.inner_side.heap_rti.unwrap_or(0);

                // Query each join-level predicate's BM25 index and collect matching ctids.
                // We use OwnedVisibilityChecker to resolve index ctids to current heap locations,
                // handling the case where tuples moved after UPDATE but before VACUUM.
                // See VisibilityChecker docs for details on two-layer visibility.
                for predicate in &join_level_predicates {
                    let indexrel = PgSearchRelation::open(predicate.indexrelid);

                    // Get the heaprel for this predicate to do visibility checks
                    let heaprelid = if predicate.rti == outer_rti {
                        join_clause.outer_side.heaprelid
                    } else {
                        join_clause.inner_side.heaprelid
                    };

                    let search_reader = SearchIndexReader::open_with_context(
                        &indexrel,
                        predicate.query.clone(),
                        false, // don't need scores
                        MvccSatisfies::Snapshot,
                        None,
                        None,
                    );

                    if let Ok(reader) = search_reader {
                        // Create a visibility checker for this relation (owns its slot)
                        let mut vis_checker = heaprelid.map(|hrid| {
                            let heaprel = PgSearchRelation::open(hrid);
                            OwnedVisibilityChecker::new(&heaprel, snapshot)
                        });

                        let results = reader.search();
                        for (scored, _doc_address) in results {
                            // Verify tuple visibility and get its current ctid
                            let current_ctid = match vis_checker {
                                Some(ref mut checker) => checker.get_current_ctid(scored.ctid),
                                None => Some(scored.ctid),
                            };

                            if let Some(ctid) = current_ctid {
                                if predicate.rti == outer_rti {
                                    state.custom_state_mut().outer_matching_ctids.insert(ctid);
                                } else if predicate.rti == inner_rti {
                                    state.custom_state_mut().inner_matching_ctids.insert(ctid);
                                }
                            }
                        }
                        // vis_checker is dropped here, cleaning up the slot
                    }
                }
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
                        // Check join-level predicate (for OR across tables)
                        if state.custom_state().has_join_level_predicates {
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
                                .map(|s| crate::postgres::utils::item_pointer_to_u64((*s).tts_tid));

                            let inner_ctid_u64 = inner_slot
                                .map(|s| crate::postgres::utils::item_pointer_to_u64((*s).tts_tid));

                            let outer_matches = outer_ctid_u64
                                .map(|ctid| {
                                    state.custom_state().outer_matching_ctids.contains(&ctid)
                                })
                                .unwrap_or(false);
                            let inner_matches = inner_ctid_u64
                                .map(|ctid| {
                                    state.custom_state().inner_matching_ctids.contains(&ctid)
                                })
                                .unwrap_or(false);

                            // For OR semantics: pass if either side matches
                            if !outer_matches && !inner_matches {
                                // Predicate not satisfied, try next match
                                continue;
                            }
                        }

                        // Evaluate join qual if we have one
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

                // Get next driving row - either from search reader or heap scan
                let (driving_ctid, driving_score) =
                    if state.custom_state().driving_search_reader.is_some() {
                        // Use search reader
                        let need_init = state.custom_state().driving_search_results.is_none();
                        if need_init {
                            let reader = state.custom_state().driving_search_reader.as_ref();
                            if let Some(reader) = reader {
                                let results = reader.search();
                                state.custom_state_mut().driving_search_results = Some(results);
                            } else {
                                return std::ptr::null_mut();
                            }
                        }

                        let driving_search_results =
                            state.custom_state_mut().driving_search_results.as_mut();
                        let Some(results) = driving_search_results else {
                            return std::ptr::null_mut();
                        };

                        let next_result = results.next();
                        let Some((scored, _doc_address)) = next_result else {
                            return std::ptr::null_mut(); // No more search results
                        };

                        (scored.ctid, scored.bm25)
                    } else if let Some(scan_desc) = state.custom_state().driving_scan_desc {
                        // Use heap scan (join-level predicates with no side-level predicates)
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
        state.custom_state_mut().driving_indexrel = None;
        state.custom_state_mut().driving_search_reader = None;
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

        // Scan all build side tuples
        while pg_sys::table_scan_getnextslot(
            scan_desc,
            pg_sys::ScanDirection::ForwardScanDirection,
            slot,
        ) {
            // Extract the ctid from the slot
            let ctid = item_pointer_to_u64((*slot).tts_tid);

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
                // Get next driving row
                let (driving_ctid, driving_score) =
                    if state.custom_state().driving_search_reader.is_some() {
                        let need_init = state.custom_state().driving_search_results.is_none();
                        if need_init {
                            let reader = state.custom_state().driving_search_reader.as_ref();
                            if let Some(reader) = reader {
                                let results = reader.search();
                                state.custom_state_mut().driving_search_results = Some(results);
                            } else {
                                return std::ptr::null_mut();
                            }
                        }

                        let driving_search_results =
                            state.custom_state_mut().driving_search_results.as_mut();
                        let Some(results) = driving_search_results else {
                            return std::ptr::null_mut();
                        };

                        let next_result = results.next();
                        let Some((scored, _doc_address)) = next_result else {
                            return std::ptr::null_mut();
                        };

                        (scored.ctid, scored.bm25)
                    } else if let Some(driving_scan_desc) = state.custom_state().driving_scan_desc {
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

            // Now scan the build side for this driving row
            while pg_sys::table_scan_getnextslot(
                scan_desc,
                pg_sys::ScanDirection::ForwardScanDirection,
                build_slot,
            ) {
                let build_ctid = item_pointer_to_u64((*build_slot).tts_tid);

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

        for (i, col_info) in output_columns.iter().enumerate() {
            if i >= natts {
                break;
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

    /// Extract join-level search predicates from the restrictlist.
    /// This finds @@@ operators in OR conditions and extracts the search queries.
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

        let restrict_infos = PgList::<pg_sys::RestrictInfo>::from_pg(restrictlist);

        for ri in restrict_infos.iter_ptr() {
            if ri.is_null() || (*ri).clause.is_null() {
                continue;
            }

            let clause = (*ri).clause;

            // Extract search predicate OpExprs from this clause, filtered to outer/inner RTIs
            let search_op = anyelement_query_input_opoid();
            let opexprs: Vec<_> = expr_extract_search_opexprs(clause.cast(), &[search_op])
                .into_iter()
                .filter(|(rti, _)| *rti == outer_rti || *rti == inner_rti)
                .collect();

            // For each OpExpr, extract the SearchQueryInput using the qual extraction machinery
            for (rti, opexpr) in opexprs {
                // Determine which side this predicate applies to
                let (indexrelid, bm25_index) = if rti == outer_rti {
                    if let Some(oid) = outer_side.indexrelid {
                        if let Some((_, idx)) =
                            rel_get_bm25_index(outer_side.heaprelid.unwrap_or(pg_sys::InvalidOid))
                        {
                            (oid, Some(idx))
                        } else {
                            (oid, None)
                        }
                    } else {
                        continue;
                    }
                } else if rti == inner_rti {
                    if let Some(oid) = inner_side.indexrelid {
                        if let Some((_, idx)) =
                            rel_get_bm25_index(inner_side.heaprelid.unwrap_or(pg_sys::InvalidOid))
                        {
                            (oid, Some(idx))
                        } else {
                            (oid, None)
                        }
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };

                // Create a RestrictInfo list containing just this OpExpr
                // We need to wrap the OpExpr in a RestrictInfo for extract_quals
                let mut ri_list = PgList::<pg_sys::RestrictInfo>::new();
                let fake_ri = pg_sys::palloc0(std::mem::size_of::<pg_sys::RestrictInfo>())
                    as *mut pg_sys::RestrictInfo;
                (*fake_ri).type_ = pg_sys::NodeTag::T_RestrictInfo;
                (*fake_ri).clause = opexpr.cast();
                ri_list.push(fake_ri);

                // Use extract_quals to convert to SearchQueryInput
                if let Some(bm25_idx) = bm25_index {
                    let context = PlannerContext::from_planner(root);
                    let mut state = QualExtractState::default();

                    if let Some(qual) = extract_quals(
                        &context,
                        rti,
                        ri_list.as_ptr().cast(),
                        anyelement_query_input_opoid(),
                        RestrictInfoType::BaseRelation,
                        &bm25_idx,
                        false,
                        &mut state,
                        false, // Don't attempt pushdown for join-level predicates
                    ) {
                        let query = SearchQueryInput::from(&qual);
                        join_clause = join_clause.add_join_level_predicate(rti, indexrelid, query);
                    }
                }
            }
        }

        join_clause
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

        // Try to identify equi-join conditions (OpExpr with Var = Var)
        let mut is_equi_join = false;

        if (*clause).type_ == pg_sys::NodeTag::T_OpExpr {
            let opexpr = clause as *mut pg_sys::OpExpr;
            let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);

            // Equi-join: should have exactly 2 args, both Var nodes
            if args.len() == 2 {
                let arg0 = args.get_ptr(0).unwrap();
                let arg1 = args.get_ptr(1).unwrap();

                if (*arg0).type_ == pg_sys::NodeTag::T_Var
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
        if !is_equi_join {
            result.other_conditions.push(ri);
        }
    }

    result
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

        // Skip if this is an equi-join condition (Var = Var between outer and inner)
        let mut is_equi_join = false;
        if (*clause).type_ == pg_sys::NodeTag::T_OpExpr {
            let opexpr = clause as *mut pg_sys::OpExpr;
            let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);

            if args.len() == 2 {
                let arg0 = args.get_ptr(0).unwrap();
                let arg1 = args.get_ptr(1).unwrap();

                if (*arg0).type_ == pg_sys::NodeTag::T_Var
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

    // For now, we only handle single base relations on each side.
    // Multi-relation joins on one side would require more complex handling.
    let mut rti_iter = bms_iter(relids);
    let rti = rti_iter.next()?;

    // If there are multiple relations on this side, we can't handle it yet
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
