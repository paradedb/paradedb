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

    // Query limit (for lazy loading optimization)
    pub limit: Option<f64>,

    // Field map for lazy loading
    pub field_map: Option<super::field_map::MultiTableFieldMap>,
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
            limit: None,
            field_map: None,
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

/// Main execution step for join operations
pub unsafe fn exec_join_step(
    state: &mut CustomScanStateWrapper<PdbScan>,
) -> *mut pg_sys::TupleTableSlot {
    // Check if we have join execution state
    if state.custom_state().join_exec_state.is_none() {
        warning!("ParadeDB: No join execution state - this shouldn't happen");
        return std::ptr::null_mut();
    }

    // Get the current phase
    let current_phase = if let Some(ref join_state) = state.custom_state().join_exec_state {
        join_state.phase.clone()
    } else {
        return std::ptr::null_mut();
    };

    warning!(
        "ParadeDB: Executing join step in phase: {:?}",
        current_phase
    );

    match current_phase {
        JoinExecPhase::NotStarted => {
            warning!("ParadeDB: Starting join execution");

            // Initialize search execution
            if let Some(ref join_state) = state.custom_state().join_exec_state {
                if let Some(ref predicates) = join_state.search_predicates {
                    warning!(
                        "ParadeDB: Found search predicates - outer: {}, inner: {}",
                        predicates.outer_predicates.len(),
                        predicates.inner_predicates.len()
                    );

                    // Log details about each predicate
                    for (i, pred) in predicates.outer_predicates.iter().enumerate() {
                        warning!(
                            "ParadeDB: Outer predicate {}: relation={}, uses_search={}",
                            i,
                            pred.relname,
                            pred.uses_search_operator
                        );
                    }
                    for (i, pred) in predicates.inner_predicates.iter().enumerate() {
                        warning!(
                            "ParadeDB: Inner predicate {}: relation={}, uses_search={}",
                            i,
                            pred.relname,
                            pred.uses_search_operator
                        );
                    }
                } else {
                    warning!("ParadeDB: No search predicates found");
                }
            }

            // Move to join matching phase
            if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
                join_state.phase = JoinExecPhase::JoinMatching;

                // Initialize mock results for testing
                join_state.outer_results = Some(vec![(1, 1.0), (2, 0.8)]);
                join_state.inner_results = Some(vec![(1, 0.9), (2, 0.7)]);
                join_state.outer_position = 0;
                join_state.inner_position = 0;

                warning!(
                    "ParadeDB: Initialized mock results - outer: {}, inner: {}",
                    join_state.outer_results.as_ref().unwrap().len(),
                    join_state.inner_results.as_ref().unwrap().len()
                );
            }

            // Continue to matching phase
            exec_join_step(state)
        }
        JoinExecPhase::JoinMatching => {
            warning!("ParadeDB: In join matching phase");

            // Check if this is a TopN join query
            if let Some(ref join_state) = state.custom_state().join_exec_state {
                if super::top_n_join::is_topn_join(join_state) {
                    warning!("ParadeDB: Detected TopN join query, using optimized execution");
                    return super::top_n_join::execute_topn_join(state);
                }
            }

            // Check if we should use lazy join execution
            if crate::gucs::is_lazy_join_loading_enabled() {
                // Check if this query is suitable for lazy loading
                if let Some(ref join_state) = state.custom_state().join_exec_state {
                    if join_state.is_search_join() {
                        warning!("ParadeDB: Checking if lazy join execution is beneficial");
                        // Use lazy join execution if appropriate
                        return super::lazy_join::execute_lazy_join(state);
                    }
                }
            }

            // Log current positions
            if let Some(ref join_state) = state.custom_state().join_exec_state {
                let outer_len = join_state
                    .outer_results
                    .as_ref()
                    .map(|r| r.len())
                    .unwrap_or(0);
                let inner_len = join_state
                    .inner_results
                    .as_ref()
                    .map(|r| r.len())
                    .unwrap_or(0);
                warning!(
                    "ParadeDB: Current positions - outer: {}/{}, inner: {}/{}",
                    join_state.outer_position,
                    outer_len,
                    join_state.inner_position,
                    inner_len
                );
            }

            // Fall back to standard join execution
            super::match_and_return_next_tuple(state)
        }
        JoinExecPhase::Finished => {
            warning!("ParadeDB: Join execution finished");
            std::ptr::null_mut()
        }
        _ => {
            warning!("ParadeDB: Unhandled join phase: {:?}", current_phase);
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
