// Copyright (c) 2023-2025 ParadeDB, Inc.
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

//! WindowScan - Custom scan for window functions
//!
//! This module handles window functions (e.g., `COUNT(*) OVER ()`, `paradedb.agg(...) OVER ()`)
//! by creating a custom path at UPPERREL_WINDOW stage and replacing WindowFunc nodes with
//! placeholders at planning time.
//!
//! This approach mirrors AggregateScan's pattern of:
//! 1. Creating a custom path at create_upper_paths_hook
//! 2. Replacing problematic nodes (Aggref/WindowFunc) in plan_custom_path
//! 3. Handling execution in the custom scan state

use crate::api::AsCStr;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::pdbscan::projections::window_agg;
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::customscan::{
    range_table, CreateUpperPathsHookArgs, CustomScan, CustomScanState, ExecMethod,
    PlainExecCapable,
};

use pgrx::pg_sys::AsPgCStr;
use pgrx::{pg_sys, IntoDatum, PgList};
use std::ffi::CStr;
use std::ptr::addr_of_mut;

/// Check if a query has window functions
unsafe fn query_has_window_functions(parse: *mut pg_sys::Query) -> bool {
    if parse.is_null() || (*parse).targetList.is_null() {
        return false;
    }

    let target_list = PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList);
    for te in target_list.iter_ptr() {
        if !(*te).expr.is_null() && (*(*te).expr).type_ == pg_sys::NodeTag::T_WindowFunc {
            return true;
        }
    }
    false
}

/// Check if a query has the search operator (@@@)
unsafe fn query_has_search_operator(parse: *mut pg_sys::Query) -> bool {
    // TODO: Implement proper operator detection
    // For now, assume true if we have window functions
    true
}

#[derive(Default)]
pub struct WindowScan;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowPrivateData {
    pub window_aggregates: Vec<window_agg::WindowAggregateInfo>,
    pub scanrelid: pg_sys::Index,
    pub heaprelid: pg_sys::Oid,
    pub indexrelid: pg_sys::Oid,
    pub query: crate::query::SearchQueryInput,
    pub limit: Option<usize>,
    pub orderby_info: Option<Vec<crate::api::OrderByInfo>>,
}

impl From<*mut pg_sys::List> for WindowPrivateData {
    fn from(list: *mut pg_sys::List) -> Self {
        unsafe {
            let list = PgList::<pg_sys::Node>::from_pg(list);
            let node = list.get_ptr(0).unwrap();
            let content = node
                .as_c_str()
                .unwrap()
                .to_str()
                .expect("string node should be valid utf8");
            serde_json::from_str(content).unwrap()
        }
    }
}

impl From<WindowPrivateData> for *mut pg_sys::List {
    fn from(value: WindowPrivateData) -> Self {
        let content = serde_json::to_string(&value).unwrap();
        unsafe {
            let mut ser = PgList::new();
            ser.push(pg_sys::makeString(content.as_pg_cstr()).cast::<pg_sys::Node>());
            ser.into_pg()
        }
    }
}

impl CustomScan for WindowScan {
    const NAME: &'static CStr = c"ParadeDB Window Scan";
    type Args = CreateUpperPathsHookArgs;
    type State = WindowScanState;
    type PrivateData = WindowPrivateData;

