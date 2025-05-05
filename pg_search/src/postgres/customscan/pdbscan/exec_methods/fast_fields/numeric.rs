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

use crate::index::fast_fields_helper::{FFHelper, WhichFastField};
use crate::index::reader::index::SearchResults;
use crate::postgres::customscan::pdbscan::exec_methods::fast_fields::{
    ff_to_datum, FastFieldExecState,
};
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::is_block_all_visible;
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys::CustomScanState;
use pgrx::{pg_sys, PgTupleDesc};

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
        unsafe {
            self.inner.heaprel = state.heaprel();
            self.inner.tupdesc = Some(PgTupleDesc::from_pg_unchecked(
                (*cstate).ss.ps.ps_ResultTupleDesc,
            ));
            self.inner.slot = pg_sys::MakeTupleTableSlot(
                (*cstate).ss.ps.ps_ResultTupleDesc,
                &pg_sys::TTSOpsVirtual,
            );
            self.inner.ffhelper = FFHelper::with_fields(
                state.search_reader.as_ref().unwrap(),
                &self.inner.which_fast_fields,
            );
        }
    }

    fn query(&mut self, state: &mut PdbScanState) -> bool {
        if let Some(parallel_state) = state.parallel_state {
            if let Some(segment_id) = unsafe { checkout_segment(parallel_state) } {
                self.inner.search_results = state.search_reader.as_ref().unwrap().search_segment(
                    state.need_scores(),
                    segment_id,
                    &state.search_query_input,
                );
                return true;
            }

            // no more segments to query
            self.inner.search_results = SearchResults::None;
            false
        } else if self.inner.did_query {
            // not parallel, so we're done
            false
        } else {
            // not parallel, first time query
            self.inner.search_results = state.search_reader.as_ref().unwrap().search(
                state.need_scores(),
                false,
                &state.search_query_input,
                state.limit,
            );
            self.inner.did_query = true;
            true
        }
    }

    fn internal_next(&mut self, _state: &mut PdbScanState) -> ExecState {
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

                        let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
                        let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

                        // Create mapping from attributes to fast fields
                        let fast_fields = &mut self.inner.ffhelper;
                        let which_fast_fields = &self.inner.which_fast_fields;
                        let tupdesc = self.inner.tupdesc.as_ref().unwrap();

                        // Build attribute to fast field mapping
                        let mut attr_to_ff_map = std::collections::HashMap::new();
                        let mut next_ff_idx = 0;
                        for i in 0..natts {
                            if !attr_to_ff_map.contains_key(&i)
                                && next_ff_idx < which_fast_fields.len()
                            {
                                attr_to_ff_map.insert(i, next_ff_idx);
                                next_ff_idx += 1;
                            }
                        }

                        // Ensure every attribute has a mapping
                        for i in 0..natts {
                            debug_assert!(
                                attr_to_ff_map.contains_key(&i),
                                "Attribute at position {} has no fast field mapping",
                                i
                            );
                            // Verify that the fast field index is valid
                            if let Some(&ff_idx) = attr_to_ff_map.get(&i) {
                                debug_assert!(
                                    ff_idx < which_fast_fields.len(),
                                    "Attribute at position {} maps to invalid fast field index {}",
                                    i,
                                    ff_idx
                                );
                            }
                        }

                        // Process attributes using our mapping
                        for i in 0..natts {
                            if let Some(&ff_idx) = attr_to_ff_map.get(&i) {
                                if ff_idx < which_fast_fields.len() {
                                    let which_fast_field = &which_fast_fields[ff_idx];
                                    let att = tupdesc.get(i).unwrap();

                                    match ff_to_datum(
                                        (which_fast_field, ff_idx),
                                        att.atttypid,
                                        scored.bm25,
                                        doc_address,
                                        fast_fields,
                                        &mut self.inner.strbuf,
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
                                } else {
                                    // Fast field index out of bounds
                                    datums[i] = pg_sys::Datum::null();
                                    isnull[i] = true;
                                }
                            } else {
                                // This attribute doesn't have a matching fast field
                                datums[i] = pg_sys::Datum::null();
                                isnull[i] = true;
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
