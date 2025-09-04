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

use std::cell::RefCell;

use crate::api::OrderByInfo;
use crate::index::reader::index::{SearchIndexReader, TopNSearchResults, MAX_TOPN_FEATURES};
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::postgres::ParallelScanState;
use crate::query::SearchQueryInput;

use pgrx::{check_for_interrupts, direct_function_call, pg_sys, IntoDatum};
use tantivy::index::SegmentId;

// TODO:  should these be GUCs?  I think yes, probably
const SUBSEQUENT_RETRY_SCALE_FACTOR: usize = 2;
const MAX_CHUNK_SIZE: usize = 5000;

pub struct TopNScanExecState {
    // required
    limit: usize,
    orderby_info: Option<Vec<OrderByInfo>>,

    // set during init
    search_query_input: Option<SearchQueryInput>,
    search_reader: Option<SearchIndexReader>,

    // state tracking
    search_results: TopNSearchResults,
    nresults: usize,
    did_query: bool,
    exhausted: bool,
    found: usize,
    offset: usize,
    chunk_size: usize,
    // If parallel, the segments which have been claimed by this worker.
    claimed_segments: RefCell<Option<Vec<SegmentId>>>,
    scale_factor: f64,
}

impl TopNScanExecState {
    pub fn new(
        heaprelid: pg_sys::Oid,
        limit: usize,
        orderby_info: Option<Vec<OrderByInfo>>,
    ) -> Self {
        if matches!(&orderby_info, Some(orderby_info) if orderby_info.len() > MAX_TOPN_FEATURES) {
            panic!("Cannot sort by more than {MAX_TOPN_FEATURES} features.");
        }

        let scale_factor = unsafe {
            let n_dead = direct_function_call::<i64>(
                pg_sys::pg_stat_get_dead_tuples,
                &[heaprelid.into_datum()],
            )
            .unwrap();
            let n_live = direct_function_call::<i64>(
                pg_sys::pg_stat_get_live_tuples,
                &[heaprelid.into_datum()],
            )
            .unwrap();

            1.0 + ((1.0 + n_dead as f64) / (1.0 + n_live as f64))
        } * crate::gucs::limit_fetch_multiplier();

        Self {
            limit,
            orderby_info,
            search_query_input: None,
            search_reader: None,
            search_results: TopNSearchResults::empty(),
            nresults: 0,
            did_query: false,
            exhausted: false,
            found: 0,
            offset: 0,
            chunk_size: 0,
            claimed_segments: RefCell::default(),
            scale_factor,
        }
    }

    /// Produces an iterator of Segments to query.
    ///
    /// This method produces segments in three different modes:
    /// 1. For non-parallel execution: emits all segments eagerly.
    /// 2. For parallel execution, depending on whether this exec has been executed before (since
    ///    being `reset`):
    ///    a. 0th execution: lazily emits segments, so that this worker will finish querying
    ///    one segment and adding it to the Collector before attempting to claim another. This
    ///    allows all of the workers to load balance the work of searching the segments.
    ///    b. Nth execution: eagerly emits all segments which were previously collected. This is
    ///    necessary to allow for re-scans (when a Top-N result later proves not to be visible)
    ///    to consistently revisit the same segments.
    fn segments_to_query<'s>(
        &'s self,
        search_reader: &SearchIndexReader,
        parallel_state: Option<*mut ParallelScanState>,
    ) -> Box<dyn Iterator<Item = SegmentId> + 's> {
        match (parallel_state, self.claimed_segments.borrow().clone()) {
            (None, _) => {
                // Not parallel: will search all segments.
                let all_segments = search_reader.segment_ids();
                Box::new(all_segments.into_iter().inspect(|segment_id| {
                    check_for_interrupts!();
                }))
            }
            (Some(_), Some(claimed_segments)) => {
                // Parallel, but we have already claimed our segments. Emit them again.
                Box::new(claimed_segments.into_iter().inspect(|segment_id| {
                    check_for_interrupts!();
                }))
            }
            (Some(parallel_state), None) => {
                // Parallel, and we have not claimed our segments. Claim them, lazily, and then
                // record that we have done so.
                let mut claimed_segments = Vec::new();
                Box::new(std::iter::from_fn(move || {
                    check_for_interrupts!();
                    let maybe_segment_id = unsafe { checkout_segment(parallel_state) };
                    let Some(segment_id) = maybe_segment_id else {
                        // No more segments: record that we successfully claimed all segments, and
                        // then conclude iteration.
                        *self.claimed_segments.borrow_mut() =
                            Some(std::mem::take(&mut claimed_segments));
                        return None;
                    };

                    // We claimed a segment. record it, and then return it.
                    claimed_segments.push(segment_id);
                    Some(segment_id)
                }))
            }
        }
    }
}

