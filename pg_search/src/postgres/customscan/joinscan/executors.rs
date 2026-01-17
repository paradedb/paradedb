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
//! These wrappers provide streaming iteration with visibility checking,
//! allowing JoinScan to use TopN (for LIMIT queries) or Normal scan
//! without materializing all results upfront.

use std::collections::HashSet;

use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::{
    MultiSegmentSearchResults, SearchIndexReader, TopNSearchResults,
};
use crate::postgres::heap::OwnedVisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;
use pgrx::pg_sys;

/// Result from the executor's next() call.
#[allow(dead_code)]
pub enum JoinExecResult {
    /// A visible tuple with its ctid and score.
    Visible { ctid: u64, score: f32 },
    /// No more results available (exhausted or limit reached).
    Eof,
}

/// Execution method type for JoinSideExecutor.
#[allow(dead_code)]
enum ExecMethod {
    /// TopN execution for queries with LIMIT - fetches incrementally.
    TopN(TopNExecState),
    /// Normal execution - full scan.
    Normal(NormalExecState),
}

/// State for TopN (limited) execution.
#[allow(dead_code)]
struct TopNExecState {
    limit: usize,
    search_results: TopNSearchResults,
    offset: usize,
    chunk_size: usize,
    scale_factor: f64,
    found: usize,
    exhausted: bool,
    did_query: bool,
}

/// State for Normal (full scan) execution.
#[allow(dead_code)]
struct NormalExecState {
    search_results: Option<MultiSegmentSearchResults>,
    did_query: bool,
}

/// Wrapper around BaseScan exec methods for JoinScan usage.
///
/// Provides streaming iteration with visibility checking, allowing the join
/// to fetch results incrementally rather than materializing all ctids upfront.
#[allow(dead_code)]
pub struct JoinSideExecutor {
    /// The search index reader.
    search_reader: SearchIndexReader,
    /// Visibility checker for resolving stale ctids.
    visibility_checker: OwnedVisibilityChecker,
    /// The execution method (TopN or Normal).
    exec_method: ExecMethod,
}

#[allow(dead_code)]
impl JoinSideExecutor {
    /// Create a new TopN executor for driving side with LIMIT.
    ///
    /// This executor fetches results incrementally - starting with a scaled
    /// multiple of the limit, and asking for more if needed.
    pub fn new_topn(
        limit: usize,
        heaprel: &PgSearchRelation,
        indexrelid: pg_sys::Oid,
        query: SearchQueryInput,
        snapshot: pg_sys::Snapshot,
    ) -> Self {
        let indexrel = PgSearchRelation::open(indexrelid);
        let search_reader = SearchIndexReader::open_with_context(
            &indexrel,
            query,
            true, // need_scores for ordering
            MvccSatisfies::Snapshot,
            None,
            None,
        )
        .expect("Failed to open search reader for TopN executor");

        let visibility_checker = OwnedVisibilityChecker::new(heaprel, snapshot);

        // Calculate scale factor based on dead/live tuple ratio
        let scale_factor = unsafe {
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
        } * crate::gucs::limit_fetch_multiplier();

        Self {
            search_reader,
            visibility_checker,
            exec_method: ExecMethod::TopN(TopNExecState {
                limit,
                search_results: TopNSearchResults::empty(),
                offset: 0,
                chunk_size: 0,
                scale_factor,
                found: 0,
                exhausted: false,
                did_query: false,
            }),
        }
    }

