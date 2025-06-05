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

use crate::api::index::FieldName;
use crate::api::HashMap;
use crate::api::Varno;
use crate::index::reader::index::{SearchIndexReader, SearchResults};
use crate::postgres::customscan::builders::custom_path::{ExecMethodType, SortDirection};
use crate::postgres::customscan::pdbscan::exec_methods::ExecMethod;
use crate::postgres::customscan::pdbscan::projections::snippet::SnippetType;
use crate::postgres::customscan::pdbscan::qual_inspect::Qual;
use crate::postgres::customscan::CustomScanState;
use crate::postgres::options::SearchIndexOptions;
use crate::postgres::utils::u64_to_item_pointer;
use crate::postgres::visibility_checker::VisibilityChecker;
use crate::postgres::ParallelScanState;
use crate::query::{AsHumanReadable, SearchQueryInput};
use pgrx::heap_tuple::PgHeapTuple;
use pgrx::{name_data_to_str, pg_sys, PgRelation, PgTupleDesc};
use std::cell::UnsafeCell;
use tantivy::snippet::SnippetGenerator;
use tantivy::SegmentReader;

#[derive(Default)]
pub struct PdbScanState {
    pub parallel_state: Option<*mut ParallelScanState>,

    // Note: the range table index at execution time might be different from the one at planning time,
    // so we need to use the one at execution time when creating the custom scan state.
    // But, we also keep the planning RTI for the case when we need to use it for the `var_attname_lookup`
    // because the `var_attname_lookup` is created based on the planning RTI.
    // See https://www.postgresql.org/docs/current/custom-scan-plan.html
    pub planning_rti: pg_sys::Index,
    pub execution_rti: pg_sys::Index,

    base_search_query_input: SearchQueryInput,
    search_query_input: SearchQueryInput,
    pub search_reader: Option<SearchIndexReader>,

    pub search_results: SearchResults,
    pub targetlist_len: usize,

    pub limit: Option<usize>,
    pub sort_field: Option<FieldName>,
    pub sort_direction: Option<SortDirection>,

    pub retry_count: usize,
    pub heap_tuple_check_count: usize,
    pub virtual_tuple_count: usize,
    pub invisible_tuple_count: usize,

    pub heaprelid: pg_sys::Oid,
    pub heaprel: Option<pg_sys::Relation>,
    pub indexrel: Option<pg_sys::Relation>,
    pub indexrelid: pg_sys::Oid,
    pub lockmode: pg_sys::LOCKMODE,

    pub heaprel_namespace: String,
    pub heaprel_relname: String,

    pub visibility_checker: Option<VisibilityChecker>,
    pub segment_count: usize,
    pub quals: Option<Qual>,

    pub need_scores: bool,
    pub const_score_node: Option<*mut pg_sys::Const>,
    pub score_funcoid: pg_sys::Oid,

    pub const_snippet_nodes: HashMap<SnippetType, Vec<*mut pg_sys::Const>>,

    pub snippet_funcoid: pg_sys::Oid,
    pub snippet_positions_funcoid: pg_sys::Oid,

    pub snippet_generators:
        HashMap<SnippetType, Option<(tantivy::schema::Field, SnippetGenerator)>>,

    pub var_attname_lookup: HashMap<(Varno, pg_sys::AttrNumber), FieldName>,
    pub placeholder_targetlist: Option<*mut pg_sys::List>,

    pub exec_method_type: ExecMethodType,
    exec_method: UnsafeCell<Box<dyn ExecMethod>>,
    exec_method_name: String,
}

impl CustomScanState for PdbScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        unsafe {
            // SAFETY: inner_scan_state is always initialized and call to `init()` could never move `self`
            (*self.exec_method.get()).init(self, cstate)
        }
    }
}

impl PdbScanState {
    pub fn set_base_search_query_input(&mut self, input: SearchQueryInput) {
        self.base_search_query_input = input;
    }

