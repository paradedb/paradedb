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

use self::build::{JoinCSClause, JoinSideInfo, SerializableJoinType};
use self::privdat::PrivateData;
use self::scan_state::JoinScanState;
use crate::api::operator::anyelement_query_input_opoid;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::builders::custom_path::{CustomPathBuilder, Flags};
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::customscan::{CustomScan, ExecMethod, JoinPathlistHookArgs, PlainExecCapable};
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::query::SearchQueryInput;
use crate::DEFAULT_STARTUP_COST;
use pgrx::itemptr::item_pointer_to_u64;
use pgrx::{pg_sys, PgList};
use scan_state::InnerRow;
use std::ffi::CStr;

#[derive(Default)]
pub struct JoinScan;

/// Helper to iterate over Bitmapset members
unsafe fn bms_iter(bms: *mut pg_sys::Bitmapset) -> impl Iterator<Item = pg_sys::Index> {
    let mut set_bit: i32 = -1;
    std::iter::from_fn(move || {
        set_bit = pg_sys::bms_next_member(bms, set_bit);
        if set_bit < 0 {
            None
        } else {
            Some(set_bit as pg_sys::Index)
        }
    })
}

/// Extract equi-join keys from the restrict list (e.g., p.supplier_id = s.id).
/// Returns a list of (outer_attno, inner_attno) pairs.
unsafe fn extract_join_keys(
    extra: *mut pg_sys::JoinPathExtraData,
    outer_rti: pg_sys::Index,
    inner_rti: pg_sys::Index,
) -> Vec<(pg_sys::AttrNumber, pg_sys::AttrNumber)> {
    let mut keys = Vec::new();

    if extra.is_null() {
        return keys;
    }

    let restrictlist = (*extra).restrictlist;
    if restrictlist.is_null() {
        return keys;
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

        // Look for OpExpr nodes that could be equi-join conditions
        if (*clause).type_ != pg_sys::NodeTag::T_OpExpr {
            continue;
        }

        let opexpr = clause as *mut pg_sys::OpExpr;
        let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);

        // Equi-join: should have exactly 2 args
        if args.len() != 2 {
            continue;
        }

        let arg0 = args.get_ptr(0).unwrap();
        let arg1 = args.get_ptr(1).unwrap();

        // Both args should be Var nodes for simple equi-joins
        if (*arg0).type_ != pg_sys::NodeTag::T_Var || (*arg1).type_ != pg_sys::NodeTag::T_Var {
            continue;
        }

        let var0 = arg0 as *mut pg_sys::Var;
        let var1 = arg1 as *mut pg_sys::Var;

        let varno0 = (*var0).varno as pg_sys::Index;
        let varno1 = (*var1).varno as pg_sys::Index;
        let attno0 = (*var0).varattno;
        let attno1 = (*var1).varattno;

        // Check if this is an equi-join between outer and inner
        if varno0 == outer_rti && varno1 == inner_rti {
            keys.push((attno0, attno1));
        } else if varno0 == inner_rti && varno1 == outer_rti {
            keys.push((attno1, attno0));
        }
    }

    keys
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

    // Get the RTE for this relation
    let rtable = (*(*root).parse).rtable;
    if rtable.is_null() {
        return None;
    }

    let rte = pg_sys::rt_fetch(rti, rtable);
    if rte.is_null() {
        return None;
    }

    // We only support plain relations
    if (*rte).rtekind != pg_sys::RTEKind::RTE_RELATION {
        return None;
    }

    let relid = (*rte).relid;
    let relkind = pg_sys::get_rel_relkind(relid) as u8;
    if relkind != pg_sys::RELKIND_RELATION && relkind != pg_sys::RELKIND_MATVIEW {
        return None;
    }

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

            // For M1, we only handle INNER JOINs
            if jointype != pg_sys::JoinType::JOIN_INNER {
                return None;
            }

            // Check if there's a LIMIT in the query
            let limit = if (*root).limit_tuples > -1.0 {
                Some((*root).limit_tuples as usize)
            } else {
                None
            };

            // For M1, we require a LIMIT for Single Feature joins
            // (Join-level predicates for Aggregate Score joins are deferred to M3)
            if limit.is_none() {
                return None;
            }

            // Extract information from both sides of the join
            let outer_side = extract_join_side_info(root, args.outerrel)?;
            let inner_side = extract_join_side_info(root, args.innerrel)?;

            // Extract join keys from the restrict list
            let outer_rti = outer_side.heap_rti.unwrap_or(0);
            let inner_rti = inner_side.heap_rti.unwrap_or(0);
            let join_key_pairs = extract_join_keys(args.extra, outer_rti, inner_rti);

            // Build the join clause with join keys
            let mut join_clause = JoinCSClause::new()
                .with_outer_side(outer_side)
                .with_inner_side(inner_side)
                .with_join_type(SerializableJoinType::from(jointype))
                .with_limit(limit);

            // Add extracted join keys
            for (outer_attno, inner_attno) in join_key_pairs {
                join_clause = join_clause.add_join_key(outer_attno, inner_attno);
            }

            // Check if this is a valid join for M1
            // We need at least one side with a BM25 index AND a search predicate
            if !join_clause.has_driving_side() {
                return None;
            }

            // Create the private data
            let private_data = PrivateData::new(join_clause);

            // Build the CustomPath
            // For now, use simple cost estimates (will be improved later)
            let startup_cost = DEFAULT_STARTUP_COST;
            let total_cost = startup_cost + 1000.0; // Arbitrary cost for now

            // Force the path to be chosen when we have a valid join opportunity
            let builder = builder
                .set_flag(Flags::Force)
                .set_startup_cost(startup_cost)
                .set_total_cost(total_cost)
                .set_rows(limit.unwrap_or(1000) as f64);

            Some(builder.build(private_data))
        }
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
        // For joins, scanrelid must be 0 (it's not scanning a single relation)
        builder.set_scanrelid(0);

        let mut node = builder.build();

        // For joins, we need to set custom_scan_tlist to describe the output columns.
        // Copy the target list to custom_scan_tlist so PostgreSQL knows what columns we produce.
        node.custom_scan_tlist = node.scan.plan.targetlist;

        node
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        // Transfer join clause to scan state
        builder.custom_state().join_clause = builder.custom_private().join_clause.clone();
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
            explainer.add_text("Outer RTI", &rti.to_string());
        }
        if join_clause.outer_side.has_search_predicate {
            if let Some(ref query) = join_clause.outer_side.query {
                explainer.add_query(query);
            }
        }

        // Show inner side info
        if let Some(rti) = join_clause.inner_side.heap_rti {
            explainer.add_text("Inner RTI", &rti.to_string());
        }
        if join_clause.inner_side.has_search_predicate {
            if let Some(ref query) = join_clause.inner_side.query {
                explainer.add_query(query);
            }
        }

        // Show limit if present
        if let Some(limit) = join_clause.limit {
            explainer.add_text("Limit", &limit.to_string());
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

            // Create result tuple slot (matches the custom scan's output descriptor)
            let result_slot = pg_sys::MakeTupleTableSlot(
                state.csstate.ss.ps.ps_ResultTupleDesc,
                &pg_sys::TTSOpsVirtual,
            );
            state.custom_state_mut().result_slot = Some(result_slot);

            // Open relations for the driving side (side with search predicate)
            if let Some(heaprelid) = driving_side.heaprelid {
                let heaprel = PgSearchRelation::open(heaprelid);

                // Create visibility checker for driving side
                let vis_checker = VisibilityChecker::with_rel_and_snap(&heaprel, snapshot);
                state.custom_state_mut().driving_visibility_checker = Some(vis_checker);

                // Create a slot for fetching driving tuples
                let driving_slot =
                    pg_sys::MakeTupleTableSlot(heaprel.rd_att, &pg_sys::TTSOpsBufferHeapTuple);
                state.custom_state_mut().driving_fetch_slot = Some(driving_slot);

                // Open search reader for driving side (it must have a search predicate)
                if let (Some(indexrelid), Some(ref query)) = (
                    driving_side.indexrelid,
                    &driving_side.query,
                ) {
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
                let scan_desc = pg_sys::table_beginscan(heaprel.as_ptr(), snapshot, 0, std::ptr::null_mut());
                state.custom_state_mut().build_scan_desc = Some(scan_desc);

                state.custom_state_mut().build_heaprel = Some(heaprel);
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

            // Phase 2: Probe hash table with driving side search results
            loop {
                // If we have pending matches, return one
                if let Some(build_ctid) = state.custom_state_mut().pending_build_ctids.pop_front() {
                    if let Some(slot) = Self::build_result_tuple(state, build_ctid) {
                        state.custom_state_mut().rows_returned += 1;
                        return slot;
                    }
                    // Visibility check failed, try next match
                    continue;
                }

                // Initialize search results if not done yet
                let need_init = state.custom_state().driving_search_results.is_none();
                if need_init {
                    let reader = state.custom_state().driving_search_reader.as_ref();
                    if let Some(reader) = reader {
                        let results = reader.search();
                        state.custom_state_mut().driving_search_results = Some(results);
                    } else {
                        return std::ptr::null_mut(); // No search reader - EOF
                    }
                }

                // Get next driving row from search results
                let driving_search_results = state.custom_state_mut().driving_search_results.as_mut();
                let Some(results) = driving_search_results else {
                    return std::ptr::null_mut(); // No search results - EOF
                };

                let next_result = results.next();
                let Some((scored, _doc_address)) = next_result else {
                    return std::ptr::null_mut(); // No more driving rows - EOF
                };

                let driving_ctid = scored.ctid;
                let driving_score = scored.bm25;
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

            // Drop tuple slots
            if let Some(slot) = state.custom_state().build_scan_slot {
                pg_sys::ExecDropSingleTupleTableSlot(slot);
            }
            if let Some(slot) = state.custom_state().driving_fetch_slot {
                pg_sys::ExecDropSingleTupleTableSlot(slot);
            }
            if let Some(slot) = state.custom_state().result_slot {
                pg_sys::ExecDropSingleTupleTableSlot(slot);
            }
        }

        // Clean up resources
        state.custom_state_mut().driving_heaprel = None;
        state.custom_state_mut().driving_indexrel = None;
        state.custom_state_mut().driving_search_reader = None;
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
        let join_clause = state.custom_state().join_clause.clone();
        let driving_is_outer = state.custom_state().driving_is_outer;
        
        // Get the build side join key attribute number (first join key)
        // If driving is outer, build is inner, so use inner_attno
        // If driving is inner, build is outer, so use outer_attno
        let build_key_attno = join_clause
            .join_keys
            .first()
            .map(|jk| {
                if driving_is_outer {
                    jk.inner_attno as i32
                } else {
                    jk.outer_attno as i32
                }
            })
            .unwrap_or(1);
        
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

            // Extract the join key value
            let mut is_null = false;
            let datum = pg_sys::slot_getattr(slot, build_key_attno, &mut is_null);

            if is_null {
                continue; // Skip NULL join keys
            }

            // Convert to i64 (assuming integer join keys for M1)
            let key_value = datum.value() as i64;

            // Add to hash table
            state
                .custom_state_mut()
                .hash_table
                .entry(key_value)
                .or_default()
                .push(InnerRow { ctid });
        }
    }

    /// Extract the join key from the driving tuple.
    unsafe fn extract_driving_join_key(
        state: &mut CustomScanStateWrapper<Self>,
        driving_ctid: u64,
    ) -> Option<i64> {
        let join_clause = state.custom_state().join_clause.clone();
        let driving_is_outer = state.custom_state().driving_is_outer;
        
        // Get the driving side join key attribute number (first join key)
        // If driving is outer, use outer_attno
        // If driving is inner, use inner_attno
        let driving_key_attno = join_clause
            .join_keys
            .first()
            .map(|jk| {
                if driving_is_outer {
                    jk.outer_attno as i32
                } else {
                    jk.inner_attno as i32
                }
            })
            .unwrap_or(1);

        // Get the driving slot first (immutable borrow)
        let driving_slot = state.custom_state().driving_fetch_slot?;

        // Then get the visibility checker (mutable borrow)
        let vis_checker = state.custom_state_mut().driving_visibility_checker.as_mut()?;

        // Fetch the driving tuple and check visibility
        vis_checker.exec_if_visible(driving_ctid, driving_slot, |_rel| {
            // Extract the join key value
            let mut is_null = false;
            let datum = pg_sys::slot_getattr(driving_slot, driving_key_attno, &mut is_null);

            if is_null {
                None
            } else {
                Some(datum.value() as i64)
            }
        })?
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

        // Fetch driving tuple (should still be valid from extract_driving_join_key)
        let driving_vis = state.custom_state_mut().driving_visibility_checker.as_mut()?;
        if driving_vis.exec_if_visible(driving_ctid, driving_slot, |_| ()).is_none() {
            return None;
        }

        // Fetch build tuple
        let build_vis = state.custom_state_mut().build_visibility_checker.as_mut()?;
        if build_vis.exec_if_visible(build_ctid, build_slot, |_| ()).is_none() {
            return None;
        }

        // Get the result tuple descriptor
        let result_tupdesc = state.csstate.ss.ps.ps_ResultTupleDesc;
        let natts = (*result_tupdesc).natts as usize;

        // Clear the result slot
        pg_sys::ExecClearTuple(result_slot);

        // Get target list from the custom scan
        let tlist = (*state.csstate.ss.ps.plan).targetlist;
        let target_entries = PgList::<pg_sys::TargetEntry>::from_pg(tlist);

        // Make sure slots have all attributes deformed
        pg_sys::slot_getallattrs(driving_slot);
        pg_sys::slot_getallattrs(build_slot);

        // Get the join clause to determine RTIs
        let join_clause = &state.custom_state().join_clause;
        let outer_rti = join_clause.outer_side.heap_rti.unwrap_or(0);
        let inner_rti = join_clause.inner_side.heap_rti.unwrap_or(0);

        // Map RTI to slot:
        // - If driving is outer: outer_rti -> driving_slot, inner_rti -> build_slot
        // - If driving is inner: inner_rti -> driving_slot, outer_rti -> build_slot
        let (driving_rti, build_rti) = if driving_is_outer {
            (outer_rti, inner_rti)
        } else {
            (inner_rti, outer_rti)
        };

        // Fill the result slot based on the target list
        let datums = (*result_slot).tts_values;
        let nulls = (*result_slot).tts_isnull;

        for (i, te) in target_entries.iter_ptr().enumerate() {
            if i >= natts {
                break;
            }

            let expr = (*te).expr;
            if (*expr).type_ == pg_sys::NodeTag::T_Var {
                let var = expr as *mut pg_sys::Var;
                let varno = (*var).varno as pg_sys::Index;
                let varattno = (*var).varattno;

                // Determine which slot to read from based on varno
                let (source_slot, attno) = if varno == driving_rti {
                    (driving_slot, varattno)
                } else if varno == build_rti {
                    (build_slot, varattno)
                } else {
                    // Unknown varno - set null
                    *nulls.add(i) = true;
                    continue;
                };

                // Get the attribute value from the source slot
                if attno <= 0 {
                    // System attribute or whole-row reference - not supported yet
                    *nulls.add(i) = true;
                    continue;
                }

                let source_natts = (*(*source_slot).tts_tupleDescriptor).natts as i16;
                if attno > source_natts {
                    *nulls.add(i) = true;
                    continue;
                }

                let mut is_null = false;
                let value = pg_sys::slot_getattr(source_slot, attno as i32, &mut is_null);
                *datums.add(i) = value;
                *nulls.add(i) = is_null;
            } else {
                // Non-Var expression - not supported yet
                *nulls.add(i) = true;
            }
        }

        // Mark slot as containing a virtual tuple
        (*result_slot).tts_flags &= !(pg_sys::TTS_FLAG_EMPTY as u16);
        (*result_slot).tts_nvalid = natts as i16;

        Some(result_slot)
    }
}

impl ExecMethod for JoinScan {
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        <JoinScan as PlainExecCapable>::exec_methods()
    }
}

impl PlainExecCapable for JoinScan {}
