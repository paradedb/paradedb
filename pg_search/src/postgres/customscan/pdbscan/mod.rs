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
mod projections;
mod qual_inspect;

use crate::api::operator::{anyelement_jsonb_opoid, attname_from_var, estimate_selectivity};
use crate::api::{AsCStr, AsInt};
use crate::index::reader::{SearchIndexReader, SearchResults};
use crate::index::{SearchIndex, WriterDirectory};
use crate::postgres::customscan::builders::custom_path::{CustomPathBuilder, Flags};
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::pdbscan::projections::score::{
    inject_scores, score_funcoid, uses_scores,
};
use crate::postgres::customscan::pdbscan::projections::snippet::{
    inject_snippet, snippet_funcoid, uses_snippets, SnippetInfo,
};
use crate::postgres::customscan::pdbscan::projections::{
    maybe_needs_const_projections, pullout_funcexprs,
};
use crate::postgres::customscan::pdbscan::qual_inspect::{extract_quals, Qual};
use crate::postgres::customscan::{CustomScan, CustomScanState};
use crate::postgres::options::SearchIndexCreateOptions;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::utils::{
    relfilenode_from_index_oid, relfilenode_from_pg_relation, VisibilityChecker,
};
use crate::schema::SearchConfig;
use crate::{DEFAULT_STARTUP_COST, GUCS, UNKNOWN_SELECTIVITY};
use pgrx::pg_sys::AsPgCStr;
use pgrx::{name_data_to_str, pg_sys, PgList, PgRelation};
use shared::gucs::GlobalGucSettings;
use std::collections::HashMap;
use std::ffi::CStr;
use std::ptr::{addr_of, addr_of_mut};
use tantivy::snippet::SnippetGenerator;

#[derive(Default)]
pub struct PdbScan;

#[derive(Default)]
pub struct PdbScanState {
    heaprelid: pg_sys::Oid,
    indexrelid: pg_sys::Oid,
    rti: pg_sys::Index,

    index_name: String,
    index_uuid: String,
    key_field: String,
    search_reader: Option<SearchIndexReader>,
    search_config: SearchConfig,
    search_results: SearchResults,

    heaprel: Option<pg_sys::Relation>,
    indexrel: Option<pg_sys::Relation>,
    lockmode: pg_sys::LOCKMODE,

    snapshot: Option<pg_sys::Snapshot>,
    visibility_checker: Option<VisibilityChecker>,

    need_scores: bool,
    snippet_generators: HashMap<SnippetInfo, Option<SnippetGenerator>>,
    score_funcoid: pg_sys::Oid,
    snippet_funcoid: pg_sys::Oid,
    var_attname_lookup: HashMap<(i32, pg_sys::AttrNumber), String>,
}

impl CustomScanState for PdbScanState {}

impl PdbScanState {
    #[inline(always)]
    pub fn need_scores(&self) -> bool {
        self.need_scores
    }

    #[inline(always)]
    pub fn need_snippets(&self) -> bool {
        !self.snippet_generators.is_empty()
    }

    #[inline(always)]
    pub fn snapshot(&self) -> pg_sys::Snapshot {
        self.snapshot.unwrap()
    }

    #[inline(always)]
    pub fn heaprel(&self) -> pg_sys::Relation {
        self.heaprel.unwrap()
    }

    #[inline(always)]
    pub fn heaprelname(&self) -> &str {
        unsafe { name_data_to_str(&(*(*self.heaprel()).rd_rel).relname) }
    }

    #[inline(always)]
    pub fn heaptupdesc(&self) -> pg_sys::TupleDesc {
        unsafe { (*self.heaprel()).rd_att }
    }

    #[inline(always)]
    pub fn visibility_checker(&mut self) -> &mut VisibilityChecker {
        self.visibility_checker.as_mut().unwrap()
    }
}

struct PrivateData(PgList<pg_sys::Node>);

impl PrivateData {
    unsafe fn heaprelid(&self) -> Option<pg_sys::Oid> {
        self.0
            .get_ptr(0)
            .and_then(|node| node.as_int().map(|i| pg_sys::Oid::from(i as u32)))
    }

    unsafe fn indexrelid(&self) -> Option<pg_sys::Oid> {
        self.0
            .get_ptr(1)
            .and_then(|node| node.as_int().map(|i| pg_sys::Oid::from(i as u32)))
    }

    unsafe fn range_table_index(&self) -> Option<pg_sys::Index> {
        self.0
            .get_ptr(2)
            .and_then(|node| node.as_int().map(|i| i as pg_sys::Index))
    }

    fn quals(&self) -> Option<Qual> {
        let base_restrict_info = self.0.get_ptr(3)?;
        unsafe { extract_quals(base_restrict_info, anyelement_jsonb_opoid()) }
    }

    fn var_attname_lookup(&self) -> Option<PgList<pg_sys::Node>> {
        unsafe {
            self.0
                .get_ptr(4)
                .map(|ptr| PgList::<pg_sys::Node>::from_pg(ptr.cast()))
        }
    }
}

