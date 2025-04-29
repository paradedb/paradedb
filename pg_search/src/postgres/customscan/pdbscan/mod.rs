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
    anyelement_query_input_opoid, anyelement_text_opoid, attname_from_var, estimate_selectivity,
    find_var_relation,
};
use crate::api::Cardinality;
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
use crate::postgres::customscan::pdbscan::exec_methods::fast_fields::{
    estimate_cardinality, is_string_agg_capable,
};
use crate::postgres::customscan::pdbscan::parallel::{compute_nworkers, list_segment_ids};
use crate::postgres::customscan::pdbscan::privdat::PrivateData;
use crate::postgres::customscan::pdbscan::projections::score::{
    is_score_func, score_funcoid, uses_scores,
};
use crate::postgres::customscan::pdbscan::projections::snippet::{
    snippet_funcoid, uses_snippets, SnippetInfo,
};
use crate::postgres::customscan::pdbscan::projections::{
    inject_placeholders, maybe_needs_const_projections, pullout_funcexprs,
};
use crate::postgres::customscan::pdbscan::qual_inspect::extract_quals;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::postgres::customscan::{self, CustomScan, CustomScanState};
use crate::postgres::rel_get_bm25_index;
use crate::postgres::visibility_checker::VisibilityChecker;
use crate::query::{AsHumanReadable, SearchQueryInput};
use crate::schema::SearchIndexSchema;
use crate::{nodecast, DEFAULT_STARTUP_COST, PARAMETERIZED_SELECTIVITY, UNKNOWN_SELECTIVITY};
use exec_methods::normal::NormalScanExecState;
use exec_methods::ExecState;
<<<<<<< HEAD
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys::{AsPgCStr, CustomExecMethods};
use pgrx::{direct_function_call, pg_sys, IntoDatum, PgList, PgMemoryContexts, PgRelation};
use std::collections::HashMap;
use std::convert::TryInto;
=======
use pgrx::pg_sys::CustomExecMethods;
use pgrx::{direct_function_call, pg_sys, IntoDatum, PgList, PgMemoryContexts, PgRelation};
use rustc_hash::FxHashMap;
>>>>>>> dev
use std::ffi::CStr;
use std::mem;
use std::ptr::addr_of_mut;
use tantivy::schema::Value;
use tantivy::snippet::SnippetGenerator;
use tantivy::Index;

#[derive(Default)]
pub struct PdbScan;

impl customscan::ExecMethod for PdbScan {
    fn exec_methods() -> *const CustomExecMethods {
        <PdbScan as ParallelQueryCapable>::exec_methods()
    }
}

impl CustomScan for PdbScan {
    const NAME: &'static CStr = c"ParadeDB Scan";

    type State = PdbScanState;
    type PrivateData = PrivateData;

