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

#![allow(clippy::unnecessary_cast)] // helps with integer casting differences between postgres versions
mod exec_methods;
mod optimized_unified_evaluator;
pub mod parallel;
mod privdat;
mod projections;
mod pushdown;
mod qual_inspect;
mod scan_state;
mod solve_expr;
mod unified_evaluator;

use crate::api::operator::{
    anyelement_query_input_opoid, anyelement_text_opoid, estimate_selectivity,
};
use crate::api::Cardinality;
use crate::api::{HashMap, HashSet};
use crate::debug_log;
use crate::index::fast_fields_helper::WhichFastField;
use crate::index::mvcc::{MVCCDirectory, MvccSatisfies};
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::builders::custom_path::{
    CustomPathBuilder, ExecMethodType, Flags, OrderByStyle, RestrictInfoType, SortDirection,
};
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::pdbscan::exec_methods::{
    fast_fields, normal::NormalScanExecState, ExecState,
};
use crate::postgres::customscan::pdbscan::optimized_unified_evaluator::{
    apply_optimized_unified_heap_filter, ExpressionTreeOptimizer, OptimizedEvaluationResult,
};
use crate::postgres::customscan::pdbscan::parallel::{compute_nworkers, list_segment_ids};
use crate::postgres::customscan::pdbscan::privdat::PrivateData;
use crate::postgres::customscan::pdbscan::projections::score::{
    is_score_func, score_funcoid, uses_scores,
};
use crate::postgres::customscan::pdbscan::projections::snippet::{
    snippet_funcoid, snippet_positions_funcoid, uses_snippets, SnippetType,
};
use crate::postgres::customscan::pdbscan::projections::{
    inject_placeholders, maybe_needs_const_projections, pullout_funcexprs,
};
use crate::postgres::customscan::pdbscan::qual_inspect::{
    extract_join_predicates, extract_quals, extract_quals_with_non_indexed, Qual,
};
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::postgres::customscan::pdbscan::unified_evaluator::{
    apply_complete_unified_heap_filter, UnifiedEvaluationResult,
};
use crate::postgres::customscan::{self, CustomScan, CustomScanState};
use crate::postgres::rel_get_bm25_index;
use crate::postgres::var::find_var_relation;
use crate::postgres::visibility_checker::VisibilityChecker;
use crate::query::AsHumanReadable;
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;
use crate::{nodecast, DEFAULT_STARTUP_COST, PARAMETERIZED_SELECTIVITY, UNKNOWN_SELECTIVITY};
use crate::{FULL_RELATION_SELECTIVITY, UNASSIGNED_SELECTIVITY};
use pgrx::pg_sys::CustomExecMethods;
use pgrx::{
    direct_function_call, pg_sys, FromDatum, IntoDatum, PgList, PgMemoryContexts, PgRelation,
};
use std::ffi::CStr;
use std::ptr::addr_of_mut;
use tantivy::snippet::SnippetGenerator;
use tantivy::{DocAddress, Index};

#[derive(Default)]
pub struct PdbScan;

impl PdbScan {
    // This is the core logic for (re-)initializing the search reader
    fn init_search_reader(state: &mut CustomScanStateWrapper<Self>) {
        let planstate = state.planstate();
        let expr_context = state.runtime_context;
        state
            .custom_state_mut()
            .prepare_query_for_execution(planstate, expr_context);

        // Open the index
        let indexrel = state
            .custom_state()
            .indexrel
            .as_ref()
            .map(|indexrel| unsafe { PgRelation::from_pg(*indexrel) })
            .expect("custom_state.indexrel should already be open");

        let search_reader = SearchIndexReader::open(&indexrel, unsafe {
            if pg_sys::ParallelWorkerNumber == -1 {
                // the leader only sees snapshot-visible segments
                MvccSatisfies::Snapshot
            } else {
                // the workers have their own rules, which is literally every segment
                // this is because the workers pick a specific segment to query that
                // is known to be held open/pinned by the leader but might not pass a ::Snapshot
                // visibility test due to concurrent merges/garbage collects
                MvccSatisfies::ParallelWorker(list_segment_ids(
                    state.custom_state().parallel_state.expect(
                        "Parallel Custom Scan rescan_custom_scan should have a parallel state",
                    ),
                ))
            }
        })
        .expect("should be able to open the search index reader");
        state.custom_state_mut().search_reader = Some(search_reader);

        let csstate = addr_of_mut!(state.csstate);
        state.custom_state_mut().init_exec_method(csstate);

        if state.custom_state().need_snippets() {
            let mut snippet_generators: HashMap<
                SnippetType,
                Option<(tantivy::schema::Field, SnippetGenerator)>,
            > = state
                .custom_state_mut()
                .snippet_generators
                .drain()
                .collect();

            // Pre-compute enhanced queries for snippet generation if we have join predicates
            let enhanced_query_for_snippets =
                if let Some(ref join_predicate) = state.custom_state().join_predicates {
                    // Combine base query with join predicate for snippet generation
                    let base_query = state.custom_state().search_query_input();
                    Some(SearchQueryInput::Boolean {
                        must: vec![base_query.clone()],
                        should: vec![join_predicate.clone()],
                        must_not: vec![],
                    })
                } else {
                    None
                };

            for (snippet_type, generator) in &mut snippet_generators {
                // Use enhanced query if available, otherwise use base query
                let query_to_use = enhanced_query_for_snippets
                    .as_ref()
                    .unwrap_or_else(|| state.custom_state().search_query_input());

                let mut new_generator = state
                    .custom_state()
                    .search_reader
                    .as_ref()
                    .unwrap()
                    .snippet_generator(snippet_type.field().root(), query_to_use);

                // If SnippetType::Positions, set max_num_chars to u32::MAX because the entire doc must be considered
                // This assumes text fields can be no more than u32::MAX bytes
                let max_num_chars = match snippet_type {
                    SnippetType::Text(_, _, config) => config.max_num_chars,
                    SnippetType::Positions(_, _) => u32::MAX as usize,
                };
                new_generator.1.set_max_num_chars(max_num_chars);

                *generator = Some(new_generator);
            }

            state.custom_state_mut().snippet_generators = snippet_generators;
        }

        unsafe {
            inject_score_and_snippet_placeholders(state);
        }
    }

    fn cleanup_varibilities_from_tantivy_query(json_value: &mut serde_json::Value) {
        match json_value {
            serde_json::Value::Object(obj) => {
                // Check if this is a "with_index" object and remove its "oid" if present
                if obj.contains_key("with_index") {
                    if let Some(with_index) = obj.get_mut("with_index") {
                        if let Some(with_index_obj) = with_index.as_object_mut() {
                            with_index_obj.remove("oid");
                        }
                    }
                }

                // Remove any field named "postgres_expression"
                obj.remove("postgres_expression");

                // Recursively process all values in the object
                for (_, value) in obj.iter_mut() {
                    Self::cleanup_varibilities_from_tantivy_query(value);
                }
            }
            serde_json::Value::Array(arr) => {
                // Recursively process all elements in the array
                for item in arr.iter_mut() {
                    Self::cleanup_varibilities_from_tantivy_query(item);
                }
            }
            // Base cases: primitive values don't need processing
            _ => {}
        }
    }

    unsafe fn extract_all_possible_quals(
        builder: &mut CustomPathBuilder<PrivateData>,
        root: *mut pg_sys::PlannerInfo,
        rti: pg_sys::Index,
        restrict_info: PgList<pg_sys::RestrictInfo>,
        pdbopoid: pg_sys::Oid,
        ri_type: RestrictInfoType,
        schema: &SearchIndexSchema,
    ) -> (Option<Qual>, RestrictInfoType, PgList<pg_sys::RestrictInfo>) {
        // Phase 4: Enhanced unified approach with smart base query optimization
        // Instead of using All query, we extract indexed predicates to create a more efficient base query
        // that reduces the document set before unified evaluation

        debug_log!(
            "🔍 [EXTRACT_QUALS] Starting qual extraction for rti={}, restrict_info count={}",
            rti,
            restrict_info.len()
        );

        let mut uses_tantivy_to_query = false;

        // First, try to extract any search operators (@@@ operators) from the expressions
        debug_log!(
            "🔍 [EXTRACT_QUALS] Calling extract_quals_with_non_indexed for base restrictions"
        );
        let (indexed_quals, _all_quals) = extract_quals_with_non_indexed(
            root,
            rti,
            restrict_info.as_ptr().cast(),
            pdbopoid,
            ri_type,
            schema,
            true, // Always convert external predicates to allow partial extraction
            &mut uses_tantivy_to_query,
        );

        debug_log!("🔍 [EXTRACT_QUALS] Base extraction result: uses_tantivy_to_query={}, indexed_quals={:?}", 
                   uses_tantivy_to_query, indexed_quals);

        // If we found any search operators, we proceed with the unified approach
        if uses_tantivy_to_query {
            debug_log!(
                "🔍 [EXTRACT_QUALS] Found search operators, proceeding with unified approach"
            );

            // Extract the entire expression as a node string for heap filtering
            // This ensures we preserve the complete boolean logic
            if let Some(node_string) = extract_restrictinfo_string(&restrict_info, rti, root) {
                debug_log!(
                    "🔍 [EXTRACT_QUALS] Extracted node string for heap filter: {}",
                    node_string
                );
                builder
                    .custom_private()
                    .set_heap_filter_node_string(Some(node_string));
            } else {
                debug_log!("🔍 [EXTRACT_QUALS] Failed to extract node string for heap filter");
            }

            // Phase 4: Smart base query optimization
            // Create an optimized base query that reduces the document set
            debug_log!("🔍 [EXTRACT_QUALS] Calling optimize_base_query_for_unified_evaluation");
            let optimized_base_qual = optimize_base_query_for_unified_evaluation(
                &indexed_quals,
                &restrict_info,
                root,
                rti,
                pdbopoid,
                ri_type,
                schema,
                builder, // Pass the builder so we can directly set the optimized SearchQueryInput
            );

            debug_log!(
                "🔍 [EXTRACT_QUALS] Optimized base qual: {:?}",
                optimized_base_qual
            );
            return (optimized_base_qual, ri_type, restrict_info);
        }

        debug_log!(
            "🔍 [EXTRACT_QUALS] No search operators in base restrictions, checking join clauses"
        );

        // If we have no search operators, try to extract from join clauses
        let joinri: PgList<pg_sys::RestrictInfo> = PgList::from_pg(builder.args().rel().joininfo);
        debug_log!("🔍 [EXTRACT_QUALS] Join clauses count: {}", joinri.len());

        let mut join_uses_tantivy_to_query = false;
        let (join_indexed_quals, _join_all_quals) = extract_quals_with_non_indexed(
            root,
            rti,
            joinri.as_ptr().cast(),
            anyelement_query_input_opoid(),
            RestrictInfoType::Join,
            schema,
            true, // Always convert external predicates for joins
            &mut join_uses_tantivy_to_query,
        );

        debug_log!("🔍 [EXTRACT_QUALS] Join extraction result: uses_tantivy_to_query={}, indexed_quals={:?}", 
                   join_uses_tantivy_to_query, join_indexed_quals);

        if join_uses_tantivy_to_query {
            debug_log!(
                "🔍 [EXTRACT_QUALS] Found search operators in join clauses, using join quals"
            );
            return (join_indexed_quals, RestrictInfoType::Join, joinri);
        }

        // No search operators found anywhere, can't use custom scan
        debug_log!("🔍 [EXTRACT_QUALS] No search operators found anywhere, returning None");
        (None, ri_type, restrict_info)
    }

    /// Initialize the heap filter expression state using the heap filter node string (if it exists).
    unsafe fn init_heap_filter_expr_state(state: &mut CustomScanStateWrapper<Self>) {
        if state.custom_state().heap_filter_node_string.is_none()
            || state.custom_state().heap_filter_expr_state.is_some()
        {
            return;
        }
        // Initialize ExprState if not already done

        // Create the Expr node from the heap filter node string
        let expr = create_heap_filter_expr(
            state
                .custom_state()
                .heap_filter_node_string
                .as_ref()
                .unwrap(),
        );
        // Initialize the ExprState
        let expr_state = pg_sys::ExecInitExpr(expr, state.planstate());
        state.custom_state_mut().heap_filter_expr_state = Some(expr_state);
    }
}

