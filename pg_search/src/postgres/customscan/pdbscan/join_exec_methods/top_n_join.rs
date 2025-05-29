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

//! TopN join execution for queries with LIMIT and ORDER BY
//!
//! This module implements an optimized execution path for join queries that have
//! both LIMIT and ORDER BY clauses, similar to TopNScanExecState but for joins.

use crate::postgres::customscan::builders::custom_path::SortDirection;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::pdbscan::PdbScan;
use pgrx::{pg_sys, warning};
use std::cmp::Ordering;

use super::state::JoinExecState;

// Similar to TopNScanExecState constants
const SUBSEQUENT_RETRY_SCALE_FACTOR: usize = 2;
const MAX_CHUNK_SIZE: usize = 10000; // More conservative for joins

/// Represents a join match with score for sorting
#[derive(Debug, Clone)]
struct ScoredJoinMatch {
    outer_ctid: u64,
    inner_ctid: u64,
    outer_score: f32,
    inner_score: f32,
    combined_score: f32,
    outer_idx: usize,
    inner_idx: usize,
}

impl ScoredJoinMatch {
    fn new(
        outer_ctid: u64,
        inner_ctid: u64,
        outer_score: f32,
        inner_score: f32,
        outer_idx: usize,
        inner_idx: usize,
    ) -> Self {
        Self {
            outer_ctid,
            inner_ctid,
            outer_score,
            inner_score,
            combined_score: outer_score + inner_score,
            outer_idx,
            inner_idx,
        }
    }
}

/// State for TopN join execution
pub struct TopNJoinExecState {
    // Core parameters
    limit: usize,
    sort_direction: SortDirection,
    need_scores: bool,

    // Join matches buffer
    matches: Vec<ScoredJoinMatch>,
    current_match_idx: usize,

    // Tracking
    found_visible: usize,
    total_evaluated: usize,
    chunk_size: usize,
    retry_count: usize,

    // State
    initialized: bool,
    exhausted: bool,
}

impl TopNJoinExecState {
    /// Create a new TopN join execution state
    pub fn new(limit: usize, sort_direction: Option<SortDirection>, need_scores: bool) -> Self {
        Self {
            limit,
            sort_direction: sort_direction.unwrap_or(SortDirection::Desc),
            need_scores,
            matches: Vec::new(),
            current_match_idx: 0,
            found_visible: 0,
            total_evaluated: 0,
            chunk_size: 0,
            retry_count: 0,
            initialized: false,
            exhausted: false,
        }
    }

    /// Execute the TopN join algorithm
    pub unsafe fn execute(
        &mut self,
        state: &mut CustomScanStateWrapper<PdbScan>,
    ) -> *mut pg_sys::TupleTableSlot {
        if !self.initialized {
            self.initialize(state);
            self.initialized = true;
        }

        loop {
            // Check if we've found enough visible tuples
            if self.found_visible >= self.limit {
                warning!(
                    "ParadeDB: TopN join found {} visible tuples, limit reached",
                    self.found_visible
                );
                return std::ptr::null_mut();
            }

            // Try to get next match from buffer
            if let Some(matched_tuple) = self.get_next_match(state) {
                return matched_tuple;
            }

            // If we're exhausted and no more matches, we're done
            if self.exhausted {
                warning!(
                    "ParadeDB: TopN join exhausted after {} evaluations",
                    self.total_evaluated
                );
                return std::ptr::null_mut();
            }

            // Need more matches - expand the search
            self.expand_search(state);
        }
    }

    /// Initialize the TopN join execution
    unsafe fn initialize(&mut self, state: &mut CustomScanStateWrapper<PdbScan>) {
        warning!(
            "ParadeDB: Initializing TopN join execution with limit {}",
            self.limit
        );

        // Initial chunk size based on limit
        self.chunk_size = (self.limit * 2).min(1000);

        // Perform initial search and matching
        self.perform_join_matching(state, self.chunk_size);
    }

