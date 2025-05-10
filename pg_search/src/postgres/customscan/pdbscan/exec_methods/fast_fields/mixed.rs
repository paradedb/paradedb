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
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys;
use pgrx::PgOid;
use rayon::prelude::*;
use smallvec::SmallVec;
use std::cell::RefCell;
use std::collections::HashMap;
use tantivy::collector::Collector;
use tantivy::index::SegmentId;
use tantivy::query::Query;
use tantivy::schema::document::OwnedValue;
use tantivy::termdict::TermOrdinal;
use tantivy::{DocAddress, Executor, SegmentOrdinal};

// Thread-local string buffers for reuse to reduce memory allocations
thread_local! {
    // Pool of reusable string buffers for term resolution
    static STRING_BUFFER_POOL: RefCell<Vec<String>> = RefCell::new(Vec::with_capacity(128));

    // Reusable scratch buffer for temporary operations
    static SCRATCH_BUFFER: RefCell<String> = RefCell::new(String::with_capacity(1024));

    // Thread-local term cache to avoid repeated dictionary lookups
    static TERM_CACHE: RefCell<TermCache> = RefCell::new(TermCache::new(1024));
}

/// Helper function to get a string buffer from the thread-local pool
fn get_string_buffer() -> String {
    STRING_BUFFER_POOL.with(|pool| {
        pool.borrow_mut()
            .pop()
            .unwrap_or_else(|| String::with_capacity(256))
    })
}

/// Helper function to return a string buffer to the thread-local pool
fn recycle_string_buffer(mut buffer: String) {
    buffer.clear();
    STRING_BUFFER_POOL.with(|pool| {
        // Limit pool size to prevent memory growth
        if pool.borrow().len() < 32 {
            pool.borrow_mut().push(buffer);
        }
    })
}

/// A cache for term ordinal to string conversions to avoid repeated dictionary lookups
struct TermCache {
    capacity: usize,
    entries: HashMap<(TermOrdinal, u64), String>,
    hits: usize,
    misses: usize,
}

impl TermCache {
    /// Creates a new term cache with the specified capacity
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::with_capacity(capacity),
            hits: 0,
            misses: 0,
        }
    }

    /// Looks up a term in the cache, returning None if not found
    fn get(&mut self, term_ord: TermOrdinal, dictionary_id: u64) -> Option<&String> {
        if let Some(value) = self.entries.get(&(term_ord, dictionary_id)) {
            self.hits += 1;
            return Some(value);
        }
        self.misses += 1;
        None
    }

    /// Inserts a term into the cache, possibly evicting old entries
    fn insert(&mut self, term_ord: TermOrdinal, dictionary_id: u64, value: String) {
        // Simple eviction strategy - if cache is full, clear it
        if self.entries.len() >= self.capacity {
            self.entries.clear();
        }

        self.entries.insert((term_ord, dictionary_id), value);
    }

    /// Gets a term from the cache or resolves it from the dictionary
    fn get_or_resolve(
        &mut self,
        term_ord: TermOrdinal,
        dictionary: &tantivy::columnar::StrColumn,
        dictionary_id: u64,
    ) -> Option<String> {
        // Try to get from cache first
        if let Some(term) = self.get(term_ord, dictionary_id) {
            return Some(term.clone());
        }

        // If not found, resolve from dictionary
        let mut term_buf = get_string_buffer();

        // Since we don't have ord_to_str directly, we need to use the columnar API
        let result = if dictionary.num_terms() > 0 {
            if dictionary.ord_to_str(term_ord, &mut term_buf).is_ok() && !term_buf.is_empty() {
                let term = term_buf.clone();
                // Cache the resolved term
                self.insert(term_ord, dictionary_id, term.clone());
                Some(term)
            } else {
                // Handle special case for term_ord = 0 (empty or null terms)
                if term_ord == 0 {
                    let mut bytes_buffer = Vec::new();
                    if dictionary
                        .dictionary()
                        .ord_to_term(0, &mut bytes_buffer)
                        .ok()
                        == Some(true)
                    {
                        if let Ok(s) = std::str::from_utf8(&bytes_buffer) {
                            let term = s.to_string();
                            self.insert(term_ord, dictionary_id, term.clone());
                            Some(term)
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
        } else {
            None
        };

        recycle_string_buffer(term_buf);
        result
    }

    /// Gets statistics about cache performance
    #[allow(dead_code)]
    pub fn stats(&self) -> (usize, usize, f32) {
        let total = self.hits + self.misses;
        let hit_rate = if total > 0 {
            self.hits as f32 / total as f32
        } else {
            0.0
        };
        (self.hits, self.misses, hit_rate)
    }
}

/// Helper function to get or resolve a term from the dictionary using the thread-local cache
fn resolve_term_with_cache(
    term_ord: TermOrdinal,
    dictionary: &tantivy::columnar::StrColumn,
    dictionary_id: u64,
) -> Option<String> {
    TERM_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .get_or_resolve(term_ord, dictionary, dictionary_id)
    })
}

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

                        // Take the string buffer from inner or create a new one
                        let mut string_buf =
                            self.inner.strbuf.take().unwrap_or_else(get_string_buffer);

                        // Process each column, converting fast field values to PostgreSQL datums
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
                                }
                                Some(datum) => {
                                    datums[i] = datum;
                                    isnull[i] = false;
                                }
                            }

                            // Extract the string buffer back
                            string_buf = str_opt.unwrap_or_else(get_string_buffer);
                        }

                        // Store the string buffer back for reuse
                        recycle_string_buffer(string_buf);
                        self.inner.strbuf = Some(get_string_buffer());

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
    unsafe {
        match atttypid {
            // Fast path for common text types to avoid TantivyValue conversion
            pg_sys::TEXTOID | pg_sys::VARCHAROID | pg_sys::NAMEOID => {
                // Convert directly to PG text datum using PGRX's string conversion
                let text_ptr = pgrx::pg_sys::cstring_to_text_with_len(
                    term.as_ptr() as *const std::os::raw::c_char,
                    term.len() as i32,
                );
                Some(pg_sys::Datum::from(text_ptr as usize))
            }
            // Other types need to go through TantivyValue
            _ => match TantivyValue::try_from(term.to_string()) {
                Ok(tantivy_value) => tantivy_value
                    .try_into_datum(PgOid::from(atttypid))
                    .unwrap_or_default(),
                Err(_) => None,
            },
        }
    }
}

