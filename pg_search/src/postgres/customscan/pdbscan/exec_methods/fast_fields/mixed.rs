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
use crate::postgres::customscan::pdbscan::is_block_all_visible;
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::query::SearchQueryInput;
use parking_lot::Mutex;
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys;
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};
use tantivy::collector::Collector;
use tantivy::index::SegmentId;
use tantivy::query::Query;
use tantivy::{DocAddress, Executor, SegmentOrdinal};

// Define struct to hold field values of different types - make it public
#[derive(Debug, Clone)]
pub enum FieldValue {
    I64(i64),
    U64(u64),
    F64(f64),
    Bool(bool),
    None,
}

/// MixedFastFieldExecState handles mixed (string and numeric) fast field retrieval
/// Use when you have multiple string fast fields, or a mix of string and numeric fast fields
pub struct MixedFastFieldExecState {
    // Core functionality via composition instead of reimplementation
    inner: FastFieldExecState,

    // For optimized results (now handling both string and numeric fields)
    mixed_results: MixedAggResults,

    // Track field types for organization
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
}

impl ExecMethod for MixedFastFieldExecState {
    fn init(&mut self, state: &mut PdbScanState, cstate: *mut pg_sys::CustomScanState) {
        self.inner.init(state, cstate);
        self.mixed_results = MixedAggResults::None;
        self.num_rows_fetched = 0;
        self.num_visible = 0;
    }

    fn query(&mut self, state: &mut PdbScanState) -> bool {
        // Always use our optimized path regardless of field types
        if let Some(parallel_state) = state.parallel_state {
            if let Some(segment_id) = unsafe { checkout_segment(parallel_state) } {
                let searcher = MixedAggSearcher(state.search_reader.as_ref().unwrap());
                self.mixed_results = searcher.mixed_agg_by_segment(
                    state.need_scores(),
                    &state.search_query_input,
                    &self.string_fields,
                    &self.numeric_fields,
                    segment_id,
                );
                return true;
            }

            // no more segments to query
            self.mixed_results = MixedAggResults::None;
            self.inner.search_results = SearchResults::None;
            false
        } else if self.inner.did_query {
            // not parallel, so we're done
            false
        } else {
            // not parallel, first time query
            let searcher = MixedAggSearcher(state.search_reader.as_ref().unwrap());
            self.mixed_results = searcher.mixed_agg(
                state.need_scores(),
                &state.search_query_input,
                &self.string_fields,
                &self.numeric_fields,
            );
            self.inner.did_query = true;
            true
        }
    }