impl customscan::ExecMethod for PdbScan {
    fn exec_methods() -> *const CustomExecMethods {
        <PdbScan as ParallelQueryCapable>::exec_methods()
    }
}

impl CustomScan for PdbScan {
    const NAME: &'static CStr = c"ParadeDB Scan";

    type State = PdbScanState;
    type PrivateData = PrivateData;

    fn rel_pathlist_callback(
        mut builder: CustomPathBuilder<Self::PrivateData>,
    ) -> Option<pg_sys::CustomPath> {
        // Debug logging to track which table and test is being processed
        let table_name = unsafe {
            let rte = builder.args().rte();
            if rte.relid != pg_sys::InvalidOid {
                match pg_sys::get_rel_name(rte.relid) {
                    name if !name.is_null() => {
                        let name_cstr = std::ffi::CStr::from_ptr(name);
                        name_cstr.to_string_lossy().to_string()
                    }
                    _ => "unknown".to_string(),
                }
            } else {
                "no_table".to_string()
            }
        };

        let restrict_info = unsafe {
            PgList::<pg_sys::RestrictInfo>::from_pg((*builder.args().rel).baserestrictinfo)
        };
        debug_log!("🎯 [PDBSCAN] rel_pathlist_callback called for table={}, rti={}, {} restriction clauses", 
                   table_name, builder.args().rti, restrict_info.len());

        unsafe {
            let (restrict_info, ri_type) = builder.restrict_info();
            debug_log!(
                "🎯 [PDBSCAN] restrict_info type: {:?}, count: {}",
                ri_type,
                restrict_info.len()
            );

            if matches!(ri_type, RestrictInfoType::None) {
                // this relation has no restrictions (WHERE clause predicates), so there's no need
                // for us to do anything
                debug_log!("🎯 [PDBSCAN] No restrictions found, returning None");
                return None;
            }

            let rti = builder.args().rti;
            let (table, bm25_index) = {
                let rte = builder.args().rte();

                // we only support plain relation and join rte's
                if rte.rtekind != pg_sys::RTEKind::RTE_RELATION
                    && rte.rtekind != pg_sys::RTEKind::RTE_JOIN
                {
                    debug_log!(
                        "🎯 [PDBSCAN] Unsupported RTE kind: {:?}, returning None",
                        rte.rtekind
                    );
                    return None;
                }

                // and we only work on plain relations
                let relkind = pg_sys::get_rel_relkind(rte.relid) as u8;
                if relkind != pg_sys::RELKIND_RELATION && relkind != pg_sys::RELKIND_MATVIEW {
                    debug_log!(
                        "🎯 [PDBSCAN] Unsupported relation kind: {}, returning None",
                        relkind
                    );
                    return None;
                }

                // and that relation must have a `USING bm25` index
                let (table, bm25_index) = match rel_get_bm25_index(rte.relid) {
                    Some(result) => {
                        debug_log!("🎯 [PDBSCAN] Found BM25 index for table {}", table_name);
                        result
                    }
                    None => {
                        debug_log!(
                            "🎯 [PDBSCAN] No BM25 index found for table {}, returning None",
                            table_name
                        );
                        return None;
                    }
                };

                (table, bm25_index)
            };

            let root = builder.args().root;
            let rel = builder.args().rel;

            let directory = MVCCDirectory::snapshot(bm25_index.oid());
            let index = Index::open(directory).expect("custom_scan: should be able to open index");
            let schema = SearchIndexSchema::open(bm25_index.oid())
                .expect("custom_scan: should be able to open schema");
            let pathkey = pullup_orderby_pathkey(&mut builder, rti, &schema, root);

            #[cfg(any(feature = "pg14", feature = "pg15"))]
            let baserels = (*builder.args().root).all_baserels;
            #[cfg(any(feature = "pg16", feature = "pg17"))]
            let baserels = (*builder.args().root).all_query_rels;

            let limit = if (*builder.args().root).limit_tuples > -1.0 {
                // Check if this is a single relation or a partitioned table setup
                let rel_is_single_or_partitioned = pg_sys::bms_equal((*rel).relids, baserels)
                    || is_partitioned_table_setup(builder.args().root, (*rel).relids, baserels);

                if rel_is_single_or_partitioned {
                    // We can use the limit for estimates if:
                    // a) we have a limit, and
                    // b) we're querying a single relation OR partitions of a partitioned table
                    Some((*builder.args().root).limit_tuples)
                } else {
                    None
                }
            } else {
                None
            };

            // quick look at the target list to see if we might need to do our const projections
            let target_list = (*(*builder.args().root).parse).targetList;
            let maybe_needs_const_projections = maybe_needs_const_projections(target_list.cast());

            // Get all columns referenced by this RTE throughout the entire query
            let referenced_columns = collect_maybe_fast_field_referenced_columns(rti, rel);

            // Save the count of referenced columns for decision-making
            builder
                .custom_private()
                .set_referenced_columns_count(referenced_columns.len());

            let is_topn = limit.is_some() && pathkey.is_some();

            // When collecting which_fast_fields, analyze the entire set of referenced columns,
            // not just those in the target list. To avoid execution-time surprises, the "planned"
            // fast fields must be a superset of the fast fields which are extracted from the
            // execution-time target list: see `assign_exec_method` for more info.
            builder.custom_private().set_planned_which_fast_fields(
                exec_methods::fast_fields::collect_fast_fields(
                    target_list,
                    &referenced_columns,
                    rti,
                    &schema,
                    &table,
                    false,
                )
                .into_iter()
                .collect(),
            );
            let maybe_ff = builder.custom_private().maybe_ff();

            //
            // look for quals we can support
            //
            debug_log!(
                "🎯 [PDBSCAN] About to extract quals for table {}",
                table_name
            );
            let (quals, ri_type, restrict_info) = Self::extract_all_possible_quals(
                &mut builder,
                root,
                rti,
                restrict_info,
                anyelement_query_input_opoid(),
                ri_type,
                &schema,
            );

            let Some(quals) = quals else {
                // if we are not able to push down all of the quals, then do not propose the custom
                // scan, as that would mean executing filtering against heap tuples (which amounts
                // to a join, and would require more planning).
                debug_log!(
                    "🎯 [PDBSCAN] No quals extracted for table={}, returning None (no custom scan)",
                    table_name
                );
                return None;
            };

            debug_log!(
                "🎯 [PDBSCAN] Successfully extracted quals for table {}: {:?}",
                table_name,
                quals
            );
            let query = SearchQueryInput::from(&quals);
            debug_log!(
                "🎯 [PDBSCAN] Generated SearchQueryInput for table {}: {:?}",
                table_name,
                query
            );

            let norm_selec = if restrict_info.len() == 1 {
                (*restrict_info.get_ptr(0).unwrap()).norm_selec
            } else {
                UNASSIGNED_SELECTIVITY
            };
            debug_log!(
                "🎯 [PDBSCAN] norm_selec for table {}: {}",
                table_name,
                norm_selec
            );

            let mut selectivity = if let Some(limit) = limit {
                // use the limit
                let sel = limit
                    / table
                        .reltuples()
                        .map(|n| n as Cardinality)
                        .unwrap_or(UNKNOWN_SELECTIVITY);
                debug_log!(
                    "🎯 [PDBSCAN] Using limit-based selectivity for table {}: {}",
                    table_name,
                    sel
                );
                sel
            } else if norm_selec != UNASSIGNED_SELECTIVITY {
                // we can use the norm_selec that already happened
                debug_log!(
                    "🎯 [PDBSCAN] Using norm_selec for table {}: {}",
                    table_name,
                    norm_selec
                );
                norm_selec
            } else if quals.contains_external_var() {
                // if the query has external vars (references to other relations which decide whether the rows in this
                // relation are visible) then we end up returning *everything* from _this_ relation
                debug_log!(
                    "🎯 [PDBSCAN] Using full relation selectivity for table {} (external vars)",
                    table_name
                );
                FULL_RELATION_SELECTIVITY
            } else if quals.contains_exprs() {
                // if the query has expressions then it's parameterized and we have to guess something
                debug_log!(
                    "🎯 [PDBSCAN] Using parameterized selectivity for table {} (expressions)",
                    table_name
                );
                PARAMETERIZED_SELECTIVITY
            } else {
                // ask the index
                let sel = estimate_selectivity(&bm25_index, &query).unwrap_or(UNKNOWN_SELECTIVITY);
                debug_log!(
                    "🎯 [PDBSCAN] Using index-estimated selectivity for table {}: {}",
                    table_name,
                    sel
                );
                sel
            };

            // we must use this path if we need to do const projections for scores or snippets
            let force_path = maybe_needs_const_projections || is_topn || quals.contains_all();
            debug_log!("🎯 [PDBSCAN] Force path decision for table {}: {} (const_proj={}, is_topn={}, contains_all={})", 
                       table_name, force_path, maybe_needs_const_projections, is_topn, quals.contains_all());
            builder = builder.set_force_path(force_path);

            builder.custom_private().set_heaprelid(table.oid());
            builder.custom_private().set_indexrelid(bm25_index.oid());
            builder.custom_private().set_range_table_index(rti);
            builder.custom_private().set_query(query);
            builder.custom_private().set_limit(limit);
            builder.custom_private().set_segment_count(
                index
                    .searchable_segments()
                    .map(|segments| segments.len())
                    .unwrap_or(0),
            );

            if is_topn && pathkey.is_some() {
                let pathkey = pathkey.as_ref().unwrap();
                // sorting by a field only works if we're not doing const projections
                // the reason for this is that tantivy can't do both scoring and ordering by
                // a fast field at the same time.
                //
                // and sorting by score always works
                match (maybe_needs_const_projections, pathkey) {
                    (false, OrderByStyle::Field(..)) => {
                        builder.custom_private().set_sort_info(pathkey);
                    }
                    (_, OrderByStyle::Score(..)) => {
                        builder.custom_private().set_sort_info(pathkey);
                    }
                    _ => {}
                }
            } else if limit.is_some()
                && PgList::<pg_sys::PathKey>::from_pg((*builder.args().root).query_pathkeys)
                    .is_empty()
            {
                // we have a limit but no order by, so record that.  this will let us go through
                // our "top n" machinery, but getting "limit" (essentially) random docs, which
                // is what the user asked for
                builder
                    .custom_private()
                    .set_sort_direction(Some(SortDirection::None));
            }

            let nworkers = if (*builder.args().rel).consider_parallel {
                let segment_count = index.searchable_segments().unwrap_or_default().len();
                compute_nworkers(limit, segment_count, builder.custom_private().is_sorted())
            } else {
                0
            };

            // TODO: Re-examine this `is_string_fast_field_capable` check after #2612 has landed,
            // as it should likely be checking for `is_mixed_fast_field_capable` as well, and
            // should probably have different thresholds.
            // See https://github.com/paradedb/paradedb/issues/2620
            if pathkey.is_some()
                && !is_topn
                && fast_fields::is_string_fast_field_capable(builder.custom_private()).is_some()
            {
                let pathkey = pathkey.as_ref().unwrap();

                // we're going to do a StringAgg, and it may or may not be more efficient to use
                // parallel queries, depending on the cardinality of what we're going to select
                let parallel_scan_preferred = || -> bool {
                    let cardinality = {
                        let estimate = if let OrderByStyle::Field(_, field) = &pathkey {
                            // NB:  '4' is a magic number
                            fast_fields::estimate_cardinality(&bm25_index, field).unwrap_or(0) * 4
                        } else {
                            0
                        };
                        estimate as f64 * selectivity
                    };

                    let pathkey_cnt =
                        PgList::<pg_sys::PathKey>::from_pg((*builder.args().root).query_pathkeys)
                            .len();

                    // if we only have 1 path key or if our estimated cardinality is over some
                    // hardcoded value, it's seemingly more efficient to do a parallel scan
                    pathkey_cnt == 1 || cardinality > 1_000_000.0
                };

                if nworkers > 0 && parallel_scan_preferred() {
                    // If we use parallel workers, there is no point in sorting, because the
                    // plan will already need to sort and merge the outputs from the workers.
                    // See the TODO below about being able to claim sorting for parallel
                    // workers.
                    builder = builder.set_parallel(nworkers);
                } else {
                    // otherwise we'll do a regular scan
                    builder.custom_private().set_sort_info(pathkey);
                }
            } else if !quals.contains_external_var() && nworkers > 0 {
                builder = builder.set_parallel(nworkers);
            }

            let exec_method_type = choose_exec_method(builder.custom_private());
            builder
                .custom_private()
                .set_exec_method_type(exec_method_type);

            // Once we have chosen an execution method type, we have a final determination of the
            // properties of the output, and can make claims about whether it is sorted.
            if builder.custom_private().exec_method_type().is_sorted() {
                if let Some(pathkey) = pathkey.as_ref() {
                    builder = builder.add_path_key(pathkey);
                }
            }

            //
            // finally, we have enough information to set the cost and estimation information
            //

            if builder.is_parallel() {
                // if we're likely to do a parallel scan, divide the selectivity up by the number of
                // workers we're likely to use.  this lets Postgres make better decisions based on what
                // an individual parallel scan is actually going to return
                selectivity /= (nworkers
                    + if pg_sys::parallel_leader_participation {
                        1
                    } else {
                        0
                    }) as f64;
            }

            let reltuples = table.reltuples().unwrap_or(1.0) as f64;
            let rows = (reltuples * selectivity).max(1.0);
            let per_tuple_cost = {
                if maybe_ff {
                    // returning fields from fast fields
                    pg_sys::cpu_index_tuple_cost
                } else {
                    // requires heap access to return fields
                    pg_sys::cpu_tuple_cost
                }
            };

            let startup_cost = DEFAULT_STARTUP_COST;
            let total_cost = startup_cost + (rows * per_tuple_cost);

            debug_log!("🎯 [PDBSCAN] Path costs for table {}: startup={}, total={}, rows={}, per_tuple={}, selectivity={}", 
                       table_name, startup_cost, total_cost, rows, per_tuple_cost, selectivity);

            builder = builder.set_rows(rows);
            builder = builder.set_startup_cost(startup_cost);
            builder = builder.set_total_cost(total_cost);

            // indicate that we'll be doing projection ourselves
            builder = builder.set_flag(Flags::Projection);

            let path = builder.build();
            debug_log!(
                "🎯 [PDBSCAN] Successfully built custom path for table {}",
                table_name
            );
            Some(path)
        }
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self::PrivateData>) -> pg_sys::CustomScan {
        unsafe {
            let mut tlist = PgList::<pg_sys::TargetEntry>::from_pg(builder.args().tlist.as_ptr());

            // Store the length of the target list
            builder
                .custom_private_mut()
                .set_target_list_len(Some(tlist.len()));

            let private_data = builder.custom_private();

            let rti: i32 = private_data
                .range_table_index()
                .expect("range table index should have been set")
                .try_into()
                .expect("range table index should not be negative");
            let processed_tlist =
                PgList::<pg_sys::TargetEntry>::from_pg((*builder.args().root).processed_tlist);

            let mut attname_lookup = HashMap::default();
            let score_funcoid = score_funcoid();
            let snippet_funcoid = snippet_funcoid();
            let snippet_positions_funcoid = snippet_positions_funcoid();
            for te in processed_tlist.iter_ptr() {
                let func_vars_at_level = pullout_funcexprs(
                    te.cast(),
                    &[score_funcoid, snippet_funcoid, snippet_positions_funcoid],
                    rti,
                    builder.args().root,
                );

                for (funcexpr, var, attname) in func_vars_at_level {
                    // if we have a tlist, then we need to add the specific function that uses
                    // a Var at our level to that tlist.
                    //
                    // if we don't have a tlist (it's empty), then that means Postgres will later
                    // give us everything we need

                    if !tlist.is_empty() {
                        let te = pg_sys::copyObjectImpl(te.cast()).cast::<pg_sys::TargetEntry>();
                        (*te).resno = (tlist.len() + 1) as _;
                        (*te).expr = funcexpr.cast();

                        tlist.push(te);
                    }

                    // track a triplet of (varno, varattno, attname) as 3 individual
                    // entries in the `attname_lookup` List
                    attname_lookup.insert(((*var).varno, (*var).varattno), attname);
                }
            }

            // Extract join-level snippet predicates for this relation
            // Get values we need before the mutable borrow

            // Extract the indexrelid early to avoid borrow checker issues later
            let indexrelid = private_data.indexrelid().expect("indexrelid should be set");
            let indexrel = PgRelation::with_lock(indexrelid, pg_sys::AccessShareLock as _);
            let directory = MVCCDirectory::snapshot(indexrel.oid());
            let index = Index::open(directory)
                .expect("should be able to open index for snippet extraction");
            let schema = SearchIndexSchema::open(indexrelid)
                .expect("should be able to open schema for snippet extraction");

            let base_query = builder
                .custom_private()
                .query()
                .clone()
                .expect("should have a SearchQueryInput");
            let join_predicates = extract_join_predicates(
                builder.args().root,
                rti as pg_sys::Index,
                anyelement_query_input_opoid(),
                &schema,
                &base_query,
            );

            builder
                .custom_private_mut()
                .set_join_predicates(join_predicates);

            builder
                .custom_private_mut()
                .set_var_attname_lookup(attname_lookup);
            builder.build()
        }
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        debug_log!("🎯 [PDBSCAN] Creating custom scan state");
        unsafe {
            builder.custom_state().heaprelid = builder
                .custom_private()
                .heaprelid()
                .expect("heaprelid should have a value");
            builder.custom_state().indexrelid = builder
                .custom_private()
                .indexrelid()
                .expect("indexrelid should have a value");

            builder.custom_state().execution_rti =
                (*builder.args().cscan).scan.scanrelid as pg_sys::Index;

            builder.custom_state().exec_method_type =
                builder.custom_private().exec_method_type().clone();

            builder.custom_state().targetlist_len = builder.target_list().len();

            // information about if we're sorted by score and our limit
            builder.custom_state().limit = builder.custom_private().limit();
            builder.custom_state().sort_field = builder.custom_private().sort_field();
            builder.custom_state().sort_direction = builder.custom_private().sort_direction();

            builder.custom_state().segment_count = builder.custom_private().segment_count();
            builder.custom_state().var_attname_lookup = builder
                .custom_private()
                .var_attname_lookup()
                .as_ref()
                .cloned()
                .expect("should have an attribute name lookup");

            let score_funcoid = score_funcoid();
            let snippet_funcoid = snippet_funcoid();
            let snippet_positions_funcoid = snippet_positions_funcoid();

            builder.custom_state().score_funcoid = score_funcoid;
            builder.custom_state().snippet_funcoid = snippet_funcoid;
            builder.custom_state().snippet_positions_funcoid = snippet_positions_funcoid;
            builder.custom_state().need_scores = uses_scores(
                builder.target_list().as_ptr().cast(),
                score_funcoid,
                builder.custom_state().execution_rti,
            );

            builder.custom_state().heap_filter_node_string =
                builder.custom_private().heap_filter_node_string().clone();

            // Store join snippet predicates in the scan state
            builder.custom_state().join_predicates =
                builder.custom_private().join_predicates().clone();

            // store our query into our custom state too
            let base_query = builder
                .custom_private()
                .query()
                .clone()
                .expect("should have a SearchQueryInput");
            builder
                .custom_state()
                .set_base_search_query_input(base_query);

            if builder.custom_state().need_scores {
                let state = builder.custom_state();
                // Pre-compute enhanced score query if we have join predicates that could affect scoring
                let mut enhanced_score_query = None;
                if let Some(ref join_predicate) = state.join_predicates {
                    // Check the ORIGINAL base query for this relation, not the modified search_query_input
                    // which may contain simplified join predicates from other relations
                    let original_base_query = state.base_search_query_input();

                    // Only enhance scoring if the base query doesn't already have search predicates
                    // If base query has @@@ conditions, it already provides scoring context
                    if !base_query_has_search_predicates(original_base_query, state.indexrelid) {
                        // Combine base query with join predicate using Boolean structure
                        // This provides enhanced search context for scoring while maintaining
                        // the same filtering behavior as the base query
                        enhanced_score_query = Some(SearchQueryInput::Boolean {
                            must: vec![original_base_query.clone()],
                            should: vec![join_predicate.clone()],
                            must_not: vec![],
                        });
                    }
                }

                // Store enhanced score query for use during search execution
                // This will be None for single-table queries, which is correct
                if let Some(enhanced_score_query) = enhanced_score_query {
                    builder
                        .custom_state()
                        .set_base_search_query_input(enhanced_score_query);
                }
            }

            let node = builder.target_list().as_ptr().cast();
            builder.custom_state().planning_rti = builder
                .custom_private()
                .range_table_index()
                .expect("range table index should have been set");
            builder.custom_state().snippet_generators = uses_snippets(
                builder.custom_state().planning_rti,
                &builder.custom_state().var_attname_lookup,
                node,
                snippet_funcoid,
                snippet_positions_funcoid,
            )
            .into_iter()
            .map(|field| (field, None))
            .collect();

            assign_exec_method(&mut builder);

            builder.build()
        }
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    ) {
        explainer.add_text("Table", state.custom_state().heaprelname());
        explainer.add_text("Index", state.custom_state().indexrelname());
        if explainer.is_costs() {
            explainer.add_unsigned_integer(
                "Segment Count",
                state.custom_state().segment_count as u64,
                None,
            );
        }

        if explainer.is_analyze() {
            explainer.add_unsigned_integer(
                "Heap Fetches",
                state.custom_state().heap_tuple_check_count as u64,
                None,
            );
            if explainer.is_verbose() {
                explainer.add_unsigned_integer(
                    "Virtual Tuples",
                    state.custom_state().virtual_tuple_count as u64,
                    None,
                );
                explainer.add_unsigned_integer(
                    "Invisible Tuples",
                    state.custom_state().invisible_tuple_count as u64,
                    None,
                );
            }
        }

        explainer.add_text(
            "Exec Method",
            state
                .custom_state()
                .exec_method_name()
                .split("::")
                .last()
                .unwrap(),
        );
        exec_methods::fast_fields::explain(state, explainer);

        explainer.add_bool("Scores", state.custom_state().need_scores());
        if state.custom_state().is_sorted() {
            if let Some(sort_field) = &state.custom_state().sort_field {
                explainer.add_text("   Sort Field", sort_field);
            } else {
                explainer.add_text("   Sort Field", "paradedb.score()");
            }
            explainer.add_text(
                "   Sort Direction",
                state
                    .custom_state()
                    .sort_direction
                    .unwrap_or(SortDirection::Asc),
            );
        }

        if let Some(limit) = state.custom_state().limit {
            explainer.add_unsigned_integer("   Top N Limit", limit as u64, None);
            if explainer.is_analyze() {
                explainer.add_unsigned_integer(
                    "   Queries",
                    state.custom_state().query_count as u64,
                    None,
                );
            }
        }

        // Show heap filter expression if present using proper deparse context
        if let Some(ref heap_filter_node_string) = state.custom_state().heap_filter_node_string {
            unsafe {
                let human_readable_filter = create_human_readable_filter_text(
                    heap_filter_node_string,
                    explainer,
                    state.custom_state().execution_rti,
                );
                explainer.add_text("Heap Filter", &human_readable_filter);
            }
        }

        let mut json_value = state
            .custom_state()
            .query_to_json()
            .expect("query should serialize to json");
        // Remove the oid from the with_index object
        // This helps to reduce the variability of the explain output used in regression tests
        Self::cleanup_varibilities_from_tantivy_query(&mut json_value);
        let updated_json_query =
            serde_json::to_string(&json_value).expect("updated query should serialize to json");
        explainer.add_text("Tantivy Query", &updated_json_query);

        if explainer.is_verbose() {
            explainer.add_text(
                "Human Readable Query",
                state.custom_state().human_readable_query_string(),
            );
        }
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        debug_log!("🎯 [PDBSCAN] Beginning custom scan");
        unsafe {
            // open the heap and index relations with the proper locks
            let rte = pg_sys::exec_rt_fetch(state.custom_state().execution_rti, estate);
            assert!(!rte.is_null());
            let lockmode = (*rte).rellockmode as pg_sys::LOCKMODE;

            let (heaprel, indexrel) = if lockmode == pg_sys::NoLock as pg_sys::LOCKMODE {
                (
                    pg_sys::RelationIdGetRelation(state.custom_state().heaprelid),
                    pg_sys::RelationIdGetRelation(state.custom_state().indexrelid),
                )
            } else {
                (
                    pg_sys::relation_open(state.custom_state().heaprelid, lockmode),
                    pg_sys::relation_open(state.custom_state().indexrelid, lockmode),
                )
            };

            state.custom_state_mut().heaprel = Some(heaprel);
            state.custom_state_mut().indexrel = Some(indexrel);
            state.custom_state_mut().lockmode = lockmode;

            state.custom_state_mut().heaprel_namespace =
                PgRelation::from_pg(heaprel).namespace().to_string();
            state.custom_state_mut().heaprel_relname =
                PgRelation::from_pg(heaprel).name().to_string();

            if eflags & (pg_sys::EXEC_FLAG_EXPLAIN_ONLY as i32) != 0 {
                // don't do anything else if we're only explaining the query
                return;
            }

            // setup the structures we need to do mvcc checking
            state.custom_state_mut().visibility_checker = Some(
                VisibilityChecker::with_rel_and_snap(heaprel, pg_sys::GetActiveSnapshot()),
            );

            // and finally, get the custom scan itself properly initialized
            let tupdesc = state.custom_state().heaptupdesc();
            pg_sys::ExecInitScanTupleSlot(
                estate,
                addr_of_mut!(state.csstate.ss),
                tupdesc,
                pg_sys::table_slot_callbacks(state.custom_state().heaprel()),
            );
            pg_sys::ExecInitResultTypeTL(addr_of_mut!(state.csstate.ss.ps));
            pg_sys::ExecAssignProjectionInfo(
                state.planstate(),
                (*state.csstate.ss.ss_ScanTupleSlot).tts_tupleDescriptor,
            );

            if state.custom_state_mut().has_postgres_expressions() {
                // we have some runtime Postgres expressions that need to be evaluated in `rescan_custom_scan`
                //
                // Our planstate's ExprContext isn't sufficiently configured for that, so we need to
                // make a new one and swap some pointers around

                // hold onto the planstate's current ExprContext
                let planstate = state.planstate();
                let stdecontext = (*planstate).ps_ExprContext;

                // assign a new one
                pg_sys::ExecAssignExprContext(estate, planstate);

                // take that one and assign it to our state's `runtime_context`.  This is what
                // will be used during `rescan_custom_state` to evaluate expressions
                state.runtime_context = state.csstate.ss.ps.ps_ExprContext;

                // and restore our planstate's original ExprContext
                (*planstate).ps_ExprContext = stdecontext;
            }
        }
    }

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        Self::init_search_reader(state);
        state.custom_state_mut().reset();
    }

    #[allow(clippy::blocks_in_conditions)]
    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        if state.custom_state().search_reader.is_none() {
            Self::init_search_reader(state);
        }

        unsafe {
            Self::init_heap_filter_expr_state(state);
        }

        loop {
            let exec_method = state.custom_state_mut().exec_method_mut();

            // get the next matching document from our search results and look for it in the heap
            match exec_method.next(state.custom_state_mut()) {
                // reached the end of the SearchResults
                ExecState::Eof => {
                    return std::ptr::null_mut();
                }

                // SearchResults found a match
                ExecState::RequiresVisibilityCheck {
                    ctid,
                    score,
                    doc_address,
                } => {
                    unsafe {
                        let slot = match check_visibility(state, ctid, state.scanslot().cast()) {
                            // the ctid is visible
                            Some(slot) => {
                                exec_method.increment_visible();
                                state.custom_state_mut().heap_tuple_check_count += 1;
                                slot
                            }

                            // the ctid is not visible
                            None => {
                                state.custom_state_mut().invisible_tuple_count += 1;
                                continue;
                            }
                        };

                        // Apply enhanced heap filtering with unified expression evaluation
                        let filter_result = {
                            // Get expr_context first to avoid borrowing conflicts
                            let expr_context = (*state.planstate()).ps_ExprContext;

                            // Get all other values we need upfront to avoid borrowing conflicts
                            let heap_filter_node_string =
                                state.custom_state().heap_filter_node_string.clone();
                            let has_search_reader = state.custom_state().search_reader.is_some();
                            let has_indexrel = state.custom_state().indexrel.is_some();

                            debug_log!(
                                "🔍 [DEBUG] Checking heap filter conditions for doc_id: {}",
                                doc_address.doc_id
                            );
                            debug_log!(
                                "🔍 [DEBUG] heap_filter_node_string: {:?}",
                                heap_filter_node_string
                            );
                            debug_log!("🔍 [DEBUG] search_reader available: {}", has_search_reader);

                            if let Some(heap_filter_node_string) = heap_filter_node_string {
                                if has_search_reader && has_indexrel {
                                    debug_log!("🚀 [DEBUG] Using unified heap filter with node string length: {}", heap_filter_node_string.len());

                                    // Get the indexrel OID safely
                                    let indexrel_oid =
                                        *state.custom_state().indexrel.as_ref().unwrap();

                                    // Now we can safely create the schema without borrowing conflicts
                                    let indexrel = PgRelation::from_pg(indexrel_oid);
                                    let schema = SearchIndexSchema::open(indexrel.oid())
                                        .expect("should be able to open schema");

                                    // Get the search_reader safely
                                    let search_reader =
                                        state.custom_state().search_reader.as_ref().unwrap();

                                    let result = apply_complete_unified_heap_filter(
                                        search_reader,
                                        &schema,
                                        &heap_filter_node_string,
                                        expr_context,
                                        slot,
                                        doc_address.doc_id,
                                        doc_address,
                                        score,
                                    );

                                    result
                                        .unwrap_or_else(|_| UnifiedEvaluationResult::no_match())
                                        .into()
                                } else {
                                    debug_log!("⚠️  [DEBUG] No search_reader/indexrel available, falling back to enhanced heap filter");
                                    apply_enhanced_heap_filter(state, slot, score, doc_address)
                                }
                            } else {
                                debug_log!("⚠️  [DEBUG] No heap_filter_node_string available, falling back to enhanced heap filter");
                                apply_enhanced_heap_filter(state, slot, score, doc_address)
                            }
                        };

                        if !filter_result.matches {
                            // Tuple doesn't pass heap filter, skip it
                            continue;
                        }

                        // Use the enhanced score from the unified evaluator
                        let enhanced_score = filter_result.score;

                        if !state.custom_state().need_scores()
                            && !state.custom_state().need_snippets()
                        {
                            //
                            // we don't need scores or snippets
                            // do the projection and return
                            //

                            (*(*state.projection_info()).pi_exprContext).ecxt_scantuple = slot;
                            return pg_sys::ExecProject(state.projection_info());
                        } else {
                            //
                            // we do need scores or snippets
                            //
                            // replace their placeholder values and then rebuild the ProjectionInfo
                            // and project it
                            //

                            let mut per_tuple_context = PgMemoryContexts::For(
                                (*(*state.projection_info()).pi_exprContext).ecxt_per_tuple_memory,
                            );
                            per_tuple_context.reset();

                            if state.custom_state().need_scores() {
                                let const_score_node = state
                                    .custom_state()
                                    .const_score_node
                                    .expect("const_score_node should be set");
                                (*const_score_node).constvalue =
                                    enhanced_score.into_datum().unwrap();
                                (*const_score_node).constisnull = false;
                            }

                            if state.custom_state().need_snippets() {
                                per_tuple_context.switch_to(|_| {
                                    for (snippet_type, const_snippet_nodes) in
                                        &state.custom_state().const_snippet_nodes
                                    {
                                        match snippet_type {
                                            SnippetType::Text(_, _, config) => {
                                                let snippet = state
                                                    .custom_state()
                                                    .make_snippet(ctid, snippet_type);

                                                for const_ in const_snippet_nodes {
                                                    match &snippet {
                                                        Some(text) => {
                                                            (**const_).constvalue =
                                                                text.into_datum().unwrap();
                                                            (**const_).constisnull = false;
                                                        }
                                                        None => {
                                                            (**const_).constvalue =
                                                                pg_sys::Datum::null();
                                                            (**const_).constisnull = true;
                                                        }
                                                    }
                                                }
                                            }
                                            SnippetType::Positions(..) => {
                                                let positions = state
                                                    .custom_state()
                                                    .get_snippet_positions(ctid, snippet_type);

                                                for const_ in const_snippet_nodes {
                                                    match &positions {
                                                        Some(positions) => {
                                                            (**const_).constvalue = positions
                                                                .clone()
                                                                .into_datum()
                                                                .unwrap();
                                                            (**const_).constisnull = false;
                                                        }
                                                        None => {
                                                            (**const_).constvalue =
                                                                pg_sys::Datum::null();
                                                            (**const_).constisnull = true;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });
                            }

                            // finally, do the projection
                            return per_tuple_context.switch_to(|_| {
                                let planstate = state.planstate();

                                (*(*state.projection_info()).pi_exprContext).ecxt_scantuple = slot;
                                let proj_info = pg_sys::ExecBuildProjectionInfo(
                                    state
                                        .custom_state()
                                        .placeholder_targetlist
                                        .expect("placeholder_targetlist must be set"),
                                    (*planstate).ps_ExprContext,
                                    (*planstate).ps_ResultTupleSlot,
                                    planstate,
                                    (*state.csstate.ss.ss_ScanTupleSlot).tts_tupleDescriptor,
                                );
                                pg_sys::ExecProject(proj_info)
                            });
                        }
                    }
                }

                ExecState::Virtual { slot } => unsafe {
                    state.custom_state_mut().virtual_tuple_count += 1;

                    // Apply enhanced heap filtering - for virtual tuples, we don't have a specific score
                    let filter_result =
                        apply_enhanced_heap_filter(state, slot, 1.0, DocAddress::new(0, 0));
                    if !filter_result.matches {
                        // Tuple doesn't pass heap filter, skip it
                        continue;
                    }

                    return slot;
                },
            }
        }
    }

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>) {}

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        // get some things dropped now
        drop(state.custom_state_mut().visibility_checker.take());
        drop(state.custom_state_mut().search_reader.take());
        drop(std::mem::take(
            &mut state.custom_state_mut().snippet_generators,
        ));
        drop(std::mem::take(&mut state.custom_state_mut().search_results));

        if let Some(heaprel) = state.custom_state_mut().heaprel.take() {
            unsafe {
                pg_sys::relation_close(heaprel, state.custom_state().lockmode);
            }
        }
        if let Some(indexrel) = state.custom_state_mut().indexrel.take() {
            unsafe {
                pg_sys::relation_close(indexrel, state.custom_state().lockmode);
            }
        }
    }
}

///
/// Choose and return an ExecMethodType based on the properties of the builder at planning time.
///
/// If the query can return "fast fields", make that determination here, falling back to the
/// [`NormalScanExecState`] if not.
///
/// We support [`StringFastFieldExecState`] when there's 1 fast field and it's a string, or
/// [`NumericFastFieldExecState`] when there's one or more numeric fast fields, or
/// [`MixedFastFieldExecState`] when there are multiple string fast fields or a mix of string
/// and numeric fast fields.
///
/// If we have failed to extract all relevant information at planning time, then the fast-field
/// execution methods might still fall back to `Normal` at execution time: see the notes in
/// `assign_exec_method` and `compute_exec_which_fast_fields`.
///
/// `paradedb.score()`, `ctid`, and `tableoid` are considered fast fields for the purposes of
/// these specialized [`ExecMethod`]s.
///
fn choose_exec_method(privdata: &PrivateData) -> ExecMethodType {
    if let Some((limit, sort_direction)) = privdata.limit().zip(privdata.sort_direction()) {
        // having a valid limit and sort direction means we can do a TopN query
        // and TopN can do snippets
        ExecMethodType::TopN {
            heaprelid: privdata.heaprelid().expect("heaprelid must be set"),
            limit,
            sort_direction,
            need_scores: privdata.need_scores(),
        }
    } else if fast_fields::is_numeric_fast_field_capable(privdata) {
        // Check for numeric-only fast fields first because they're more selective
        ExecMethodType::FastFieldNumeric {
            which_fast_fields: privdata.planned_which_fast_fields().clone().unwrap(),
        }
    } else if let Some(field) = fast_fields::is_string_fast_field_capable(privdata) {
        ExecMethodType::FastFieldString {
            field,
            which_fast_fields: privdata.planned_which_fast_fields().clone().unwrap(),
        }
    } else if fast_fields::is_mixed_fast_field_capable(privdata) {
        ExecMethodType::FastFieldMixed {
            which_fast_fields: privdata.planned_which_fast_fields().clone().unwrap(),
        }
    } else {
        // Fall back to normal execution
        ExecMethodType::Normal
    }
}

///
/// Creates and assigns the execution method which was chosen at planning time.
///
/// If a fast-fields execution method was chosen at planning time, we might still fall back to
/// NormalScanExecState if we fail to extract the superset of fields during planning time which was
/// needed at execution time.
///
fn assign_exec_method(builder: &mut CustomScanStateBuilder<PdbScan, PrivateData>) {
    match builder.custom_state_ref().exec_method_type.clone() {
        ExecMethodType::Normal => builder
            .custom_state()
            .assign_exec_method(NormalScanExecState::default(), Some(ExecMethodType::Normal)),
        ExecMethodType::TopN {
            heaprelid,
            limit,
            sort_direction,
            need_scores,
        } => builder.custom_state().assign_exec_method(
            exec_methods::top_n::TopNScanExecState::new(
                heaprelid,
                limit,
                sort_direction,
                need_scores,
            ),
            None,
        ),
        ExecMethodType::FastFieldString {
            field,
            which_fast_fields,
        } => {
            if let Some(which_fast_fields) =
                compute_exec_which_fast_fields(builder, which_fast_fields)
            {
                builder.custom_state().assign_exec_method(
                    exec_methods::fast_fields::string::StringFastFieldExecState::new(
                        field,
                        which_fast_fields,
                    ),
                    None,
                )
            } else {
                builder.custom_state().assign_exec_method(
                    NormalScanExecState::default(),
                    Some(ExecMethodType::Normal),
                )
            }
        }
        ExecMethodType::FastFieldNumeric { which_fast_fields } => {
            if let Some(which_fast_fields) =
                compute_exec_which_fast_fields(builder, which_fast_fields)
            {
                builder.custom_state().assign_exec_method(
                    exec_methods::fast_fields::numeric::NumericFastFieldExecState::new(
                        which_fast_fields,
                    ),
                    None,
                )
            } else {
                builder.custom_state().assign_exec_method(
                    NormalScanExecState::default(),
                    Some(ExecMethodType::Normal),
                )
            }
        }
        ExecMethodType::FastFieldMixed { which_fast_fields } => {
            if let Some(which_fast_fields) =
                compute_exec_which_fast_fields(builder, which_fast_fields)
            {
                builder.custom_state().assign_exec_method(
                    exec_methods::fast_fields::mixed::MixedFastFieldExecState::new(
                        which_fast_fields,
                    ),
                    None,
                )
            } else {
                builder.custom_state().assign_exec_method(
                    NormalScanExecState::default(),
                    Some(ExecMethodType::Normal),
                )
            }
        }
    }
}

///
/// Computes the execution time `which_fast_fields`, which are validated to be a subset of the
/// planning time `which_fast_fields`. If it's not the case, we return `None` to indicate that
/// we should fall back to the `Normal` execution mode.
///
fn compute_exec_which_fast_fields(
    builder: &mut CustomScanStateBuilder<PdbScan, PrivateData>,
    planned_which_fast_fields: HashSet<WhichFastField>,
) -> Option<Vec<WhichFastField>> {
    let exec_which_fast_fields = unsafe {
        let indexrel = PgRelation::open(builder.custom_state().indexrelid);
        let heaprel = indexrel
            .heap_relation()
            .expect("index should belong to a table");
        let directory = MVCCDirectory::snapshot(indexrel.oid());
        let index =
            Index::open(directory).expect("create_custom_scan_state: should be able to open index");
        let schema = SearchIndexSchema::open(indexrel.oid())
            .expect("custom_scan: should be able to open schema");

        // Calculate the ordered set of fast fields which have actually been requested in
        // the target list.
        //
        // In order for our planned ExecMethodType to be accurate, this must always be a
        // subset of the fast fields which were extracted at planning time.
        exec_methods::fast_fields::collect_fast_fields(
            builder.target_list().as_ptr(),
            // At this point, all fast fields which we need to extract are listed directly
            // in our execution-time target list, so there is no need to extract from other
            // positions.
            &HashSet::default(),
            builder.custom_state().execution_rti,
            &schema,
            &heaprel,
            true,
        )
    };

    if fast_fields::is_all_special_or_junk_fields(&exec_which_fast_fields) {
        // In some cases, enough columns are pruned between planning and execution that there
        // is no point actually using fast fields, and we can fall back to `Normal`.
        //
        // TODO: In order to implement https://github.com/paradedb/paradedb/issues/2623, we will
        // need to differentiate these cases, so that we can always emit the sort order that we
        // claimed.
        return None;
    }

    let missing_fast_fields = exec_which_fast_fields
        .iter()
        .filter(|ff| !planned_which_fast_fields.contains(ff))
        .collect::<Vec<_>>();

    if !missing_fast_fields.is_empty() {
        debug_log!(
            "Failed to extract all fast fields at planning time: \
             was missing {missing_fast_fields:?} from {planned_which_fast_fields:?} \
             Falling back to Normal execution.",
        );
        return None;
    }

    Some(exec_which_fast_fields)
}

/// Use the [`VisibilityChecker`] to lookup the [`SearchIndexScore`] document in the underlying heap
/// and if it exists return a formed [`TupleTableSlot`].
#[inline(always)]
fn check_visibility(
    state: &mut CustomScanStateWrapper<PdbScan>,
    ctid: u64,
    bslot: *mut pg_sys::BufferHeapTupleTableSlot,
) -> Option<*mut pg_sys::TupleTableSlot> {
    state
        .custom_state_mut()
        .visibility_checker()
        .exec_if_visible(ctid, bslot.cast(), move |heaprel| bslot.cast())
}

/// Apply heap filtering using PostgreSQL's built-in expression evaluation
/// Returns true if the tuple passes all restrictions, false otherwise
#[inline(always)]
unsafe fn apply_heap_filter(
    state: &mut CustomScanStateWrapper<PdbScan>,
    slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    if state.custom_state().heap_filter_expr_state.is_none() {
        return true;
    }

    // Evaluate the expression if we have an ExprState
    let expr_state = state.custom_state().heap_filter_expr_state.unwrap();
    let expr_context = (*state.planstate()).ps_ExprContext;

    // Set the scan tuple in the expression context
    (*expr_context).ecxt_scantuple = slot;

    let mut isnull = false;
    let result = pg_sys::ExecEvalExpr(expr_state, expr_context, &mut isnull);

    if isnull {
        // NULL result means the tuple doesn't pass the filter
        false
    } else {
        // Convert the result to a boolean - simple and direct
        bool::from_datum(result, false).unwrap_or(false)
    }
}

unsafe fn inject_score_and_snippet_placeholders(state: &mut CustomScanStateWrapper<PdbScan>) {
    if !state.custom_state().need_scores() && !state.custom_state().need_snippets() {
        // scores/snippets aren't necessary so we use whatever we originally setup as our ProjectionInfo
        return;
    }

    // inject score and/or snippet placeholder [`pg_sys::Const`] nodes into what is a copy of the Plan's
    // targetlist.  We store this in our custom state's "placeholder_targetlist" for use during the
    // forced projection we must do later.
    let planstate = state.planstate();

    let (targetlist, const_score_node, const_snippet_nodes) = inject_placeholders(
        (*(*planstate).plan).targetlist,
        state.custom_state().planning_rti,
        state.custom_state().score_funcoid,
        state.custom_state().snippet_funcoid,
        state.custom_state().snippet_positions_funcoid,
        &state.custom_state().var_attname_lookup,
        &state.custom_state().snippet_generators,
    );

    state.custom_state_mut().placeholder_targetlist = Some(targetlist);
    state.custom_state_mut().const_score_node = Some(const_score_node);
    state.custom_state_mut().const_snippet_nodes = const_snippet_nodes;
}

unsafe fn pullup_orderby_pathkey<P: Into<*mut pg_sys::List> + Default>(
    builder: &mut CustomPathBuilder<P>,
    rti: pg_sys::Index,
    schema: &SearchIndexSchema,
    root: *mut pg_sys::PlannerInfo,
) -> Option<OrderByStyle> {
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*builder.args().root).query_pathkeys);

    if let Some(first_pathkey) = pathkeys.get_ptr(0) {
        let equivclass = (*first_pathkey).pk_eclass;
        let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

        for member in members.iter_ptr() {
            let expr = (*member).em_expr;

            if is_score_func(expr.cast(), rti as _) {
                return Some(OrderByStyle::Score(first_pathkey));
            } else if let Some(var) = is_lower_func(expr.cast(), rti as _) {
                let (heaprelid, attno, _) = find_var_relation(var, root);
                let heaprel = PgRelation::with_lock(heaprelid, pg_sys::AccessShareLock as _);
                let tupdesc = heaprel.tuple_desc();
                if let Some(att) = tupdesc.get(attno as usize - 1) {
                    if let Some(search_field) = schema.search_field(att.name()) {
                        if search_field.is_lower_sortable() {
                            return Some(OrderByStyle::Field(first_pathkey, att.name().into()));
                        }
                    }
                }
            } else if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, expr) {
                if let Some(var) = nodecast!(Var, T_Var, (*relabel).arg) {
                    let (heaprelid, attno, _) = find_var_relation(var, root);
                    let heaprel = PgRelation::with_lock(heaprelid, pg_sys::AccessShareLock as _);
                    let tupdesc = heaprel.tuple_desc();
                    if let Some(att) = tupdesc.get(attno as usize - 1) {
                        if let Some(search_field) = schema.search_field(att.name()) {
                            if search_field.is_raw_sortable() {
                                return Some(OrderByStyle::Field(first_pathkey, att.name().into()));
                            }
                        }
                    }
                }
            } else if let Some(var) = nodecast!(Var, T_Var, expr) {
                let (heaprelid, attno, _) = find_var_relation(var, root);
                if heaprelid == pg_sys::Oid::INVALID {
                    return None;
                }
                let heaprel = PgRelation::with_lock(heaprelid, pg_sys::AccessShareLock as _);
                let tupdesc = heaprel.tuple_desc();
                if let Some(att) = tupdesc.get(attno as usize - 1) {
                    if let Some(search_field) = schema.search_field(att.name()) {
                        if search_field.is_raw_sortable() {
                            return Some(OrderByStyle::Field(first_pathkey, att.name().into()));
                        }
                    }
                }
            }
        }
    }
    None
}