/// Container for storing mixed field values from fast fields.
///
/// This struct optimizes storage and retrieval of both string and numeric field values
/// retrieved from the index. It uses SmallVec for efficient storage of small collections,
/// avoiding heap allocations for the common case of queries with few fields.
#[derive(Debug, Clone, Default)]
pub struct FieldValues {
    /// String field values, with None representing a field with no value
    /// Using SmallVec for stack allocation when there are few fields (common case)
    string_values: SmallVec<[(String, Option<String>); 8]>,

    /// Numeric field values using Tantivy's OwnedValue type for type flexibility
    /// Using SmallVec for stack allocation when there are few fields (common case)
    numeric_values: SmallVec<[(String, OwnedValue); 8]>,
}

impl FieldValues {
    /// Creates a new FieldValues container with pre-allocated capacity.
    fn with_capacity(string_capacity: usize, numeric_capacity: usize) -> Self {
        Self {
            string_values: SmallVec::with_capacity(string_capacity),
            numeric_values: SmallVec::with_capacity(numeric_capacity),
        }
    }

    /// Sets a string field value.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name
    /// * `value` - The string value or None if no value
    fn set_string(&mut self, field: &String, value: Option<String>) {
        // Check if field already exists
        for (i, (existing_field, _)) in self.string_values.iter().enumerate() {
            if existing_field == field {
                // Update existing entry
                self.string_values[i].1 = value;
                return;
            }
        }
        // Add new entry
        self.string_values.push((field.clone(), value));
    }

    /// Sets a numeric field value.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name
    /// * `value` - The numeric value as an OwnedValue
    fn set_numeric(&mut self, field: &String, value: OwnedValue) {
        // Check if field already exists
        for (i, (existing_field, _)) in self.numeric_values.iter().enumerate() {
            if existing_field == field {
                // Update existing entry
                self.numeric_values[i].1 = value;
                return;
            }
        }
        // Add new entry
        self.numeric_values.push((field.clone(), value));
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
        self.string_values
            .iter()
            .find(|(f, _)| f == field)
            .map(|(_, v)| v)
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
        self.numeric_values
            .iter()
            .find(|(f, _)| f == field)
            .map(|(_, v)| v)
    }

    /// Checks if a string field exists
    fn contains_string_field(&self, field: &str) -> bool {
        self.string_values.iter().any(|(f, _)| f == field)
    }
}

/// Direct document result structure for simplified result processing
struct DocResult {
    doc_address: DocAddress,
    score: SearchIndexScore,
    field_values: FieldValues,
}

/// Simplify the MixedAggResults enum to reduce abstraction layers
#[derive(Default)]
#[allow(clippy::large_enum_variant)]
enum MixedAggResults {
    /// No results available
    #[default]
    None,

    /// Direct document results with minimal transformation
    Direct {
        /// Current results being processed
        results: Vec<DocResult>,
        /// Current position in results
        position: usize,
    },

    /// Single segment results for parallel execution
    SingleSegment(crossbeam::channel::IntoIter<(SearchIndexScore, DocAddress, FieldValues)>),
}

// Update the implementation of Iterator for MixedAggResults to use the new Direct variant
impl Iterator for MixedAggResults {
    type Item = (SearchIndexScore, DocAddress, FieldValues);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MixedAggResults::None => None,
            MixedAggResults::Direct { results, position } => {
                if *position < results.len() {
                    let result = &results[*position];
                    *position += 1;
                    Some((
                        result.score,
                        result.doc_address,
                        result.field_values.clone(),
                    ))
                } else {
                    None
                }
            }
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

        // Pre-size based on expected field counts
        let string_field_count = string_fields.len();
        let numeric_field_count = numeric_fields.len();