    fn create_custom_path(builder: CustomPathBuilder<Self>) -> Option<pg_sys::CustomPath> {
        unsafe {
            pgrx::warning!("WindowScan::create_custom_path called at UPPERREL_WINDOW stage");

            // Get root pointer directly from args to avoid borrow issues
            let root_ptr = builder.args().root as *const _ as *mut pg_sys::PlannerInfo;
            let parse = (*root_ptr).parse;

            pgrx::warning!("  parse.is_null() = {}", parse.is_null());

            // Check if query has window functions and search operator
            if parse.is_null() {
                pgrx::warning!("  Returning None: parse is null");
                return None;
            }

            let has_window_funcs = query_has_window_functions(parse);
            let has_search_op = query_has_search_operator(parse);
            pgrx::warning!("  has_window_functions = {}", has_window_funcs);
            pgrx::warning!("  has_search_operator = {}", has_search_op);

            if !has_window_funcs || !has_search_op {
                pgrx::warning!("  Returning None: missing window functions or search operator");
                return None;
            }

            // Extract window function specifications
            let window_specs_map = window_agg::extract_window_specifications(parse);
            pgrx::warning!("  window_specs_map.len() = {}", window_specs_map.len());
            if window_specs_map.is_empty() {
                pgrx::warning!("  Returning None: window_specs_map is empty");
                return None;
            }

            // Convert HashMap to Vec of WindowAggregateInfo
            let window_aggregates: Vec<window_agg::WindowAggregateInfo> = window_specs_map
                .into_iter()
                .map(
                    |(target_entry_index, agg_spec)| window_agg::WindowAggregateInfo {
                        target_entry_index,
                        agg_spec,
                    },
                )
                .collect();

            // Get the scanrelid from the input relation
            let input_rel = builder.args().input_rel();
            pgrx::warning!("  input_rel.reloptkind = {:?}", (*input_rel).reloptkind);
            pgrx::warning!(
                "  input_rel.cheapest_total_path.is_null() = {}",
                (*input_rel).cheapest_total_path.is_null()
            );

            if (*input_rel).cheapest_total_path.is_null() {
                pgrx::warning!("  Returning None: cheapest_total_path is null");
                return None;
            }

            // Extract scanrelid from input_rel - should be a single base relation
            let parent_relids = (*input_rel).relids;
            let scanrelid = range_table::bms_exactly_one_member(parent_relids);
            pgrx::warning!("  scanrelid = {:?}", scanrelid);
            if scanrelid.is_none() {
                pgrx::warning!("  Returning None: could not extract scanrelid");
                return None;
            }
            let scanrelid = scanrelid.unwrap();

            let input_path = (*input_rel).cheapest_total_path;
            let path_type = (*input_path).type_;
            pgrx::warning!("  input_path type = {:?}", path_type);

            // Extract necessary information from the input path
            let (heaprelid, indexrelid, query) = if path_type == pg_sys::NodeTag::T_CustomPath {
                pgrx::warning!("  Input path is a CustomPath - extracting info");
                let custom_path = input_path as *mut pg_sys::CustomPath;
                let pdbscan_privdata =
                    crate::postgres::customscan::pdbscan::privdat::PrivateData::from(
                        (*custom_path).custom_private,
                    );

                let heaprelid = pdbscan_privdata
                    .heaprelid()
                    .expect("heaprelid should exist");
                let indexrelid = pdbscan_privdata
                    .indexrelid()
                    .expect("indexrelid should exist");
                let query = pdbscan_privdata
                    .query()
                    .clone()
                    .unwrap_or(crate::query::SearchQueryInput::All);

                (heaprelid, indexrelid, query)
            } else {
                // Not a custom path, return None - we can only handle PdbScan input
                pgrx::warning!(
                    "  Returning None: input path is not a CustomPath (type = {:?})",
                    path_type
                );
                return None;
            };

            // Extract ORDER BY and LIMIT information from the query
            // This is needed for TopN execution with window aggregates
            let (limit, orderby_info) = extract_orderby_and_limit(parse, root_ptr);
            pgrx::warning!(
                "  limit = {:?}, orderby_info = {:?}",
                limit,
                orderby_info.is_some()
            );

            let mut custom_path = builder.build(WindowPrivateData {
                window_aggregates,
                scanrelid,
                heaprelid,
                indexrelid,
                query,
                limit,
                orderby_info,
            });

            // Copy costs from input path - window aggregates computed in same pass
            custom_path.path.startup_cost = (*input_path).startup_cost;
            custom_path.path.total_cost = (*input_path).total_cost;

            // Set pathkeys to indicate we can produce sorted output
            // This is critical - without pathkeys, PostgreSQL will add a Sort node
            if !(*parse).sortClause.is_null() {
                custom_path.path.pathkeys = (*root_ptr).query_pathkeys;
            }

            pgrx::warning!("  WindowScan::create_custom_path returning Some(custom_path)");
            Some(custom_path)
        }
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
        // Set scanrelid - WindowScan scans the base relation directly (like AggregateScan)
        // This tells PostgreSQL which relation we're scanning
        builder.set_scanrelid(builder.custom_private().scanrelid);

        unsafe {
            // Get root and parse pointers before consuming builder
            let root = builder.args().root;
            let parse = (*root).parse;

            let mut cscan = builder.build();
            let plan = &mut cscan.scan.plan;

            // At UPPERREL_WINDOW stage, PostgreSQL doesn't build the target list for us
            // We need to extract it from the parse tree and build it ourselves
            if (*plan).targetlist.is_null() {
                if !parse.is_null() && !(*parse).targetList.is_null() {
                    // Copy the target list from the parse tree
                    // This includes all columns plus the window functions
                    (*plan).targetlist = (*parse).targetList;
                }
            }

            // Replace WindowFunc nodes with window_func() placeholders
            // This is similar to how AggregateScan replaces Aggref nodes
            replace_windowfuncs_in_target_list(plan);

            cscan
        }
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        pgrx::warning!("WindowScan::create_custom_scan_state called");
        unsafe {
            // Extract fields from our private data
            let privdata = builder.custom_private();
            let window_aggregates = privdata.window_aggregates.clone();
            let heaprelid = privdata.heaprelid;
            let indexrelid = privdata.indexrelid;
            let query = privdata.query.clone();
            let limit = privdata.limit;
            let orderby_info = privdata.orderby_info.clone();

            builder.custom_state().window_aggregates = window_aggregates.clone();

            // Create and initialize PdbScanState
            let mut pdbscan_state =
                Box::new(crate::postgres::customscan::pdbscan::scan_state::PdbScanState::default());

            pdbscan_state.heaprelid = heaprelid;
            pdbscan_state.indexrelid = indexrelid;
            pdbscan_state.open_relations(pg_sys::AccessShareLock as _);
            pdbscan_state.execution_rti = (*builder.args().cscan).scan.scanrelid as pg_sys::Index;

            // Set exec method type to TopN for window aggregates
            pdbscan_state.exec_method_type =
                crate::postgres::customscan::builders::custom_path::ExecMethodType::TopN {
                    heaprelid,
                    limit: limit.unwrap_or(usize::MAX),
                    orderby_info,
                    window_aggregates: window_aggregates.clone(),
                };

            pdbscan_state.targetlist_len = builder.target_list().len();
            pdbscan_state.window_aggregates = window_aggregates;
            pdbscan_state.set_base_search_query_input(query);

            builder.custom_state().pdbscan_state = Some(pdbscan_state);
            builder.build()
        }
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pg_sys::List,
        explainer: &mut crate::postgres::customscan::explainer::Explainer,
    ) {
        explainer.add_text(
            "Window Aggregates",
            format!(
                "{} window aggregate(s)",
                state.custom_state().window_aggregates.len()
            ),
        );

        // TODO: Add more explain details (search query, etc.)
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        pgrx::warning!("WindowScan::begin_custom_scan called");
        unsafe {
            // Get execution RTI and lockmode first
            let execution_rti = state
                .custom_state()
                .pdbscan_state
                .as_ref()
                .expect("pdbscan_state should be initialized")
                .execution_rti;
            pgrx::warning!("  execution_rti = {}", execution_rti);

            let rte = pg_sys::exec_rt_fetch(execution_rti, estate);
            assert!(!rte.is_null());
            let lockmode = (*rte).rellockmode as pg_sys::LOCKMODE;

            // Open relations
            state
                .custom_state_mut()
                .pdbscan_state
                .as_mut()
                .unwrap()
                .open_relations(lockmode);

            if eflags & (pg_sys::EXEC_FLAG_EXPLAIN_ONLY as i32) != 0 {
                return;
            }

            // Setup MVCC checking and heap fetching
            {
                let pdbscan_state = state.custom_state_mut().pdbscan_state.as_mut().unwrap();
                pdbscan_state.visibility_checker =
                    Some(crate::postgres::heap::VisibilityChecker::with_rel_and_snap(
                        pdbscan_state.heaprel(),
                        pg_sys::GetActiveSnapshot(),
                    ));
                pdbscan_state.doc_from_heap_state = Some(
                    crate::postgres::heap::HeapFetchState::new(pdbscan_state.heaprel()),
                );
            }

            // Initialize scan tuple slot
            let tupdesc = state
                .custom_state()
                .pdbscan_state
                .as_ref()
                .unwrap()
                .heaptupdesc();
            let heaprel_ptr = state
                .custom_state()
                .pdbscan_state
                .as_ref()
                .unwrap()
                .heaprel()
                .as_ptr();

            pg_sys::ExecInitScanTupleSlot(
                estate,
                addr_of_mut!(state.csstate.ss),
                tupdesc,
                pg_sys::table_slot_callbacks(heaprel_ptr),
            );
            pg_sys::ExecInitResultTypeTL(addr_of_mut!(state.csstate.ss.ps));

            let planstate = state.planstate();
            let tupdesc2 = (*state.csstate.ss.ss_ScanTupleSlot).tts_tupleDescriptor;
            pg_sys::ExecAssignProjectionInfo(planstate, tupdesc2);

            state
                .custom_state_mut()
                .pdbscan_state
                .as_mut()
                .unwrap()
                .init_expr_context(estate, planstate);
            state.runtime_context = state.csstate.ss.ps.ps_ExprContext;
        }
    }

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        // Initialize search reader and reset state (same as PdbScan::rescan_custom_scan)
        Self::init_search_reader(state);
        if let Some(pdbscan_state) = state.custom_state_mut().pdbscan_state.as_mut() {
            pdbscan_state.reset();
        }
    }

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        // Initialize search reader if needed
        let needs_init = state
            .custom_state()
            .pdbscan_state
            .as_ref()
            .map(|s| s.search_reader.is_none())
            .unwrap_or(true);

        if needs_init {
            Self::init_search_reader(state);
        }

        // Execute the scan loop (same logic as PdbScan::exec_custom_scan)
        loop {
            let pdbscan_state = state.custom_state_mut().pdbscan_state.as_mut().unwrap();
            let exec_method = pdbscan_state.exec_method_mut();
            let next_result = exec_method.next(pdbscan_state);

            match next_result {
                crate::postgres::customscan::pdbscan::exec_methods::ExecState::Eof => {
                    return std::ptr::null_mut();
                }
                crate::postgres::customscan::pdbscan::exec_methods::ExecState::RequiresVisibilityCheck {
                    ctid,
                    score,
                    doc_address,
                } => unsafe {
                    // Get scanslot first (immutable borrow)
                    let scanslot = state.scanslot().cast();

                    // Check visibility using the helper function
                    let pdbscan_state = state.custom_state_mut().pdbscan_state.as_mut().unwrap();
                    let slot_opt = crate::postgres::customscan::pdbscan::check_visibility_with_state(
                        pdbscan_state,
                        ctid,
                        scanslot,
                    );

                    let slot = match slot_opt {
                        Some(slot) => {
                            let pdbscan_state = state.custom_state_mut().pdbscan_state.as_mut().unwrap();
                            let exec_method = pdbscan_state.exec_method_mut();
                            exec_method.increment_visible();
                            pdbscan_state.heap_tuple_check_count += 1;
                            slot
                        }
                        None => {
                            let pdbscan_state = state.custom_state_mut().pdbscan_state.as_mut().unwrap();
                            pdbscan_state.invisible_tuple_count += 1;
                            continue;
                        }
                    };

                    let pdbscan_state = state.custom_state().pdbscan_state.as_ref().unwrap();
                    let needs_special_projection = pdbscan_state.need_scores()
                        || pdbscan_state.need_snippets()
                        || pdbscan_state.window_aggregate_results.is_some();

                    if !needs_special_projection {
                        // Simple projection
                        (*(*state.projection_info()).pi_exprContext).ecxt_scantuple = slot;
                        return pg_sys::ExecProject(state.projection_info());
                    } else {
                        // Special projection with scores/snippets/window aggregates
                        return Self::project_tuple_with_special_values(
                            state,
                            slot,
                            score,
                            ctid,
                            doc_address,
                        );
                    }
                },
                crate::postgres::customscan::pdbscan::exec_methods::ExecState::Virtual { slot } => {
                    let pdbscan_state = state.custom_state_mut().pdbscan_state.as_mut().unwrap();
                    pdbscan_state.virtual_tuple_count += 1;
                    return slot;
                }
            }
        }
    }

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        // Cleanup is handled by Drop impls
    }

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        // Cleanup is handled by Drop impls
    }
}

