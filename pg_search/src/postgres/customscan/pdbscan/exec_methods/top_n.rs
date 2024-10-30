// Copyright (c) 2023-2024 Retake, Inc.
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

use crate::index::reader::{SearchIndexReader, SearchResults};
use crate::index::SearchIndex;
use crate::postgres::customscan::builders::custom_path::SortDirection;
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use pgrx::{direct_function_call, pg_sys, IntoDatum};
use tantivy::query::{Query, QueryClone};

// TODO:  should these be GUCs?  I think yes, probably
const SUBSEQUENT_RETRY_SCALE_FACTOR: usize = 2;
const MAX_CHUNK_SIZE: usize = 5000;

#[derive(Default)]
pub struct TopNScanExecState {
    // required
    heaprelid: pg_sys::Oid,
    limit: usize,
    sort_direction: SortDirection,

    // set during init
    have_less: bool,
    query: Option<Box<dyn Query>>,
    search_reader: Option<SearchIndexReader>,
    sort_field: Option<String>,
    search_results: SearchResults,

    // state tracking
    last_ctid: u64,
    found: usize,
    chunk_size: usize,
    retry_count: usize,
}

impl TopNScanExecState {
    pub fn new(heaprelid: pg_sys::Oid, limit: usize, sort_direction: SortDirection) -> Self {
        Self {
            heaprelid,
            limit,
            sort_direction,
            ..Default::default()
        }
    }
}

impl ExecMethod for TopNScanExecState {
    fn init(&mut self, state: &PdbScanState, _cstate: *mut pg_sys::CustomScanState) {
        let sort_field = state.sort_field.clone();
        let search_reader = state.search_reader.as_ref().unwrap();
        let query = state.query.as_ref().map(|q| q.box_clone());

        self.query = query;
        self.sort_field = sort_field;
        self.search_results = search_reader.search_top_n(
            SearchIndex::executor(),
            self.query.as_ref().unwrap(),
            self.sort_field.clone(),
            self.sort_direction.into(),
            self.limit,
        );

        let len = self
            .search_results
            .len()
            .expect("search_results should not be empty");

        self.have_less = len < self.limit;
        self.search_reader = state.search_reader.clone();
    }

    fn next(&mut self) -> ExecState {
        unsafe {
            let mut next = self.search_results.next();
            loop {
                match next {
                    None => {
                        if self.found == self.limit || self.have_less {
                            // we found all the matching rows
                            return ExecState::Eof;
                        }
                    }
                    Some((scored, doc_address)) => {
                        return ExecState::RequiresVisibilityCheck {
                            ctid: scored.ctid,
                            score: scored.bm25,
                            doc_address,
                        }
                    }
                }

                // we underflowed our tuples, so go get some more, if there are any
                self.retry_count += 1;

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

                let mut results = self.search_reader.as_ref().unwrap().search_top_n(
                    SearchIndex::executor(),
                    self.query.as_ref().unwrap(),
                    self.sort_field.clone(),
                    self.sort_direction.into(),
                    self.chunk_size,
                );

                // fast forward and stop on the ctid we last found
                for (scored, doc_address) in &mut results {
                    if scored.ctid == self.last_ctid {
                        // we've now advanced to the last ctid we found
                        break;
                    }
                }

                // this should be the next valid tuple after that
                next = match results.next() {
                    // ... and there it is!
                    Some(next) => Some(next),

                    // there wasn't one, so we've now read all possible matches
                    None => {
                        return ExecState::Eof;
                    }
                };

                // we now have a new iterator of results to use going forward
                self.search_results = results;

                // but we'll loop back around and evaluate whatever `next` is now pointing to
                continue;
            }
        }
    }
}
