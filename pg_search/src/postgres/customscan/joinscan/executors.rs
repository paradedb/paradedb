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

//! JoinScan executor wrappers around BaseScan execution methods.
//!
//! These wrappers provide streaming iteration, allowing JoinScan to use
//! TopN (for LIMIT queries) or FastField (for unlimited scans) without
//! materializing all results upfront.
//!
//! # Known Limitations
//!
//! TODO(parallel): No parallel execution support. Would require partitioning the
//! search results across workers and coordinating iteration.

use std::collections::HashSet;

use crate::index::fast_fields_helper::FFHelper;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::{
    MultiSegmentSearchResults, SearchIndexReader, TopNSearchResults,
};
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;
use pgrx::{pg_sys, IntoDatum};
use tantivy::SegmentOrdinal;

/// Batch size for FastField execution - number of results to fetch at once.
const FAST_FIELD_BATCH_SIZE: usize = 1024;

/// Result from the executor's next() call.
pub enum JoinExecResult {
    /// A visible tuple with its ctid and score.
    Visible { ctid: u64, score: f32 },
    /// No more results available (exhausted or limit reached).
    Eof,
}

/// Execution method type for JoinSideExecutor.
enum ExecMethod {
    /// TopN execution for queries with LIMIT - fetches incrementally in batches.
    /// For joins, the batch_size controls how many driving rows to fetch at once,
    /// but the actual LIMIT is enforced by the join loop based on output rows.
    /// Note: Currently unused for joins due to pagination bug with unordered results.
    #[allow(dead_code)]
    TopN(TopNExecState),
    /// FastField execution - batched ctid lookups for efficiency.
    FastField(FastFieldExecState),
}

/// State for TopN (batched) execution.
///
/// This fetches results in batches of `batch_size`, allowing incremental fetching
/// without scanning the entire index upfront. For joins, the join loop controls
/// when to stop based on output rows, not on driving rows fetched.
struct TopNExecState {
    /// Batch size for each TopN query (how many to fetch at once).
    batch_size: usize,
    /// Current batch of search results.
    search_results: TopNSearchResults,
    /// Offset into the index for pagination.
    offset: usize,
    /// Scale factor for batch size growth.
    scale_factor: f64,
    /// Whether the index is exhausted (no more results available).
    exhausted: bool,
    /// Whether we've done at least one query.
    did_query: bool,
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

/// Wrapper around BaseScan exec methods for JoinScan usage.
///
/// Provides streaming iteration, allowing the join to fetch results
/// incrementally rather than materializing all ctids upfront.
/// Note: Visibility checking is done downstream in extract_driving_join_key.
pub struct JoinSideExecutor {
    /// The search index reader.
    search_reader: SearchIndexReader,
    /// The execution method (TopN or FastField).
    exec_method: ExecMethod,
}

impl JoinSideExecutor {
    /// Create a new TopN executor for driving side with LIMIT.
    ///
    /// This executor fetches results incrementally in batches, allowing the join
    /// to request more driving rows as needed until enough output rows are produced.
    ///
    /// Note: Currently unused for joins due to pagination bug with unordered results.
    /// Kept for potential single-relation scan use or future ordered TopK joins.
    ///
    /// # Arguments
    /// * `initial_batch_size` - Initial batch size (typically `limit * scale_factor`)
    /// * `heaprel` - The heap relation for the driving side
    /// * `indexrelid` - The BM25 index OID
    /// * `query` - The search query input
    /// * `_snapshot` - The snapshot (unused but kept for API consistency)
    /// * `batch_scale_hint` - Optional scale factor hint from planner. If None,
    ///   calculates from dead tuple ratio at runtime.
    #[allow(dead_code)]
    pub fn new_topn(
        initial_batch_size: usize,
        heaprel: &PgSearchRelation,
        indexrelid: pg_sys::Oid,
        query: SearchQueryInput,
        _snapshot: pg_sys::Snapshot,
        batch_scale_hint: Option<f64>,
    ) -> Self {
        let indexrel = PgSearchRelation::open(indexrelid);
        // TODO(error-handling): This expect() will panic if index opening fails.
        // Consider returning Result<Self, _> and letting caller fall back to
        // PostgreSQL's native join execution.
        let search_reader = SearchIndexReader::open_with_context(
            &indexrel,
            query,
            true, // need_scores for ordering
            MvccSatisfies::Snapshot,
            None,
            None,
        )
        .expect("Failed to open search reader for TopN executor");

        // Use hint if provided, otherwise calculate scale factor based on dead/live tuple ratio
        let scale_factor = batch_scale_hint.unwrap_or_else(|| {
            let base_factor = unsafe {
                let n_dead = pgrx::direct_function_call::<i64>(
                    pg_sys::pg_stat_get_dead_tuples,
                    &[heaprel.rel_oid().into_datum()],
                )
                .unwrap_or(0);
                let n_live = pgrx::direct_function_call::<i64>(
                    pg_sys::pg_stat_get_live_tuples,
                    &[heaprel.rel_oid().into_datum()],
                )
                .unwrap_or(1);

                1.0 + ((1.0 + n_dead as f64) / (1.0 + n_live as f64))
            };
            base_factor * crate::gucs::limit_fetch_multiplier()
        });

        Self {
            search_reader,
            exec_method: ExecMethod::TopN(TopNExecState {
                batch_size: initial_batch_size.max(10), // At least 10 to avoid tiny batches
                search_results: TopNSearchResults::empty(),
                offset: 0,
                scale_factor,
                exhausted: false,
                did_query: false,
            }),
        }
    }

