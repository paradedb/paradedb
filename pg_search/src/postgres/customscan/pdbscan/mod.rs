// Copyright (c) 2023-2024 Retake, Inc.
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
mod privdat;
mod projections;
mod qual_inspect;
mod scan_state;

use crate::api::operator::{anyelement_jsonb_opoid, attname_from_var, estimate_selectivity};
use crate::api::{AsCStr, AsInt, Cardinality};
use crate::index::score::SearchIndexScore;
use crate::index::SearchIndex;
use crate::postgres::customscan::builders::custom_path::{CustomPathBuilder, Flags};
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::pdbscan::exec_methods::{
    normal_scan_exec, top_n_scan_exec, ExecState, TopNScanExecState,
};
use crate::postgres::customscan::pdbscan::privdat::PrivateData;
use crate::postgres::customscan::pdbscan::projections::score::{
    inject_scores, is_score_func, score_funcoid, uses_scores,
};
use crate::postgres::customscan::pdbscan::projections::snippet::{
    inject_snippet, snippet_funcoid, uses_snippets, SnippetInfo,
};
use crate::postgres::customscan::pdbscan::projections::{
    maybe_needs_const_projections, pullout_funcexprs,
};
use crate::postgres::customscan::pdbscan::qual_inspect::extract_quals;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::postgres::customscan::CustomScan;
use crate::postgres::index::open_search_index;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::visibility_checker::VisibilityChecker;
use crate::query::SearchQueryInput;
use crate::{DEFAULT_STARTUP_COST, GUCS, UNKNOWN_SELECTIVITY};
use pgrx::pg_sys::AsPgCStr;
use pgrx::{pg_sys, PgList, PgMemoryContexts, PgRelation};
use scan_state::SortDirection;
use std::collections::HashMap;
use std::ffi::CStr;
use std::ptr::{addr_of, addr_of_mut};
use tantivy::snippet::SnippetGenerator;
use tantivy::DocAddress;

#[derive(Default)]
pub struct PdbScan;

impl CustomScan for PdbScan {
    const NAME: &'static CStr = c"ParadeDB Scan";
    type State = PdbScanState;
    type PrivateData = PrivateData;

