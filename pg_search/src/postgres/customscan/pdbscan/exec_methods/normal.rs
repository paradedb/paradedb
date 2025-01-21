// Copyright (c) 2023-2025 Retake, Inc.
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

use crate::index::reader::index::SearchResults;
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::is_block_all_visible;
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::postgres::utils::u64_to_item_pointer;
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys;

pub struct NormalScanExecState {
    can_use_visibility_map: bool,
    heaprel: pg_sys::Relation,
    slot: *mut pg_sys::TupleTableSlot,
    vmbuff: pg_sys::Buffer,

    search_results: SearchResults,

    // tracks our previous block visibility so we can elide checking again
    blockvis: (pg_sys::BlockNumber, bool),

    did_query: bool,
}

impl Default for NormalScanExecState {
    fn default() -> Self {
        Self {
            can_use_visibility_map: false,
            heaprel: std::ptr::null_mut(),
            slot: std::ptr::null_mut(),
            vmbuff: pg_sys::InvalidBuffer as pg_sys::Buffer,
            search_results: SearchResults::None,
            blockvis: (pg_sys::InvalidBlockNumber, false),
            did_query: false,
        }
    }
}

impl Drop for NormalScanExecState {
    fn drop(&mut self) {
        unsafe {
            if crate::postgres::utils::IsTransactionState()
                && self.vmbuff != pg_sys::InvalidBuffer as pg_sys::Buffer
            {
                pg_sys::ReleaseBuffer(self.vmbuff);
            }
        }
    }
}
impl ExecMethod for NormalScanExecState {
    fn init(&mut self, state: &mut PdbScanState, cstate: *mut pg_sys::CustomScanState) {
        unsafe {
            self.heaprel = state.heaprel.unwrap();
            self.slot = pg_sys::MakeTupleTableSlot(
                (*cstate).ss.ps.ps_ResultTupleDesc,
                &pg_sys::TTSOpsVirtual,
            );
            self.can_use_visibility_map = state.targetlist_len == 0;
        }
    }

    fn uses_visibility_map(&self, state: &PdbScanState) -> bool {
        // if we don't return any actual fields, then we'll use the visibility map
        state.targetlist_len == 0
    }

    fn query(&mut self, state: &mut PdbScanState) -> bool {
        if let Some(parallel_state) = state.parallel_state {
            if let Some(segment_ord) = unsafe { checkout_segment(parallel_state) } {
                self.search_results = state.search_reader.as_ref().unwrap().search_segment(
                    state.need_scores(),
                    segment_ord,
                    &state.search_query_input,
                );
                return true;
            }

            // no more segments to query
            self.search_results = SearchResults::None;
            false
        } else if self.did_query {
            // not parallel, so we're done
            false
        } else {
            // not parallel, first time query
            self.do_query(state);
            self.did_query = true;
            true
        }
    }

    fn internal_next(&mut self, _state: &mut PdbScanState) -> ExecState {
        match self.search_results.next() {
            // no more rows
            None => ExecState::Eof,

            // we have a row, and we're set up such that we can check it with the visibility map
            Some((scored, doc_address)) if self.can_use_visibility_map => unsafe {
                let mut tid = pg_sys::ItemPointerData::default();
                u64_to_item_pointer(scored.ctid, &mut tid);

                let blockno = item_pointer_get_block_number(&tid);
                let is_visible = if blockno == self.blockvis.0 {
                    // we know the visibility of this block because we just checked it last time
                    self.blockvis.1
                } else {
                    // new block so check its visibility
                    self.blockvis.0 = blockno;
                    self.blockvis.1 = is_block_all_visible(self.heaprel, &mut self.vmbuff, blockno);
                    self.blockvis.1
                };

                if is_visible {
                    // everything on this block is visible

                    let slot = self.slot;
                    (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
                    (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
                    (*slot).tts_nvalid = 0;

                    ExecState::Virtual { slot }
                } else {
                    // not sure about the block visibility so the tuple requires a heap check
                    ExecState::RequiresVisibilityCheck {
                        ctid: scored.ctid,
                        score: scored.bm25,
                        doc_address,
                    }
                }
            },

            // we have a row, but we can't use the visibility map
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
        self.search_results = state.search_reader.as_ref().unwrap().search(
            state.need_scores(),
            false,
            &state.search_query_input,
            state.limit,
        );
        self.did_query = true;
        true
    }
}
