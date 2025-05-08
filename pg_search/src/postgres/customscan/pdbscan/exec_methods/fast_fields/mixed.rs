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

//! Implementation of a mixed field execution state for fast field retrieval.
//!
//! This module provides an optimized execution method that can efficiently handle
//! both multiple string fast fields and numeric fast fields simultaneously,
//! overcoming the limitation where previously ParadeDB could only support
//! either multiple numeric fast fields OR a single string fast field.

use crate::index::fast_fields_helper::{FastFieldType, WhichFastField};
use crate::index::reader::index::{SearchIndexReader, SearchIndexScore, SearchResults};
use crate::postgres::customscan::pdbscan::exec_methods::fast_fields::{
    ff_to_datum, FastFieldExecState,
};
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::is_block_all_visible;
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::postgres::types::TantivyValue;
use crate::query::SearchQueryInput;
use parking_lot::Mutex;
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys;
use pgrx::PgOid;
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};
use tantivy::collector::Collector;
use tantivy::index::SegmentId;
use tantivy::query::Query;
use tantivy::schema::document::OwnedValue;
use tantivy::{DocAddress, Executor, SegmentOrdinal};

/// Execution state for mixed fast field retrieval optimized for both string and numeric fields.
///
/// This execution state is designed to handle two scenarios that previous implementations
/// couldn't handle efficiently:
/// 1. Multiple string fast fields in a single query
/// 2. A mix of string and numeric fast fields in a single query
///
/// Rather than reimplementing all functionality, this struct uses composition to build on
/// the existing FastFieldExecState while adding optimized processing paths for mixed field types.
///
/// # Usage Context
/// This execution method is selected when a query uses multiple fast fields with at least one
/// string fast field. It processes both string and numeric fields directly from the index's
/// fast field data structures, avoiding the need to fetch full documents.
pub struct MixedFastFieldExecState {
    /// Core functionality shared with other fast field execution methods
    inner: FastFieldExecState,

    /// Optimized results storage for both string and numeric fields
    mixed_results: MixedAggResults,

    /// Cached list of string fast fields for quick reference
    string_fields: Vec<String>,

    /// Cached list of numeric fast fields for quick reference
    numeric_fields: Vec<String>,

    /// Statistics tracking the number of rows fetched
    num_rows_fetched: usize,

    /// Statistics tracking the number of visible rows
    num_visible: usize,
}

