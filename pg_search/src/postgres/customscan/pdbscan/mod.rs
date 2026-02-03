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

#![allow(clippy::unnecessary_cast)] // helps with integer casting differences between postgres versions
mod exec_methods;
pub mod parallel;
mod privdat;
pub mod projections;
mod scan_state;

use std::ffi::CStr;
use std::ptr::addr_of_mut;
use std::sync::atomic::Ordering;
use std::sync::Once;

use crate::api::operator::{anyelement_query_input_opoid, estimate_selectivity};
use crate::api::window_aggregate::window_agg_oid;
use crate::api::{HashMap, HashSet, OrderByFeature, OrderByInfo, Varno};
use crate::gucs;
use crate::index::fast_fields_helper::WhichFastField;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::{SearchIndexReader, MAX_TOPN_FEATURES};
use crate::postgres::customscan::builders::custom_path::{
    restrict_info, CustomPathBuilder, ExecMethodType, Flags, RestrictInfoType,
};
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::orderby::{
    extract_pathkey_styles_with_sortability_check, PathKeyInfo, UnusableReason,
};
use crate::postgres::customscan::pdbscan::exec_methods::{
    fast_fields, normal::NormalScanExecState, ExecState,
};
use crate::postgres::customscan::pdbscan::parallel::{compute_nworkers, list_segment_ids};
use crate::postgres::customscan::pdbscan::privdat::PrivateData;
use crate::postgres::customscan::pdbscan::projections::score::uses_scores;
use crate::postgres::customscan::pdbscan::projections::snippet::{
    snippet_funcoids, snippet_positions_funcoids, snippets_funcoids, uses_snippets, SnippetType,
};
use crate::postgres::customscan::pdbscan::projections::window_agg::{
    deserialize_window_agg_placeholders, resolve_window_aggregate_filters_at_plan_time,
    WindowAggregateInfo,
};
use crate::postgres::customscan::pdbscan::projections::{
    inject_placeholders, maybe_needs_const_projections, pullout_funcexprs,
};
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::postgres::customscan::qual_inspect::{
    extract_join_predicates, extract_quals, optimize_quals_with_heap_expr, PlannerContext, Qual,
    QualExtractState,
};
use crate::postgres::customscan::score_funcoids;
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::customscan::{
    self, range_table, CustomScan, CustomScanState, RelPathlistHookArgs,
};
use crate::postgres::heap::{HeapFetchState, VisibilityChecker};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::utils::filter_implied_predicates;
use crate::query::pdb_query::pdb;
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;
use crate::{nodecast, DEFAULT_STARTUP_COST, PARAMETERIZED_SELECTIVITY, UNKNOWN_SELECTIVITY};
use crate::{FULL_RELATION_SELECTIVITY, UNASSIGNED_SELECTIVITY};

use pgrx::pg_sys::CustomExecMethods;
use pgrx::{direct_function_call, pg_sys, FromDatum, IntoDatum, PgList, PgMemoryContexts};
use tantivy::snippet::SnippetGenerator;
use tantivy::Index;

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
            .expect("custom_state.indexrel should already be open");

        let search_query_input = state.custom_state().search_query_input();
        let need_scores = state.custom_state().need_scores();

        let search_reader = SearchIndexReader::open_with_context(
            indexrel,
            search_query_input.clone(),
            need_scores,
            unsafe {
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
            },
            std::ptr::NonNull::new(expr_context),
            std::ptr::NonNull::new(planstate),
        )
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
                    .snippet_generator(
                        snippet_type.field().root(),
                        query_to_use,
                        std::ptr::NonNull::new(expr_context),
                    );

                snippet_type.configure_generator(&mut new_generator.1);

                *generator = Some(new_generator);
            }

            state.custom_state_mut().snippet_generators = snippet_generators;
        }

        unsafe {
            inject_pdb_placeholders(state);
        }
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn extract_all_possible_quals(
        builder: &mut CustomPathBuilder<PdbScan>,
        root: *mut pg_sys::PlannerInfo,
        rti: pg_sys::Index,
        restrict_info: PgList<pg_sys::RestrictInfo>,
        ri_type: RestrictInfoType,
        indexrel: &PgSearchRelation,
        uses_score_or_snippet: bool,
        attempt_pushdown: bool,
    ) -> (Option<Qual>, RestrictInfoType, PgList<pg_sys::RestrictInfo>) {
        let mut state = QualExtractState::default();
        let context = PlannerContext::from_planner(root);

        // Filter out predicates that are implied by the partial index predicate.
        // If a partial index has predicate P (e.g., "deleted_at IS NULL"), and the query
        // also has predicate P, we don't need to create a heap filter for P since the
        // partial index already guarantees it.
        let filtered_restrict_info = filter_implied_predicates(indexrel.rd_indpred, &restrict_info);

        let mut quals = extract_quals(
            &context,
            rti,
            filtered_restrict_info.as_ptr().cast(),
            anyelement_query_input_opoid(),
            ri_type,
            indexrel,
            false, // Base relation quals should not convert external to all
            &mut state,
            attempt_pushdown,
        );

        // If we couldn't push down quals, try to push down quals from the join
        // This is only done if we have a join predicate, and only if we have used our operator
        let (quals, ri_type, restrict_info) = if quals.is_none() {
            let joinri: PgList<pg_sys::RestrictInfo> =
                PgList::from_pg(builder.args().rel().joininfo);
            let mut quals = extract_quals(
                &context,
                rti,
                joinri.as_ptr().cast(),
                anyelement_query_input_opoid(),
                RestrictInfoType::Join,
                indexrel,
                true, // Join quals should convert external to all
                &mut state,
                attempt_pushdown,
            );

            let quals = Self::handle_heap_expr_optimization(&state, &mut quals, root, rti);

            // If we have found something to push down in the join, then we can use the join quals
            // Note: these Join quals won't help in filtering down the data (as they contain
            // external vars, e.g. `b.category_name @@@ "technology"` in
            // `a.name @@@ "abc" OR b.category_name @@@ "technology"`), and we cannot evaluate
            // boolean expressions that contain external vars. That's why, when handling the Join
            // quals, we'd endup scanning the whole tantivy index.
            // However, the Join quals help with scoring and snippet generation, as the documents
            // that match partially the Join quals will be scored and snippets generated. That is
            // why it only makes sense to use the Join quals if we have used our operator and
            // also used pdb.score or pdb.snippet functions in the query.
            if state.uses_our_operator && uses_score_or_snippet {
                (quals, RestrictInfoType::Join, joinri)
            } else {
                (None, ri_type, restrict_info)
            }
        } else {
            let quals = Self::handle_heap_expr_optimization(&state, &mut quals, root, rti);
            (quals, ri_type, restrict_info)
        };

        // Finally, decide whether we can actually use the extracted quals.
        // We allow custom scan if:
        // 1. The query uses @@@ operator, OR
        // 2. enable_custom_scan_without_operator is true, OR
        // 3. The query has window aggregates (pdb.agg()) that we must handle
        let has_window_aggs = query_has_window_agg_functions(root);
        if state.uses_our_operator || gucs::enable_custom_scan_without_operator() || has_window_aggs
        {
            (quals, ri_type, restrict_info)
        } else {
            (None, ri_type, restrict_info)
        }
    }

    unsafe fn handle_heap_expr_optimization(
        state: &QualExtractState,
        quals: &mut Option<Qual>,
        root: *mut pg_sys::PlannerInfo,
        rti: pg_sys::Index,
    ) -> Option<Qual> {
        if state.uses_heap_expr && !state.uses_our_operator {
            return None;
        }

        // Apply HeapExpr optimization to the base relation quals
        if let Some(ref mut q) = quals {
            let rtable = (*(*root).parse).rtable;
            let rtable_size = if !rtable.is_null() {
                PgList::<pg_sys::RangeTblEntry>::from_pg(rtable).len()
            } else {
                0
            };

            // Bounds check: rti is 1-indexed, so it must be between 1 and rtable_size
            if rti > 0 && (rti as usize) <= rtable_size {
                let rte = pg_sys::rt_fetch(rti, rtable);
                let relation_oid = (*rte).relid;
                optimize_quals_with_heap_expr(q);
            }
            // Skip optimization silently if RTE is out of bounds
            // This can happen with OR EXISTS subqueries where variables reference RTEs from different contexts
        }

        quals.clone()
    }
}