    /// Get next match from the buffer
    unsafe fn get_next_match(
        &mut self,
        state: &mut CustomScanStateWrapper<PdbScan>,
    ) -> Option<*mut pg_sys::TupleTableSlot> {
        while self.current_match_idx < self.matches.len() {
            let match_info = &self.matches[self.current_match_idx];
            self.current_match_idx += 1;

            // Check visibility and create tuple
            if let Some(tuple) = self.create_and_check_tuple(state, match_info) {
                self.found_visible += 1;
                return Some(tuple);
            }
        }

        None
    }

    /// Expand the search when we need more results
    unsafe fn expand_search(&mut self, state: &mut CustomScanStateWrapper<PdbScan>) {
        self.retry_count += 1;

        warning!(
            "ParadeDB: TopN join expanding search, retry #{}",
            self.retry_count
        );

        // Calculate new chunk size with scaling factor
        let factor = if self.chunk_size == 0 {
            // First expansion - be conservative
            2
        } else {
            SUBSEQUENT_RETRY_SCALE_FACTOR
        };

        self.chunk_size = (self.chunk_size * factor)
            .max(self.limit * factor)
            .min(MAX_CHUNK_SIZE);

        warning!("ParadeDB: Expanding chunk size to {}", self.chunk_size);

        // Clear current matches and get more
        self.matches.clear();
        self.current_match_idx = 0;

        self.perform_join_matching(state, self.chunk_size);
    }

    /// Perform join matching up to the specified limit
    unsafe fn perform_join_matching(
        &mut self,
        state: &mut CustomScanStateWrapper<PdbScan>,
        limit: usize,
    ) {
        let join_state = match state.custom_state_mut().join_exec_state.as_mut() {
            Some(state) => state,
            None => {
                self.exhausted = true;
                return;
            }
        };

        // Get search results - clone to avoid borrowing issues
        let outer_results = join_state.outer_results.clone().unwrap_or_default();
        let inner_results = join_state.inner_results.clone().unwrap_or_default();

        if outer_results.is_empty() || inner_results.is_empty() {
            warning!(
                "ParadeDB: TopN join has empty results - outer: {}, inner: {}",
                outer_results.len(),
                inner_results.len()
            );
            self.exhausted = true;
            return;
        }

        warning!(
            "ParadeDB: TopN join matching with {} outer x {} inner results",
            outer_results.len(),
            inner_results.len()
        );

        // Collect all potential matches with scores
        let mut all_matches = Vec::new();

        for (outer_idx, (outer_ctid, outer_score)) in outer_results.iter().enumerate() {
            for (inner_idx, (inner_ctid, inner_score)) in inner_results.iter().enumerate() {
                self.total_evaluated += 1;

                // Evaluate join condition
                if super::evaluate_join_condition(state, *outer_ctid, *inner_ctid) {
                    all_matches.push(ScoredJoinMatch::new(
                        *outer_ctid,
                        *inner_ctid,
                        *outer_score,
                        *inner_score,
                        outer_idx,
                        inner_idx,
                    ));
                }

                // Early exit if we have enough matches
                if all_matches.len() >= limit * 2 {
                    break;
                }
            }

            if all_matches.len() >= limit * 2 {
                break;
            }
        }

        warning!(
            "ParadeDB: Found {} join matches out of {} evaluated",
            all_matches.len(),
            self.total_evaluated
        );

        // Sort matches by score
        self.sort_matches(&mut all_matches);

        // Take top N matches
        self.matches = all_matches.into_iter().take(limit).collect();

        // Check if we've evaluated everything
        if outer_results.len() * inner_results.len() <= self.total_evaluated {
            warning!("ParadeDB: TopN join has evaluated all possible combinations");
            self.exhausted = true;
        }
    }

