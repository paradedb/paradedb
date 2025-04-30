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

use crate::index::fast_fields_helper::{FastFieldType, WhichFastField};
use crate::index::reader::index::{SearchIndexReader, SearchIndexScore, SearchResults};
use crate::postgres::customscan::pdbscan::exec_methods::fast_fields::{
    ff_to_datum, FastFieldExecState,
};
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::query::SearchQueryInput;
use parking_lot::Mutex;
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys;
use rayon::prelude::*;
use std::collections::HashMap;
use tantivy::collector::Collector;
use tantivy::index::SegmentId;
use tantivy::query::Query;
use tantivy::{DocAddress, Executor, SegmentOrdinal};

/// MixedFastFieldExecState handles mixed (string and numeric) fast field retrieval
/// Use when you have multiple string fast fields, or a mix of string and numeric fast fields
pub struct MixedFastFieldExecState {
    // Core functionality via composition instead of reimplementation
    inner: FastFieldExecState,

    // For string optimization (similar to StringFastFieldExecState)
    mixed_results: MixedAggResults,

    // Track field types
    string_fields: Vec<String>,
    numeric_fields: Vec<String>,

    // Statistics
    num_rows_fetched: usize,
    num_visible: usize,
}

impl MixedFastFieldExecState {
    pub fn new(which_fast_fields: Vec<WhichFastField>) -> Self {
        // Categorize fields by type
        let mut string_fields = Vec::new();
        let mut numeric_fields = Vec::new();

        for field in &which_fast_fields {
            if let WhichFastField::Named(field_name, field_type) = field {
                match field_type {
                    FastFieldType::String => string_fields.push(field_name.clone()),
                    FastFieldType::Numeric => numeric_fields.push(field_name.clone()),
                }
            }
        }

        Self {
            inner: FastFieldExecState::new(which_fast_fields),
            mixed_results: MixedAggResults::None,
            string_fields,
            numeric_fields,
            num_rows_fetched: 0,
            num_visible: 0,
        }
    }

    // Helper method to determine if we should use string optimization
    fn should_use_string_optimization(&self) -> bool {
        !self.string_fields.is_empty() && self.numeric_fields.is_empty()
    }
}

impl ExecMethod for MixedFastFieldExecState {
    fn init(&mut self, state: &mut PdbScanState, cstate: *mut pg_sys::CustomScanState) {
        // Initialize inner FastFieldExecState manually since it doesn't implement ExecMethod
        unsafe {
            self.inner.heaprel = state.heaprel();
            self.inner.tupdesc = Some(pgrx::PgTupleDesc::from_pg_unchecked(
                (*cstate).ss.ps.ps_ResultTupleDesc,
            ));
            self.inner.slot = pg_sys::MakeTupleTableSlot(
                (*cstate).ss.ps.ps_ResultTupleDesc,
                &pg_sys::TTSOpsVirtual,
            );
            self.inner.ffhelper = crate::index::fast_fields_helper::FFHelper::with_fields(
                state.search_reader.as_ref().unwrap(),
                &self.inner.which_fast_fields,
            );
        }
    }