    fn internal_next(&mut self, _state: &mut PdbScanState) -> ExecState {
        // Always use our optimized path if we have results
        if matches!(self.mixed_results, MixedAggResults::None) {
            // No results from optimized path
            return ExecState::Eof;
        }
        unsafe {
            // Handle results from our optimized path
            match self.mixed_results.next() {
                None => ExecState::Eof,
                Some((scored, doc_address, field_values)) => {
                    let slot = self.inner.slot;
                    let natts = (*(*slot).tts_tupleDescriptor).natts as usize;

                    crate::postgres::utils::u64_to_item_pointer(scored.ctid, &mut (*slot).tts_tid);
                    (*slot).tts_tableOid = (*self.inner.heaprel).rd_id;

                    let blockno = item_pointer_get_block_number(&(*slot).tts_tid);
                    let is_visible = if blockno == self.inner.blockvis.0 {
                        // we know the visibility of this block because we just checked it last time
                        self.inner.blockvis.1
                    } else {
                        // new block so check its visibility
                        self.inner.blockvis.0 = blockno;
                        self.inner.blockvis.1 = is_block_all_visible(
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

                        for (i, att) in self.inner.tupdesc.as_ref().unwrap().iter().enumerate() {
                            let which_fast_field = &which_fast_fields[i];

                            // Process field based on field type - handle both string and numeric
                            if let WhichFastField::Named(field_name, field_type) = which_fast_field
                            {
                                match field_type {
                                    // String field handling
                                    FastFieldType::String => {
                                        if let Some(Some(term_string)) =
                                            field_values.get_string(field_name)
                                        {
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
                                    // Numeric field handling
                                    FastFieldType::Numeric => {
                                        if let Some(num_value) =
                                            field_values.get_numeric(field_name)
                                        {
                                            // Convert the numeric value to the right Datum type
                                            if let Some(datum) =
                                                numeric_to_datum(num_value, att.atttypid)
                                            {
                                                datums[i] = datum;
                                                isnull[i] = false;
                                                continue;
                                            }
                                        }
                                    }
                                }
                            }

                            // If we didn't handle it above, use standard ff_to_datum as fallback
                            let mut str_opt = Some(string_buf);

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

// Helper function to convert numeric value to PostgreSQL Datum
#[inline]
fn numeric_to_datum(value: &FieldValue, atttypid: pgrx::pg_sys::Oid) -> Option<pg_sys::Datum> {
    use pgrx::pg_sys;

    // Based on the atttype, convert the numeric value to the right datum type
    match value {
        FieldValue::I64(i) => {
            let i_val = *i;
            Some(pg_sys::Datum::from(i_val as i32)) // Cast to i32 first for proper Datum conversion
        }
        FieldValue::U64(u) => {
            let u_val = *u;
            Some(pg_sys::Datum::from(u_val as u32)) // Cast to u32 first
        }
        FieldValue::F64(f) => {
            // For floating point, use the f64_to_datum conversion
            unsafe {
                let float_val = *f;
                Some(pg_sys::Float8GetDatum(float_val))
            }
        }
        FieldValue::Bool(b) => {
            let b_val = *b;
            Some(pg_sys::Datum::from(b_val)) // Use From trait
        }
        _ => None, // Can't handle other types
    }
}

// Define a struct to hold field values of different types
#[derive(Debug, Clone, Default)]
pub struct FieldValues {
    string_values: HashMap<String, Option<String>>,
    numeric_values: HashMap<String, FieldValue>,
}

impl FieldValues {
    fn new() -> Self {
        Self {
            string_values: HashMap::new(),
            numeric_values: HashMap::new(),
        }
    }

    fn set_string(&mut self, field: String, value: Option<String>) {
        self.string_values.insert(field, value);
    }

    fn set_numeric(&mut self, field: String, value: FieldValue) {
        self.numeric_values.insert(field, value);
    }

    fn get_string(&self, field: &str) -> Option<&Option<String>> {
        self.string_values.get(field)
    }

    fn get_numeric(&self, field: &str) -> Option<&FieldValue> {
        self.numeric_values.get(field)
    }
}

// Types for string optimization
type SearchResultsIter = std::vec::IntoIter<(SearchIndexScore, DocAddress)>;
type BatchedResultsIter = std::vec::IntoIter<(FieldValues, SearchResultsIter)>;
type MergedResultsMap = BTreeMap<DocAddress, (FieldValues, SearchIndexScore)>;

// Define MixedAggResults enum
#[derive(Default)]
enum MixedAggResults {
    #[default]
    None,
    Batched {
        current: (FieldValues, SearchResultsIter),
        set: BatchedResultsIter,
    },
    SingleSegment(crossbeam::channel::IntoIter<(SearchIndexScore, DocAddress, FieldValues)>),
}

impl Iterator for MixedAggResults {
    type Item = (SearchIndexScore, DocAddress, FieldValues);

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
        string_fields: &[String],
        numeric_fields: &[String],
    ) -> MixedAggResults {
        // Now handles both string and numeric fields
        let collector = multi_field_collector::MultiFieldCollector {
            need_scores,
            string_fields: string_fields.to_vec(),
            numeric_fields: numeric_fields.to_vec(),
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

        let merged: Mutex<MergedResultsMap> = Mutex::new(BTreeMap::new());

        // Process results, which contains a Vec of segment results
        // Process results, which contains a Vec of segment results
        results.into_par_iter().for_each(
            |(string_columns, string_results, numeric_columns, numeric_values)| {
                // Process string fields
                for field_idx in 0..string_columns.len() {
                    if field_idx >= string_results.len() {
                        continue; // Skip if no results for this field
                    }

                    let (field_name, str_ff) = &string_columns[field_idx];
                    let field_result = &string_results[field_idx];

                    // Process each term ordinate
                    for (term_ord, docs) in field_result.iter() {
                        // Resolve the term for this field
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
                                .or_insert_with(|| (FieldValues::new(), *score));
                            entry.0.set_string(field_name.clone(), term_value.clone());
                        }
                    }
                }

                // Process numeric fields
                for field_idx in 0..numeric_columns.len() {
                    if field_idx >= numeric_values.len() {
                        continue; // Skip if no results for this field
                    }

                    let (field_name, _) = &numeric_columns[field_idx];
                    let field_values = &numeric_values[field_idx];

                    // Add values to all documents
                    for (doc_id, value) in field_values.iter() {
                        let mut guard = merged.lock();
                        if let Some((field_values, _)) = guard.get_mut(doc_id) {
                            field_values.set_numeric(field_name.clone(), value.clone());
                        }
                    }
                }
            },
        );

        // Convert to format for MixedAggResults
        let processed_docs = merged.into_inner();

        // Group results by field value patterns
        let mut field_groups: HashMap<String, (FieldValues, Vec<(SearchIndexScore, DocAddress)>)> =
            HashMap::new();

        // Use a string representation for grouping
        for (doc_addr, (field_values, score)) in processed_docs {
            // Create a stable string representation of the field values
            let mut term_keys: Vec<(&String, &Option<String>)> =
                field_values.string_values.iter().collect();
            term_keys.sort_by(|a, b| a.0.cmp(b.0));

            let mut num_keys: Vec<(&String, &FieldValue)> =
                field_values.numeric_values.iter().collect();
            num_keys.sort_by(|a, b| a.0.cmp(b.0));

            let fields_key = format!(
                "S:{}|N:{}",
                term_keys
                    .iter()
                    .map(|(k, v)| format!("{}:{:?}", k, v))
                    .collect::<Vec<_>>()
                    .join(","),
                num_keys
                    .iter()
                    .map(|(k, v)| format!("{}:{:?}", k, v))
                    .collect::<Vec<_>>()
                    .join(",")
            );

            let entry = field_groups
                .entry(fields_key)
                .or_insert_with(|| (field_values.clone(), Vec::new()));
            entry.1.push((score, doc_addr));
        }

        // Convert the grouped results to iterator format
        let result_vec: Vec<(FieldValues, Vec<(SearchIndexScore, DocAddress)>)> =
            field_groups.into_values().collect();

        let set = result_vec
            .into_iter()
            .map(|(terms, docs)| (terms, docs.into_iter()))
            .collect::<Vec<_>>()
            .into_iter();

        MixedAggResults::Batched {
            current: (FieldValues::new(), vec![].into_iter()),
            set,
        }
    }

    pub fn mixed_agg_by_segment(
        &self,
        need_scores: bool,
        query: &SearchQueryInput,
        string_fields: &[String],
        numeric_fields: &[String],
        segment_id: SegmentId,
    ) -> MixedAggResults {
        // Initialize for parallel segment processing with both field types
        let (segment_ord, segment_reader) = self
            .0
            .segment_readers()
            .iter()
            .enumerate()
            .find(|(_, reader)| reader.segment_id() == segment_id)
            .unwrap_or_else(|| panic!("segment {segment_id} should exist"));

        let collector = multi_field_collector::MultiFieldCollector {
            need_scores,
            string_fields: string_fields.to_vec(),
            numeric_fields: numeric_fields.to_vec(),
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

        let segment_result = collector
            .collect_segment(
                weight.as_ref(),
                segment_ord as SegmentOrdinal,
                segment_reader,
            )
            .expect("single segment collection should succeed");

        // Process segment results
        let (sender, receiver) = crossbeam::channel::unbounded();

        // Track documents and their field values
        let mut doc_fields = HashMap::new();

        // Process string fields
        let string_columns = &segment_result.0;
        let string_results = &segment_result.1;

        for field_idx in 0..string_columns.len() {
            if field_idx >= string_results.len() {
                continue;
            }

            let (field_name, str_ff) = &string_columns[field_idx];
            let field_result = &string_results[field_idx];

            // Process each term ordinate
            for (term_ord, docs) in field_result {
                // Get the term
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
                for (score, doc_addr) in docs {
                    let entry = doc_fields
                        .entry(*doc_addr)
                        .or_insert_with(|| (FieldValues::new(), *score));
                    entry.0.set_string(field_name.clone(), term_value.clone());
                }
            }
        }

        // Process numeric fields
        let numeric_columns = &segment_result.2;
        let numeric_values = &segment_result.3;

        for field_idx in 0..numeric_columns.len() {
            if field_idx >= numeric_values.len() {
                continue;
            }

            let (field_name, _) = &numeric_columns[field_idx];
            let field_values = &numeric_values[field_idx];

            // Add values to all documents
            for (doc_id, value) in field_values {
                if let Some(entry) = doc_fields.get_mut(doc_id) {
                    entry.0.set_numeric(field_name.clone(), value.clone());
                }
            }
        }

        // Send all results
        for (doc_addr, (mut field_values, score)) in doc_fields {
            // Ensure all requested fields have entries
            for field in string_fields {
                if !field_values.string_values.contains_key(field) {
                    field_values.set_string(field.clone(), None);
                }
            }

            // Send the result
            sender.send((score, doc_addr, field_values)).ok();
        }

        MixedAggResults::SingleSegment(receiver.into_iter())
    }
}

// Replace the previous term collector with one that handles both string and numeric fields
mod multi_field_collector {
    use crate::index::reader::index::SearchIndexScore;
    use std::collections::{BTreeMap, HashMap};
    use tantivy::collector::{Collector, SegmentCollector};
    use tantivy::columnar::StrColumn;

    use crate::index::fast_fields_helper::FFType;
    use crate::postgres::customscan::pdbscan::exec_methods::fast_fields::mixed::FieldValue;
    use tantivy::termdict::TermOrdinal;
    use tantivy::{DocAddress, DocId, Score, SegmentOrdinal, SegmentReader};

    pub struct MultiFieldCollector {
        pub need_scores: bool,
        pub string_fields: Vec<String>,
        pub numeric_fields: Vec<String>,
    }

    impl Collector for MultiFieldCollector {
        type Fruit = Vec<(
            Vec<(String, StrColumn)>,
            Vec<BTreeMap<TermOrdinal, Vec<(SearchIndexScore, DocAddress)>>>,
            Vec<(String, FFType)>,
            Vec<HashMap<DocAddress, FieldValue>>,
        )>;
        type Child = MultiFieldSegmentCollector;

        fn for_segment(
            &self,
            segment_local_id: SegmentOrdinal,
            segment_reader: &SegmentReader,
        ) -> tantivy::Result<Self::Child> {
            let ff = segment_reader.fast_fields();

            // Get string columns
            let mut string_columns = Vec::new();
            for field_name in &self.string_fields {
                if let Ok(Some(str_column)) = ff.str(field_name) {
                    string_columns.push((field_name.clone(), str_column));
                }
            }

            // Get numeric columns
            let mut numeric_columns = Vec::new();
            let mut numeric_values = Vec::new();

            for field_name in &self.numeric_fields {
                // Try to get numeric field based on different types
                let ff_type = if let Ok(i64_col) = ff.i64(field_name) {
                    Some(FFType::I64(i64_col))
                } else if let Ok(u64_col) = ff.u64(field_name) {
                    Some(FFType::U64(u64_col))
                } else if let Ok(f64_col) = ff.f64(field_name) {
                    Some(FFType::F64(f64_col))
                } else if let Ok(bool_col) = ff.bool(field_name) {
                    Some(FFType::Bool(bool_col))
                } else {
                    None
                };

                if let Some(field_type) = ff_type {
                    numeric_columns.push((field_name.clone(), field_type));
                    numeric_values.push(HashMap::new());
                }
            }

            // Create collectors for results
            let string_results = string_columns.iter().map(|_| BTreeMap::default()).collect();

            Ok(MultiFieldSegmentCollector {
                segment_ord: segment_local_id,
                string_columns,
                string_results,
                numeric_columns,
                numeric_values,
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
            // Return the list of fruits directly
            Ok(segment_fruits)
        }
    }

    pub struct MultiFieldSegmentCollector {
        pub segment_ord: SegmentOrdinal,
        pub string_columns: Vec<(String, StrColumn)>,
        pub string_results: Vec<BTreeMap<TermOrdinal, Vec<(SearchIndexScore, DocAddress)>>>,
        pub numeric_columns: Vec<(String, FFType)>,
        pub numeric_values: Vec<HashMap<DocAddress, FieldValue>>,
        ctid_ff: FFType,
    }

    impl SegmentCollector for MultiFieldSegmentCollector {
        type Fruit = (
            Vec<(String, StrColumn)>,
            Vec<BTreeMap<TermOrdinal, Vec<(SearchIndexScore, DocAddress)>>>,
            Vec<(String, FFType)>,
            Vec<HashMap<DocAddress, FieldValue>>,
        );

        fn collect(&mut self, doc: DocId, score: Score) {
            let doc_address = DocAddress::new(self.segment_ord, doc);
            let ctid = self.ctid_ff.as_u64(doc).expect("ctid should be present");
            let scored = SearchIndexScore::new(ctid, score);

            // Collect string fields
            for (field_idx, (_, str_column)) in self.string_columns.iter().enumerate() {
                let term_ord = str_column.term_ords(doc).next().unwrap_or(0);
                self.string_results[field_idx]
                    .entry(term_ord)
                    .or_default()
                    .push((scored, doc_address));
            }

            // Collect numeric fields
            for (field_idx, (_, field_type)) in self.numeric_columns.iter().enumerate() {
                let field_value = match field_type {
                    FFType::I64(col) => {
                        if let Some(val) = col.first(doc) {
                            FieldValue::I64(val)
                        } else {
                            FieldValue::None
                        }
                    }
                    FFType::U64(col) => {
                        if let Some(val) = col.first(doc) {
                            FieldValue::U64(val)
                        } else {
                            FieldValue::None
                        }
                    }
                    FFType::F64(col) => {
                        if let Some(val) = col.first(doc) {
                            FieldValue::F64(val)
                        } else {
                            FieldValue::None
                        }
                    }
                    FFType::Bool(col) => {
                        if let Some(val) = col.first(doc) {
                            FieldValue::Bool(val)
                        } else {
                            FieldValue::None
                        }
                    }
                    _ => FieldValue::None,
                };

                // Store the value for this document
                self.numeric_values[field_idx].insert(doc_address, field_value);
            }
        }

        fn harvest(self) -> Self::Fruit {
            (
                self.string_columns,
                self.string_results,
                self.numeric_columns,
                self.numeric_values,
            )
        }
    }
}