unsafe fn is_lower_func(node: *mut pg_sys::Node, rti: i32) -> Option<*mut pg_sys::Var> {
    let funcexpr = nodecast!(FuncExpr, T_FuncExpr, node)?;
    if (*funcexpr).funcid == text_lower_funcoid() {
        let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
        assert!(
            args.len() == 1,
            "`lower(text)` function must have 1 argument"
        );
        if let Some(var) = nodecast!(Var, T_Var, args.get_ptr(0).unwrap()) {
            if (*var).varno as i32 == rti as i32 {
                return Some(var);
            }
        } else if let Some(relabel) =
            nodecast!(RelabelType, T_RelabelType, args.get_ptr(0).unwrap())
        {
            if let Some(var) = nodecast!(Var, T_Var, (*relabel).arg) {
                if (*var).varno as i32 == rti as i32 {
                    return Some(var);
                }
            }
        }
    }

    None
}

pub fn text_lower_funcoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"pg_catalog.lower(text)".into_datum()],
        )
        .expect("the `pg_catalog.lower(text)` function should exist")
    }
}

#[inline(always)]
pub fn is_block_all_visible(
    heaprel: pg_sys::Relation,
    vmbuff: &mut pg_sys::Buffer,
    heap_blockno: pg_sys::BlockNumber,
) -> bool {
    unsafe {
        let status = pg_sys::visibilitymap_get_status(heaprel, heap_blockno, vmbuff);
        status != 0
    }
}