    fn query(&mut self, state: &mut PdbScanState) -> bool {
        // For string-only fields, use optimized string strategy
        if self.should_use_string_optimization() && !self.string_fields.is_empty() {
            if let Some(parallel_state) = state.parallel_state {
                if let Some(segment_id) = unsafe { checkout_segment(parallel_state) } {
                    let searcher = MixedAggSearcher(state.search_reader.as_ref().unwrap());
                    self.mixed_results = searcher.mixed_agg_by_segment(
                        state.need_scores(),
                        &state.search_query_input,
                        &self.string_fields, // Pass all string fields
                        segment_id,
                    );
                    return true;
                }

                // no more segments to query
                self.mixed_results = MixedAggResults::None;
                false
            } else if self.inner.did_query {
                // not parallel, so we're done
                false
            } else {
                // not parallel, first time query - use string optimization
                let searcher = MixedAggSearcher(state.search_reader.as_ref().unwrap());
                self.mixed_results = searcher.mixed_agg(
                    state.need_scores(),
                    &state.search_query_input,
                    &self.string_fields, // Pass all string fields
                );
                self.inner.did_query = true;
                true
            }
        } else {
            // Handle standard query execution for mixed or numeric fields
            if let Some(parallel_state) = state.parallel_state {
                if let Some(segment_id) = unsafe { checkout_segment(parallel_state) } {
                    self.inner.search_results = state
                        .search_reader
                        .as_ref()
                        .unwrap()
                        .search_segment(state.need_scores(), segment_id, &state.search_query_input);
                    return true;
                }

                // no more segments to query
                self.inner.search_results = SearchResults::None;
                false
            } else if self.inner.did_query {
                // not parallel, so we're done
                false
            } else {
                // not parallel, first time query
                self.inner.search_results = state.search_reader.as_ref().unwrap().search(
                    state.need_scores(),
                    false,
                    &state.search_query_input,
                    state.limit,
                );
                self.inner.did_query = true;
                true
            }
        }
    }

