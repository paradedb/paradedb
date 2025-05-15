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

use crate::index::reader::index::{SearchIndexReader, SearchResults};
use crate::postgres::customscan::builders::custom_path::SortDirection;
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::query::SearchQueryInput;
use pgrx::{check_for_interrupts, pg_sys};

#[derive(Default)]
pub struct TopNScanExecState {
    // required
    limit: usize,
    sort_direction: SortDirection,
    need_scores: bool,

    // set during init
    search_query_input: Option<SearchQueryInput>,
    search_reader: Option<SearchIndexReader>,
    sort_field: Option<String>,
    search_results: SearchResults,
    all_segments_queried: bool,
}

impl TopNScanExecState {
    pub fn new(limit: usize, sort_direction: SortDirection, need_scores: bool) -> Self {
        Self {
            limit,
            sort_direction,
            need_scores,
            ..Default::default()
        }
    }

    fn query_more_results(&mut self, state: &mut PdbScanState) -> SearchResults {
        if let Some(parallel_state) = state.parallel_state {
            // we're parallel, so go get a segment from the parallel state
            if let Some(segment_id) = unsafe { checkout_segment(parallel_state) } {
                let search_reader = state.search_reader.as_ref().unwrap();
                search_reader.search_top_n_in_segment(
                    segment_id,
                    self.search_query_input.as_ref().unwrap(),
                    state
                        .visibility_checker
                        .clone()
                        .expect("Must have VisibilityChecker at query time."),
                    self.sort_field.clone(),
                    self.sort_direction.into(),
                    self.limit,
                    self.need_scores,
                )
            } else {
                // no more segments to query
                self.all_segments_queried = true;
                SearchResults::None
            }
        } else if self.all_segments_queried {
            SearchResults::None
        } else {
            // not parallel, first time query
            let search_reader = &self.search_reader.as_ref().unwrap();
            self.all_segments_queried = true;
            search_reader.search_top_n(
                self.search_query_input.as_ref().unwrap(),
                state
                    .visibility_checker
                    .clone()
                    .expect("Must have VisibilityChecker at query time."),
                self.sort_field.clone(),
                self.sort_direction.into(),
                self.limit,
                self.need_scores,
            )
        }
    }
}

impl ExecMethod for TopNScanExecState {
    fn init(&mut self, state: &mut PdbScanState, _cstate: *mut pg_sys::CustomScanState) {
        let sort_field = state.sort_field.clone();

        self.search_query_input = Some(state.search_query_input.clone());
        self.sort_field = sort_field;
        self.search_reader = state.search_reader.clone();
    }

    fn query(&mut self, state: &mut PdbScanState) -> bool {
        let search_results = self.query_more_results(state);

        if matches!(search_results, SearchResults::None) {
            false
        } else {
            self.search_results = search_results;
            true
        }
    }

    fn internal_next(&mut self, state: &mut PdbScanState) -> ExecState {
        loop {
            check_for_interrupts!();

            match self.search_results.next() {
                None if self.all_segments_queried => {
                    return ExecState::Eof;
                }
                None => {
                    self.search_results = self.query_more_results(state);
                    continue;
                }
                Some((scored, doc_address)) => {
                    return ExecState::Ctid {
                        ctid: scored.ctid,
                        score: scored.bm25,
                        doc_address,
                    };
                }
            }
        }
    }

    fn reset(&mut self, _state: &mut PdbScanState) {
        // The search results iterator is mutable, and is consumed during the scan. During a
        // rescan, we must regenerate it. To do otherwise, we'd need to hold a buffer of all
        // results, rather then (or in addition to) an iterator.
        self.search_results = SearchResults::default();
        self.all_segments_queried = false;
    }
}