impl ExecMethod for TopNScanExecState {
    ///
    /// Query more results.
    ///
    /// Called either because:
    /// * We've never run a query before (did_query=False).
    /// * Some of the results that we returned were not visible, and so the `chunk_size`, or
    ///   `offset` values have changed.
    ///
    fn query(&mut self, state: &mut PdbScanState) -> bool {
        self.did_query = true;

        if self.found >= self.limit || self.exhausted {
            return false;
        }

        // We track the total number of queries executed by Top-N (for any of the above reasons).
        state.increment_query_count();

        // Calculate the limit for this query, and what the offset will be for the next query.
        let local_limit =
            (self.limit as f64 * self.scale_factor).max(self.chunk_size as f64) as usize;
        let next_offset = self.offset + local_limit;

        self.search_results = state
            .search_reader
            .as_ref()
            .unwrap()
            .search_top_n_in_segments(
                self.segments_to_query(state.search_reader.as_ref().unwrap(), state.parallel_state),
                self.orderby_info.as_ref(),
                local_limit,
                self.offset,
            );

        // Record the offset to start from for the next query.
        self.offset = next_offset;

        // If we got fewer results than we requested, then the query is exhausted: there is no
        // point executing further queries.
        self.exhausted = self.search_results.original_len() < local_limit;

        // But if we got any results at all, then the query was a success.
        self.search_results.original_len() > 0
    }

    fn increment_visible(&mut self) {
        self.found += 1;
    }

    fn internal_next(&mut self, state: &mut PdbScanState) -> ExecState {
        loop {
            check_for_interrupts!();

            match self.search_results.next() {
                None if !self.did_query => {
                    // we haven't even done a query yet, so this is our very first time in
                    return ExecState::Eof;
                }
                None | Some(_) if self.found >= self.limit => {
                    // we found all the matching rows
                    return ExecState::Eof;
                }
                Some((scored, doc_address)) => {
                    self.nresults += 1;
                    return ExecState::RequiresVisibilityCheck {
                        ctid: scored.ctid,
                        score: scored.bm25,
                        doc_address,
                    };
                }
                None => {
                    // Fall through to query more results.
                }
            }

            // set the chunk size to the scaling factor times the limit
            self.chunk_size = (self.chunk_size * SUBSEQUENT_RETRY_SCALE_FACTOR)
                .max((self.limit as f64 * self.scale_factor) as usize)
                .min(MAX_CHUNK_SIZE);

            // Then try querying again, and continue looping if we got more results.
            if !self.query(state) {
                return ExecState::Eof;
            }
        }
    }

    fn reset(&mut self, state: &mut PdbScanState) {
        // Reset state
        self.claimed_segments.take();
        self.did_query = false;
        self.exhausted = false;
        self.search_query_input = Some(state.search_query_input().clone());
        self.search_reader = state.search_reader.clone();
        self.search_results = TopNSearchResults::empty();

        // Reset counters - excluding nresults which tracks processed results
        self.chunk_size = 0;
        self.found = 0;
        self.offset = 0;
    }
}
