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

//! PostgreSQL planner hooks for custom scan integration.
//!
//! This module provides hooks into PostgreSQL's query planner to enable custom execution
//! strategies for search queries and aggregations.
//!
//! ## Hook Architecture
//!
//! We use two main PostgreSQL hooks:
//!
//! ### 1. `set_rel_pathlist_hook` - Base Relation Scanning
//!
//! Called when PostgreSQL builds access paths for base relations (tables). This is where
//! we inject custom scan paths for:
//! - **PdbScan**: Search queries with the `@@@` operator
//! - Supports TopN (ORDER BY + LIMIT), Normal, and FastFields execution modes
//! - Handles window functions in TopN queries
//!
//! ### 2. `create_upper_paths_hook` - Upper Node Planning
//!
//! Called when PostgreSQL plans upper nodes like aggregations and window functions. We handle:
//!
//! #### UPPERREL_GROUP_AGG Stage
//! - **AggregateScan**: GROUP BY queries with search conditions
//! - Combines grouping and aggregation with Tantivy collectors
//!
//! #### UPPERREL_WINDOW Stage
//! - Window functions are handled via `planner_hook` (see below), not here
//! - We investigated handling them here but discovered it's not feasible due to timing:
//!   base relation planning is already complete at this stage
//! - We do detect window functions here for debugging/validation purposes
//!
//! ### 3. `planner_hook` - Early Query Tree Manipulation
//!
//! Called at the very start of query planning, before PostgreSQL processes the query tree.
//! Currently used for:
//! - **Window Functions**: Detecting and replacing window functions (e.g., `COUNT(*) OVER ()`)
//!   with placeholder functions (`window_func(json)`) that contain serialized aggregation specs
//! - Only replaces window functions in queries with the `@@@` operator
//!
//! **Why we need this hook (and not `create_upper_paths_hook` at `UPPERREL_WINDOW`):**
//!
//! We investigated moving this to `create_upper_paths_hook` at `UPPERREL_WINDOW` stage to align
//! with the AggregateScan pattern, but discovered a fundamental timing issue:
//!
//! - **At `UPPERREL_WINDOW` stage**: Base relation planning is already complete. PdbScan has
//!   already created its paths based on the original parse tree. If we replace WindowFunc nodes
//!   at this stage, we can't go back and tell PdbScan to re-plan with the modified parse tree.
//!
//! - **At `planner_hook` stage**: We replace WindowFunc nodes BEFORE any planning happens.
//!   When PdbScan runs during base relation planning, it sees the `window_func()` placeholders
//!   and can handle them appropriately in TopN queries.
//!
//! **Key difference from AggregateScan:**
//!
//! AggregateScan CAN work at `UPPERREL_GROUP_AGG` by replacing Aggref nodes in `plan_custom_path`
//! because:
//! 1. GROUP BY queries create a separate upper relation (`UPPERREL_GROUP_AGG`)
//! 2. AggregateScan creates a custom path that wraps the base relation
//! 3. When chosen, it replaces Aggref nodes and handles aggregation itself
//! 4. The base relation (input path) doesn't need to know about aggregates
//!
//! Window functions use `planner_hook` instead of `create_upper_paths_hook` because:
//! 1. Window functions often appear in TopN queries (ORDER BY + LIMIT)
//! 2. TopN is handled by PdbScan at base relation planning time
//! 3. Replacing WindowFunc nodes in `planner_hook` allows PdbScan to see them during planning
//! 4. PdbScan can then integrate window aggregate execution into TopN (single-pass with `MultiCollector`)
//!
//! **Alternative considered:** Creating a WindowScan at `UPPERREL_WINDOW` that:
//! - Extracts window specifications
//! - Replaces WindowFunc nodes in `plan_custom_path`
//! - Delegates execution to PdbScan's code
//!
//! This would work (it's just code reuse/refactoring), but `planner_hook` is simpler because:
//! - No need for a separate WindowScan custom scan type
//! - No need to duplicate path creation logic
//! - Direct integration with PdbScan's existing window aggregate support
//! - Less code, same functionality
//!
//! **Conclusion:** The `planner_hook` approach is the simplest and most maintainable solution.
//! It enables efficient single-pass execution of TopN + window aggregates.
//!
//! ## Window Function Flow
//!
//! 1. **Planner Hook**: Detects window functions in queries with `@@@`
//! 2. **Replacement**: Replaces them with `window_func(json)` placeholders
//! 3. **Custom Scan Planning**: PdbScan deserializes the JSON and plans execution
//! 4. **Execution**: TopN executor combines TopDocs and AggregationCollector in single pass
//!
//! ## Operator Detection
//!
//! Both PdbScan and AggregateScan detect the `@@@` operator to determine if they should
//! handle a query. The planner hook also checks for this operator before replacing window
//! functions. This detection logic is currently duplicated and could be unified.

