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

//! Join execution state management with optimized memory usage and logging

use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::pdbscan::join_qual_inspect::JoinSearchPredicates;
use crate::postgres::customscan::pdbscan::privdat::JoinCompositeInfo;
use crate::postgres::customscan::pdbscan::PdbScan;
use pgrx::{pg_sys, warning};
use std::collections::HashMap;

/// Execution phases for join processing
#[derive(Debug, Clone, PartialEq)]
pub enum JoinExecPhase {
    NotStarted,
    OuterSearch,
    InnerSearch,
    JoinMatching,
    Finished,
}

/// Join execution statistics with memory-efficient storage
#[derive(Debug, Default)]
pub struct JoinExecStats {
    pub outer_tuples: u32,
    pub inner_tuples: u32,
    pub join_matches: u32,
    pub tuples_returned: u32,
    pub heap_fetch_attempts: u32,
    pub heap_fetch_successes: u32,
    // Use smaller types for timing to save memory
    pub outer_search_time_us: u32,
    pub inner_search_time_us: u32,
    pub join_matching_time_us: u32,
}

/// Tuple slots for join execution
pub struct JoinTupleSlots {
    pub outer_slot: Option<*mut pg_sys::TupleTableSlot>,
    pub inner_slot: Option<*mut pg_sys::TupleTableSlot>,
    pub result_slot: Option<*mut pg_sys::TupleTableSlot>,
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

/// Main join execution state with optimized memory layout
pub struct JoinExecState {
    // Core state
    pub phase: JoinExecPhase,

    // Relation information
    pub outer_relid: pg_sys::Oid,
    pub inner_relid: pg_sys::Oid,

    // Search predicates
    pub search_predicates: Option<JoinSearchPredicates>,

    // Execution state
    pub stats: JoinExecStats,
    pub tuple_slots: JoinTupleSlots,

    // Search results
    pub outer_results: Option<Vec<(u64, f32)>>,
    pub inner_results: Option<Vec<(u64, f32)>>,

    // Position tracking
    pub outer_position: usize,
    pub inner_position: usize,

    // Search readers
    pub search_readers: HashMap<pg_sys::Oid, SearchIndexReader>,

    // Composite join information
    pub composite_info: Option<JoinCompositeInfo>,

    // Variable mappings
    pub varno_to_relid: HashMap<pg_sys::Index, pg_sys::Oid>,

    // Intermediate result handling
    pub has_intermediate_input: bool,
    pub intermediate_side: Option<IntermediateSide>,
    pub intermediate_iterator: Option<IntermediateResultIterator>,
}

/// Which side of the join has intermediate results
#[derive(Debug, Clone, PartialEq)]
pub enum IntermediateSide {
    Outer,
    Inner,
}

/// Iterator for intermediate join results from PostgreSQL's executor
pub struct IntermediateResultIterator {
    /// The child plan node that provides intermediate results
    pub child_plan_state: *mut pg_sys::PlanState,
    /// Current intermediate tuple
    pub current_tuple: Option<*mut pg_sys::TupleTableSlot>,
    /// Tuple descriptor for intermediate results
    pub intermediate_tupdesc: pg_sys::TupleDesc,
    /// Cached intermediate tuples for join processing
    pub cached_tuples: Vec<Vec<Option<String>>>,
    /// Current position in cached tuples
    pub current_position: usize,
}

impl Default for JoinExecState {
    fn default() -> Self {
        Self {
            phase: JoinExecPhase::NotStarted,
            outer_relid: pg_sys::InvalidOid,
            inner_relid: pg_sys::InvalidOid,
            search_predicates: None,
            stats: JoinExecStats::default(),
            tuple_slots: JoinTupleSlots::default(),
            outer_results: None,
            inner_results: None,
            outer_position: 0,
            inner_position: 0,
            search_readers: HashMap::new(),
            composite_info: None,
            varno_to_relid: HashMap::new(),
            has_intermediate_input: false,
            intermediate_side: None,
            intermediate_iterator: None,
        }
    }
}

impl JoinExecState {
    /// Create new join execution state with search predicates
    pub fn new(search_predicates: Option<JoinSearchPredicates>) -> Self {
        Self {
            search_predicates,
            ..Default::default()
        }
    }

    /// Check if this is a search-optimized join
    pub fn is_search_join(&self) -> bool {
        self.search_predicates
            .as_ref()
            .map(|p| p.has_search_predicates())
            .unwrap_or(false)
    }

    /// Check if both sides have search predicates
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

    // Validate state
    if state.custom_state().join_exec_state.is_none() {
        warning!("ParadeDB: No join execution state found");

        // Create fallback state
        let fallback_state = JoinExecState::default();
        state.custom_state_mut().join_exec_state = Some(fallback_state);
        return;
    }

    warning!("ParadeDB: Join execution initialization complete");
}

/// Execute the next step of join processing
pub unsafe fn exec_join_step(
    state: &mut CustomScanStateWrapper<PdbScan>,
) -> *mut pg_sys::TupleTableSlot {
    // Import the actual implementation functions from the parent module
    use super::{
        init_search_execution_safe, match_and_return_next_tuple_safe,
        validate_join_execution_prerequisites,
    };

    // Validate state
    if state.custom_state().join_exec_state.is_none() {
        warning!("ParadeDB: Join execution state not initialized");
        return std::ptr::null_mut();
    }

    // Validate prerequisites for join execution
    if !validate_join_execution_prerequisites(state) {
        warning!("ParadeDB: Join execution prerequisites not met");
        return std::ptr::null_mut();
    }

    // Get current phase
    let current_phase = state
        .custom_state()
        .join_exec_state
        .as_ref()
        .unwrap()
        .phase
        .clone();

    // Execute appropriate phase
    match current_phase {
        JoinExecPhase::NotStarted => {
            warning!("ParadeDB: Starting join execution - initializing search");

            // Initialize search execution
            if !init_search_execution_safe(state) {
                warning!("ParadeDB: Failed to initialize search execution");
                if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
                    join_state.phase = JoinExecPhase::Finished;
                }
                return std::ptr::null_mut();
            }

            // Move directly to join matching phase since search is now complete
            if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
                join_state.phase = JoinExecPhase::JoinMatching;
            }

            // Continue to join matching
            exec_join_step(state)
        }
        JoinExecPhase::OuterSearch => {
            warning!("ParadeDB: Outer search phase - transitioning to inner search");

            // Move to inner search phase
            if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
                join_state.phase = JoinExecPhase::InnerSearch;
            }

            exec_join_step(state)
        }
        JoinExecPhase::InnerSearch => {
            warning!("ParadeDB: Inner search phase - transitioning to join matching");

            // Move to join matching phase
            if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
                join_state.phase = JoinExecPhase::JoinMatching;
            }

            exec_join_step(state)
        }
        JoinExecPhase::JoinMatching => {
            warning!("ParadeDB: Performing join matching");

            // Execute the actual join matching logic
            match_and_return_next_tuple_safe(state)
        }
        JoinExecPhase::Finished => {
            warning!("ParadeDB: Join execution finished");
            std::ptr::null_mut()
        }
    }
}

/// Clean up join execution resources
pub unsafe fn cleanup_join_execution(state: &mut CustomScanStateWrapper<PdbScan>) {
    if let Some(mut join_state) = state.custom_state_mut().join_exec_state.take() {
        warning!("ParadeDB: Cleaning up join execution state");

        // Clear large data structures
        join_state.search_readers.clear();
        join_state.outer_results = None;
        join_state.inner_results = None;
        join_state.varno_to_relid.clear();

        warning!("ParadeDB: Join execution cleanup complete");
    }
}