        // Thread-local dictionary for caching during collection
        // Prevents re-resolving the same terms repeatedly in a single segment
        thread_local! {
            static COLLECTION_CACHE: RefCell<HashMap<(TermOrdinal, u64), Option<String>>> =
                RefCell::new(HashMap::with_capacity(1024));
        }

        // Process all segment results in parallel
        let processed_results = results
            .into_par_iter()
            .flat_map(
                |(string_columns, string_results, numeric_columns, numeric_values)| {
                    // Local documents for this segment
                    let mut segment_docs = HashMap::new();

                    // Clear the thread-local collection cache at the beginning of each segment
                    COLLECTION_CACHE.with(|cache| cache.borrow_mut().clear());

                    // Process string fields
                    for field_idx in 0..string_columns.len() {
                        if field_idx >= string_results.len() {
                            continue; // Skip if no results for this field
                        }

                        let (field_name, str_ff) = &string_columns[field_idx];
                        let field_result = &string_results[field_idx];

                        // Generate a unique dictionary ID for caching
                        let dictionary_id = str_ff.dictionary() as *const _ as u64;

                        // Process each term ordinate and its documents
                        for (term_ord, docs) in field_result.iter() {
                            // Use collection cache first then fall back to main cache
                            let term_value = COLLECTION_CACHE.with(|cache| {
                                let mut cache = cache.borrow_mut();
                                if let Some(term) = cache.get(&(*term_ord, dictionary_id)) {
                                    term.clone()
                                } else {
                                    // Resolve and cache for this collection run
                                    let resolved =
                                        resolve_term_with_cache(*term_ord, str_ff, dictionary_id);
                                    cache.insert((*term_ord, dictionary_id), resolved.clone());
                                    resolved
                                }
                            });

                            // Add this term to all matching documents
                            for (score, doc_addr) in docs {
                                let entry = segment_docs.entry(*doc_addr).or_insert_with(|| {
                                    (
                                        FieldValues::with_capacity(
                                            string_field_count,
                                            numeric_field_count,
                                        ),
                                        *score,
                                    )
                                });
                                entry.0.set_string(field_name, term_value.clone());
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
                            if let Some((field_values, _)) = segment_docs.get_mut(doc_id) {
                                field_values.set_numeric(field_name, value.clone());
                            }
                        }
                    }

                    // Convert to Vec of DocResults
                    segment_docs
                        .into_iter()
                        .map(|(doc_address, (field_values, score))| DocResult {
                            doc_address,
                            score,
                            field_values,
                        })
                        .collect::<Vec<_>>()
                },
            )
            .collect::<Vec<_>>();

        // Return as direct results for processing
        MixedAggResults::Direct {
            results: processed_results,
            position: 0,
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

        // Pre-size based on expected field counts
        let string_field_count = string_fields.len();
        let numeric_field_count = numeric_fields.len();
        let expected_doc_count = segment_reader.num_docs() as usize / 10; // Estimate 10% match rate

        // Thread-local cache for this segment only
        thread_local! {
            static SEGMENT_CACHE: RefCell<HashMap<(TermOrdinal, u64), Option<String>>> =
                RefCell::new(HashMap::with_capacity(1024));
        }

        // Clear the segment cache
        SEGMENT_CACHE.with(|cache| cache.borrow_mut().clear());

        // Track documents and their field values
        let mut doc_fields = HashMap::with_capacity(expected_doc_count);

        // Process string fields from this segment
        let string_columns = &segment_result.0;
        let string_results = &segment_result.1;

        for field_idx in 0..string_columns.len() {
            if field_idx >= string_results.len() {
                continue;
            }

            let (field_name, str_ff) = &string_columns[field_idx];
            let field_result = &string_results[field_idx];

            // Generate a unique dictionary ID for caching
            let dictionary_id = str_ff.dictionary() as *const _ as u64;

            // Process each term ordinate using cached resolution
            for (term_ord, docs) in field_result {
                // Use segment cache first then fall back to main cache
                let term_value = SEGMENT_CACHE.with(|cache| {
                    let mut cache = cache.borrow_mut();
                    if let Some(term) = cache.get(&(*term_ord, dictionary_id)) {
                        term.clone()
                    } else {
                        // Resolve and cache for this segment
                        let resolved = resolve_term_with_cache(*term_ord, str_ff, dictionary_id);
                        cache.insert((*term_ord, dictionary_id), resolved.clone());
                        resolved
                    }
                });

                // Add term to each document
                for (score, doc_addr) in docs {
                    let entry = doc_fields.entry(*doc_addr).or_insert_with(|| {
                        (
                            FieldValues::with_capacity(string_field_count, numeric_field_count),
                            *score,
                        )
                    });
                    entry.0.set_string(field_name, term_value.clone());
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
            for (doc_id, value) in field_values.iter() {
                if let Some(entry) = doc_fields.get_mut(doc_id) {
                    entry.0.set_numeric(field_name, value.clone());
                }
            }
        }

        // Send all results through the channel
        for (doc_addr, (mut field_values, score)) in doc_fields {
            // Ensure all requested fields have entries (even if null)
            for field in string_fields {
                if !field_values.contains_string_field(field) {
                    field_values.set_string(field, None);
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