use crate::api::operator::{anyelement_query_input_opoid, anyelement_text_opoid};
use crate::api::window_function::window_func_oid;
use crate::gucs;
use crate::nodecast;
use crate::postgres::customscan::agg::AggregationSpec;
use crate::postgres::customscan::builders::custom_path::{CustomPathBuilder, Flags};
use crate::postgres::customscan::pdbscan::projections::window_agg;
use crate::postgres::customscan::{CreateUpperPathsHookArgs, CustomScan, RelPathlistHookArgs};
use crate::postgres::utils::expr_contains_any_operator;
use once_cell::sync::Lazy;
use pgrx::{pg_guard, pg_sys, PgList, PgMemoryContexts};
use std::collections::{hash_map::Entry, HashMap};

unsafe fn add_path(rel: *mut pg_sys::RelOptInfo, mut path: pg_sys::CustomPath) {
    let forced = path.flags & Flags::Force as u32 != 0;
    path.flags ^= Flags::Force as u32; // make sure to clear this flag because it's special to us

    let mut custom_path = PgMemoryContexts::CurrentMemoryContext
        .copy_ptr_into(&mut path, std::mem::size_of_val(&path));

    if (*custom_path).path.parallel_aware {
        // add the partial path since the user-generated plan is parallel aware
        pg_sys::add_partial_path(rel, custom_path.cast());

        // remove all the existing possible paths
        (*rel).pathlist = std::ptr::null_mut();

        // then make another copy of it, increase its costs really, really high and
        // submit it as a regular path too, immediately after clearing out all the other
        // existing possible paths.
        //
        // We don't want postgres to choose this path, but we have to have at least one
        // non-partial path available for it to consider
        let copy = PgMemoryContexts::CurrentMemoryContext
            .copy_ptr_into(&mut path, std::mem::size_of_val(&path));
        (*copy).path.parallel_aware = false;
        (*copy).path.total_cost = 1000000000.0;
        (*copy).path.startup_cost = 1000000000.0;

        // will be added down below
        custom_path = copy.cast();
    } else if forced {
        // remove all the existing possible paths
        (*rel).pathlist = std::ptr::null_mut();
    }

    // add this path for consideration
    pg_sys::add_path(rel, custom_path.cast());
}

pub fn register_rel_pathlist<CS>(_: CS)
where
    CS: CustomScan<Args = RelPathlistHookArgs> + 'static,
{
    unsafe {
        static mut PREV_HOOKS: Lazy<HashMap<std::any::TypeId, pg_sys::set_rel_pathlist_hook_type>> =
            Lazy::new(Default::default);

        #[pg_guard]
        extern "C-unwind" fn __priv_callback<CS>(
            root: *mut pg_sys::PlannerInfo,
            rel: *mut pg_sys::RelOptInfo,
            rti: pg_sys::Index,
            rte: *mut pg_sys::RangeTblEntry,
        ) where
            CS: CustomScan<Args = RelPathlistHookArgs> + 'static,
        {
            unsafe {
                #[allow(static_mut_refs)]
                if let Some(Some(prev_hook)) = PREV_HOOKS.get(&std::any::TypeId::of::<CS>()) {
                    (*prev_hook)(root, rel, rti, rte);
                }

                paradedb_rel_pathlist_callback::<CS>(root, rel, rti, rte);
            }
        }

        #[allow(static_mut_refs)]
        match PREV_HOOKS.entry(std::any::TypeId::of::<CS>()) {
            Entry::Occupied(_) => panic!("{} is already registered", std::any::type_name::<CS>()),
            Entry::Vacant(entry) => entry.insert(pg_sys::set_rel_pathlist_hook),
        };

        pg_sys::set_rel_pathlist_hook = Some(__priv_callback::<CS>);

        pg_sys::RegisterCustomScanMethods(CS::custom_scan_methods())
    }
}