impl MixedFastFieldExecState {
    /// Creates a new MixedFastFieldExecState from a list of fast fields.
    ///
    /// This constructor analyzes the provided fast fields and categorizes them
    /// into string and numeric types for optimized processing.
    ///
    /// # Arguments
    ///
    /// * `which_fast_fields` - Vector of fast fields that will be processed
    ///
    /// # Returns
    ///
    /// A new MixedFastFieldExecState instance
    pub fn new(which_fast_fields: Vec<WhichFastField>) -> Self {
        // Categorize fields by type for optimized processing
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
    /// Initializes the execution state with the necessary context.
    ///
    /// # Arguments
    ///
    /// * `state` - The current scan state containing query information
    /// * `cstate` - PostgreSQL's custom scan state pointer
    fn init(&mut self, state: &mut PdbScanState, cstate: *mut pg_sys::CustomScanState) {
        // Initialize the inner FastFieldExecState
        self.inner.init(state, cstate);

        // Reset mixed field specific state
        self.mixed_results = MixedAggResults::None;
        self.num_rows_fetched = 0;
        self.num_visible = 0;
    }

    /// Executes the search query and prepares result processing.
    ///
    /// This method handles both parallel and non-parallel execution paths.
    /// For parallel execution, it processes a single segment at a time.
    /// For non-parallel execution, it processes all segments at once.
    ///
    /// # Arguments
    ///
    /// * `state` - The current scan state containing query information
    ///
    /// # Returns
    ///
    /// `true` if there are results to process, `false` otherwise
    fn query(&mut self, state: &mut PdbScanState) -> bool {
        // Handle parallel query execution
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

            // No more segments to query in parallel mode
            self.mixed_results = MixedAggResults::None;
            self.inner.search_results = SearchResults::None;
            false
        } else if self.inner.did_query {
            // Not parallel and already queried
            false
        } else {
            // First time query in non-parallel mode
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

    /// Fetches the next result and prepares it for returning to PostgreSQL.
    ///
    /// This method converts optimized search results into PostgreSQL tuple format,
    /// handling value retrieval for both string and numeric fields.
    ///
    /// # Arguments
    ///
    /// * `state` - The current scan state
    ///
    /// # Returns
    ///
    /// The next execution state containing the result or EOF
    fn internal_next(&mut self, _state: &mut PdbScanState) -> ExecState {
        // Check if we have any results left
        if matches!(self.mixed_results, MixedAggResults::None) {
            return ExecState::Eof;
        }

        unsafe {
            // Process the next result from our optimized path
            match self.mixed_results.next() {
                None => ExecState::Eof,
                Some((scored, doc_address, field_values)) => {
                    let slot = self.inner.slot;
                    let natts = (*(*slot).tts_tupleDescriptor).natts as usize;

                    // Set ctid and table OID
                    crate::postgres::utils::u64_to_item_pointer(scored.ctid, &mut (*slot).tts_tid);
                    (*slot).tts_tableOid = (*self.inner.heaprel).rd_id;

                    // Check visibility of the current block
                    let blockno = item_pointer_get_block_number(&(*slot).tts_tid);
                    let is_visible = if blockno == self.inner.blockvis.0 {
                        // Already checked this block's visibility
                        self.inner.blockvis.1
                    } else {
                        // New block, check visibility
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

                        // Setup slot for returning data
                        (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
                        (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
                        (*slot).tts_nvalid = natts as _;

                        let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
                        let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

                        // Initialize all values to NULL
                        for i in 0..natts {
                            datums[i] = pg_sys::Datum::null();
                            isnull[i] = true;
                        }

                        let fast_fields = &mut self.inner.ffhelper;
                        let which_fast_fields = &self.inner.which_fast_fields;
                        let tupdesc = self.inner.tupdesc.as_ref().unwrap();

                        // Take the string buffer from inner
                        let mut string_buf = self.inner.strbuf.take().unwrap_or_default();

                        // Process each column, converting fast field values to PostgreSQL datums
                        pgrx::warning!("⭐️ [Mixed] First pass: Direct positional mapping");
                        for (i, att) in self.inner.tupdesc.as_ref().unwrap().iter().enumerate() {
                            // Skip if already processed
                            if !isnull[i] {
                                continue;
                            }

                            let which_fast_field = &which_fast_fields[i];

                            // Get attribute info if available
                            let att_info = if i < tupdesc.len() {
                                tupdesc.get(i)
                            } else {
                                None
                            };

                            let att_typid =
                                att_info.map(|att| att.atttypid).unwrap_or(pg_sys::TEXTOID);
                            let att_name = att_info
                                .map(|att| att.name().to_string())
                                .unwrap_or_default();

                            // Try the optimized fast field path first
                            if let WhichFastField::Named(field_name, field_type) = which_fast_field
                            {
                                pgrx::warning!(
                                    "⭐️ [Mixed] Processing position {}: att_name={}, field_name={}",
                                    i,
                                    att_name,
                                    field_name
                                );

                                match field_type {
                                    // String field handling
                                    FastFieldType::String => {
                                        if let Some(Some(term_string)) =
                                            field_values.get_string(field_name)
                                        {
                                            // Use the term directly for this string field if available
                                            if let Some(datum) =
                                                term_to_datum(term_string, att_typid, slot)
                                            {
                                                datums[i] = datum;
                                                isnull[i] = false;
                                                pgrx::warning!(
                                                    "⭐️ [Mixed] Assigned string field: pos={}, field={}, value={}",
                                                    i, field_name, term_string
                                                );
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
                                            if let Ok(Some(datum)) = TantivyValue(num_value.clone())
                                                .try_into_datum(PgOid::from(att_typid))
                                            {
                                                datums[i] = datum;
                                                isnull[i] = false;
                                                pgrx::warning!(
                                                        "⭐️ [Mixed] Assigned numeric field: pos={}, field={}",
                                                        i, field_name
                                                    );
                                                continue;
                                            }
                                        }
                                    }
                                }
                            }

                            // Fallback to standard ff_to_datum if optimized path didn't work
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
                                    pgrx::warning!(
                                        "⭐️ [Mixed] Assigned NULL via fallback: pos={}, field={}",
                                        i,
                                        which_fast_field.name()
                                    );
                                }
                                Some(datum) => {
                                    datums[i] = datum;
                                    isnull[i] = false;
                                    pgrx::warning!(
                                        "⭐️ [Mixed] Assigned via fallback: pos={}, field={}",
                                        i,
                                        which_fast_field.name()
                                    );
                                }
                            }

                            // Extract the string buffer back
                            string_buf = str_opt.unwrap_or_default();
                        }

                        // Store the string buffer back for reuse
                        self.inner.strbuf = Some(string_buf);

                        self.num_rows_fetched += 1;
                        ExecState::Virtual { slot }
                    } else {
                        // Row needs visibility check
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

    /// Resets the execution state to its initial state.
    ///
    /// # Arguments
    ///
    /// * `state` - The current scan state
    fn reset(&mut self, _state: &mut PdbScanState) {
        // Reset inner FastFieldExecState
        self.inner.search_results = SearchResults::None;
        self.inner.did_query = false;
        self.inner.blockvis = (pg_sys::InvalidBlockNumber, false);

        // Reset mixed results state
        self.mixed_results = MixedAggResults::None;

        // Reset statistics
        self.num_rows_fetched = 0;
        self.num_visible = 0;
    }

    /// Increments the count of visible rows.
    ///
    /// Called when a row passes visibility checks.
    fn increment_visible(&mut self) {
        self.num_visible += 1;
    }
}

/// Converts a string term to a PostgreSQL Datum of the appropriate type.
///
/// This helper function is used to directly convert string fast field values
/// to PostgreSQL Datum values without going through intermediate representations.
///
/// # Arguments
///
/// * `term` - The string term to convert
/// * `atttypid` - PostgreSQL type OID for the target column
/// * `slot` - Tuple slot for memory allocation context
///
/// # Returns
///
/// The converted Datum value or None if conversion fails
#[inline]
fn term_to_datum(
    term: &str,
    atttypid: pgrx::pg_sys::Oid,
    slot: *mut pg_sys::TupleTableSlot,
) -> Option<pg_sys::Datum> {
    // Use TantivyValue to convert the string to a Datum
    match TantivyValue::try_from(String::from(term)) {
        Ok(tantivy_value) => {
            // Convert to datum using the common try_into_datum method
            unsafe { tantivy_value.try_into_datum(PgOid::from(atttypid)) }.unwrap_or_default()
        }
        Err(_) => None,
    }
}

/// Container for storing mixed field values from fast fields.
///
/// This struct optimizes storage and retrieval of both string and numeric field values
/// retrieved from the index. Each field value is stored in a type-specific hashmap
/// to avoid unnecessary conversions and enable efficient lookups by field name.
#[derive(Debug, Clone, Default)]
pub struct FieldValues {
    /// String field values, with None representing a field with no value
    string_values: HashMap<String, Option<String>>,

    /// Numeric field values using Tantivy's OwnedValue type for type flexibility
    numeric_values: HashMap<String, OwnedValue>,
}

impl FieldValues {
    /// Creates a new empty FieldValues container.
    fn new() -> Self {
        Self {
            string_values: HashMap::new(),
            numeric_values: HashMap::new(),
        }
    }

    /// Sets a string field value.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name
    /// * `value` - The string value or None if no value
    fn set_string(&mut self, field: String, value: Option<String>) {
        self.string_values.insert(field, value);
    }

    /// Sets a numeric field value.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name
    /// * `value` - The numeric value as an OwnedValue
    fn set_numeric(&mut self, field: String, value: OwnedValue) {
        self.numeric_values.insert(field, value);
    }

    /// Gets a string field value.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name to retrieve
    ///
    /// # Returns
    ///
    /// A reference to the Option<String> value, or None if field doesn't exist
    fn get_string(&self, field: &str) -> Option<&Option<String>> {
        self.string_values.get(field)
    }

    /// Gets a numeric field value.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name to retrieve
    ///
    /// # Returns
    ///
    /// A reference to the OwnedValue, or None if field doesn't exist
    fn get_numeric(&self, field: &str) -> Option<&OwnedValue> {
        self.numeric_values.get(field)
    }
}

// Type aliases for common composite types used in the implementation
/// Iterator for search results from a single batch
type SearchResultsIter = std::vec::IntoIter<(SearchIndexScore, DocAddress)>;
/// Iterator for batched results with field values
type BatchedResultsIter = std::vec::IntoIter<(FieldValues, SearchResultsIter)>;
/// Map of document addresses to field values and scores
type MergedResultsMap = BTreeMap<DocAddress, (FieldValues, SearchIndexScore)>;
/// Group of field values and document addresses/scores
type FieldGroupValue = (FieldValues, Vec<(SearchIndexScore, DocAddress)>);
/// Map of field value representations to groups of documents
type FieldGroups = HashMap<String, FieldGroupValue>;

/// Enum representing different states of mixed aggregation results.
///
/// This enum provides a unified interface for iterating through results
/// from different processing paths (batched, single segment, etc.)
#[derive(Default)]
enum MixedAggResults {
    /// No results available
    #[default]
    None,

    /// Batched results with field values grouped by common patterns
    Batched {
        /// Current batch being processed
        current: (FieldValues, SearchResultsIter),
        /// Iterator for remaining batches
        set: BatchedResultsIter,
    },

    /// Results from a single segment in parallel execution
    SingleSegment(crossbeam::channel::IntoIter<(SearchIndexScore, DocAddress, FieldValues)>),
}

impl Iterator for MixedAggResults {
    type Item = (SearchIndexScore, DocAddress, FieldValues);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MixedAggResults::None => None,
            MixedAggResults::Batched { current, set } => loop {
                // Try to get next item from current batch
                if let Some(next) = current.1.next() {
                    return Some((next.0, next.1, current.0.clone()));
                } else if let Some(next_set) = set.next() {
                    // Move to next batch if current is exhausted
                    *current = next_set;
                } else {
                    // No more batches
                    return None;
                }
            },
            MixedAggResults::SingleSegment(iter) => iter.next(),
        }
    }
}

/// Searcher that aggregates results for mixed field types.
///
/// This searcher is responsible for efficiently retrieving both string and numeric
/// fast field values and organizing them for processing. It handles both single-threaded
/// execution and parallel segment-based execution.
struct MixedAggSearcher<'a>(&'a SearchIndexReader);

impl MixedAggSearcher<'_> {
    /// Executes a search and aggregates mixed field values across all segments.
    ///
    /// This method is used for non-parallel execution and processes all segments
    /// in a single-threaded manner, organizing results by field values to improve
    /// deduplication and subsequent processing efficiency.
    ///
    /// # Arguments
    ///
    /// * `need_scores` - Whether relevancy scores are needed
    /// * `query` - The search query to execute
    /// * `string_fields` - List of string fast fields to retrieve
    /// * `numeric_fields` - List of numeric fast fields to retrieve
    ///
    /// # Returns
    ///
    /// Aggregated results organized by field values
    pub fn mixed_agg(
        &self,
        need_scores: bool,
        query: &SearchQueryInput,
        string_fields: &[String],
        numeric_fields: &[String],
    ) -> MixedAggResults {
        // Create collector that handles both string and numeric fields
        let collector = multi_field_collector::MultiFieldCollector {
            need_scores,
            string_fields: string_fields.to_vec(),
            numeric_fields: numeric_fields.to_vec(),
        };

        // Execute search with the appropriate scoring mode
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

        // Use thread-safe map to combine results from all segments
        let merged: Mutex<MergedResultsMap> = Mutex::new(BTreeMap::new());

        // Process all segment results in parallel
        results.into_par_iter().for_each(
            |(string_columns, string_results, numeric_columns, numeric_values)| {
                // Process string fields
                for field_idx in 0..string_columns.len() {
                    if field_idx >= string_results.len() {
                        continue; // Skip if no results for this field
                    }

                    let (field_name, str_ff) = &string_columns[field_idx];
                    let field_result = &string_results[field_idx];

                    // Process each term ordinate and its documents
                    for (term_ord, docs) in field_result.iter() {
                        // Resolve the term ordinal to an actual string value
                        let term_value = {
                            // Try to get a value from the term ordinal
                            let mut term_str = String::new();

                            // Track if we got a successful resolution
                            let got_term = if str_ff.num_terms() > 0 {
                                // Try to resolve the term ordinal to a string
                                str_ff.ord_to_str(*term_ord, &mut term_str).is_ok()
                            } else {
                                false
                            };

                            if got_term && !term_str.is_empty() {
                                Some(term_str)
                            } else {
                                // Special handling for term_ord = 0 (empty terms)
                                if *term_ord == 0 && !docs.is_empty() {
                                    // Use the dictionary directly to look up the term
                                    let mut bytes_buffer = Vec::new();
                                    if str_ff.dictionary().ord_to_term(0, &mut bytes_buffer).ok()
                                        == Some(true)
                                    {
                                        if let Ok(s) = std::str::from_utf8(&bytes_buffer) {
                                            Some(s.to_string())
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            }
                        };

                        // Add this term to all matching documents
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

                    // Add numeric values to all matching documents
                    for (doc_id, value) in field_values.iter() {
                        let mut guard = merged.lock();
                        if let Some((field_values, _)) = guard.get_mut(doc_id) {
                            field_values.set_numeric(field_name.clone(), value.clone());
                        }
                    }
                }
            },
        );

        // Get the final merged results map
        let processed_docs = merged.into_inner();

        // Group results by field value patterns for more efficient processing
        let mut field_groups: FieldGroups = HashMap::new();

        // Group documents with the same field values
        for (doc_addr, (field_values, score)) in processed_docs {
            // Create a stable string representation of the field values for grouping
            let mut term_keys: Vec<(&String, &Option<String>)> =
                field_values.string_values.iter().collect();
            term_keys.sort_by(|a, b| a.0.cmp(b.0));

            let mut num_keys: Vec<(&String, &OwnedValue)> =
                field_values.numeric_values.iter().collect();
            num_keys.sort_by(|a, b| a.0.cmp(b.0));

            // Create a key that represents all field values
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

            // Group by field values to avoid duplicate value copies
            let entry = field_groups
                .entry(fields_key)
                .or_insert_with(|| (field_values.clone(), Vec::new()));
            entry.1.push((score, doc_addr));
        }

        // Convert the grouped results to iterator format
        let result_vec: Vec<FieldGroupValue> = field_groups.into_values().collect();

        let set = result_vec
            .into_iter()
            .map(|(terms, docs)| (terms, docs.into_iter()))
            .collect::<Vec<_>>()
            .into_iter();

        // Return as batched results for processing
        MixedAggResults::Batched {
            current: (FieldValues::new(), vec![].into_iter()),
            set,
        }
    }

    /// Executes a search and aggregates mixed field values for a single segment.
    ///
    /// This method is used for parallel execution where each worker processes
    /// a single segment. The results are streamed through a channel to avoid
    /// excessive memory usage.
    ///
    /// # Arguments
    ///
    /// * `need_scores` - Whether relevancy scores are needed
    /// * `query` - The search query to execute
    /// * `string_fields` - List of string fast fields to retrieve
    /// * `numeric_fields` - List of numeric fast fields to retrieve
    /// * `segment_id` - ID of the segment to process
    ///
    /// # Returns
    ///
    /// Aggregated results for the specified segment
    pub fn mixed_agg_by_segment(
        &self,
        need_scores: bool,
        query: &SearchQueryInput,
        string_fields: &[String],
        numeric_fields: &[String],
        segment_id: SegmentId,
    ) -> MixedAggResults {
        // Find the segment reader for the specified segment ID
        let (segment_ord, segment_reader) = self
            .0
            .segment_readers()
            .iter()
            .enumerate()
            .find(|(_, reader)| reader.segment_id() == segment_id)
            .unwrap_or_else(|| panic!("segment {segment_id} should exist"));

        // Create collector for both string and numeric fields
        let collector = multi_field_collector::MultiFieldCollector {
            need_scores,
            string_fields: string_fields.to_vec(),
            numeric_fields: numeric_fields.to_vec(),
        };

        // Create a query weight for this segment
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

        // Execute search on this specific segment
        let segment_result = collector
            .collect_segment(
                weight.as_ref(),
                segment_ord as SegmentOrdinal,
                segment_reader,
            )
            .expect("single segment collection should succeed");

        // Create a channel to stream results
        let (sender, receiver) = crossbeam::channel::unbounded();

        // Track documents and their field values
        let mut doc_fields = HashMap::new();

        // Process string fields from this segment
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
                // Resolve the term to a string value
                let term_value = {
                    // Try to get a value from the term ordinal
                    let mut term_str = String::new();

                    // Try to resolve the term ordinal to a string
                    let got_term = if str_ff.num_terms() > 0 {
                        str_ff.ord_to_str(*term_ord, &mut term_str).is_ok()
                    } else {
                        false
                    };

                    if got_term && !term_str.is_empty() {
                        Some(term_str)
                    } else {
                        // Special handling for term_ord = 0 (empty terms)
                        if *term_ord == 0 && !docs.is_empty() {
                            // Use the dictionary directly to look up the term
                            let mut bytes_buffer = Vec::new();
                            if str_ff.dictionary().ord_to_term(0, &mut bytes_buffer).ok()
                                == Some(true)
                            {
                                if let Ok(s) = std::str::from_utf8(&bytes_buffer) {
                                    Some(s.to_string())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                };

                // Add term to each document
                for (score, doc_addr) in docs {
                    let entry = doc_fields
                        .entry(*doc_addr)
                        .or_insert_with(|| (FieldValues::new(), *score));
                    entry.0.set_string(field_name.clone(), term_value.clone());
                }
            }
        }

        // Process numeric fields from this segment
        let numeric_columns = &segment_result.2;
        let numeric_values = &segment_result.3;

        for field_idx in 0..numeric_columns.len() {
            if field_idx >= numeric_values.len() {
                continue;
            }

            let (field_name, _) = &numeric_columns[field_idx];
            let field_values = &numeric_values[field_idx];

            // Add numeric values to all matching documents
            for (doc_id, value) in field_values {
                if let Some(entry) = doc_fields.get_mut(doc_id) {
                    entry.0.set_numeric(field_name.clone(), value.clone());
                }
            }
        }

        // Send all results through the channel
        for (doc_addr, (mut field_values, score)) in doc_fields {
            // Ensure all requested fields have entries (even if null)
            for field in string_fields {
                if !field_values.string_values.contains_key(field) {
                    field_values.set_string(field.clone(), None);
                }
            }

            // Send the result
            sender.send((score, doc_addr, field_values)).ok();
        }

        // Return as single segment results for processing
        MixedAggResults::SingleSegment(receiver.into_iter())
    }
}

/// Module for collecting both string and numeric fast field values during search.
///
/// This implementation extends Tantivy's collector framework to efficiently gather
/// multiple field types simultaneously during a single index traversal.
mod multi_field_collector {
    use crate::index::reader::index::SearchIndexScore;
    use std::collections::{BTreeMap, HashMap};
    use tantivy::collector::{Collector, SegmentCollector};
    use tantivy::columnar::StrColumn;
    use tantivy::schema::document::OwnedValue;

    use crate::index::fast_fields_helper::FFType;
    use tantivy::termdict::TermOrdinal;
    use tantivy::{DocAddress, DocId, Score, SegmentOrdinal, SegmentReader};

    /// Collector that gathers both string and numeric field values from documents.
    ///
    /// This collector is specifically designed to support the mixed fast field
    /// execution state by collecting all needed field values in a single pass
    /// through the index, minimizing document access costs.
    pub struct MultiFieldCollector {
        /// Whether to collect document scores
        pub need_scores: bool,

        /// List of string fast fields to collect
        pub string_fields: Vec<String>,

        /// List of numeric fast fields to collect
        pub numeric_fields: Vec<String>,
    }

    impl Collector for MultiFieldCollector {
        // Each fruit contains the columns, results, and values for both string and numeric fields
        type Fruit = Vec<(
            Vec<(String, StrColumn)>,
            Vec<BTreeMap<TermOrdinal, Vec<(SearchIndexScore, DocAddress)>>>,
            Vec<(String, FFType)>,
            Vec<HashMap<DocAddress, OwnedValue>>,
        )>;
        type Child = MultiFieldSegmentCollector;

        /// Creates a segment collector for a specific segment.
        ///
        /// This method initializes a collector for each segment that can access
        /// both string and numeric fast fields for the specified fields.
        ///
        /// # Arguments
        ///
        /// * `segment_local_id` - Local ID of the segment being processed
        /// * `segment_reader` - Reader for accessing the segment data
        ///
        /// # Returns
        ///
        /// A segment collector configured for this segment
        fn for_segment(
            &self,
            segment_local_id: SegmentOrdinal,
            segment_reader: &SegmentReader,
        ) -> tantivy::Result<Self::Child> {
            let ff = segment_reader.fast_fields();

            // Get string columns for all requested string fields
            let mut string_columns = Vec::new();
            for field_name in &self.string_fields {
                if let Ok(Some(str_column)) = ff.str(field_name) {
                    string_columns.push((field_name.clone(), str_column));
                }
            }

            // Get numeric columns for all requested numeric fields
            let mut numeric_columns = Vec::new();
            let mut numeric_values = Vec::new();

            for field_name in &self.numeric_fields {
                // Try different numeric field types in order
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

            // Initialize result containers for each string field
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

        /// Indicates whether this collector requires document scores.
        fn requires_scoring(&self) -> bool {
            self.need_scores
        }

        /// Merges results from all segment collectors.
        ///
        /// This method simply collects all segment results into a vector,
        /// as the actual merging happens in the MixedAggSearcher implementation.
        fn merge_fruits(
            &self,
            segment_fruits: Vec<<Self::Child as SegmentCollector>::Fruit>,
        ) -> tantivy::Result<Self::Fruit> {
            // Just return the list of segment results
            Ok(segment_fruits)
        }
    }

    /// Segment-level collector for gathering mixed field values.
    ///
    /// This collector processes individual documents within a segment,
    /// collecting both string term ordinals and numeric values for
    /// all requested fields.
    pub struct MultiFieldSegmentCollector {
        /// Segment ordinal for constructing doc addresses
        pub segment_ord: SegmentOrdinal,

        /// String columns to collect from
        pub string_columns: Vec<(String, StrColumn)>,

        /// Results for string fields, organized by term ordinal
        pub string_results: Vec<BTreeMap<TermOrdinal, Vec<(SearchIndexScore, DocAddress)>>>,

        /// Numeric columns to collect from
        pub numeric_columns: Vec<(String, FFType)>,

        /// Results for numeric fields, organized by doc address
        pub numeric_values: Vec<HashMap<DocAddress, OwnedValue>>,

        /// Fast field for retrieving ctid values
        ctid_ff: FFType,
    }

    impl SegmentCollector for MultiFieldSegmentCollector {
        type Fruit = (
            Vec<(String, StrColumn)>,
            Vec<BTreeMap<TermOrdinal, Vec<(SearchIndexScore, DocAddress)>>>,
            Vec<(String, FFType)>,
            Vec<HashMap<DocAddress, OwnedValue>>,
        );

        /// Processes a single document, collecting all field values.
        ///
        /// This method is called for each matching document and collects
        /// both string and numeric field values in a single pass.
        ///
        /// # Arguments
        ///
        /// * `doc` - Document ID within the segment
        /// * `score` - Relevancy score for the document
        fn collect(&mut self, doc: DocId, score: Score) {
            let doc_address = DocAddress::new(self.segment_ord, doc);
            let ctid = self.ctid_ff.as_u64(doc).expect("ctid should be present");
            let scored = SearchIndexScore::new(ctid, score);

            // Collect string fields - group by term ordinal for efficiency
            for (field_idx, (_, str_column)) in self.string_columns.iter().enumerate() {
                let term_ord = str_column.term_ords(doc).next().unwrap_or(0);
                self.string_results[field_idx]
                    .entry(term_ord)
                    .or_default()
                    .push((scored, doc_address));
            }

            // Collect numeric fields - store in document-keyed maps
            for (field_idx, (_, field_type)) in self.numeric_columns.iter().enumerate() {
                // Convert the field value based on its type
                let field_value = match field_type {
                    FFType::I64(col) => {
                        if let Some(val) = col.first(doc) {
                            OwnedValue::I64(val)
                        } else {
                            OwnedValue::Null
                        }
                    }
                    FFType::U64(col) => {
                        if let Some(val) = col.first(doc) {
                            OwnedValue::U64(val)
                        } else {
                            OwnedValue::Null
                        }
                    }
                    FFType::F64(col) => {
                        if let Some(val) = col.first(doc) {
                            OwnedValue::F64(val)
                        } else {
                            OwnedValue::Null
                        }
                    }
                    FFType::Bool(col) => {
                        if let Some(val) = col.first(doc) {
                            OwnedValue::Bool(val)
                        } else {
                            OwnedValue::Null
                        }
                    }
                    _ => OwnedValue::Null,
                };

                // Store the value for this document
                self.numeric_values[field_idx].insert(doc_address, field_value);
            }
        }

        /// Returns all collected field values for this segment.
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