impl CustomScan for PdbScan {
    const NAME: &'static CStr = c"ParadeDB Scan";
    type State = PdbScanState;

    fn callback(mut builder: CustomPathBuilder) -> Option<pg_sys::CustomPath> {
        if !GUCS.enable_custom_scan() {
            return None;
        }

        unsafe {
            if builder.restrict_info().is_empty() {
                return None;
            }
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

            // quick look at the PathTarget list to see if we might need to do our const projections
            let path_target = builder.path_target();
            let maybe_needs_const_projections =
                maybe_needs_const_projections((*(*builder.args().root).parse).targetList.cast());
            let is_join = rte.rtekind == pg_sys::RTEKind::RTE_JOIN;

            //
            // look for quals we can support
            //
            if let Some(quals) = extract_quals(
                builder.restrict_info().as_ptr().cast(),
                anyelement_jsonb_opoid(),
            ) {
                let rti = builder.args().rti;
                builder = builder
                    .add_private_data(pg_sys::makeInteger(table.oid().as_u32() as _).cast())
                    .add_private_data(pg_sys::makeInteger(bm25_index.oid().as_u32() as _).cast())
                    .add_private_data(pg_sys::makeInteger(rti as _).cast());

                let restrict_info = builder.restrict_info();

                let selectivity = if restrict_info.len() == 1 {
                    // we can use the norm_selec that already happened
                    (*restrict_info.get_ptr(0).unwrap()).norm_selec
                } else {
                    // ask the index
                    let search_config = SearchConfig::from(quals);
                    estimate_selectivity(
                        table.oid(),
                        relfilenode_from_pg_relation(&bm25_index),
                        &search_config,
                    )
                    .unwrap_or(UNKNOWN_SELECTIVITY)
                };

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

                let root = builder.args().root;
                builder = builder.set_rows(rows);
                builder = builder.set_startup_cost(startup_cost);
                builder = builder.set_total_cost(total_cost + cpu_run_cost);
                builder = builder.add_private_data(restrict_info.into_pg().cast());

                builder = builder.set_flag(Flags::Projection);

                return Some(builder.build());
            }
        }

        None
    }

    fn plan_custom_path(mut builder: CustomScanBuilder) -> pg_sys::CustomScan {
        unsafe {
            let private_data =
                PrivateData(PgList::<pg_sys::Node>::from_pg(builder.custom_private()));
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

            builder.add_private_data(attname_lookup.into_pg().cast());

            builder.build()
        }
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self>,
    ) -> *mut CustomScanStateWrapper<Self> {
        unsafe {
            let private_data = PrivateData(builder.private_data());
            let heaprelid = private_data
                .heaprelid()
                .expect("heaprelid should have a value");
            let indexrelid = private_data
                .indexrelid()
                .expect("indexrelid should have a value");

            builder.custom_state().heaprelid = heaprelid;
            builder.custom_state().indexrelid = indexrelid;

            {
                let indexrel = PgRelation::with_lock(indexrelid, pg_sys::AccessShareLock as _);
                let ops = indexrel.rd_options as *mut SearchIndexCreateOptions;
                let uuid = (*ops)
                    .get_uuid()
                    .expect("`USING bm25` index should have a value `uuid` option");
                let key_field = (*ops)
                    .get_key_field()
                    .expect("`USING bm25` index should have a valued `key_field` option")
                    .0;

                builder.custom_state().index_name = indexrel.name().to_string();
                builder.custom_state().index_uuid = uuid;
                builder.custom_state().key_field = key_field;
                builder.custom_state().rti = private_data
                    .range_table_index()
                    .expect("range table index should have been set");
            }

            let quals = private_data.quals().expect("should have a Qual structure");
            builder.custom_state().search_config = SearchConfig::from(quals);

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

            let var_attname_lookup = private_data
                .var_attname_lookup()
                .expect("should have an attribute name lookup");
            assert_eq!(var_attname_lookup.len() % 3, 0);

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
            let attname_lookup = &builder.custom_state().var_attname_lookup;
            builder.custom_state().snippet_generators =
                uses_snippets(attname_lookup, node, snippet_funcoid)
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
        explainer.add_text("Table", state.custom_state.heaprelname());
        explainer.add_text("Index", &state.custom_state.index_name);
        explainer.add_bool("Scores", state.custom_state.need_scores());

        let query = &state.custom_state.search_config.query;
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
            state.custom_state().heaprel = Some(heaprel);
            state.custom_state().indexrel = Some(indexrel);
            state.custom_state().lockmode = lockmode;

            // setup the structures we need to do mvcc checking
            state.custom_state().snapshot = Some(pg_sys::GetActiveSnapshot());
            state.custom_state().visibility_checker = Some(VisibilityChecker::with_rel_and_snap(
                heaprel,
                pg_sys::GetActiveSnapshot(),
            ));

            // and finally, get the custom scan itself properly initialized
            let tupdesc = state.custom_state.heaptupdesc();
            pg_sys::ExecInitScanTupleSlot(
                estate,
                addr_of_mut!(state.csstate.ss),
                tupdesc,
                pg_sys::table_slot_callbacks(state.custom_state.heaprel()),
            );
            pg_sys::ExecInitResultTypeTL(addr_of_mut!(state.csstate.ss.ps));
            pg_sys::ExecAssignProjectionInfo(
                state.planstate(),
                (*state.csstate.ss.ss_ScanTupleSlot).tts_tupleDescriptor,
            );
        }

        PdbScan::rescan_custom_scan(state)
    }

