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

use crate::index::reader::SearchResults;
use crate::index::SearchIndex;
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use pgrx::pg_sys;

#[derive(Default)]
pub struct NormalScanExecState {
    search_results: SearchResults,
    did_query: bool,
}

impl ExecMethod for NormalScanExecState {
    fn init(&mut self, state: &PdbScanState, _cstate: *mut pg_sys::CustomScanState) {
        let search_reader = state.search_reader.as_ref().unwrap();
        let query = state.query.as_ref().unwrap();
    }

    fn query(&mut self, state: &PdbScanState) -> bool {
        self.do_query(state)
    }

    fn internal_next(&mut self) -> ExecState {
        match self.search_results.next() {
            None => ExecState::Eof,
            Some((scored, doc_address)) => ExecState::RequiresVisibilityCheck {
                ctid: scored.ctid,
                score: scored.bm25,
                doc_address,
            },
        }
    }
}

impl NormalScanExecState {
    #[inline(always)]
    fn do_query(&mut self, state: &PdbScanState) -> bool {
        if self.did_query {
            return false;
        }
        self.search_results = state.search_reader.as_ref().unwrap().search_via_channel(
            state.need_scores(),
            false,
            SearchIndex::executor(),
            state.query.as_ref().unwrap(),
            state.limit,
        );
        self.did_query = true;
        true
    }
}
