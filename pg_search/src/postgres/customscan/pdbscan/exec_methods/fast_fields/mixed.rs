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
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use pgrx::{pg_sys, IntoDatum, PgTupleDesc};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use tantivy::DocAddress;
use tantivy::DocId;

pub struct MixedFastFieldExecState {
    // Core state from FastFieldExecState
    heaprel: pg_sys::Relation,
    tupdesc: Option<PgTupleDesc<'static>>,
    ffhelper: FFHelper,
    slot: *mut pg_sys::TupleTableSlot,
    which_fast_fields: Vec<WhichFastField>,
    search_results: SearchResults,
    vmbuff: pg_sys::Buffer,
    blockvis: (pg_sys::BlockNumber, bool),
    did_query: bool,

    // Added state for tracking string and numeric fields
    string_fields: FxHashSet<String>,
    numeric_fields: FxHashSet<String>,

    // Current state tracking
    current_doc: Option<DocAddress>,
    string_values: FxHashMap<String, Option<String>>,
    numeric_values: FxHashMap<String, Option<i64>>,

    // Result tracking
    num_rows_fetched: usize,
    num_visible: usize,

    // Search state
    search_iterator: Option<BTreeMap<DocId, DocAddress>>,
    current_item: Option<(DocId, DocAddress)>,
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
        // Separate string and numeric fields
        let string_fields = which_fast_fields
            .iter()
            .filter_map(|field| match field {
                WhichFastField::Named(name, FastFieldType::String) => Some(name.clone()),
                _ => None,
            })
            .collect();

        let numeric_fields = which_fast_fields
            .iter()
            .filter_map(|field| match field {
                WhichFastField::Named(name, FastFieldType::Numeric) => Some(name.clone()),
                _ => None,
            })
            .collect();

