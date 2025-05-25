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

use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::pdbscan::join_qual_inspect::JoinSearchPredicates;
use crate::postgres::customscan::pdbscan::PdbScan;
use pgrx::{pg_sys, warning, PgMemoryContexts};

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

    // For join nodes, we need to set up tuple slots differently
    // We'll create a composite tuple descriptor that includes columns from both relations

    // For now, create a minimal tuple descriptor to avoid the "variable not found" error
    // This will be expanded in the next milestone with proper variable mapping
    let target_list_len = state.custom_state().targetlist_len;
    let tupdesc = pg_sys::CreateTemplateTupleDesc(target_list_len as _);

    // Set up basic attributes for the join result
    for i in 0..target_list_len {
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

    // Finalize the tuple descriptor
    pg_sys::TupleDescInitEntryCollation(tupdesc, 1, pg_sys::DEFAULT_COLLATION_OID);

    // Set up the scan tuple slot with our custom tuple descriptor
    pg_sys::ExecInitScanTupleSlot(
        estate,
        std::ptr::addr_of_mut!(state.csstate.ss),
        tupdesc,
        pg_sys::table_slot_callbacks(std::ptr::null_mut()),
    );

    // Initialize result type and projection info
    pg_sys::ExecInitResultTypeTL(std::ptr::addr_of_mut!(state.csstate.ss.ps));

    // Set up projection info for the join result
    // For now, use a simple identity projection
    pg_sys::ExecAssignProjectionInfo(
        state.planstate(),
        (*state.csstate.ss.ss_ScanTupleSlot).tts_tupleDescriptor,
    );

    warning!(
        "ParadeDB: Join execution initialized with {} target columns",
        target_list_len
    );
}

/// Execute the next step of join processing
pub unsafe fn exec_join_step(
    state: &mut CustomScanStateWrapper<PdbScan>,
) -> *mut pg_sys::TupleTableSlot {
    warning!("ParadeDB: Executing join step");

    // For now, return NULL to indicate end of results
    // This will be expanded in Milestone 2.3 with actual join logic
    std::ptr::null_mut()
}

/// Clean up join execution resources
pub unsafe fn cleanup_join_execution(state: &mut CustomScanStateWrapper<PdbScan>) {
    warning!("ParadeDB: Cleaning up join execution");

    // Clean up any join-specific resources
    // This will be expanded as we add more join execution state
}
