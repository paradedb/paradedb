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
//! This module provides an optimized execution method that can efficientl y handle
//! both multiple string fast fields and numeric fast fields simultaneously,
//! overcoming the limitation where previously ParadeDB could only support
//! either multiple numeric fast fields OR a single string fast field.

use std::rc::Rc;

use crate::api::HashMap;
use crate::index::fast_fields_helper::{FFIndex, FFType, WhichFastField};
use crate::index::reader::index::{SearchIndexReader, SearchIndexScore};
use crate::postgres::customscan::pdbscan::exec_methods::fast_fields::{
    non_string_ff_to_datum, ords_to_sorted_terms, FastFieldExecState,
};
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::fast_fields::StrColumn;
use crate::postgres::customscan::pdbscan::is_block_all_visible;
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::postgres::types::TantivyValue;
use crate::query::SearchQueryInput;

use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys;
use pgrx::PgOid;
use tantivy::collector::Collector;
use tantivy::index::SegmentId;
use tantivy::schema::document::OwnedValue;
use tantivy::schema::Schema;
use tantivy::termdict::TermOrdinal;
use tantivy::{DocAddress, Executor, SegmentOrdinal};
use tinyvec::TinyVec;

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
///
/// # Feature Flag
/// This execution method is controlled by the `paradedb.enable_mixed_fast_field_exec` GUC setting.
/// It is disabled by default and can be enabled with:
/// ```sql
/// SET paradedb.enable_mixed_fast_field_exec = true;
/// ```
pub struct MixedFastFieldExecState {
    /// Core functionality shared with other fast field execution methods
    inner: FastFieldExecState,

