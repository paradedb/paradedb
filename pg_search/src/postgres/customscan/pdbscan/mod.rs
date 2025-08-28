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
pub mod parallel;
mod privdat;
mod projections;
mod scan_state;
mod solve_expr;

use crate::api::operator::{anyelement_query_input_opoid, estimate_selectivity};
use crate::api::{HashMap, HashSet, OrderByFeature, OrderByInfo};
use crate::gucs;
use crate::index::fast_fields_helper::WhichFastField;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::{SearchIndexReader, MAX_TOPN_FEATURES};
use crate::postgres::customscan::builders::custom_path::{
    restrict_info, CustomPathBuilder, ExecMethodType, Flags, OrderByStyle, RestrictInfoType,
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
use crate::postgres::customscan::pdbscan::parallel::{compute_nworkers, list_segment_ids};
use crate::postgres::customscan::pdbscan::privdat::PrivateData;
use crate::postgres::customscan::pdbscan::projections::score::{is_score_func, uses_scores};
use crate::postgres::customscan::pdbscan::projections::snippet::{
    snippet_funcoid, snippet_positions_funcoid, uses_snippets, SnippetType,
};
use crate::postgres::customscan::pdbscan::projections::{
    inject_placeholders, maybe_needs_const_projections, pullout_funcexprs,
};
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::postgres::customscan::qual_inspect::{
    extract_join_predicates, extract_quals, optimize_quals_with_heap_expr, Qual, QualExtractState,
};
use crate::postgres::customscan::score_funcoid;
use crate::postgres::customscan::{
    self, range_table, CustomScan, CustomScanState, RelPathlistHookArgs,
};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::var::find_var_relation;
use crate::postgres::visibility_checker::VisibilityChecker;
use crate::query::pdb_query::pdb;
use crate::query::SearchQueryInput;
use crate::schema::{SearchField, SearchIndexSchema};
use crate::{nodecast, DEFAULT_STARTUP_COST, PARAMETERIZED_SELECTIVITY, UNKNOWN_SELECTIVITY};
use crate::{FULL_RELATION_SELECTIVITY, UNASSIGNED_SELECTIVITY};
use pgrx::pg_sys::CustomExecMethods;
use pgrx::{direct_function_call, pg_sys, IntoDatum, PgList, PgMemoryContexts};
use std::ffi::CStr;
use std::ptr::addr_of_mut;
use std::sync::atomic::Ordering;
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
        let mvcc_style = unsafe {
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
        };

        let search_reader = if !expr_context.is_null() && !planstate.is_null() {
            // Use context-aware method for proper postgres expression evaluation
            SearchIndexReader::open_with_context(
                indexrel,
                search_query_input.clone(),
                need_scores,
                mvcc_style,
                expr_context,
                planstate,
            )
        } else {
            // Use regular method without context
            SearchIndexReader::open(
                indexrel,
                search_query_input.clone(),
                need_scores,
                mvcc_style,
            )
        }
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
                    .snippet_generator(snippet_type.field().root(), query_to_use.clone());

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

    unsafe fn extract_all_possible_quals(
        builder: &mut CustomPathBuilder<PdbScan>,
        root: *mut pg_sys::PlannerInfo,
        rti: pg_sys::Index,
        restrict_info: PgList<pg_sys::RestrictInfo>,
        ri_type: RestrictInfoType,
        indexrel: &PgSearchRelation,
        uses_score_or_snippet: bool,
    ) -> (Option<Qual>, RestrictInfoType, PgList<pg_sys::RestrictInfo>) {
        let mut state = QualExtractState::default();
        let mut quals = extract_quals(
            root,
            rti,
            restrict_info.as_ptr().cast(),
            anyelement_query_input_opoid(),
            ri_type,
            indexrel,
            false, // Base relation quals should not convert external to all
            &mut state,
        );

        // If we couldn't push down quals, try to push down quals from the join
        // This is only done if we have a join predicate, and only if we have used our operator
        let (quals, ri_type, restrict_info) = if quals.is_none() {
            let joinri: PgList<pg_sys::RestrictInfo> =
                PgList::from_pg(builder.args().rel().joininfo);
            let mut quals = extract_quals(
                root,
                rti,
                joinri.as_ptr().cast(),
                anyelement_query_input_opoid(),
                RestrictInfoType::Join,
                indexrel,
                true, // Join quals should convert external to all
                &mut state,
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
            // also used paradedb.score or paradedb.snippet functions in the query.
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
        if state.uses_our_operator || gucs::enable_custom_scan_without_operator() {
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
            let rte = pg_sys::rt_fetch(rti, (*(*root).parse).rtable);
            let relation_oid = (*rte).relid;
            optimize_quals_with_heap_expr(q);
        }

        quals.clone()
    }
}

impl customscan::ExecMethod for PdbScan {
    fn exec_methods() -> *const CustomExecMethods {
        <PdbScan as ParallelQueryCapable>::exec_methods()
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
            if matches!(ri_type, RestrictInfoType::None) {
                // this relation has no restrictions (WHERE clause predicates), so there's no need
                // for us to do anything
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
            #[cfg(any(feature = "pg16", feature = "pg17"))]
            let baserels = (*builder.args().root).all_query_rels;

            let limit = if (*builder.args().root).limit_tuples > -1.0 {
                // Check if this is a single relation or a partitioned table setup
                let rel_is_single_or_partitioned = pg_sys::bms_equal((*rel).relids, baserels)
                    || range_table::is_partitioned_table_setup(
                        builder.args().root,
                        (*rel).relids,
                        baserels,
                    );

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

            //
            // look for quals we can support
            //
            let (quals, ri_type, restrict_info) = Self::extract_all_possible_quals(
                &mut builder,
                root,
                rti,
                restrict_info,
                ri_type,
                &bm25_index,
                maybe_needs_const_projections,
            );

            let Some(quals) = quals else {
                // if we are not able to push down all of the quals, then do not propose the custom
                // scan, as that would mean executing filtering against heap tuples (which amounts
                // to a join, and would require more planning).
                return None;
            };

            // Check if this is a partial index and if the query is compatible with it
            if !bm25_index.rd_indpred.is_null() {
                // This is a partial index - we need to check if the query can be satisfied by it
                if !quals.is_query_compatible_with_partial_index() {
                    // The query cannot be satisfied by this partial index, fall back to heap scan
                    return None;
                }
            }

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
                // we can only (currently) do const projections if the first sort field is a score,
                // because we currently discard all but the first sort field, and so will not
                // produce a valid Score value. see TopNSearchResults.
                let orderby_supported = !maybe_needs_const_projections
                    || matches!(pathkeys.first(), Some(OrderByStyle::Score(..)));
                if orderby_supported {
                    custom_private.set_maybe_orderby_info(Some(pathkeys));
                }
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
                builder.args().root,
                rti as pg_sys::Index,
                anyelement_query_input_opoid(),
                &indexrel,
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
        if let Some(orderby_info) = state.custom_state().orderby_info().as_ref() {
            explainer.add_text(
                "   TopN Order By",
                orderby_info
                    .iter()
                    .map(|oi| match oi {
                        OrderByInfo {
                            feature: OrderByFeature::Field(fieldname),
                            direction,
                        } => {
                            format!("{fieldname} {}", direction.as_ref())
                        }
                        OrderByInfo {
                            feature: OrderByFeature::Score,
                            direction,
                        } => {
                            format!("paradedb.score() {}", direction.as_ref())
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
                    state.custom_state().query_count as u64,
                    None,
                );
            }
        }

        // Add a flag to indicate if the query is a full index scan
        if state
            .custom_state()
            .base_search_query_input()
            .is_full_scan_query()
        {
            explainer.add_bool("Full Index Scan", true);
        }
        explainer.add_query(state.custom_state().base_search_query_input());
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

            if eflags & (pg_sys::EXEC_FLAG_EXPLAIN_ONLY as i32) != 0 {
                // don't do anything else if we're only explaining the query
                return;
            }

            // setup the structures we need to do mvcc checking
            state.custom_state_mut().visibility_checker =
                Some(VisibilityChecker::with_rel_and_snap(
                    state.custom_state().heaprel(),
                    pg_sys::GetActiveSnapshot(),
                ));

            // and finally, get the custom scan itself properly initialized
            let tupdesc = state.custom_state().heaptupdesc();
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

            if state.custom_state_mut().has_postgres_expressions() {
                // we have some runtime Postgres expressions/sub-queries that need to be evaluated
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
                                (*const_score_node).constvalue = score.into_datum().unwrap();
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

                ExecState::Virtual { slot } => {
                    state.custom_state_mut().virtual_tuple_count += 1;
                    return slot;
                }
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
/// `paradedb.score()`, `ctid`, and `tableoid` are considered fast fields for the purposes of
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

pub enum PathKeyInfo {
    /// There are no PathKeys at all.
    None,
    /// There were PathKeys, but we cannot execute them.
    Unusable,
    /// There were PathKeys, but we can only execute a non-empty prefix of them.
    UsablePrefix(Vec<OrderByStyle>),
    /// There are some PathKeys, and we can execute all of them.
    UsableAll(Vec<OrderByStyle>),
}

impl PathKeyInfo {
    pub fn is_usable(&self) -> bool {
        match self {
            PathKeyInfo::UsablePrefix(_) | PathKeyInfo::UsableAll(_) => true,
            PathKeyInfo::None | PathKeyInfo::Unusable => false,
        }
    }

    pub fn pathkeys(&self) -> Option<&Vec<OrderByStyle>> {
        match self {
            PathKeyInfo::UsablePrefix(pathkeys) | PathKeyInfo::UsableAll(pathkeys) => {
                Some(pathkeys)
            }
            PathKeyInfo::None | PathKeyInfo::Unusable => None,
        }
    }
}

/// Determine whether there are any pathkeys at all, and whether we might be able to push down
/// ordering in TopN.
///
/// If between 1 and 3 pathkeys are declared, and are indexed as fast, then return
/// `UsableAll(Vec<OrderByStyles>)` for them for use in TopN.
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
        PathKeyInfo::UsableAll(_) => {
            // Too many pathkeys were extracted.
            PathKeyInfo::Unusable
        }
        PathKeyInfo::UsablePrefix(_) => {
            // TopN cannot execute for a prefix of pathkeys, because it eliminates results before
            // the suffix of the pathkey comes into play.
            PathKeyInfo::Unusable
        }
        pki @ (PathKeyInfo::None | PathKeyInfo::Unusable) => pki,
    }
}

/// Extract pathkeys from ORDER BY clauses using comprehensive expression handling
/// This function handles score functions, lower functions, relabel types, and regular variables
///
/// Returns PathKeyInfo indicating whether any PathKeys existed at all, and if so, whether they
/// might be usable via fast fields.
///
/// TODO: Used by both custom scans: move up one module.
pub unsafe fn extract_pathkey_styles_with_sortability_check<F1, F2>(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    schema: &SearchIndexSchema,
    regular_sortability_check: F1,
    lower_sortability_check: F2,
) -> PathKeyInfo
where
    F1: Fn(&SearchField) -> bool,
    F2: Fn(&SearchField) -> bool,
{
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);
    if pathkeys.is_empty() {
        return PathKeyInfo::None;
    }

    let mut pathkey_styles = Vec::new();
    for pathkey_ptr in pathkeys.iter_ptr() {
        let pathkey = pathkey_ptr;
        let equivclass = (*pathkey).pk_eclass;
        let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

        let mut found_valid_member = false;

        for member in members.iter_ptr() {
            let expr = (*member).em_expr;

            // Check if this is a score function
            if is_score_func(expr.cast(), rti) {
                pathkey_styles.push(OrderByStyle::Score(pathkey));
                found_valid_member = true;
                break;
            }
            // Check if this is a lower function
            else if let Some(var) = is_lower_func(expr.cast(), rti) {
                let (heaprelid, attno, _) = find_var_relation(var, root);
                if heaprelid != pg_sys::Oid::INVALID {
                    let heaprel =
                        PgSearchRelation::with_lock(heaprelid, pg_sys::AccessShareLock as _);
                    let tupdesc = heaprel.tuple_desc();
                    if let Some(att) = tupdesc.get(attno as usize - 1) {
                        if let Some(search_field) = schema.search_field(att.name()) {
                            if lower_sortability_check(&search_field) {
                                pathkey_styles
                                    .push(OrderByStyle::Field(pathkey, att.name().into()));
                                found_valid_member = true;
                                break;
                            }
                        }
                    }
                }
            }
            // Check if this is a RelabelType expression
            else if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, expr) {
                if let Some(var) = nodecast!(Var, T_Var, (*relabel).arg) {
                    let (heaprelid, attno, _) = find_var_relation(var, root);
                    if heaprelid != pg_sys::Oid::INVALID {
                        let heaprel =
                            PgSearchRelation::with_lock(heaprelid, pg_sys::AccessShareLock as _);
                        let tupdesc = heaprel.tuple_desc();
                        if let Some(att) = tupdesc.get(attno as usize - 1) {
                            if let Some(search_field) = schema.search_field(att.name()) {
                                if regular_sortability_check(&search_field) {
                                    pathkey_styles
                                        .push(OrderByStyle::Field(pathkey, att.name().into()));
                                    found_valid_member = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            // Check if this is a regular Var (column reference)
            else if let Some(var) = nodecast!(Var, T_Var, expr) {
                let (heaprelid, attno, _) = find_var_relation(var, root);
                if heaprelid != pg_sys::Oid::INVALID {
                    let heaprel =
                        PgSearchRelation::with_lock(heaprelid, pg_sys::AccessShareLock as _);
                    let tupdesc = heaprel.tuple_desc();
                    if let Some(att) = tupdesc.get(attno as usize - 1) {
                        if let Some(search_field) = schema.search_field(att.name()) {
                            if regular_sortability_check(&search_field) {
                                pathkey_styles
                                    .push(OrderByStyle::Field(pathkey, att.name().into()));
                                found_valid_member = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        // If we couldn't find any valid member for this pathkey, then we can't handle this series
        // of pathkeys.
        if !found_valid_member {
            if pathkey_styles.is_empty() {
                return PathKeyInfo::Unusable;
            } else {
                return PathKeyInfo::UsablePrefix(pathkey_styles);
            }
        }
    }

    PathKeyInfo::UsableAll(pathkey_styles)
}

/// Check if a node is a lower() function call for a specific relation
unsafe fn is_lower_func(node: *mut pg_sys::Node, rti: pg_sys::Index) -> Option<*mut pg_sys::Var> {
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

/// Helper function to get the OID of the text lower function
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
        SearchQueryInput::FieldedQuery { query: pdb::Query::ParseWithField { query_string, .. }, .. } => {
            // For ParseWithField, check if it's a text search or a range query
            !is_range_query_string(query_string)
        }
        SearchQueryInput::Parse { .. }
        | SearchQueryInput::TermSet { .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::UnclassifiedString { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::Boost { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::TermSet { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::Term { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::Phrase { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::Proximity { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::TokenizedPhrase { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::PhrasePrefix { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::FuzzyTerm { .. }, .. }
        | SearchQueryInput::FieldedQuery { query: pdb::Query::Match { .. }, .. }
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