impl customscan::ExecMethod for PdbScan {
    fn exec_methods() -> *const CustomExecMethods {
        <PdbScan as ParallelQueryCapable>::exec_methods()
    }
}

/// Check if the query's target list contains window_agg() function calls
///
/// This is called AFTER window function replacement in PdbScan's create_custom_path.
/// It looks for FuncExpr nodes with window_agg() OID, NOT WindowFunc nodes.
///
/// This is different from query_has_window_functions() in hook.rs which looks for WindowFunc
/// nodes BEFORE replacement in the planner hook.
///
/// Used to determine if we should create a custom path even without @@@ operator.
///
/// Also validates that pdb.agg() is not present - if it is, that means the planner hook
/// didn't replace it (e.g., not a TopN query), and we should reject it.
unsafe fn query_has_window_agg_functions(root: *mut pg_sys::PlannerInfo) -> bool {
    if root.is_null() || (*root).parse.is_null() {
        return false;
    }

    let parse = (*root).parse;
    let window_agg_func_oid = window_agg_oid();
    let paradedb_agg_func_oid = crate::api::agg_funcoid();

    // If functions don't exist yet (e.g., during extension creation), skip check
    if window_agg_func_oid == pg_sys::InvalidOid {
        return false;
    }

    let window_agg_func_oid = window_agg_func_oid.to_u32();
    let paradedb_agg_func_oid = paradedb_agg_func_oid.to_u32();

    // Check target list for window_agg() or pdb.agg() function calls
    if !(*parse).targetList.is_null() {
        let target_list = PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList);
        for te in target_list.iter_ptr() {
            if !(*te).expr.is_null() {
                // Check if this is a FuncExpr with window_agg or pdb.agg OID
                if let Some(func_expr) = nodecast!(FuncExpr, T_FuncExpr, (*te).expr) {
                    let func_oid = (*func_expr).funcid.to_u32();
                    if func_oid == window_agg_func_oid {
                        return true;
                    } else if func_oid == paradedb_agg_func_oid {
                        // pdb.agg() should have been replaced by planner hook
                        // If it's still here, it means it wasn't a valid TopN query
                        pgrx::error!(
                            "pdb.agg() can only be used as a window function in TopN queries \
                             (queries with ORDER BY and LIMIT). For GROUP BY aggregates, use standard \
                             SQL aggregates like COUNT(*), SUM(), etc. \
                             Hint: Try using '@@@ pdb.all()' with ORDER BY and LIMIT, \
                             or see https://github.com/paradedb/paradedb/issues for more information."
                        );
                    }
                }
            }
        }
    }

    false
}

/// Is the function identified by `funcid` an approved set-returning-function
/// that is safe for our limit push-down optimization?
fn is_limit_safe_srf(funcid: pg_sys::Oid) -> bool {
    static mut UNNEST_OID: pg_sys::Oid = pg_sys::InvalidOid;
    static APPROVE_SRF_ONCE: Once = Once::new();

    unsafe {
        APPROVE_SRF_ONCE.call_once(|| {
            if let Some(oid) = direct_function_call::<pg_sys::Oid>(
                pg_sys::regprocedurein,
                &[c"pg_catalog.unnest(anyarray)".into_datum()],
            ) {
                UNNEST_OID = oid;
            }
        });
        funcid == UNNEST_OID && UNNEST_OID != pg_sys::InvalidOid
    }
}

/// Check if the query's target list contains only set-returning functions (e.g. `unnest()`) which
/// are safe for use with our LIMIT optimization, and if so, then return the LIMIT from the parse.
///
/// Only set returning functions which produce at least as many rows as they consume are safe.
unsafe fn maybe_limit_from_parse(root: *mut pg_sys::PlannerInfo) -> Option<f64> {
    if root.is_null() || (*root).parse.is_null() || (*(*root).parse).targetList.is_null() {
        return None;
    }
    // non-Const LIMIT is not a thing we can handle here
    let limit_const = nodecast!(Const, T_Const, (*(*root).parse).limitCount)?;
    let limit =
        i64::from_datum((*limit_const).constvalue, (*limit_const).constisnull).map(|v| v as f64)?;

    let offset = if (*(*root).parse).limitOffset.is_null() {
        0.0
    } else if let Some(offset_const) = nodecast!(Const, T_Const, (*(*root).parse).limitOffset) {
        i64::from_datum((*offset_const).constvalue, (*offset_const).constisnull)
            .map(|v| v as f64)?
    } else {
        // non-Const OFFSET is not a thing we can handle here
        return None;
    };

    let mut found_limit_safe_srf = false;
    let target_list = PgList::<pg_sys::TargetEntry>::from_pg((*(*root).parse).targetList);
    for te in target_list.iter_ptr() {
        if !(*te).expr.is_null() && pg_sys::expression_returns_set((*te).expr.cast()) {
            // It's a set-returning function, is it one we approve of?
            if let Some(func_expr) = nodecast!(FuncExpr, T_FuncExpr, (*te).expr) {
                if is_limit_safe_srf((*func_expr).funcid) {
                    found_limit_safe_srf = true;
                } else {
                    // We don't recognize this SRF, and can't vouch for it.
                    return None;
                }
            }
        }
    }

    // A LIMIT was applied in the parse for our node, but is being handled elsewhere
    // due to a set-returning-function that we know produces at least as many tuples as
    // it is given.
    if found_limit_safe_srf {
        Some(limit + offset)
    } else {
        None
    }
}

impl CustomScan for PdbScan {
    const NAME: &'static CStr = c"ParadeDB Scan";

    type Args = RelPathlistHookArgs;
    type State = PdbScanState;
    type PrivateData = PrivateData;