    fn callback(mut builder: CustomPathBuilder<Self::PrivateData>) -> Option<pg_sys::CustomPath> {
        unsafe {
            if builder.restrict_info().is_empty() {
                return None;
            }
            let rti = builder.args().rti;
            let (table, bm25_index, is_join) = {
                let rte = builder.args().rte();

                // first, we only work on plain relations
                if rte.rtekind != pg_sys::RTEKind::RTE_RELATION
                    && rte.rtekind != pg_sys::RTEKind::RTE_JOIN
                {
                    return None;
                }
                let relkind = pg_sys::get_rel_relkind(rte.relid) as u8;
                if relkind != pg_sys::RELKIND_RELATION && relkind != pg_sys::RELKIND_MATVIEW {
                    return None;
                }

                // and that relation must have a `USING bm25` index
                let (table, bm25_index) = rel_get_bm25_index(rte.relid)?;

                (table, bm25_index, rte.rtekind == pg_sys::RTEKind::RTE_JOIN)
            };

            let pathkey = pullup_ordery_by_score_pathkey(&mut builder, rti);
            let limit = if pathkey.is_some() && (*builder.args().root).limit_tuples > -1.0 {
                // we can only use the limit if we have an orderby score pathkey
                Some((*builder.args().root).limit_tuples)
            } else {
                None
            };

            // quick look at the PathTarget list to see if we might need to do our const projections
            let path_target = builder.path_target();
            let maybe_needs_const_projections =
                maybe_needs_const_projections((*(*builder.args().root).parse).targetList.cast());

            //
            // look for quals we can support
            //
            let restrict_info = builder.restrict_info();
            if let Some(quals) =
                extract_quals(rti, restrict_info.as_ptr().cast(), anyelement_jsonb_opoid())
            {
                let selectivity = if let Some(limit) = limit {
                    // use the limit
                    limit / table.reltuples().map(|n| n as Cardinality).unwrap_or(limit)
                } else if restrict_info.len() == 1 {
                    // we can use the norm_selec that already happened
                    (*restrict_info.get_ptr(0).unwrap()).norm_selec
                } else {
                    // ask the index
                    let search_config = SearchQueryInput::from(quals);
                    estimate_selectivity(&bm25_index, &search_config).unwrap_or(UNKNOWN_SELECTIVITY)
                };

                builder.custom_private().set_heaprelid(table.oid());
                builder.custom_private().set_indexrelid(bm25_index.oid());
                builder.custom_private().set_range_table_index(rti);
                builder.custom_private().set_quals(restrict_info);

                if limit.is_some() && pathkey.is_some() {
                    // we can only set our limit/pathkey values if we have both
                    builder = builder.add_path_key(pathkey);
                    builder.custom_private().set_limit(limit);
                    builder
                        .custom_private()
                        .set_sort_direction(pathkey_sort_direction(pathkey));
                }

                let reltuples = table.reltuples().unwrap_or(1.0) as f64;
                let rows = (reltuples * selectivity).max(1.0);
                let startup_cost = DEFAULT_STARTUP_COST;

                let cpu_index_tuple_cost = pg_sys::cpu_index_tuple_cost;
                let total_cost = startup_cost + selectivity * reltuples * cpu_index_tuple_cost;

                let cpu_run_cost = {
                    // if we think we need scores, we need a much cheaper plan so that Postgres will
                    // prefer it over all the others.
                    // TODO:  these are curious values that I picked out of thin air and probably need attention
                    let per_tuple = 4.0;
                    let cpu_run_cost = pg_sys::cpu_tuple_cost + per_tuple;

                    cpu_run_cost + rows * per_tuple
                };

                let (startup_cost, total_cost, cpu_run_cost) =
                    if is_join || maybe_needs_const_projections {
                        // NB:  just force smallest costs possible so we'll be used in join and
                        // other situations where we need const projections
                        (0.0, 0.0, 0.0)
                    } else {
                        (startup_cost, total_cost, cpu_run_cost)
                    };

                builder = builder.set_rows(rows);
                builder = builder.set_startup_cost(startup_cost);
                builder = builder.set_total_cost(total_cost + cpu_run_cost);
                builder = builder.set_flag(Flags::Projection);

                return Some(builder.build());
            }
        }

        None
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

            let mut attname_lookup = PgList::<pg_sys::Node>::new();
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
                    attname_lookup.push(pg_sys::makeInteger((*var).varno as _).cast());
                    attname_lookup.push(pg_sys::makeInteger((*var).varattno as _).cast());
                    attname_lookup.push(pg_sys::makeString(attname.as_pg_cstr()).cast());
                }
            }

            builder
                .custom_private_mut()
                .set_var_attname_lookup(attname_lookup.into_pg());
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

            {
                let indexrel = PgRelation::with_lock(
                    builder.custom_state().indexrelid,
                    pg_sys::AccessShareLock as _,
                );
                let ops = indexrel.rd_options as *mut SearchIndexCreateOptions;
                let key_field = (*ops)
                    .get_key_field()
                    .expect("`USING bm25` index should have a valued `key_field` option")
                    .0;

                builder.custom_state().index_name = indexrel.name().to_string();
                builder.custom_state().key_field = key_field;
                builder.custom_state().rti = builder
                    .custom_private()
                    .range_table_index()
                    .expect("range table index should have been set");
            }

            // information about if we're sorted by score and our limit
            builder.custom_state().limit = builder.custom_private().limit();
            builder.custom_state().sort_direction = builder.custom_private().sort_direction();

            // store our query quals into our custom state too
            let quals = builder
                .custom_private()
                .quals()
                .expect("should have a Qual structure");
            builder.custom_state().search_query_input = SearchQueryInput::from(quals);

            // now build up the var attribute name lookup map
            unsafe fn populate_var_attname_lookup(
                lookup: &mut HashMap<(i32, pg_sys::AttrNumber), String>,
                iter: impl Iterator<Item = *mut pg_sys::Node>,
            ) -> Option<()> {
                let mut iter = iter.peekable();
                while let Some(node) = iter.next() {
                    let (varno, varattno, attname) = {
                        let varno = node.as_int()?;
                        let varattno = iter.next()?.as_int()?;
                        let attname = iter.next()?.as_c_str()?.as_ptr();

                        (varno, varattno, attname)
                    };

                    lookup.insert(
                        (varno as _, varattno as _),
                        CStr::from_ptr(attname).to_string_lossy().to_string(),
                    );
                }

                Some(())
            }

            let var_attname_lookup = builder
                .custom_private()
                .var_attname_lookup()
                .expect("should have an attribute name lookup");
            assert_eq!(
                var_attname_lookup.len() % 3,
                0,
                "correct number of var_attname_lookup entries"
            );

            if populate_var_attname_lookup(
                &mut builder.custom_state().var_attname_lookup,
                var_attname_lookup.iter_ptr(),
            )
            .is_none()
            {
                panic!("failed to properly build `var_attname_lookup` due to mis-typed List");
            }

            builder.custom_state().score_funcoid = score_funcoid();
            builder.custom_state().snippet_funcoid = snippet_funcoid();
            builder.custom_state().need_scores = uses_scores(
                builder.target_list().as_ptr().cast(),
                builder.custom_state().score_funcoid,
            );
            let node = builder.target_list().as_ptr().cast();
            let snippet_funcoid = builder.custom_state().snippet_funcoid;
            let rti = builder.custom_state().rti;
            let attname_lookup = &builder.custom_state().var_attname_lookup;
            builder.custom_state().snippet_generators =
                uses_snippets(rti, attname_lookup, node, snippet_funcoid)
                    .into_iter()
                    .map(|field| (field, None))
                    .collect();

            builder.build()
        }
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    ) {
        explainer.add_text("Table", state.custom_state().heaprelname());
        explainer.add_text("Index", &state.custom_state().index_name);
        if explainer.is_analyze() && state.custom_state().invisible_tuple_count > 0 {
            explainer.add_unsigned_integer(
                "Invisible Tuples",
                state.custom_state().invisible_tuple_count as u64,
                None,
            );
        }

        explainer.add_bool("Scores", state.custom_state().need_scores());
        if let (Some(sort_direction), Some(limit)) = (
            state.custom_state().sort_direction,
            state.custom_state().limit,
        ) {
            explainer.add_text("   Sort Direction", sort_direction);
            explainer.add_unsigned_integer("   Top N Limit", limit as u64, None);
            if explainer.is_analyze() && state.custom_state().retry_count > 0 {
                explainer.add_unsigned_integer(
                    "   Invisible Tuple Retries",
                    state.custom_state().retry_count as u64,
                    None,
                );
            }
        }

        let query = &state.custom_state().search_query_input;
        let pretty_json = if explainer.is_verbose() {
            serde_json::to_string_pretty(&query)
        } else {
            serde_json::to_string(&query)
        }
        .expect("query should serialize to json");
        explainer.add_text("Tantivy Query", &pretty_json);
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
            let heaprel = pg_sys::relation_open(state.custom_state().heaprelid, lockmode);
            let indexrel = pg_sys::relation_open(state.custom_state().indexrelid, lockmode);
            state.custom_state_mut().heaprel = Some(heaprel);
            state.custom_state_mut().indexrel = Some(indexrel);
            state.custom_state_mut().lockmode = lockmode;

            // setup the structures we need to do mvcc checking
            state.custom_state_mut().snapshot = Some(pg_sys::GetActiveSnapshot());
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
        }

        if eflags & (pg_sys::EXEC_FLAG_EXPLAIN_ONLY as i32) != 0 {
            // don't do anything else if we're only explaining the query
            return;
        }

        PdbScan::rescan_custom_scan(state)
    }

    #[allow(clippy::blocks_in_conditions)]
    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        let scan_func = unsafe {
            // extra help during debugging is cool
            debug_assert!(
                state.custom_state().scan_func.is_some(),
                "exec_custom_scan: scan_func should be set"
            );
            // SAFETY:  we assign the scan_func down in rescan_custom_scan() and assert there that it's valid
            *state.custom_state().scan_func.as_ref().unwrap_unchecked()
        };
        loop {
            // get the next matching document from our search results and look for it in the heap
            match scan_func(state, unsafe {
                state.custom_state().inner_scan_state.unwrap_unchecked()
            }) {
                // reached the end of the SearchResults
                ExecState::Eof => return std::ptr::null_mut(),

                // SearchResults returned a tuple we can't see
                ExecState::Invisible { .. } => {
                    state.custom_state_mut().invisible_tuple_count += 1;
                    continue;
                }

                // SearchResults found the tuple
                ExecState::Found {
                    scored,
                    doc_address,
                    slot,
                } => {
                    unsafe {
                        // project it if we need to
                        let projection_info =
                            maybe_rebuild_projinfo_for_const_projection(state, scored, doc_address);

                        // finally, do the projection
                        (*(*projection_info).pi_exprContext).ecxt_scantuple = slot;
                        return pg_sys::ExecProject(projection_info);
                    }
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

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        let need_scores = state.custom_state().need_scores();
        let need_snippets = state.custom_state().need_snippets();
        let mut search_config = state.custom_state().search_config.clone();

        search_config.stable_sort = Some(false);
        search_config.need_scores = need_scores;

        // Open the index and query it
        let indexrel = state
            .custom_state()
            .indexrel
            .as_ref()
            .map(|indexrel| unsafe { PgRelation::from_pg(*indexrel) })
            .expect("custom_state.indexrel should already be open");
        let search_index =
            open_search_index(&indexrel).expect("should be able to open search index");
        let search_reader = search_index
            .get_reader()
            .expect("search index reader should have been constructed correctly");

        let search_query_input = &state.custom_state().search_query_input;

        state.custom_state_mut().query =
            Some(search_index.query(&search_query_input, &search_reader));
        let search_results = if let (Some(limit), Some(sort_direction)) = (
            state.custom_state().limit,
            state.custom_state().sort_direction,
        ) {
            let results = search_reader.search_top_n(
                SearchIndex::executor(),
                state.custom_state().query.as_ref().unwrap(),
                sort_direction.into(),
                limit,
            );
            state.custom_state_mut().scan_func = Some(top_n_scan_exec);
            state.custom_state_mut().inner_scan_state = unsafe {
                let mut topn_state = TopNScanExecState::default();
                topn_state.limit = results.len().unwrap();
                topn_state.have_less = topn_state.limit < state.custom_state().limit.unwrap();
                Some(
                    PgMemoryContexts::CurrentMemoryContext
                        .copy_ptr_into(&mut topn_state, std::mem::size_of::<TopNScanExecState>())
                        .cast(),
                )
            };
            results
        } else {
            let results = search_reader.search_minimal(
                false,
                SearchIndex::executor(),
                &search_config,
                state.custom_state().query.as_ref().unwrap(),
            );
            state.custom_state_mut().scan_func = Some(normal_scan_exec);
            state.custom_state_mut().inner_scan_state = Some(std::ptr::null_mut());
            results
        };

        state.custom_state_mut().search_results = search_results;

        assert!(
            state.custom_state().scan_func.is_some(),
            "CustomScan scan_func should be set"
        );
        assert!(
            state.custom_state().inner_scan_state.is_some(),
            "CustomScan inner_scan_state should be set"
        );

        if need_snippets {
            let mut snippet_generators: HashMap<SnippetInfo, Option<SnippetGenerator>> = state
                .custom_state_mut()
                .snippet_generators
                .drain()
                .collect();
            let query = &state.custom_state().query.as_ref().unwrap();
            for (snippet_info, generator) in &mut snippet_generators {
                let mut new_generator =
                    search_reader.snippet_generator(snippet_info.field.as_ref(), *query);
                new_generator.set_max_num_chars(snippet_info.max_num_chars);
                *generator = Some(new_generator);
            }

            state.custom_state_mut().snippet_generators = snippet_generators;
        }

        state.custom_state_mut().search_reader = Some(search_reader);
    }
}

/// Use the [`VisibilityChecker`] to lookup the [`SearchIndexScore`] document in the underlying heap
/// and if it exists return a formed [`TupleTableSlot`].
#[inline(always)]
fn make_tuple_table_slot(
    state: &mut CustomScanStateWrapper<PdbScan>,
    scored: &SearchIndexScore,
    bslot: *mut pg_sys::BufferHeapTupleTableSlot,
) -> Option<*mut pg_sys::TupleTableSlot> {
    state
        .custom_state_mut()
        .visibility_checker()
        .exec_if_visible(scored.ctid, move |heaprelid, htup, buffer| unsafe {
            (*bslot).base.base.tts_tableOid = heaprelid;
            (*bslot).base.tupdata = htup;
            (*bslot).base.tupdata.t_self = (*htup.t_data).t_ctid;

            // materialize a heap tuple for it
            pg_sys::ExecStoreBufferHeapTuple(
                addr_of_mut!((*bslot).base.tupdata),
                bslot.cast(),
                buffer,
            )
        })
}

unsafe fn maybe_rebuild_projinfo_for_const_projection(
    state: &mut CustomScanStateWrapper<PdbScan>,
    scored: SearchIndexScore,
    doc_address: DocAddress,
) -> *mut pg_sys::ProjectionInfo {
    if !state.custom_state().need_scores() && !state.custom_state().need_snippets() {
        // scores/snippets aren't necessary so we use whatever we originally setup as our ProjectionInfo
        return state.projection_info();
    }

    // the query requires scores.  since we have it in `scored.bm25`, we inject
    // that constant value into every position in the Plan's TargetList that uses
    // our `paradedb.score(record)` function.  This is what `inject_scores()` does
    // and it returns a whole new TargetList.
    //
    // It's from that TargetList we build a new ProjectionInfo with the FuncExprs
    // replaced with the actual score f32 Const nodes.  Essentially we're manually
    // projecting the scores where they need to go
    let planstate = state.planstate();
    let projection_targetlist = (*(*planstate).plan).targetlist;

    let mut const_projected_targetlist = projection_targetlist;

    if state.custom_state().need_scores() {
        const_projected_targetlist = inject_scores(
            const_projected_targetlist.cast(),
            state.custom_state().score_funcoid,
            scored.bm25,
        )
        .cast();
    }
    if state.custom_state().need_snippets() {
        let snippet_funcoid = state.custom_state().snippet_funcoid;
        let search_state = state
            .custom_state()
            .search_reader
            .as_ref()
            .expect("CustomState should hae a SearchState since it requires snippets");
        for (snippet_info, generator) in &state.custom_state().snippet_generators {
            const_projected_targetlist = inject_snippet(
                state.custom_state().rti,
                &state.custom_state().var_attname_lookup,
                const_projected_targetlist.cast(),
                snippet_funcoid,
                search_state,
                &snippet_info.field,
                &snippet_info.start_tag,
                &snippet_info.end_tag,
                generator
                    .as_ref()
                    .expect("SnippetGenerator should have been created"),
                doc_address,
            )
            .cast();
        }
    }

    // build the new ProjectionInfo based on our modified TargetList
    pg_sys::ExecBuildProjectionInfo(
        const_projected_targetlist.cast(),
        (*planstate).ps_ExprContext,
        (*planstate).ps_ResultTupleSlot,
        planstate,
        (*state.csstate.ss.ss_ScanTupleSlot).tts_tupleDescriptor,
    )
}

unsafe fn pullup_ordery_by_score_pathkey<P: Into<*mut pg_sys::List> + Default>(
    builder: &mut CustomPathBuilder<P>,
    rti: pg_sys::Index,
) -> Option<*mut pg_sys::PathKey> {
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*builder.args().root).query_pathkeys);
    let mut pathkey = None;
    if let Some(first_pathkey) = pathkeys.get_ptr(0) {
        let equivclass = (*first_pathkey).pk_eclass;
        let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

        for member in members.iter_ptr() {
            if is_score_func((*member).em_expr.cast(), rti as _) {
                pathkey = Some(first_pathkey);
                break;
            }
        }
    }
    pathkey
}

unsafe fn pathkey_sort_direction(pathkey: Option<*mut pg_sys::PathKey>) -> Option<SortDirection> {
    pathkey.map(|pathkey| (*pathkey).pk_strategy.into())
}

fn tts_ops_name(slot: *mut pg_sys::TupleTableSlot) -> &'static str {
    unsafe {
        if std::ptr::eq((*slot).tts_ops, addr_of!(pg_sys::TTSOpsVirtual)) {
            "Virtual"
        } else if std::ptr::eq((*slot).tts_ops, addr_of!(pg_sys::TTSOpsHeapTuple)) {
            "Heap"
        } else if std::ptr::eq((*slot).tts_ops, addr_of!(pg_sys::TTSOpsMinimalTuple)) {
            "Minimal"
        } else if std::ptr::eq((*slot).tts_ops, addr_of!(pg_sys::TTSOpsBufferHeapTuple)) {
            "BufferHeap"
        } else {
            "Unknown"
        }
    }
}
