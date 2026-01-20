// Copyright (c) 2023-2026 ParadeDB, Inc.
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

//! JoinScan executor using FastField for batched ctid lookups.
//!
//! This executor provides streaming iteration over search results, allowing
//! JoinScan to fetch results incrementally without materializing all ctids upfront.
//!
//! # Known Limitations
//!
//! TODO(parallel): No parallel execution support. Would require partitioning the
//! search results across workers and coordinating iteration.
//!
//! TODO(topn): TopN execution for LIMIT queries was removed because
//! `search_top_n_unordered_in_segments` uses skip/offset pagination which doesn't
//! work correctly for unordered results (order can differ between calls). A proper
//! implementation would keep the iterator alive across batch fetches.

use crate::index::fast_fields_helper::FFHelper;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::{MultiSegmentSearchResults, SearchIndexReader};
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;
use pgrx::pg_sys;
use tantivy::SegmentOrdinal;

/// Batch size for FastField execution - number of results to fetch at once.
const FAST_FIELD_BATCH_SIZE: usize = 1024;

/// Result from the executor's next() call.
pub enum JoinExecResult {
    /// A visible tuple with its ctid and score.
    Visible { ctid: u64, score: f32 },
    /// No more results available.
    Eof,
}

/// State for FastField (batched) execution.
///
/// Uses FFHelper to batch-lookup ctids from fast fields for efficiency.
struct FastFieldExecState {
    /// Search results iterator.
    search_results: MultiSegmentSearchResults,
    /// Fast field helper for batched ctid lookups.
    ffhelper: FFHelper,
    /// Current batch of (ctid, score) pairs.
    batch: Vec<(u64, f32)>,
    /// Current position in the batch.
    batch_pos: usize,
}

/// Executor for JoinScan using FastField batched ctid lookups.
///
/// Provides streaming iteration, allowing the join to fetch results
/// incrementally rather than materializing all ctids upfront.
/// Note: Visibility checking is done downstream in extract_driving_join_key.
pub struct JoinSideExecutor {
    /// The search index reader.
    search_reader: SearchIndexReader,
    /// The execution state.
    exec_state: FastFieldExecState,
}

impl JoinSideExecutor {
    /// Create a new FastField executor.
    ///
    /// This executor uses batched ctid lookups via FFHelper for efficiency.
    ///
    /// # Arguments
    /// * `need_scores` - Whether to compute BM25 scores. Set to true if
    ///   paradedb.score() is used anywhere in the query (SELECT, ORDER BY, etc.)
    pub fn new_fast_field(
        _heaprel: &PgSearchRelation,
        indexrelid: pg_sys::Oid,
        query: SearchQueryInput,
        _snapshot: pg_sys::Snapshot,
        need_scores: bool,
    ) -> Self {
        let indexrel = PgSearchRelation::open(indexrelid);
        let search_reader = SearchIndexReader::open_with_context(
            &indexrel,
            query,
            need_scores,
            MvccSatisfies::Snapshot,
            None,
            None,
        )
        .expect("Failed to open search reader for FastField executor");

        // Create FFHelper for batched ctid lookups (empty field list - we only need ctid)
        let ffhelper = FFHelper::with_fields(&search_reader, &[]);

        // Initialize search results
        let search_results = search_reader.search();

        Self {
            search_reader,
            exec_state: FastFieldExecState {
                search_results,
                ffhelper,
                batch: Vec::with_capacity(FAST_FIELD_BATCH_SIZE),
                batch_pos: 0,
            },
        }
    }

    /// Get the next visible ctid with its score.
    pub fn next_visible(&mut self) -> JoinExecResult {
        loop {
            pgrx::check_for_interrupts!();

            // Return from current batch if available
            if self.exec_state.batch_pos < self.exec_state.batch.len() {
                let (ctid, score) = self.exec_state.batch[self.exec_state.batch_pos];
                self.exec_state.batch_pos += 1;
                return JoinExecResult::Visible { ctid, score };
            }

            // Need to fetch a new batch
            if !self.fetch_batch() {
                return JoinExecResult::Eof;
            }
        }
    }

    /// Reset the executor for a rescan.
    ///
    /// This is called when JoinScan is nested inside another join and needs
    /// to be rescanned. We re-initialize the search results from the reader.
    pub fn reset(&mut self) {
        self.exec_state.search_results = self.search_reader.search();
        self.exec_state.batch.clear();
        self.exec_state.batch_pos = 0;
    }

    /// Fetch a batch of ctids using FFHelper for efficiency.
    fn fetch_batch(&mut self) -> bool {
        // Clear and reset batch
        self.exec_state.batch.clear();
        self.exec_state.batch_pos = 0;

        // Collect a batch of results from the current segment
        loop {
            let Some(scorer_iter) = self.exec_state.search_results.current_segment() else {
                return false;
            };

            let segment_ord = scorer_iter.segment_ord();
            let mut scores = Vec::with_capacity(FAST_FIELD_BATCH_SIZE);
            let mut doc_ids = Vec::with_capacity(FAST_FIELD_BATCH_SIZE);

            // Collect doc_ids and scores for this batch
            while doc_ids.len() < FAST_FIELD_BATCH_SIZE {
                let Some((score, doc_address)) = scorer_iter.next() else {
                    // No more results in this segment
                    self.exec_state.search_results.current_segment_pop();
                    break;
                };
                scores.push(score);
                doc_ids.push(doc_address.doc_id);
            }

            if doc_ids.is_empty() {
                // This segment is empty, try the next one
                continue;
            }

            // Batch lookup ctids using FFHelper
            let ctids =
                Self::batch_lookup_ctids_inner(&self.exec_state.ffhelper, segment_ord, &doc_ids);

            // Build the batch of (ctid, score) pairs
            self.exec_state.batch.reserve(ctids.len());
            for (ctid, score) in ctids.into_iter().zip(scores) {
                self.exec_state.batch.push((ctid, score));
            }

            return !self.exec_state.batch.is_empty();
        }
    }

    /// Batch lookup ctids for the given doc_ids in a segment.
    fn batch_lookup_ctids_inner(
        ffhelper: &FFHelper,
        segment_ord: SegmentOrdinal,
        doc_ids: &[u32],
    ) -> Vec<u64> {
        let ctid_column = ffhelper.ctid(segment_ord);
        let mut ctids = Vec::with_capacity(doc_ids.len());
        ctids.resize(doc_ids.len(), None);

        ctid_column.as_u64s(doc_ids, &mut ctids);

        ctids
            .into_iter()
            .map(|ctid| ctid.expect("All docs must have ctids"))
            .collect()
    }
}