    fn create_custom_path(mut builder: CustomPathBuilder<Self>) -> Option<pg_sys::CustomPath> {
        unsafe {
            let (restrict_info, ri_type) = restrict_info(builder.args().rel());

            // Check if the query has window aggregates (pdb.agg() or window_agg())
            let has_window_aggs = query_has_window_agg_functions(builder.args().root);

            if matches!(ri_type, RestrictInfoType::None) && !has_window_aggs {
                // this relation has no restrictions (WHERE clause predicates) and no window aggregates,
                // so there's no need for us to do anything
                return None;
            }

            let rti = builder.args().rti;
            let (table, bm25_index) = {
                let rte = builder.args().rte();

                // we only support plain relation and join rte's
                if rte.rtekind != pg_sys::RTEKind::RTE_RELATION
                    && rte.rtekind != pg_sys::RTEKind::RTE_JOIN
                {
                    return None;
                }

                // and we only work on plain relations
                let relkind = pg_sys::get_rel_relkind(rte.relid) as u8;
                if relkind != pg_sys::RELKIND_RELATION && relkind != pg_sys::RELKIND_MATVIEW {
                    return None;
                }

                // and that relation must have a `USING bm25` index
                let (table, bm25_index) = rel_get_bm25_index(rte.relid)?;

                (table, bm25_index)
            };

            let root = builder.args().root;
            let rel = builder.args().rel;

            // quick look at the target list to see if we might need to do our const projections
            let target_list = (*(*builder.args().root).parse).targetList;
            let maybe_needs_const_projections = maybe_needs_const_projections(target_list.cast());

            //
            // look for quals we can support.  we do this first so that we can get out early if this
            // isn't a query we can support.
            //
            // Opening the Directory and Index down below is expensive, so if we can avoid it,
            // especially for non-SELECT (ie, UPDATE) statements, that's good
            //
            let is_select =
                (*(*builder.args().root).parse).commandType == pg_sys::CmdType::CMD_SELECT;
            let (quals, ri_type, restrict_info) = Self::extract_all_possible_quals(
                &mut builder,
                root,
                rti,
                restrict_info,
                ri_type,
                &bm25_index,
                maybe_needs_const_projections,
                is_select,
            );

            // If we have window aggregates but no quals, we must still create the custom path
            // because pdb.agg() can only be executed by our custom scan
            let quals = if let Some(q) = quals {
                q
            } else if has_window_aggs {
                // We have window aggregates but couldn't extract quals.
                // This can happen in two cases:
                // 1. No WHERE clause at all -> safe to use Qual::All
                // 2. WHERE clause exists but couldn't be extracted:
                //    a. filter_pushdown enabled -> HeapExpr was created during extraction, safe to use Qual::All
                //    b. filter_pushdown disabled -> unsafe, reject the query
                let has_where_clause = !(*root).parse.is_null()
                    && !(*(*root).parse).jointree.is_null()
                    && !(*(*(*root).parse).jointree).quals.is_null();

                if has_where_clause && !crate::gucs::enable_filter_pushdown() {
                    // There's a WHERE clause but we couldn't extract quals and filter_pushdown is disabled.
                    // This means qual extraction failed without creating HeapExpr, so we cannot handle
                    // this query safely - the WHERE clause would be silently ignored.
                    return None;
                }

                // Safe to use Qual::All because:
                // - Either there's no WHERE clause (nothing to filter), OR
                // - filter_pushdown is enabled, meaning HeapExpr was created during qual extraction
                //   and will be evaluated by PostgreSQL's executor after we return results
                Qual::All
            } else {
                // No quals and no window aggregates - we can't help
                return None;
            };

            // Check if this is a partial index and if the query is compatible with it
            if !bm25_index.rd_indpred.is_null() {
                // This is a partial index - we need to check if the query predicates
                // imply the partial index predicate using PostgreSQL's predicate_implied_by.
                //
                // For example:
                // - Partial index: WHERE deleted_at IS NULL
                // - Query: WHERE deleted_at IS NULL AND category_id = 5
                // - predicate_implied_by(index_pred, query_clauses) returns true
                //
                // But:
                // - Partial index: WHERE category = 'Electronics'
                // - Query: WHERE description @@@ 'Apple' AND rating >= 4
                // - predicate_implied_by returns false (query doesn't imply category = 'Electronics')

                // Extract the restriction clauses as a list of Expr nodes
                let mut clause_list: *mut pg_sys::List = std::ptr::null_mut();
                for ri in restrict_info.iter_ptr() {
                    clause_list =
                        pg_sys::lappend(clause_list, (*ri).clause as *mut std::ffi::c_void);
                }

                // Check if query clauses imply the partial index predicate
                let is_compatible =
                    pg_sys::predicate_implied_by(bm25_index.rd_indpred, clause_list, false);

                if !is_compatible {
                    // The query predicates don't imply the partial index predicate,
                    // so we cannot safely use this partial index
                    return None;
                }
            }

            //
            // ===================
            // If we make it this far, we're going to submit a path... it better be a good one!
            // ====================
            //

            // TODO: `impl Default for PrivateData` requires that many fields are in invalid
            // states. Should consider having a separate builder for PrivateData.
            let mut custom_private = PrivateData::default();

            let directory = MvccSatisfies::LargestSegment.directory(&bm25_index);
            let segment_count = directory.total_segment_count(); // return value only valid after the index has been opened
            let index = Index::open(directory).expect("custom_scan: should be able to open index");
            let segment_count = segment_count.load(Ordering::Relaxed);
            let schema = bm25_index
                .schema()
                .expect("custom_scan: should have a schema");
            let topn_pathkey_info = pullup_topn_pathkeys(&mut builder, rti, &schema, root);

            #[cfg(any(feature = "pg14", feature = "pg15"))]
            let baserels = (*builder.args().root).all_baserels;
            #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
            let baserels = (*builder.args().root).all_query_rels;

            let limit = if (*builder.args().root).limit_tuples > -1.0 {
                // Check if this is a single relation or a partitioned table setup
                let rel_is_single_or_partitioned = pg_sys::bms_equal((*rel).relids, baserels)
                    || range_table::is_partitioned_table_setup(
                        builder.args().root,
                        (*rel).relids,
                        baserels,
                    );

                // Check for LEFT JOIN LATERAL where left side drives the query
                let is_left_driven_lateral = is_left_join_lateral(builder.args().root, rel)
                    && where_clause_only_references_left(builder.args().root, rti);

                if rel_is_single_or_partitioned || is_left_driven_lateral {
                    // We can use the limit for estimates if:
                    // a) we have a limit, and
                    // b) we're either:
                    //    * querying a single relation OR
                    //    * querying partitions of a partitioned table OR
                    //    * we're in a LEFT JOIN LATERAL where the left side drives the query
                    Some((*builder.args().root).limit_tuples)
                } else {
                    None
                }
            } else {
                maybe_limit_from_parse(builder.args().root)
            };

            // Get all columns referenced by this RTE throughout the entire query
            let referenced_columns = collect_maybe_fast_field_referenced_columns(rti, rel);

            // Save the count of referenced columns for decision-making
            custom_private.set_referenced_columns_count(referenced_columns.len());

            let is_maybe_topn = limit.is_some() && topn_pathkey_info.is_usable();

            // When collecting which_fast_fields, analyze the entire set of referenced columns,
            // not just those in the target list. To avoid execution-time surprises, the "planned"
            // fast fields must be a superset of the fast fields which are extracted from the
            // execution-time target list: see `assign_exec_method` for more info.
            custom_private.set_planned_which_fast_fields(
                exec_methods::fast_fields::collect_fast_fields(
                    target_list,
                    &referenced_columns,
                    rti,
                    &table,
                    &bm25_index,
                    false,
                )
                .into_iter()
                .collect(),
            );
            let maybe_ff = custom_private.maybe_ff();

            let query = SearchQueryInput::from(&quals);
            let norm_selec = if restrict_info.len() == 1 {
                (*restrict_info.get_ptr(0).unwrap()).norm_selec
            } else {
                UNASSIGNED_SELECTIVITY
            };

            let selectivity = if norm_selec != UNASSIGNED_SELECTIVITY {
                // we can use the norm_selec that already happened
                norm_selec
            } else if quals.contains_external_var() {
                // if the query has external vars (references to other relations which decide whether the rows in this
                // relation are visible) then we end up returning *everything* from _this_ relation
                FULL_RELATION_SELECTIVITY
            } else if quals.contains_exprs() {
                // if the query has expressions then it's parameterized and we have to guess something
                PARAMETERIZED_SELECTIVITY
            } else {
                // ask the index
                estimate_selectivity(&bm25_index, query.clone()).unwrap_or(UNKNOWN_SELECTIVITY)
            };

            // we must use this path if we need to do const projections for scores or snippets
            builder = builder.set_force_path(
                maybe_needs_const_projections || is_maybe_topn || quals.contains_all(),
            );

            custom_private.set_heaprelid(table.oid());
            custom_private.set_indexrelid(bm25_index.oid());
            custom_private.set_range_table_index(rti);
            custom_private.set_query(query);
            custom_private.set_limit(limit);
            custom_private.set_segment_count(segment_count);

            // Determine whether we might be able to sort.
            if is_maybe_topn && topn_pathkey_info.pathkeys().is_some() {
                let pathkeys = topn_pathkey_info.pathkeys().unwrap();
                custom_private.set_maybe_orderby_info(topn_pathkey_info.pathkeys());
            }

            // Choose the exec method type, and make claims about whether it is sorted.
            let exec_method_type = choose_exec_method(&custom_private, &topn_pathkey_info);
            custom_private.set_exec_method_type(exec_method_type);
            if custom_private.exec_method_type().is_sorted_topn() {
                // TODO: Note that the ExecMethodType does not actually hold a pg_sys::PathKey,
                // because we don't want/need to serialize them for execution.
                if let Some(pathkeys) = topn_pathkey_info.pathkeys() {
                    for pathkey in pathkeys {
                        builder = builder.add_path_key(pathkey);
                    }
                }
            }

            //
            // finally, we have enough information to set the cost and estimation information, and
            // to decide on parallelism
            //

            // calculate the total number of rows that might match the query, and the number of
            // rows that we expect that scan to return: these may be different in the case of a
            // `limit`.
            let reltuples = table.reltuples().unwrap_or(1.0) as f64;
            let total_rows = (reltuples * selectivity).max(1.0);
            let mut result_rows = total_rows.min(limit.unwrap_or(f64::MAX)).max(1.0);

            let nworkers = if (*builder.args().rel).consider_parallel {
                compute_nworkers(
                    custom_private.exec_method_type(),
                    limit,
                    total_rows,
                    segment_count,
                    quals.contains_external_var(),
                    quals.contains_correlated_param(builder.args().root),
                )
            } else {
                0
            };

            if nworkers > 0 {
                builder = builder.set_parallel(nworkers);

                // if we're likely to do a parallel scan, divide the result_rows by the number of workers
                // we're likely to use.  this lets Postgres make better decisions based on what
                // an individual parallel scan is actually going to return
                let processes = std::cmp::max(
                    1,
                    nworkers
                        + if pg_sys::parallel_leader_participation {
                            1
                        } else {
                            0
                        },
                );
                result_rows /= processes as f64;
            }

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
            let total_cost = startup_cost + (result_rows * per_tuple_cost);

            builder = builder.set_rows(result_rows);
            builder = builder.set_startup_cost(startup_cost);
            builder = builder.set_total_cost(total_cost);

            // indicate that we'll be doing projection ourselves
            builder = builder.set_flag(Flags::Projection);

            Some(builder.build(custom_private))
        }
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
        unsafe {
            let mut tlist = PgList::<pg_sys::TargetEntry>::from_pg(builder.args().tlist.as_ptr());

            // Store the length of the target list
            builder
                .custom_private_mut()
                .set_target_list_len(Some(tlist.len()));

            // Extract window_agg(json) calls from processed_tlist using expression tree walker
            // Similar to how uses_scores/uses_snippets work - walk the tree to find our placeholders
            // Note: This updates target_entry_index to match the processed_tlist positions
            let processed_tlist = (*builder.args().root).processed_tlist;

            let mut window_aggregates = deserialize_window_agg_placeholders(processed_tlist);

            if !window_aggregates.is_empty() {
                // Convert PostgresExpression filters to SearchQueryInput now that we have root
                // Note: root was not available in the planner hook, so we needed to delay this until now.
                let private_data = builder.custom_private();
                if let Some(heaprelid) = private_data.heaprelid() {
                    if let Some((_, bm25_index)) = rel_get_bm25_index(heaprelid) {
                        let root = builder.args().root;
                        let rti = private_data
                            .range_table_index()
                            .expect("range table index should be set");

                        resolve_window_aggregate_filters_at_plan_time(
                            &mut window_aggregates,
                            &bm25_index,
                            root,
                            rti,
                        );

                        // Validate that all fields in window aggregates exist in the index schema
                        if let Ok(schema) = crate::schema::SearchIndexSchema::open(&bm25_index) {
                            for window_agg in &window_aggregates {
                                for agg_type in window_agg.targetlist.aggregates() {
                                    if let Err(e) = agg_type.validate_fields(&schema) {
                                        pgrx::error!("{}", e);
                                    }
                                }
                            }
                        }
                    }
                }

                builder
                    .custom_private_mut()
                    .set_window_aggregates(window_aggregates);
            }

            let private_data = builder.custom_private();
            let rti = private_data
                .range_table_index()
                .expect("range table index should have been set")
                .try_into()
                .expect("range table index should not be negative");
            let processed_tlist = PgList::<pg_sys::TargetEntry>::from_pg(processed_tlist);

            let mut attname_lookup = HashMap::default();
            let funcoids: Vec<pg_sys::Oid> = score_funcoids()
                .iter()
                .copied()
                .chain(snippet_funcoids().iter().copied())
                .chain(snippets_funcoids().iter().copied())
                .chain(snippet_positions_funcoids().iter().copied())
                .collect();
            for te in processed_tlist.iter_ptr() {
                let func_vars_at_level =
                    pullout_funcexprs(te.cast(), &funcoids, rti, builder.args().root);

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
                    attname_lookup.insert((rti as Varno, (*var).varattno), attname);
                }
            }

            // Extract join-level snippet predicates for this relation
            // Get values we need before the mutable borrow

            // Extract the indexrelid early to avoid borrow checker issues later
            let indexrelid = private_data.indexrelid().expect("indexrelid should be set");
            let indexrel = PgSearchRelation::with_lock(indexrelid, pg_sys::AccessShareLock as _);
            let directory = MvccSatisfies::Snapshot.directory(&indexrel);
            let index = Index::open(directory)
                .expect("should be able to open index for snippet extraction");

            let base_query = builder
                .custom_private()
                .query()
                .clone()
                .expect("should have a SearchQueryInput");
            let join_predicates = extract_join_predicates(
                &PlannerContext::from_planner(builder.args().root),
                rti as pg_sys::Index,
                anyelement_query_input_opoid(),
                &indexrel,
                &base_query,
                true,
            );

            builder
                .custom_private_mut()
                .set_join_predicates(join_predicates);

            builder
                .custom_private_mut()
                .set_var_attname_lookup(attname_lookup);

            builder
                .custom_private_mut()
                .set_ambulkdelete_epoch(MetaPage::open(&indexrel).ambulkdelete_epoch());

            builder.build()
        }
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        unsafe {
            builder.custom_state().heaprelid = builder
                .custom_private()
                .heaprelid()
                .expect("heaprelid should have a value");
            builder.custom_state().indexrelid = builder
                .custom_private()
                .indexrelid()
                .expect("indexrelid should have a value");

            builder
                .custom_state()
                .open_relations(pg_sys::AccessShareLock as _);

            builder.custom_state().execution_rti =
                (*builder.args().cscan).scan.scanrelid as pg_sys::Index;

            builder.custom_state().exec_method_type =
                builder.custom_private().exec_method_type().clone();

            builder.custom_state().targetlist_len = builder.target_list().len();

            builder.custom_state().segment_count = builder.custom_private().segment_count();
            builder.custom_state().var_attname_lookup = builder
                .custom_private()
                .var_attname_lookup()
                .as_ref()
                .cloned()
                .expect("should have an attribute name lookup");

            let score_funcoids = score_funcoids();
            let snippet_funcoids = snippet_funcoids();
            let snippets_funcoids = snippets_funcoids();
            let snippet_positions_funcoids = snippet_positions_funcoids();

            builder.custom_state().score_funcoids = score_funcoids;
            builder.custom_state().snippet_funcoids = snippet_funcoids;
            builder.custom_state().snippets_funcoids = snippets_funcoids;
            builder.custom_state().snippet_positions_funcoids = snippet_positions_funcoids;
            builder.custom_state().need_scores = uses_scores(
                builder.target_list().as_ptr().cast(),
                score_funcoids,
                builder.custom_state().execution_rti,
            );

            // Store join snippet predicates in the scan state
            builder.custom_state().join_predicates =
                builder.custom_private().join_predicates().clone();

            // Store window aggregates in the scan state
            let window_aggs = builder.custom_private().window_aggregates().clone();
            builder.custom_state().window_aggregates = window_aggs;

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
                snippet_funcoids,
                snippets_funcoids,
                snippet_positions_funcoids,
            )
            .into_iter()
            .map(|field| (field, None))
            .collect();