/// Although this hook function can be used to examine, modify, or remove paths generated by the
/// core system, a custom scan provider will typically confine itself to generating CustomPath
/// objects and adding them to rel using add_path. The custom scan provider is responsible for
/// initializing the CustomPath object, which is declared like this:
#[pg_guard]
pub extern "C-unwind" fn paradedb_rel_pathlist_callback<CS>(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    rti: pg_sys::Index,
    rte: *mut pg_sys::RangeTblEntry,
) where
    CS: CustomScan<Args = RelPathlistHookArgs> + 'static,
{
    unsafe {
        if !gucs::enable_custom_scan() {
            return;
        }

        let Some(path) = CS::create_custom_path(CustomPathBuilder::new(
            root,
            rel,
            RelPathlistHookArgs {
                root,
                rel,
                rti,
                rte,
            },
        )) else {
            return;
        };

        add_path(rel, path)
    }
}

pub fn register_upper_path<CS>(_: CS)
where
    CS: CustomScan<Args = CreateUpperPathsHookArgs> + 'static,
{
    unsafe {
        static mut PREV_HOOKS: Lazy<
            HashMap<std::any::TypeId, pg_sys::create_upper_paths_hook_type>,
        > = Lazy::new(Default::default);

        #[pg_guard]
        extern "C-unwind" fn __priv_callback<CS>(
            root: *mut pg_sys::PlannerInfo,
            stage: pg_sys::UpperRelationKind::Type,
            input_rel: *mut pg_sys::RelOptInfo,
            output_rel: *mut pg_sys::RelOptInfo,
            extra: *mut ::std::os::raw::c_void,
        ) where
            CS: CustomScan<Args = CreateUpperPathsHookArgs> + 'static,
        {
            unsafe {
                #[allow(static_mut_refs)]
                if let Some(Some(prev_hook)) = PREV_HOOKS.get(&std::any::TypeId::of::<CS>()) {
                    (*prev_hook)(root, stage, input_rel, output_rel, extra);
                }

                paradedb_upper_paths_callback::<CS>(root, stage, input_rel, output_rel, extra);
            }
        }

        #[allow(static_mut_refs)]
        match PREV_HOOKS.entry(std::any::TypeId::of::<CS>()) {
            Entry::Occupied(_) => panic!("{} is already registered", std::any::type_name::<CS>()),
            Entry::Vacant(entry) => entry.insert(pg_sys::create_upper_paths_hook),
        };

        pg_sys::create_upper_paths_hook = Some(__priv_callback::<CS>);

        pg_sys::RegisterCustomScanMethods(CS::custom_scan_methods())
    }
}