    /// Create a new Normal executor for full scan (build side or unlimited driving).
    pub fn new_normal(
        heaprel: &PgSearchRelation,
        indexrelid: pg_sys::Oid,
        query: SearchQueryInput,
        snapshot: pg_sys::Snapshot,
    ) -> Self {
        let indexrel = PgSearchRelation::open(indexrelid);
        let search_reader = SearchIndexReader::open_with_context(
            &indexrel,
            query,
            false, // don't need scores for normal scan
            MvccSatisfies::Snapshot,
            None,
            None,
        )
        .expect("Failed to open search reader for Normal executor");

        // Initialize search results immediately (not lazily)
        let search_results = search_reader.search();

        let visibility_checker = OwnedVisibilityChecker::new(heaprel, snapshot);

        Self {
            search_reader,
            visibility_checker,
            exec_method: ExecMethod::Normal(NormalExecState {
                search_results: Some(search_results),
                did_query: true,
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
            self.next_normal_impl()
        }
    }

    /// Collect all matching ctids into a HashSet.
    ///
    /// This is used for build side filtering and join-level predicate evaluation
    /// where we need to materialize all matching ctids.
    pub fn collect_all_ctids(mut self) -> HashSet<u64> {
        let mut ctids = HashSet::new();

        while let JoinExecResult::Visible { ctid, .. } = self.next_visible() {
            ctids.insert(ctid);
        }

        ctids
    }

    /// Check if we've found enough results (for TopN with limit).
    pub fn reached_limit(&self) -> bool {
        match &self.exec_method {
            ExecMethod::TopN(state) => state.found >= state.limit,
            ExecMethod::Normal(_) => false,
        }
    }

    // --- TopN execution ---

    fn next_topn_impl(&mut self) -> JoinExecResult {
        loop {
            pgrx::check_for_interrupts!();

            let ExecMethod::TopN(state) = &mut self.exec_method else {
                unreachable!()
            };

            // Check if we've found enough
            if state.found >= state.limit {
                return JoinExecResult::Eof;
            }

            // Try to get next result from current batch
            if let Some((scored, _doc_address)) = state.search_results.next() {
                // Check visibility and resolve stale ctids
                if let Some(current_ctid) = self.visibility_checker.get_current_ctid(scored.ctid) {
                    return JoinExecResult::Visible {
                        ctid: current_ctid,
                        score: scored.bm25,
                    };
                }
                // Tuple not visible, try next
                continue;
            }

            // Current batch exhausted, try to query more
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

        if state.found >= state.limit || state.exhausted {
            return false;
        }

        // Calculate the limit for this query
        let local_limit =
            (state.limit as f64 * state.scale_factor).max(state.chunk_size as f64) as usize;
        let next_offset = state.offset + local_limit;
        let current_offset = state.offset;

        // Get segment IDs before borrowing search_reader
        let segment_ids: Vec<_> = self.search_reader.segment_ids();

        // Execute the TopN query
        let results = self.search_reader.search_top_n_unordered_in_segments(
            segment_ids.into_iter(),
            local_limit,
            current_offset,
        );

        // Update state
        let ExecMethod::TopN(state) = &mut self.exec_method else {
            unreachable!()
        };
        let original_len = results.original_len();
        state.search_results = results;
        state.offset = next_offset;
        state.exhausted = original_len < local_limit;

        // Update chunk size for retry scaling
        state.chunk_size = (state.chunk_size * crate::gucs::topn_retry_scale_factor() as usize)
            .max(
                (state.limit as f64
                    * state.scale_factor
                    * crate::gucs::topn_retry_scale_factor() as f64) as usize,
            )
            .min(crate::gucs::max_topn_chunk_size() as usize);

        original_len > 0
    }

    // --- Normal execution ---

    fn next_normal_impl(&mut self) -> JoinExecResult {
        loop {
            pgrx::check_for_interrupts!();

            let ExecMethod::Normal(state) = &mut self.exec_method else {
                unreachable!()
            };

            // Initialize search results if not done yet
            if !state.did_query {
                state.search_results = Some(self.search_reader.search());
                state.did_query = true;
            }

            let Some(search_results) = state.search_results.as_mut() else {
                return JoinExecResult::Eof;
            };

            // Try to get next result
            match search_results.next() {
                Some((scored, _doc_address)) => {
                    // Check visibility and resolve stale ctids
                    if let Some(current_ctid) =
                        self.visibility_checker.get_current_ctid(scored.ctid)
                    {
                        return JoinExecResult::Visible {
                            ctid: current_ctid,
                            score: scored.bm25,
                        };
                    }
                    // Tuple not visible, try next
                    continue;
                }
                None => return JoinExecResult::Eof,
            }
        }
    }
}

// Need to add IntoDatum for Oid
use pgrx::IntoDatum;