        Self {
            heaprel: std::ptr::null_mut(),
            tupdesc: None,
            ffhelper: Default::default(),
            slot: std::ptr::null_mut(),
            vmbuff: pg_sys::InvalidBuffer as pg_sys::Buffer,
            which_fast_fields,
            search_results: SearchResults::None,
            blockvis: (pg_sys::InvalidBlockNumber, false),
            did_query: false,

            string_fields,
            numeric_fields,
            current_doc: None,
            string_values: FxHashMap::default(),
            numeric_values: FxHashMap::default(),
            num_rows_fetched: 0,
            num_visible: 0,
            search_iterator: None,
            current_item: None,
        }
    }

    // Helper method to initialize search if needed
    fn ensure_search_initialized(&mut self, state: &mut PdbScanState) {
        if self.search_iterator.is_none() {
            self.search_results = state.search_results;

            // Convert search results to a map
            self.search_iterator = Some(BTreeMap::new());
            let mut builder = state.search_reader.as_ref().unwrap();
            let result = self.search_results.execute(&mut builder);

            if let Ok(result) = result {
                for (i, doc) in result.iter().enumerate() {
                    if let Some(address) = doc.address {
                        self.search_iterator
                            .as_mut()
                            .unwrap()
                            .insert(i as DocId, address);
                    }
                }
            }

            self.advance_iterator();
        }
    }

    // Move to next doc in the iterator
    fn advance_iterator(&mut self) {
        self.current_item = self
            .search_iterator
            .as_mut()
            .and_then(|map| map.pop_first());
    }

    // Fetch all fast field values for the current document
    fn fetch_values(
        &mut self,
        state: &mut PdbScanState,
        doc_address: DocAddress,
    ) -> Result<(), String> {
        // Use ffhelper to get fast field values
        for field_name in &self.string_fields {
            // Find the field index in ffhelper
            if let Some(index) = self.which_fast_fields.iter().position(|ff| {
                matches!(ff, WhichFastField::Named(name, FastFieldType::String) if name == field_name)
            }) {
                // Get string value from ffhelper
                let mut buffer = String::with_capacity(256);
                if let Some(()) = self.ffhelper.string(index, doc_address, &mut buffer) {
                    self.string_values.insert(field_name.clone(), Some(buffer.clone()));
                } else {
                    self.string_values.insert(field_name.clone(), None);
                }
            }
        }

        // Fetch numeric values
        for field_name in &self.numeric_fields {
            // Find the field index in ffhelper
            if let Some(index) = self.which_fast_fields.iter().position(|ff| {
                matches!(ff, WhichFastField::Named(name, FastFieldType::Numeric) if name == field_name)
            }) {
                // Get numeric value from ffhelper
                if let Some(value) = self.ffhelper.i64(index, doc_address) {
                    self.numeric_values.insert(field_name.clone(), Some(value));
                } else {
                    self.numeric_values.insert(field_name.clone(), None);
                }
            }
        }

        Ok(())
    }

    // Create a virtual tuple with fetched values
    fn create_virtual_tuple(&self, state: &mut PdbScanState) -> *mut pg_sys::TupleTableSlot {
        // Get slot
        let slot = self.slot;

        // Clear the slot
        unsafe {
            pg_sys::ExecClearTuple(slot);
        }

        // Fill values for both string and numeric fields
        self.fill_string_values(slot, state);
        self.fill_numeric_values(slot, state);

        // Mark slot as containing valid virtual tuple
        unsafe {
            pg_sys::ExecStoreVirtualTuple(slot);
        }

        // Return the filled slot
        slot
    }

    fn fill_string_values(&self, slot: *mut pg_sys::TupleTableSlot, state: &PdbScanState) {
        // Similar to StringFastFieldExecState::fill_values but iterates through all string fields
        for field_name in &self.string_fields {
            if let Some(attno) = self.get_attno(field_name, &self.tupdesc) {
                if let Some(Some(value)) = self.string_values.get(field_name) {
                    unsafe {
                        let datum = value.clone().into_datum().unwrap();
                        *(*slot).tts_values.add(attno - 1) = datum;
                        *(*slot).tts_isnull.add(attno - 1) = false;
                    }
                } else {
                    unsafe {
                        *(*slot).tts_isnull.add(attno - 1) = true;
                    }
                }
            }
        }
    }

    fn fill_numeric_values(&self, slot: *mut pg_sys::TupleTableSlot, state: &PdbScanState) {
        // Similar to NumericFastFieldExecState::fill_values but iterates through all numeric fields
        for field_name in &self.numeric_fields {
            if let Some(attno) = self.get_attno(field_name, &self.tupdesc) {
                if let Some(Some(value)) = self.numeric_values.get(field_name) {
                    unsafe {
                        let datum = (*value).into_datum().unwrap();
                        *(*slot).tts_values.add(attno - 1) = datum;
                        *(*slot).tts_isnull.add(attno - 1) = false;
                    }
                } else {
                    unsafe {
                        *(*slot).tts_isnull.add(attno - 1) = true;
                    }
                }
            }
        }
    }

    fn get_attno(&self, field_name: &str, tupdesc: &Option<PgTupleDesc>) -> Option<usize> {
        tupdesc.as_ref().and_then(|td| {
            for i in 0..td.len() {
                let att = td.get(i).unwrap();
                if att.name() == field_name {
                    return Some(i + 1);
                }
            }
            None
        })
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

    fn internal_next(&mut self, state: &mut PdbScanState) -> ExecState {
        self.ensure_search_initialized(state);

        if let Some((_, doc_address)) = self.current_item {
            // Save current doc address for later
            self.current_doc = Some(doc_address);

            // Move to next document for subsequent calls
            self.advance_iterator();

            // Try to fetch all fast field values
            if let Err(_) = self.fetch_values(state, doc_address) {
                // If we can't get fast field values, fall back to heap access
                return ExecState::RequiresVisibilityCheck {
                    ctid: doc_address.segment_ord as u64, // Use segment_ord as ctid
                    score: 0.0,                           // No score needed
                    doc_address,
                };
            }

            self.num_rows_fetched += 1;

            // Create a virtual tuple from the fast fields
            let virtual_slot = self.create_virtual_tuple(state);
            return ExecState::Virtual { slot: virtual_slot };
        }

        // No more documents
        ExecState::Eof
    }

    fn reset(&mut self, state: &mut PdbScanState) {
        self.search_results = SearchResults::None;
        self.did_query = false;
        self.blockvis = (pg_sys::InvalidBlockNumber, false);
        self.search_iterator = None;
        self.current_item = None;
        self.string_values.clear();
        self.numeric_values.clear();
        self.num_rows_fetched = 0;
        self.num_visible = 0;
    }

    fn increment_visible(&mut self) {
        self.num_visible += 1;
    }
}
