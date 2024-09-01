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

use crate::customscan::builders::custom_path::{CustomPathBuilder, Flags};
use crate::customscan::builders::custom_scan::CustomScanBuilder;
use crate::customscan::builders::custom_state::{CustomScanStateBuilder, CustomScanStateWrapper};
use crate::customscan::explainer::Explainer;
use crate::customscan::port::executor_h::{ExecProject, ResetExprContext};
use crate::customscan::port::tuptable_h::ExecClearTuple;
use crate::customscan::{node, CustomScan, CustomScanState};
use crate::env::needs_commit;
use crate::globals::WriterGlobal;
use crate::index::state::SearchResults;
use crate::index::SearchIndex;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::utils::VisibilityChecker;
use crate::query::SearchQueryInput;
use crate::schema::SearchConfig;
use crate::writer::WriterDirectory;
use pgrx::pg_sys::{table_slot_callbacks, CustomPath, EState, TupleTableSlot};
use pgrx::{
    is_a, name_data_to_str, node_to_string, pg_sys, FromDatum, IntoDatum, PgList, PgRelation,
    PgTupleDesc,
};
use std::ffi::CStr;

#[derive(Default)]
pub struct Example;

#[derive(Default)]
pub struct ExampleScanState {
    snapshot: Option<pg_sys::Snapshot>,
    heaprel: Option<pg_sys::Relation>,
    index_name: String,
    index_oid: pg_sys::Oid,
    index_uuid: String,
    key_field: String,
    tantivy_query: String,
    search_results: SearchResults,

    visibility_checker: Option<VisibilityChecker>,
    score_field_indices: Vec<usize>,
}

impl CustomScanState for ExampleScanState {}

impl ExampleScanState {
    #[inline(always)]
    pub fn snapshot(&self) -> pg_sys::Snapshot {
        self.snapshot.unwrap()
    }

    #[inline(always)]
    pub fn heaprel(&self) -> pg_sys::Relation {
        self.heaprel.unwrap()
    }

    #[inline(always)]
    pub fn heaprelid(&self) -> pg_sys::Oid {
        unsafe { (*self.heaprel()).rd_id }
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
    fn heaprelid(&self) -> Option<pg_sys::Oid> {
        unsafe {
            Some(pg_sys::Oid::from(
                (*node::<pg_sys::Integer>(self.0.get_ptr(0)?.cast(), pg_sys::NodeTag::T_Integer)?)
                    .ival as u32,
            ))
        }
    }

    fn indexrelid(&self) -> Option<pg_sys::Oid> {
        unsafe {
            Some(pg_sys::Oid::from(
                (*node::<pg_sys::Integer>(self.0.get_ptr(1)?.cast(), pg_sys::NodeTag::T_Integer)?)
                    .ival as u32,
            ))
        }
    }

    fn quals(
        &self,
    ) -> impl Iterator<Item = (Option<*mut pg_sys::Var>, Option<*mut pg_sys::Const>)> + '_ {
        let mut range = (2..self.0.len()).step_by(2);
        std::iter::from_fn(move || unsafe {
            let i = range.next()?;
            let var = node::<pg_sys::Var>(self.0.get_ptr(i)?.cast(), pg_sys::NodeTag::T_Var);
            let const_ =
                node::<pg_sys::Const>(self.0.get_ptr(i + 1)?.cast(), pg_sys::NodeTag::T_Const);
            Some((var, const_))
        })
    }
}

impl CustomScan for Example {
    const NAME: &'static CStr = c"ParadeDB Example Scan";
    type State = ExampleScanState;