    /// Create a new FastField executor for unlimited scans.
    ///
    /// This executor uses batched ctid lookups via FFHelper for efficiency.
    /// Preferred over Normal scan for better performance.
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
            exec_method: ExecMethod::FastField(FastFieldExecState {
                search_results,
                ffhelper,
                batch: Vec::with_capacity(FAST_FIELD_BATCH_SIZE),
                batch_pos: 0,
            }),
        }
    }

    /// Get the next visible ctid with its score.
    ///
    /// For TopN execution, this may trigger additional queries if the current
    /// batch is exhausted but we haven't found enough results yet.
    pub fn next_visible(&mut self) -> JoinExecResult {
        // Determine which method to use
        let is_topn = matches!(self.exec_method, ExecMethod::TopN(_));

        if is_topn {
            self.next_topn_impl()
        } else {
            self.next_fast_field_impl()
        }
    }

    /// Collect all matching ctids into a HashSet.
    ///
    /// This is used for build side filtering and join-level predicate evaluation
    /// where we need to materialize all matching ctids.
    ///
    /// TODO(use-or-remove): This method was created for build side filtering in
    /// begin_custom_scan but isn't currently used. The build side materialization
    /// in mod.rs directly iterates SearchIndexReader results instead.
    ///
    /// Options:
    /// 1. Integrate this into begin_custom_scan for build side filtering
    /// 2. Use this for join-level predicate ctid set collection
    /// 3. Remove if neither use case materializes
    ///
    /// See TODO(build-side-streaming) in mod.rs for context.
    #[allow(dead_code)]
    pub fn collect_all_ctids(mut self) -> HashSet<u64> {
        let mut ctids = HashSet::new();

        while let JoinExecResult::Visible { ctid, .. } = self.next_visible() {
            ctids.insert(ctid);
        }

        ctids
    }

    /// Check if the index is exhausted (no more results available).
    #[allow(dead_code)]
    pub fn is_exhausted(&self) -> bool {
        match &self.exec_method {
            ExecMethod::TopN(state) => state.exhausted,
            // For FastField, we can't easily check without mutating, so return false
            // The next_visible() call will return Eof when truly exhausted
            ExecMethod::FastField(_) => false,
        }
    }

    /// Reset the executor for a rescan.
    ///
    /// This is called when JoinScan is nested inside another join and needs
    /// to be rescanned. We re-initialize the search results from the reader.
    pub fn reset(&mut self) {
        match &mut self.exec_method {
            ExecMethod::TopN(state) => {
                // Reset TopN state
                state.search_results = TopNSearchResults::empty();
                state.offset = 0;
                state.exhausted = false;
                state.did_query = false;
            }
            ExecMethod::FastField(state) => {
                // Re-initialize search results from the reader
                state.search_results = self.search_reader.search();
                state.batch.clear();
                state.batch_pos = 0;
            }
        }
    }

    // --- TopN execution ---

    fn next_topn_impl(&mut self) -> JoinExecResult {
        loop {
            pgrx::check_for_interrupts!();

            let ExecMethod::TopN(state) = &mut self.exec_method else {
                unreachable!()
            };

            // Try to get next result from current batch.
            // Return raw ctid without visibility checking - visibility is checked downstream.
            if let Some((scored, _doc_address)) = state.search_results.next() {
                return JoinExecResult::Visible {
                    ctid: scored.ctid,
                    score: scored.bm25,
                };
            }

            // Current batch exhausted, try to query more from the index
            if !self.query_topn_impl() {
                return JoinExecResult::Eof;
            }
        }
    }

    fn query_topn_impl(&mut self) -> bool {
        let ExecMethod::TopN(state) = &mut self.exec_method else {
            unreachable!()
        };

        state.did_query = true;

        if state.exhausted {
            return false;
        }

        // Use current batch_size, growing it over time for efficiency
        let fetch_size = state.batch_size;

        // Query for the next batch - use all segments
        let segment_ids = self
            .search_reader
            .segment_readers()
            .iter()
            .map(|r| r.segment_id());

        let results = self.search_reader.search_top_n_unordered_in_segments(
            segment_ids,
            fetch_size,
            state.offset,
        );

        // Check if we got any results
        let original_len = results.original_len();
        if original_len == 0 {
            state.exhausted = true;
            return false;
        }

        // Update state for next iteration
        // Grow batch size for future fetches (up to a reasonable max)
        state.batch_size = (state.batch_size as f64 * state.scale_factor).min(10000.0) as usize;
        state.offset += original_len;
        state.search_results = results;

        true
    }

    // --- FastField execution ---

    fn next_fast_field_impl(&mut self) -> JoinExecResult {
        loop {
            pgrx::check_for_interrupts!();

            let ExecMethod::FastField(state) = &mut self.exec_method else {
                unreachable!()
            };

            // Return from current batch if available
            if state.batch_pos < state.batch.len() {
                let (ctid, score) = state.batch[state.batch_pos];
                state.batch_pos += 1;
                return JoinExecResult::Visible { ctid, score };
            }

            // Need to fetch a new batch
            if !self.fetch_fast_field_batch() {
                return JoinExecResult::Eof;
            }
        }
    }

    /// Fetch a batch of ctids using FFHelper for efficiency.
    fn fetch_fast_field_batch(&mut self) -> bool {
        // Extract what we need before the mutable borrow
        let ExecMethod::FastField(state) = &mut self.exec_method else {
            unreachable!()
        };

        // Clear and reset batch
        state.batch.clear();
        state.batch_pos = 0;

        // Collect a batch of results from the current segment
        loop {
            let Some(scorer_iter) = state.search_results.current_segment() else {
                return false;
            };

            let segment_ord = scorer_iter.segment_ord();
            let mut scores = Vec::with_capacity(FAST_FIELD_BATCH_SIZE);
            let mut doc_ids = Vec::with_capacity(FAST_FIELD_BATCH_SIZE);

            // Collect doc_ids and scores for this batch
            while doc_ids.len() < FAST_FIELD_BATCH_SIZE {
                let Some((score, doc_address)) = scorer_iter.next() else {
                    // No more results in this segment
                    state.search_results.current_segment_pop();
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
            let ctids = Self::batch_lookup_ctids_inner(&state.ffhelper, segment_ord, &doc_ids);

            // Build the batch of (ctid, score) pairs
            state.batch.reserve(ctids.len());
            for (ctid, score) in ctids.into_iter().zip(scores) {
                state.batch.push((ctid, score));
            }

            return !state.batch.is_empty();
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
