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

use crate::index::fast_fields_helper::{FFHelper, FastFieldType, WhichFastField};
use crate::index::reader::index::SearchResults;
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::is_block_all_visible;
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::{pg_sys, IntoDatum, PgTupleDesc};
use rustc_hash::FxHashMap;
use tantivy::DocAddress;

/// MixedFastFieldExecState handles mixed (string and numeric) fast field retrieval
/// Use when you have multiple string fast fields, or a mix of string and numeric fast fields
pub struct MixedFastFieldExecState {
    // Core state
    heaprel: pg_sys::Relation,
    tupdesc: Option<PgTupleDesc<'static>>,
    ffhelper: FFHelper,
    slot: *mut pg_sys::TupleTableSlot,
    which_fast_fields: Vec<WhichFastField>,

    // Search state
    search_results: SearchResults,
    did_query: bool,

    // Visibility checking
    vmbuff: pg_sys::Buffer,
    blockvis: (pg_sys::BlockNumber, bool),

    // Field tracking
    string_fields: Vec<(usize, String)>, // (index in which_fast_fields, field_name)
    numeric_fields: Vec<(usize, String)>, // (index in which_fast_fields, field_name)

    // Values cache for current document
    string_values: FxHashMap<String, Option<String>>,
    numeric_values: FxHashMap<String, Option<i64>>,

    // Statistics
    num_rows_fetched: usize,
    num_visible: usize,
}

impl Drop for MixedFastFieldExecState {
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

impl MixedFastFieldExecState {
    pub fn new(which_fast_fields: Vec<WhichFastField>) -> Self {
        // Organize fields by type for efficient retrieval
        let mut string_fields = Vec::new();
        let mut numeric_fields = Vec::new();

        for (idx, field) in which_fast_fields.iter().enumerate() {
            match field {
                WhichFastField::Named(name, FastFieldType::String) => {
                    string_fields.push((idx, name.clone()));
                }
                WhichFastField::Named(name, FastFieldType::Numeric) => {
                    numeric_fields.push((idx, name.clone()));
                }
                _ => {} // Other field types handled elsewhere
            }
        }

        Self {
            heaprel: std::ptr::null_mut(),
            tupdesc: None,
            ffhelper: Default::default(),
            slot: std::ptr::null_mut(),
            which_fast_fields,

            search_results: SearchResults::None,
            did_query: false,

            vmbuff: pg_sys::InvalidBuffer as pg_sys::Buffer,
            blockvis: (pg_sys::InvalidBlockNumber, false),

            string_fields,
            numeric_fields,

            string_values: FxHashMap::default(),
            numeric_values: FxHashMap::default(),

            num_rows_fetched: 0,
            num_visible: 0,
        }
    }

    // Fetch all fast field values for the current document
    fn fetch_values(&mut self, doc_address: DocAddress) -> Result<(), String> {
        // Clear previous values
        self.string_values.clear();
        self.numeric_values.clear();

        // Fetch string values
        let mut found_values = false;
        for (field_index, field_name) in &self.string_fields {
            let mut buffer = String::with_capacity(256);
            if let Some(()) = self.ffhelper.string(*field_index, doc_address, &mut buffer) {
                self.string_values
                    .insert(field_name.clone(), Some(buffer.clone()));
                found_values = true;
            } else {
                self.string_values.insert(field_name.clone(), None);
            }
        }

        // Fetch numeric values
        for (field_index, field_name) in &self.numeric_fields {
            if let Some(value) = self.ffhelper.i64(*field_index, doc_address) {
                self.numeric_values.insert(field_name.clone(), Some(value));
                found_values = true;
            } else {
                self.numeric_values.insert(field_name.clone(), None);
            }
        }

        if found_values {
            Ok(())
        } else {
            Err("No fast field values found".to_string())
        }
    }

