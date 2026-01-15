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

/// Helper to extract Var nodes from an expression and add them to the target list
/// if not already present. This ensures the custom_scan_tlist includes all columns
/// needed by join conditions.
unsafe fn add_vars_to_tlist(expr: *mut pg_sys::Node, tlist: &mut PgList<pg_sys::TargetEntry>) {
    if expr.is_null() {
        return;
    }

    match (*expr).type_ {
        pg_sys::NodeTag::T_Var => {
            let var = expr as *mut pg_sys::Var;
            let varno = (*var).varno;
            let varattno = (*var).varattno;

            // Check if this Var is already in the tlist
            let mut found = false;
            for te in tlist.iter_ptr() {
                if (*(*te).expr).type_ == pg_sys::NodeTag::T_Var {
                    let te_var = (*te).expr as *mut pg_sys::Var;
                    if (*te_var).varno == varno && (*te_var).varattno == varattno {
                        found = true;
                        break;
                    }
                }
            }

            // Add to tlist if not found
            if !found {
                let resno = (tlist.len() + 1) as i16;
                let te = pg_sys::makeTargetEntry(
                    expr as *mut pg_sys::Expr,
                    resno,
                    std::ptr::null_mut(), // no resname needed
                    false,                // not resjunk
                );
                tlist.push(te);
            }
        }
        pg_sys::NodeTag::T_OpExpr => {
            let opexpr = expr as *mut pg_sys::OpExpr;
            let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
            for arg in args.iter_ptr() {
                add_vars_to_tlist(arg, tlist);
            }
        }
        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = expr as *mut pg_sys::BoolExpr;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            for arg in args.iter_ptr() {
                add_vars_to_tlist(arg, tlist);
            }
        }
        pg_sys::NodeTag::T_FuncExpr => {
            let funcexpr = expr as *mut pg_sys::FuncExpr;
            let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
            for arg in args.iter_ptr() {
                add_vars_to_tlist(arg, tlist);
            }
        }
        _ => {
            // For other node types, we could use expression_tree_walker
            // but for simplicity, we handle common cases above
        }
    }
}

/// Result of extracting join conditions from the restrict list.
struct JoinConditions {
    /// Equi-join keys: (outer_attno, inner_attno) pairs for hash join.
    equi_keys: Vec<(pg_sys::AttrNumber, pg_sys::AttrNumber)>,
    /// Other join conditions (non-equijoin) that need to be evaluated after hash lookup.
    /// These are the RestrictInfo nodes themselves.
    other_conditions: Vec<*mut pg_sys::RestrictInfo>,
    /// Whether any join-level condition contains our @@@ operator.
    has_search_predicate: bool,
}