    fn callback(mut builder: CustomPathBuilder) -> Option<CustomPath> {
        unsafe {
            let rte = builder.args().rte();

            // first, we only work on plain relations
            if rte.rtekind != pg_sys::RTEKind::RTE_RELATION {
                return None;
            }
            let relkind = pg_sys::get_rel_relkind(rte.relid) as u8;
            if relkind != pg_sys::RELKIND_RELATION && relkind != pg_sys::RELKIND_MATVIEW {
                return None;
            }

            // and that relation must have a `USING bm25` index
            let (table, bm25_index) = rel_get_bm25_index(rte.relid)?;
            let mut quals = Vec::new();

            //
            // look for quals we can support
            //
            for ri in builder.base_restrict_info().iter_ptr() {
                if let Some(clause) =
                    node::<pg_sys::OpExpr>((*ri).clause.cast(), pg_sys::NodeTag::T_OpExpr)
                {
                    // this is just hacky code for testing.
                    //
                    // matches:  text_field = 'string value'
                    if (*clause).opno == pg_sys::Oid::from(pg_sys::TextEqualOperator) {
                        let args = PgList::<pg_sys::Node>::from_pg((*clause).args);
                        if args.len() == 2 {
                            let first = args.get_ptr(0).unwrap();
                            let second = args.get_ptr(1).unwrap();

                            if let (Some(first), Some(second)) = (
                                node::<pg_sys::Var>(first.cast(), pg_sys::NodeTag::T_Var),
                                node::<pg_sys::Const>(second.cast(), pg_sys::NodeTag::T_Const),
                            ) {
                                if (*second).consttype == pg_sys::TEXTOID {
                                    quals.push((first, second))
                                }
                            }
                        }
                    }
                }
            }

            if !quals.is_empty() {
                builder = builder
                    .add_private_data(pg_sys::makeInteger(table.oid().as_u32() as _).cast())
                    .add_private_data(pg_sys::makeInteger(bm25_index.oid().as_u32() as _).cast());

                for (var, const_val) in quals {
                    builder = builder
                        .add_private_data(var.cast())
                        .add_private_data(const_val.cast());
                }

                return Some(builder.set_flag(Flags::Projection).build());
            }
        }

        None
    }

