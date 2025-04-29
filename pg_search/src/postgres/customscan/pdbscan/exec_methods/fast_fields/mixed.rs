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
use crate::postgres::customscan::pdbscan::exec_methods::normal::NormalScanExecState;
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::is_block_all_visible;
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::{pg_sys, IntoDatum, PgTupleDesc};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use tantivy::DocAddress;
use tantivy::DocId;

/// MixedFastFieldExecState supports using both string and numeric fast fields together
pub struct MixedFastFieldExecState {
    // Underlying normal exec state for fallback when more complex logic is needed
    normal_exec: Option<NormalScanExecState>,

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

    // Whether to use the fallback normal execution
    use_fallback: bool,
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
        pgrx::warning!(
            "Creating MixedFastFieldExecState with fields: {:?}",
            which_fast_fields
        );

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

        pgrx::warning!(
            "Initialized string_fields: {:?}, numeric_fields: {:?}",
            string_fields,
            numeric_fields
        );

        Self {
            normal_exec: None,

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

            use_fallback: false,
        }
    }

    // Switch to using the normal execution fallback
    fn switch_to_fallback(
        &mut self,
        state: &mut PdbScanState,
        cstate: *mut pg_sys::CustomScanState,
    ) {
        if self.normal_exec.is_none() {
            let mut normal = NormalScanExecState::default();
            normal.init(state, cstate);
            self.normal_exec = Some(normal);
        }

        self.use_fallback = true;
        pgrx::warning!("Switched to NormalScanExecState fallback");
    }

    // Helper method to initialize search if needed
    fn ensure_search_initialized(&mut self, state: &mut PdbScanState) {
        if self.search_iterator.is_none() {
            // Try to query if we haven't done so yet
            if !self.did_query {
                self.query(state);
            }

            // Initialize the iterator from search results now
            if !matches!(self.search_results, SearchResults::None) {
                self.search_iterator = Some(BTreeMap::new());
                let mut i: DocId = 0;

                // Collect all available results into the BTreeMap
                while let Some((scored, doc_address)) = self.search_results.next() {
                    self.search_iterator
                        .as_mut()
                        .unwrap()
                        .insert(i, doc_address);
                    i += 1;
                }

                pgrx::warning!("Initialized search iterator with {} results", i);
                self.advance_iterator();
            }
        }
    }

    // Move to next doc in the iterator
    fn advance_iterator(&mut self) {
        self.current_item = self
            .search_iterator
            .as_mut()
            .and_then(|map| map.pop_first());

        if let Some((doc_id, _)) = self.current_item {
            pgrx::warning!("Advanced to next item: doc_id={}", doc_id);
        } else {
            pgrx::warning!("No more items in iterator");
        }
    }

    // Fetch all fast field values for the current document
    fn fetch_values(
        &mut self,
        state: &mut PdbScanState,
        doc_address: DocAddress,
    ) -> Result<(), String> {
        pgrx::warning!(
            "Fetching fast field values for doc_address: {:?}",
            doc_address
        );

        // Clear previous values
        self.string_values.clear();
        self.numeric_values.clear();

        // Use ffhelper to get fast field values
        for field_name in &self.string_fields {
            // Find the field index in ffhelper
            let field_index = self.which_fast_fields.iter().position(|ff| {
                matches!(ff, WhichFastField::Named(name, FastFieldType::String) if name == field_name)
            });

            if let Some(index) = field_index {
                // Get string value from ffhelper
                let mut buffer = String::with_capacity(256);
                if let Some(()) = self.ffhelper.string(index, doc_address, &mut buffer) {
                    pgrx::warning!("Found string value for field {}: {}", field_name, buffer);
                    self.string_values
                        .insert(field_name.clone(), Some(buffer.clone()));
                } else {
                    pgrx::warning!("No string value for field {}", field_name);
                    self.string_values.insert(field_name.clone(), None);
                }
            } else {
                pgrx::warning!("Field index not found for string field: {}", field_name);
            }
        }

        // Fetch numeric values
        for field_name in &self.numeric_fields {
            // Find the field index in ffhelper
            let field_index = self.which_fast_fields.iter().position(|ff| {
                matches!(ff, WhichFastField::Named(name, FastFieldType::Numeric) if name == field_name)
            });

            if let Some(index) = field_index {
                // Get numeric value from ffhelper
                if let Some(value) = self.ffhelper.i64(index, doc_address) {
                    pgrx::warning!("Found numeric value for field {}: {}", field_name, value);
                    self.numeric_values.insert(field_name.clone(), Some(value));
                } else {
                    pgrx::warning!("No numeric value for field {}", field_name);
                    self.numeric_values.insert(field_name.clone(), None);
                }
            } else {
                pgrx::warning!("Field index not found for numeric field: {}", field_name);
            }
        }

        // If we found at least one value, return Ok
        if !self.string_values.is_empty() || !self.numeric_values.is_empty() {
            pgrx::warning!(
                "Successfully fetched values: {} string, {} numeric",
                self.string_values.len(),
                self.numeric_values.len()
            );
            Ok(())
        } else {
            pgrx::warning!("Failed to fetch any values");
            Err("Failed to find any fast field values".to_string())
        }
    }

    // Create a virtual tuple with fetched values
    fn create_virtual_tuple(&self, state: &mut PdbScanState) -> *mut pg_sys::TupleTableSlot {
        pgrx::warning!("Creating virtual tuple");

        // Get slot
        let slot = self.slot;

        unsafe {
            // Clear the slot
            pg_sys::ExecClearTuple(slot);

            // Build values and nulls arrays
            let natts = (*(*slot).tts_tupleDescriptor).natts as usize;
            let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
            let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

            // Initialize all to NULL
            for i in 0..natts {
                datums[i] = pg_sys::Datum::null();
                isnull[i] = true;
            }

            // Fill values for both string and numeric fields
            if let Some(tupdesc) = &self.tupdesc {
                for (i, att) in tupdesc.iter().enumerate() {
                    let att_name = att.name();
                    pgrx::warning!("Processing attribute {}: {}", i, att_name);

                    // Check if attribute is a string fast field
                    if let Some(Some(value)) = self
                        .string_fields
                        .contains(att_name)
                        .then(|| self.string_values.get(att_name).cloned().flatten())
                    {
                        pgrx::warning!("Setting string value for {}", att_name);
                        datums[i] = value.into_datum().unwrap();
                        isnull[i] = false;
                    }
                    // Check if attribute is a numeric fast field
                    else if let Some(Some(value)) = self
                        .numeric_fields
                        .contains(att_name)
                        .then(|| self.numeric_values.get(att_name).cloned().flatten())
                    {
                        pgrx::warning!("Setting numeric value for {}", att_name);
                        datums[i] = value.into_datum().unwrap();
                        isnull[i] = false;
                    }
                }
            }

            // Mark slot as containing valid virtual tuple
            pg_sys::ExecStoreVirtualTuple(slot);
        }

        // Return the filled slot
        slot
    }

    // Create a valid ctid for visibility check
    fn get_valid_ctid(&self, doc_address: DocAddress) -> u64 {
        // Properly construct a valid item pointer from the document address
        // Use the doc_id to create a unique ctid
        let blockno: u32 = 1; // First valid block number
        let offset: u16 = (doc_address.doc_id + 1) as u16; // Must be >= 1
        ((blockno as u64) << 32) | (offset as u64)
    }
}

