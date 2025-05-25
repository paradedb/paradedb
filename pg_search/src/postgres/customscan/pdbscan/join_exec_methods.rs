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

//! Join execution methods for custom join nodes
//!
//! This module implements the execution framework for join nodes, including
//! variable mapping, tuple slot management, and join execution logic.

use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::pdbscan::join_qual_inspect::JoinSearchPredicates;
use crate::postgres::customscan::pdbscan::PdbScan;
use pgrx::{pg_sys, warning};
use std::collections::HashMap;

/// Join execution state for custom join nodes
pub struct JoinExecState {
    /// Search predicates extracted during planning
    pub search_predicates: Option<JoinSearchPredicates>,
    /// Current execution phase
    pub phase: JoinExecPhase,
    /// Tuple slots for join execution
    pub tuple_slots: JoinTupleSlots,
    /// Join execution statistics
    pub stats: JoinExecStats,
    /// Search readers for each relation
    pub search_readers: HashMap<pg_sys::Oid, SearchIndexReader>,
    /// Current outer relation results
    pub outer_results: Option<Vec<(u64, f32)>>, // (ctid, score)
    /// Current inner relation results  
    pub inner_results: Option<Vec<(u64, f32)>>, // (ctid, score)
    /// Current position in outer results
    pub outer_position: usize,
    /// Current position in inner results
    pub inner_position: usize,
    /// Relations involved in the join
    pub outer_relid: pg_sys::Oid,
    pub inner_relid: pg_sys::Oid,
}

/// Execution phases for join processing
#[derive(Debug, Clone, PartialEq)]
pub enum JoinExecPhase {
    /// Initial phase - not started
    NotStarted,
    /// Executing outer relation search
    OuterSearch,
    /// Executing inner relation search  
    InnerSearch,
    /// Performing join matching
    JoinMatching,
    /// Finished execution
    Finished,
}

/// Tuple slots for join execution
pub struct JoinTupleSlots {
    /// Slot for outer relation tuples
    pub outer_slot: Option<*mut pg_sys::TupleTableSlot>,
    /// Slot for inner relation tuples
    pub inner_slot: Option<*mut pg_sys::TupleTableSlot>,
    /// Slot for join result tuples
    pub result_slot: Option<*mut pg_sys::TupleTableSlot>,
}

/// Join execution statistics
#[derive(Debug, Default)]
pub struct JoinExecStats {
    /// Number of outer tuples processed
    pub outer_tuples: usize,
    /// Number of inner tuples processed
    pub inner_tuples: usize,
    /// Number of join matches found
    pub join_matches: usize,
    /// Number of tuples returned
    pub tuples_returned: usize,
}

impl Default for JoinExecState {
    fn default() -> Self {
        Self {
            search_predicates: None,
            phase: JoinExecPhase::NotStarted,
            tuple_slots: JoinTupleSlots::default(),
            stats: JoinExecStats::default(),
            search_readers: HashMap::new(),
            outer_results: None,
            inner_results: None,
            outer_position: 0,
            inner_position: 0,
            outer_relid: pg_sys::InvalidOid,
            inner_relid: pg_sys::InvalidOid,
        }
    }
}

impl Default for JoinTupleSlots {
    fn default() -> Self {
        Self {
            outer_slot: None,
            inner_slot: None,
            result_slot: None,
        }
    }
}

impl JoinExecState {
    /// Initialize join execution state
    pub fn new(search_predicates: Option<JoinSearchPredicates>) -> Self {
        Self {
            search_predicates,
            phase: JoinExecPhase::NotStarted,
            tuple_slots: JoinTupleSlots::default(),
            stats: JoinExecStats::default(),
            search_readers: HashMap::new(),
            outer_results: None,
            inner_results: None,
            outer_position: 0,
            inner_position: 0,
            outer_relid: pg_sys::InvalidOid,
            inner_relid: pg_sys::InvalidOid,
        }
    }

    /// Check if this is a search-optimized join
    pub fn is_search_join(&self) -> bool {
        self.search_predicates
            .as_ref()
            .map(|p| p.has_search_predicates())
            .unwrap_or(false)
    }

    /// Check if this is a bilateral search join
    pub fn is_bilateral_search(&self) -> bool {
        self.search_predicates
            .as_ref()
            .map(|p| p.has_bilateral_search())
            .unwrap_or(false)
    }
}

/// Initialize join execution for a custom scan state
pub unsafe fn init_join_execution(
    state: &mut CustomScanStateWrapper<PdbScan>,
    estate: *mut pg_sys::EState,
) {
    warning!("ParadeDB: Initializing join execution");

    // Extract search predicates from the join path if available
    // For now, we'll create a basic join execution state
    let join_exec_state = JoinExecState::default();
    state.custom_state_mut().join_exec_state = Some(join_exec_state);

    // For join nodes, we need to set up tuple slots differently
    // We'll create a composite tuple descriptor that includes columns from both relations

    // Get the target list to understand what columns we need to provide
    let target_list_len = state.custom_state().targetlist_len;

    // Create a tuple descriptor based on the actual target list
    // This is more sophisticated than the previous version
    let tupdesc = create_join_tuple_descriptor(state, target_list_len);

    // Set up the scan tuple slot with our custom tuple descriptor
    // For join nodes, use virtual tuple slot callbacks instead of table callbacks
    pg_sys::ExecInitScanTupleSlot(
        estate,
        std::ptr::addr_of_mut!(state.csstate.ss),
        tupdesc,
        &pg_sys::TTSOpsVirtual,
    );

    // Initialize result type and projection info
    pg_sys::ExecInitResultTypeTL(std::ptr::addr_of_mut!(state.csstate.ss.ps));

    // Set up projection info for the join result
    pg_sys::ExecAssignProjectionInfo(
        state.planstate(),
        (*state.csstate.ss.ss_ScanTupleSlot).tts_tupleDescriptor,
    );

    warning!(
        "ParadeDB: Join execution initialized with {} target columns",
        target_list_len
    );
}