            builder.custom_state().ambulkdelete_epoch =
                builder.custom_private().ambulkdelete_epoch();

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
                if let Some(explain_data) = &state.custom_state().parallel_explain_data {
                    explainer.add_json("Parallel Workers", &explain_data.workers);
                }
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
        if let Some(orderby_info) = state.custom_state().orderby_info().as_ref() {
            explainer.add_text(
                "   TopN Order By",
                orderby_info
                    .iter()
                    .map(|oi| match oi {
                        OrderByInfo {
                            feature: OrderByFeature::Field(fieldname),
                            direction,
                            ..
                        } => {
                            format!("{fieldname} {}", direction.as_ref())
                        }
                        OrderByInfo {
                            feature: OrderByFeature::Score,
                            direction,
                            ..
                        } => {
                            format!("pdb.score() {}", direction.as_ref())
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }

        if let Some(limit) = state.custom_state().limit() {
            explainer.add_unsigned_integer("   TopN Limit", limit as u64, None);
            if explainer.is_analyze() {
                explainer.add_unsigned_integer(
                    "   Queries",
                    state.custom_state().total_query_count().try_into().unwrap(),
                    None,
                );
            }
        }

        // Add a flag to indicate if the query is a full index scan
        let base_query = state.custom_state().base_search_query_input();

        // Only process the query if it's initialized
        // For EXPLAIN without ANALYZE, the query might not be initialized yet
        if !matches!(base_query, SearchQueryInput::Uninitialized) {
            if base_query.is_full_scan_query() {
                explainer.add_bool("Full Index Scan", true);
            }

            // Show query with integrated estimates if GUC is enabled and verbose
            if gucs::explain_recursive_estimates() && explainer.is_verbose() {
                // Get or create a search reader for estimates.
                // - EXPLAIN ANALYZE: search_reader is already initialized by begin_custom_scan
                // - EXPLAIN (without ANALYZE): search_reader is None, so we create a temporary
                //   reader using MvccSatisfies::LargestSegment for estimation purposes only
                let query_tree =
                    if let Some(search_reader) = state.custom_state().search_reader.as_ref() {
                        // EXPLAIN ANALYZE: use the existing search reader
                        search_reader
                            .build_query_tree_with_estimates(base_query.clone())
                            .expect("building query tree with estimates should not fail")
                    } else {
                        // EXPLAIN (without ANALYZE): create a temporary reader for estimates
                        let indexrel = state
                            .custom_state()
                            .indexrel
                            .as_ref()
                            .expect("indexrel should be open");

                        let temp_reader = SearchIndexReader::open_with_context(
                            indexrel,
                            base_query.clone(),
                            false,                         // don't need scores for estimates
                            MvccSatisfies::LargestSegment, // Use largest segment for estimation
                            None,                          // No expr_context needed for estimates
                            None,                          // No planstate needed for estimates
                        )
                        .expect("opening temporary search reader for estimates should not fail");

                        temp_reader
                            .build_query_tree_with_estimates(base_query.clone())
                            .expect("building query tree with estimates should not fail")
                    };

                explainer.add_query_with_estimates(&query_tree);
            } else {
                // Regular display without estimates
                explainer.add_query(base_query);
            }
        } else {
            explainer.add_text("Tantivy Query", "(query not yet initialized)");
        }
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        unsafe {
            // open the heap and index relations with the proper locks
            let rte = pg_sys::exec_rt_fetch(state.custom_state().execution_rti, estate);
            assert!(!rte.is_null());
            let lockmode = (*rte).rellockmode as pg_sys::LOCKMODE;

            state.custom_state_mut().open_relations(lockmode);

            // For EXPLAIN ANALYZE queries, we need to continue with full initialization
            // For EXPLAIN-only (without ANALYZE), begin_custom_scan is not called at all
            if eflags & (pg_sys::EXEC_FLAG_EXPLAIN_ONLY as i32) != 0 {
                // don't do anything else if we're only explaining the query
                return;
            }

            // setup the structures we need to do mvcc checking and heap fetching
            state.custom_state_mut().visibility_checker =
                Some(VisibilityChecker::with_rel_and_snap(
                    state.custom_state().heaprel(),
                    pg_sys::GetActiveSnapshot(),
                ));
            state.custom_state_mut().doc_from_heap_state =
                Some(HeapFetchState::new(state.custom_state().heaprel()));

            // and finally, get the custom scan itself properly initialized
            let tupdesc = state.custom_state().heaptupdesc();
            let planstate = state.planstate();

            pg_sys::ExecInitScanTupleSlot(
                estate,
                addr_of_mut!(state.csstate.ss),
                tupdesc,
                pg_sys::table_slot_callbacks(state.custom_state().heaprel().as_ptr()),
            );
            pg_sys::ExecInitResultTypeTL(addr_of_mut!(state.csstate.ss.ps));
            pg_sys::ExecAssignProjectionInfo(
                state.planstate(),
                (*state.csstate.ss.ss_ScanTupleSlot).tts_tupleDescriptor,
            );

            state
                .custom_state_mut()
                .init_expr_context(estate, planstate);
            state.runtime_context = state.csstate.ss.ps.ps_ExprContext;
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

                        let needs_special_projection = state.custom_state().need_scores()
                            || state.custom_state().need_snippets()
                            || state.custom_state().window_aggregate_results.is_some();

                        if !needs_special_projection {
                            //
                            // we don't need scores, snippets, or window aggregates
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
                                (*const_score_node).constvalue = score.into_datum().unwrap();
                                (*const_score_node).constisnull = false;
                            }

                            // Update window aggregate values
                            if let Some(agg_results) =
                                &state.custom_state().window_aggregate_results
                            {
                                for (te_idx, datum) in agg_results {
                                    if let Some(const_node) =
                                        state.custom_state().const_window_agg_nodes.get(te_idx)
                                    {
                                        (**const_node).constvalue = *datum;
                                        (**const_node).constisnull = false;
                                    }
                                }
                            }

                            // finally, do the projection
                            return per_tuple_context.switch_to(|_| {
                                // TODO: We go _back_ to the heap to get snippet information here
                                // inside of `make_snippet` and `get_snippet_positions`. It's possible
                                // that we could use a wider tuple slot to fetch the extra columns that
                                // we need during our initial lookup above (but then we'd need to copy
                                // into the correctly shaped slot for this scan).
                                maybe_project_snippets(state.custom_state(), ctid);

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

                ExecState::Virtual { slot } => {
                    state.custom_state_mut().virtual_tuple_count += 1;
                    return slot;
                }
            }
        }
    }

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        if let Some(parallel_state) = state.custom_state().parallel_state {
            state.custom_state_mut().parallel_explain_data =
                Some(unsafe { (*parallel_state).explain_data() });
        }
    }

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        // get some things dropped now
        drop(state.custom_state_mut().visibility_checker.take());
        drop(state.custom_state_mut().doc_from_heap_state.take());
        drop(state.custom_state_mut().search_reader.take());
        drop(std::mem::take(
            &mut state.custom_state_mut().snippet_generators,
        ));

        state.custom_state_mut().heaprel.take();
        state.custom_state_mut().indexrel.take();
    }
}

///
/// Choose and return an ExecMethodType based on the properties of the builder at planning time.
///
/// If the query can return "fast fields", make that determination here, falling back to the
/// [`NormalScanExecState`] if not.
///
/// We support [`MixedFastFieldExecState`] when there are a mix of string and numeric fast fields.
///
/// If we have failed to extract all relevant information at planning time, then the fast-field
/// execution methods might still fall back to `Normal` at execution time: see the notes in
/// `assign_exec_method` and `compute_exec_which_fast_fields`.
///
/// `pdb.score()`, `ctid`, and `tableoid` are considered fast fields for the purposes of
/// these specialized [`ExecMethod`]s.
///
fn choose_exec_method(privdata: &PrivateData, topn_pathkey_info: &PathKeyInfo) -> ExecMethodType {
    // See if we can use TopN.
    if let Some(limit) = privdata.limit() {
        if let Some(orderby_info) = privdata.maybe_orderby_info() {
            // having a valid limit and sort direction means we can do a TopN query
            // and TopN can do snippets
            return ExecMethodType::TopN {
                heaprelid: privdata.heaprelid().expect("heaprelid must be set"),
                limit,
                orderby_info: Some(orderby_info.clone()),
                window_aggregates: privdata.window_aggregates().clone(),
            };
        }
        if matches!(topn_pathkey_info, PathKeyInfo::None) {
            // we have a limit but no pathkeys at all. we can still go through our "top n"
            // machinery, but getting "limit" (essentially) random docs, which is what the user
            // asked for
            return ExecMethodType::TopN {
                heaprelid: privdata.heaprelid().expect("heaprelid must be set"),
                limit,
                orderby_info: None,
                window_aggregates: privdata.window_aggregates().clone(),
            };
        }
    }

    // Otherwise, see if we can use a fast fields method.
    if fast_fields::is_mixed_fast_field_capable(privdata) {
        return ExecMethodType::FastFieldMixed {
            which_fast_fields: privdata.planned_which_fast_fields().clone().unwrap(),
            limit: privdata.limit(),
        };
    }

    // Else, fall back to normal execution
    ExecMethodType::Normal
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
            orderby_info,
            window_aggregates,
        } => builder.custom_state().assign_exec_method(
            exec_methods::top_n::TopNScanExecState::new(heaprelid, limit, orderby_info),
            None,
        ),

        ExecMethodType::FastFieldMixed {
            which_fast_fields,
            limit,
        } => {
            if let Some(which_fast_fields) =
                compute_exec_which_fast_fields(builder, which_fast_fields)
            {
                builder.custom_state().assign_exec_method(
                    exec_methods::fast_fields::mixed::MixedFastFieldExecState::new(
                        which_fast_fields,
                        limit,
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
    let target_list = builder.target_list().as_ptr();
    let exec_which_fast_fields = unsafe {
        let custom_state = builder.custom_state();
        let indexrel = custom_state.indexrel();
        let execution_rti = custom_state.execution_rti;
        let heaprel = custom_state.heaprel();
        //
        // In order for our planned ExecMethodType to be accurate, this must always be a
        // subset of the fast fields which were extracted at planning time.
        exec_methods::fast_fields::collect_fast_fields(
            target_list,
            // At this point, all fast fields which we need to extract are listed directly
            // in our execution-time target list, so there is no need to extract from other
            // positions.
            &HashSet::default(),
            execution_rti,
            heaprel,
            indexrel,
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
        pgrx::log!(
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

/// Inject ParadeDB-specific placeholders (score, snippets, window aggregates) into the tuple slot
unsafe fn inject_pdb_placeholders(state: &mut CustomScanStateWrapper<PdbScan>) {
    let need_scores = state.custom_state().need_scores();
    let need_snippets = state.custom_state().need_snippets();
    let has_window_aggs = !state.custom_state().window_aggregates.is_empty();

    if !need_scores && !need_snippets && !has_window_aggs {
        // nothing to inject, use whatever we originally setup as our ProjectionInfo
        return;
    }

    // inject score and/or snippet placeholder [`pg_sys::Const`] nodes into what is a copy of the Plan's
    // targetlist.  We store this in our custom state's "placeholder_targetlist" for use during the
    // forced projection we must do later.
    let planstate = state.planstate();

    let (targetlist, const_score_node, const_snippet_nodes) = inject_placeholders(
        (*(*planstate).plan).targetlist,
        state.custom_state().planning_rti,
        state.custom_state().score_funcoids,
        state.custom_state().snippet_funcoids,
        state.custom_state().snippets_funcoids,
        state.custom_state().snippet_positions_funcoids,
        &state.custom_state().var_attname_lookup,
        &state.custom_state().snippet_generators,
    );

    // Now inject window aggregate placeholders
    let (targetlist, const_window_agg_nodes) = if !state.custom_state().window_aggregates.is_empty()
    {
        inject_window_aggregate_placeholders(targetlist, &state.custom_state().window_aggregates)
    } else {
        (targetlist, HashMap::default())
    };

    state.custom_state_mut().placeholder_targetlist = Some(targetlist);
    state.custom_state_mut().const_score_node = Some(const_score_node);
    state.custom_state_mut().const_snippet_nodes = const_snippet_nodes;
    state.custom_state_mut().const_window_agg_nodes = const_window_agg_nodes;
}

/// Inject placeholder Const nodes for window aggregates at execution time
/// At this point, the WindowFunc has been replaced with paradedb.window_agg(json) calls
/// This function finds those calls (which may be wrapped in other functions)
/// and replaces them with placeholder Const nodes that will be filled in during execution.
unsafe fn inject_window_aggregate_placeholders(
    targetlist: *mut pg_sys::List,
    window_aggs: &[WindowAggregateInfo],
) -> (*mut pg_sys::List, HashMap<usize, *mut pg_sys::Const>) {
    let mut const_nodes = HashMap::default();
    let tlist = PgList::<pg_sys::TargetEntry>::from_pg(targetlist);
    let window_agg_procid = window_agg_oid();

    // If window_agg function doesn't exist yet, return original targetlist
    if window_agg_procid == pg_sys::InvalidOid {
        return (targetlist, const_nodes);
    }

    // Process each window aggregate target entry
    let mut new_tlist = PgList::<pg_sys::TargetEntry>::new();

    for (idx, te) in tlist.iter_ptr().enumerate() {
        // Check if this target entry is one of our window aggregates
        let agg_info = window_aggs.iter().find(|a| a.target_entry_index == idx);

        if let Some(agg_info) = agg_info {
            // This target entry should contain a window_agg call (possibly wrapped)
            let (new_expr, const_node_opt) = replace_window_agg_with_const(
                (*te).expr as *mut pg_sys::Node,
                window_agg_procid,
                agg_info.result_type_oid(),
            );

            // Create a new target entry with the modified expression
            let new_te = pg_sys::flatCopyTargetEntry(te);
            (*new_te).expr = new_expr.cast();
            new_tlist.push(new_te);

            if let Some(const_node) = const_node_opt {
                const_nodes.insert(idx, const_node);
            }
        } else {
            // Not a window aggregate - just copy it
            new_tlist.push(te);
        }
    }

    (new_tlist.into_pg(), const_nodes)
}

// Helper function to recursively search and replace window_agg calls
//
// Note: This follows a similar recursive pattern to replace_in_node() in hook.rs,
// but operates at a different stage:
// - That function: Planning stage - replaces WindowFunc → window_agg() placeholder
// - This function: Execution stage - replaces window_agg() → Const placeholder for value injection
//
// TODO: This duplication could potentially be eliminated by moving to UPPERREL_WINDOW handling.
// See https://github.com/paradedb/paradedb/issues/3455
unsafe fn replace_window_agg_with_const(
    node: *mut pg_sys::Node,
    window_agg_procid: pg_sys::Oid,
    result_type_oid: pg_sys::Oid,
) -> (*mut pg_sys::Node, Option<*mut pg_sys::Const>) {
    if node.is_null() {
        return (node, None);
    }

    // Check if this is the window_agg FuncExpr
    if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
        if (*funcexpr).funcid == window_agg_procid {
            // Found it! Replace with a Const node
            let const_node = pg_sys::makeConst(
                result_type_oid,
                -1,
                pg_sys::DEFAULT_COLLATION_OID,
                if result_type_oid == pg_sys::INT8OID {
                    8
                } else {
                    -1
                },
                pg_sys::Datum::null(),
                true,                               // constisnull
                result_type_oid == pg_sys::INT8OID, // constbyval (true for INT8)
            );

            return (const_node.cast(), Some(const_node));
        }

        // Not window_agg, but might have window_agg as an argument
        let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
        let mut new_args = PgList::<pg_sys::Node>::new();
        let mut found_const = None;
        let mut modified = false;

        for arg in args.iter_ptr() {
            let (new_arg, const_opt) =
                replace_window_agg_with_const(arg, window_agg_procid, result_type_oid);
            if const_opt.is_some() {
                found_const = const_opt;
                modified = true;
            }
            if new_arg != arg {
                modified = true;
            }
            new_args.push(new_arg);
        }

        if modified {
            // Create a new FuncExpr with modified arguments
            let new_funcexpr = pg_sys::makeFuncExpr(
                (*funcexpr).funcid,
                (*funcexpr).funcresulttype,
                new_args.into_pg(),
                (*funcexpr).funccollid,
                (*funcexpr).inputcollid,
                (*funcexpr).funcformat,
            );
            return (new_funcexpr.cast(), found_const);
        }
    }

    (node, None)
}

/// Determine whether there are any pathkeys at all, and whether we might be able to push down
/// ordering in TopN.
///
/// If between 1 and 3 pathkeys are declared, and are indexed as fast, then return
/// `UsableAll(Vec<OrderByStyles>)` for them for use in TopN.
///
/// This function must be kept in sync with `validate_topn_compatibility` in `hook.rs` to ensure
/// that queries validated during the planner hook phase can be executed by the custom scan.
unsafe fn pullup_topn_pathkeys(
    builder: &mut CustomPathBuilder<PdbScan>,
    rti: pg_sys::Index,
    schema: &SearchIndexSchema,
    root: *mut pg_sys::PlannerInfo,
) -> PathKeyInfo {
    match extract_pathkey_styles_with_sortability_check(
        root,
        rti,
        schema,
        |search_field| search_field.is_raw_sortable(),
        |search_field| search_field.is_lower_sortable(),
    ) {
        PathKeyInfo::UsableAll(styles) if styles.len() <= MAX_TOPN_FEATURES => {
            // TopN is the base scan's only executor which supports sorting, and supports up to
            // MAX_TOPN_FEATURES order-by clauses.
            PathKeyInfo::UsableAll(styles)
        }
        PathKeyInfo::UsableAll(styles) => {
            // Too many pathkeys were extracted.
            PathKeyInfo::Unusable(UnusableReason::TooManyColumns {
                count: styles.len(),
                max: MAX_TOPN_FEATURES,
            })
        }
        PathKeyInfo::UsablePrefix(styles) => {
            // TopN cannot execute for a prefix of pathkeys, because it eliminates results before
            // the suffix of the pathkey comes into play.
            PathKeyInfo::Unusable(UnusableReason::PrefixOnly {
                matched: styles.len(),
            })
        }
        pki @ (PathKeyInfo::None | PathKeyInfo::Unusable(_)) => pki,
    }
}

#[inline(always)]
pub fn is_block_all_visible(
    heaprel: &PgSearchRelation,
    vmbuff: &mut pg_sys::Buffer,
    heap_blockno: pg_sys::BlockNumber,
) -> bool {
    unsafe {
        let status = pg_sys::visibilitymap_get_status(heaprel.as_ptr(), heap_blockno, vmbuff);
        status != 0
    }
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

#[rustfmt::skip]
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

        // Despite being part of FieldedQuery, these do not use a field, as far as the user knows
        SearchQueryInput::FieldedQuery {  query: pdb::Query::All, ..} |
        SearchQueryInput::FieldedQuery {  query: pdb::Query::Empty, ..} => false,

        // These are NOT search predicates (they're range/exists/other predicates)
        SearchQueryInput::FieldedQuery { query: pdb::Query::Range { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::RangeContains { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::RangeIntersects { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::RangeTerm { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::RangeWithin { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::Exists, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::FastFieldRangeWeight { .. }, .. }
        | SearchQueryInput::MoreLikeThis { .. } => false,

        // These are search predicates that use the @@@ operator
        SearchQueryInput::FieldedQuery { query: pdb::Query::ParseWithField { query_string, .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::Parse { query_string, .. }, .. } => {
            // For ParseWithField, check if it's a text search or a range query
            !is_range_query_string(query_string)
        }
        SearchQueryInput::Parse { .. }
        | SearchQueryInput::TermSet { .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::UnclassifiedString { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::UnclassifiedArray { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::ScoreAdjusted { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::TermSet { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::Term { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::Phrase { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::PhraseArray { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::Proximity { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::TokenizedPhrase { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::PhrasePrefix { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::FuzzyTerm { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::Match { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::MatchArray { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::Regex { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::RegexPhrase { .. }, .. } => true,

        // // Term with no field is not a search predicate
        // NB:  We don't support unqualified term queries anymore
        // SearchQueryInput::Term { field: None, .. } => false,

        // Postgres expressions are unknown, assume they could be search predicates
        SearchQueryInput::PostgresExpression { .. } => true,

        // HeapFilter contains search predicates
        SearchQueryInput::HeapFilter { indexed_query, .. } => {
            base_query_has_search_predicates(indexed_query, current_index_oid)
        }
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

/// Project configured snippets (if any).
///
/// Must be called inside the per-tuple `MemoryContext`.
unsafe fn maybe_project_snippets(state: &PdbScanState, ctid: u64) {
    if !state.need_snippets() {
        return;
    }

    for (snippet_type, const_snippet_nodes) in &state.const_snippet_nodes {
        match snippet_type {
            SnippetType::SingleText(_, config, _) => {
                let snippet = state.make_snippet(ctid, snippet_type);

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
            SnippetType::MultipleText(_, config, _, _) => {
                let snippets = state.make_snippets(ctid, snippet_type);

                for const_ in const_snippet_nodes {
                    match &snippets {
                        Some(array) => {
                            (**const_).constvalue = array.clone().into_datum().unwrap();
                            (**const_).constisnull = false;
                        }
                        None => {
                            (**const_).constvalue = pg_sys::Datum::null();
                            (**const_).constisnull = true;
                        }
                    }
                }
            }
            SnippetType::Positions(..) => {
                let positions = state.get_snippet_positions(ctid, snippet_type);

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
}

/// Check if the query contains a LEFT JOIN LATERAL pattern
///
/// This function verifies that the query has the specific structure:
/// `... LEFT JOIN LATERAL (...) ...`
///
/// We verify:
/// 1. The parse tree contains a LEFT JOIN node
/// 2. That LEFT JOIN's right side is marked as LATERAL in the range table
///
/// This enables TopN optimization because LEFT JOIN semantics guarantee all
/// left-side rows are preserved. If WHERE/ORDER BY/LIMIT only reference the
/// left table, we can safely apply TopN to the left scan before the join.
unsafe fn is_left_join_lateral(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
) -> bool {
    // Check if this is a join query
    if !(*root).hasJoinRTEs {
        return false;
    }

    // Check if this is a base relation (not a join itself)
    if (*rel).reloptkind != pg_sys::RelOptKind::RELOPT_BASEREL {
        return false;
    }

    // Check the parse tree for LEFT JOIN patterns with LATERAL
    // We need to verify:
    // 1. There's a LEFT JOIN in the query
    // 2. The right side of that LEFT JOIN is LATERAL
    //
    // We use a combination of checks:
    // - Parse tree: to find LEFT JOIN structure
    // - RTE lateral flag: to confirm the right side is LATERAL

    // First, quickly check if any LATERAL references exist at all
    let simple_rel_array = (*root).simple_rel_array;
    if simple_rel_array.is_null() {
        return false;
    }

    let mut has_lateral = false;
    let simple_rel_array_size = (*root).simple_rel_array_size;
    for i in 1..simple_rel_array_size {
        let other_rel = *simple_rel_array.add(i as usize);
        if !other_rel.is_null() && !(*other_rel).lateral_relids.is_null() {
            has_lateral = true;
            break;
        }
    }

    if !has_lateral {
        return false;
    }

    // Now check the parse tree for LEFT JOIN structure
    let jointree = (*(*root).parse).jointree;
    if jointree.is_null() || (*jointree).fromlist.is_null() {
        return false;
    }

    let fromlist = PgList::<pg_sys::Node>::from_pg((*jointree).fromlist);
    for node in fromlist.iter_ptr() {
        if has_left_join_lateral_pattern(node, (*root).parse) {
            return true;
        }
    }

    false
}

/// Recursively check if a node contains a LEFT JOIN LATERAL pattern
unsafe fn has_left_join_lateral_pattern(
    node: *mut pg_sys::Node,
    query: *mut pg_sys::Query,
) -> bool {
    if node.is_null() {
        return false;
    }

    if let Some(join_expr) = nodecast!(JoinExpr, T_JoinExpr, node) {
        // Check if this is a LEFT JOIN
        if (*join_expr).jointype == pg_sys::JoinType::JOIN_LEFT {
            // Check if the right side has LATERAL
            if is_lateral_subquery((*join_expr).rarg, query) {
                return true;
            }
        }

        // Recursively check nested joins
        if has_left_join_lateral_pattern((*join_expr).larg, query) {
            return true;
        }
        if has_left_join_lateral_pattern((*join_expr).rarg, query) {
            return true;
        }
    }

    false
}

/// Check if a node represents a LATERAL subquery
unsafe fn is_lateral_subquery(node: *mut pg_sys::Node, query: *mut pg_sys::Query) -> bool {
    if node.is_null() || query.is_null() {
        return false;
    }

    // Check if it's a RangeTblRef pointing to a LATERAL RTE
    if let Some(rtref) = nodecast!(RangeTblRef, T_RangeTblRef, node) {
        let rtable = (*query).rtable;
        if !rtable.is_null() {
            let rte = pg_sys::rt_fetch((*rtref).rtindex as pg_sys::Index, rtable);
            if !rte.is_null() && (*rte).lateral {
                return true;
            }
        }
    }

    // For nested joins, recursively check
    if let Some(join_expr) = nodecast!(JoinExpr, T_JoinExpr, node) {
        if is_lateral_subquery((*join_expr).larg, query) {
            return true;
        }
        if is_lateral_subquery((*join_expr).rarg, query) {
            return true;
        }
    }

    false
}

/// Verify WHERE clause only references the left table (current relation)
///
/// This method is used to check whether we can safely push down a LEFT LATERAL JOIN as TopN.
/// Because TopN eliminates rows _before_ the JOIN is actually executed, the WHERE clause (and
/// join condition) may only reference the left hand side of the join to avoid eliminating rows via the
/// limit which would be filtered by conditions on the right hand side.
unsafe fn where_clause_only_references_left(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
) -> bool {
    // Get WHERE clause
    let quals = if !(*root).parse.is_null()
        && !(*(*root).parse).jointree.is_null()
        && !(*(*(*root).parse).jointree).quals.is_null()
    {
        (*(*(*root).parse).jointree).quals
    } else {
        return true; // No WHERE clause means it only references left
    };

    // Walk the quals to check if they only reference our relation
    #[pgrx::pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        if let Some(var) = nodecast!(Var, T_Var, node) {
            let rti = *(data as *const pg_sys::Index);
            // If we find a Var that's not from our relation, return true (fail)
            if (*var).varno as i32 != rti as i32 && (*var).varno > 0 {
                return true;
            }
        }

        pg_sys::expression_tree_walker(node, Some(walker), data)
    }

    // If walker returns true, it found a reference to another relation
    !walker(quals, &rti as *const _ as *mut _)
}
