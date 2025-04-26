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

use crate::index::reader::index::SearchResults;
use crate::index::reader::index::SortDirection as IndexSortDirection;
use crate::postgres::customscan::builders::custom_path::SortDirection;
use crate::postgres::customscan::pdbscan::debug_document_id;
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
            pgrx::warning!("NormalScanExecState::init: Initializing scan state");
            self.heaprel = state.heaprel.unwrap();
            self.slot = pg_sys::MakeTupleTableSlot(
                (*cstate).ss.ps.ps_ResultTupleDesc,
                &pg_sys::TTSOpsVirtual,
            );
            self.can_use_visibility_map = state.targetlist_len == 0;
            pgrx::warning!(
                "NormalScanExecState::init: Initialized with can_use_visibility_map={}",
                self.can_use_visibility_map
            );
        }
    }

    fn uses_visibility_map(&self, state: &PdbScanState) -> bool {
        // if we don't return any actual fields, then we'll use the visibility map
        let uses_vm = state.targetlist_len == 0;
        pgrx::warning!(
            "NormalScanExecState::uses_visibility_map: targetlist_len={}, using VM={}",
            state.targetlist_len,
            uses_vm
        );
        uses_vm
    }

    fn query(&mut self, state: &mut PdbScanState) -> bool {
        pgrx::warning!(
            "NormalScanExecState::query: Starting, parallel_state={:?}, did_query={}, rti={:?}",
            state.parallel_state.is_some(),
            self.did_query,
            state.rti
        );

        if let Some(parallel_state) = state.parallel_state {
            if let Some(segment_id) = unsafe { checkout_segment(parallel_state) } {
                pgrx::warning!(
                    "NormalScanExecState::query: Parallel mode, processing segment_id={} for rti={:?}",
                    segment_id,
                    state.rti
                );

                // First get segment results
                pgrx::warning!(
                    "NormalScanExecState::query: Searching segment {} with limit={:?} for rti={:?}",
                    segment_id,
                    state.limit,
                    state.rti
                );

                // Critical fix: For CTE-based queries, we need completely deterministic ordering
                // When multiple parallel workers process segments independently, use search_top_n_in_segment
                // instead of search_segment to ensure deterministic sorting within each segment
                pgrx::warning!("NormalScanExecState::query: Using deterministic search_top_n_in_segment for CTE compatibility");

                // Use a large default limit to capture all results, but respect any actual limit
                // For CTE queries, using a sufficiently large limit ensures we process all possible matches
                let large_limit = state.limit.unwrap_or(100000); // Increased limit for better coverage

                self.search_results = state
                    .search_reader
                    .as_ref()
                    .unwrap()
                    .search_top_n_in_segment(
                        segment_id,
                        &state.search_query_input,
                        None, // No sort field, use score-based sorting
                        crate::index::reader::index::SortDirection::Desc, // Sort by score desc
                        large_limit,
                        state.need_scores(),
                    );

                // Log details about the results
                match &self.search_results {
                    SearchResults::TopNByScore(_, _, results) => {
                        let count = results.len();
                        pgrx::warning!(
                            "NormalScanExecState::query: Segment {} returned TopNByScore with {} results for rti={:?}",
                            segment_id, count, state.rti
                        );

                        // Clone the results to log details without consuming the iterator
                        let mut results_clone = results.clone();
                        for i in 0..std::cmp::min(10, results_clone.len()) {
                            if let Some((score, doc_address)) = results_clone.next() {
                                pgrx::warning!(
                                    "NormalScanExecState::query: Result #{}: score={}, segment={}, doc_id={}",
                                    i, score, doc_address.segment_ord, doc_address.doc_id
                                );
                            }
                        }
                    }
                    SearchResults::TopNByTweakedScore(_, _, results) => {
                        let count = results.len();
                        pgrx::warning!(
                            "NormalScanExecState::query: Segment {} returned TopNByTweakedScore with {} results for rti={:?}",
                            segment_id, count, state.rti
                        );

                        // Clone the results to log details without consuming the iterator
                        let mut results_clone = results.clone();
                        for i in 0..std::cmp::min(10, results_clone.len()) {
                            if let Some((tweaked_score, doc_address)) = results_clone.next() {
                                pgrx::warning!(
                                    "NormalScanExecState::query: Result #{}: tweaked_score={:?}, segment={}, doc_id={}",
                                    i, tweaked_score, doc_address.segment_ord, doc_address.doc_id
                                );
                            }
                        }
                    }
                    SearchResults::None => {
                        pgrx::warning!(
                            "NormalScanExecState::query: Segment {} returned no results for rti={:?}",
                            segment_id, state.rti
                        );
                    }
                    _ => {
                        pgrx::warning!(
                            "NormalScanExecState::query: Segment {} returned other result type for rti={:?}",
                            segment_id, state.rti
                        );
                    }
                }

                pgrx::warning!(
                    "NormalScanExecState::query: Executed segment search, returning true"
                );
                return true;
            }

            // no more segments to query
            pgrx::warning!(
                "NormalScanExecState::query: No more segments to query for rti={:?}",
                state.rti
            );
            self.search_results = SearchResults::None;
            false
        } else if self.did_query {
            // not parallel, so we're done
            pgrx::warning!(
                "NormalScanExecState::query: Not parallel and already queried, returning false for rti={:?}",
                state.rti
            );
            false
        } else {
            // not parallel, first time query
            pgrx::warning!(
                "NormalScanExecState::query: Not parallel, first time query, calling do_query for rti={:?}",
                state.rti
            );
            let result = self.do_query(state);
            pgrx::warning!(
                "NormalScanExecState::query: do_query result={} for rti={:?}",
                result,
                state.rti
            );
            result
        }
    }

    fn internal_next(&mut self, state: &mut PdbScanState) -> ExecState {
        pgrx::warning!(
            "NormalScanExecState::internal_next: Getting next result for rti={:?}",
            state.rti
        );

        match self.search_results.next() {
            // no more rows
            None => {
                pgrx::warning!("NormalScanExecState::internal_next: No more results, returning Eof for rti={:?}", state.rti);
                ExecState::Eof
            }

            // we have a row, and we're set up such that we can check it with the visibility map
            Some((scored, doc_address)) if self.can_use_visibility_map => unsafe {
                // Get document ID for debugging
                let doc_id_info = if let Some(reader) = state.search_reader.as_ref() {
                    debug_document_id(&reader.searcher(), doc_address)
                } else {
                    "no reader available".to_string()
                };

                pgrx::warning!(
                    "NormalScanExecState::internal_next: Got result with ctid={}, score={}, {}, can use VM for rti={:?}",
                    scored.ctid, scored.bm25, doc_id_info, state.rti
                );

                let mut tid = pg_sys::ItemPointerData::default();
                u64_to_item_pointer(scored.ctid, &mut tid);

                let blockno = item_pointer_get_block_number(&tid);
                let offno = pg_sys::ItemPointerGetOffsetNumber(&tid);
                pgrx::warning!(
                    "NormalScanExecState::internal_next: Processing tuple at block={}, offset={}, segment={}, doc_id={} for rti={:?}",
                    blockno, offno, doc_address.segment_ord, doc_address.doc_id, state.rti
                );

                let is_visible = if blockno == self.blockvis.0 {
                    // we know the visibility of this block because we just checked it last time
                    pgrx::warning!(
                        "NormalScanExecState::internal_next: Reusing block visibility for rti={:?}",
                        state.rti
                    );
                    self.blockvis.1
                } else {
                    // new block so check its visibility
                    pgrx::warning!("NormalScanExecState::internal_next: Checking new block visibility for rti={:?}", state.rti);
                    self.blockvis.0 = blockno;
                    self.blockvis.1 = is_block_all_visible(self.heaprel, &mut self.vmbuff, blockno);
                    pgrx::warning!(
                        "NormalScanExecState::internal_next: Block visibility={} for rti={:?}",
                        self.blockvis.1,
                        state.rti
                    );
                    self.blockvis.1
                };

                if is_visible {
                    // everything on this block is visible
                    pgrx::warning!("NormalScanExecState::internal_next: Block is visible, returning Virtual state for rti={:?}", state.rti);

                    let slot = self.slot;
                    (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
                    (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
                    (*slot).tts_nvalid = 0;

                    ExecState::Virtual { slot }
                } else {
                    // not sure about the block visibility so the tuple requires a heap check
                    pgrx::warning!(
                        "NormalScanExecState::internal_next: Block visibility uncertain, returning RequiresVisibilityCheck for ctid={}, score={}, rti={:?}",
                        scored.ctid, scored.bm25, state.rti
                    );
                    ExecState::RequiresVisibilityCheck {
                        ctid: scored.ctid,
                        score: scored.bm25,
                        doc_address,
                    }
                }
            },

            // we have a row, but we can't use the visibility map
            Some((scored, doc_address)) => {
                // Get document ID for debugging
                let doc_id_info = if let Some(reader) = state.search_reader.as_ref() {
                    debug_document_id(&reader.searcher(), doc_address)
                } else {
                    "no reader available".to_string()
                };

                pgrx::warning!(
                    "NormalScanExecState::internal_next: Got result with ctid={}, score={}, {}, can't use VM for rti={:?}",
                    scored.ctid, scored.bm25, doc_id_info, state.rti
                );

                // Convert to heap tuple coordinates for better logging
                let mut tid = pg_sys::ItemPointerData::default();
                u64_to_item_pointer(scored.ctid, &mut tid);
                unsafe {
                    let blockno = item_pointer_get_block_number(&tid);
                    let offno = pg_sys::ItemPointerGetOffsetNumber(&tid);
                    pgrx::warning!(
                        "NormalScanExecState::internal_next: Processing tuple at block={}, offset={}, segment={}, doc_id={} for rti={:?}",
                        blockno, offno, doc_address.segment_ord, doc_address.doc_id, state.rti
                    );
                }

                ExecState::RequiresVisibilityCheck {
                    ctid: scored.ctid,
                    score: scored.bm25,
                    doc_address,
                }
            }
        }
    }
}

impl NormalScanExecState {
    #[inline(always)]
    fn do_query(&mut self, state: &PdbScanState) -> bool {
        pgrx::warning!(
            "NormalScanExecState::do_query: Starting query execution, did_query={}, rti={:?}",
            self.did_query,
            state.rti
        );
        if self.did_query {
            pgrx::warning!("NormalScanExecState::do_query: Query already executed, returning false for rti={:?}", state.rti);
            return false;
        }

        pgrx::warning!(
            "NormalScanExecState::do_query: Executing search with need_scores={}, limit={:?}, query={:?} for rti={:?}",
            state.need_scores(),
            state.limit,
            &state.search_query_input,
            state.rti
        );

        let reader = state
            .search_reader
            .as_ref()
            .expect("must have a search_reader to do a query");

        // For CTEs and other complex queries, need completely deterministic ordering
        // Use search_top_n directly rather than search + merge for consistent behavior
        // with the parallel mode
        pgrx::warning!(
            "NormalScanExecState::do_query: Using deterministic search_top_n with limit={:?} for rti={:?}",
            state.limit, state.rti
        );

        // Use a large default limit to ensure we get all results, but respect any actual limit
        // For CTE queries, using a sufficiently large limit ensures we process all possible matches
        let large_limit = state.limit.unwrap_or(100000); // Increased limit for better coverage

        // Use search_top_n directly for deterministic ordering by score and then by ctid
        self.search_results = reader.search_top_n(
            &state.search_query_input,
            None, // No sort field, use score-based sorting
            crate::index::reader::index::SortDirection::Desc,
            large_limit,
            state.need_scores(),
        );

        // Log search results initial state
        match &self.search_results {
            SearchResults::None => {
                pgrx::warning!(
                    "NormalScanExecState::do_query: Search returned no results for rti={:?}",
                    state.rti
                );
            }
            SearchResults::TopNByScore(_, _, results) => {
                let count = results.len();
                pgrx::warning!(
                    "NormalScanExecState::do_query: Search returned TopNByScore with {} results for rti={:?}",
                    count, state.rti
                );

                // Log first few results for debugging with clone
                let mut results_clone = results.clone();
                for i in 0..std::cmp::min(10, results_clone.len()) {
                    if let Some((score, doc_address)) = results_clone.next() {
                        pgrx::warning!(
                            "NormalScanExecState::do_query: Result #{} - score={}, segment={}, doc_id={} for rti={:?}",
                            i, score, doc_address.segment_ord, doc_address.doc_id, state.rti
                        );
                    }
                }
            }
            SearchResults::TopNByTweakedScore(_, _, results) => {
                let count = results.len();
                pgrx::warning!(
                    "NormalScanExecState::do_query: Search returned TopNByTweakedScore with {} results for rti={:?}",
                    count, state.rti
                );

                // Log first few results for debugging with clone
                let mut results_clone = results.clone();
                for i in 0..std::cmp::min(10, results_clone.len()) {
                    if let Some((tweaked_score, doc_address)) = results_clone.next() {
                        pgrx::warning!(
                            "NormalScanExecState::do_query: Result #{} - tweaked_score={:?}, segment={}, doc_id={} for rti={:?}",
                            i, tweaked_score, doc_address.segment_ord, doc_address.doc_id, state.rti
                        );
                    }
                }
            }
            SearchResults::TopNByField(_, _, _) => {
                pgrx::warning!("NormalScanExecState::do_query: Search returned TopNByField results for rti={:?}", state.rti);
            }
            SearchResults::SingleSegment(_, _, _, _) => {
                pgrx::warning!("NormalScanExecState::do_query: Search returned SingleSegment results for rti={:?}", state.rti);
            }
            SearchResults::AllSegments(_, _, iters) => {
                pgrx::warning!(
                    "NormalScanExecState::do_query: Search returned AllSegments with {} iterators for rti={:?}",
                    iters.len(), state.rti
                );
            }
        }

        self.did_query = true;
        pgrx::warning!(
            "NormalScanExecState::do_query: Query execution completed, returning true for rti={:?}",
            state.rti
        );
        true
    }
}