impl WindowScan {
    /// Initialize the search reader for the PdbScanState
    /// This is adapted from PdbScan::init_search_reader
    fn init_search_reader(state: &mut CustomScanStateWrapper<Self>) {
        unsafe {
            let planstate = state.planstate();
            let expr_context = state.runtime_context;

            // Prepare query for execution
            {
                let pdbscan_state = state.custom_state_mut().pdbscan_state.as_mut().unwrap();
                pdbscan_state.prepare_query_for_execution(planstate, expr_context);
            }

            // Open the search reader
            let (search_query_input, need_scores) = {
                let pdbscan_state = state.custom_state().pdbscan_state.as_ref().unwrap();
                let indexrel = pdbscan_state
                    .indexrel
                    .as_ref()
                    .expect("indexrel should be open");
                let search_query_input = pdbscan_state.search_query_input().clone();
                let need_scores = pdbscan_state.need_scores();

                let search_reader =
                    crate::index::reader::index::SearchIndexReader::open_with_context(
                        indexrel,
                        search_query_input.clone(),
                        need_scores,
                        crate::index::mvcc::MvccSatisfies::Snapshot,
                        std::ptr::NonNull::new(expr_context),
                        std::ptr::NonNull::new(planstate),
                    )
                    .expect("should be able to open the search index reader");

                let pdbscan_state = state.custom_state_mut().pdbscan_state.as_mut().unwrap();
                pdbscan_state.search_reader = Some(search_reader);

                (search_query_input, need_scores)
            };

            // Initialize exec method
            {
                let csstate_ptr = addr_of_mut!(state.csstate);
                let pdbscan_state = state.custom_state_mut().pdbscan_state.as_mut().unwrap();
                pdbscan_state.init_exec_method(csstate_ptr);
            }

            // Inject placeholders for scores, snippets, and window aggregates
            Self::inject_placeholders(state);
        }
    }