/// Check if an expression contains our @@@ operator recursively.
unsafe fn contains_search_operator(expr: *mut pg_sys::Node) -> bool {
    if expr.is_null() {
        return false;
    }

    let our_opoid = anyelement_query_input_opoid();

    match (*expr).type_ {
        pg_sys::NodeTag::T_OpExpr => {
            let opexpr = expr as *mut pg_sys::OpExpr;
            // Check if this is our @@@ operator
            if (*opexpr).opno == our_opoid {
                return true;
            }
            // Recurse into arguments
            let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
            for arg in args.iter_ptr() {
                if contains_search_operator(arg) {
                    return true;
                }
            }
            false
        }
        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = expr as *mut pg_sys::BoolExpr;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            for arg in args.iter_ptr() {
                if contains_search_operator(arg) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

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
        if contains_search_operator(clause.cast()) {
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
                        result.equi_keys.push((attno0, attno1));
                        is_equi_join = true;
                    } else if varno0 == inner_rti && varno1 == outer_rti {
                        result.equi_keys.push((attno1, attno0));
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

            // Extract join conditions from the restrict list
            let outer_rti = outer_side.heap_rti.unwrap_or(0);
            let inner_rti = inner_side.heap_rti.unwrap_or(0);
            let join_conditions = extract_join_conditions(args.extra, outer_rti, inner_rti);

            // Build the join clause with join keys
            let mut join_clause = JoinCSClause::new()
                .with_outer_side(outer_side)
                .with_inner_side(inner_side)
                .with_join_type(SerializableJoinType::from(jointype))
                .with_limit(limit)
                .with_has_other_conditions(!join_conditions.other_conditions.is_empty())
                .with_has_join_level_search_predicate(join_conditions.has_search_predicate);

            // Add extracted equi-join keys
            for (outer_attno, inner_attno) in join_conditions.equi_keys {
                join_clause = join_clause.add_join_key(outer_attno, inner_attno);
            }

            // Check if this is a valid join for M1
            // We need at least one side with a BM25 index AND a search predicate,
            // OR a join-level search predicate (like OR across tables)
            let has_side_predicate = join_clause.has_driving_side();
            let has_join_level_predicate = join_conditions.has_search_predicate;

            if !has_side_predicate && !has_join_level_predicate {
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

        // Get the clauses from the builder args (these are join conditions)
        let clauses = builder.args().clauses;

        let mut node = builder.build();

        unsafe {
            // For joins, we need to set custom_scan_tlist to describe the output columns.
            // Start with the plan's target list
            let mut tlist = PgList::<pg_sys::TargetEntry>::from_pg(node.scan.plan.targetlist);

            // Extract clauses and collect Vars from join conditions
            if !clauses.is_null() {
                let restrict_infos = PgList::<pg_sys::RestrictInfo>::from_pg(clauses);
                let mut expr_list = PgList::<pg_sys::Expr>::new();

                for ri in restrict_infos.iter_ptr() {
                    if ri.is_null() || (*ri).clause.is_null() {
                        continue;
                    }

                    expr_list.push((*ri).clause);

                    // Extract Vars from this clause and add to tlist if not already present
                    add_vars_to_tlist((*ri).clause.cast(), &mut tlist);
                }

                node.custom_exprs = expr_list.into_pg().cast();
            }

            // Set custom_scan_tlist with all needed columns
            node.custom_scan_tlist = tlist.into_pg();
        }

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

            // Compile join qual expressions from custom_exprs
            // These are the join conditions that need to be evaluated after building tuples
            let cscan = state.csstate.ss.ps.plan as *mut pg_sys::CustomScan;
            let custom_exprs = (*cscan).custom_exprs;

            if !custom_exprs.is_null() {
                // Create an expression context for evaluating join quals
                let econtext = pg_sys::CreateExprContext(estate);
                state.custom_state_mut().join_qual_econtext = Some(econtext);

                // Compile the expression list into an ExprState
                // ExecInitQual expects a List of Expr nodes and returns a combined ExprState
                let qual_state =
                    pg_sys::ExecInitQual(custom_exprs.cast(), &mut state.csstate.ss.ps);
                if !qual_state.is_null() {
                    state.custom_state_mut().join_qual_state = Some(qual_state);
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

            // Phase 2: Probe hash table with driving side search results
            loop {
                // If we have pending matches, return one
                if let Some(build_ctid) = state.custom_state_mut().pending_build_ctids.pop_front() {
                    if let Some(slot) = Self::build_result_tuple(state, build_ctid) {
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
        state.custom_state_mut().driving_scan_desc = None;
        state.custom_state_mut().build_heaprel = None;
        state.custom_state_mut().build_scan_desc = None;
        state.custom_state_mut().build_scan_slot = None;
        state.custom_state_mut().driving_fetch_slot = None;
        state.custom_state_mut().result_slot = None;
    }
}

/// Magic key used for cross-join (when there are no equi-join keys)
const CROSS_JOIN_KEY: i64 = 0;

impl JoinScan {
    /// Build the hash table from the build side by scanning the heap.
    unsafe fn build_hash_table(state: &mut CustomScanStateWrapper<Self>) {
        let join_clause = state.custom_state().join_clause.clone();
        let driving_is_outer = state.custom_state().driving_is_outer;

        // Get the build side join key attribute number (first join key)
        // If driving is outer, build is inner, so use inner_attno
        // If driving is inner, build is outer, so use outer_attno
        // If there are no equi-join keys, we'll use CROSS_JOIN_KEY for all tuples
        let build_key_attno = join_clause.join_keys.first().map(|jk| {
            if driving_is_outer {
                jk.inner_attno as i32
            } else {
                jk.outer_attno as i32
            }
        });
        let has_equi_join_keys = build_key_attno.is_some();

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

            // Determine the hash key
            let key_value = if let Some(attno) = build_key_attno {
                // We have an equi-join key, extract its value
                let mut is_null = false;
                let datum = pg_sys::slot_getattr(slot, attno, &mut is_null);

                if is_null {
                    continue; // Skip NULL join keys for equi-joins
                }

                // Convert to i64 (assuming integer join keys for M1)
                datum.value() as i64
            } else {
                // No equi-join keys - this is a cross join, use magic key
                CROSS_JOIN_KEY
            };

            // Add to hash table
            state
                .custom_state_mut()
                .hash_table
                .entry(key_value)
                .or_default()
                .push(InnerRow { ctid });
        }

        // Store whether we're doing a cross join for later use
        state.custom_state_mut().is_cross_join = !has_equi_join_keys;
    }

    /// Extract the join key from the driving tuple.
    /// For cross joins (no equi-join keys), returns CROSS_JOIN_KEY.
    unsafe fn extract_driving_join_key(
        state: &mut CustomScanStateWrapper<Self>,
        driving_ctid: u64,
    ) -> Option<i64> {
        let driving_slot = state.custom_state().driving_fetch_slot?;
        let uses_heap_scan = state.custom_state().driving_uses_heap_scan;
        let is_cross_join = state.custom_state().is_cross_join;

        // For cross joins, just return the magic key
        if is_cross_join {
            if uses_heap_scan {
                // Tuple already in slot from heap scan, no visibility check needed
                return Some(CROSS_JOIN_KEY);
            } else {
                // Fetch tuple by ctid and verify visibility
                let vis_checker = state
                    .custom_state_mut()
                    .driving_visibility_checker
                    .as_mut()?;
                return vis_checker
                    .exec_if_visible(driving_ctid, driving_slot, |_rel| Some(CROSS_JOIN_KEY))?;
            }
        }

        // For equi-joins, extract the actual join key
        let join_clause = state.custom_state().join_clause.clone();
        let driving_is_outer = state.custom_state().driving_is_outer;

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

        if uses_heap_scan {
            // Tuple already in slot from heap scan
            let mut is_null = false;
            let datum = pg_sys::slot_getattr(driving_slot, driving_key_attno, &mut is_null);
            if is_null {
                None
            } else {
                Some(datum.value() as i64)
            }
        } else {
            // Fetch tuple by ctid and verify visibility
            let vis_checker = state
                .custom_state_mut()
                .driving_visibility_checker
                .as_mut()?;

            vis_checker.exec_if_visible(driving_ctid, driving_slot, |_rel| {
                let mut is_null = false;
                let datum = pg_sys::slot_getattr(driving_slot, driving_key_attno, &mut is_null);
                if is_null {
                    None
                } else {
                    Some(datum.value() as i64)
                }
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
            if driving_vis
                .exec_if_visible(driving_ctid, driving_slot, |_| ())
                .is_none()
            {
                return None;
            }
        }

        // Fetch build tuple
        let build_vis = state.custom_state_mut().build_visibility_checker.as_mut()?;
        if build_vis
            .exec_if_visible(build_ctid, build_slot, |_| ())
            .is_none()
        {
            return None;
        }

        // Get the result tuple descriptor
        let result_tupdesc = state.csstate.ss.ps.ps_ResultTupleDesc;
        let natts = (*result_tupdesc).natts as usize;

        // Clear the result slot
        pg_sys::ExecClearTuple(result_slot);

        // For custom scans with custom_scan_tlist, the plan.targetlist has Vars with
        // varno=INDEX_VAR (-3) that reference positions in custom_scan_tlist.
        // We need to read from custom_scan_tlist to get the original Var references.
        let cscan = state.csstate.ss.ps.plan as *mut pg_sys::CustomScan;
        let custom_scan_tlist = (*cscan).custom_scan_tlist;
        let tlist = if !custom_scan_tlist.is_null() {
            custom_scan_tlist
        } else {
            (*state.csstate.ss.ps.plan).targetlist
        };
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
                let source_slot = if varno == driving_rti {
                    driving_slot
                } else if varno == build_rti {
                    build_slot
                } else {
                    // Unknown varno - set null
                    *nulls.add(i) = true;
                    continue;
                };

                // Get the attribute value from the source slot
                if varattno <= 0 {
                    // System attribute or whole-row reference - not supported yet
                    *nulls.add(i) = true;
                    continue;
                }

                let source_natts = (*(*source_slot).tts_tupleDescriptor).natts as i16;
                if varattno > source_natts {
                    *nulls.add(i) = true;
                    continue;
                }

                let mut is_null = false;
                let value = pg_sys::slot_getattr(source_slot, varattno as i32, &mut is_null);
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