// Helper function to create an iterator over Bitmapset members
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

// Helper function to check if a Bitmapset is empty
unsafe fn bms_is_empty(bms: *mut pg_sys::Bitmapset) -> bool {
    bms_iter(bms).next().is_none()
}

// Helper function to determine if we're dealing with a partitioned table setup
unsafe fn is_partitioned_table_setup(
    root: *mut pg_sys::PlannerInfo,
    rel_relids: *mut pg_sys::Bitmapset,
    baserels: *mut pg_sys::Bitmapset,
) -> bool {
    // If the relation bitmap is empty, early return
    if bms_is_empty(rel_relids) {
        return false;
    }

    // Get the rtable for relkind checks
    let rtable = (*(*root).parse).rtable;

    // For each relation in baserels
    for baserel_idx in bms_iter(baserels) {
        // Skip invalid indices
        if baserel_idx == 0 || baserel_idx >= (*root).simple_rel_array_size as pg_sys::Index {
            continue;
        }

        // Get the RTE to check if this is a partitioned table
        let rte = pg_sys::rt_fetch(baserel_idx, rtable);
        if (*rte).relkind as u8 != pg_sys::RELKIND_PARTITIONED_TABLE {
            continue;
        }

        // Access RelOptInfo safely using offset and read
        if (*root).simple_rel_array.is_null() {
            continue;
        }

        // This is a partitioned table, get its RelOptInfo to find partitions
        let rel_info_ptr = *(*root).simple_rel_array.add(baserel_idx as usize);
        if rel_info_ptr.is_null() {
            continue;
        }

        let rel_info = &*rel_info_ptr;

        // Check if it has partitions
        if rel_info.all_partrels.is_null() {
            continue;
        }

        // Check if any relation in rel_relids is among the partitions
        if pg_sys::bms_overlap(rel_info.all_partrels, rel_relids) {
            return true;
        }
    }

    false
}