    /// Sort matches according to sort direction
    fn sort_matches(&self, matches: &mut Vec<ScoredJoinMatch>) {
        match self.sort_direction {
            SortDirection::Asc => {
                matches.sort_by(|a, b| {
                    a.combined_score
                        .partial_cmp(&b.combined_score)
                        .unwrap_or(Ordering::Equal)
                });
            }
            SortDirection::Desc => {
                matches.sort_by(|a, b| {
                    b.combined_score
                        .partial_cmp(&a.combined_score)
                        .unwrap_or(Ordering::Equal)
                });
            }
            SortDirection::None => {
                // No sorting needed
            }
        }
    }

    /// Create a tuple from join match and check visibility
    unsafe fn create_and_check_tuple(
        &self,
        state: &mut CustomScanStateWrapper<PdbScan>,
        match_info: &ScoredJoinMatch,
    ) -> Option<*mut pg_sys::TupleTableSlot> {
        // Create the join result tuple
        let slot =
            super::create_join_result_tuple(state, match_info.outer_idx, match_info.inner_idx);

        if !slot.is_null() {
            // Update score if needed
            if self.need_scores && !slot.is_null() {
                // Store the combined score for later use
                // This would be picked up by score placeholder injection
                if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
                    // Store score in stats temporarily (hack for now)
                    // In production, we'd have a proper place for this
                    join_state.stats.join_matches = match_info.combined_score as u32;
                }
            }

            Some(slot)
        } else {
            None
        }
    }

    /// Reset the TopN state for rescans
    pub fn reset(&mut self) {
        warning!("ParadeDB: Resetting TopN join state");

        self.matches.clear();
        self.current_match_idx = 0;
        self.found_visible = 0;
        self.total_evaluated = 0;
        self.chunk_size = 0;
        self.retry_count = 0;
        self.initialized = false;
        self.exhausted = false;
    }
}

/// Check if this join query is suitable for TopN execution
pub fn is_topn_join(join_state: &JoinExecState) -> bool {
    // Check if the optimization is enabled
    if !crate::gucs::is_topn_join_optimization_enabled() {
        return false;
    }

    // Must have a limit
    let has_limit = join_state.limit.is_some();

    // Must be a search join (has search predicates)
    let is_search_join = join_state.is_search_join();

    // For now, we only handle bilateral search joins
    let is_bilateral = join_state.is_bilateral_search();

    has_limit && is_search_join && is_bilateral
}

/// Execute a TopN join query
pub unsafe fn execute_topn_join(
    state: &mut CustomScanStateWrapper<PdbScan>,
) -> *mut pg_sys::TupleTableSlot {
    // Check if TopN state needs initialization
    let needs_initialization = {
        if let Some(ref join_state) = state.custom_state().join_exec_state {
            join_state.topn_state.is_none()
        } else {
            return std::ptr::null_mut();
        }
    };

    if needs_initialization {
        // Initialize TopN state
        let limit = state
            .custom_state()
            .join_exec_state
            .as_ref()
            .and_then(|js| js.limit)
            .unwrap_or(1000.0) as usize;

        let sort_direction = state.custom_state().sort_direction;
        let need_scores = state.custom_state().need_scores();

        warning!(
            "ParadeDB: Creating new TopN join state with limit {}, sort_direction: {:?}, need_scores: {}",
            limit, sort_direction, need_scores
        );

        let topn_state = TopNJoinExecState::new(limit, sort_direction, need_scores);

        // Store it in the join execution state
        if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
            join_state.topn_state = Some(topn_state);
        }
    }

    // Execute using the cached state
    // We need to extract the state temporarily to avoid the double borrow
    let mut topn_state = if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state
    {
        join_state.topn_state.take()
    } else {
        None
    };

    if let Some(ref mut topn_exec_state) = topn_state {
        let result = topn_exec_state.execute(state);

        // Put the state back
        if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
            join_state.topn_state = topn_state;
        }

        result
    } else {
        warning!("ParadeDB: TopN join state not available after initialization");
        std::ptr::null_mut()
    }
}