/// Create a tuple descriptor for join results
unsafe fn create_join_tuple_descriptor(
    state: &mut CustomScanStateWrapper<PdbScan>,
    target_list_len: usize,
) -> pg_sys::TupleDesc {
    warning!(
        "ParadeDB: Creating join tuple descriptor with {} columns",
        target_list_len
    );

    // Ensure we have at least one column to avoid issues
    let actual_len = if target_list_len == 0 {
        1
    } else {
        target_list_len
    };

    // For now, create a simple tuple descriptor
    // In a more complete implementation, we would analyze the target list
    // to determine the actual column types and names
    let tupdesc = pg_sys::CreateTemplateTupleDesc(actual_len as _);

    // Set up basic attributes for the join result
    for i in 0..actual_len {
        let attnum = (i + 1) as pg_sys::AttrNumber;
        let attname = format!("join_col_{}", attnum);
        let attname_cstr = std::ffi::CString::new(attname).unwrap();

        pg_sys::TupleDescInitEntry(
            tupdesc,
            attnum,
            attname_cstr.as_ptr(),
            pg_sys::TEXTOID, // Default to text for now
            -1,              // typmod
            0,               // attdim
        );
    }

    warning!("ParadeDB: Tuple descriptor created successfully");
    tupdesc
}

/// Execute the next step of join processing
pub unsafe fn exec_join_step(
    state: &mut CustomScanStateWrapper<PdbScan>,
) -> *mut pg_sys::TupleTableSlot {
    warning!("ParadeDB: Executing join step");

    // Get or initialize join execution state
    if state.custom_state().join_exec_state.is_none() {
        warning!("ParadeDB: Join execution state not initialized, returning EOF");
        return std::ptr::null_mut();
    }

    // For now, implement a simple stub that returns a single test tuple
    // This demonstrates the execution framework without implementing full join logic
    let join_state = state.custom_state().join_exec_state.as_ref().unwrap();

    match join_state.phase {
        JoinExecPhase::NotStarted => {
            warning!("ParadeDB: Starting join execution");

            // Update phase to indicate we've started
            if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
                join_state.phase = JoinExecPhase::Finished; // Skip to finished for now
            }

            // Create a test tuple to demonstrate the framework works
            create_test_join_tuple(state)
        }
        JoinExecPhase::Finished => {
            warning!("ParadeDB: Join execution finished, returning EOF");
            std::ptr::null_mut()
        }
        _ => {
            warning!("ParadeDB: Join execution in progress, returning EOF for now");
            std::ptr::null_mut()
        }
    }
}

/// Create a test join tuple to demonstrate the execution framework
unsafe fn create_test_join_tuple(
    state: &mut CustomScanStateWrapper<PdbScan>,
) -> *mut pg_sys::TupleTableSlot {
    warning!("ParadeDB: Creating test join tuple");

    let slot = state.csstate.ss.ss_ScanTupleSlot;

    // Validate slot and tuple descriptor
    if slot.is_null() {
        warning!("ParadeDB: Slot is null, returning null");
        return std::ptr::null_mut();
    }

    let tupdesc = (*slot).tts_tupleDescriptor;
    if tupdesc.is_null() {
        warning!("ParadeDB: Tuple descriptor is null, returning null");
        return std::ptr::null_mut();
    }

    let natts = (*tupdesc).natts as usize;
    warning!("ParadeDB: Creating tuple with {} attributes", natts);

    // Clear the slot first
    pg_sys::ExecClearTuple(slot);

    // For now, just return an empty tuple to avoid crashes
    // In a real implementation, we would populate this with actual join data
    pg_sys::ExecStoreVirtualTuple(slot);

    warning!("ParadeDB: Created empty test join tuple");

    // Update statistics
    if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
        join_state.stats.tuples_returned += 1;
    }

    slot
}

/// Clean up join execution resources
pub unsafe fn cleanup_join_execution(state: &mut CustomScanStateWrapper<PdbScan>) {
    warning!("ParadeDB: Cleaning up join execution");

    // Clean up join execution state
    if let Some(mut join_state) = state.custom_state_mut().join_exec_state.take() {
        warning!("ParadeDB: Cleaning up join execution state");

        // Clean up search readers
        join_state.search_readers.clear();

        // Clear result vectors
        join_state.outer_results = None;
        join_state.inner_results = None;

        // Log final statistics
        warning!(
            "ParadeDB: Join execution stats - outer: {}, inner: {}, matches: {}, returned: {}",
            join_state.stats.outer_tuples,
            join_state.stats.inner_tuples,
            join_state.stats.join_matches,
            join_state.stats.tuples_returned
        );
    }

    warning!("ParadeDB: Join execution cleanup complete");
}