/// Gather all columns referenced by the specified RTE (Range Table Entry) throughout the query.
/// This gives us a more complete picture than just looking at the target list.
///
/// This function is critical for issue #2505/#2556 where we need to detect all columns used in JOIN
/// conditions to ensure we select the right execution method. Previously, only looking at the
/// target list would miss columns referenced in JOIN conditions, leading to execution-time errors.
///
unsafe fn collect_maybe_fast_field_referenced_columns(
    rte_index: pg_sys::Index,
    rel: *mut pg_sys::RelOptInfo,
) -> HashSet<pg_sys::AttrNumber> {
    let mut referenced_columns = HashSet::default();

    // Check reltarget exprs.
    let reltarget_exprs = PgList::<pg_sys::Expr>::from_pg((*(*rel).reltarget).exprs);
    for rte in reltarget_exprs.iter_ptr() {
        if let Some(var) = nodecast!(Var, T_Var, rte) {
            if (*var).varno as u32 == rte_index {
                referenced_columns.insert((*var).varattno);
            }
        }
        // NOTE: Unless we encounter the fallback in `compute_exec_which_fast_fields`, then we
        // can be reasonably confident that directly inspecting Vars is sufficient. We haven't
        // seen it yet in the wild.
    }

    referenced_columns
}

/// Check if the base query has search predicates for the current table's index
fn base_query_has_search_predicates(
    query: &SearchQueryInput,
    current_index_oid: pg_sys::Oid,
) -> bool {
    match query {
        SearchQueryInput::All => false,
        SearchQueryInput::Uninitialized => false,
        SearchQueryInput::Empty => false,

        SearchQueryInput::WithIndex { oid, query } => {
            // Only consider search predicates for the current table's index
            if *oid == current_index_oid {
                // This is a search predicate for our index
                // Check the inner query directly for range vs search predicates
                base_query_has_search_predicates(query, current_index_oid)
            } else {
                // This is a search predicate for a different index, ignore it
                false
            }
        }

        // Boolean queries need recursive checking
        SearchQueryInput::Boolean {
            must,
            should,
            must_not,
        } => {
            must.iter()
                .any(|q| base_query_has_search_predicates(q, current_index_oid))
                || should
                    .iter()
                    .any(|q| base_query_has_search_predicates(q, current_index_oid))
                || must_not
                    .iter()
                    .any(|q| base_query_has_search_predicates(q, current_index_oid))
        }

        // Wrapper queries need recursive checking
        SearchQueryInput::Boost { query, .. } => {
            base_query_has_search_predicates(query, current_index_oid)
        }
        SearchQueryInput::ConstScore { query, .. } => {
            base_query_has_search_predicates(query, current_index_oid)
        }
        SearchQueryInput::ScoreFilter {
            query: Some(query), ..
        } => base_query_has_search_predicates(query, current_index_oid),
        SearchQueryInput::ScoreFilter { query: None, .. } => false,
        SearchQueryInput::DisjunctionMax { disjuncts, .. } => disjuncts
            .iter()
            .any(|q| base_query_has_search_predicates(q, current_index_oid)),

        // These are NOT search predicates (they're range/exists/other predicates)
        SearchQueryInput::Range { .. }
        | SearchQueryInput::RangeContains { .. }
        | SearchQueryInput::RangeIntersects { .. }
        | SearchQueryInput::RangeTerm { .. }
        | SearchQueryInput::RangeWithin { .. }
        | SearchQueryInput::Exists { .. }
        | SearchQueryInput::FastFieldRangeWeight { .. }
        | SearchQueryInput::MoreLikeThis { .. } => false,

        // These are search predicates that use the @@@ operator
        SearchQueryInput::ParseWithField { query_string, .. } => {
            // For ParseWithField, check if it's a text search or a range query
            !is_range_query_string(query_string)
        }
        SearchQueryInput::Parse { .. }
        | SearchQueryInput::TermSet { .. }
        | SearchQueryInput::Term { field: Some(_), .. }
        | SearchQueryInput::Phrase { .. }
        | SearchQueryInput::PhrasePrefix { .. }
        | SearchQueryInput::FuzzyTerm { .. }
        | SearchQueryInput::Match { .. }
        | SearchQueryInput::Regex { .. }
        | SearchQueryInput::RegexPhrase { .. } => true,

        // Term with no field is not a search predicate
        SearchQueryInput::Term { field: None, .. } => false,

        // Postgres expressions are unknown, assume they could be search predicates
        SearchQueryInput::PostgresExpression { .. } => true,
    }
}

