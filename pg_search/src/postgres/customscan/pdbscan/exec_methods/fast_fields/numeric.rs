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

use crate::index::fast_fields_helper::WhichFastField;
use crate::index::reader::index::SearchResults;
use crate::postgres::customscan::pdbscan::exec_methods::fast_fields::{
    non_string_ff_to_datum, FastFieldExecState,
};
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::is_block_all_visible;
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys;
use pgrx::pg_sys::CustomScanState;

pub struct NumericFastFieldExecState {
    inner: FastFieldExecState,
}

impl NumericFastFieldExecState {
    pub fn new(which_fast_fields: Vec<WhichFastField>) -> Self {
        Self {
            inner: FastFieldExecState::new(which_fast_fields),
        }
    }
}

impl ExecMethod for NumericFastFieldExecState {
    fn init(&mut self, state: &mut PdbScanState, cstate: *mut CustomScanState) {
        self.inner.init(state, cstate);
    }

    fn query(&mut self, state: &mut PdbScanState) -> bool {
        if let Some(parallel_state) = state.parallel_state {
            if let Some(segment_id) = unsafe { checkout_segment(parallel_state) } {
                // Check if we should use sorted search
                if let Some(sort_direction) = state.sort_direction {
                    let can_use_sorted_search = if let Some(sort_field) = state.sort_field.as_ref()
                    {
                        // Sorting by field: can only use sorted search if we don't need scores
                        !state.need_scores()
                    } else {
                        // Sorting by score: can always use sorted search
                        true
                    };

                    if can_use_sorted_search {
                        pgrx::warning!(
                            " >>> EXEC: NumericFastFieldExecState using SORTED search, segment={}, sort_direction={:?}, sort_field={:?}",
                            segment_id,
                            sort_direction,
                            state.sort_field
                        );

                        // Use sorted search
                        self.inner.search_results = state
                            .search_reader
                            .as_ref()
                            .unwrap()
                            .search_top_n_in_segments(
                                vec![segment_id].into_iter(),
                                state.search_query_input(),
                                state.sort_field.clone(),
                                sort_direction.into(),
                                1000, // Use a reasonable limit for parallel segments
                                0,    // offset
                                state.need_scores(),
                            );
                    } else {
                        pgrx::warning!(
                            " >>> EXEC: NumericFastFieldExecState using UNORDERED search (needs scores + field sort), segment={}",
                            segment_id
                        );

                        // Fall back to unordered search
                        self.inner.search_results = state.search_reader.as_ref().unwrap().search(
                            state.need_scores(),
                            false,
                            state.search_query_input(),
                            None,
                        );
                    }
                } else {
                    pgrx::warning!(
                        " >>> EXEC: NumericFastFieldExecState using UNORDERED search (no sort direction), segment={}",
                        segment_id
                    );

                    // No sort information, use unordered search
                    self.inner.search_results = state.search_reader().search(
                        state.search_query_input(),
                        vec![segment_id],
                        state.need_scores(),
                    );
                }
                true
            } else {
                false
            }
        } else {
            // Non-parallel case
            if let Some(sort_direction) = state.sort_direction {
                let can_use_sorted_search = if let Some(sort_field) = state.sort_field.as_ref() {
                    // Sorting by field: can only use sorted search if we don't need scores
                    !state.need_scores()
                } else {
                    // Sorting by score: can always use sorted search
                    true
                };

                if can_use_sorted_search {
                    pgrx::warning!(
                        " >>> EXEC: NumericFastFieldExecState using SORTED search (non-parallel), sort_direction={:?}, sort_field={:?}",
                        sort_direction,
                        state.sort_field
                    );

                    // Use sorted search for all segments
                    self.inner.search_results = state.search_reader().search_top_n_in_segments(
                        state.search_reader().segment_readers().keys().copied(),
                        state.search_query_input(),
                        state.sort_field.as_ref().map(|s| s.as_str()),
                        sort_direction,
                        1000, // Use a reasonable limit
                        0,    // offset
                        state.need_scores(),
                    );
                } else {
                    pgrx::warning!(
                        " >>> EXEC: NumericFastFieldExecState using UNORDERED search (needs scores + field sort, non-parallel)"
                    );

                    // Fall back to unordered search
                    self.inner.search_results = state.search_reader.as_ref().unwrap().search(
                        state.need_scores(),
                        false,
                        state.search_query_input(),
                        None,
                    );
                }
            } else {
                pgrx::warning!(
                    " >>> EXEC: NumericFastFieldExecState using UNORDERED search (no sort direction, non-parallel)"
                );

                // No sort information, use unordered search
                self.inner.search_results = state.search_reader().search(
                    state.need_scores(),
                    false,
                    state.search_query_input(),
                    None,
                );
            }
            true
        }
    }

    fn internal_next(&mut self, state: &mut PdbScanState) -> ExecState {
        unsafe {
            match self.inner.search_results.next() {
                None => ExecState::Eof,
                Some((scored, doc_address)) => {
                    let slot = self.inner.slot;
                    let natts = (*(*slot).tts_tupleDescriptor).natts as usize;

                    crate::postgres::utils::u64_to_item_pointer(scored.ctid, &mut (*slot).tts_tid);
                    (*slot).tts_tableOid = (*self.inner.heaprel).rd_id;

                    let blockno = item_pointer_get_block_number(&(*slot).tts_tid);
                    let is_visible = if blockno == self.inner.blockvis.0 {
                        // we know the visibility of this block because we just checked it last time
                        self.inner.blockvis.1
                    } else {
                        // new block so check its visibility
                        self.inner.blockvis.0 = blockno;
                        self.inner.blockvis.1 = is_block_all_visible(
                            self.inner.heaprel,
                            &mut self.inner.vmbuff,
                            blockno,
                        );
                        self.inner.blockvis.1
                    };

                    if is_visible {
                        self.inner.blockvis = (blockno, true);

                        (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
                        (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
                        (*slot).tts_nvalid = natts as _;

                        let tupdesc = self.inner.tupdesc.as_ref().unwrap();
                        let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
                        let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

                        for (i, att) in tupdesc.iter().enumerate() {
                            match non_string_ff_to_datum(
                                (&self.inner.which_fast_fields[i], i),
                                att.atttypid,
                                scored.bm25,
                                doc_address,
                                &mut self.inner.ffhelper,
                                slot,
                            ) {
                                None => {
                                    datums[i] = pg_sys::Datum::null();
                                    isnull[i] = true;
                                }
                                Some(datum) => {
                                    datums[i] = datum;
                                    isnull[i] = false;
                                }
                            }
                        }

                        ExecState::Virtual { slot }
                    } else {
                        ExecState::RequiresVisibilityCheck {
                            ctid: scored.ctid,
                            score: scored.bm25,
                            doc_address,
                        }
                    }
                }
            }
        }
    }

    fn reset(&mut self, _state: &mut PdbScanState) {
        self.inner.reset(_state);
    }
}
