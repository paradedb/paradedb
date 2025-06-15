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

use std::iter::Peekable;

use crate::api::FieldName;
use crate::index::reader::index::{SearchIndexReader, SearchResults};
use crate::postgres::customscan::builders::custom_path::SortDirection;
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::parallel::checkout_my_segment_block;
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
    heaprelid: pg_sys::Oid,
    limit: usize,
    sort_direction: SortDirection,
    need_scores: bool,

    // set during init
    search_query_input: Option<SearchQueryInput>,
    search_reader: Option<SearchIndexReader>,
    sort_field: Option<FieldName>,

    // state tracking
    search_results: Peekable<SearchResults>,
    nresults: usize,
    did_query: bool,
    found: usize,
    offset: usize,
    chunk_size: usize,
    // If parallel, the segments which have been claimed by this worker.
    claimed_segments: Vec<SegmentId>,
}

impl TopNScanExecState {
    pub fn new(
        heaprelid: pg_sys::Oid,
        limit: usize,
        sort_direction: SortDirection,
        need_scores: bool,
    ) -> Self {
        Self {
            heaprelid,
            limit,
            sort_direction,
            need_scores,
            search_query_input: None,
            search_reader: None,
            sort_field: None,
            search_results: SearchResults::None.peekable(),
            nresults: 0,
            did_query: false,
            found: 0,
            offset: 0,
            chunk_size: 0,
            claimed_segments: Default::default(),
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
    fn segments_to_query(
        &mut self,
        search_reader: &SearchIndexReader,
        parallel_state: Option<*mut ParallelScanState>,
    ) -> impl IntoIterator<Item = SegmentId> {
        if !self.claimed_segments.is_empty() {
            return self.claimed_segments.clone();
        }
        let segment_ids = match parallel_state {
            None => search_reader.segment_ids(),

            Some(parallel_state) => unsafe {
                let nworkers = {
                    let _mutex = (*parallel_state).acquire_mutex();
                    (*parallel_state).nlaunched()
                };

                checkout_my_segment_block(nworkers, parallel_state)
            },
        };

        self.claimed_segments = segment_ids;
        self.claimed_segments.clone()
    }
}

impl ExecMethod for TopNScanExecState {
    fn init(&mut self, state: &mut PdbScanState, _cstate: *mut pg_sys::CustomScanState) {
        let sort_field = state.sort_field.clone();

        self.search_query_input = Some(state.search_query_input().clone());
        self.sort_field = sort_field;
        self.search_reader = state.search_reader.clone();
    }

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

        if self.found >= self.limit {
            return false;
        }

        // We track the total number of queries executed by Top-N (for any of the above reasons).
        state.query_count += 1;

        // Calculate the limit for this query, and what the offset will be for the next query.
        let local_limit = self.limit.max(self.chunk_size);
        let next_offset = self.offset + local_limit;

        self.search_results = state
            .search_reader
            .as_ref()
            .unwrap()
            .search_top_n_in_segments(
                self.segments_to_query(state.search_reader.as_ref().unwrap(), state.parallel_state)
                    .into_iter(),
                self.search_query_input.as_ref().unwrap(),
                self.sort_field.clone(),
                self.sort_direction.into(),
                local_limit,
                self.offset,
                self.need_scores,
            )
            .peekable();

        // Record the offset to start from for the next query.
        self.offset = next_offset;

        self.search_results.peek().is_some()
    }

    fn increment_visible(&mut self) {
        self.found += 1;
    }

    fn internal_next(&mut self, state: &mut PdbScanState) -> ExecState {
        unsafe {
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

                // calculate a scaling factor to use against the limit
                let factor = if self.chunk_size == 0 {
                    // if we haven't done any chunking yet, calculate the scaling factor
                    // based on the proportion of dead tuples compared to live tuples
                    let heaprelid = self.heaprelid;
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

                    (1.0 + ((1.0 + n_dead as f64) / (1.0 + n_live as f64))).ceil() as usize
                } else {
                    // we've already done chunking, so just use a default scaling factor
                    // to avoid exponentially growing the chunk size
                    SUBSEQUENT_RETRY_SCALE_FACTOR
                };

                // set the chunk size to the scaling factor times the limit
                self.chunk_size = (self.chunk_size * factor)
                    .max(self.limit * factor)
                    .min(MAX_CHUNK_SIZE);

                // Then try querying again, and continue looping if we got more results.
                if !self.query(state) {
                    return ExecState::Eof;
                }
            }
        }
    }

    fn reset(&mut self, _state: &mut PdbScanState) {
        // Reset tracking state but don't clear search_results

        // Reset counters - excluding nresults which tracks processed results
        self.did_query = false;

        // Reset the tracking state
        self.chunk_size = 0;
        self.offset = 0;
        self.found = 0;
        self.claimed_segments.clear();
    }
}
