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
mod pushdown;
mod qual_inspect;
mod scan_state;
mod solve_expr;

use crate::api::operator::{
    anyelement_query_input_opoid, attname_from_var, estimate_selectivity, find_var_relation,
};
use crate::api::Cardinality;
use crate::api::{HashMap, HashSet};
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
use crate::postgres::customscan::pdbscan::qual_inspect::extract_quals;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::postgres::customscan::{self, CustomScan, CustomScanState};
use crate::postgres::rel_get_bm25_index;
use crate::postgres::visibility_checker::VisibilityChecker;
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;
use crate::{gucs, FULL_RELATION_SELECTIVITY, UNASSIGNED_SELECTIVITY};
use crate::{nodecast, DEFAULT_STARTUP_COST, PARAMETERIZED_SELECTIVITY, UNKNOWN_SELECTIVITY};
use pgrx::pg_sys::CustomExecMethods;
use pgrx::{direct_function_call, pg_sys, IntoDatum, PgList, PgMemoryContexts, PgRelation};
use std::ffi::CStr;
use std::ptr::addr_of_mut;
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
            for (snippet_type, generator) in &mut snippet_generators {
                let mut new_generator = state
                    .custom_state()
                    .search_reader
                    .as_ref()
                    .unwrap()
                    .snippet_generator(
                        snippet_type.field(),
                        state.custom_state().search_query_input(),
                    );

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
        unsafe {
            let (restrict_info, ri_type) = builder.restrict_info();
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

            let directory = MVCCDirectory::snapshot(bm25_index.oid());
            let index = Index::open(directory).expect("custom_scan: should be able to open index");
            let schema = SearchIndexSchema::open(index.schema(), &bm25_index);
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
            let mut uses_our_operator = false;
            let quals = extract_quals(
                root,
                rti,
                restrict_info.as_ptr().cast(),
                anyelement_query_input_opoid(),
                ri_type,
                &schema,
                &mut uses_our_operator,
            );

            if !uses_our_operator {
                // for now, we're not going to submit our custom scan for queries that don't also
                // use our `@@@` operator.  Perhaps in the future we can do this, but we don't want to
                // circumvent Postgres' other possible plans that might do index scans over a btree
                // index or something
                return None;
            }

            let Some(quals) = quals else {
                // if we are not able to push down all of the quals, then do not propose the custom
                // scan, as that would mean executing filtering against heap tuples (which amounts
                // to a join, and would require more planning).
                return None;
            };
            let query = SearchQueryInput::from(&quals);
            let norm_selec = if restrict_info.len() == 1 {
                (*restrict_info.get_ptr(0).unwrap()).norm_selec
            } else {
                UNASSIGNED_SELECTIVITY
            };

            let mut selectivity = if let Some(limit) = limit {
                // use the limit
                limit
                    / table
                        .reltuples()
                        .map(|n| n as Cardinality)
                        .unwrap_or(UNKNOWN_SELECTIVITY)
            } else if norm_selec != UNASSIGNED_SELECTIVITY {
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
                estimate_selectivity(&bm25_index, &query).unwrap_or(UNKNOWN_SELECTIVITY)
            };

            // we must use this path if we need to do const projections for scores or snippets
            builder = builder
                .set_force_path(maybe_needs_const_projections || is_topn || quals.contains_all());

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

            if pathkey.is_some()
                && !is_topn
                && fast_fields::is_string_agg_capable_with_prereqs(builder.custom_private())
                    .is_some()
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
            if builder
                .custom_private()
                .exec_method_type()
                .is_sorted(nworkers)
            {
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

            builder = builder.set_rows(rows);
            builder = builder.set_startup_cost(startup_cost);
            builder = builder.set_total_cost(total_cost);

            // indicate that we'll be doing projection ourselves
            builder = builder.set_flag(Flags::Projection);

            Some(builder.build())
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

            // Check if this is a join node (scanrelid = 0) or a scan node
            if builder.is_join() {
                // This is a join node - handle differently
                pgrx::warning!("ParadeDB: Planning custom join path with scanrelid = 0");

                // For join nodes, we need to ensure the target list is properly set up
                // The target list should contain all the columns that the upper plan nodes expect

                // For now, we'll use the target list as-is since it should already contain
                // the correct entries for the join result
                pgrx::warning!("ParadeDB: Join target list has {} entries", tlist.len());

                // Set up basic join metadata in private data
                // This will be expanded in later milestones

                return builder.build();
            }

            // Original scan node logic continues here
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
                );

                for (funcexpr, var) in func_vars_at_level {
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
                    let attname = attname_from_var(builder.args().root, var)
                        .1
                        .expect("function call argument should be a column name");
                    attname_lookup.insert(((*var).varno, (*var).varattno), attname);
                }
            }

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
            let scanrelid = (*builder.args().cscan).scan.scanrelid;

            if scanrelid == 0 {
                // This is a join node - handle differently
                pgrx::warning!(
                    "ParadeDB: Creating custom scan state for join node (scanrelid = 0)"
                );

                // For join nodes, we don't have specific heap/index relations
                // We'll need to handle this differently in execution
                // For now, set up minimal state that won't crash

                builder.custom_state().execution_rti = 0; // No specific relation
                builder.custom_state().exec_method_type = ExecMethodType::Normal;
                builder.custom_state().targetlist_len = builder.target_list().len();

                // Set default values for join execution
                builder.custom_state().limit = None;
                builder.custom_state().sort_field = None;
                builder.custom_state().sort_direction = None;

                // For joins, we'll need a different execution strategy
                // This will be implemented in later milestones

                return builder.build();
            }

            // Original scan node logic continues here
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

            // store our query into our custom state too
            let base_query = builder
                .custom_private()
                .query()
                .clone()
                .expect("should have a SearchQueryInput");
            builder
                .custom_state()
                .set_base_search_query_input(base_query);

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
            if explainer.is_analyze() && state.custom_state().retry_count > 0 {
                explainer.add_unsigned_integer(
                    "   Invisible Tuple Retries",
                    state.custom_state().retry_count as u64,
                    None,
                );
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
        unsafe {
            // Check if this is a join node
            if state.custom_state().execution_rti == 0 {
                pgrx::warning!("ParadeDB: Beginning custom scan for join node");

                // For join nodes, we don't open specific relations
                // Instead, we'll need to handle the join execution differently
                // For now, just set up minimal state for EXPLAIN

                if eflags & (pg_sys::EXEC_FLAG_EXPLAIN_ONLY as i32) != 0 {
                    // For EXPLAIN, we still need to set up basic tuple slot infrastructure
                    // Use a generic tuple descriptor for the result
                    let tupdesc =
                        pg_sys::CreateTemplateTupleDesc(state.custom_state().targetlist_len as _);
                    pg_sys::ExecInitScanTupleSlot(
                        estate,
                        addr_of_mut!(state.csstate.ss),
                        tupdesc,
                        pg_sys::table_slot_callbacks(std::ptr::null_mut()),
                    );
                    pg_sys::ExecInitResultTypeTL(addr_of_mut!(state.csstate.ss.ps));
                    return;
                }

                // For actual execution, we'll need to implement join logic
                // This will be expanded in later milestones
                pgrx::warning!("ParadeDB: Join execution not yet implemented - returning early");
                return;
            }

            // Original scan node logic continues here
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
        // Check if this is a join node
        if state.custom_state().execution_rti == 0 {
            // Join execution not yet implemented - return EOF
            return std::ptr::null_mut();
        }

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
/// Choose and return an ExecMethodType based on the properties of the builder.
///
/// If the query can return "fast fields", make that determination here, falling back to the
/// [`NormalScanExecState`] if not.
///
/// We support [`StringFastFieldExecState`] when there's 1 fast field and it's a string, or
/// [`NumericFastFieldExecState`] when there's one or more numeric fast fields, or
/// [`MixedFastFieldExecState`] when there are multiple string fast fields or a mix of string
/// and numeric fast fields.
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
    } else if fast_fields::is_numeric_fast_field_capable(privdata)
        && gucs::is_fast_field_exec_enabled()
    {
        // Check for numeric-only fast fields first because they're more selective
        ExecMethodType::FastFieldNumeric {
            which_fast_fields: privdata.planned_which_fast_fields().clone().unwrap(),
        }
    } else if fast_fields::is_string_agg_capable(privdata).is_some()
        && gucs::is_fast_field_exec_enabled()
    {
        let field = fast_fields::is_string_agg_capable(privdata).unwrap();
        ExecMethodType::FastFieldString {
            field,
            which_fast_fields: privdata.planned_which_fast_fields().clone().unwrap(),
        }
    } else if gucs::is_mixed_fast_field_exec_enabled() {
        // Use MixedFastFieldExec if enabled
        //
        // We'd suggest using MixedFastFieldExec as the last resort (default) at the planning
        // stage, but we will fall back to NormalExecState (in assign_exec_method) if we can't
        // execute using MixedFastFieldExec with the given fields and possibly expressions.
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
/// Currently, if MixedFastFieldExecState is chosen, we will fall back to NormalScanExecState if
/// we fail to extract the superset of fields during planning time which was needed at execution
/// time.
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
        let schema = SearchIndexSchema::open(index.schema(), &indexrel);

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

    let is_missing_fast_fields = exec_which_fast_fields.is_empty()
        || exec_which_fast_fields
            .iter()
            .any(|ff| !planned_which_fast_fields.contains(ff));

    if is_missing_fast_fields {
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
                    if schema.is_field_lower_sortable(att.name()) {
                        return Some(OrderByStyle::Field(first_pathkey, att.name().to_string()));
                    }
                }
            } else if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, expr) {
                if let Some(var) = nodecast!(Var, T_Var, (*relabel).arg) {
                    let (heaprelid, attno, _) = find_var_relation(var, root);
                    let heaprel = PgRelation::with_lock(heaprelid, pg_sys::AccessShareLock as _);
                    let tupdesc = heaprel.tuple_desc();
                    if let Some(att) = tupdesc.get(attno as usize - 1) {
                        if schema.is_field_raw_sortable(att.name()) {
                            return Some(OrderByStyle::Field(
                                first_pathkey,
                                att.name().to_string(),
                            ));
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
                    if schema.is_field_raw_sortable(att.name()) {
                        return Some(OrderByStyle::Field(first_pathkey, att.name().to_string()));
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
pub unsafe fn bms_iter(bms: *mut pg_sys::Bitmapset) -> impl Iterator<Item = pg_sys::Index> {
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
pub unsafe fn bms_is_empty(bms: *mut pg_sys::Bitmapset) -> bool {
    bms_iter(bms).next().is_none()
}

pub unsafe fn get_rel_name_from_rti_list(
    rtis: *mut pg_sys::Bitmapset,
    root: *mut pg_sys::PlannerInfo,
) -> Vec<String> {
    bms_iter(rtis)
        .map(|rti| get_rel_name_from_rti(rti, root))
        .collect()
}

pub unsafe fn get_rel_name_from_rti(rti: pg_sys::Index, root: *mut pg_sys::PlannerInfo) -> String {
    let rte = pg_sys::rt_fetch(rti, (*(*root).parse).rtable);
    if (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION {
        get_rel_name((*rte).relid)
    } else {
        format!("rti_{}_non_rel", rti)
    }
}

pub unsafe fn get_rel_name(relid: pg_sys::Oid) -> String {
    let relname = pg_sys::get_rel_name(relid);
    if relname.is_null() {
        format!("rti_{}", relid)
    } else {
        std::ffi::CStr::from_ptr(relname)
            .to_string_lossy()
            .to_string()
    }
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
        // NOTE: Unless we encounter the second type of fallback in `assign_exec_method`, then we
        // can be reasonably confident that directly inspecting Vars is sufficient. We haven't seen
        // it yet in the wild.
    }

    referenced_columns
}