    /// Inject placeholder nodes for scores, snippets, and window aggregates
    unsafe fn inject_placeholders(state: &mut CustomScanStateWrapper<Self>) {
        let pdbscan_state = state.custom_state().pdbscan_state.as_ref().unwrap();
        let need_scores = pdbscan_state.need_scores();
        let need_snippets = pdbscan_state.need_snippets();
        let has_window_aggs = !pdbscan_state.window_aggregates.is_empty();

        if !need_scores && !need_snippets && !has_window_aggs {
            return;
        }

        let planstate = state.planstate();
        let pdbscan_state = state.custom_state().pdbscan_state.as_ref().unwrap();

        let (targetlist, const_score_node, const_snippet_nodes) =
            crate::postgres::customscan::pdbscan::projections::inject_placeholders(
                (*(*planstate).plan).targetlist,
                pdbscan_state.planning_rti,
                pdbscan_state.score_funcoid,
                pdbscan_state.snippet_funcoid,
                pdbscan_state.snippet_positions_funcoid,
                &pdbscan_state.var_attname_lookup,
                &pdbscan_state.snippet_generators,
            );

        // Inject window aggregate placeholders
        let (targetlist, const_window_agg_nodes) = if !pdbscan_state.window_aggregates.is_empty() {
            crate::postgres::customscan::pdbscan::inject_window_aggregate_placeholders(
                targetlist,
                &pdbscan_state.window_aggregates,
            )
        } else {
            (targetlist, crate::api::HashMap::default())
        };

        let pdbscan_state = state.custom_state_mut().pdbscan_state.as_mut().unwrap();
        pdbscan_state.placeholder_targetlist = Some(targetlist);
        pdbscan_state.const_score_node = Some(const_score_node);
        pdbscan_state.const_snippet_nodes = const_snippet_nodes;
        pdbscan_state.const_window_agg_nodes = const_window_agg_nodes;
    }