    // Fill the slot with fast field values
    fn fill_slot(
        &self,
        doc_address: DocAddress,
        ctid: u64,
        score: f32,
    ) -> *mut pg_sys::TupleTableSlot {
        unsafe {
            let slot = self.slot;
            pg_sys::ExecClearTuple(slot);

            // Set the ctid and tableoid
            let mut tid = pg_sys::ItemPointerData::default();
            crate::postgres::utils::u64_to_item_pointer(ctid, &mut tid);
            (*slot).tts_tid = tid;
            (*slot).tts_tableOid = (*self.heaprel).rd_id;

            // Get tuple descriptor and set up for values
            if let Some(tupdesc) = &self.tupdesc {
                let natts = tupdesc.len();
                (*slot).tts_nvalid = natts as i16;

                let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
                let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

                // Initialize all to NULL
                for i in 0..natts {
                    datums[i] = pg_sys::Datum::null();
                    isnull[i] = true;
                }

                // Fill from fast fields
                'outer: for i in 0..natts {
                    let att = tupdesc.get(i).unwrap();
                    let att_name = att.name();

                    // Special system attributes
                    if att_name == "ctid" {
                        datums[i] = tid.into_datum().unwrap();
                        isnull[i] = false;
                        continue;
                    } else if att_name == "tableoid" {
                        datums[i] = (*self.heaprel).rd_id.into_datum().unwrap();
                        isnull[i] = false;
                        continue;
                    } else if att_name == "score" {
                        datums[i] = score.into_datum().unwrap();
                        isnull[i] = false;
                        continue;
                    }

                    // Check in string fields first
                    for (_, field_name) in &self.string_fields {
                        if field_name == att_name {
                            if let Some(Some(value)) = self.string_values.get(field_name) {
                                datums[i] = value.clone().into_datum().unwrap();
                                isnull[i] = false;
                            }
                            continue 'outer;
                        }
                    }

                    // Check in numeric fields next
                    for (_, field_name) in &self.numeric_fields {
                        if field_name == att_name {
                            if let Some(Some(value)) = self.numeric_values.get(field_name) {
                                datums[i] = (*value).into_datum().unwrap();
                                isnull[i] = false;
                            }
                            continue 'outer;
                        }
                    }

                    // This attribute isn't a fast field - leave as NULL
                }
            }

            // Mark slot as containing a valid virtual tuple
            pg_sys::ExecStoreVirtualTuple(slot);
            slot
        }
    }
}

impl ExecMethod for MixedFastFieldExecState {
    fn init(&mut self, state: &mut PdbScanState, cstate: *mut pg_sys::CustomScanState) {
        unsafe {
            self.heaprel = state.heaprel();
            self.tupdesc = Some(PgTupleDesc::from_pg_unchecked(
                (*cstate).ss.ps.ps_ResultTupleDesc,
            ));
            self.slot = pg_sys::MakeTupleTableSlot(
                (*cstate).ss.ps.ps_ResultTupleDesc,
                &pg_sys::TTSOpsVirtual,
            );
            self.ffhelper = FFHelper::with_fields(
                state.search_reader.as_ref().unwrap(),
                &self.which_fast_fields,
            );
        }
    }

    fn query(&mut self, state: &mut PdbScanState) -> bool {
        if let Some(parallel_state) = state.parallel_state {
            if let Some(segment_id) = unsafe { checkout_segment(parallel_state) } {
                self.search_results = state.search_reader.as_ref().unwrap().search_segment(
                    state.need_scores(),
                    segment_id,
                    &state.search_query_input,
                );
                self.did_query = true;
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
            self.search_results = state
                .search_reader
                .as_ref()
                .expect("must have a search_reader to do a query")
                .search(
                    state.need_scores(),
                    false,
                    &state.search_query_input,
                    state.limit,
                );
            self.did_query = true;
            true
        }
    }

    fn internal_next(&mut self, state: &mut PdbScanState) -> ExecState {
        if matches!(self.search_results, SearchResults::None) && !self.query(state) {
            return ExecState::Eof;
        }

        match self.search_results.next() {
            None => ExecState::Eof,
            Some((scored, doc_address)) => {
                let ctid = scored.ctid;
                let score = scored.bm25;

                unsafe {
                    // Check if the block is visible
                    let mut tid = pg_sys::ItemPointerData::default();
                    crate::postgres::utils::u64_to_item_pointer(ctid, &mut tid);

                    let blockno = item_pointer_get_block_number(&tid);
                    let is_visible = if blockno == self.blockvis.0 {
                        // we know the visibility of this block because we just checked it last time
                        self.blockvis.1
                    } else {
                        // new block so check its visibility
                        self.blockvis.0 = blockno;
                        self.blockvis.1 =
                            is_block_all_visible(self.heaprel, &mut self.vmbuff, blockno);
                        self.blockvis.1
                    };

                    if is_visible {
                        // If the block is visible, try to fetch values from fast fields
                        match self.fetch_values(doc_address) {
                            Ok(()) => {
                                // Successfully fetched fast field values
                                self.num_rows_fetched += 1;
                                ExecState::Virtual {
                                    slot: self.fill_slot(doc_address, ctid, score),
                                }
                            }
                            Err(_) => {
                                // Couldn't get values from fast fields, need heap check
                                ExecState::RequiresVisibilityCheck {
                                    ctid,
                                    score,
                                    doc_address,
                                }
                            }
                        }
                    } else {
                        // Block not visible, need heap check
                        ExecState::RequiresVisibilityCheck {
                            ctid,
                            score,
                            doc_address,
                        }
                    }
                }
            }
        }
    }

    fn reset(&mut self, state: &mut PdbScanState) {
        self.search_results = SearchResults::None;
        self.did_query = false;
        self.blockvis = (pg_sys::InvalidBlockNumber, false);
        self.string_values.clear();
        self.numeric_values.clear();
        self.num_rows_fetched = 0;
        self.num_visible = 0;
    }

    fn increment_visible(&mut self) {
        self.num_visible += 1;
    }
}