    fn internal_next(&mut self, _state: &mut PdbScanState) -> ExecState {
        // If we're using string optimization
        if self.should_use_string_optimization()
            && !matches!(self.mixed_results, MixedAggResults::None)
        {
            unsafe {
                // Handle results from string optimization path
                match self.mixed_results.next() {
                    None => ExecState::Eof,
                    Some((scored, doc_address, terms)) => {
                        let slot = self.inner.slot;
                        let natts = (*(*slot).tts_tupleDescriptor).natts as usize;

                        crate::postgres::utils::u64_to_item_pointer(
                            scored.ctid,
                            &mut (*slot).tts_tid,
                        );
                        (*slot).tts_tableOid = (*self.inner.heaprel).rd_id;

                        let blockno = item_pointer_get_block_number(&(*slot).tts_tid);
                        let is_visible = if blockno == self.inner.blockvis.0 {
                            // we know the visibility of this block because we just checked it last time
                            self.inner.blockvis.1
                        } else {
                            // new block so check its visibility
                            self.inner.blockvis.0 = blockno;
                            self.inner.blockvis.1 =
                                crate::postgres::customscan::pdbscan::is_block_all_visible(
                                    self.inner.heaprel,
                                    &mut self.inner.vmbuff,
                                    blockno,
                                );
                            self.inner.blockvis.1
                        };

                        if is_visible {
                            self.inner.blockvis = (blockno, true);

                            (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
                            (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
                            (*slot).tts_nvalid = natts as _;

                            let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
                            let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

                            // Initialize all to NULL
                            for i in 0..natts {
                                datums[i] = pg_sys::Datum::null();
                                isnull[i] = true;
                            }

                            let fast_fields = &mut self.inner.ffhelper;
                            let which_fast_fields = &self.inner.which_fast_fields;

                            // Take the string buffer from inner
                            let mut string_buf = self.inner.strbuf.take().unwrap_or_default();

                            for (i, att) in self.inner.tupdesc.as_ref().unwrap().iter().enumerate()
                            {
                                let which_fast_field = &which_fast_fields[i];

                                // If this is a string field and we have a term for it from our optimized results
                                if let WhichFastField::Named(field_name, FastFieldType::String) =
                                    which_fast_field
                                {
                                    if let Some(Some(term_string)) = terms.get(field_name) {
                                        // Use the term directly for this string field if available
                                        if let Some(datum) =
                                            term_to_datum(term_string, att.atttypid, slot)
                                        {
                                            datums[i] = datum;
                                            isnull[i] = false;
                                            continue;
                                        }
                                    }
                                }

                                // Put the string into an Option for ff_to_datum
                                let mut str_opt = Some(string_buf);

                                // For other fields or if we don't have a preloaded term, use standard ff_to_datum
                                match ff_to_datum(
                                    (which_fast_field, i),
                                    att.atttypid,
                                    scored.bm25,
                                    doc_address,
                                    fast_fields,
                                    &mut str_opt,
                                    slot,
                                ) {
                                    None => {
                                        datums[i] = pg_sys::Datum::null();
                                        isnull[i] = true;
                                    }
                                    Some(datum) => {
                                        datums[i] = datum;
                                        isnull[i] = false;
                                    }
                                }

                                // Extract the string back
                                string_buf = str_opt.unwrap_or_default();
                            }

                            // Put the string buffer back
                            self.inner.strbuf = Some(string_buf);

                            self.num_rows_fetched += 1;
                            ExecState::Virtual { slot }
                        } else {
                            ExecState::RequiresVisibilityCheck {
                                ctid: scored.ctid,
                                score: scored.bm25,
                                doc_address,
                            }
                        }
                    }
                }
            }
        } else {
            // Use standard path for mixed or numeric fields
            unsafe {
                match self.inner.search_results.next() {
                    None => ExecState::Eof,
                    Some((scored, doc_address)) => {
                        let slot = self.inner.slot;
                        let natts = (*(*slot).tts_tupleDescriptor).natts as usize;

                        crate::postgres::utils::u64_to_item_pointer(
                            scored.ctid,
                            &mut (*slot).tts_tid,
                        );
                        (*slot).tts_tableOid = (*self.inner.heaprel).rd_id;

                        let blockno = item_pointer_get_block_number(&(*slot).tts_tid);
                        let is_visible = if blockno == self.inner.blockvis.0 {
                            // we know the visibility of this block because we just checked it last time
                            self.inner.blockvis.1
                        } else {
                            // new block so check its visibility
                            self.inner.blockvis.0 = blockno;
                            self.inner.blockvis.1 =
                                crate::postgres::customscan::pdbscan::is_block_all_visible(
                                    self.inner.heaprel,
                                    &mut self.inner.vmbuff,
                                    blockno,
                                );
                            self.inner.blockvis.1
                        };

                        if is_visible {
                            self.inner.blockvis = (blockno, true);

                            (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
                            (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
                            (*slot).tts_nvalid = natts as _;

                            let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
                            let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

                            // Initialize all to NULL
                            for i in 0..natts {
                                datums[i] = pg_sys::Datum::null();
                                isnull[i] = true;
                            }

                            let fast_fields = &mut self.inner.ffhelper;
                            let which_fast_fields = &self.inner.which_fast_fields;

                            for (i, att) in self.inner.tupdesc.as_ref().unwrap().iter().enumerate()
                            {
                                let which_fast_field = &which_fast_fields[i];

                                match ff_to_datum(
                                    (which_fast_field, i),
                                    att.atttypid,
                                    scored.bm25,
                                    doc_address,
                                    fast_fields,
                                    &mut self.inner.strbuf,
                                    slot,
                                ) {
                                    None => {
                                        datums[i] = pg_sys::Datum::null();
                                        isnull[i] = true;
                                    }
                                    Some(datum) => {
                                        datums[i] = datum;
                                        isnull[i] = false;
                                    }
                                }
                            }

                            self.num_rows_fetched += 1;
                            ExecState::Virtual { slot }
                        } else {
                            ExecState::RequiresVisibilityCheck {
                                ctid: scored.ctid,
                                score: scored.bm25,
                                doc_address,
                            }
                        }
                    }
                }
            }
        }
    }

    fn reset(&mut self, _state: &mut PdbScanState) {
        // Reset inner FastFieldExecState manually
        self.inner.search_results = SearchResults::None;
        self.inner.did_query = false;
        self.inner.blockvis = (pg_sys::InvalidBlockNumber, false);

        // Reset string optimization state
        self.mixed_results = MixedAggResults::None;

        // Reset our own statistics
        self.num_rows_fetched = 0;
        self.num_visible = 0;
    }

    fn increment_visible(&mut self) {
        self.num_visible += 1;
    }
}

// Helper function to convert string term to PostgreSQL Datum
#[inline]
fn term_to_datum(
    term: &str,
    atttypid: pgrx::pg_sys::Oid,
    slot: *mut pg_sys::TupleTableSlot,
) -> Option<pg_sys::Datum> {
    use pgrx::pg_sys;
    unsafe {
        let cstr = std::ffi::CString::new(term).ok()?;
        let text_ptr = pg_sys::cstring_to_text_with_len(cstr.as_ptr(), term.len() as _);
        // Convert text_ptr to Datum correctly by converting pointer to integer
        Some(pg_sys::Datum::from(text_ptr as usize))
    }
}

// Types for string optimization
type SearchResultsIter = std::vec::IntoIter<(SearchIndexScore, DocAddress)>;
type FieldTermsMap = HashMap<String, Option<String>>;

// Define MixedAggResults enum
#[derive(Default)]
enum MixedAggResults {
    #[default]
    None,
    Batched {
        current: (FieldTermsMap, SearchResultsIter),
        set: std::vec::IntoIter<(FieldTermsMap, SearchResultsIter)>,
    },
    SingleSegment(crossbeam::channel::IntoIter<(SearchIndexScore, DocAddress, FieldTermsMap)>),
}

impl Iterator for MixedAggResults {
    type Item = (SearchIndexScore, DocAddress, FieldTermsMap);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MixedAggResults::None => None,
            MixedAggResults::Batched { current, set } => loop {
                if let Some(next) = current.1.next() {
                    return Some((next.0, next.1, current.0.clone()));
                } else if let Some(next_set) = set.next() {
                    *current = next_set;
                } else {
                    return None;
                }
            },
            MixedAggResults::SingleSegment(iter) => iter.next(),
        }
    }
}

// Equivalent of StringAggSearcher for mixed fields, but supporting multiple string fields
struct MixedAggSearcher<'a>(&'a SearchIndexReader);

impl MixedAggSearcher<'_> {
    pub fn mixed_agg(
        &self,
        need_scores: bool,
        query: &SearchQueryInput,
        fields: &[String],
    ) -> MixedAggResults {
        // Enhanced version that supports multiple string fields
        let collector = multi_string_term_collector::MultiStringTermCollector {
            need_scores,
            fields: fields.to_vec(),
        };

        let query = self.0.query(query);
        let results = self
            .0
            .searcher()
            .search_with_executor(
                &query,
                &collector,
                &Executor::SingleThread,
                if need_scores {
                    tantivy::query::EnableScoring::Enabled {
                        searcher: self.0.searcher(),
                        statistics_provider: self.0.searcher(),
                    }
                } else {
                    tantivy::query::EnableScoring::Disabled {
                        schema: &self.0.schema().schema,
                        searcher_opt: Some(self.0.searcher()),
                    }
                },
            )
            .expect("failed to search");

        // Process results using a more structured approach that avoids HashMap key issues
        // Create a two-level map: DocAddress -> (FieldTermsMap, Score)
        // This avoids using HashMap as a key
        let merged: Mutex<HashMap<DocAddress, (FieldTermsMap, SearchIndexScore)>> =
            Mutex::new(HashMap::new());

        results
            .into_par_iter()
            .for_each(|(field_columns, segment_results)| {
                // For each field, track which documents have which terms
                for (field_idx, (field_name, str_ff)) in field_columns.iter().enumerate() {
                    if field_idx >= segment_results.len() {
                        continue; // Skip if there are no results for this field
                    }

                    let field_results = &segment_results[field_idx];

                    // Process each term ordinate in the results
                    for (term_ord, docs) in field_results.iter() {
                        // Resolve term for this field (if applicable)
                        let term_value = if *term_ord != 0 && str_ff.num_terms() > 0 {
                            let mut term_str = String::new();
                            if str_ff.ord_to_str(*term_ord, &mut term_str).is_ok() {
                                Some(term_str)
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        // Add this term to all documents
                        for (score, doc_addr) in docs {
                            let mut guard = merged.lock();
                            let entry = guard
                                .entry(*doc_addr)
                                .or_insert_with(|| (HashMap::new(), *score));
                            entry.0.insert(field_name.clone(), term_value.clone());
                        }
                    }
                }
            });

        // Convert to format for MixedAggResults
        let processed_docs = merged.into_inner();

        // Group results by term patterns
        let mut term_groups: HashMap<String, (FieldTermsMap, Vec<(SearchIndexScore, DocAddress)>)> =
            HashMap::new();

        // Use a string representation for grouping since HashMap isn't hashable
        for (doc_addr, (terms, score)) in processed_docs {
            // Create a stable string representation of the terms
            let mut term_keys: Vec<(&String, &Option<String>)> = terms.iter().collect();
            term_keys.sort_by(|a, b| a.0.cmp(b.0));

            let terms_key = term_keys
                .iter()
                .map(|(k, v)| format!("{}:{:?}", k, v))
                .collect::<Vec<_>>()
                .join(",");

            let entry = term_groups
                .entry(terms_key)
                .or_insert_with(|| (terms.clone(), Vec::new()));
            entry.1.push((score, doc_addr));
        }

        // Convert the grouped results to the format needed for the iterator
        let result_vec: Vec<(FieldTermsMap, Vec<(SearchIndexScore, DocAddress)>)> =
            term_groups.into_values().collect();

        let set = result_vec
            .into_iter()
            .map(|(terms, docs)| (terms, docs.into_iter()))
            .collect::<Vec<_>>()
            .into_iter();

        MixedAggResults::Batched {
            current: (HashMap::new(), vec![].into_iter()),
            set,
        }
    }

    pub fn mixed_agg_by_segment(
        &self,
        need_scores: bool,
        query: &SearchQueryInput,
        fields: &[String],
        segment_id: SegmentId,
    ) -> MixedAggResults {
        // Initialize for parallel segment processing
        let (segment_ord, segment_reader) = self
            .0
            .segment_readers()
            .iter()
            .enumerate()
            .find(|(_, reader)| reader.segment_id() == segment_id)
            .unwrap_or_else(|| panic!("segment {segment_id} should exist"));

        let collector = multi_string_term_collector::MultiStringTermCollector {
            need_scores,
            fields: fields.to_vec(),
        };

        let weight = self
            .0
            .query(query)
            .weight(if need_scores {
                tantivy::query::EnableScoring::Enabled {
                    searcher: self.0.searcher(),
                    statistics_provider: self.0.searcher(),
                }
            } else {
                tantivy::query::EnableScoring::Disabled {
                    schema: &self.0.schema().schema,
                    searcher_opt: Some(self.0.searcher()),
                }
            })
            .expect("weight should be constructable");

        let (field_columns, field_results) = collector
            .collect_segment(
                weight.as_ref(),
                segment_ord as SegmentOrdinal,
                segment_reader,
            )
            .expect("single segment collection should succeed");

        // Process each field's results for this segment
        let (sender, receiver) = crossbeam::channel::unbounded();

        // Build a map of doc_address -> terms for all fields
        let mut doc_terms: HashMap<
            DocAddress,
            (HashMap<String, Option<String>>, SearchIndexScore),
        > = HashMap::new();

        for (field_idx, (field_name, str_ff)) in field_columns.iter().enumerate() {
            if field_idx >= field_results.len() {
                continue; // Skip if there are no results for this field
            }

            let field_result = &field_results[field_idx];

            // Process each term ordinate
            for (term_ord, doc_scores) in field_result.iter() {
                // Get term if it exists
                let term_value = if *term_ord != 0 && str_ff.num_terms() > 0 {
                    let mut term = String::new();
                    if str_ff.ord_to_str(*term_ord, &mut term).is_ok() {
                        Some(term)
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Add term for each document
                for (score, doc_addr) in doc_scores {
                    let entry = doc_terms
                        .entry(*doc_addr)
                        .or_insert_with(|| (HashMap::new(), *score));
                    entry.0.insert(field_name.clone(), term_value.clone());
                }
            }
        }

        // Send all results
        for (doc_addr, (mut terms, score)) in doc_terms {
            // Ensure all requested fields have entries
            for field in fields {
                if !terms.contains_key(field) {
                    terms.insert(field.clone(), None);
                }
            }

            // Send the result
            sender.send((score, doc_addr, terms)).ok();
        }

        MixedAggResults::SingleSegment(receiver.into_iter())
    }
}

// This module handles multiple string fields simultaneously
mod multi_string_term_collector {
    use crate::index::reader::index::SearchIndexScore;
    use std::collections::BTreeMap;
    use tantivy::collector::{Collector, SegmentCollector};
    use tantivy::columnar::StrColumn;

    use crate::index::fast_fields_helper::FFType;
    use tantivy::termdict::TermOrdinal;
    use tantivy::{DocAddress, DocId, Score, SegmentOrdinal, SegmentReader};

    pub struct MultiStringTermCollector {
        pub need_scores: bool,
        pub fields: Vec<String>,
    }

    impl Collector for MultiStringTermCollector {
        type Fruit = Vec<(
            Vec<(String, StrColumn)>,
            Vec<BTreeMap<TermOrdinal, Vec<(SearchIndexScore, DocAddress)>>>,
        )>;
        type Child = MultiStringTermSegmentCollector;

        fn for_segment(
            &self,
            segment_local_id: SegmentOrdinal,
            segment_reader: &SegmentReader,
        ) -> tantivy::Result<Self::Child> {
            let ff = segment_reader.fast_fields();

            // Get string columns for all requested fields
            let mut field_columns = Vec::new();
            for field_name in &self.fields {
                if let Ok(Some(str_column)) = ff.str(field_name) {
                    field_columns.push((field_name.clone(), str_column));
                }
                // Skip fields that aren't string fast fields
            }

            // Create collectors for each field we found
            let field_results = field_columns.iter().map(|_| BTreeMap::default()).collect();

            Ok(MultiStringTermSegmentCollector {
                segment_ord: segment_local_id,
                field_columns,
                field_results,
                ctid_ff: FFType::new_ctid(ff),
            })
        }

        fn requires_scoring(&self) -> bool {
            self.need_scores
        }

        fn merge_fruits(
            &self,
            segment_fruits: Vec<<Self::Child as SegmentCollector>::Fruit>,
        ) -> tantivy::Result<Self::Fruit> {
            Ok(segment_fruits)
        }
    }

    pub struct MultiStringTermSegmentCollector {
        pub segment_ord: SegmentOrdinal,
        pub field_columns: Vec<(String, StrColumn)>,
        pub field_results: Vec<BTreeMap<TermOrdinal, Vec<(SearchIndexScore, DocAddress)>>>,
        ctid_ff: FFType,
    }

    impl SegmentCollector for MultiStringTermSegmentCollector {
        type Fruit = (
            Vec<(String, StrColumn)>,
            Vec<BTreeMap<TermOrdinal, Vec<(SearchIndexScore, DocAddress)>>>,
        );

        fn collect(&mut self, doc: DocId, score: Score) {
            let doc_address = DocAddress::new(self.segment_ord, doc);
            let ctid = self.ctid_ff.as_u64(doc).expect("ctid should be present");
            let scored = SearchIndexScore::new(ctid, score);

            // For each field, collect its term ordinate for this document
            for (field_idx, (_, str_column)) in self.field_columns.iter().enumerate() {
                let term_ord = str_column.term_ords(doc).next().unwrap_or(0);
                self.field_results[field_idx]
                    .entry(term_ord)
                    .or_default()
                    .push((scored, doc_address));
            }
        }

        fn harvest(self) -> Self::Fruit {
            (self.field_columns, self.field_results)
        }
    }
}
