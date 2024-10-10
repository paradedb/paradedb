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

use crate::index::score::SearchIndexScore;
use crate::index::SearchIndex;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::pdbscan::{make_tuple_table_slot, PdbScan};
use pgrx::pg_sys::TupleTableSlot;
use pgrx::{direct_function_call, pg_sys, IntoDatum};
use tantivy::DocAddress;

// TODO:  should these be GUCs?  I think yes, probably
const SUBSEQUENT_RETRY_SCALE_FACTOR: usize = 2;
const MAX_CHUNK_SIZE: usize = 5000;

pub enum ExecState {
    Found {
        scored: SearchIndexScore,
        doc_address: DocAddress,
        slot: *mut TupleTableSlot,
    },
    Invisible {
        scored: SearchIndexScore,
    },
    Eof,
}

pub type NormalScanExecState = ();

#[inline(always)]
pub fn normal_scan_exec(
    state: &mut CustomScanStateWrapper<PdbScan>,
    _: *mut std::ffi::c_void,
) -> ExecState {
    match state.custom_state_mut().search_results.next() {
        None => ExecState::Eof,
        Some((scored, doc_address)) => {
            let scanslot = state.scanslot();
            let bslot = state.scanslot() as *mut pg_sys::BufferHeapTupleTableSlot;

            match make_tuple_table_slot(state, &scored, bslot) {
                None => ExecState::Invisible { scored },
                Some(slot) => ExecState::Found {
                    scored,
                    doc_address,
                    slot,
                },
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct TopNScanExecState {
    last_ctid: u64,
    pub limit: usize,
    pub have_less: bool,
    found: usize,
    pub chunk_size: usize,
}

#[inline(always)]
pub fn top_n_scan_exec(
    state: &mut CustomScanStateWrapper<PdbScan>,
    isc: *mut std::ffi::c_void,
) -> ExecState {
    unsafe {
        let topn_state = isc.cast::<TopNScanExecState>().as_mut().unwrap();

        let mut next = state.custom_state_mut().search_results.next();
        loop {
            match next {
                None => {
                    if topn_state.found == topn_state.limit || topn_state.have_less {
                        // we found all the matching rows
                        return ExecState::Eof;
                    }
                }
                Some((scored, doc_address)) => {
                    let scanslot = state.scanslot();
                    let bslot = state.scanslot() as *mut pg_sys::BufferHeapTupleTableSlot;

                    topn_state.last_ctid = scored.ctid;

                    return match make_tuple_table_slot(state, &scored, bslot) {
                        None => ExecState::Invisible { scored },
                        Some(slot) => {
                            topn_state.found += 1;
                            ExecState::Found {
                                scored,
                                doc_address,
                                slot,
                            }
                        }
                    };
                }
            }

            // we underflowed our tuples, so go get some more, if there are any
            state.custom_state_mut().retry_count += 1;

            // calculate a scaling factor to use against the limit
            let factor = if topn_state.chunk_size == 0 {
                // if we haven't done any chunking yet, calculate the scaling factor
                // based on the proportion of dead tuples compared to live tuples
                let heaprelid = state.custom_state().heaprelid;
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

                (1.0 + (n_dead as f64 / (1.0 + n_live as f64))).floor() as usize
            } else {
                // we've already done chunking, so just use a default scaling factor
                // to avoid exponentially growing the chunk size
                SUBSEQUENT_RETRY_SCALE_FACTOR
            };

            // set the chunk size to the scaling factor times the limit
            topn_state.chunk_size = (topn_state.chunk_size * factor)
                .max(topn_state.limit * factor)
                .min(MAX_CHUNK_SIZE);

            let mut results = state
                .custom_state()
                .search_reader
                .as_ref()
                .unwrap()
                .search_top_n(
                    SearchIndex::executor(),
                    state.custom_state().query.as_ref().unwrap(),
                    state
                        .custom_state()
                        .sort_direction
                        .as_ref()
                        .cloned()
                        .unwrap()
                        .into(),
                    topn_state.chunk_size,
                );

            // fast forward and stop on the ctid we last found
            for (scored, _) in &mut results {
                if scored.ctid == topn_state.last_ctid {
                    // we've now advanced to the last ctid we found
                    break;
                }
            }

            // this should be the next valid tuple after that
            next = match results.next() {
                // ... and there it is!
                Some(next) => Some(next),

                // there wasn't one, so we've now read all possible matches
                None => return ExecState::Eof,
            };

            // we now have a new iterator of results to use going forward
            state.custom_state_mut().search_results = results;

            // but we'll loop back around and evaluate whatever `next` is now pointing to
            continue;
        }
    }
}