    pub fn prepare_query_for_execution(
        &mut self,
        planstate: *mut pg_sys::PlanState,
        expr_context: *mut pg_sys::ExprContext,
    ) {
        self.search_query_input = self.base_search_query_input.clone();
        if self.search_query_input.has_postgres_expressions() {
            self.search_query_input.init_postgres_expressions(planstate);
            self.search_query_input
                .solve_postgres_expressions(expr_context);
        }
    }

    pub fn search_query_input(&self) -> &SearchQueryInput {
        if matches!(self.search_query_input, SearchQueryInput::Uninitialized) {
            panic!("search_query_input should be initialized");
        }
        &self.search_query_input
    }

    #[inline(always)]
    pub fn assign_exec_method<T: ExecMethod + 'static>(
        &mut self,
        method: T,
        updated_exec_method_type: Option<ExecMethodType>,
    ) {
        self.exec_method = UnsafeCell::new(Box::new(method));
        self.exec_method_name = std::any::type_name::<T>().to_string();
        if let Some(exec_method_type) = updated_exec_method_type {
            self.exec_method_type = exec_method_type;
        }
    }

    #[inline(always)]
    pub fn exec_method<'a>(&self) -> &'a dyn ExecMethod {
        let ptr = self.exec_method.get();
        assert!(!ptr.is_null());
        unsafe { ptr.as_ref().unwrap_unchecked().as_ref() }
    }

    #[inline(always)]
    pub fn exec_method_mut<'a>(&mut self) -> &'a mut Box<dyn ExecMethod> {
        let ptr = self.exec_method.get();
        assert!(!ptr.is_null());
        unsafe { ptr.as_mut().unwrap_unchecked() }
    }

    pub fn exec_method_name(&self) -> &str {
        &self.exec_method_name
    }

    pub fn query_to_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(&self.base_search_query_input)
    }

    pub fn parallel_serialization_data(&self) -> (&[SegmentReader], Vec<u8>) {
        let serialized_query = serde_json::to_vec(self.search_query_input())
            .expect("should be able to serialize query");

        let segment_readers = self
            .search_reader
            .as_ref()
            .expect("search reader must be initialized to build parallel serialization data")
            .segment_readers();

        (segment_readers, serialized_query)
    }

    pub fn human_readable_query_string(&self) -> String {
        self.base_search_query_input.as_human_readable()
    }

    pub fn has_postgres_expressions(&mut self) -> bool {
        self.base_search_query_input.has_postgres_expressions()
    }

    #[inline(always)]
    pub fn need_scores(&self) -> bool {
        self.need_scores
            || self.base_search_query_input.need_scores()
            || self
                .quals
                .as_ref()
                .map(|quals| quals.contains_score_exprs())
                .unwrap_or_default()
    }

    #[inline(always)]
    pub fn determine_key_field(&self) -> FieldName {
        unsafe {
            let indexrel = PgRelation::with_lock(self.indexrelid, pg_sys::AccessShareLock as _);
            let ops = SearchIndexOptions::from_relation(&indexrel);
            ops.key_field_name()
        }
    }

    #[inline(always)]
    pub fn need_snippets(&self) -> bool {
        !self.snippet_generators.is_empty()
    }

    #[track_caller]
    #[inline(always)]
    pub fn heaprel(&self) -> pg_sys::Relation {
        self.heaprel.unwrap()
    }

    #[inline(always)]
    pub fn indexrel(&self) -> pg_sys::Relation {
        self.indexrel.unwrap()
    }

    #[inline(always)]
    pub fn heaprel_namespace(&self) -> &str {
        &self.heaprel_namespace
    }

    #[inline(always)]
    pub fn heaprelname(&self) -> &str {
        &self.heaprel_relname
    }

    #[inline(always)]
    pub fn indexrelname(&self) -> &str {
        unsafe { name_data_to_str(&(*(*self.indexrel()).rd_rel).relname) }
    }

    #[inline(always)]
    pub fn heaptupdesc(&self) -> pg_sys::TupleDesc {
        unsafe { (*self.heaprel()).rd_att }
    }

    #[inline(always)]
    pub fn visibility_checker(&mut self) -> &mut VisibilityChecker {
        self.visibility_checker.as_mut().unwrap()
    }

    pub fn make_snippet(&self, ctid: u64, snippet_type: &SnippetType) -> Option<String> {
        let text = unsafe { self.doc_from_heap(ctid, snippet_type.field())? };
        let (field, generator) = self.snippet_generators.get(snippet_type)?.as_ref()?;
        let mut snippet = generator.snippet(&text);
        if let SnippetType::Text(_, _, config) = snippet_type {
            snippet.set_snippet_prefix_postfix(&config.start_tag, &config.end_tag);
        }

        let html = snippet.to_html();
        if html.trim().is_empty() {
            None
        } else {
            Some(html)
        }
    }

    pub fn get_snippet_positions(
        &self,
        ctid: u64,
        snippet_type: &SnippetType,
    ) -> Option<Vec<Vec<i32>>> {
        let text = unsafe { self.doc_from_heap(ctid, snippet_type.field())? };
        let (field, generator) = self.snippet_generators.get(snippet_type)?.as_ref()?;
        let snippet = generator.snippet(&text);
        let highlighted = snippet.highlighted();

        if highlighted.is_empty() {
            None
        } else {
            Some(
                snippet
                    .highlighted()
                    .iter()
                    .map(|span| vec![span.start as i32, span.end as i32])
                    .collect(),
            )
        }
    }

    pub fn is_sorted(&self) -> bool {
        matches!(
            self.sort_direction,
            Some(SortDirection::Asc | SortDirection::Desc)
        )
    }

    pub fn reset(&mut self) {
        if let Some(parallel_state) = self.parallel_state {
            unsafe {
                let worker_number = pg_sys::ParallelWorkerNumber;
                if worker_number == -1 {
                    let _mutex = (*parallel_state).acquire_mutex();
                    ParallelScanState::reset(&mut *parallel_state);
                }
            }
        }
        self.search_results = SearchResults::None;
        self.retry_count = 0;
        self.heap_tuple_check_count = 0;
        self.virtual_tuple_count = 0;
        self.invisible_tuple_count = 0;
        self.exec_method_mut().reset(self);
    }

    /// Given a ctid and field name, get the corresponding value from the heap
    ///
    /// This function supports text and text[] fields
    unsafe fn doc_from_heap(&self, ctid: u64, field: &FieldName) -> Option<String> {
        let heaprel = self
            .heaprel
            .expect("make_snippet: heaprel should be initialized");
        let mut ipd = pg_sys::ItemPointerData::default();
        u64_to_item_pointer(ctid, &mut ipd);

        let mut htup = pg_sys::HeapTupleData {
            t_self: ipd,
            ..Default::default()
        };
        let mut buffer: pg_sys::Buffer = pg_sys::InvalidBuffer as i32;

        #[cfg(feature = "pg14")]
        {
            if !pg_sys::heap_fetch(heaprel, pg_sys::GetActiveSnapshot(), &mut htup, &mut buffer) {
                return None;
            }
        }

        #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
        {
            if !pg_sys::heap_fetch(
                heaprel,
                pg_sys::GetActiveSnapshot(),
                &mut htup,
                &mut buffer,
                false,
            ) {
                return None;
            }
        }

        pg_sys::ReleaseBuffer(buffer);

        let tuple_desc = PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);
        let heap_tuple = PgHeapTuple::from_heap_tuple(tuple_desc.clone(), &mut htup);
        let (index, attribute) = heap_tuple.get_attribute_by_name(&field.root()).unwrap();

        if pg_sys::type_is_array(attribute.type_oid().value()) {
            // varchar[] and text[] are flattened into a single string
            // to emulate Tantivy's default behavior for highlighting text arrays
            Some(
                pgrx::htup::heap_getattr::<Vec<Option<String>>, _>(
                    &pgrx::pgbox::PgBox::from_pg(&mut htup),
                    index,
                    &tuple_desc,
                )
                .unwrap_or_default()
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" "),
            )
        } else {
            heap_tuple
                .get_by_name(&field.root())
                .unwrap_or_else(|_| panic!("{} should exist in the heap tuple", field))
        }
    }
}