#[pg_guard]
pub extern "C-unwind" fn paradedb_upper_paths_callback<CS>(
    root: *mut pg_sys::PlannerInfo,
    stage: pg_sys::UpperRelationKind::Type,
    input_rel: *mut pg_sys::RelOptInfo,
    output_rel: *mut pg_sys::RelOptInfo,
    extra: *mut ::std::os::raw::c_void,
) where
    CS: CustomScan<Args = CreateUpperPathsHookArgs> + 'static,
{
    // Determine which CustomScan type we're dealing with
    let is_aggregate_scan = std::any::TypeId::of::<CS>()
        == std::any::TypeId::of::<crate::postgres::customscan::aggregatescan::AggregateScan>();

    // AggregateScan handles both UPPERREL_GROUP_AGG and UPPERREL_WINDOW stages
    if stage == pg_sys::UpperRelationKind::UPPERREL_GROUP_AGG
        || stage == pg_sys::UpperRelationKind::UPPERREL_WINDOW
    {
        if !is_aggregate_scan {
            return; // Not AggregateScan, skip
        }

        let stage_name = if stage == pg_sys::UpperRelationKind::UPPERREL_GROUP_AGG {
            "UPPERREL_GROUP_AGG"
        } else {
            "UPPERREL_WINDOW"
        };
        pgrx::warning!("Hook: {} stage, calling AggregateScan", stage_name);

        if stage == pg_sys::UpperRelationKind::UPPERREL_GROUP_AGG
            && !gucs::enable_aggregate_custom_scan()
        {
            pgrx::warning!("  aggregate custom scan disabled, returning");
            return;
        }

        unsafe {
            let path = CS::create_custom_path(CustomPathBuilder::new(
                root,
                output_rel,
                CreateUpperPathsHookArgs {
                    root,
                    stage,
                    input_rel,
                    output_rel,
                    extra,
                },
            ));

            if path.is_some() {
                pgrx::warning!("  AggregateScan returned a path, adding it");
                add_path(output_rel, path.unwrap());
            } else {
                pgrx::warning!("  AggregateScan returned None");
            }
        }
        return;
    }
}

/// Handle window functions at UPPERREL_WINDOW stage
///
/// This function is called when PostgreSQL is planning window functions. At this stage,
/// PostgreSQL has already created WindowAgg paths, but the WindowFunc nodes are still
/// in the parse tree. We extract the window functions, check if we should handle them
/// (query has @@@ operator), and if so, replace them with placeholders.
///
/// By replacing the WindowFunc nodes with window_func() placeholders, we prevent PostgreSQL
/// from using the WindowAggPath it created. The modified parse tree will cause the query
/// to be handled by PdbScan during base relation planning.
unsafe fn handle_window_upper_path(
    root: *mut pg_sys::PlannerInfo,
    input_rel: *mut pg_sys::RelOptInfo,
    output_rel: *mut pg_sys::RelOptInfo,
) {
    // Get the parse tree from root
    if root.is_null() || (*root).parse.is_null() {
        return;
    }

    let parse = (*root).parse;

    // Check if query has window functions and search operator
    if !query_has_window_functions(parse) || !query_has_search_operator(parse) {
        return;
    }

    // Extract window function specifications from the parse tree
    let window_specs = window_agg::extract_window_specifications(parse);
    if window_specs.is_empty() {
        return;
    }

    // We detected window functions with @@@ operator, but we don't handle them here.
    // The planner_hook has already replaced them before we reach this stage.
    // This function exists for validation/debugging purposes and to document why
    // we can't handle window functions at UPPERREL_WINDOW stage.
    //
    // Key insight: At this stage, base relation planning is complete. PdbScan has already
    // created its paths. We can't modify the parse tree here and expect PdbScan to re-plan.
    // The replacement must happen BEFORE base relation planning, which is what planner_hook does.
}

/// Register a global planner hook to intercept and modify queries before planning.
/// This is called once during extension initialization and affects all queries.
pub unsafe fn register_window_function_hook() {
    static mut PREV_PLANNER_HOOK: pg_sys::planner_hook_type = None;

    PREV_PLANNER_HOOK = pg_sys::planner_hook;
    pg_sys::planner_hook = Some(paradedb_planner_hook);
}

