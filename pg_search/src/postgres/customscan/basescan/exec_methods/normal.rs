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

use crate::index::reader::index::MultiSegmentSearchResults;
use crate::postgres::customscan::basescan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::basescan::scan_state::BaseScanState;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::u64_to_item_pointer;
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys;

pub struct NormalScanExecState {
    can_use_visibility_map: bool,
    heaprel: Option<PgSearchRelation>,
    slot: *mut pg_sys::TupleTableSlot,

    search_results: Option<MultiSegmentSearchResults>,

    did_query: bool,
}

impl Default for NormalScanExecState {
    fn default() -> Self {
        Self {
            can_use_visibility_map: false,
            heaprel: None,
            slot: std::ptr::null_mut(),
            search_results: None,
            did_query: false,
        }
    }
}

impl ExecMethod for NormalScanExecState {
    fn init(&mut self, state: &mut BaseScanState, cstate: *mut pg_sys::CustomScanState) {
        unsafe {
            self.heaprel = state.heaprel.clone();
            self.slot = pg_sys::MakeTupleTableSlot(
                (*cstate).ss.ps.ps_ResultTupleDesc,
                &pg_sys::TTSOpsVirtual,
            );
            self.can_use_visibility_map = state.targetlist_len == 0;
        }
    }

    fn uses_visibility_map(&self, state: &BaseScanState) -> bool {
        // if we don't return any actual fields, then we'll use the visibility map
        state.targetlist_len == 0
    }

    fn query(&mut self, state: &mut BaseScanState) -> bool {
        if self.did_query {
            return false;
        }

        let search_reader = state.search_reader.as_ref().unwrap();

        self.search_results = if let Some(parallel_state) = state.parallel_state {
            // NormalScanExecState evaluates isolated batches directly, so it does not participate
            // in global statistics planning for `estimated_rows`. Thus, we pass 0 here.
            Some(search_reader.search_lazy(parallel_state, 0))
        } else {
            // not parallel, first time query
            Some(search_reader.search())
        };

        self.did_query = true;
        true
    }

    fn internal_next(&mut self, state: &mut BaseScanState) -> ExecState {
        match self.search_results.as_mut().and_then(|r| r.next()) {
            None => ExecState::Eof,

            // we have a row, and we're set up such that we can check it with the visibility map
            Some((scored, doc_address)) if self.can_use_visibility_map => unsafe {
                let mut tid = pg_sys::ItemPointerData::default();
                u64_to_item_pointer(scored.ctid, &mut tid);

                let blockno = item_pointer_get_block_number(&tid);
                // We only use `is_block_all_visible` here to determine if we can emit a virtual
                // tuple. If not all rows on the block are visible, we return `FromHeap`, which
                // will cause the consumer to perform a full visibility check by fetching from
                // the heap.
                let is_visible = state.visibility_checker().is_block_all_visible(blockno);

                if is_visible {
                    // everything on this block is visible

                    let slot = self.slot;
                    (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
                    (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
                    (*slot).tts_nvalid = 0;

                    ExecState::Virtual { slot }
                } else {
                    // some rows on this block are not visible, so we need to fetch it from
                    // the heap to do a proper visibility check
                    ExecState::FromHeap {
                        ctid: scored.ctid,
                        score: scored.bm25,
                        doc_address,
                    }
                }
            },

            // otherwise we'll always fetch from the heap
            Some((scored, doc_address)) => ExecState::FromHeap {
                ctid: scored.ctid,
                score: scored.bm25,
                doc_address,
            },
        }
    }

    fn reset(&mut self, _state: &mut BaseScanState) {
        self.did_query = false;
        self.search_results = None;
    }
}