    /// Project a tuple with special values (scores, snippets, window aggregates)
    /// This is adapted from PdbScan::exec_custom_scan's projection logic
    unsafe fn project_tuple_with_special_values(
        state: &mut CustomScanStateWrapper<Self>,
        slot: *mut pg_sys::TupleTableSlot,
        score: tantivy::Score,
        ctid: u64,
        doc_address: tantivy::DocAddress,
    ) -> *mut pg_sys::TupleTableSlot {
        use pgrx::PgMemoryContexts;

        let mut per_tuple_context = PgMemoryContexts::For(
            (*(*state.projection_info()).pi_exprContext).ecxt_per_tuple_memory,
        );
        per_tuple_context.reset();

        let pdbscan_state = state.custom_state_mut().pdbscan_state.as_mut().unwrap();

        // Update score
        if pdbscan_state.need_scores() {
            let const_score_node = pdbscan_state
                .const_score_node
                .expect("const_score_node should be set");
            (*const_score_node).constvalue = score.into_datum().unwrap();
            (*const_score_node).constisnull = false;
        }

        // Update window aggregate values
        if let Some(agg_results) = &pdbscan_state.window_aggregate_results {
            for (te_idx, datum) in agg_results {
                if let Some(const_node) = pdbscan_state.const_window_agg_nodes.get(te_idx) {
                    (**const_node).constvalue = *datum;
                    (**const_node).constisnull = false;
                }
            }
        }

        // Update snippets
        if pdbscan_state.need_snippets() {
            per_tuple_context.switch_to(|_| {
                for (snippet_type, const_snippet_nodes) in &pdbscan_state.const_snippet_nodes {
                    match snippet_type {
                        crate::postgres::customscan::pdbscan::projections::SnippetType::Text(_, _, config, _) => {
                            let snippet = pdbscan_state.make_snippet(ctid, snippet_type);
                            for const_ in const_snippet_nodes {
                                match &snippet {
                                    Some(text) => {
                                        (**const_).constvalue = text.into_datum().unwrap();
                                        (**const_).constisnull = false;
                                    }
                                    None => {
                                        (**const_).constvalue = pg_sys::Datum::null();
                                        (**const_).constisnull = true;
                                    }
                                }
                            }
                        }
                        crate::postgres::customscan::pdbscan::projections::SnippetType::Positions(..) => {
                            let positions = pdbscan_state.get_snippet_positions(ctid, snippet_type);
                            for const_ in const_snippet_nodes {
                                match &positions {
                                    Some(positions) => {
                                        (**const_).constvalue = positions.clone().into_datum().unwrap();
                                        (**const_).constisnull = false;
                                    }
                                    None => {
                                        (**const_).constvalue = pg_sys::Datum::null();
                                        (**const_).constisnull = true;
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }

        // Do the projection
        (*(*state.projection_info()).pi_exprContext).ecxt_scantuple = slot;
        pg_sys::ExecProject(state.projection_info())
    }
}

/// Extract ORDER BY and LIMIT information from the query parse tree
/// This is needed for TopN execution with window aggregates
unsafe fn extract_orderby_and_limit(
    parse: *mut pg_sys::Query,
    root: *mut pg_sys::PlannerInfo,
) -> (Option<usize>, Option<Vec<crate::api::OrderByInfo>>) {
    use pgrx::{FromDatum, IntoDatum, PgList};

    // Extract LIMIT
    let limit = if !(*parse).limitCount.is_null() {
        let const_node = crate::nodecast!(Const, T_Const, (*parse).limitCount);
        if let Some(const_node) = const_node {
            i64::from_datum((*const_node).constvalue, (*const_node).constisnull)
                .and_then(|v| usize::try_from(v).ok())
        } else {
            None
        }
    } else {
        None
    };

    // Extract ORDER BY
    let orderby_info = if !(*parse).sortClause.is_null() {
        let sort_clause = PgList::<pg_sys::SortGroupClause>::from_pg((*parse).sortClause);
        let mut orderby_vec = Vec::new();

        for sort_clause_ptr in sort_clause.iter_ptr() {
            let expr = pg_sys::get_sortgroupclause_expr(sort_clause_ptr, (*parse).targetList);
            let var_context = crate::postgres::var::VarContext::from_planner(root);

            if let Some((_, field_name)) =
                crate::postgres::var::find_one_var_and_fieldname(var_context, expr)
            {
                // Determine sort direction from nulls_first flag
                // In PostgreSQL, ASC NULLS LAST is the default, DESC NULLS FIRST is the default
                // We use nulls_first as a heuristic: if nulls_first is true, it's likely DESC
                let direction = if (*sort_clause_ptr).nulls_first {
                    crate::api::SortDirection::Desc
                } else {
                    crate::api::SortDirection::Asc
                };

                orderby_vec.push(crate::api::OrderByInfo {
                    feature: crate::api::OrderByFeature::Field(field_name),
                    direction,
                });
            }
        }

        if orderby_vec.is_empty() {
            None
        } else {
            Some(orderby_vec)
        }
    } else {
        None
    };

    (limit, orderby_info)
}

#[derive(Default)]
pub struct WindowScanState {
    pub window_aggregates: Vec<window_agg::WindowAggregateInfo>,
    pub pdbscan_state: Option<Box<crate::postgres::customscan::pdbscan::scan_state::PdbScanState>>,
}

impl CustomScanState for WindowScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        // Delegate to PdbScan's initialization
        if let Some(ref mut pdbscan_state) = self.pdbscan_state {
            pdbscan_state.init_exec_method(cstate);
        }
    }
}

impl PlainExecCapable for WindowScan {}

impl ExecMethod for WindowScan {
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        <WindowScan as PlainExecCapable>::exec_methods()
    }
}

/// Replace WindowFunc nodes in the target list with window_func() placeholders
unsafe fn replace_windowfuncs_in_target_list(plan: *mut pg_sys::Plan) {
    if (*plan).targetlist.is_null() {
        return;
    }

    let original_tlist = PgList::<pg_sys::TargetEntry>::from_pg((*plan).targetlist);
    let mut new_targetlist = PgList::<pg_sys::TargetEntry>::new();

    for te in original_tlist.iter_ptr() {
        if (*(*te).expr).type_ == pg_sys::NodeTag::T_WindowFunc {
            // Create a flat copy of the target entry
            let new_te = pg_sys::flatCopyTargetEntry(te);

            // Replace the T_WindowFunc with a T_FuncExpr placeholder (window_func)
            let funcexpr = make_window_func_placeholder((*te).expr as *mut pg_sys::WindowFunc);
            (*new_te).expr = funcexpr as *mut pg_sys::Expr;

            new_targetlist.push(new_te);
        } else {
            // For non-WindowFunc entries, just make a flat copy
            let copied_te = pg_sys::flatCopyTargetEntry(te);
            new_targetlist.push(copied_te);
        }
    }

    (*plan).targetlist = new_targetlist.into_pg();
}

/// Create a window_func() placeholder to replace a WindowFunc node
unsafe fn make_window_func_placeholder(
    window_func: *mut pg_sys::WindowFunc,
) -> *mut pg_sys::FuncExpr {
    // TODO: Serialize the window function specification to JSON and pass as argument
    // For now, create a simple placeholder

    let funcexpr: *mut pg_sys::FuncExpr =
        pg_sys::palloc0(std::mem::size_of::<pg_sys::FuncExpr>()).cast();
    (*funcexpr).xpr.type_ = pg_sys::NodeTag::T_FuncExpr;
    (*funcexpr).funcid = window_func_procid();
    (*funcexpr).funcresulttype = pg_sys::INT8OID; // Default to int8 for now
    (*funcexpr).funcretset = false;
    (*funcexpr).funcvariadic = false;
    (*funcexpr).funcformat = pg_sys::CoercionForm::COERCE_EXPLICIT_CALL;
    (*funcexpr).funccollid = pg_sys::InvalidOid;
    (*funcexpr).inputcollid = pg_sys::InvalidOid;
    (*funcexpr).location = (*window_func).location;
    (*funcexpr).args = PgList::<pg_sys::Node>::new().into_pg();

    funcexpr
}

/// Get the Oid of the window_func placeholder function
unsafe fn window_func_procid() -> pg_sys::Oid {
    // Get the OID of paradedb.window_func
    pgrx::direct_function_call::<pg_sys::Oid>(
        pg_sys::regprocedurein,
        &[c"paradedb.window_func(text)".into_datum()],
    )
    .expect("the `paradedb.window_func` function should exist")
}