/// Planner hook that replaces WindowFunc nodes before PostgreSQL processes them
/// Only replaces window functions if the query will be handled by our custom scans
#[pg_guard]
unsafe extern "C-unwind" fn paradedb_planner_hook(
    parse: *mut pg_sys::Query,
    query_string: *const ::core::ffi::c_char,
    cursor_options: ::core::ffi::c_int,
    bound_params: pg_sys::ParamListInfo,
) -> *mut pg_sys::PlannedStmt {
    // Check if this is a SELECT query with window functions that we can handle
    if !parse.is_null() && (*parse).commandType == pg_sys::CmdType::CMD_SELECT {
        // Note: it's important to check for window functions first, then search operator,
        // otherwise we'd call query_has_search_operator() during the DROP EXTENSION, which
        // would cause a panic.
        if query_has_window_functions(parse) && query_has_search_operator(parse) {
            // Extract and replace window functions recursively (including subqueries)
            replace_windowfuncs_recursively(parse);
        }
    }

    // Call the previous planner hook or standard planner
    static mut PREV_PLANNER_HOOK: pg_sys::planner_hook_type = None;
    if let Some(prev_hook) = PREV_PLANNER_HOOK {
        prev_hook(parse, query_string, cursor_options, bound_params)
    } else {
        pg_sys::standard_planner(parse, query_string, cursor_options, bound_params)
    }
}

/// Check if the target list contains any window functions
unsafe fn has_window_functions(target_list: *mut pg_sys::List) -> bool {
    let tlist = PgList::<pg_sys::TargetEntry>::from_pg(target_list);
    for te in tlist.iter_ptr() {
        if nodecast!(WindowFunc, T_WindowFunc, (*te).expr).is_some() {
            return true;
        }
    }
    false
}

/// Check if the query (or any subquery) contains window functions
unsafe fn query_has_window_functions(parse: *mut pg_sys::Query) -> bool {
    if parse.is_null() {
        return false;
    }

    // Check the current query's target list
    if !(*parse).targetList.is_null() && has_window_functions((*parse).targetList) {
        return true;
    }

    // Check subqueries in RTEs (only if SUBQUERY_SUPPORT is enabled)
    if window_agg::window_functions::SUBQUERY_SUPPORT && !(*parse).rtable.is_null() {
        let rtable = PgList::<pg_sys::RangeTblEntry>::from_pg((*parse).rtable);
        for (idx, rte) in rtable.iter_ptr().enumerate() {
            if (*rte).rtekind == pg_sys::RTEKind::RTE_SUBQUERY
                && !(*rte).subquery.is_null()
                && query_has_window_functions((*rte).subquery)
            {
                return true;
            }
        }
    }

    false
}

/// Check if the query contains the @@@ search operator
/// This indicates that our custom scans will likely handle this query
unsafe fn query_has_search_operator(parse: *mut pg_sys::Query) -> bool {
    let searchqueryinput_opno = anyelement_query_input_opoid();
    let text_opno = anyelement_text_opoid();

    // Check WHERE clause (jointree->quals)
    if !(*parse).jointree.is_null() {
        let jointree = (*parse).jointree;
        if !(*jointree).quals.is_null()
            && expr_contains_any_operator((*jointree).quals, &[searchqueryinput_opno, text_opno])
        {
            return true;
        }
    }

    // Check HAVING clause
    if !(*parse).havingQual.is_null()
        && expr_contains_any_operator((*parse).havingQual, &[searchqueryinput_opno, text_opno])
    {
        return true;
    }

    // Check target list (for search operators in SELECT expressions, aggregates, window functions)
    if !(*parse).targetList.is_null() {
        let target_list = PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList);
        for te in target_list.iter_ptr() {
            if !(*te).expr.is_null()
                && expr_contains_any_operator(
                    (*te).expr as *mut pg_sys::Node,
                    &[searchqueryinput_opno, text_opno],
                )
            {
                return true;
            }
        }
    }

    // Check if any RTEs contain subqueries with @@@ operators
    if !(*parse).rtable.is_null() {
        let rtable = PgList::<pg_sys::RangeTblEntry>::from_pg((*parse).rtable);
        for (idx, rte) in rtable.iter_ptr().enumerate() {
            if (*rte).rtekind == pg_sys::RTEKind::RTE_SUBQUERY
                && !(*rte).subquery.is_null()
                && query_has_search_operator((*rte).subquery)
            {
                return true;
            }
        }
    }

    false
}