/// Check if a query string represents a range query (contains operators like >, <, etc.)
fn is_range_query_string(query_string: &str) -> bool {
    // Range queries typically start with operators
    query_string.trim_start().starts_with('>')
        || query_string.trim_start().starts_with('<')
        || query_string.trim_start().starts_with(">=")
        || query_string.trim_start().starts_with("<=")
        || query_string.contains("..")  // Range syntax like "1..10"
        || query_string.contains(" TO ") // Range syntax like "1 TO 10"
}

/// Recursively clean RestrictInfo wrappers from any PostgreSQL node
/// This handles nested RestrictInfo nodes in complex expressions like BoolExpr
unsafe fn clean_restrictinfo_recursively(node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    if node.is_null() {
        return node;
    }

    match (*node).type_ {
        pg_sys::NodeTag::T_RestrictInfo => {
            // Unwrap RestrictInfo and recursively clean the inner clause
            let restrict_info = node.cast::<pg_sys::RestrictInfo>();
            let inner_clause = if !(*restrict_info).orclause.is_null() {
                (*restrict_info).orclause
            } else {
                (*restrict_info).clause
            };
            clean_restrictinfo_recursively(inner_clause.cast())
        }
        pg_sys::NodeTag::T_BoolExpr => {
            // For BoolExpr, clean all arguments
            let bool_expr = node.cast::<pg_sys::BoolExpr>();
            let args_list = (*bool_expr).args;

            if !args_list.is_null() {
                let mut new_args = std::ptr::null_mut();
                let old_args = PgList::<pg_sys::Node>::from_pg(args_list);

                for arg in old_args.iter_ptr() {
                    let cleaned_arg = clean_restrictinfo_recursively(arg);
                    new_args = pg_sys::lappend(new_args, cleaned_arg.cast::<core::ffi::c_void>());
                }

                // Create a new BoolExpr with cleaned arguments
                let new_bool_expr = pg_sys::palloc0(std::mem::size_of::<pg_sys::BoolExpr>())
                    .cast::<pg_sys::BoolExpr>();
                *new_bool_expr = *bool_expr; // Copy the original
                (*new_bool_expr).args = new_args;
                new_bool_expr.cast()
            } else {
                node
            }
        }
        _ => {
            // For other node types, return as-is (we could extend this for other complex types)
            node
        }
    }
}

/// Replace sub-expressions that reference other relations with TRUE constants
/// This preserves the logical structure while ensuring cross-relation predicates don't cause evaluation errors
unsafe fn replace_cross_relation_expressions_with_true(
    node: *mut pg_sys::Node,
    target_rti: pg_sys::Index,
) -> *mut pg_sys::Node {
    if node.is_null() {
        return node;
    }

    // Check if this entire expression references only the target relation
    if expression_references_only_relation(node, target_rti) {
        return node; // Keep as-is
    }

    // If this expression references other relations, check if we can replace parts of it
    match (*node).type_ {
        pg_sys::NodeTag::T_BoolExpr => {
            // For BoolExpr, recursively process arguments
            let bool_expr = node.cast::<pg_sys::BoolExpr>();
            let args_list = (*bool_expr).args;

            if !args_list.is_null() {
                let mut new_args = std::ptr::null_mut();
                let old_args = PgList::<pg_sys::Node>::from_pg(args_list);

                for arg in old_args.iter_ptr() {
                    let processed_arg =
                        replace_cross_relation_expressions_with_true(arg, target_rti);
                    new_args = pg_sys::lappend(new_args, processed_arg.cast::<core::ffi::c_void>());
                }

                // Create a new BoolExpr with processed arguments
                let new_bool_expr = pg_sys::palloc0(std::mem::size_of::<pg_sys::BoolExpr>())
                    .cast::<pg_sys::BoolExpr>();
                *new_bool_expr = *bool_expr; // Copy the original
                (*new_bool_expr).args = new_args;
                new_bool_expr.cast()
            } else {
                node
            }
        }
        _ => {
            // For leaf expressions that reference other relations, replace with TRUE
            create_bool_const_true().unwrap_or(node)
        }
    }
}

/// Check if a PostgreSQL expression node references only the specified relation
unsafe fn expression_references_only_relation(
    node: *mut pg_sys::Node,
    target_rti: pg_sys::Index,
) -> bool {
    if node.is_null() {
        return true;
    }

    extern "C-unwind" fn walker(node: *mut pg_sys::Node, context: *mut core::ffi::c_void) -> bool {
        unsafe {
            if node.is_null() {
                return false;
            }

            let target_rti = context.cast::<pg_sys::Index>().read();

            if let Some(var) = nodecast!(Var, T_Var, node) {
                // If this variable references a different relation, return true to stop walking
                // and indicate the expression is not suitable for this relation
                return (*var).varno as pg_sys::Index != target_rti;
            }

            // Continue walking the expression tree
            pg_sys::expression_tree_walker(node, Some(walker), context)
        }
    }

    let mut target_rti_copy = target_rti;
    !pg_sys::expression_tree_walker(
        node,
        Some(walker),
        (&mut target_rti_copy as *mut pg_sys::Index).cast(),
    )
}

/// Create a TRUE constant node
unsafe fn create_bool_const_true() -> Option<*mut pg_sys::Node> {
    let const_node = pg_sys::palloc0(std::mem::size_of::<pg_sys::Const>()).cast::<pg_sys::Const>();
    (*const_node).xpr.type_ = pg_sys::NodeTag::T_Const;
    (*const_node).consttype = pg_sys::BOOLOID;
    (*const_node).consttypmod = -1;
    (*const_node).constcollid = pg_sys::InvalidOid;
    (*const_node).constlen = 1;
    (*const_node).constvalue = true.into_datum().unwrap();
    (*const_node).constisnull = false;
    (*const_node).constbyval = true;
    (*const_node).location = -1;
    Some(const_node.cast())
}