    /// Optimized results storage for both string and numeric fields
    mixed_results: MixedAggResults,

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
        let fields_len = which_fast_fields.len();
        Self {
            inner: FastFieldExecState::new(which_fast_fields),
            mixed_results: MixedAggResults::new(fields_len, vec![]),
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
        self.mixed_results = MixedAggResults::new(self.inner.which_fast_fields.len(), vec![]);
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
                    state.search_query_input(),
                    &self.inner.which_fast_fields,
                    segment_id,
                );
                return true;
            }

            // No more segments to query in parallel mode
            self.mixed_results = MixedAggResults::new(self.inner.which_fast_fields.len(), vec![]);
            false
        } else if self.inner.did_query {
            // Not parallel and already queried
            false
        } else {
            // First time query in non-parallel mode
            let searcher = MixedAggSearcher(state.search_reader.as_ref().unwrap());
            self.mixed_results = searcher.mixed_agg(
                state.need_scores(),
                state.search_query_input(),
                &self.inner.which_fast_fields,
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
    fn internal_next(&mut self, state: &mut PdbScanState) -> ExecState {
        unsafe {
            // Process the next result from our optimized path
            match self.mixed_results.next() {
                None => ExecState::Eof,
                Some((scored, doc_address, field_values)) => {
                    let heaprel = self
                        .inner
                        .heaprel
                        .as_ref()
                        .expect("MixedFastFieldsExecState: heaprel should be initialized");
                    let slot = self.inner.slot;
                    let natts = (*(*slot).tts_tupleDescriptor).natts as usize;

                    // Set ctid and table OID
                    crate::postgres::utils::u64_to_item_pointer(scored.ctid, &mut (*slot).tts_tid);
                    (*slot).tts_tableOid = heaprel.oid();

                    // Check visibility of the current block
                    let blockno = item_pointer_get_block_number(&(*slot).tts_tid);
                    let is_visible = if blockno == self.inner.blockvis.0 {
                        // We already know the visibility of this block because we just checked it last time
                        self.inner.blockvis.1
                    } else {
                        // New block, check visibility
                        self.inner.blockvis.0 = blockno;
                        self.inner.blockvis.1 =
                            is_block_all_visible(heaprel, &mut self.inner.vmbuff, blockno);
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

                        let which_fast_fields = &self.inner.which_fast_fields;
                        let tupdesc = self.inner.tupdesc.as_ref().unwrap();
                        debug_assert!(natts == which_fast_fields.len());

                        // Process each column, converting fast field values to PostgreSQL datums
                        for (i, ((att, field_value), which_fast_field)) in self
                            .inner
                            .tupdesc
                            .as_ref()
                            .unwrap()
                            .iter()
                            .zip(field_values.into_iter())
                            .zip(which_fast_fields)
                            .enumerate()
                        {
                            match which_fast_field {
                                WhichFastField::Named(_, _) => {
                                    // We extracted this field: convert it into a datum.
                                    match field_value.try_into_datum(PgOid::from(att.atttypid)) {
                                        Ok(Some(datum)) => {
                                            datums[i] = datum;
                                            isnull[i] = false;
                                            continue;
                                        }
                                        Ok(None) => {
                                            // Null datum.
                                            continue;
                                        }
                                        Err(e) => {
                                            panic!(
                                                "Failed to convert to attribute type for \
                                                {:?} and {which_fast_field:?}: {e}",
                                                att.atttypid
                                            );
                                        }
                                    }
                                }
                                _ => {
                                    // Fall back to non_string_ff_to_datum for things like the score, ctid,
                                    // etc.
                                    if let Some(datum) = non_string_ff_to_datum(
                                        (&which_fast_fields[i], i),
                                        att.atttypid,
                                        scored.bm25,
                                        doc_address,
                                        &mut self.inner.ffhelper,
                                        slot,
                                    ) {
                                        datums[i] = datum;
                                        isnull[i] = false;
                                    }
                                }
                            }
                        }

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
    fn reset(&mut self, state: &mut PdbScanState) {
        // Reset inner FastFieldExecState
        self.inner.reset(state);

        // Reset mixed results state
        self.mixed_results = MixedAggResults::new(self.inner.which_fast_fields.len(), vec![]);

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

/// Either an Rc<str>, or a TantivyValue representing any non-string value.
///
/// This indirection avoids cloning the  from string fast fields until it is
/// time to convert them into
enum MixedField {
    Other(TantivyValue),
    String(Rc<str>),
}

impl Default for MixedField {
    fn default() -> Self {
        Self::Other(TantivyValue::default())
    }
}

/// A fixed-size container for storing mixed field values from fast fields.
pub struct FieldValues(TinyVec<[MixedField; 4]>);

impl FieldValues {
    /// Creates a new fixed-size FieldValues container.
    fn new(size: usize) -> Self {
        Self((0..size).map(|_| MixedField::default()).collect())
    }

    fn set_string(&mut self, field: FFIndex, value: Option<Rc<str>>) {
        self.0[field] = value.map(MixedField::String).unwrap_or_default();
    }

    fn set_numeric(&mut self, field: FFIndex, value: OwnedValue) {
        self.0[field] = MixedField::Other(TantivyValue(value));
    }

    fn into_iter(self) -> impl Iterator<Item = TantivyValue> {
        self.0.into_iter().map(|v| match v {
            MixedField::String(s) => TantivyValue(OwnedValue::Str((*s).to_owned())),
            MixedField::Other(o) => o,
        })
    }
}

/// The result of searching one segment.
type SegmentResult = (
    // A vec of string columns and their associated FFIndex values.
    Vec<(FFIndex, StrColumn)>,
    // A vec (of the same length) of string column matches.
    Vec<Vec<(TermOrdinal, SearchIndexScore, DocAddress)>>,
    // A vec of integer columns and their associated FFIndex values.
    Vec<(FFIndex, FFType)>,
    // A vec (of the same length) of numeric column matches.
    Vec<Vec<(OwnedValue, SearchIndexScore, DocAddress)>>,
);

struct MixedAggResults {
    /// Length of the FieldValues for this MixedAgg.
    fields_len: usize,
    /// Per-segment results which have yet to be emitted.
    per_segment: std::vec::IntoIter<SegmentResult>,
    /// An iterator for the current segment.
    current_segment: Box<dyn Iterator<Item = (SearchIndexScore, DocAddress, FieldValues)>>,
}

impl MixedAggResults {
    fn new(fields_len: usize, per_segment: Vec<SegmentResult>) -> Self {
        Self {
            fields_len,
            per_segment: per_segment.into_iter(),
            current_segment: Box::new(std::iter::empty()),
        }
    }

    fn next_segment(&mut self) -> bool {
        let Some((mut string_columns, mut string_results, numeric_columns, numeric_values)) =
            self.per_segment.next()
        else {
            return false;
        };

        // Build a hashmap of any non-sort column values, which will then be hash joined to the
        // sort column, if any.
        let rows = string_results
            .first()
            .map(|res| res.len())
            .or(numeric_values.first().map(|res| res.len()))
            .unwrap_or(16);
        let mut doc_fields = HashMap::with_capacity_and_hasher(rows, Default::default());

        // Pop a string column to use as the sort order for emitted rows, if any.
        //
        // Note that under some combinations of `paradedb.enable_fast_field_exec` and
        // `paradedb.enable_mixed_fast_field_exec`, Mixed might be used without a string column.
        //
        // TODO: Make the choice of sort column to use a planning-time decision.
        let string_sort_column = string_columns.pop();
        let string_sort_results = string_results.pop();

        // Process remaining string fields from this segment
        for ((field_idx, str_ff), field_result) in string_columns.into_iter().zip(string_results) {
            // Resolve all term ordinals to their string values.
            let field_results_iter =
                ords_to_sorted_terms(str_ff, field_result, |(term_ordinal, _, _)| *term_ordinal);

            // Add term to each document
            for ((_, score, doc_addr), term_value) in field_results_iter {
                doc_fields
                    .entry(doc_addr)
                    .or_insert_with(|| (FieldValues::new(self.fields_len), score))
                    .0
                    .set_string(field_idx, term_value);
            }
        }

        // Process numeric fields from this segment
        for ((field_idx, _), field_values) in numeric_columns.into_iter().zip(numeric_values) {
            // Add numeric values to all matching documents
            for (value, score, doc_addr) in field_values {
                doc_fields
                    .entry(doc_addr)
                    .or_insert_with(|| (FieldValues::new(self.fields_len), score))
                    .0
                    .set_numeric(field_idx, value);
            }
        }

        if let Some(((sort_field_idx, sort_str_ff), string_sort_results)) =
            string_sort_column.zip(string_sort_results)
        {
            // We have a sort column, so create an iterator that lazily scans the sort column and
            // joins the remaining columns.
            let fields_len = self.fields_len;
            self.current_segment = Box::new(
                ords_to_sorted_terms(sort_str_ff, string_sort_results, |(term_ordinal, _, _)| {
                    *term_ordinal
                })
                .map(move |((_, score, doc_addr), term_value)| {
                    let (mut field_values, score) = doc_fields
                        .remove(&doc_addr)
                        .unwrap_or_else(|| (FieldValues::new(fields_len), score));
                    field_values.set_string(sort_field_idx, term_value);
                    (score, doc_addr, field_values)
                }),
            );
        } else {
            // Otherwise, emit the remaining columns directly.
            self.current_segment = Box::new(
                doc_fields
                    .into_iter()
                    .map(|(doc_addr, (field_values, score))| (score, doc_addr, field_values)),
            );
        }

        true
    }
}

impl Iterator for MixedAggResults {
    type Item = (SearchIndexScore, DocAddress, FieldValues);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // See if there are more results from the current segment.
            if let Some(next) = self.current_segment.next() {
                return Some(next);
            }

            // Get results from the next segment, if any.
            if !self.next_segment() {
                return None;
            }
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
    /// * `fields` - Fast fields to retrieve
    ///
    /// # Returns
    ///
    /// Aggregated results organized by field values
    pub fn mixed_agg(
        &self,
        need_scores: bool,
        query: &SearchQueryInput,
        fields: &[WhichFastField],
    ) -> MixedAggResults {
        // Create collector that handles both string and numeric fields
        let collector = multi_field_collector::MultiFieldCollector {
            need_scores,
            fields: fields.to_vec(),
        };

        // Execute search with the appropriate scoring mode
        let schema = Schema::from(self.0.schema().clone());
        let results = self
            .0
            .searcher()
            .search_with_executor(
                self.0.query(),
                &collector,
                &Executor::SingleThread,
                if need_scores {
                    tantivy::query::EnableScoring::Enabled {
                        searcher: self.0.searcher(),
                        statistics_provider: self.0.searcher(),
                    }
                } else {
                    tantivy::query::EnableScoring::Disabled {
                        schema: &schema,
                        searcher_opt: Some(self.0.searcher()),
                    }
                },
            )
            .expect("failed to search");

        MixedAggResults::new(fields.len(), results)
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
        fields: &[WhichFastField],
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
            fields: fields.to_vec(),
        };

        // Create a query weight for this segment
        let schema = Schema::from(self.0.schema().clone());
        let weight = self.0.weight();

        // Execute search on this specific segment
        let result = collector
            .collect_segment(
                weight.as_ref(),
                segment_ord as SegmentOrdinal,
                segment_reader,
            )
            .expect("single segment collection should succeed");

        MixedAggResults::new(fields.len(), vec![result])
    }
}

/// Module for collecting both string and numeric fast field values during search.
///
/// This implementation extends Tantivy's collector framework to efficiently gather
/// multiple field types simultaneously during a single index traversal.
mod multi_field_collector {
    use crate::index::fast_fields_helper::{FFIndex, FFType, FastFieldType, WhichFastField};
    use crate::index::reader::index::SearchIndexScore;
    use crate::postgres::customscan::pdbscan::exec_methods::fast_fields::NULL_TERM_ORDINAL;

    use tantivy::collector::{Collector, SegmentCollector};
    use tantivy::columnar::StrColumn;
    use tantivy::schema::document::OwnedValue;
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

        /// List of fast fields to collect
        pub fields: Vec<WhichFastField>,
    }

    impl Collector for MultiFieldCollector {
        // Each fruit contains the columns, results, and values for both string and numeric fields
        type Fruit = Vec<super::SegmentResult>;
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

            // Get columns for all requested fields
            let mut string_columns = Vec::new();
            let mut string_results = Vec::new();
            let mut numeric_columns = Vec::new();
            let mut numeric_values = Vec::new();
            for (field_idx, fast_field) in self.fields.iter().enumerate() {
                match fast_field {
                    WhichFastField::Named(field_name, FastFieldType::String) => {
                        if let Ok(Some(str_column)) = ff.str(field_name) {
                            string_columns.push((field_idx, str_column));
                            string_results.push(Vec::default());
                        }
                    }
                    WhichFastField::Named(field_name, FastFieldType::Numeric) => {
                        // Try different numeric field types in order
                        let ff_type = if let Ok(i64_col) = ff.i64(field_name) {
                            FFType::I64(i64_col)
                        } else if let Ok(u64_col) = ff.u64(field_name) {
                            FFType::U64(u64_col)
                        } else if let Ok(f64_col) = ff.f64(field_name) {
                            FFType::F64(f64_col)
                        } else if let Ok(bool_col) = ff.bool(field_name) {
                            FFType::Bool(bool_col)
                        } else if let Ok(date_col) = ff.date(field_name) {
                            FFType::Date(date_col)
                        } else {
                            panic!("Unrecognized numeric fast field type for: {field_name}");
                        };

                        numeric_columns.push((field_idx, ff_type));
                        numeric_values.push(Vec::default());
                    }
                    _ => {}
                }
            }

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
        pub string_columns: Vec<(FFIndex, StrColumn)>,

        /// Results for string fields
        pub string_results: Vec<Vec<(TermOrdinal, SearchIndexScore, DocAddress)>>,

        /// Numeric columns to collect from
        pub numeric_columns: Vec<(FFIndex, FFType)>,

        /// Results for numeric fields, organized by doc address
        pub numeric_values: Vec<Vec<(OwnedValue, SearchIndexScore, DocAddress)>>,

        /// Fast field for retrieving ctid values
        ctid_ff: FFType,
    }

    impl SegmentCollector for MultiFieldSegmentCollector {
        type Fruit = super::SegmentResult;

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

            // Collect string fields
            for (string_column_idx, (_, str_column)) in self.string_columns.iter().enumerate() {
                let term_ord = str_column
                    .term_ords(doc)
                    .next()
                    .unwrap_or(NULL_TERM_ORDINAL);
                self.string_results[string_column_idx].push((term_ord, scored, doc_address));
            }

            // Collect numeric fields - store in document-keyed maps
            for (numeric_column_idx, (_, field_type)) in self.numeric_columns.iter().enumerate() {
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
                    FFType::Date(col) => {
                        if let Some(val) = col.first(doc) {
                            OwnedValue::Date(val)
                        } else {
                            OwnedValue::Null
                        }
                    }
                    x => panic!("Unhandled column type {x:?}"),
                };

                // Store the value for this document
                self.numeric_values[numeric_column_idx].push((field_value, scored, doc_address));
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