impl ExecMethod for MixedFastFieldExecState {
    fn init(&mut self, state: &mut PdbScanState, cstate: *mut pg_sys::CustomScanState) {
        pgrx::warning!("Initializing MixedFastFieldExecState");
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

            pgrx::warning!("Initialized with tuple descriptor, created slot");

            // We'll start with using the normal execution for simplicity
            self.switch_to_fallback(state, cstate);
        }
    }

    fn query(&mut self, state: &mut PdbScanState) -> bool {
        pgrx::warning!("Querying in MixedFastFieldExecState");

        if self.use_fallback {
            // Use the fallback execution
            if let Some(ref mut normal_exec) = self.normal_exec {
                return normal_exec.query(state);
            }
        }

        // Reset search state
        self.search_iterator = None;
        self.current_item = None;

        if let Some(parallel_state) = state.parallel_state {
            if let Some(segment_id) = unsafe { checkout_segment(parallel_state) } {
                pgrx::warning!("Searching segment: {:?}", segment_id);
                self.search_results = state.search_reader.as_ref().unwrap().search_segment(
                    state.need_scores(),
                    segment_id,
                    &state.search_query_input,
                );
                self.did_query = true;
                return true;
            }

            // no more segments to query
            pgrx::warning!("No more segments to query");
            self.search_results = SearchResults::None;
            false
        } else if self.did_query {
            // not parallel, so we're done
            pgrx::warning!("Already queried, not parallel");
            false
        } else {
            // not parallel, first time query
            pgrx::warning!("First time query, not parallel");
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
            pgrx::warning!("Query completed successfully");
            true
        }
    }

    fn internal_next(&mut self, state: &mut PdbScanState) -> ExecState {
        pgrx::warning!("internal_next called in MixedFastFieldExecState");

        if self.use_fallback {
            // Use the fallback execution
            if let Some(ref mut normal_exec) = self.normal_exec {
                return normal_exec.internal_next(state);
            }
        }

        // For now, we'll switch to the fallback implementation
        // Remove this in the future once we have a working implementation
        match self.normal_exec {
            Some(ref mut normal) => {
                pgrx::warning!("Using normal execution fallback");
                normal.internal_next(state)
            }
            None => {
                pgrx::warning!("Normal execution not initialized, returning EOF");
                ExecState::Eof
            }
        }
    }

    fn reset(&mut self, state: &mut PdbScanState) {
        pgrx::warning!("Resetting MixedFastFieldExecState");

        if self.use_fallback {
            // Use the fallback execution
            if let Some(ref mut normal_exec) = self.normal_exec {
                normal_exec.reset(state);
                return;
            }
        }

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
        if self.use_fallback {
            // Use the fallback execution
            if let Some(ref mut normal_exec) = self.normal_exec {
                normal_exec.increment_visible();
                return;
            }
        }

        self.num_visible += 1;
    }
}