unsafe fn create_heap_filter_expr(heap_filter_node_string: &str) -> *mut pg_sys::Expr {
    // Handle multiple clauses separated by our delimiter
    if heap_filter_node_string.contains("|||AND_CLAUSE_SEPARATOR|||")
        || heap_filter_node_string.contains("|||OR_CLAUSE_SEPARATOR|||")
        || heap_filter_node_string.contains("|||CLAUSE_SEPARATOR|||")
    {
        // Multiple clauses - determine the boolean operation and split accordingly
        let (clause_strings, bool_op) =
            if heap_filter_node_string.contains("|||AND_CLAUSE_SEPARATOR|||") {
                (
                    heap_filter_node_string
                        .split("|||AND_CLAUSE_SEPARATOR|||")
                        .collect::<Vec<&str>>(),
                    pg_sys::BoolExprType::AND_EXPR,
                )
            } else if heap_filter_node_string.contains("|||OR_CLAUSE_SEPARATOR|||") {
                (
                    heap_filter_node_string
                        .split("|||OR_CLAUSE_SEPARATOR|||")
                        .collect::<Vec<&str>>(),
                    pg_sys::BoolExprType::OR_EXPR,
                )
            } else {
                // Legacy support for old CLAUSE_SEPARATOR (assume AND)
                (
                    heap_filter_node_string
                        .split("|||CLAUSE_SEPARATOR|||")
                        .collect::<Vec<&str>>(),
                    pg_sys::BoolExprType::AND_EXPR,
                )
            };

        // Create individual nodes for each clause
        let mut args_list = std::ptr::null_mut();
        for clause_str in clause_strings.iter() {
            let clause_cstr = std::ffi::CString::new(*clause_str)
                .expect("Failed to create CString from clause string");
            let clause_node = pg_sys::stringToNode(clause_cstr.as_ptr());

            if !clause_node.is_null() {
                // For the legacy ExprState path, replace @@@ operators with TRUE constants
                let processed_node = replace_search_operators_with_true(clause_node.cast());
                args_list = pg_sys::lappend(args_list, processed_node.cast::<core::ffi::c_void>());
            } else {
                panic!("Failed to parse clause string: {}", clause_str);
            }
        }

        if !args_list.is_null() {
            // Create a BoolExpr to combine all clauses with the detected boolean operation
            let bool_expr =
                pg_sys::palloc0(std::mem::size_of::<pg_sys::BoolExpr>()).cast::<pg_sys::BoolExpr>();
            (*bool_expr).xpr.type_ = pg_sys::NodeTag::T_BoolExpr;
            (*bool_expr).boolop = bool_op;
            (*bool_expr).args = args_list;
            (*bool_expr).location = -1;

            bool_expr.cast::<pg_sys::Expr>()
        } else {
            panic!("Failed to parse any clauses: {}", heap_filter_node_string);
        }
    } else {
        // Single clause - for legacy ExprState path, process @@@ operators
        let node_cstr = std::ffi::CString::new(heap_filter_node_string)
            .expect("Failed to create CString from node string");
        let node = pg_sys::stringToNode(node_cstr.as_ptr());

        if !node.is_null() {
            // For the legacy ExprState path, replace @@@ operators with TRUE constants
            let processed_node = replace_search_operators_with_true(node.cast());
            processed_node.cast::<pg_sys::Expr>()
        } else {
            panic!("Failed to deserialize node: {}", heap_filter_node_string);
        }
    }
}

/// Replace @@@ operators with TRUE constants in expressions for heap evaluation
/// Since documents reaching heap filter have already passed Tantivy search,
/// @@@ operators should be considered TRUE for those documents
unsafe fn replace_search_operators_with_true(node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    if node.is_null() {
        return node;
    }

    match (*node).type_ {
        pg_sys::NodeTag::T_OpExpr => {
            let opexpr = node.cast::<pg_sys::OpExpr>();
            if (*opexpr).opno == anyelement_query_input_opoid()
                || (*opexpr).opno == anyelement_text_opoid()
            {
                // This is a @@@ operator - replace with TRUE constant
                return create_bool_const_true().unwrap_or(node);
            }
            // Otherwise, recurse into args to handle nested expressions
            let args_list = (*opexpr).args;
            if args_list.is_null() {
                return node;
            }
            let old_args = PgList::<pg_sys::Node>::from_pg(args_list);
            let mut new_args = std::ptr::null_mut();
            for arg in old_args.iter_ptr() {
                let processed = replace_search_operators_with_true(arg);
                new_args = pg_sys::lappend(new_args, processed.cast::<core::ffi::c_void>());
            }
            // Build a shallow copy with replaced args
            let new_opexpr = pg_sys::copyObjectImpl(node.cast()).cast::<pg_sys::OpExpr>();
            (*new_opexpr).args = new_args;
            new_opexpr.cast()
        }
        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = node.cast::<pg_sys::BoolExpr>();
            if (*boolexpr).args.is_null() {
                return node;
            }

            // Process all arguments to replace @@@ operators with TRUE
            let old_args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            let mut new_args = std::ptr::null_mut();
            for arg in old_args.iter_ptr() {
                let processed = replace_search_operators_with_true(arg);
                new_args = pg_sys::lappend(new_args, processed.cast::<core::ffi::c_void>());
            }

            // Create new boolean expression with processed arguments
            let new_bool_expr = pg_sys::copyObjectImpl(node.cast()).cast::<pg_sys::BoolExpr>();
            (*new_bool_expr).args = new_args;
            new_bool_expr.cast()
        }
        _ => node, // For other node types, return as-is
    }
}

/// Create a human-readable representation of a PostgreSQL expression node
/// Uses PostgreSQL's deparse_expression with proper context from ExplainState
unsafe fn create_human_readable_filter_text(
    heap_filter_node_string: &str,
    explainer: &Explainer,
    current_rti: pg_sys::Index,
) -> String {
    // Parse the heap filter node string back to a PostgreSQL node
    let heap_filter_expr = create_heap_filter_expr(heap_filter_node_string).cast::<pg_sys::Node>();

    // Now use PostgreSQL's deparse_expression with the proper context from ExplainState
    let deparse_cxt = explainer.deparse_cxt();

    if !deparse_cxt.is_null() {
        // Try deparse_expression with the proper context from EXPLAIN
        let cstring_ptr = pg_sys::deparse_expression(heap_filter_expr, deparse_cxt, false, false);
        if !cstring_ptr.is_null() {
            let result = std::ffi::CStr::from_ptr(cstring_ptr)
                .to_string_lossy()
                .into_owned();
            pg_sys::pfree(cstring_ptr.cast());

            // If the result is not empty, use it
            if !result.is_empty() {
                return result;
            }
        }
    }

    "<expression>".to_string()
}

/// Extract complete expressions from restrict_info and serialize them as node strings
/// Preserves both indexed and non-indexed predicates for unified evaluation
/// Replaces cross-relation expressions with TRUE to avoid evaluation errors
unsafe fn extract_restrictinfo_string(
    restrict_info: &PgList<pg_sys::RestrictInfo>,
    current_rti: pg_sys::Index,
    _root: *mut pg_sys::PlannerInfo,
) -> Option<String> {
    if restrict_info.is_empty() {
        return None;
    }

    // CRITICAL FIX: Check if we have a single complex expression that should not be split
    if restrict_info.len() == 1 {
        let ri = restrict_info.get_ptr(0).unwrap();
        let clause = if !(*ri).orclause.is_null() {
            (*ri).orclause
        } else {
            (*ri).clause
        };

        // Clean any nested RestrictInfo nodes recursively
        let cleaned_clause = clean_restrictinfo_recursively(clause.cast());

        // Check if this is a complex expression that should be treated as a single unit
        let is_complex_expression = match (*cleaned_clause).type_ {
            pg_sys::NodeTag::T_BoolExpr => {
                let bool_expr = cleaned_clause.cast::<pg_sys::BoolExpr>();
                match (*bool_expr).boolop {
                    pg_sys::BoolExprType::NOT_EXPR => true, // NOT expressions must not be split
                    pg_sys::BoolExprType::OR_EXPR => true,  // OR expressions must not be split
                    pg_sys::BoolExprType::AND_EXPR => true, // AND expressions must not be split
                    _ => false,
                }
            }
            _ => false,
        };

        if is_complex_expression {
            // Treat as a single expression - do not use CLAUSE_SEPARATOR
            let processed_clause =
                replace_cross_relation_expressions_with_true(cleaned_clause, current_rti);

            let clause_string: *mut i8 =
                pg_sys::nodeToString(processed_clause.cast::<core::ffi::c_void>());
            let rust_string = std::ffi::CStr::from_ptr(clause_string)
                .to_string_lossy()
                .into_owned();
            return Some(rust_string);
        }
    }

    // Extract just the clauses (not the RestrictInfo wrappers) and serialize them
    let mut clause_strings = Vec::new();

    for ri in restrict_info.iter_ptr() {
        // Extract the actual clause from the RestrictInfo and unwrap any nested RestrictInfo
        let clause = if !(*ri).orclause.is_null() {
            (*ri).orclause
        } else {
            (*ri).clause
        };

        // Clean any nested RestrictInfo nodes recursively
        let cleaned_clause = clean_restrictinfo_recursively(clause.cast());

        // For the unified approach: replace cross-relation expressions with TRUE constants
        // but PRESERVE search operators (@@@ operators) for unified evaluation
        let processed_clause =
            replace_cross_relation_expressions_with_true(cleaned_clause, current_rti);

        // Unified Approach: DO NOT remove search operators - preserve the entire expression
        // This allows the unified evaluator to handle both indexed and non-indexed predicates
        let clause_string: *mut i8 =
            pg_sys::nodeToString(processed_clause.cast::<core::ffi::c_void>());
        let rust_string = std::ffi::CStr::from_ptr(clause_string)
            .to_string_lossy()
            .into_owned();
        clause_strings.push(rust_string);
    }

    if clause_strings.is_empty() {
        None
    } else if clause_strings.len() == 1 {
        // Single clause - return it directly
        clause_strings.into_iter().next()
    } else {
        // Multiple clauses - join them with a separator for evaluation later
        // TODO: We should detect the original boolean operation (AND vs OR) and use different separators
        // For now, we assume AND logic as that's what PostgreSQL's query planner typically does
        // when decomposing expressions into multiple RestrictInfo entries
        Some(clause_strings.join("|||AND_CLAUSE_SEPARATOR|||"))
    }
}

/// Safe wrapper for replace_operator_with_true that ensures we never return null at the top level
unsafe fn remove_operator_safe(
    node: *mut pg_sys::Node,
    operator_oid: pg_sys::Oid,
) -> *mut pg_sys::Node {
    let result = remove_operator(node, operator_oid);
    if result.is_null() {
        // If the entire expression was filtered out, return TRUE
        create_bool_const_true().unwrap_or(node)
    } else {
        result
    }
}

/// Filter out occurrences of our specific operator (e.g., @@@) from heap filter expressions
/// This completely removes them from boolean expressions rather than replacing with TRUE
/// For OR expressions with our operator, retain the other side; for AND expressions, remove our operator
/// Returns TRUE constant if the entire expression would be filtered out
unsafe fn remove_operator(node: *mut pg_sys::Node, operator_oid: pg_sys::Oid) -> *mut pg_sys::Node {
    if node.is_null() {
        return node;
    }

    match (*node).type_ {
        pg_sys::NodeTag::T_OpExpr => {
            let opexpr = node.cast::<pg_sys::OpExpr>();
            if (*opexpr).opno == operator_oid {
                // If this is a top-level OpExpr with our operator, return TRUE
                // If it's nested in a BoolExpr, return NULL to indicate removal
                return std::ptr::null_mut();
            }
            // Otherwise, recurse into args to handle nested expressions
            let args_list = (*opexpr).args;
            if args_list.is_null() {
                return node;
            }
            let old_args = PgList::<pg_sys::Node>::from_pg(args_list);
            let mut new_args = std::ptr::null_mut();
            for arg in old_args.iter_ptr() {
                let processed = remove_operator(arg, operator_oid);
                if !processed.is_null() {
                    new_args = pg_sys::lappend(new_args, processed.cast::<core::ffi::c_void>());
                }
            }
            // Build a shallow copy with replaced args
            let new_opexpr = pg_sys::copyObjectImpl(node.cast()).cast::<pg_sys::OpExpr>();
            (*new_opexpr).args = new_args;
            new_opexpr.cast()
        }
        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = node.cast::<pg_sys::BoolExpr>();
            if (*boolexpr).args.is_null() {
                return node;
            }

            // Process all arguments to filter out our operator
            let old_args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            let mut new_args = std::ptr::null_mut();
            for arg in old_args.iter_ptr() {
                let processed = remove_operator(arg, operator_oid);
                if !processed.is_null() {
                    new_args = pg_sys::lappend(new_args, processed.cast::<core::ffi::c_void>());
                }
            }

            // Handle special cases based on boolean operation type
            if pg_sys::list_length(new_args) == 0 {
                // All args were filtered out - return TRUE constant instead of null
                return create_bool_const_true().unwrap_or(node);
            } else if pg_sys::list_length(new_args) == 1
                && ((*boolexpr).boolop == pg_sys::BoolExprType::AND_EXPR
                    || (*boolexpr).boolop == pg_sys::BoolExprType::OR_EXPR)
            {
                // If we have AND(x) or OR(x), just return x directly
                return (*pg_sys::list_head(new_args)).ptr_value.cast();
            }

            // Create new boolean expression with filtered arguments
            let new_bool_expr = pg_sys::copyObjectImpl(node.cast()).cast::<pg_sys::BoolExpr>();
            (*new_bool_expr).args = new_args;
            new_bool_expr.cast()
        }
        _ => node, // For other node types, return as-is
    }
}