    fn plan_custom_path(builder: CustomScanBuilder) -> pgrx::pg_sys::CustomScan {
        builder.build()
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self>,
    ) -> CustomScanStateWrapper<Self> {
        unsafe {
            let private_data = PrivateData(builder.private_data());
            let heaprelid = private_data
                .heaprelid()
                .expect("heaprelid should have a value");
            let indexrelid = private_data
                .indexrelid()
                .expect("indexrelid should have a value");

            {
                let indexrel = PgRelation::with_lock(indexrelid, pg_sys::AccessShareLock as _);
                let ops = indexrel.rd_options as *mut SearchIndexCreateOptions;
                let uuid = unsafe { &*ops }
                    .get_uuid()
                    .expect("`USING bm25` index should have a value `uuid` option");
                let key_field = unsafe { &*ops }
                    .get_key_field()
                    .expect("`USING bm25` index should have a valued `key_field` option")
                    .0;

                builder.custom_state().index_oid = indexrel.oid();
                builder.custom_state().index_name = indexrel.name().to_string();
                builder.custom_state().index_uuid = uuid;
                builder.custom_state().key_field = key_field;
            }

            let heaprel = pg_sys::relation_open(heaprelid, pg_sys::AccessShareLock as _);
            let tupdesc = PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);

            let tantivy_query = &mut builder.custom_state().tantivy_query;
            for (var, const_) in private_data.quals() {
                let var = var.expect("node should be a `Var`");
                let const_ = const_.expect("node should be a `Const`");

                let attname = tupdesc
                    .get((*var).varattno as usize - 1)
                    .unwrap_or_else(|| {
                        panic!(
                            "heap relation should have an attribute with number {}",
                            (*var).varattno
                        )
                    })
                    .name();
                let value =
                    <&str>::from_datum((*const_).constvalue, (*const_).constisnull).unwrap();

                if !tantivy_query.is_empty() {
                    tantivy_query.push_str(" AND ");
                }

                tantivy_query.push_str(attname);
                tantivy_query.push_str(":(");
                tantivy_query.push_str(value);
                tantivy_query.push(')');
            }

            builder.custom_state().heaprel = Some(heaprel);
            builder.custom_state().snapshot = Some(pg_sys::GetActiveSnapshot());
            builder.custom_state().visibility_checker = Some(VisibilityChecker::with_rel_and_snap(
                heaprel,
                pg_sys::GetActiveSnapshot(),
            ));

            // look for columns named "score_bm25" of type FLOAT4
            unsafe {
                for (i, entry) in builder.target_list().iter_ptr().enumerate() {
                    let entry = entry.as_ref().expect("`TargetEntry` should not be null");
                    let expr = (*entry).expr;

                    if is_a(expr.cast(), pg_sys::NodeTag::T_Const) {
                        let const_: *mut pg_sys::Const = expr.cast();
                        if (*const_).consttype == pg_sys::FLOAT4OID
                            && CStr::from_ptr((*entry).resname) == c"score_bm25"
                        {
                            builder.custom_state().score_field_indices.push(i);
                        }
                    }
                }
            }

            builder.build()
        }
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pgrx::pg_sys::List,
        explainer: &mut Explainer,
    ) {
        let tupdesc = unsafe { PgTupleDesc::from_pg_unchecked(state.projection_tupdesc()) };
        let mut projections = String::new();

        for att in tupdesc {
            if !projections.is_empty() {
                projections.push_str(", ");
            }
            projections.push_str(att.name());
        }

        explainer.add_text("Table", state.custom_state.heaprelname());
        explainer.add_text("Index", &state.custom_state.index_name);
        (!projections.is_empty()).then(|| explainer.add_text("Projections", &projections));
        explainer.add_text("Tantivy Query", &state.custom_state.tantivy_query);
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut EState,
        eflags: i32,
    ) {
        unsafe {
            let tupdesc = state.custom_state.heaptupdesc();

            pg_sys::ExecInitScanTupleSlot(
                estate,
                &mut state.csstate.ss,
                tupdesc,
                table_slot_callbacks(state.custom_state.heaprel()),
            );
            pg_sys::ExecInitResultTypeTL(&mut state.csstate.ss.ps);
            pg_sys::ExecAssignProjectionInfo(&mut state.csstate.ss.ps, tupdesc);
        }

        let indexrelid = state.custom_state.index_oid.as_u32();
        #[rustfmt::skip]
        let search_config = SearchConfig {
            query: SearchQueryInput::Parse {query_string: state.custom_state.tantivy_query.clone()},
            index_name: state.custom_state.index_name.clone(),
            index_oid: indexrelid,
            table_oid: state.custom_state.heaprelid().as_u32(),
            key_field: state.custom_state.key_field.clone(),
            offset_rows: None,
            limit_rows: None,
            max_num_chars: None,
            highlight_field: None,
            prefix: None,
            postfix: None,
            stable_sort: Some(false),   // for speed!
            uuid: state.custom_state.index_uuid.clone(),
            order_by_field: None,
            order_by_direction: None,
        };

        // Create the index and scan state
        let directory = WriterDirectory::from_index_oid(indexrelid);
        let search_index = SearchIndex::from_cache(&directory, &search_config.uuid)
            .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));
        let writer_client = WriterGlobal::client();

        let search_state = search_index
            .search_state(&writer_client, &search_config, needs_commit(indexrelid))
            .expect("`SearchState` should have been constructed correctly");

        state.custom_state.search_results = search_state.search_minimal(SearchIndex::executor());
    }

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut TupleTableSlot {
        loop {
            match state.custom_state.search_results.next() {
                // we've returned all the matching results
                None => return std::ptr::null_mut(),

                // need to fetch the returned ctid from the heap and perform projection
                Some((scored, _)) => {
                    let bslot = state.scanslot() as *mut pg_sys::BufferHeapTupleTableSlot;
                    let heaprelid = state.custom_state.heaprelid();
                    // let scanslot = state.scanslot();
                    let slot = state.custom_state.visibility_checker().exec_if_visible(
                        scored.ctid,
                        move |mut htup, buffer| unsafe {
                            (*bslot).base.base.tts_tableOid = heaprelid;
                            (*bslot).base.tupdata.t_self = (*htup.t_data).t_ctid;
                            pg_sys::ExecStoreBufferHeapTuple(&mut htup, bslot.cast(), buffer)
                        },
                    );

                    match slot {
                        // project the slot and return it
                        Some(slot) => unsafe {
                            state.set_projection_scanslot(slot);
                            let slot = ExecProject(state.projection_info());

                            for i in &state.custom_state.score_field_indices {
                                let i = *i;

                                if i < (*slot).tts_nvalid as usize {
                                    let values = std::slice::from_raw_parts_mut(
                                        (*slot).tts_values,
                                        (*slot).tts_nvalid as usize,
                                    );
                                    let nulls = std::slice::from_raw_parts_mut(
                                        (*slot).tts_isnull,
                                        (*slot).tts_nvalid as usize,
                                    );

                                    values[i] = scored.bm25.into_datum().unwrap();
                                    nulls[i] = false;
                                }
                            }
                            return slot;
                        },

                        // ctid isn't visible, move to the next one
                        None => continue,
                    }
                }
            }
        }
    }

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        // TODO:  anything to do here?
    }

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        if let Some(heaprel) = state.custom_state.heaprel.take() {
            unsafe {
                pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as _);
            }
        }
    }
}
