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

use std::sync::Arc;

use tantivy::{DocAddress, DocId, SegmentOrdinal};

use crate::index::fast_fields_helper::{FFHelper, TidCache, TidReader};
use crate::index::reader::index::MultiSegmentSearchResults;
use crate::postgres::customscan::basescan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::basescan::scan_state::BaseScanState;
use crate::postgres::customscan::limit_offset::LimitOffset;
use crate::postgres::rel::PgSearchRelation;
use pgrx::pg_sys;

const BATCH_SIZE: usize = 8192;

#[derive(Clone, Copy, Debug)]
enum PreparedItem {
    Virtual,
    FromHeap {
        ctid: u64,
        score: f32,
        doc_address: DocAddress,
    },
}

pub struct NormalScanExecState {
    limit_offset: Option<LimitOffset>,
    limit: Option<usize>,
    emitted: usize,
    last_emitted: usize,
    batch_scale: usize,

    can_use_visibility_map: bool,
    heaprel: Option<PgSearchRelation>,
    slot: *mut pg_sys::TupleTableSlot,

    search_results: Option<MultiSegmentSearchResults>,

    did_query: bool,
    /// Cached per-segment ctid fast-field reader.
    ctid_cache: TidCache,
    /// Cached (segment_ord, is_all_visible) to avoid re-proving across docs in the same segment.
    segment_all_visible: Option<(SegmentOrdinal, bool)>,

    prepared_batch: Vec<PreparedItem>,
    batch_idx: usize,

    batch_doc_ids: Vec<DocId>,
    batch_scores: Vec<f32>,
    batch_mask: Vec<bool>,
    batch_ctids: Vec<Option<u64>>,
    batch_scratch: Vec<Option<u64>>,
}

impl NormalScanExecState {
    pub fn new(limit_offset: Option<LimitOffset>) -> Self {
        Self {
            limit_offset,
            limit: None,
            emitted: 0,
            last_emitted: 0,
            batch_scale: 1,
            can_use_visibility_map: false,
            heaprel: None,
            slot: std::ptr::null_mut(),
            search_results: None,
            did_query: false,
            ctid_cache: None,
            segment_all_visible: None,
            prepared_batch: Vec::with_capacity(BATCH_SIZE),
            batch_idx: 0,
            batch_doc_ids: Vec::with_capacity(BATCH_SIZE),
            batch_scores: Vec::with_capacity(BATCH_SIZE),
            batch_mask: Vec::with_capacity(BATCH_SIZE),
            batch_ctids: Vec::with_capacity(BATCH_SIZE),
            batch_scratch: Vec::with_capacity(BATCH_SIZE),
        }
    }

    fn refill_batch(&mut self, state: &mut BaseScanState) -> bool {
        self.prepared_batch.clear();
        self.batch_idx = 0;

        loop {
            if self.emitted == self.last_emitted {
                self.batch_scale = (self.batch_scale * 2).min(BATCH_SIZE);
            } else {
                self.batch_scale = 1;
                self.last_emitted = self.emitted;
            }

            let needed = match self.limit {
                Some(lim) => {
                    let rem = lim.saturating_sub(self.emitted);
                    if rem == 0 {
                        BATCH_SIZE
                    } else {
                        (rem * self.batch_scale).clamp(rem, BATCH_SIZE)
                    }
                }
                None => BATCH_SIZE,
            };

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
                    while self.batch_doc_ids.len() < needed {
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
            if state.visibility_checker().ffhelper().is_none() {
                let ffhelper = Arc::new(FFHelper::for_ctid(state.search_reader.as_ref().unwrap()));
                state.visibility_checker().set_ffhelper(ffhelper);
            }

            let is_all_vis = match self.segment_all_visible {
                Some((cur_ord, all_vis)) if cur_ord == seg_ord => all_vis,
                _ => {
                    let all_vis = state
                        .visibility_checker()
                        .is_segment_all_visible_ord(seg_ord);
                    self.segment_all_visible = Some((seg_ord, all_vis));
                    all_vis
                }
            };

            if is_all_vis {
                self.prepared_batch.resize(count, PreparedItem::Virtual);
            } else {
                self.batch_mask.resize(count, true);
                state.visibility_checker().check_segment_docs_mask(
                    seg_ord,
                    &self.batch_doc_ids,
                    &mut self.batch_mask,
                );
                for &is_visible in &self.batch_mask {
                    if is_visible {
                        self.prepared_batch.push(PreparedItem::Virtual);
                    }
                }
            }
        } else {
            if self.ctid_cache.as_ref().is_none_or(|(o, _)| *o != seg_ord) {
                let segment_reader = state
                    .search_reader
                    .as_ref()
                    .unwrap()
                    .searcher()
                    .segment_reader(seg_ord);
                self.ctid_cache = Some((
                    seg_ord,
                    TidReader::open(segment_reader).expect("ctid columns should be present"),
                ));
            }

            let (_, tid_reader) = self.ctid_cache.as_ref().unwrap();
            self.batch_ctids.resize(count, None);
            self.batch_ctids.fill(None);
            tid_reader.as_u64s(
                &self.batch_doc_ids,
                &mut self.batch_ctids,
                &mut self.batch_scratch,
            );

            for (i, maybe_ctid) in self.batch_ctids.iter().enumerate() {
                if let Some(ctid) = *maybe_ctid {
                    self.prepared_batch.push(PreparedItem::FromHeap {
                        ctid,
                        score: self.batch_scores[i],
                        doc_address: DocAddress {
                            segment_ord: seg_ord,
                            doc_id: self.batch_doc_ids[i],
                        },
                    });
                }
            }
        }
    }
}

impl Default for NormalScanExecState {
    fn default() -> Self {
        Self::new(None)
    }
}

impl ExecMethod for NormalScanExecState {
    fn init(&mut self, state: &mut BaseScanState, cstate: *mut pg_sys::CustomScanState) {
        let cstate_ref = unsafe { &*cstate };
        self.heaprel = state.heaprel.clone();
        self.slot = unsafe {
            pg_sys::MakeTupleTableSlot(cstate_ref.ss.ps.ps_ResultTupleDesc, &pg_sys::TTSOpsVirtual)
        };
        // Use the visibility map only when we have no columns to project AND no
        // executor-level quals (e.g. RLS SubPlan expressions). Virtual slots lack
        // the full scan tuple descriptor that ExecQual requires.
        self.can_use_visibility_map = state.targetlist_len == 0 && cstate_ref.ss.ps.qual.is_null();

        let estate = unsafe { (*cstate).ss.ps.state };
        self.limit = self.limit_offset.as_ref().and_then(|lo| lo.resolve(estate));
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
                self.emitted += 1;
                let slot = self.slot;
                unsafe {
                    let slot = &mut *slot;
                    slot.tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
                    slot.tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
                    slot.tts_nvalid = 0;
                }
                ExecState::Virtual { slot }
            }
            PreparedItem::FromHeap {
                ctid,
                score,
                doc_address,
            } => ExecState::FromHeap {
                ctid,
                score,
                doc_address,
            },
        }
    }

    fn increment_visible(&mut self) {
        self.emitted += 1;
    }

    fn reset(&mut self, _state: &mut BaseScanState) {
        self.did_query = false;
        self.search_results = None;
        self.ctid_cache = None;
        self.segment_all_visible = None;
        self.prepared_batch.clear();
        self.batch_idx = 0;
        self.emitted = 0;
        self.last_emitted = 0;
        self.batch_scale = 1;
    }
}