/// Enhanced heap filter that uses the OptimizedExpressionEvaluator for better scoring
/// This replaces the simple boolean apply_heap_filter with enhanced scoring capabilities
unsafe fn apply_enhanced_heap_filter(
    state: &mut CustomScanStateWrapper<PdbScan>,
    slot: *mut pg_sys::TupleTableSlot,
    current_score: f32,
    doc_address: DocAddress,
) -> OptimizedEvaluationResult {
    // Get values first to avoid borrowing conflicts
    let heap_filter_node_string = state.custom_state().heap_filter_node_string.clone();

    // If there's no heap filter, just return the current score
    if heap_filter_node_string.is_none() {
        return OptimizedEvaluationResult::new(true, current_score);
    }

    let expr_context = (*state.planstate()).ps_ExprContext;
    let has_search_reader = state.custom_state().search_reader.is_some();
    let has_indexrel = state.custom_state().indexrel.is_some();

    if has_search_reader && has_indexrel {
        // Get the references we need
        let search_reader = state.custom_state().search_reader.as_ref().unwrap();
        let indexrel_oid = *state.custom_state().indexrel.as_ref().unwrap();
        let indexrel = PgRelation::from_pg(indexrel_oid);
        let schema =
            SearchIndexSchema::open(indexrel.oid()).expect("should be able to open schema");

        // Use the optimized heap filter for enhanced scoring
        match apply_optimized_unified_heap_filter(
            search_reader,
            &schema,
            heap_filter_node_string.as_ref().unwrap(),
            expr_context,
            slot,
            0, // We don't have the actual DocId here, using 0 as placeholder
            doc_address,
            current_score,
        ) {
            Ok(result) => result,
            Err(e) => {
                pgrx::log!("Error in optimized heap filter: {}", e);
                // Fall back to the original apply_heap_filter behavior
                let matches = apply_heap_filter(state, slot);
                OptimizedEvaluationResult::new(matches, current_score)
            }
        }
    } else {
        // Fall back to the original apply_heap_filter behavior
        let matches = apply_heap_filter(state, slot);
        OptimizedEvaluationResult::new(matches, current_score)
    }
}

/// Phase 4: Smart base query optimization for unified evaluation
/// Creates an optimized base query that reduces the document set while preserving
/// the ability to handle mixed expressions in the heap filter
unsafe fn optimize_base_query_for_unified_evaluation(
    indexed_quals: &Option<Qual>,
    restrict_info: &PgList<pg_sys::RestrictInfo>,
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
    builder: &mut CustomPathBuilder<PrivateData>,
) -> Option<Qual> {
    debug_log!(
        "🔧 [OPTIMIZE] Starting base query optimization with indexed_quals: {:?}",
        indexed_quals
    );

    // NEW UNIFIED EXECUTION APPROACH: Build SearchQueryInput directly from raw expressions
    // This maximizes what gets pushed to Tantivy and only breaks down when hitting non-indexed fields
    // CRITICAL: Work with raw restrict_info expressions BEFORE decomposition

    // CRITICAL FIX: Use heap filter node string when available as it contains the original un-decomposed expression
    // For Test 2.2, the heap filter contains the original NOT expression before PostgreSQL decomposes it
    if let Some(heap_filter_node_string) = builder.custom_private().heap_filter_node_string() {
        debug_log!(
            "🔧 [OPTIMIZE] Found heap filter node string, attempting to parse original expression"
        );

        // The heap filter contains the original expression before PostgreSQL decomposition
        // Try to parse it and extract search operators directly
        let heap_filter_expr = create_heap_filter_expr(&heap_filter_node_string);
        if !heap_filter_expr.is_null() {
            debug_log!("🔧 [OPTIMIZE] Successfully parsed heap filter expression, building SearchQueryInput");

            // Extract the raw node from the heap filter expression
            let heap_filter_node = heap_filter_expr.cast::<pg_sys::Node>();

            // Check if this original expression contains search operators
            if expression_contains_search_operators(heap_filter_node, pdbopoid) {
                debug_log!("🔧 [OPTIMIZE] Heap filter expression contains search operators, building optimal SearchQueryInput");

                // Use the original heap filter expression (not the decomposed version)
                match ExpressionTreeOptimizer::build_search_query_from_expression(
                    heap_filter_node,
                    schema,
                ) {
                    Ok(search_query_input) => {
                        debug_log!(
                            "🔧 [OPTIMIZE] Successfully built SearchQueryInput from heap filter: {:?}",
                            search_query_input.as_human_readable()
                        );

                        // Check if we extracted meaningful search operators
                        match search_query_input {
                            SearchQueryInput::All => {
                                // Expression contains only non-indexed fields, fall back to old approach
                                debug_log!("🔧 [OPTIMIZE] Heap filter expression contains only non-indexed fields, falling back");
                            }
                            _ => {
                                // We have a proper Tantivy query - set it directly in the builder
                                debug_log!(
                                    "🔧 [OPTIMIZE] Setting optimized SearchQueryInput from heap filter directly: {:?}",
                                    search_query_input.as_human_readable()
                                );

                                // Set the optimized SearchQueryInput directly in the PrivateData
                                builder.custom_private().set_query(search_query_input);

                                // Return a synthetic Qual to indicate we handled the optimization
                                return Some(Qual::All);
                            }
                        }
                    }
                    Err(e) => {
                        debug_log!(
                            "🔧 [OPTIMIZE] Failed to build SearchQueryInput from heap filter: {}",
                            e
                        );
                    }
                }
            } else {
                debug_log!(
                    "🔧 [OPTIMIZE] Heap filter expression does not contain search operators"
                );
            }
        } else {
            debug_log!("🔧 [OPTIMIZE] Failed to parse heap filter expression");
        }
    } else {
        debug_log!("🔧 [OPTIMIZE] No heap filter node string available");
    }

    // FALLBACK: Check the original expressions from restrict_info (before decomposition)
    for ri in restrict_info.iter_ptr() {
        // Get the original clause before any decomposition
        let original_clause = if !(*ri).orclause.is_null() {
            (*ri).orclause
        } else {
            (*ri).clause
        };

        // Clean any nested RestrictInfo nodes recursively but preserve original structure
        let cleaned_original = clean_restrictinfo_recursively(original_clause.cast());

        // Check if this original expression contains search operators
        if expression_contains_search_operators(cleaned_original, pdbopoid) {
            debug_log!("🔧 [OPTIMIZE] Found expression with search operators in original clause, building optimal SearchQueryInput");

            // CRITICAL: Use the cleaned original expression (not the decomposed version)
            // This preserves the NOT ((name @@@ 'Apple' AND category = 'Electronics') OR (category = 'Furniture'))
            // structure instead of working with the decomposed AND clauses
            match ExpressionTreeOptimizer::build_search_query_from_expression(
                cleaned_original,
                schema,
            ) {
                Ok(search_query_input) => {
                    debug_log!(
                        "🔧 [OPTIMIZE] Successfully built SearchQueryInput from original: {:?}",
                        search_query_input.as_human_readable()
                    );

                    // CRITICAL FIX: For Test 2.2 and unified execution approach
                    // For Test 2.2: NOT ((name @@@ 'Apple' AND category = 'Electronics') OR (category = 'Furniture'))
                    // This should create NOT(name @@@ 'Apple') as the Tantivy query
                    // and let the heap filter handle the complete expression
                    match search_query_input {
                        SearchQueryInput::All => {
                            // Expression contains only non-indexed fields, fall back to old approach
                            debug_log!("🔧 [OPTIMIZE] Original expression contains only non-indexed fields, falling back");
                        }
                        _ => {
                            // We have a proper Tantivy query - set it directly in the builder
                            debug_log!(
                                "🔧 [OPTIMIZE] Setting optimized SearchQueryInput from original directly: {:?}",
                                search_query_input.as_human_readable()
                            );

                            // Set the optimized SearchQueryInput directly in the PrivateData
                            // This bypasses the Qual -> SearchQueryInput conversion that defaults to All
                            builder.custom_private().set_query(search_query_input);

                            // Return a synthetic Qual to indicate we handled the optimization
                            return Some(Qual::All);
                        }
                    }
                }
                Err(e) => {
                    debug_log!(
                        "🔧 [OPTIMIZE] Failed to build SearchQueryInput from original: {}",
                        e
                    );
                }
            }
        }
    }

    // FALLBACK: Use the original approach if the new method didn't work

    // First, check if we have direct indexed predicates
    debug_log!("🔧 [OPTIMIZE] No suitable expressions found, returning None");
    // If no expressions with search operators found, return None to indicate
    // that we can't handle this query with the custom scan
    None
}

/// Optimize NOT expressions that contain both search and non-search predicates
/// For NOT (search_pred OR non_search_pred), we want to create a Tantivy query
/// that only handles the search predicates to reduce the document set
unsafe fn optimize_not_expression_with_mixed_predicates(
    indexed_qual: &Qual,
    _restrict_info: &PgList<pg_sys::RestrictInfo>,
    _root: *mut pg_sys::PlannerInfo,
    _rti: pg_sys::Index,
    _pdbopoid: pg_sys::Oid,
    _schema: &SearchIndexSchema,
) -> Option<Qual> {
    debug_log!(
        "🔧 [NOT_OPT] Analyzing qual for NOT optimization: {:?}",
        indexed_qual
    );

    match indexed_qual {
        // Handle: And([Not(search_pred), ExternalExpr])
        // This represents: NOT (search_pred OR non_search_pred)
        Qual::And(and_clauses) if and_clauses.len() == 2 => {
            debug_log!("🔧 [NOT_OPT] Found AND with 2 clauses, checking for NOT pattern");

            // Look for pattern: [Not(search_pred), ExternalExpr]
            if let (Qual::Not(not_clause), Qual::ExternalExpr) = (&and_clauses[0], &and_clauses[1])
            {
                debug_log!("🔧 [NOT_OPT] Found NOT + ExternalExpr pattern, optimizing");

                // CRITICAL FIX: Return the NOT qualifier directly
                // The Not conversion logic in qual_inspect.rs already handles search operators correctly:
                // - If the inner expression contains search operators, it creates Boolean { must: [], must_not: [...] }
                // - If it doesn't contain search operators, it creates Boolean { must: [All], must_not: [...] }
                // We don't need to wrap it in And([All, ...]) - that creates malformed queries
                return Some(Qual::Not(Box::new((**not_clause).clone())));
            }

            // Also check the reverse order: [ExternalExpr, Not(search_pred)]
            if let (Qual::ExternalExpr, Qual::Not(not_clause)) = (&and_clauses[0], &and_clauses[1])
            {
                debug_log!("🔧 [NOT_OPT] Found ExternalExpr + NOT pattern, optimizing");
                return Some(Qual::Not(Box::new((**not_clause).clone())));
            }
        }
        _ => {
            debug_log!("🔧 [NOT_OPT] Qual doesn't match NOT optimization pattern");
        }
    }

    None
}

/// Check if a PostgreSQL expression contains search operators (@@@)
unsafe fn expression_contains_search_operators(
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
) -> bool {
    if node.is_null() {
        return false;
    }

    match (*node).type_ {
        pg_sys::NodeTag::T_OpExpr => {
            let op_expr = node.cast::<pg_sys::OpExpr>();
            if (*op_expr).opno == pdbopoid {
                return true;
            }
            // Check arguments recursively
            let args = PgList::<pg_sys::Node>::from_pg((*op_expr).args);
            for arg in args.iter_ptr() {
                if expression_contains_search_operators(arg, pdbopoid) {
                    return true;
                }
            }
            false
        }
        pg_sys::NodeTag::T_BoolExpr => {
            let bool_expr = node.cast::<pg_sys::BoolExpr>();
            let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);
            for arg in args.iter_ptr() {
                if expression_contains_search_operators(arg, pdbopoid) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}