/// Recursively replace window functions in the query and all subqueries
unsafe fn replace_windowfuncs_recursively(parse: *mut pg_sys::Query) {
    if parse.is_null() {
        return;
    }

    // Extract window functions from current query
    let window_specs = window_agg::extract_window_specifications(parse);
    if !window_specs.is_empty() {
        // Replace window functions in current query
        replace_windowfuncs_in_query(parse, &window_specs);
    }

    // Recursively process subqueries in RTEs
    if !(*parse).rtable.is_null() {
        let rtable = PgList::<pg_sys::RangeTblEntry>::from_pg((*parse).rtable);
        for (idx, rte) in rtable.iter_ptr().enumerate() {
            if (*rte).rtekind == pg_sys::RTEKind::RTE_SUBQUERY && !(*rte).subquery.is_null() {
                // Check if subquery support is enabled
                if window_agg::window_functions::SUBQUERY_SUPPORT {
                    replace_windowfuncs_recursively((*rte).subquery);
                }
                // If SUBQUERY_SUPPORT is false, we skip processing subqueries,
                // leaving their window functions for PostgreSQL to handle
            }
        }
    }
}

/// Replace WindowFunc nodes in the Query's target list with placeholder functions
///
/// Takes a map of target_entry_index -> AggregationSpec and replaces each WindowFunc
/// with a paradedb.window_func(json) call containing the serialized AggregationSpec.
unsafe fn replace_windowfuncs_in_query(
    parse: *mut pg_sys::Query,
    window_specs: &HashMap<usize, AggregationSpec>,
) {
    if (*parse).targetList.is_null() {
        return;
    }

    let original_tlist = PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList);
    let mut new_targetlist = PgList::<pg_sys::TargetEntry>::new();
    let window_func_procid = window_func_oid();
    let mut replaced_count = 0;

    for (idx, te) in original_tlist.iter_ptr().enumerate() {
        if let Some(_window_func) = nodecast!(WindowFunc, T_WindowFunc, (*te).expr) {
            // Create a flat copy of the target entry
            let new_te = pg_sys::flatCopyTargetEntry(te);

            // Get the window specification for this target entry
            if let Some(window_spec) = window_specs.get(&idx) {
                let json = serde_json::to_string(window_spec)
                    .expect("Failed to serialize WindowSpecification");

                // Create a Const node for the JSON string
                let json_cstring = std::ffi::CString::new(json).expect("Invalid JSON string");
                let json_text = pg_sys::cstring_to_text(json_cstring.as_ptr());
                let json_datum = pg_sys::Datum::from(json_text as usize);

                // Create an argument list with the JSON string
                let mut args = PgList::<pg_sys::Node>::new();
                let json_const = pg_sys::makeConst(
                    pg_sys::TEXTOID,
                    -1,
                    pg_sys::DEFAULT_COLLATION_OID,
                    -1,
                    json_datum,
                    false, // not null
                    false, // not passed by value (text is varlena)
                );
                args.push(json_const.cast());

                // Create a FuncExpr that calls paradedb.window_func(json)
                let funcexpr = pg_sys::makeFuncExpr(
                    window_func_procid,
                    window_spec.result_type_oid(),
                    args.into_pg(),
                    pg_sys::InvalidOid,
                    pg_sys::InvalidOid,
                    pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
                );

                // Replace the WindowFunc with our placeholder FuncExpr
                (*new_te).expr = funcexpr.cast();
                new_targetlist.push(new_te);
                replaced_count += 1;
            } else {
                // Still copy the entry but don't replace it
                new_targetlist.push(te);
            }
        } else {
            // For non-WindowFunc entries, just make a flat copy
            let copied_te = pg_sys::flatCopyTargetEntry(te);
            new_targetlist.push(copied_te);
        }
    }

    (*parse).targetList = new_targetlist.into_pg();
}