    #[allow(clippy::blocks_in_conditions)]
    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        loop {
            // get the next matching document from our search results and look for it in the heap
            let (scored, slot) = match state.custom_state().search_results.next() {
                // we've returned all the matching results
                None => return std::ptr::null_mut(),

                // need to fetch the returned ctid from the heap and store its heap representation
                // in a TupleTableSlow
                Some((scored, _)) => {
                    let scanslot = state.scanslot();
                    let bslot = state.scanslot() as *mut pg_sys::BufferHeapTupleTableSlot;
                    let heaprelid = state.custom_state().heaprelid;

                    // ask the visibility checker to find the document in the postgres heap
                    match state.custom_state().visibility_checker().exec_if_visible(
                        scored.ctid,
                        move |htup, buffer| unsafe {
                            (*bslot).base.base.tts_tableOid = heaprelid;
                            (*bslot).base.tupdata = htup;
                            (*bslot).base.tupdata.t_self = (*htup.t_data).t_ctid;

                            // materialize a heap tuple for it
                            pg_sys::ExecStoreBufferHeapTuple(
                                addr_of_mut!((*bslot).base.tupdata),
                                bslot.cast(),
                                buffer,
                            )
                        },
                    ) {
                        // ctid isn't visible, move to the next one
                        None => continue,

                        // we found the ctid in the heap
                        Some(slot) => (scored, slot),
                    }
                }
            };

            unsafe {
                let mut projection_info = state.projection_info();

                projection_info = if state.custom_state().need_scores()
                    || state.custom_state.need_snippets()
                {
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
                        let snippet_funcoid = state.custom_state.snippet_funcoid;
                        let search_reader = state.custom_state.search_reader.as_ref().expect(
                            "CustomState should have a SearchIndexReader since it requires snippets",
                        );
                        for (snippet_info, generator) in &mut state.custom_state.snippet_generators
                        {
                            const_projected_targetlist = inject_snippet(
                                &state.custom_state.var_attname_lookup,
                                const_projected_targetlist.cast(),
                                snippet_funcoid,
                                search_reader,
                                &snippet_info.field,
                                &snippet_info.start_tag,
                                &snippet_info.end_tag,
                                snippet_info.max_num_chars,
                                generator
                                    .as_mut()
                                    .expect("SnippetGenerator should have been created"),
                                scored
                                    .doc_address
                                    .expect("should have generated a DocAddress"),
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
                } else {
                    // scores aren't necessary so we use whatever we originally setup as our ProjectionInfo
                    projection_info
                };

                // finally, do the projection
                (*(*projection_info).pi_exprContext).ecxt_scantuple = slot;
                return pg_sys::ExecProject(projection_info);
            }
        }
    }

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>) {}

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        // get the VisibilityChecker dropped
        state.custom_state().visibility_checker.take();

        if let Some(heaprel) = state.custom_state().heaprel.take() {
            unsafe {
                pg_sys::relation_close(heaprel, state.custom_state().lockmode);
            }
        }
        if let Some(indexrel) = state.custom_state().indexrel.take() {
            unsafe {
                pg_sys::relation_close(indexrel, state.custom_state().lockmode);
            }
        }
    }

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        let indexrelid = state.custom_state.indexrelid;
        let need_scores = state.custom_state.need_scores();
        let need_snippets = state.custom_state.need_snippets();
        let search_config = &mut state.custom_state.search_config;

        search_config.stable_sort = Some(false);
        search_config.need_scores = need_scores;

        // Create the index and scan state
        let database_oid = crate::MyDatabaseId();
        let relfilenode = relfilenode_from_index_oid(indexrelid.as_u32());

        let directory =
            WriterDirectory::from_oids(database_oid, indexrelid.as_u32(), relfilenode.as_u32());
        let search_index = SearchIndex::from_cache(&directory, &search_config.uuid)
            .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));

        let search_reader = search_index
            .get_reader()
            .expect("search index reader should have been constructed correctly");

        let query = search_index.query(search_config, &search_reader);
        state.custom_state.search_results =
            search_reader.search_minimal(false, SearchIndex::executor(), search_config, &query);

        if need_snippets {
            for (snippet_info, generator) in state.custom_state.snippet_generators.iter_mut() {
                *generator =
                    Some(search_reader.snippet_generator(snippet_info.field.as_ref(), &query))
            }
            state.custom_state.search_reader = Some(search_reader);
        }
    }
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