    fn callback(mut builder: CustomPathBuilder<Self::PrivateData>) -> Option<pg_sys::CustomPath> {
        unsafe {
            let (restrict_info, ri_type) = builder.restrict_info();
            let rti = builder.args().rti;
            pgrx::warning!(
                "PdbScan: Got restrict_info for rti={}, ri_type={:?}, list len={}",
                rti,
                ri_type,
                if ri_type == RestrictInfoType::Join {
                    PgList::<pg_sys::Node>::from_pg(restrict_info.as_ptr().cast()).len()
                } else {
                    0
                }
            );
            if matches!(ri_type, RestrictInfoType::None) {
                // this relation has no restrictions (WHERE clause predicates), so there's no need
                // to continue with the scan
                pgrx::warning!("PdbScan: No restrictions found for rti={}, skipping", rti);

                // Add more context logging when no restrictions are found
                let root = builder.args().root;
                if !(*root).parent_root.is_null() {
                    pgrx::warning!("PdbScan: This is a subquery with parent_root");
                    // pgrx::warning!(
                    //     "PdbScan: Parent query has {} CTEs and {} RTEs",
                    //     if !(*(*root).parent_root).cteinfo.is_null() {
                    //         (*(*(*root).parent_root).cteinfo).nctes
                    //     } else {
                    //         0
                    //     },
                    //     if !(*(*root).parent_root).simple_rte_array.is_null() {
                    //         (*(*root).parent_root).simple_rel_array_size
                    //     } else {
                    //         0
                    //     }
                    // );

                    // Analyze parent query structure for diagnostics
                    // analyze_query_restrictions((*root).parent_root);
                } else {
                    // Not a subquery, analyze the main query
                    // analyze_query_restrictions(root);
                }

                return None;
            }

            let rti = builder.args().rti;
            // Log query node type (useful for CTEs and subqueries)
            let query_type = (*(*builder.args().root).parse).type_;
            pgrx::warning!(
                "PdbScan: Processing query type={:?} for rti={}",
                query_type,
                rti
            );

            // Log range table entry details
            let rte = builder.args().rte();
            pgrx::warning!(
                "PdbScan: RTE kind={}, relid={:?}, ctename={}",
                rte.rtekind,
                rte.relid,
                if !rte.ctename.is_null() {
                    std::ffi::CStr::from_ptr(rte.ctename).to_string_lossy()
                } else {
                    "NULL".into()
                }
            );

            // For CTEs, log additional information
            if !(*builder.args().root).parent_root.is_null() {
                pgrx::warning!("PdbScan: This is a subquery with parent_root");
            }

            // Check if this is a CTE query
            let ctequery = is_cte_query(builder.args().root);
            if ctequery {
                pgrx::warning!("PdbScan: This appears to be a CTE query");
            }

            let (table, bm25_index, is_join) = {
                let rte = builder.args().rte();

                // we only support plain relation and join rte's
                if rte.rtekind != pg_sys::RTEKind::RTE_RELATION
                    && rte.rtekind != pg_sys::RTEKind::RTE_JOIN
                {
                    return None;
                }

                // If this is a join RTE, we need to examine the join expression to find tables involved
                if rte.rtekind == pg_sys::RTEKind::RTE_JOIN {
                    // For join RTEs, we'll use the restrict_info instead of trying to find
                    // an index on the join relation itself. The BM25 operator will be found
                    // during qual extraction, so we'll set is_join to true and continue.
                    (
                        PgRelation::from_pg(std::ptr::null_mut()),
                        PgRelation::from_pg(std::ptr::null_mut()),
                        true,
                    )
                } else {
                    // For non-join RTEs, continue with the usual logic
                    // and we only work on plain relations
                    let relkind = pg_sys::get_rel_relkind(rte.relid) as u8;
                    if relkind != pg_sys::RELKIND_RELATION && relkind != pg_sys::RELKIND_MATVIEW {
                        return None;
                    }

                    // and that relation must have a `USING bm25` index
                    let (table, bm25_index) = rel_get_bm25_index(rte.relid)?;

                    (table, bm25_index, rte.rtekind == pg_sys::RTEKind::RTE_JOIN)
                }
            };

            let root = builder.args().root;

            // If this is a join, we need to examine the restrictinfo to find tables with BM25 indexes
            let (join_table, join_bm25_index) = if is_join {
                let (restrict_info, ri_type) = builder.restrict_info();
                if !matches!(ri_type, RestrictInfoType::None) {
                    // Look through the restrict_info to find tables that might have BM25 indexes
                    pgrx::warning!("Examining join condition for BM25 index in rti={}", rti);
                    find_bm25_index_in_join_condition(restrict_info.as_ptr().cast(), root)?
                } else {
                    return None;
                }
            } else {
                (table.clone(), bm25_index)
            };

            // Safety check to make sure we have valid relations
            if join_bm25_index.is_null() || join_table.is_null() {
                pgrx::warning!("Unable to find valid BM25 index for join relation");
                return None;
            }

            pgrx::warning!(
                "Found BM25 index: table={} index={}",
                join_table.name(),
                join_bm25_index.name()
            );

            let directory = MVCCDirectory::snapshot(join_bm25_index.oid());
            let index = Index::open(directory).expect("custom_scan: should be able to open index");
            let schema = SearchIndexSchema::open(index.schema(), &join_bm25_index);
            let pathkey = pullup_orderby_pathkey(&mut builder, rti, &schema, root);

            #[cfg(any(feature = "pg14", feature = "pg15"))]
            let baserels = (*builder.args().root).all_baserels;
            #[cfg(any(feature = "pg16", feature = "pg17"))]
            let baserels = (*builder.args().root).all_query_rels;

            let limit = if (*builder.args().root).limit_tuples > -1.0 {
                // Check if this is a single relation or a partitioned table setup
                let rel_is_single_or_partitioned =
                    pg_sys::bms_equal((*builder.args().rel).relids, baserels)
                        || is_partitioned_table_setup(
                            builder.args().root,
                            (*builder.args().rel).relids,
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
            builder
                .custom_private()
                .set_targetlist_len(PgList::<pg_sys::TargetEntry>::from_pg(target_list).len());
            let maybe_needs_const_projections = maybe_needs_const_projections(target_list.cast());
            let ff_cnt =
                exec_methods::fast_fields::count(&mut builder, rti, &table, &schema, target_list);
            let maybe_ff = builder.custom_private().maybe_ff();
            let is_topn = limit.is_some() && pathkey.is_some();
            builder
                .custom_private()
                .set_which_fast_fields(exec_methods::fast_fields::collect(
                    maybe_ff,
                    target_list,
                    rti,
                    &schema,
                    &table,
                ));

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
                pgrx::warning!("PdbScan: extract_quals failed, trying anyelement_text_opoid");
                let quals = extract_quals(
                    root,
                    rti,
                    restrict_info.as_ptr().cast(),
                    anyelement_text_opoid(),
                    ri_type,
                    &schema,
                    &mut uses_our_operator,
                );
                if uses_our_operator {
                    pgrx::warning!("PdbScan: extract_quals succeeded with anyelement_text_opoid");
                } else {
                    pgrx::warning!("PdbScan: extract_quals failed with anyelement_text_opoid");
                }
            } else {
                pgrx::warning!(
                    "PdbScan: extract_quals succeeded with anyelement_query_input_opoid"
                );
            }

            pgrx::warning!(
                "PdbScan: extract_quals returned uses_our_operator={}, has_quals={}",
                uses_our_operator,
                quals.is_some()
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

            let has_expressions = quals.contains_exprs();
            let selectivity = if let Some(limit) = limit {
                // use the limit
                limit
                    / table
                        .reltuples()
                        .map(|n| n as Cardinality)
                        .unwrap_or(UNKNOWN_SELECTIVITY)
            } else if restrict_info.len() == 1 {
                // we can use the norm_selec that already happened
                let norm_select = (*restrict_info.get_ptr(0).unwrap()).norm_selec;
                if norm_select != UNKNOWN_SELECTIVITY {
                    norm_select
                } else {
                    // assume PARAMETERIZED_SELECTIVITY
                    PARAMETERIZED_SELECTIVITY
                }
            } else {
                // ask the index
                if has_expressions {
                    // we have no idea, so assume PARAMETERIZED_SELECTIVITY
                    PARAMETERIZED_SELECTIVITY
                } else {
<<<<<<< HEAD
                    let query = SearchQueryInput::from(&quals);
                    estimate_selectivity(&join_bm25_index, &query).unwrap_or(UNKNOWN_SELECTIVITY)
=======
                    estimate_selectivity(&bm25_index, &query).unwrap_or(UNKNOWN_SELECTIVITY)
>>>>>>> dev
                }
            };

            // we must use this path if we need to do const projections for scores or snippets
            builder = builder.set_force_path(
                has_expressions
                    && (maybe_needs_const_projections || is_topn || quals.contains_all()),
            );

            builder.custom_private().set_heaprelid(join_table.oid());
            builder
                .custom_private()
                .set_indexrelid(join_bm25_index.oid());
            builder.custom_private().set_range_table_index(rti);
            builder.custom_private().set_query(query);
            builder.custom_private().set_limit(limit);

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

            let reltuples = table.reltuples().unwrap_or(1.0) as f64;
            let rows = (reltuples * selectivity).max(1.0);

            let per_tuple_cost = {
                // if we think we need scores, we need a much cheaper plan so that Postgres will
                // prefer it over all the others.
                if is_join || maybe_needs_const_projections {
                    0.0
                } else if maybe_ff {
                    // returns fields from fast fields
                    pg_sys::cpu_index_tuple_cost / 100.0
                } else {
                    // requires heap access to return fields
                    pg_sys::cpu_tuple_cost * 200.0
                }
            };

            let startup_cost = if is_join || maybe_needs_const_projections {
                0.0
            } else {
                DEFAULT_STARTUP_COST
            };

            let total_cost = startup_cost + (rows * per_tuple_cost);
            let segment_count = index.searchable_segments().unwrap_or_default().len();
            let nworkers = if (*builder.args().rel).consider_parallel {
                compute_nworkers(limit, segment_count, builder.custom_private().is_sorted())
            } else {
                0
            };

            builder.custom_private().set_segment_count(
                index
                    .searchable_segments()
                    .map(|segments| segments.len())
                    .unwrap_or(0),
            );
            builder = builder.set_rows(rows);
            builder = builder.set_startup_cost(startup_cost);
            builder = builder.set_total_cost(total_cost);
            builder = builder.set_flag(Flags::Projection);

            if pathkey.is_some()
                && !is_topn
                && is_string_agg_capable(builder.custom_private()).is_some()
            {
                let pathkey = pathkey.as_ref().unwrap();

                // we're going to do a StringAgg, and it may or may not be more efficient to use
                // parallel queries, depending on the cardinality of what we're going to select
                let parallel_scan_preferred = || -> bool {
                    let cardinality = {
                        let estimate = if let OrderByStyle::Field(_, field) = &pathkey {
                            // NB:  '4' is a magic number
                            estimate_cardinality(&join_bm25_index, field).unwrap_or(0) * 4
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
            } else if nworkers > 0 {
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

            Some(builder.build())
        }
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self::PrivateData>) -> pg_sys::CustomScan {
        unsafe {
            let private_data = builder.custom_private();

            let mut tlist = PgList::<pg_sys::TargetEntry>::from_pg(builder.args().tlist.as_ptr());

            let rti: i32 = private_data
                .range_table_index()
                .expect("range table index should have been set")
                .try_into()
                .expect("range table index should not be negative");
            let processed_tlist =
                PgList::<pg_sys::TargetEntry>::from_pg((*builder.args().root).processed_tlist);

            let mut attname_lookup = FxHashMap::default();
            let score_funcoid = score_funcoid();
            let snippet_funcoid = snippet_funcoid();
            for te in processed_tlist.iter_ptr() {
                let func_vars_at_level =
                    pullout_funcexprs(te.cast(), &[score_funcoid, snippet_funcoid], rti);

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
            builder.custom_state().heaprelid = builder
                .custom_private()
                .heaprelid()
                .expect("heaprelid should have a value");
            builder.custom_state().indexrelid = builder
                .custom_private()
                .indexrelid()
                .expect("indexrelid should have a value");

            builder.custom_state().rti = builder
                .custom_private()
                .range_table_index()
                .expect("range table index should have been set");

            builder.custom_state().exec_method_type =
                builder.custom_private().exec_method_type().clone();

            builder.custom_state().which_fast_fields =
                builder.custom_private().which_fast_fields().clone();

            builder.custom_state().targetlist_len = builder.target_list().len();

            // information about if we're sorted by score and our limit
            builder.custom_state().limit = builder.custom_private().limit();
            builder.custom_state().sort_field = builder.custom_private().sort_field();
            builder.custom_state().sort_direction = builder.custom_private().sort_direction();

            // store our query into our custom state too
            builder.custom_state().search_query_input = builder
                .custom_private()
                .query()
                .as_ref()
                .cloned()
                .expect("should have a SearchQueryInput");

            builder.custom_state().segment_count = builder.custom_private().segment_count();

            builder.custom_state().var_attname_lookup = builder
                .custom_private()
                .var_attname_lookup()
                .as_ref()
                .cloned()
                .expect("should have an attribute name lookup");

            let score_funcoid = score_funcoid();
            let snippet_funcoid = snippet_funcoid();

            builder.custom_state().score_funcoid = score_funcoid;
            builder.custom_state().snippet_funcoid = snippet_funcoid;

            builder.custom_state().need_scores = uses_scores(
                builder.target_list().as_ptr().cast(),
                score_funcoid,
                (*builder.args().cscan).scan.scanrelid as pg_sys::Index,
            );

            let node = builder.target_list().as_ptr().cast();
            let rti = builder.custom_state().rti;
            let attname_lookup = &builder.custom_state().var_attname_lookup;
            builder.custom_state().snippet_generators =
                uses_snippets(rti, attname_lookup, node, snippet_funcoid)
                    .into_iter()
                    .map(|field| (field, None))
                    .collect();

            let need_snippets = builder.custom_state().need_snippets();

            assign_exec_method(builder.custom_state());

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
        explainer.add_unsigned_integer(
            "Segment Count",
            state.custom_state().segment_count as u64,
            None,
        );

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

        let json_query = serde_json::to_string(&state.custom_state().search_query_input)
            .expect("query should serialize to json");
        explainer.add_text("Tantivy Query", &json_query);

        if explainer.is_verbose() {
            explainer.add_text(
                "Human Readable Query",
                state.custom_state().search_query_input.as_human_readable(),
            );
        }
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        unsafe {
            // open the heap and index relations with the proper locks
            let rte = pg_sys::exec_rt_fetch(state.custom_state().rti, estate);
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

            let planstate = state.planstate();
            let nexprs = state
                .custom_state_mut()
                .search_query_input
                .init_postgres_expressions(planstate);
            state.custom_state_mut().nexprs = nexprs;

            if nexprs > 0 {
                // we have some runtime Postgres expressions that need to be evaluated in `rescan_custom_scan`
                //
                // Our planstate's ExprContext isn't sufficiently configured for that, so we need to
                // make a new one and swap some pointers around

                // hold onto the planstate's current ExprContext
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
        pgrx::warning!(
            "PdbScan::rescan_custom_scan: Beginning rescan for rti={:?}, need_scores={}, with custom_state.nexprs={}",
            state.custom_state().rti,
            state.custom_state().need_scores(),
            state.custom_state().nexprs
        );

        // Enhanced logging for heap table RTE information
        if let Some(heaprel) = state.custom_state().heaprel {
            unsafe {
                let rel_id = (*heaprel).rd_id;
                pgrx::warning!(
                    "PdbScan::rescan_custom_scan: Scanning heap relation with OID={} for rti={:?}",
                    rel_id,
                    state.custom_state().rti
                );
            }
        }

        // Track if we're in a nested loop context
        unsafe {
            let parent_plan = (*state.csstate.ss.ps.plan).lefttree;
            if !parent_plan.is_null() {
                let plan_tag = (*parent_plan).type_;
                pgrx::warning!(
                    "PdbScan::rescan_custom_scan: Parent plan type={:?} for rti={:?}",
                    plan_tag,
                    state.custom_state().rti
                );

                // Check if we're in a nested loop context
                if plan_tag == pg_sys::NodeTag::T_NestLoop {
                    pgrx::warning!(
                        "PdbScan::rescan_custom_scan: NESTED LOOP JOIN DETECTED for rti={:?}! This is the critical case.",
                        state.custom_state().rti
                    );
                }
            }
        }

        // Log current state of search query input
        pgrx::warning!(
            "PdbScan::rescan_custom_scan: Current search_query_input={:?} for rti={:?}",
            state.custom_state().search_query_input,
            state.custom_state().rti
        );

        // Log memory context information
        pgrx::warning!(
            "PdbScan::rescan_custom_scan: Current memory context={:?} for rti={:?}",
            unsafe { pg_sys::CurrentMemoryContext },
            state.custom_state().rti
        );

        if state.custom_state().nexprs > 0 {
            let expr_context = state.runtime_context;
            pgrx::warning!(
                "PdbScan::rescan_custom_scan: Solving postgres expressions with context={:?} for rti={:?}",
                expr_context,
                state.custom_state().rti
            );

            state
                .custom_state_mut()
                .search_query_input
                .solve_postgres_expressions(expr_context);

            pgrx::warning!(
                "PdbScan::rescan_custom_scan: After solving expressions, search_query_input={:?} for rti={:?}",
                state.custom_state().search_query_input,
                state.custom_state().rti
            );
        }

        let need_snippets = state.custom_state().need_snippets();

        // Open the index
        let indexrel = state
            .custom_state()
            .indexrel
            .as_ref()
            .map(|indexrel| unsafe { PgRelation::from_pg(*indexrel) })
            .expect("custom_state.indexrel should already be open");

        pgrx::warning!(
            "PdbScan::rescan_custom_scan: Opening index relation {} for rti={:?}",
            indexrel.name(),
            state.custom_state().rti
        );

        let search_reader = SearchIndexReader::open(&indexrel, unsafe {
            if pg_sys::ParallelWorkerNumber == -1 {
                // the leader only sees snapshot-visible segments
                pgrx::warning!(
                    "PdbScan::rescan_custom_scan: Using MvccSatisfies::Snapshot for leader for rti={:?}",
                    state.custom_state().rti
                );
                MvccSatisfies::Snapshot
            } else {
                // the workers have their own rules, which is literally every segment
                // this is because the workers pick a specific segment to query that
                // is known to be held open/pinned by the leader but might not pass a ::Snapshot
                // visibility test due to concurrent merges/garbage collects
                pgrx::warning!(
                    "PdbScan::rescan_custom_scan: Using MvccSatisfies::ParallelWorker for worker for rti={:?}",
                    state.custom_state().rti
                );
                MvccSatisfies::ParallelWorker(list_segment_ids(
                    state.custom_state().parallel_state.expect(
                        "Parallel Custom Scan rescan_custom_scan should have a parallel state",
                    ),
                ))
            }
        })
        .expect("should be able to open the search index reader");

        // Log if the old reader is being replaced
        if state.custom_state().search_reader.is_some() {
            pgrx::warning!(
                "PdbScan::rescan_custom_scan: Replacing existing search reader for rti={:?}",
                state.custom_state().rti
            );
        }

        state.custom_state_mut().search_reader = Some(search_reader);

        // Reset exec_method and clear any previous search state
        let csstate = addr_of_mut!(state.csstate);
        pgrx::warning!(
            "PdbScan::rescan_custom_scan: Initializing new exec_method for rti={:?}",
            state.custom_state().rti
        );

        // // Check if prior search_results exist and log them
        // if let Some(exec_method) = &state.custom_state().exec_method {
        //     pgrx::warning!(
        //         "PdbScan::rescan_custom_scan: Resetting exec_method and associated search_results for rti={:?}",
        //         state.custom_state().rti
        //     );
        // }

        state.custom_state_mut().init_exec_method(csstate);

        if need_snippets {
            let mut snippet_generators: FxHashMap<
                SnippetInfo,
                Option<(tantivy::schema::Field, SnippetGenerator)>,
            > = state
                .custom_state_mut()
                .snippet_generators
                .drain()
                .collect();
            for (snippet_info, generator) in &mut snippet_generators {
                let mut new_generator = state
                    .custom_state()
                    .search_reader
                    .as_ref()
                    .unwrap()
                    .snippet_generator(
                        &snippet_info.field,
                        &state.custom_state().search_query_input,
                    );
                new_generator
                    .1
                    .set_max_num_chars(snippet_info.max_num_chars);
                *generator = Some(new_generator);
            }

            state.custom_state_mut().snippet_generators = snippet_generators;
        }

        unsafe {
            inject_score_and_snippet_placeholders(state);
        }

        pgrx::warning!(
            "PdbScan::rescan_custom_scan: Completed rescan for rti={:?}",
            state.custom_state().rti
        );
    }

    #[allow(clippy::blocks_in_conditions)]
    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        if state.custom_state().search_reader.is_none() {
            PdbScan::rescan_custom_scan(state);
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
                                    for (snippet_info, const_snippet_nodes) in
                                        &state.custom_state().const_snippet_nodes
                                    {
                                        let snippet =
                                            state.custom_state().make_snippet(ctid, snippet_info);

                                        for const_ in const_snippet_nodes {
                                            match &snippet {
                                                Some(text) => {
                                                    (**const_).constvalue =
                                                        text.into_datum().unwrap();
                                                    (**const_).constisnull = false;
                                                }
                                                None => {
                                                    (**const_).constvalue = pg_sys::Datum::null();
                                                    (**const_).constisnull = true;
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
/// [`NumericFastFieldExecState`] when there's one or more numeric fast fields
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
    } else if let Some(field) = exec_methods::fast_fields::is_string_agg_capable(privdata) {
        ExecMethodType::FastFieldString {
            field,
            which_fast_fields: privdata.which_fast_fields().clone().unwrap(),
        }
    } else if exec_methods::fast_fields::is_numeric_fast_field_capable(privdata) {
        ExecMethodType::FastFieldNumeric {
            which_fast_fields: privdata.which_fast_fields().clone().unwrap_or_default(),
        }
    } else {
        ExecMethodType::Normal
    }
}

fn assign_exec_method(custom_state: &mut PdbScanState) {
    match &custom_state.exec_method_type {
        ExecMethodType::Normal => custom_state.assign_exec_method(NormalScanExecState::default()),
        ExecMethodType::TopN {
            heaprelid,
            limit,
            sort_direction,
            need_scores,
        } => custom_state.assign_exec_method(exec_methods::top_n::TopNScanExecState::new(
            *heaprelid,
            *limit,
            *sort_direction,
            *need_scores,
        )),
        ExecMethodType::FastFieldString {
            field,
            which_fast_fields,
        } => custom_state.assign_exec_method(
            exec_methods::fast_fields::string::StringFastFieldExecState::new(
                field.to_owned(),
                which_fast_fields.clone(),
            ),
        ),
        ExecMethodType::FastFieldNumeric { which_fast_fields } => custom_state.assign_exec_method(
            exec_methods::fast_fields::numeric::NumericFastFieldExecState::new(
                which_fast_fields.clone(),
            ),
        ),
    }
}

/// Use the [`VisibilityChecker`] to lookup the [`SearchIndexScore`] document in the underlying heap
/// and if it exists return a formed [`TupleTableSlot`].
#[inline(always)]
fn check_visibility(
    state: &mut CustomScanStateWrapper<PdbScan>,
    ctid: u64,
    bslot: *mut pg_sys::BufferHeapTupleTableSlot,
) -> Option<*mut pg_sys::TupleTableSlot> {
    // Enhanced logging for the visibility check
    pgrx::warning!(
        "check_visibility: Checking visibility for ctid={}, rte={:?}, join_type={}, relation={}",
        ctid,
        state.custom_state().rti,
        if unsafe { pg_sys::CurrentMemoryContext.is_null() } {
            "Unknown"
        } else {
            "Known"
        },
        state.custom_state().heaprelname()
    );

    // Extract ItemPointer information for more detailed debugging
    unsafe {
        let mut tid = pg_sys::ItemPointerData::default();
        crate::postgres::utils::u64_to_item_pointer(ctid, &mut tid);
        let blockno = item_pointer_get_block_number(&tid);
        let offno = pg_sys::ItemPointerGetOffsetNumber(&tid);
        pgrx::warning!(
            "check_visibility: Decomposed ctid={} to blockno={}, offno={} for rti={:?}",
            ctid,
            blockno,
            offno,
            state.custom_state().rti
        );

        // Try to detect if we're in a nested loop join by examining scan state and context
        let is_nested_loop = (*state.csstate.ss.ps.plan).lefttree != std::ptr::null_mut()
            && (*(*state.csstate.ss.ps.plan).lefttree).type_ == pg_sys::NodeTag::T_NestLoop;

        if is_nested_loop {
            pgrx::warning!(
                "check_visibility: In nested loop join context for rti={:?}, ctid={}",
                state.custom_state().rti,
                ctid
            );

            // Try to examine the outer tuple info in the join context
            let outer_tuple = (*state.csstate.ss.ps.ps_ExprContext).ecxt_outertuple;
            if !outer_tuple.is_null() {
                pgrx::warning!(
                    "check_visibility: Found outer tuple in join for rti={:?}, ctid={}",
                    state.custom_state().rti,
                    ctid
                );

                // Try to extract company_id information from the outer tuple if available
                let outer_desc = (*outer_tuple).tts_tupleDescriptor;
                if !outer_desc.is_null() {
                    let outer_natts = (*outer_desc).natts;
                    pgrx::warning!(
                        "check_visibility: Outer tuple has {} attributes for rti={:?}, ctid={}",
                        outer_natts,
                        state.custom_state().rti,
                        ctid
                    );

                    // Look for company_id column to examine outer join condition values
                    for i in 0..outer_natts {
                        let attr = (*outer_desc).attrs.as_ptr().add(i as usize);
                        if !attr.is_null() {
                            let attr_name = std::ffi::CStr::from_ptr((*attr).attname.data.as_ptr())
                                .to_string_lossy();

                            if attr_name.contains("company_id") {
                                pgrx::warning!(
                                    "check_visibility: Found company_id attribute at position {} for rti={:?}, ctid={}",
                                    i, state.custom_state().rti, ctid
                                );

                                // // Try to get the value of company_id from the outer tuple
                                // let is_null = pg_sys::att_isnull(outer_tuple, i + 1);
                                // if !is_null {
                                //     let company_id = pg_sys::DatumGetInt64(
                                //         pg_sys::heap_getattr(
                                //             (*outer_tuple).tts_tuple,
                                //             i + 1,
                                //             outer_desc,
                                //             &mut (*outer_tuple).tts_isnull[i as usize]
                                //         )
                                //     );

                                //     pgrx::warning!(
                                //         "check_visibility: Outer tuple's company_id={} for rti={:?}, ctid={}",
                                //         company_id, state.custom_state().rti, ctid
                                //     );

                                //     // Special logging for company_id 15
                                //     if company_id == 15 {
                                //         pgrx::warning!(
                                //             "check_visibility: CRITICAL CASE - Found company_id=15 in outer tuple for rti={:?}, ctid={}",
                                //             state.custom_state().rti, ctid
                                //         );
                                //     }
                                // }
                            }
                        }
                    }
                }
            }
        }
    }

    // Call the visibility checker with enhanced logging
    let result: Option<*mut pg_sys::TupleTableSlot> = state
        .custom_state_mut()
        .visibility_checker()
        .exec_if_visible(ctid, bslot.cast(), move |heaprel| {
            // Log if it was visible
            pgrx::warning!("check_visibility: ctid={} IS VISIBLE", ctid);

            unsafe {
                // Attempt to extract more information about what's in the tuple
                // For debugging purposes only
                let heap_tuple = (*bslot).base.tuple;
                if !heap_tuple.is_null() {
                    pgrx::warning!("check_visibility: ctid={} has heap_tuple data", ctid);

                    // // Try to extract company_id from the tuple to check if it's company 15
                    // let tupdesc = (*bslot).base.tts_tupleDescriptor;
                    // if !tupdesc.is_null() {
                    //     let natts = (*tupdesc).natts;

                    //     // Look for company_id in the attributes
                    //     for i in 0..natts {
                    //         let attr = (*tupdesc).attrs.as_ptr().add(i as usize);
                    //         if !attr.is_null() {
                    //             let attr_name = std::ffi::CStr::from_ptr((*attr).attname.data.as_ptr())
                    //                 .to_string_lossy();

                    //             if attr_name == "id" {
                    //                 let is_null = pg_sys::att_isnull(bslot.cast(), i + 1);
                    //                 if !is_null {
                    //                     let id = pg_sys::DatumGetInt64(
                    //                         pg_sys::heap_getattr(
                    //                             (*bslot).base.tuple,
                    //                             i + 1,
                    //                             tupdesc,
                    //                             &mut (*bslot).base.tts_isnull[i as usize]
                    //                         )
                    //                     );

                    //                     pgrx::warning!(
                    //                         "check_visibility: Tuple has id={} for rti={:?}, ctid={}",
                    //                         id, state.custom_state().rti, ctid
                    //                     );

                    //                     // Special logging for company_id 15
                    //                     if id == 15 {
                    //                         pgrx::warning!(
                    //                             "check_visibility: FOUND COMPANY 15! ctid={} for rti={:?}",
                    //                             ctid, state.custom_state().rti
                    //                         );
                    //                     }
                    //                 }
                    //             }
                    //         }
                    //     }
                    // }
                } else {
                    pgrx::warning!("check_visibility: ctid={} has NULL heap_tuple", ctid);
                }
            }

            bslot.cast()
        });

    // Detailed logging about the visibility check result
    let current_memory_context = unsafe { pg_sys::CurrentMemoryContext };

    if result.is_some() {
        pgrx::warning!(
            "check_visibility: ctid={} visibility check PASSED in context={:?} for rti={:?}",
            ctid,
            current_memory_context,
            state.custom_state().rti
        );

        // When a tuple passes visibility, try to log its attributes
        unsafe {
            if let Some(slot) = result {
                let tupdesc = (*slot).tts_tupleDescriptor;
                if !tupdesc.is_null() {
                    pgrx::warning!(
                        "check_visibility: Examining visible tuple attributes for ctid={} with rti={:?}",
                        ctid, state.custom_state().rti
                    );

                    let natts = (*tupdesc).natts;
                    let mut values: Vec<String> = Vec::new();

                    for i in 0..natts {
                        let attr = (*tupdesc).attrs.as_ptr().add(i as usize);
                        if !attr.is_null() {
                            let attr_name = std::ffi::CStr::from_ptr((*attr).attname.data.as_ptr())
                                .to_string_lossy();

                            // let is_null = pg_sys::att_isnull(slot, i + 1);
                            // if !is_null {
                            //     values.push(format!("{}=<non-null>", attr_name));
                            // } else {
                            //     values.push(format!("{}=NULL", attr_name));
                            // }
                        }
                    }

                    pgrx::warning!(
                        "check_visibility: Visible tuple has attributes: [{}] for rti={:?}, ctid={}",
                        values.join(", "), state.custom_state().rti, ctid
                    );
                }
            }
        }
    } else {
        pgrx::warning!(
            "check_visibility: ctid={} visibility check FAILED in context={:?} for rti={:?}",
            ctid,
            current_memory_context,
            state.custom_state().rti
        );

        // // Try to get VisibilityChecker details to understand why it failed
        // if let Some(visibility_checker) = &state.custom_state().visibility_checker {
        //     let mvcc_snapshot_id = format!("{:?}", visibility_checker);
        //     pgrx::warning!(
        //         "check_visibility: Visibility check failed with visibility_checker={} for rti={:?}, ctid={}",
        //         mvcc_snapshot_id, state.custom_state().rti, ctid
        //     );
        // }
    }

    result
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
        state.custom_state().rti,
        state.custom_state().score_funcoid,
        state.custom_state().snippet_funcoid,
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
unsafe fn bms_iter(bms: *mut pg_sys::Bitmapset) -> impl Iterator<Item = i32> {
    let mut set_bit: i32 = -1;
    std::iter::from_fn(move || {
        set_bit = pg_sys::bms_next_member(bms, set_bit);
        if set_bit < 0 {
            None
        } else {
            Some(set_bit)
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
        if baserel_idx <= 0 || baserel_idx as usize >= (*root).simple_rel_array_size as usize {
            continue;
        }

        // Get the RTE to check if this is a partitioned table
        let rte = pg_sys::rt_fetch(baserel_idx as pg_sys::Index, rtable);
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

/// Recursively examines join conditions to find tables with BM25 indexes
/// Returns the first table with a BM25 index found in the join condition
unsafe fn find_bm25_index_in_join_condition(
    restrict_info: *mut pg_sys::Node,
    root: *mut pg_sys::PlannerInfo,
) -> Option<(PgRelation, PgRelation)> {
    // Safety check for null pointers
    if restrict_info.is_null() || root.is_null() {
        pgrx::warning!("find_bm25_index_in_join_condition: Null pointers detected");
        return None;
    }

    pgrx::warning!(
        "find_bm25_index_in_join_condition: Examining node type {:?}",
        (*restrict_info).type_
    );

    // First, check for CTEs in this query
    if !(*root).parse.is_null() && !(*(*root).parse).cteList.is_null() {
        pgrx::warning!(
            "find_bm25_index_in_join_condition: This query contains CTEs, performing deep trace"
        );
        trace_cte_structure(root);
    }

    // Add detailed tracing for this restrict_info node
    trace_quals_for_bm25(restrict_info);

    // Check if this is an RTE_SUBQUERY node that might contain a CTE
    if let Some(result) = check_for_cte_subquery(restrict_info, root) {
        return Some(result);
    }

    match (*restrict_info).type_ {
        pg_sys::NodeTag::T_List => {
            let restrict_infos = PgList::<pg_sys::Node>::from_pg(restrict_info.cast());

            // Try each restrict info node
            for node in restrict_infos.iter_ptr() {
                if !node.is_null() {
                    if let Some(result) = find_bm25_index_in_join_condition(node, root) {
                        return Some(result);
                    }
                }
            }

            // If we get here, none of the nodes had a BM25 index
            None
        }

        pg_sys::NodeTag::T_RestrictInfo => {
            let ri = nodecast!(RestrictInfo, T_RestrictInfo, restrict_info)?;
            let clause = if !(*ri).orclause.is_null() {
                (*ri).orclause
            } else {
                (*ri).clause
            };

            find_bm25_index_in_join_condition(clause.cast(), root)
        }

        pg_sys::NodeTag::T_OpExpr => {
            let opexpr = nodecast!(OpExpr, T_OpExpr, restrict_info)?;
            let op_oid = (*opexpr).opno;

            // Check if this is our BM25 operator
            if op_oid == anyelement_query_input_opoid() || op_oid == anyelement_text_opoid() {
                let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
                if let Some(larg) = args.get_ptr(0) {
                    // Find the left argument which should be a column reference (Var)
                    let mut arg = larg;

                    // Unwrap any RelabelType nodes
                    while (*arg).type_ == pg_sys::NodeTag::T_RelabelType {
                        let relabel_type = arg as *mut pg_sys::RelabelType;
                        arg = (*relabel_type).arg.cast();
                    }

                    if let Some(var) = nodecast!(Var, T_Var, arg) {
                        // Get information about the referenced table
                        let (heaprelid, attno, attname) = find_var_relation(var, root);
                        pgrx::warning!(
                            "Found BM25 operator referencing var: rel={:?} att={} name={} varlevelsup={}",
                            heaprelid,
                            attno,
                            attname.is_some(),
                            (*var).varlevelsup
                        );

                        // Special handling for variables that reference outer queries or CTEs
                        if (*var).varlevelsup > 0 {
                            pgrx::warning!(
                                "find_bm25_index_in_join_condition: IMPORTANT - BM25 operator references outer query or CTE (varlevelsup={})",
                                (*var).varlevelsup
                            );
                        }

                        if heaprelid != pg_sys::Oid::INVALID {
                            // We found a table reference, check if it has a BM25 index
                            if let Some((table, bm25_index)) = rel_get_bm25_index(heaprelid) {
                                // Make sure both relations are valid
                                if !table.is_null() && !bm25_index.is_null() {
                                    pgrx::warning!(
                                        "Found BM25 index in join: table={} index={}",
                                        table.name(),
                                        bm25_index.name()
                                    );
                                    return Some((table, bm25_index));
                                }
                            }
                        }
                    }
                }
            }

            None
        }

        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = nodecast!(BoolExpr, T_BoolExpr, restrict_info)?;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);

            // Check all args (AND/OR conditions)
            for arg in args.iter_ptr() {
                if let Some(result) = find_bm25_index_in_join_condition(arg, root) {
                    return Some(result);
                }
            }

            None
        }

        // Other node types we don't handle
        _ => None,
    }
}

/// Check if this query is a CTE (Common Table Expression)
unsafe fn is_cte_query(root: *mut pg_sys::PlannerInfo) -> bool {
    if root.is_null() {
        return false;
    }

    // Check if there are CTEs defined
    let parse = (*root).parse;
    if !parse.is_null() && !(*parse).cteList.is_null() {
        let cte_list = PgList::<pg_sys::CommonTableExpr>::from_pg((*parse).cteList);
        if !cte_list.is_empty() {
            pgrx::warning!("is_cte_query: Found {} CTEs in query", cte_list.len());
            for (i, cte) in cte_list.iter_ptr().enumerate() {
                let cte_name = if !(*cte).ctename.is_null() {
                    std::ffi::CStr::from_ptr((*cte).ctename).to_string_lossy()
                } else {
                    "unnamed".into()
                };

                pgrx::warning!("is_cte_query: CTE #{} name={}", i, cte_name);

                if !(*cte).ctequery.is_null() {
                    let query_type = (*(*cte).ctequery).type_;
                    pgrx::warning!("is_cte_query: CTE #{} query type={:?}", i, query_type);

                    // More detailed analysis of CTE query structure
                    pgrx::warning!("is_cte_query: Analyzing CTE #{} query structure:", i);
                    // analyze_query_restrictions((*cte).ctequery.cast());

                    // // Log target list
                    // if let Some(target_list) =
                    //     PgList::<pg_sys::Node>::from_pg((*(*cte).ctequery).targetList).get_ptr(0)
                    // {
                    //     pgrx::warning!(
                    //         "is_cte_query: CTE #{} has target list with node type {:?}",
                    //         i,
                    //         (*target_list).type_
                    //     );
                    // }

                    // // Log join trees
                    // if !(*(*cte).ctequery).jointree.is_null() {
                    //     let fromlist = (*(*(*cte).ctequery).jointree).fromlist;
                    //     if !fromlist.is_null() {
                    //         let fromlist_items = PgList::<pg_sys::Node>::from_pg(fromlist);
                    //         pgrx::warning!(
                    //             "is_cte_query: CTE #{} has {} fromlist items",
                    //             i,
                    //             fromlist_items.len()
                    //         );

                    //         for (j, item) in fromlist_items.iter_ptr().enumerate() {
                    //             pgrx::warning!(
                    //                 "is_cte_query: CTE #{} fromlist item #{} has node type {:?}",
                    //                 i,
                    //                 j,
                    //                 (*item).type_
                    //             );
                    //         }
                    //     }
                    // }
                }
            }

            // Analyze the main query too
            pgrx::warning!("is_cte_query: Analyzing main query structure:");
            // analyze_query_restrictions(root);

            return true;
        }
    }
    false
}

/// Check for a BM25 index in a subquery RTE that might be part of a CTE
unsafe fn check_for_cte_subquery(
    node: *mut pg_sys::Node,
    root: *mut pg_sys::PlannerInfo,
) -> Option<(PgRelation, PgRelation)> {
    if node.is_null() {
        return None;
    }

    pgrx::warning!(
        "check_for_cte_subquery: Checking node type={:?}",
        (*node).type_
    );

    if (*node).type_ == pg_sys::NodeTag::T_RangeTblRef {
        let rtr = node as *mut pg_sys::RangeTblRef;
        let rtindex = (*rtr).rtindex;

        pgrx::warning!(
            "check_for_cte_subquery: Found RangeTblRef with rtindex={}",
            rtindex
        );

        // Get the range table entry
        let rte = pg_sys::rt_fetch(rtindex.try_into().unwrap(), (*(*root).parse).rtable);

        if !rte.is_null() {
            pgrx::warning!("check_for_cte_subquery: RTE kind={}", (*rte).rtekind);

            if (*rte).rtekind == pg_sys::RTEKind::RTE_SUBQUERY {
                pgrx::warning!("check_for_cte_subquery: Found subquery RTE");

                // Check if this is a subquery from a CTE
                if !(*rte).ctename.is_null() {
                    let cte_name = std::ffi::CStr::from_ptr((*rte).ctename).to_string_lossy();
                    pgrx::warning!(
                        "check_for_cte_subquery: Subquery is from CTE '{}'",
                        cte_name
                    );
                }

                // Examine the subquery for BM25 operators
                let subquery = (*rte).subquery;
                if !subquery.is_null() {
                    pgrx::warning!("check_for_cte_subquery: Examining subquery");

                    // Check WHERE clause for BM25 operators
                    if !(*subquery).jointree.is_null() && !(*(*subquery).jointree).quals.is_null() {
                        pgrx::warning!("check_for_cte_subquery: Examining subquery WHERE clause");

                        let quals = (*(*subquery).jointree).quals;
                        if !quals.is_null() {
                            pgrx::warning!(
                                "check_for_cte_subquery: WHERE clause has node type {:?}",
                                (*quals).type_
                            );

                            // See if there's a BM25 operator in the quals
                            if let Some(result) = find_bm25_index_in_quals(quals, root) {
                                pgrx::warning!("check_for_cte_subquery: Found BM25 index in subquery WHERE clause");
                                return Some(result);
                            }
                        }
                    }

                    // Check target list for BM25 operators (for cases like SELECT score(...))
                    let target_list = (*subquery).targetList;
                    if !target_list.is_null() {
                        pgrx::warning!("check_for_cte_subquery: Examining subquery target list");

                        let target_entries = PgList::<pg_sys::TargetEntry>::from_pg(target_list);
                        for (i, te) in target_entries.iter_ptr().enumerate() {
                            let expr = (*te).expr;
                            if !expr.is_null() {
                                pgrx::warning!(
                                    "check_for_cte_subquery: Target entry #{} has expr type {:?}",
                                    i,
                                    (*expr).type_
                                );

                                // Check if this is the score function
                                if (*expr).type_ == pg_sys::NodeTag::T_FuncExpr {
                                    let funcexpr = expr as *mut pg_sys::FuncExpr;
                                    if (*funcexpr).funcid == score_funcoid() {
                                        pgrx::warning!("check_for_cte_subquery: Found score function in target list");

                                        // Get the table from the score function arg
                                        let args =
                                            PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
                                        if let Some(arg) = args.get_ptr(0) {
                                            if let Some(var) = nodecast!(Var, T_Var, arg) {
                                                let (heaprelid, attno, attname) =
                                                    find_var_relation(var, root);
                                                pgrx::warning!("check_for_cte_subquery: Score function references rel={:?}, att={}", heaprelid, attno);

                                                if heaprelid != pg_sys::Oid::INVALID {
                                                    if let Some((table, bm25_index)) =
                                                        rel_get_bm25_index(heaprelid)
                                                    {
                                                        if !table.is_null() && !bm25_index.is_null()
                                                        {
                                                            pgrx::warning!("check_for_cte_subquery: Found BM25 index in score function: table={}, index={}", 
                                                                     table.name(), bm25_index.name());
                                                            return Some((table, bm25_index));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Find a BM25 operator in query quals (WHERE clause expressions)
unsafe fn find_bm25_index_in_quals(
    quals: *mut pg_sys::Node,
    root: *mut pg_sys::PlannerInfo,
) -> Option<(PgRelation, PgRelation)> {
    if quals.is_null() {
        return None;
    }

    pgrx::warning!(
        "find_bm25_index_in_quals: Examining quals of type {:?}",
        (*quals).type_
    );

    match (*quals).type_ {
        pg_sys::NodeTag::T_OpExpr => {
            let opexpr = quals as *mut pg_sys::OpExpr;
            let op_oid = (*opexpr).opno;

            pgrx::warning!(
                "find_bm25_index_in_quals: Checking if op_oid={:?} is the BM25 operator",
                op_oid
            );

            // Check if this is our BM25 operator
            if op_oid == anyelement_query_input_opoid() || op_oid == anyelement_text_opoid() {
                pgrx::warning!("find_bm25_index_in_quals: Found BM25 operator");

                let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
                if let Some(larg) = args.get_ptr(0) {
                    // Find the left argument which should be a column reference (Var)
                    let mut arg = larg;

                    // Unwrap any RelabelType nodes
                    while (*arg).type_ == pg_sys::NodeTag::T_RelabelType {
                        let relabel_type = arg as *mut pg_sys::RelabelType;
                        arg = (*relabel_type).arg.cast();
                    }

                    if let Some(var) = nodecast!(Var, T_Var, arg) {
                        // Get information about the referenced table
                        let (heaprelid, attno, attname) = find_var_relation(var, root);

                        pgrx::warning!(
                            "find_bm25_index_in_quals: BM25 operator references rel={:?}, att={}, varno={}, varlevelsup={}",
                            heaprelid,
                            attno,
                            (*var).varno,
                            (*var).varlevelsup
                        );

                        if heaprelid != pg_sys::Oid::INVALID {
                            // We found a table reference, check if it has a BM25 index
                            if let Some((table, bm25_index)) = rel_get_bm25_index(heaprelid) {
                                if !table.is_null() && !bm25_index.is_null() {
                                    pgrx::warning!("find_bm25_index_in_quals: Found BM25 index: table={}, index={}", 
                                             table.name(), bm25_index.name());
                                    return Some((table, bm25_index));
                                }
                            }
                        }
                    }
                }
            }
        }
        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = quals as *mut pg_sys::BoolExpr;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);

            pgrx::warning!(
                "find_bm25_index_in_quals: Examining boolean expression with {} args and boolop={}",
                args.len(),
                (*boolexpr).boolop
            );

            // Check all args (AND/OR conditions)
            for (i, arg) in args.iter_ptr().enumerate() {
                pgrx::warning!(
                    "find_bm25_index_in_quals: Examining boolean arg #{} of type {:?}",
                    i,
                    (*arg).type_
                );
                if let Some(result) = find_bm25_index_in_quals(arg, root) {
                    return Some(result);
                }
            }
        }
        // Add more cases as needed
        _ => {
            pgrx::warning!(
                "find_bm25_index_in_quals: Unhandled qual type {:?}",
                (*quals).type_
            );
        }
    }

    None
}

// Add this new diagnostic function to analyze query structure and find BM25 operators
unsafe fn analyze_query_restrictions(root: *mut pg_sys::PlannerInfo) {
    if root.is_null() {
        pgrx::warning!("analyze_query_restrictions: Root is null");
        return;
    }

    // Log basic query structure
    pgrx::warning!("analyze_query_restrictions: Analyzing query structure");

    // Check if this query has a jointree (this is where restrictions are)
    if !(*root).parse.is_null() && !(*(*root).parse).jointree.is_null() {
        let jointree = (*(*root).parse).jointree;
        pgrx::warning!("analyze_query_restrictions: Jointree found");

        // Check what's in the jointree quals
        if !(*jointree).quals.is_null() {
            pgrx::warning!("analyze_query_restrictions: Jointree has quals");
            log_node_type((*jointree).quals);

            // Search for BM25 operators in the quals
            let has_bm25 = search_for_bm25_operator((*jointree).quals);
            pgrx::warning!(
                "analyze_query_restrictions: Jointree quals contain BM25 operators: {}",
                has_bm25
            );

            // If BM25 operators are found, do a detailed trace
            if has_bm25 {
                pgrx::warning!(
                    "analyze_query_restrictions: Performing detailed BM25 operator trace"
                );
                trace_quals_for_bm25((*jointree).quals);
            }
        } else {
            pgrx::warning!("analyze_query_restrictions: Jointree has no quals");
        }
    }

    // Check for CTEs with enhanced tracing
    if !(*root).parse.is_null() && !(*(*root).parse).cteList.is_null() {
        pgrx::warning!("analyze_query_restrictions: Performing detailed CTE structure analysis");
        trace_cte_structure(root);
    } else {
        let cte_list = (*(*root).parse).cteList;
        let cte_len = PgList::<pg_sys::Node>::from_pg(cte_list).len();
        pgrx::warning!("analyze_query_restrictions: Found {} CTEs", cte_len);

        // Examine each CTE
        let cte_items = PgList::<pg_sys::CommonTableExpr>::from_pg(cte_list);
        for (i, cte) in cte_items.iter_ptr().enumerate() {
            pgrx::warning!("analyze_query_restrictions: Examining CTE #{}", i);

            if !(*cte).ctequery.is_null() {
                pgrx::warning!("analyze_query_restrictions: CTE has query");
                log_node_type((*cte).ctequery);

                // Check if this CTE contains BM25 operators
                let has_bm25 = search_for_bm25_operator((*cte).ctequery);
                pgrx::warning!(
                    "analyze_query_restrictions: CTE query contains BM25 operators: {}",
                    has_bm25
                );

                // If BM25 operators are found, do a detailed trace
                if has_bm25 {
                    pgrx::warning!("analyze_query_restrictions: Performing detailed BM25 operator trace for CTE #{}", i);
                    trace_quals_for_bm25((*cte).ctequery);
                }
            }
        }
    }

    // Enhanced check for relations with BM25 indexes in the query
    if !(*root).simple_rte_array.is_null() {
        let rte_count = (*root).simple_rel_array_size as usize;
        pgrx::warning!("analyze_query_restrictions: Query has {} RTEs", rte_count);

        // Track all relations with BM25 indexes
        for rti in 1..rte_count {
            let simple_rel = *((*root).simple_rel_array.add(rti));

            if !simple_rel.is_null() {
                let relid = (*simple_rel).relid;
                // Skip invalid OIDs and likely system relations (OIDs < 1000)
                if relid != pg_sys::Oid::INVALID.to_u32() && relid >= 1000 {
                    pgrx::warning!(
                        "analyze_query_restrictions: RTE {} has relid: {:?}",
                        rti,
                        relid
                    );

                    // Check if this relation has a BM25 index
                    if let Some((table, index)) = rel_get_bm25_index(relid.into()) {
                        pgrx::warning!(
                            "analyze_query_restrictions: RTE {} has BM25 index: {} (table: {})",
                            rti,
                            index.name(),
                            table.name()
                        );
                    }

                    // Check if this RTE has restrictions
                    if !(*simple_rel).baserestrictinfo.is_null() {
                        let restrict_len =
                            PgList::<pg_sys::Node>::from_pg((*simple_rel).baserestrictinfo).len();
                        pgrx::warning!(
                            "analyze_query_restrictions: RTE {} has {} restriction clauses",
                            rti,
                            restrict_len
                        );

                        // Look for BM25 operators in restrictions
                        let rinfo_list =
                            PgList::<pg_sys::RestrictInfo>::from_pg((*simple_rel).baserestrictinfo);
                        for (j, rinfo) in rinfo_list.iter_ptr().enumerate() {
                            if !(*rinfo).clause.is_null() {
                                let has_bm25 = search_for_bm25_operator((*rinfo).clause.cast());
                                if has_bm25 {
                                    pgrx::warning!("analyze_query_restrictions: Found BM25 operator in RTE {} restriction #{}", 
                                        rti, j);

                                    // Do detailed tracing
                                    trace_quals_for_bm25((*rinfo).clause.cast());
                                }
                            }
                        }
                    } else {
                        pgrx::warning!(
                            "analyze_query_restrictions: RTE {} has no restrictions",
                            rti
                        );
                    }
                }
            }
        }
    }
}

// Helper function to log node type
unsafe fn log_node_type(node: *mut pg_sys::Node) {
    if node.is_null() {
        pgrx::warning!("log_node_type: Node is null");
        return;
    }

    pgrx::warning!("log_node_type: Node type={:?}", (*node).type_);
}

// Helper function to search for BM25 operator in a tree
unsafe fn search_for_bm25_operator(node: *mut pg_sys::Node) -> bool {
    if node.is_null() {
        return false;
    }

    match (*node).type_ {
        pg_sys::NodeTag::T_OpExpr => {
            let opexpr = node.cast::<pg_sys::OpExpr>();
            let opno = (*opexpr).opno;

            // Check if this is our BM25 operator
            let our_opno = anyelement_query_input_opoid();
            let our_opno_text = anyelement_text_opoid();

            // Enhanced logging for operator details
            pgrx::warning!(
                "search_for_bm25_operator: Examining OpExpr with opno={} (BM25 opno={}, BM25 text opno={}), is_match={}",
                opno.to_u32(),
                our_opno.to_u32(),
                our_opno_text.to_u32(),
                opno == our_opno || opno == our_opno_text
            );

            // More detailed OID comparison for debugging
            if opno != our_opno && opno != our_opno_text {
                pgrx::warning!(
                    "search_for_bm25_operator: OID comparison failed: opno({:?}) != our_opno({:?}, {:?}), to_u32 equality={}, equality={}",
                    opno,
                    our_opno,
                    our_opno_text,
                    opno.to_u32() == our_opno.to_u32(),
                    opno.to_u32() == our_opno_text.to_u32()
                );
            }

            // Try to get operator name
            if let Ok(op_name) = std::ffi::CStr::from_ptr(pg_sys::get_opname(opno)).to_str() {
                pgrx::warning!(
                    "search_for_bm25_operator: OpExpr operator name: {}",
                    op_name
                );

                // Special check for operators with @@ or @@@ to catch potential mismatches
                if op_name.contains("@") {
                    pgrx::warning!(
                        "search_for_bm25_operator: Found operator with @ symbol: '{}' (opno={:?}, BM25 opno={:?}, BM25 text opno={:?})",
                        op_name,
                        opno,
                        our_opno,
                        our_opno_text
                    );
                }
            }

            // Log details about arguments
            if !(*opexpr).args.is_null() {
                let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
                pgrx::warning!("search_for_bm25_operator: OpExpr has {} args", args.len());

                // Log each argument's type
                for (i, arg) in args.iter_ptr().enumerate() {
                    pgrx::warning!(
                        "search_for_bm25_operator: OpExpr arg #{} type={:?}",
                        i,
                        (*arg).type_
                    );

                    // Special case for Var nodes
                    if (*arg).type_ == pg_sys::NodeTag::T_Var {
                        let var = arg.cast::<pg_sys::Var>();
                        pgrx::warning!(
                            "search_for_bm25_operator: OpExpr arg #{} is Var with varno={}, varattno={}, varlevelsup={}",
                            i, (*var).varno, (*var).varattno, (*var).varlevelsup
                        );
                    }

                    // Special case for Const nodes
                    if (*arg).type_ == pg_sys::NodeTag::T_Const {
                        let const_node = arg.cast::<pg_sys::Const>();
                        pgrx::warning!(
                            "search_for_bm25_operator: OpExpr arg #{} is Const with consttype={:?}",
                            i,
                            (*const_node).consttype
                        );
                    }
                }
            }

            if opno == our_opno || opno == our_opno_text {
                pgrx::warning!(
                    "search_for_bm25_operator: Found BM25 operator! opno={}",
                    opno.to_u32()
                );

                // Log more info about operands
                if !(*opexpr).args.is_null() {
                    let args_len = PgList::<pg_sys::Node>::from_pg((*opexpr).args).len();
                    pgrx::warning!(
                        "search_for_bm25_operator: BM25 operator has {} args",
                        args_len
                    );

                    let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
                    for (i, arg) in args.iter_ptr().enumerate() {
                        pgrx::warning!(
                            "search_for_bm25_operator: Arg #{} type={:?}",
                            i,
                            (*arg).type_
                        );

                        // If it's a Var, log more details
                        if (*arg).type_ == pg_sys::NodeTag::T_Var {
                            let var = arg.cast::<pg_sys::Var>();
                            pgrx::warning!("search_for_bm25_operator: Var details - varno={}, varattno={}, varlevelsup={}", 
                                (*var).varno, (*var).varattno, (*var).varlevelsup);

                            // Enhanced CTE debugging: Log more details for variables that reference outer queries
                            if (*var).varlevelsup > 0 {
                                pgrx::warning!("search_for_bm25_operator: IMPORTANT - Var references an outer query or CTE (varlevelsup={})", 
                                    (*var).varlevelsup);

                                // Try to get more information about what relation this references
                                let relid =
                                    get_varno_relid(var, (*var).varlevelsup.try_into().unwrap());
                                if relid != pg_sys::Oid::INVALID {
                                    let rel = PgRelation::open(relid);
                                    pgrx::warning!("search_for_bm25_operator: Var likely references relation: {} (oid={:?})",
                                        rel.name(), relid);

                                    // Check if this relation has a BM25 index
                                    if let Some((table, index)) = rel_get_bm25_index(relid) {
                                        pgrx::warning!("search_for_bm25_operator: Referenced relation has BM25 index: {}", 
                                            index.name());
                                    }
                                }
                            }
                        }
                    }
                }

                return true;
            }

            // Recursively check arguments
            if !(*opexpr).args.is_null() {
                let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
                for arg in args.iter_ptr() {
                    if search_for_bm25_operator(arg) {
                        return true;
                    }
                }
            }
        }

        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = node.cast::<pg_sys::BoolExpr>();

            // Check arguments
            if !(*boolexpr).args.is_null() {
                let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
                for arg in args.iter_ptr() {
                    if search_for_bm25_operator(arg) {
                        return true;
                    }
                }
            }
        }

        pg_sys::NodeTag::T_Query => {
            let query = node.cast::<pg_sys::Query>();

            // Check jointree quals
            if !(*query).jointree.is_null()
                && !(*(*query).jointree).quals.is_null()
                && search_for_bm25_operator((*(*query).jointree).quals)
            {
                return true;
            }

            // Check targetList
            if !(*query).targetList.is_null() {
                let tlist = PgList::<pg_sys::TargetEntry>::from_pg((*query).targetList);
                for te in tlist.iter_ptr() {
                    if !(*te).expr.is_null() && search_for_bm25_operator((*te).expr.cast()) {
                        return true;
                    }
                }
            }

            // Check CTEs within this query
            if !(*query).cteList.is_null() {
                let cte_list = PgList::<pg_sys::CommonTableExpr>::from_pg((*query).cteList);
                for (i, cte) in cte_list.iter_ptr().enumerate() {
                    if !(*cte).ctequery.is_null() {
                        pgrx::warning!("search_for_bm25_operator: Checking CTE #{} query", i);
                        if search_for_bm25_operator((*cte).ctequery) {
                            return true;
                        }
                    }
                }
            }

            // Check RTEs for subqueries
            if !(*query).rtable.is_null() {
                let rtable = PgList::<pg_sys::RangeTblEntry>::from_pg((*query).rtable);
                for (i, rte) in rtable.iter_ptr().enumerate() {
                    if (*rte).rtekind == pg_sys::RTEKind::RTE_SUBQUERY && !(*rte).subquery.is_null()
                    {
                        pgrx::warning!("search_for_bm25_operator: Checking RTE #{} subquery", i);
                        if search_for_bm25_operator((*rte).subquery.cast()) {
                            return true;
                        }
                    }
                }
            }
        }

        // Add special handling for SubLink nodes (subqueries)
        pg_sys::NodeTag::T_SubLink => {
            let sublink = node.cast::<pg_sys::SubLink>();
            if !(*sublink).subselect.is_null() {
                pgrx::warning!(
                    "search_for_bm25_operator: Examining SubLink of type {}",
                    (*sublink).subLinkType
                );
                if search_for_bm25_operator((*sublink).subselect) {
                    return true;
                }
            }
        }

        // Add other node types as needed
        _ => {}
    }

    false
}

// Helper function to try to determine the relation OID for a Var with varlevelsup > 0
unsafe fn get_varno_relid(var: *mut pg_sys::Var, levels_up: u16) -> pg_sys::Oid {
    if levels_up == 0 || var.is_null() {
        return pg_sys::Oid::INVALID;
    }

    // This is a complex case that would require access to the parent query's range table
    // We can't easily access this information here, but log what we know
    pgrx::warning!(
        "get_varno_relid: Attempting to resolve Var with varno={}, varattno={}, varlevelsup={}",
        (*var).varno,
        (*var).varattno,
        levels_up
    );

    // In a real implementation, we would need to traverse up the query context
    // For now, this is mostly a placeholder for diagnostic purposes
    pg_sys::Oid::INVALID
}

// New function to deeply trace CTE queries for BM25 operators
unsafe fn trace_cte_structure(root: *mut pg_sys::PlannerInfo) {
    if root.is_null() || (*root).parse.is_null() {
        pgrx::warning!("trace_cte_structure: Root or parse tree is null");
        return;
    }

    // Check for CTEs in the query
    let cte_list = (*(*root).parse).cteList;
    if cte_list.is_null() {
        pgrx::warning!("trace_cte_structure: Query has no CTEs");
        return;
    }

    let ctes = PgList::<pg_sys::CommonTableExpr>::from_pg(cte_list);
    pgrx::warning!("trace_cte_structure: Found {} CTEs in query", ctes.len());

    // Examine each CTE
    for (i, cte) in ctes.iter_ptr().enumerate() {
        let cte_name = if !(*cte).ctename.is_null() {
            std::ffi::CStr::from_ptr((*cte).ctename).to_string_lossy()
        } else {
            "unnamed".into()
        };

        pgrx::warning!(
            "trace_cte_structure: Examining CTE #{} name={}",
            i,
            cte_name
        );

        if (*cte).ctequery.is_null() {
            pgrx::warning!("trace_cte_structure: CTE #{} has null query", i);
            continue;
        }

        // Check the query type
        let query_type = (*(*cte).ctequery).type_;
        pgrx::warning!(
            "trace_cte_structure: CTE #{} query type={:?}",
            i,
            query_type
        );

        // For Query nodes, examine in detail
        if query_type == pg_sys::NodeTag::T_Query {
            let query = (*cte).ctequery.cast::<pg_sys::Query>();

            // Check command type
            pgrx::warning!(
                "trace_cte_structure: CTE #{} command type={}",
                i,
                (*query).commandType
            );

            // Check for WHERE clause
            if !(*query).jointree.is_null() {
                if !(*(*query).jointree).quals.is_null() {
                    pgrx::warning!("trace_cte_structure: CTE #{} has WHERE clause", i);

                    // Log quals type
                    pgrx::warning!(
                        "trace_cte_structure: CTE #{} WHERE clause type={:?}",
                        i,
                        (*(*(*query).jointree).quals).type_
                    );

                    // Enhanced logging for WHERE clause operators
                    pgrx::warning!(
                        "trace_cte_structure: Detailed analysis of WHERE clause in CTE #{}",
                        i
                    );
                    analyze_where_clause_operators((*(*query).jointree).quals, i);

                    // Deep search for BM25 operators
                    let has_bm25 = search_for_bm25_operator((*(*query).jointree).quals);
                    pgrx::warning!(
                        "trace_cte_structure: CTE #{} WHERE clause has BM25 operator: {}",
                        i,
                        has_bm25
                    );

                    if has_bm25 {
                        trace_quals_for_bm25((*(*query).jointree).quals);
                    } else {
                        pgrx::warning!("trace_cte_structure: CTE #{} has no WHERE clause", i);
                    }
                }

                // Check from list
                if !(*(*query).jointree).fromlist.is_null() {
                    let fromlist = PgList::<pg_sys::Node>::from_pg((*(*query).jointree).fromlist);
                    pgrx::warning!(
                        "trace_cte_structure: CTE #{} has {} fromlist items",
                        i,
                        fromlist.len()
                    );

                    // Check each from item
                    for (j, item) in fromlist.iter_ptr().enumerate() {
                        pgrx::warning!(
                            "trace_cte_structure: CTE #{} fromlist item #{} type={:?}",
                            i,
                            j,
                            (*item).type_
                        );

                        // For RangeTblRef, get more info
                        if (*item).type_ == pg_sys::NodeTag::T_RangeTblRef {
                            let rtr = item.cast::<pg_sys::RangeTblRef>();
                            pgrx::warning!(
                                "trace_cte_structure: CTE #{} fromlist item #{} is RangeTblRef with rtindex={}",
                                i, j, (*rtr).rtindex
                            );

                            // Get the RTE
                            let rte = pg_sys::rt_fetch(
                                (*rtr).rtindex.try_into().unwrap(),
                                (*query).rtable,
                            );
                            if !rte.is_null() {
                                pgrx::warning!(
                                    "trace_cte_structure: CTE #{} fromlist item #{} RTE kind={}",
                                    i,
                                    j,
                                    (*rte).rtekind
                                );

                                // If it's a relation, examine it further
                                if (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION {
                                    pgrx::warning!(
                                        "trace_cte_structure: CTE #{} fromlist item #{} references relation with oid={:?}",
                                        i, j, (*rte).relid
                                    );

                                    // Check if it has a BM25 index
                                    if let Some((table, index)) = rel_get_bm25_index((*rte).relid) {
                                        pgrx::warning!(
                                            "trace_cte_structure: CTE #{} fromlist item #{} relation has BM25 index: {}",
                                            i, j, index.name()
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Check target list for score function or other BM25 references
            if !(*query).targetList.is_null() {
                let targetlist = PgList::<pg_sys::TargetEntry>::from_pg((*query).targetList);
                pgrx::warning!(
                    "trace_cte_structure: CTE #{} has {} target entries",
                    i,
                    targetlist.len()
                );

                // Check each target entry
                for (j, te) in targetlist.iter_ptr().enumerate() {
                    if !(*te).expr.is_null() {
                        pgrx::warning!(
                            "trace_cte_structure: CTE #{} target entry #{} expr type={:?}",
                            i,
                            j,
                            (*(*te).expr).type_
                        );

                        // Check for score function
                        if (*(*te).expr).type_ == pg_sys::NodeTag::T_FuncExpr {
                            let funcexpr = (*te).expr.cast::<pg_sys::FuncExpr>();
                            if (*funcexpr).funcid == score_funcoid() {
                                pgrx::warning!(
                                    "trace_cte_structure: CTE #{} target entry #{} uses score function",
                                    i, j
                                );
                            }
                        }
                    }
                }
            }

            // Check for BM25 operators in the entire query tree
            let has_bm25 = search_for_bm25_operator((*cte).ctequery);
            pgrx::warning!(
                "trace_cte_structure: CTE #{} contains BM25 operators anywhere: {}",
                i,
                has_bm25
            );
        }
    }

    // Also examine the main query's RTEs for references to the CTEs
    if !(*(*root).parse).rtable.is_null() {
        let rtable = PgList::<pg_sys::RangeTblEntry>::from_pg((*(*root).parse).rtable);
        pgrx::warning!("trace_cte_structure: Main query has {} RTEs", rtable.len());

        for (i, rte) in rtable.iter_ptr().enumerate() {
            // Check if this RTE references a CTE
            if !(*rte).ctename.is_null() {
                let cte_name = std::ffi::CStr::from_ptr((*rte).ctename).to_string_lossy();
                pgrx::warning!(
                    "trace_cte_structure: Main query RTE #{} references CTE '{}'",
                    i,
                    cte_name
                );
            }
        }
    }
}

// Function to trace through quals specifically looking for BM25 operators
unsafe fn trace_quals_for_bm25(quals: *mut pg_sys::Node) {
    if quals.is_null() {
        pgrx::warning!("trace_quals_for_bm25: Quals node is null");
        return;
    }

    pgrx::warning!(
        "trace_quals_for_bm25: Examining quals of type {:?}",
        (*quals).type_
    );

    match (*quals).type_ {
        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = quals.cast::<pg_sys::BoolExpr>();
            pgrx::warning!(
                "trace_quals_for_bm25: Boolean expression with op={}",
                (*boolexpr).boolop
            );

            if !(*boolexpr).args.is_null() {
                let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
                for (i, arg) in args.iter_ptr().enumerate() {
                    pgrx::warning!(
                        "trace_quals_for_bm25: Examining bool arg #{} of type {:?}",
                        i,
                        (*arg).type_
                    );
                    trace_quals_for_bm25(arg);
                }
            }
        }
        pg_sys::NodeTag::T_OpExpr => {
            let opexpr = quals.cast::<pg_sys::OpExpr>();
            let opno = (*opexpr).opno;
            let bm25_opno = anyelement_query_input_opoid();
            let bm25_text_opno = anyelement_text_opoid();

            pgrx::warning!(
                "trace_quals_for_bm25: OpExpr with opno={} (BM25 opno={}, BM25 text opno={})",
                opno.to_u32(),
                bm25_opno.to_u32(),
                bm25_text_opno.to_u32()
            );

            if opno == bm25_opno || opno == bm25_text_opno {
                pgrx::warning!("trace_quals_for_bm25: Found BM25 operator!");

                // Examine arguments in detail
                if !(*opexpr).args.is_null() {
                    let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);

                    for (i, arg) in args.iter_ptr().enumerate() {
                        pgrx::warning!(
                            "trace_quals_for_bm25: BM25 arg #{} type={:?}",
                            i,
                            (*arg).type_
                        );

                        // Special handling for Var nodes
                        if (*arg).type_ == pg_sys::NodeTag::T_Var {
                            let var = arg.cast::<pg_sys::Var>();
                            pgrx::warning!(
                                "trace_quals_for_bm25: BM25 arg #{} is Var with varno={}, varattno={}, varlevelsup={}",
                                i, (*var).varno, (*var).varattno, (*var).varlevelsup
                            );
                        }
                    }
                }
            }
        }
        _ => {
            pgrx::warning!(
                "trace_quals_for_bm25: Unhandled quals type {:?}",
                (*quals).type_
            );
        }
    }
}

// Add this new function just before trace_quals_for_bm25
// Function to analyze operators in WHERE clauses with additional context
unsafe fn analyze_where_clause_operators(quals: *mut pg_sys::Node, cte_idx: usize) {
    if quals.is_null() {
        pgrx::warning!("analyze_where_clause_operators: Quals node is null");
        return;
    }

    pgrx::warning!(
        "analyze_where_clause_operators: Examining WHERE clause of type {:?}",
        (*quals).type_
    );

    match (*quals).type_ {
        pg_sys::NodeTag::T_OpExpr => {
            let opexpr = quals.cast::<pg_sys::OpExpr>();
            let opno = (*opexpr).opno;
            let our_opno = anyelement_query_input_opoid();
            let our_opno_text = anyelement_text_opoid();

            pgrx::warning!(
                "analyze_where_clause_operators: CTE #{} WHERE clause has OpExpr with opno={:?} (BM25 opno={:?}, BM25 text opno={:?}), is_match={}",
                cte_idx,
                opno,
                our_opno,
                our_opno_text,
                opno == our_opno || opno == our_opno_text
            );

            // More detailed OID comparison for debugging
            if opno != our_opno && opno != our_opno_text {
                pgrx::warning!(
                    "analyze_where_clause_operators: OID comparison failed: opno({:?}) != our_opno({:?}, {:?}), equality={}, to_u32 equality={}",
                    opno,
                    our_opno,
                    our_opno_text,
                    opno == our_opno || opno == our_opno_text,
                    opno.to_u32() == our_opno.to_u32() || opno.to_u32() == our_opno_text.to_u32()
                );

                // Try to get operator name
                unsafe {
                    if let Ok(op_name) = std::ffi::CStr::from_ptr(pg_sys::get_opname(opno)).to_str()
                    {
                        pgrx::warning!(
                        "analyze_where_clause_operators: CTE #{} WHERE clause operator name: {}",
                        cte_idx,
                        op_name
                    );
                    }
                }

                // Special check for BM25-like operators
                if let Ok(op_name) = std::ffi::CStr::from_ptr(pg_sys::get_opname(opno)).to_str() {
                    if op_name == "@@@" {
                        pgrx::warning!(
                        "analyze_where_clause_operators: CTE #{} WHERE clause has SUSPICIOUS BM25-like operator '{}' (opno={}, BM25 opno={}, BM25 text opno={})",
                        cte_idx,
                        op_name,
                        opno.to_u32(),
                        our_opno.to_u32(),
                        our_opno_text.to_u32()
                    );
                    }
                }

                // Log details about the arguments
                if !(*opexpr).args.is_null() {
                    let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);

                    // Log each argument and its type
                    for (i, arg) in args.iter_ptr().enumerate() {
                        pgrx::warning!(
                        "analyze_where_clause_operators: CTE #{} WHERE clause OpExpr arg #{} type={:?}",
                        cte_idx,
                        i,
                        (*arg).type_
                    );

                        // Special handling for Var nodes
                        if (*arg).type_ == pg_sys::NodeTag::T_Var {
                            let var = arg.cast::<pg_sys::Var>();
                            pgrx::warning!(
                            "analyze_where_clause_operators: CTE #{} WHERE clause OpExpr arg #{} is Var with varno={}, varattno={}, varlevelsup={}",
                            cte_idx, i, (*var).varno, (*var).varattno, (*var).varlevelsup
                        );
                        }
                    }
                }
            }
        }
        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = quals.cast::<pg_sys::BoolExpr>();

            pgrx::warning!(
                "analyze_where_clause_operators: CTE #{} WHERE clause has BoolExpr with boolop={}",
                cte_idx,
                (*boolexpr).boolop
            );

            // Recursively analyze each argument
            if !(*boolexpr).args.is_null() {
                let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
                for (i, arg) in args.iter_ptr().enumerate() {
                    pgrx::warning!(
                        "analyze_where_clause_operators: CTE #{} WHERE clause BoolExpr arg #{} type={:?}",
                        cte_idx,
                        i,
                        (*arg).type_
                    );

                    // Recursively analyze the argument
                    analyze_where_clause_operators(arg, cte_idx);
                }
            }
        }
        // Add other cases as needed
        _ => {
            pgrx::warning!(
                "analyze_where_clause_operators: CTE #{} WHERE clause has unhandled node type {:?}",
                cte_idx,
                (*quals).type_
            );
        }
    }
}

// Add this function to help diagnose the issue
pub fn debug_document_id(searcher: &tantivy::Searcher, doc_address: tantivy::DocAddress) -> String {
    // Specify the concrete type parameter for doc()
    if let Ok(doc) = searcher.doc::<tantivy::schema::TantivyDocument>(doc_address) {
        let schema = searcher.schema();

        // Create a comprehensive debug representation of the document
        let mut doc_info = format!(
            "DocAddr({},{})",
            doc_address.segment_ord, doc_address.doc_id
        );
        let mut found_id = false;
        let mut is_company_15 = false;

        // First, specifically look for ID fields to report them prominently
        for (field, field_entry) in schema.fields() {
            let field_name = field_entry.name();

            // Check if this is an ID field
            if field_name == "id"
                || field_name.ends_with(".id")
                || field_name.ends_with("_id")
                || field_name.contains("id")
            {
                if let Some(field_value) = doc.get_first(field) {
                    // Add this ID field to our output with special formatting
                    let val_str = format!("{:?}", field_value);
                    doc_info.push_str(&format!(" | ID:'{}={}'", field_name, val_str));
                    found_id = true;

                    // Specifically check for company_id = 15
                    // More precise matching for "company_id" field
                    if (field_name == "company_id"
                        || field_name == "company.id"
                        || field_name == "companyid")
                        && val_str.trim_matches('"') == "15"
                    {
                        is_company_15 = true;
                        doc_info.push_str(" [COMPANY 15 FOUND!]");
                    }
                    // Also check if the field is just "id" and the parent object might be a company
                    else if field_name == "id" && val_str.trim_matches('"') == "15" {
                        // This might be company 15, mark it for further analysis
                        doc_info.push_str(" [POSSIBLE COMPANY 15]");
                    }
                } else {
                    doc_info.push_str(&format!(" | [{} NOT FOUND]", field_name));
                }
            } else {
                if let Some(field_value) = doc.get_first(field) {
                    // Add this field to our output
                    let val_str = format!("{:?}", field_value);
                    doc_info.push_str(&format!(" | {}={}", field_name, val_str));

                    // Also check for company name field
                    if (field_name == "company_name"
                        || field_name == "company.name"
                        || field_name == "name")
                        && val_str.contains("Important Testing")
                    {
                        is_company_15 = true;
                        doc_info.push_str(" [COMPANY 15 BY NAME!]");
                    }
                } else {
                    doc_info.push_str(&format!(" | [{} NOT FOUND EITHER]", field_name));
                }
            }
        }

        if schema.fields().next().is_none() {
            doc_info.push_str(" | [NO FIELDS EXISTED!]");
        } else {
            doc_info.push_str(" | [FIELDS EXISTED!]");
        }

        // Additional marker if this is company 15
        if is_company_15 {
            doc_info = format!("COMPANY_15: {}", doc_info);
        }

        // If we didn't find any ID field, add a warning
        if !found_id {
            doc_info.push_str(" | [NO ID FIELD FOUND]");
        }

        return doc_info;
    } else {
        format!("Failed to retrieve document at {:?}", doc_address)
    }
}
