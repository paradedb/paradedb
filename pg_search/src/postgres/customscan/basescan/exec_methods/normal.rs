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

use tantivy::{DocId, SegmentOrdinal};

use crate::index::reader::index::MultiSegmentSearchResults;
use crate::postgres::customscan::basescan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::basescan::scan_state::BaseScanState;
use pgrx::pg_sys;

const BATCH_SIZE: usize = 8192;

#[derive(Clone, Copy, Debug)]
enum PreparedItem {
    Virtual,
    FromHeap { ctid: u64, score: f32 },
}

pub struct NormalScanExecState {
    can_use_visibility_map: bool,
    slot: *mut pg_sys::TupleTableSlot,

    search_results: Option<MultiSegmentSearchResults>,

    did_query: bool,

    prepared_batch: Vec<PreparedItem>,
    batch_idx: usize,

    batch_doc_ids: Vec<DocId>,
    batch_scores: Vec<f32>,
    batch_mask: Vec<bool>,
    batch_ctids: Vec<Option<u64>>,
}

impl Default for NormalScanExecState {
    fn default() -> Self {
        Self {
            can_use_visibility_map: false,
            slot: std::ptr::null_mut(),
            search_results: None,
            did_query: false,
            prepared_batch: Vec::with_capacity(BATCH_SIZE),
            batch_idx: 0,
            batch_doc_ids: Vec::with_capacity(BATCH_SIZE),
            batch_scores: Vec::with_capacity(BATCH_SIZE),
            batch_mask: Vec::with_capacity(BATCH_SIZE),
            batch_ctids: Vec::with_capacity(BATCH_SIZE),
        }
    }
}

impl NormalScanExecState {
    fn refill_batch(&mut self, state: &mut BaseScanState) -> bool {
        self.prepared_batch.clear();
        self.batch_idx = 0;

        loop {
            self.batch_doc_ids.clear();
            self.batch_scores.clear();

            let seg_ord_opt = {
                let Some(results) = self.search_results.as_mut() else {
                    return false;
                };

                let mut seg_ord = None;
                while let Some(seg) = results.current_segment() {
                    let ord = seg.segment_ord();
                    seg_ord = Some(ord);
                    let mut exhausted = false;
                    while self.batch_doc_ids.len() < BATCH_SIZE {
                        if let Some((score, doc_address)) = seg.next() {
                            self.batch_doc_ids.push(doc_address.doc_id);
                            self.batch_scores.push(score);
                        } else {
                            exhausted = true;
                            break;
                        }
                    }
                    if exhausted {
                        results.current_segment_pop();
                    }
                    if !self.batch_doc_ids.is_empty() {
                        break;
                    }
                }
                seg_ord
            };

            let Some(seg_ord) = seg_ord_opt else {
                return false;
            };

            if self.batch_doc_ids.is_empty() {
                return false;
            }

            self.process_batch(state, seg_ord);

            if !self.prepared_batch.is_empty() {
                self.batch_idx = 0;
                return true;
            }
        }
    }

    fn process_batch(&mut self, state: &mut BaseScanState, seg_ord: SegmentOrdinal) {
        let count = self.batch_doc_ids.len();
        if count == 0 {
            return;
        }

        if self.can_use_visibility_map {
            let mask = state.visibility_checker().check_segment_docs_mask(
                seg_ord,
                &self.batch_doc_ids,
                &mut self.batch_mask,
            );
            self.prepared_batch
                .resize(mask.count_visible(), PreparedItem::Virtual);
        } else {
            let ffhelper = state.visibility_checker().ffhelper().cloned().unwrap();
            self.batch_ctids.resize(count, None);
            self.batch_ctids.fill(None);
            ffhelper
                .ctid(seg_ord)
                .as_u64s(&self.batch_doc_ids, &mut self.batch_ctids);

            for (i, maybe_ctid) in self.batch_ctids.iter().enumerate() {
                let ctid = maybe_ctid.expect("doc in search index must have a ctid");
                self.prepared_batch.push(PreparedItem::FromHeap {
                    ctid,
                    score: self.batch_scores[i],
                });
            }
        }
    }
}

impl ExecMethod for NormalScanExecState {
    fn init(&mut self, state: &mut BaseScanState, cstate: *mut pg_sys::CustomScanState) {
        let cstate_ref = unsafe { &*cstate };
        self.slot = unsafe {
            pg_sys::MakeTupleTableSlot(cstate_ref.ss.ps.ps_ResultTupleDesc, &pg_sys::TTSOpsVirtual)
        };
        // Use the visibility map only when we have no columns to project AND no
        // executor-level quals (e.g. RLS SubPlan expressions). Virtual slots lack
        // the full scan tuple descriptor that ExecQual requires.
        self.can_use_visibility_map = state.targetlist_len == 0 && cstate_ref.ss.ps.qual.is_null();
    }

    fn uses_visibility_map(&self, state: &BaseScanState) -> bool {
        state.targetlist_len == 0
    }

    fn query(&mut self, state: &mut BaseScanState) -> bool {
        if self.did_query {
            return false;
        }

        let search_reader = state.search_reader.as_ref().unwrap();

        self.search_results = if let Some(parallel_state) = state.parallel_state() {
            // NormalScanExecState evaluates isolated batches directly, so it does not participate
            // in global statistics planning for `estimated_rows`. Thus, we pass 0 here.
            Some(search_reader.search_lazy(parallel_state, None, 0))
        } else {
            // not parallel, first time query
            Some(search_reader.search())
        };

        self.did_query = true;
        true
    }

    fn internal_next(&mut self, state: &mut BaseScanState) -> ExecState {
        pgrx::check_for_interrupts!();

        if self.batch_idx >= self.prepared_batch.len() && !self.refill_batch(state) {
            return ExecState::Eof;
        }

        let item = self.prepared_batch[self.batch_idx];
        self.batch_idx += 1;

        match item {
            PreparedItem::Virtual => {
                let slot = self.slot;
                unsafe {
                    let slot = &mut *slot;
                    slot.tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
                    slot.tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
                    slot.tts_nvalid = 0;
                }
                ExecState::Virtual { slot }
            }
            PreparedItem::FromHeap { ctid, score } => ExecState::FromHeap { ctid, score },
        }
    }

    fn reset(&mut self, _state: &mut BaseScanState) {
        self.did_query = false;
        self.search_results = None;
        self.prepared_batch.clear();
        self.batch_idx = 0;
    }
}
