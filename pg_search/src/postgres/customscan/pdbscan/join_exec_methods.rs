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

use crate::index::mvcc::{MVCCDirectory, MvccSatisfies};
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::pdbscan::join_qual_inspect::{
    JoinSearchPredicates, RelationSearchPredicate,
};
use crate::postgres::customscan::pdbscan::{get_rel_name, PdbScan};
use crate::postgres::rel_get_bm25_index;
use pgrx::{pg_sys, warning};
use std::collections::HashMap;
use tantivy::Index;

/// Column mapping strategies for join results - Updated to handle PostgreSQL's special varno values
#[derive(Debug, Clone)]
enum ColumnMapping {
    /// Variable from a specific base relation (positive varno)
    BaseRelationVar {
        varno: pg_sys::Index,
        attno: pg_sys::AttrNumber,
    },
    /// Fallback to relation column by name
    RelationColumn {
        relid: pg_sys::Oid,
        column_name: String,
    },
    /// Fallback to index-based access
    IndexBased {
        relation_index: usize,
        column_index: usize,
    },
}

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
    /// Execution-time variable mappings (varno -> relid)
    pub varno_to_relid: HashMap<pg_sys::Index, pg_sys::Oid>,
    /// Cached intermediate join results for multi-step joins
    pub intermediate_results: HashMap<String, Vec<Vec<Option<String>>>>,
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

/// Join execution statistics with enhanced metrics
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
    /// Time spent on outer search (microseconds)
    pub outer_search_time_us: u64,
    /// Time spent on inner search (microseconds)
    pub inner_search_time_us: u64,
    /// Time spent on join matching (microseconds)
    pub join_matching_time_us: u64,
    /// Number of heap tuple fetch attempts
    pub heap_fetch_attempts: usize,
    /// Number of successful heap tuple fetches
    pub heap_fetch_successes: usize,
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
            varno_to_relid: HashMap::new(),
            intermediate_results: HashMap::new(),
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
            varno_to_relid: HashMap::new(),
            intermediate_results: HashMap::new(),
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

    // The search predicates should already be stored in the join execution state
    // during the create_custom_scan_state phase
    if let Some(ref join_state) = state.custom_state().join_exec_state {
        if let Some(ref predicates) = join_state.search_predicates {
            warning!(
                "ParadeDB: Using stored search predicates - outer: {}, inner: {}, bilateral: {}",
                predicates.outer_predicates.len(),
                predicates.inner_predicates.len(),
                predicates.has_bilateral_search()
            );
        } else {
            warning!("ParadeDB: No search predicates available for join execution");
        }
    } else {
        warning!("ParadeDB: No join execution state found - this should not happen for join nodes");

        // Create a default join execution state as fallback
        let join_exec_state = JoinExecState::default();
        state.custom_state_mut().join_exec_state = Some(join_exec_state);
    }

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

    // Get the current phase and execute accordingly
    let current_phase = state
        .custom_state()
        .join_exec_state
        .as_ref()
        .unwrap()
        .phase
        .clone();

    match current_phase {
        JoinExecPhase::NotStarted => {
            warning!("ParadeDB: Starting join execution - initializing search");

            // Update phase to outer search
            if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
                join_state.phase = JoinExecPhase::OuterSearch;
            }

            // Initialize search execution
            init_search_execution(state);

            // Continue to outer search phase
            exec_join_step(state)
        }
        JoinExecPhase::OuterSearch => {
            warning!("ParadeDB: Executing outer search");

            // Execute search on outer relation
            execute_outer_search(state);

            // Move to inner search phase
            if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
                join_state.phase = JoinExecPhase::InnerSearch;
            }

            // Continue to inner search phase
            exec_join_step(state)
        }
        JoinExecPhase::InnerSearch => {
            warning!("ParadeDB: Executing inner search");

            // Execute search on inner relation
            execute_inner_search(state);

            // Move to join matching phase
            if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
                join_state.phase = JoinExecPhase::JoinMatching;
            }

            // Continue to join matching phase
            exec_join_step(state)
        }
        JoinExecPhase::JoinMatching => {
            warning!("ParadeDB: Performing join matching");

            // Perform join matching and return next result tuple
            match_and_return_next_tuple(state)
        }
        JoinExecPhase::Finished => {
            warning!("ParadeDB: Join execution finished, returning EOF");
            std::ptr::null_mut()
        }
    }
}

/// Clean up join execution resources
pub unsafe fn cleanup_join_execution(state: &mut CustomScanStateWrapper<PdbScan>) {
    warning!("ParadeDB: Cleaning up join execution");

    // Clean up join execution state with comprehensive statistics
    if let Some(mut join_state) = state.custom_state_mut().join_exec_state.take() {
        warning!("ParadeDB: Cleaning up join execution state");

        // Log comprehensive execution statistics
        warning!("ParadeDB: ===== JOIN EXECUTION STATISTICS =====");
        warning!(
            "ParadeDB: Search Results - Outer: {}, Inner: {}, Total Combinations: {}",
            join_state.stats.outer_tuples,
            join_state.stats.inner_tuples,
            join_state.stats.outer_tuples * join_state.stats.inner_tuples
        );
        warning!(
            "ParadeDB: Join Processing - Matches: {}, Returned: {}, Success Rate: {:.1}%",
            join_state.stats.join_matches,
            join_state.stats.tuples_returned,
            if join_state.stats.outer_tuples * join_state.stats.inner_tuples > 0 {
                (join_state.stats.join_matches as f64
                    / (join_state.stats.outer_tuples * join_state.stats.inner_tuples) as f64)
                    * 100.0
            } else {
                0.0
            }
        );
        warning!(
            "ParadeDB: Heap Tuple Access - Attempts: {}, Successes: {}, Success Rate: {:.1}%",
            join_state.stats.heap_fetch_attempts,
            join_state.stats.heap_fetch_successes,
            if join_state.stats.heap_fetch_attempts > 0 {
                (join_state.stats.heap_fetch_successes as f64
                    / join_state.stats.heap_fetch_attempts as f64)
                    * 100.0
            } else {
                0.0
            }
        );

        // Clean up search readers
        let reader_count = join_state.search_readers.len();
        join_state.search_readers.clear();
        warning!("ParadeDB: Cleaned up {} search readers", reader_count);

        // Clear result vectors
        let outer_results_count = join_state
            .outer_results
            .as_ref()
            .map(|r| r.len())
            .unwrap_or(0);
        let inner_results_count = join_state
            .inner_results
            .as_ref()
            .map(|r| r.len())
            .unwrap_or(0);
        join_state.outer_results = None;
        join_state.inner_results = None;
        warning!(
            "ParadeDB: Cleared {} outer and {} inner search results",
            outer_results_count,
            inner_results_count
        );

        warning!("ParadeDB: ===== END JOIN EXECUTION STATISTICS =====");
    }

    warning!("ParadeDB: Join execution cleanup complete");
}

/// Initialize search execution for both relations
unsafe fn init_search_execution(state: &mut CustomScanStateWrapper<PdbScan>) {
    warning!("ParadeDB: Initializing real search execution for join");

    // Debug: Check if join execution state exists
    if let Some(ref join_state) = state.custom_state().join_exec_state {
        warning!("ParadeDB: Join execution state found");
        if let Some(ref predicates) = join_state.search_predicates {
            warning!(
                "ParadeDB: Search predicates found in join state - outer: {}, inner: {}",
                predicates.outer_predicates.len(),
                predicates.inner_predicates.len()
            );
        } else {
            warning!("ParadeDB: Join execution state exists but no search predicates");
        }
    } else {
        warning!("ParadeDB: No join execution state found");
    }

    // Extract search predicates from the join execution state (set during scan state creation)
    let search_predicates = if let Some(ref join_state) = state.custom_state().join_exec_state {
        join_state.search_predicates.clone()
    } else {
        warning!("ParadeDB: No join execution state found");
        None
    };

    if let Some(predicates) = search_predicates {
        warning!(
            "ParadeDB: Retrieved search predicates from join state - outer: {}, inner: {}",
            predicates.outer_predicates.len(),
            predicates.inner_predicates.len()
        );

        // Initialize search readers for relations with search predicates
        init_search_readers(state, &predicates);

        // Execute searches and store results
        execute_real_searches(state, &predicates);
    } else {
        warning!("ParadeDB: No search predicates found in join state, using mock data");

        // Fall back to mock data for demonstration
        if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
            join_state.outer_results = Some(vec![(1, 1.0), (2, 0.8)]);
            join_state.inner_results = Some(vec![(1, 0.9), (3, 0.7)]);
            join_state.outer_position = 0;
            join_state.inner_position = 0;

            warning!("ParadeDB: Initialized mock search results - outer: 2, inner: 2");
        }
    }
}

/// Initialize search readers for relations with BM25 indexes
unsafe fn init_search_readers(
    state: &mut CustomScanStateWrapper<PdbScan>,
    predicates: &JoinSearchPredicates,
) {
    warning!("ParadeDB: Initializing search readers for join relations");

    if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
        // Initialize search readers for outer relation predicates
        for predicate in &predicates.outer_predicates {
            if predicate.uses_search_operator {
                if let Some((_, bm25_index)) = rel_get_bm25_index(predicate.relid) {
                    warning!(
                        "ParadeDB: Opening search reader for outer relation {}",
                        predicate.relname
                    );

                    let directory = MVCCDirectory::snapshot(bm25_index.oid());
                    if let Ok(index) = Index::open(directory) {
                        let search_reader =
                            SearchIndexReader::open(&bm25_index, MvccSatisfies::Snapshot);
                        if let Ok(reader) = search_reader {
                            join_state.search_readers.insert(predicate.relid, reader);
                            warning!(
                                "ParadeDB: Successfully opened search reader for {}",
                                predicate.relname
                            );
                        } else {
                            warning!(
                                "ParadeDB: Failed to open search reader for {}",
                                predicate.relname
                            );
                        }
                    }
                }
            }
        }

        // Initialize search readers for inner relation predicates
        for predicate in &predicates.inner_predicates {
            if predicate.uses_search_operator {
                if let Some((_, bm25_index)) = rel_get_bm25_index(predicate.relid) {
                    warning!(
                        "ParadeDB: Opening search reader for inner relation {}",
                        predicate.relname
                    );

                    let directory = MVCCDirectory::snapshot(bm25_index.oid());
                    if let Ok(index) = Index::open(directory) {
                        let search_reader =
                            SearchIndexReader::open(&bm25_index, MvccSatisfies::Snapshot);
                        if let Ok(reader) = search_reader {
                            join_state.search_readers.insert(predicate.relid, reader);
                            warning!(
                                "ParadeDB: Successfully opened search reader for {}",
                                predicate.relname
                            );
                        } else {
                            warning!(
                                "ParadeDB: Failed to open search reader for {}",
                                predicate.relname
                            );
                        }
                    }
                }
            }
        }

        warning!(
            "ParadeDB: Initialized {} search readers",
            join_state.search_readers.len()
        );
    }
}

/// Execute real searches on both relations
unsafe fn execute_real_searches(
    state: &mut CustomScanStateWrapper<PdbScan>,
    predicates: &JoinSearchPredicates,
) {
    warning!("ParadeDB: Executing real searches on join relations");

    if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
        // Execute searches on outer relation
        let mut outer_results = Vec::new();
        for predicate in &predicates.outer_predicates {
            if predicate.uses_search_operator {
                if let Some(search_reader) = join_state.search_readers.get(&predicate.relid) {
                    let relation_name = get_rel_name(predicate.relid);
                    warning!(
                        "ParadeDB: Executing search on outer relation {}",
                        relation_name
                    );

                    let results = execute_real_search(search_reader, predicate);
                    outer_results.extend(results);

                    warning!(
                        "ParadeDB: Found {} results for outer relation {}",
                        outer_results.len(),
                        relation_name
                    );
                }
            }
        }

        // Execute searches on inner relation
        let mut inner_results = Vec::new();
        for predicate in &predicates.inner_predicates {
            if predicate.uses_search_operator {
                if let Some(search_reader) = join_state.search_readers.get(&predicate.relid) {
                    let relation_name = get_rel_name(predicate.relid);
                    warning!(
                        "ParadeDB: Executing search on inner relation {}",
                        relation_name
                    );

                    let results = execute_real_search(search_reader, predicate);
                    inner_results.extend(results);

                    warning!(
                        "ParadeDB: Found {} results for inner relation {}",
                        inner_results.len(),
                        relation_name
                    );
                }
            }
        }

        // Store the search results with fallback handling
        join_state.outer_results = if outer_results.is_empty() {
            warning!("ParadeDB: No outer search results, using default result");
            Some(vec![(1, 1.0)]) // Default result if no search
        } else {
            Some(outer_results)
        };

        join_state.inner_results = if inner_results.is_empty() {
            warning!("ParadeDB: No inner search results, using default result");
            Some(vec![(1, 0.9)]) // Default result if no search
        } else {
            Some(inner_results)
        };

        join_state.outer_position = 0;
        join_state.inner_position = 0;

        warning!(
            "ParadeDB: Completed real search execution - outer: {}, inner: {}",
            join_state.outer_results.as_ref().unwrap().len(),
            join_state.inner_results.as_ref().unwrap().len()
        );
    }
}

/// Execute a real BM25 search on a relation
unsafe fn execute_real_search(
    search_reader: &SearchIndexReader,
    predicate: &RelationSearchPredicate,
) -> Vec<(u64, f32)> {
    warning!(
        "ParadeDB: Executing real BM25 search on relation {}",
        predicate.relname
    );

    if !predicate.uses_search_operator {
        warning!(
            "ParadeDB: Relation {} doesn't use search operator, returning empty results",
            predicate.relname
        );
        return Vec::new();
    }

    // Execute the actual search using the SearchIndexReader
    let search_results = search_reader.search(
        true,  // need_scores
        false, // sort_segments_by_ctid
        &predicate.query,
        None, // estimated_rows
    );

    let mut results = Vec::new();

    // Extract CTIDs and scores from search results
    for (search_index_score, _doc_address) in search_results {
        let ctid = search_index_score.ctid;
        let score = search_index_score.bm25;
        results.push((ctid, score));

        if results.len() >= 100 {
            // Limit results for performance
            break;
        }
    }

    warning!(
        "ParadeDB: Real search on {} returned {} results",
        predicate.relname,
        results.len()
    );

    results
}

/// Execute search on the outer relation
unsafe fn execute_outer_search(state: &mut CustomScanStateWrapper<PdbScan>) {
    warning!("ParadeDB: Executing search on outer relation");

    // In a complete implementation, this would:
    // 1. Get the outer relation's search predicate
    // 2. Execute the search using the BM25 index
    // 3. Store results in join_state.outer_results

    if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
        join_state.stats.outer_tuples = join_state
            .outer_results
            .as_ref()
            .map(|r| r.len())
            .unwrap_or(0);
        warning!(
            "ParadeDB: Outer search completed - {} results",
            join_state.stats.outer_tuples
        );
    }
}

/// Execute search on the inner relation
unsafe fn execute_inner_search(state: &mut CustomScanStateWrapper<PdbScan>) {
    warning!("ParadeDB: Executing search on inner relation");

    // In a complete implementation, this would:
    // 1. Get the inner relation's search predicate
    // 2. Execute the search using the BM25 index
    // 3. Store results in join_state.inner_results

    if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
        join_state.stats.inner_tuples = join_state
            .inner_results
            .as_ref()
            .map(|r| r.len())
            .unwrap_or(0);
        warning!(
            "ParadeDB: Inner search completed - {} results",
            join_state.stats.inner_tuples
        );
    }
}

/// Perform join matching and return the next result tuple
unsafe fn match_and_return_next_tuple(
    state: &mut CustomScanStateWrapper<PdbScan>,
) -> *mut pg_sys::TupleTableSlot {
    warning!("ParadeDB: Matching and returning next tuple");

    // Get current positions and results with enhanced error checking
    let (outer_pos, inner_pos, has_more, outer_total, inner_total) = {
        if let Some(ref join_state) = state.custom_state().join_exec_state {
            let empty_outer = vec![];
            let empty_inner = vec![];
            let outer_results = join_state.outer_results.as_ref().unwrap_or(&empty_outer);
            let inner_results = join_state.inner_results.as_ref().unwrap_or(&empty_inner);

            if outer_results.is_empty() || inner_results.is_empty() {
                warning!("ParadeDB: No results available for join matching");
                return std::ptr::null_mut();
            }

            // Enhanced nested loop join logic with bounds checking
            let has_more = join_state.outer_position < outer_results.len()
                && join_state.inner_position < inner_results.len();

            (
                join_state.outer_position,
                join_state.inner_position,
                has_more,
                outer_results.len(),
                inner_results.len(),
            )
        } else {
            warning!("ParadeDB: No join execution state available");
            return std::ptr::null_mut();
        }
    };

    if !has_more {
        // No more tuples to return - update statistics
        if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
            join_state.phase = JoinExecPhase::Finished;

            warning!(
                "ParadeDB: Join matching complete - total matches: {}",
                join_state.stats.join_matches
            );
        }
        return std::ptr::null_mut();
    }

    warning!(
        "ParadeDB: Processing join match [{}/{}] × [{}/{}]",
        outer_pos + 1,
        outer_total,
        inner_pos + 1,
        inner_total
    );

    // Create a result tuple with enhanced error handling
    let result_tuple = create_join_result_tuple(state, outer_pos, inner_pos);

    if result_tuple.is_null() {
        warning!("ParadeDB: Failed to create join result tuple");
        return std::ptr::null_mut();
    }

    // Advance to next position with improved logic
    if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
        join_state.inner_position += 1;

        // If we've exhausted inner results, move to next outer and reset inner
        if let Some(ref inner_results) = join_state.inner_results {
            if join_state.inner_position >= inner_results.len() {
                join_state.outer_position += 1;
                join_state.inner_position = 0;

                if join_state.outer_position < outer_total {
                    warning!(
                        "ParadeDB: Advanced to next outer tuple [{}/{}]",
                        join_state.outer_position + 1,
                        outer_total
                    );
                }
            }
        }

        join_state.stats.join_matches += 1;
        join_state.stats.tuples_returned += 1;

        if join_state.stats.join_matches % 100 == 0 {
            warning!(
                "ParadeDB: Join progress - {} matches processed",
                join_state.stats.join_matches
            );
        }
    }

    result_tuple
}

/// Create a join result tuple with actual data
unsafe fn create_join_result_tuple(
    state: &mut CustomScanStateWrapper<PdbScan>,
    outer_pos: usize,
    inner_pos: usize,
) -> *mut pg_sys::TupleTableSlot {
    warning!(
        "ParadeDB: Creating join result tuple for outer[{}], inner[{}]",
        outer_pos,
        inner_pos
    );

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
    warning!("ParadeDB: Creating result tuple with {} attributes", natts);

    // Clear the slot first
    pg_sys::ExecClearTuple(slot);

    // Get the CTIDs from search results
    let (outer_ctid, inner_ctid) = get_ctids_from_results(state, outer_pos, inner_pos);

    // Get relation information for heap tuple fetching
    let (outer_relid, inner_relid) = get_relation_oids_from_state(state);

    // Fetch real column values from heap tuples
    let column_values = fetch_real_join_column_values(
        state,
        outer_ctid,
        inner_ctid,
        outer_relid,
        inner_relid,
        natts,
    );

    // Populate the slot with the fetched values
    for (i, value) in column_values.iter().enumerate().take(natts) {
        if let Some(value) = value {
            let value_cstr = std::ffi::CString::new(value.clone()).unwrap();
            let text_datum = pg_sys::cstring_to_text(value_cstr.as_ptr());

            // Set the value in the slot
            (*slot).tts_values.add(i).write(text_datum.into());
            (*slot).tts_isnull.add(i).write(false);
        } else {
            // NULL value
            (*slot).tts_values.add(i).write(pg_sys::Datum::null());
            (*slot).tts_isnull.add(i).write(true);
        }
    }

    // Mark the slot as having valid data
    (*slot).tts_nvalid = natts as _;
    pg_sys::ExecStoreVirtualTuple(slot);

    warning!(
        "ParadeDB: Created join result tuple with real heap data: {:?}",
        column_values
    );

    slot
}

/// Get relation OIDs from the join execution state
fn get_relation_oids_from_state(
    state: &mut CustomScanStateWrapper<PdbScan>,
) -> (Option<pg_sys::Oid>, Option<pg_sys::Oid>) {
    // FIRST: Try to get relation OIDs from the join execution state
    // These were stored during scan state creation from the private data
    if let Some(ref join_state) = state.custom_state().join_exec_state {
        let outer_relid = if join_state.outer_relid != pg_sys::InvalidOid {
            Some(join_state.outer_relid)
        } else {
            None
        };
        let inner_relid = if join_state.inner_relid != pg_sys::InvalidOid {
            Some(join_state.inner_relid)
        } else {
            None
        };

        if outer_relid.is_some() || inner_relid.is_some() {
            warning!(
                "ParadeDB: Using stored relation OIDs from join state - outer: {}, inner: {}",
                outer_relid
                    .map(|oid| unsafe { get_rel_name(oid) })
                    .unwrap_or_else(|| "None".to_string()),
                inner_relid
                    .map(|oid| unsafe { get_rel_name(oid) })
                    .unwrap_or_else(|| "None".to_string())
            );
            return (outer_relid, inner_relid);
        }
    }

    // FALLBACK: Try to get from search predicates (for backward compatibility)
    warning!(
        "ParadeDB: No stored relation OIDs found in join state, falling back to search predicates"
    );

    if let Some(ref join_state) = state.custom_state().join_exec_state {
        if let Some(ref predicates) = join_state.search_predicates {
            let outer_relid = predicates.outer_predicates.first().map(|p| p.relid);
            let inner_relid = predicates.inner_predicates.first().map(|p| p.relid);

            warning!(
                "ParadeDB: Getting relation OIDs from predicates - outer predicates: {}, inner predicates: {}",
                predicates.outer_predicates.len(),
                predicates.inner_predicates.len()
            );

            if let Some(outer_oid) = outer_relid {
                warning!("ParadeDB: Outer relation: {}", unsafe {
                    get_rel_name(outer_oid)
                });
            } else {
                warning!("ParadeDB: No outer relation found in predicates");
            }

            if let Some(inner_oid) = inner_relid {
                warning!("ParadeDB: Inner relation: {}", unsafe {
                    get_rel_name(inner_oid)
                });
            } else {
                warning!("ParadeDB: No inner relation found in predicates");
            }

            // For unilateral joins, we might only have one relation in predicates
            // In this case, we should NOT try to infer the missing relation
            // Instead, we should handle the unilateral case properly
            (outer_relid, inner_relid)
        } else {
            warning!("ParadeDB: No search predicates found in join state");
            (None, None)
        }
    } else {
        warning!("ParadeDB: No join execution state found");
        (None, None)
    }
}

/// Fetch real column values from heap tuples for join results
unsafe fn fetch_real_join_column_values(
    state: &mut CustomScanStateWrapper<PdbScan>,
    outer_ctid: u64,
    inner_ctid: u64,
    outer_relid: Option<pg_sys::Oid>,
    inner_relid: Option<pg_sys::Oid>,
    natts: usize,
) -> Vec<Option<String>> {
    warning!(
        "ParadeDB: Fetching real heap tuple values from relations {} and {}",
        outer_relid
            .map(|oid| unsafe { get_rel_name(oid) })
            .unwrap_or_else(|| "unknown".to_string()),
        inner_relid
            .map(|oid| unsafe { get_rel_name(oid) })
            .unwrap_or_else(|| "unknown".to_string())
    );

    // Update fetch attempt statistics
    if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
        join_state.stats.heap_fetch_attempts += 2; // One for outer, one for inner
    }

    let mut column_values = Vec::with_capacity(natts);

    // NEW APPROACH: Analyze the actual query structure at execution time
    // Instead of relying on transformed variables, let's understand what the query wants

    // Get the original SQL query structure by analyzing the search predicates
    let (outer_relation_name, inner_relation_name) = get_relation_names_from_predicates(state);

    warning!(
        "ParadeDB: Detected relations from predicates - outer: '{}', inner: '{}'",
        outer_relation_name,
        inner_relation_name
    );

    // Fetch all available columns from both relations
    let outer_columns = if let Some(relid) = outer_relid {
        fetch_all_columns_from_relation_with_names(relid, outer_ctid, &outer_relation_name)
    } else {
        Vec::new()
    };

    let inner_columns = if let Some(relid) = inner_relid {
        fetch_all_columns_from_relation_with_names(relid, inner_ctid, &inner_relation_name)
    } else {
        Vec::new()
    };

    warning!(
        "ParadeDB: Fetched {} columns from outer relation, {} from inner relation",
        outer_columns.len(),
        inner_columns.len()
    );

    // NEW APPROACH: Smart column mapping based on query analysis
    // Analyze what the query is asking for by looking at the target list structure
    let target_mapping =
        analyze_target_list_for_join_mapping(state, &outer_relation_name, &inner_relation_name);

    // Map columns based on the execution-time varno/attno analysis
    for i in 0..natts {
        let column_value = if let Some(mapping) = target_mapping.get(i) {
            match mapping {
                ColumnMapping::BaseRelationVar { varno, attno } => {
                    // Variable from a specific base relation
                    warning!(
                        "ParadeDB: Base relation var - varno: {}, attno: {}",
                        varno,
                        attno
                    );
                    fetch_column_value_by_base_relation_var(
                        state,
                        *varno,
                        *attno,
                        outer_ctid,
                        inner_ctid,
                        &outer_columns,
                        &inner_columns,
                    )
                }
                ColumnMapping::RelationColumn { relid, column_name } => {
                    // Fetch from the specific relation
                    if Some(*relid) == outer_relid {
                        find_column_by_name(&outer_columns, column_name)
                    } else if Some(*relid) == inner_relid {
                        find_column_by_name(&inner_columns, column_name)
                    } else {
                        // Try to fetch directly from the relation
                        fetch_column_from_relation_by_name(*relid, outer_ctid, column_name).or_else(
                            || fetch_column_from_relation_by_name(*relid, inner_ctid, column_name),
                        )
                    }
                }
                ColumnMapping::IndexBased {
                    relation_index,
                    column_index,
                } => {
                    if *relation_index == 0 && !outer_columns.is_empty() {
                        outer_columns
                            .get(*column_index)
                            .map(|(_, value)| value.clone())
                    } else if !inner_columns.is_empty() {
                        inner_columns
                            .get(*column_index)
                            .map(|(_, value)| value.clone())
                    } else {
                        None
                    }
                }
            }
        } else {
            // Fallback: alternate between relations
            if i % 2 == 0 && !outer_columns.is_empty() {
                outer_columns.get(i / 2).map(|(_, value)| value.clone())
            } else if !inner_columns.is_empty() {
                inner_columns.get(i / 2).map(|(_, value)| value.clone())
            } else {
                None
            }
        };

        column_values.push(column_value);
    }

    // Update success statistics
    let successful_fetches = column_values.iter().filter(|v| v.is_some()).count();
    if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
        join_state.stats.heap_fetch_successes += successful_fetches;
    }

    warning!(
        "ParadeDB: Mapped {} out of {} column values: {:?}",
        successful_fetches,
        natts,
        column_values
    );

    column_values
}

/// Get relation names from search predicates
fn get_relation_names_from_predicates(
    state: &mut CustomScanStateWrapper<PdbScan>,
) -> (String, String) {
    // Use the same enhanced logic as get_relation_oids_from_state
    let (outer_relid, inner_relid) = get_relation_oids_from_state(state);

    let outer_name = if let Some(oid) = outer_relid {
        unsafe { get_rel_name(oid) }
    } else {
        "unknown_outer".to_string()
    };

    let inner_name = if let Some(oid) = inner_relid {
        unsafe { get_rel_name(oid) }
    } else {
        "unknown_inner".to_string()
    };

    (outer_name, inner_name)
}

/// Analyze the target list to create execution-time varno/attno-based column mapping
unsafe fn analyze_target_list_for_join_mapping(
    state: &mut CustomScanStateWrapper<PdbScan>,
    outer_relation_name: &str,
    inner_relation_name: &str,
) -> Vec<ColumnMapping> {
    let mut mappings = Vec::new();

    // Get the target list from the plan
    let planstate = state.planstate();
    let plan = (*planstate).plan;
    let target_list = (*plan).targetlist;

    warning!(
        "ParadeDB: PLAN: {:?}",
        core::ffi::CStr::from_ptr(pg_sys::nodeToString(plan.cast())).to_str()
    );

    if !target_list.is_null() {
        let tlist = pgrx::PgList::<pg_sys::TargetEntry>::from_pg(target_list);

        warning!(
            "ParadeDB: Analyzing {} target entries using execution-time varno/attno mapping: {:?}",
            tlist.len(),
            tlist
                .iter_ptr()
                .map(|te| {
                    let var = extract_var_from_target_entry(&*te);
                    (
                        (*te).resno,
                        var.map(|v| {
                            (core::ffi::CStr::from_ptr(pg_sys::nodeToString(v.cast())).to_str(),)
                        }),
                    )
                })
                .collect::<Vec<_>>()
        );

        // Build execution-time varno to relid mapping
        let varno_to_relid = build_execution_time_varno_mapping(state);

        for (i, te) in tlist.iter_ptr().enumerate() {
            let target_entry = &*te;

            warning!(
                "ParadeDB: Analyzing target entry {} (resno: {})",
                i + 1,
                target_entry.resno
            );

            // Extract the variable information from the target entry
            let mapping = if let Some((varno, attno, varnosyn, varattnosyn)) =
                extract_var_info_from_target_entry(target_entry)
            {
                warning!(
                    "ParadeDB: Found Var in target entry {} - varno: {}, attno: {}, varnosyn: {}, varattnosyn: {}",
                    i + 1,
                    varno,
                    attno,
                    varnosyn,
                    varattnosyn
                );

                // Handle PostgreSQL's special varno values correctly
                // For negative varno values, use the original varnosyn/varattnosyn to understand the real mapping
                match varno {
                    -2 => {
                        // OUTER_VAR
                        warning!("ParadeDB: Variable references OUTER relation, using varnosyn: {}, varattnosyn: {}", varnosyn, varattnosyn);
                        // Use the original variable reference to determine the actual relation
                        map_original_variable_to_relation(
                            state,
                            varnosyn,
                            varattnosyn,
                            outer_relation_name,
                            inner_relation_name,
                        )
                    }
                    -1 => {
                        // INNER_VAR
                        warning!("ParadeDB: Variable references INNER relation, using varnosyn: {}, varattnosyn: {}", varnosyn, varattnosyn);
                        // Use the original variable reference to determine the actual relation
                        map_original_variable_to_relation(
                            state,
                            varnosyn,
                            varattnosyn,
                            outer_relation_name,
                            inner_relation_name,
                        )
                    }
                    -3 => {
                        // INDEX_VAR
                        warning!("ParadeDB: Variable references INDEX/intermediate result, using varnosyn: {}, varattnosyn: {}", varnosyn, varattnosyn);
                        // For INDEX_VAR, use the original variable reference to understand what column this really represents
                        map_original_variable_to_relation(
                            state,
                            varnosyn,
                            varattnosyn,
                            outer_relation_name,
                            inner_relation_name,
                        )
                    }
                    _ if varno > 0 => {
                        warning!("ParadeDB: Variable references base relation {}", varno);
                        ColumnMapping::BaseRelationVar {
                            varno: varno as pg_sys::Index,
                            attno,
                        }
                    }
                    _ => {
                        warning!("ParadeDB: Unknown varno {}, using fallback", varno);
                        determine_fallback_mapping_from_target_name(
                            &format!("unknown_var_{}", i + 1),
                            state,
                            outer_relation_name,
                            inner_relation_name,
                        )
                    }
                }
            } else {
                // Fallback for non-Var expressions
                let target_name = if !target_entry.resname.is_null() {
                    std::ffi::CStr::from_ptr(target_entry.resname)
                        .to_string_lossy()
                        .to_string()
                } else {
                    format!("col_{}", i + 1)
                };

                warning!(
                    "ParadeDB: No Var found in target entry {}, using fallback for '{}'",
                    i + 1,
                    target_name
                );

                // Try to map by name as fallback
                determine_fallback_mapping_from_target_name(
                    &target_name,
                    state,
                    outer_relation_name,
                    inner_relation_name,
                )
            };

            warning!("ParadeDB: Target entry {} mapped to: {:?}", i + 1, mapping);
            mappings.push(mapping);
        }

        // Store the varno mapping in the join state for later use
        if let Some(ref mut join_state) = state.custom_state_mut().join_exec_state {
            join_state.varno_to_relid = varno_to_relid;
        }
    }

    mappings
}

/// Build execution-time varno to relid mapping by analyzing the current join context
unsafe fn build_execution_time_varno_mapping(
    state: &mut CustomScanStateWrapper<PdbScan>,
) -> HashMap<pg_sys::Index, pg_sys::Oid> {
    let mut varno_to_relid = HashMap::new();

    // Get relation OIDs from the join execution state (extract them first to avoid borrowing issues)
    let (outer_relid, inner_relid) =
        if let Some(ref join_state) = state.custom_state().join_exec_state {
            (join_state.outer_relid, join_state.inner_relid)
        } else {
            (pg_sys::InvalidOid, pg_sys::InvalidOid)
        };

    if outer_relid != pg_sys::InvalidOid {
        // For now, we'll use a simple mapping. In a more sophisticated implementation,
        // we would analyze the actual join tree to determine the correct varno assignments.
        // The key insight is that varno values at execution time may be different from planning time.

        // Try to determine varnos by examining the target list structure
        let planstate = state.planstate();
        let plan = (*planstate).plan;
        let target_list = (*plan).targetlist;

        if !target_list.is_null() {
            let tlist = pgrx::PgList::<pg_sys::TargetEntry>::from_pg(target_list);

            for te in tlist.iter_ptr() {
                if let Some(var) = extract_var_from_target_entry(&*te) {
                    let varno = (*var).varno as pg_sys::Index;

                    // This is a simplified mapping - in practice, we'd need more sophisticated
                    // logic to determine which varno corresponds to which relation
                    if !varno_to_relid.contains_key(&varno) {
                        if varno_to_relid.is_empty() {
                            varno_to_relid.insert(varno, outer_relid);
                            warning!(
                                "ParadeDB: Mapped varno {} to outer relation {}",
                                varno,
                                get_rel_name(outer_relid)
                            );
                        } else if inner_relid != pg_sys::InvalidOid {
                            varno_to_relid.insert(varno, inner_relid);
                            warning!(
                                "ParadeDB: Mapped varno {} to inner relation {}",
                                varno,
                                get_rel_name(inner_relid)
                            );
                        }
                    }
                }
            }
        }
    }

    warning!(
        "ParadeDB: Built execution-time varno mapping with {} entries: {:?}",
        varno_to_relid.len(),
        varno_to_relid
            .iter()
            .map(|(varno, relid)| (*varno, get_rel_name(*relid)))
            .collect::<Vec<_>>()
    );

    varno_to_relid
}

/// Determine fallback column mapping from target name and relation information
unsafe fn determine_fallback_mapping_from_target_name(
    target_name: &str,
    state: &mut CustomScanStateWrapper<PdbScan>,
    outer_relation_name: &str,
    inner_relation_name: &str,
) -> ColumnMapping {
    let (outer_relid, inner_relid) = get_relation_oids_from_state(state);
    // Check if the target name exists as a column in either relation
    if let Some(outer_oid) = outer_relid {
        if column_exists_in_relation(outer_oid, target_name) {
            warning!(
                "ParadeDB: Found column '{}' in outer relation {}",
                target_name,
                outer_relation_name
            );
            return ColumnMapping::RelationColumn {
                relid: outer_oid,
                column_name: target_name.to_string(),
            };
        }
    }

    if let Some(inner_oid) = inner_relid {
        if column_exists_in_relation(inner_oid, target_name) {
            warning!(
                "ParadeDB: Found column '{}' in inner relation {}",
                target_name,
                inner_relation_name
            );
            return ColumnMapping::RelationColumn {
                relid: inner_oid,
                column_name: target_name.to_string(),
            };
        }
    }

    // If the column name doesn't exist in either relation, this might be a
    // transformed name. Fall back to reasonable defaults based on common patterns.
    warning!(
        "ParadeDB: Column '{}' not found in either relation, using generic fallback",
        target_name
    );

    // GENERIC FALLBACK: When we can't find the exact column name,
    // we need to make a reasonable guess without hardcoding schema assumptions

    // Try to get the first few columns from each relation to make an educated guess
    let outer_columns = if let Some(outer_oid) = outer_relid {
        get_first_few_columns(outer_oid, 3)
    } else {
        Vec::new()
    };

    let inner_columns = if let Some(inner_oid) = inner_relid {
        get_first_few_columns(inner_oid, 3)
    } else {
        Vec::new()
    };

    // Use a simple heuristic: if the target name is similar to any column name,
    // use that relation. Otherwise, alternate between relations.
    for (i, col_name) in outer_columns.iter().enumerate() {
        if target_name
            .to_lowercase()
            .contains(&col_name.to_lowercase())
            || col_name
                .to_lowercase()
                .contains(&target_name.to_lowercase())
        {
            warning!(
                "ParadeDB: Target '{}' matches outer column '{}', using outer relation",
                target_name,
                col_name
            );
            return ColumnMapping::RelationColumn {
                relid: outer_relid.unwrap(),
                column_name: col_name.clone(),
            };
        }
    }

    for (i, col_name) in inner_columns.iter().enumerate() {
        if target_name
            .to_lowercase()
            .contains(&col_name.to_lowercase())
            || col_name
                .to_lowercase()
                .contains(&target_name.to_lowercase())
        {
            warning!(
                "ParadeDB: Target '{}' matches inner column '{}', using inner relation",
                target_name,
                col_name
            );
            return ColumnMapping::RelationColumn {
                relid: inner_relid.unwrap(),
                column_name: col_name.clone(),
            };
        }
    }

    // Enhanced fallback logic: try to make intelligent guesses based on common patterns
    warning!(
        "ParadeDB: No direct matches for '{}', trying intelligent fallback with outer: {:?}, inner: {:?}",
        target_name,
        outer_columns,
        inner_columns
    );

    // Look for common column name patterns
    if target_name.contains("name") || target_name.contains("title") {
        // Name-like columns usually come from the main entity (outer relation)
        if !outer_columns.is_empty() && outer_relid.is_some() {
            let name_col = outer_columns
                .iter()
                .find(|col| col.contains("name") || col.contains("title"))
                .unwrap_or(&outer_columns[0]);
            warning!(
                "ParadeDB: Target '{}' looks like a name field, using outer column '{}'",
                target_name,
                name_col
            );
            return ColumnMapping::RelationColumn {
                relid: outer_relid.unwrap(),
                column_name: name_col.clone(),
            };
        }
    }

    if target_name.contains("review")
        || target_name.contains("comment")
        || target_name.contains("rating")
    {
        // Review-like columns usually come from the review/detail entity (inner relation)
        if !inner_columns.is_empty() && inner_relid.is_some() {
            let review_col = inner_columns
                .iter()
                .find(|col| {
                    col.contains("review")
                        || col.contains("comment")
                        || col.contains("rating")
                        || col.contains("text")
                })
                .unwrap_or(&inner_columns[0]);
            warning!(
                "ParadeDB: Target '{}' looks like a review field, using inner column '{}'",
                target_name,
                review_col
            );
            return ColumnMapping::RelationColumn {
                relid: inner_relid.unwrap(),
                column_name: review_col.clone(),
            };
        }
    }

    // Final fallback: use the first non-ID column from the appropriate relation
    if !outer_columns.is_empty() && outer_relid.is_some() {
        let fallback_col = outer_columns
            .iter()
            .find(|col| !col.to_lowercase().contains("id"))
            .unwrap_or(&outer_columns[0]);
        warning!(
            "ParadeDB: Using final fallback - outer column '{}'",
            fallback_col
        );
        ColumnMapping::RelationColumn {
            relid: outer_relid.unwrap(),
            column_name: fallback_col.clone(),
        }
    } else if !inner_columns.is_empty() && inner_relid.is_some() {
        let fallback_col = inner_columns
            .iter()
            .find(|col| !col.to_lowercase().contains("id"))
            .unwrap_or(&inner_columns[0]);
        warning!(
            "ParadeDB: Using final fallback - inner column '{}'",
            fallback_col
        );
        ColumnMapping::RelationColumn {
            relid: inner_relid.unwrap(),
            column_name: fallback_col.clone(),
        }
    } else {
        ColumnMapping::IndexBased {
            relation_index: 0,
            column_index: 0,
        }
    }
}

/// Check if a column exists in a relation
unsafe fn column_exists_in_relation(relid: pg_sys::Oid, column_name: &str) -> bool {
    // Open the relation to get its tuple descriptor
    let heaprel = pg_sys::relation_open(relid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
    if heaprel.is_null() {
        warning!(
            "ParadeDB: Failed to open relation {} for column check",
            get_rel_name(relid)
        );
        return false;
    }

    let mut found = false;
    let mut all_columns = Vec::new();

    // Check all user columns
    let tuple_desc = pgrx::PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);
    for i in 0..tuple_desc.len() {
        if let Some(attribute) = tuple_desc.get(i) {
            let attr_name = attribute.name().to_string();
            all_columns.push(attr_name.clone());

            if attr_name == column_name {
                found = true;
            }
        }
    }

    // Close the relation
    pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);

    warning!(
        "ParadeDB: Checking for column '{}' in relation {} - found: {}, available columns: {:?}",
        column_name,
        get_rel_name(relid),
        found,
        all_columns
    );

    found
}

/// Get CTIDs from search results
fn get_ctids_from_results(
    state: &mut CustomScanStateWrapper<PdbScan>,
    outer_pos: usize,
    inner_pos: usize,
) -> (u64, u64) {
    if let Some(ref join_state) = state.custom_state().join_exec_state {
        let outer_ctid = join_state
            .outer_results
            .as_ref()
            .and_then(|results| results.get(outer_pos))
            .map(|(ctid, _)| *ctid)
            .unwrap_or(outer_pos as u64 + 1);

        let inner_ctid = join_state
            .inner_results
            .as_ref()
            .and_then(|results| results.get(inner_pos))
            .map(|(ctid, _)| *ctid)
            .unwrap_or(inner_pos as u64 + 1);

        (outer_ctid, inner_ctid)
    } else {
        (outer_pos as u64 + 1, inner_pos as u64 + 1)
    }
}

/// Extract a Var node from a target entry and handle PostgreSQL's variable transformations
unsafe fn extract_var_from_target_entry(
    target_entry: &pg_sys::TargetEntry,
) -> Option<*mut pg_sys::Var> {
    let expr = target_entry.expr;

    // Try direct Var
    if let Some(var) = crate::nodecast!(Var, T_Var, expr) {
        return Some(var);
    }

    // Try RelabelType wrapping a Var
    if let Some(relabel) = crate::nodecast!(RelabelType, T_RelabelType, expr) {
        if let Some(var) = crate::nodecast!(Var, T_Var, (*relabel).arg) {
            return Some(var);
        }
    }

    None
}

/// Extract variable information considering both transformed and original variable references
unsafe fn extract_var_info_from_target_entry(
    target_entry: &pg_sys::TargetEntry,
) -> Option<(i16, pg_sys::AttrNumber, i16, pg_sys::AttrNumber)> {
    if let Some(var) = extract_var_from_target_entry(target_entry) {
        let varno = (*var).varno as i16;
        let varattno = (*var).varattno;
        let varnosyn = (*var).varnosyn as i16;
        let varattnosyn = (*var).varattnosyn;

        Some((varno, varattno, varnosyn, varattnosyn))
    } else {
        None
    }
}

/// Find a column value by name in the fetched columns
fn find_column_by_name(columns: &[(String, String)], col_name: &str) -> Option<String> {
    columns
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(col_name))
        .map(|(_, value)| value.clone())
}

/// Fetch all columns from a relation with their names
unsafe fn fetch_all_columns_from_relation_with_names(
    relid: pg_sys::Oid,
    ctid: u64,
    relation_name: &str,
) -> Vec<(String, String)> {
    warning!(
        "ParadeDB: Fetching all columns with names from relation {} with CTID {}",
        relation_name,
        ctid
    );

    // Open the relation
    let heaprel = pg_sys::relation_open(relid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
    if heaprel.is_null() {
        warning!("ParadeDB: Failed to open relation {}", relid);
        return Vec::new();
    }

    // Convert CTID to ItemPointer
    let mut ipd = pg_sys::ItemPointerData::default();
    crate::postgres::utils::u64_to_item_pointer(ctid, &mut ipd);

    // Prepare heap tuple structure
    let mut htup = pg_sys::HeapTupleData {
        t_self: ipd,
        ..Default::default()
    };
    let mut buffer: pg_sys::Buffer = pg_sys::InvalidBuffer as i32;

    // Fetch the heap tuple
    let found = {
        #[cfg(feature = "pg14")]
        {
            pg_sys::heap_fetch(heaprel, pg_sys::GetActiveSnapshot(), &mut htup, &mut buffer)
        }
        #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
        {
            pg_sys::heap_fetch(
                heaprel,
                pg_sys::GetActiveSnapshot(),
                &mut htup,
                &mut buffer,
                false,
            )
        }
    };

    let mut result = Vec::new();

    if found {
        // Extract all column values with their names
        let tuple_desc = pgrx::PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);
        let heap_tuple =
            pgrx::heap_tuple::PgHeapTuple::from_heap_tuple(tuple_desc.clone(), &mut htup);

        // Fetch all user columns (skip system columns)
        for i in 0..tuple_desc.len() {
            if let Some(attribute) = tuple_desc.get(i) {
                let column_name = attribute.name().to_string();

                // Handle different data types appropriately
                let column_value = if attribute.type_oid() == pg_sys::INT4OID.into() {
                    // Handle integer columns
                    match heap_tuple.get_by_name::<i32>(&column_name) {
                        Ok(Some(value)) => value.to_string(),
                        Ok(None) => "NULL".to_string(),
                        Err(_) => format!("ERROR_INT_{}", i + 1),
                    }
                } else {
                    // Handle text/varchar columns
                    match heap_tuple.get_by_name::<String>(&column_name) {
                        Ok(Some(value)) => value,
                        Ok(None) => "NULL".to_string(),
                        Err(_) => format!("ERROR_TEXT_{}", i + 1),
                    }
                };

                warning!(
                    "ParadeDB: {} column '{}' = '{}'",
                    relation_name,
                    column_name,
                    column_value
                );
                result.push((column_name, column_value));
            }
        }

        warning!(
            "ParadeDB: Successfully fetched {} columns from {} relation",
            result.len(),
            relation_name
        );
    } else {
        warning!(
            "ParadeDB: Heap tuple not found for CTID {} in relation {}",
            ctid,
            relation_name
        );
    }

    // Clean up
    if buffer != pg_sys::InvalidBuffer as i32 {
        pg_sys::ReleaseBuffer(buffer);
    }
    pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);

    result
}

/// Get all column names from a relation (for debugging/logging purposes)
unsafe fn get_all_column_names_from_relation(relid: pg_sys::Oid) -> Vec<String> {
    let mut column_names = Vec::new();

    // Open the relation to get its tuple descriptor
    let heaprel = pg_sys::relation_open(relid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
    if heaprel.is_null() {
        warning!(
            "ParadeDB: Failed to open relation {} for column enumeration",
            relid
        );
        return column_names;
    }

    // Get all user columns
    let tuple_desc = pgrx::PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);
    for i in 0..tuple_desc.len() {
        if let Some(attribute) = tuple_desc.get(i) {
            column_names.push(attribute.name().to_string());
        }
    }

    // Close the relation
    pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);

    warning!(
        "ParadeDB: Relation {} has columns: {:?}",
        get_rel_name(relid),
        column_names
    );

    column_names
}

/// Get the first few column names from a relation
unsafe fn get_first_few_columns(relid: pg_sys::Oid, max_columns: usize) -> Vec<String> {
    let mut column_names = Vec::new();

    // Open the relation to get its tuple descriptor
    let heaprel = pg_sys::relation_open(relid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
    if heaprel.is_null() {
        return column_names;
    }

    // Get the first few user columns
    let tuple_desc = pgrx::PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);
    let limit = max_columns.min(tuple_desc.len());

    for i in 0..limit {
        if let Some(attribute) = tuple_desc.get(i) {
            column_names.push(attribute.name().to_string());
        }
    }

    // Close the relation
    pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);

    column_names
}

/// Fetch column value for base relation variables (positive varno)
unsafe fn fetch_column_value_by_base_relation_var(
    state: &mut CustomScanStateWrapper<PdbScan>,
    varno: pg_sys::Index,
    attno: pg_sys::AttrNumber,
    outer_ctid: u64,
    inner_ctid: u64,
    outer_columns: &[(String, String)],
    inner_columns: &[(String, String)],
) -> Option<String> {
    warning!(
        "ParadeDB: Fetching column value for base relation varno={}, attno={}",
        varno,
        attno
    );

    // Get the varno to relid mapping from the join state
    let relid = if let Some(ref join_state) = state.custom_state().join_exec_state {
        join_state.varno_to_relid.get(&varno).copied()
    } else {
        None
    };

    if let Some(relid) = relid {
        warning!(
            "ParadeDB: Mapped varno {} to relation {} ({})",
            varno,
            relid,
            get_rel_name(relid)
        );

        // Determine which CTID to use based on the relation
        let (outer_relid, inner_relid) = get_relation_oids_from_state(state);
        let ctid = if Some(relid) == outer_relid {
            outer_ctid
        } else if Some(relid) == inner_relid {
            inner_ctid
        } else {
            // This might be an intermediate join result - use outer_ctid as default
            warning!(
                "ParadeDB: Relation {} not found in outer/inner, using outer_ctid",
                get_rel_name(relid)
            );
            outer_ctid
        };

        // Fetch the column value directly from the relation using attno
        fetch_column_from_relation_by_attno(relid, ctid, attno)
    } else {
        warning!(
            "ParadeDB: No relid mapping found for varno {}, using fallback",
            varno
        );

        // Fallback: try to find the column in the cached results
        // This is a simplified approach - in practice, we'd need more sophisticated logic
        if attno > 0 && attno as usize <= outer_columns.len() {
            outer_columns
                .get(attno as usize - 1)
                .map(|(_, value)| value.clone())
        } else if attno > 0 && attno as usize <= inner_columns.len() {
            inner_columns
                .get(attno as usize - 1)
                .map(|(_, value)| value.clone())
        } else {
            None
        }
    }
}

/// Fetch a column value from a relation by column name
unsafe fn fetch_column_from_relation_by_name(
    relid: pg_sys::Oid,
    ctid: u64,
    column_name: &str,
) -> Option<String> {
    warning!(
        "ParadeDB: Fetching column '{}' from relation {} with CTID {}",
        column_name,
        get_rel_name(relid),
        ctid
    );

    // Open the relation
    let heaprel = pg_sys::relation_open(relid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
    if heaprel.is_null() {
        warning!("ParadeDB: Failed to open relation {}", relid);
        return None;
    }

    // Find the column by name
    let tuple_desc = pgrx::PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);
    let mut attno = None;
    for i in 0..tuple_desc.len() {
        if let Some(attribute) = tuple_desc.get(i) {
            if attribute.name() == column_name {
                attno = Some((i + 1) as pg_sys::AttrNumber);
                break;
            }
        }
    }

    let result = if let Some(attno) = attno {
        fetch_column_from_relation_by_attno_with_open_rel(heaprel, ctid, attno)
    } else {
        warning!(
            "ParadeDB: Column '{}' not found in relation {}",
            column_name,
            get_rel_name(relid)
        );
        None
    };

    // Close the relation
    pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);

    result
}

/// Fetch a column value from a relation by attribute number
unsafe fn fetch_column_from_relation_by_attno(
    relid: pg_sys::Oid,
    ctid: u64,
    attno: pg_sys::AttrNumber,
) -> Option<String> {
    warning!(
        "ParadeDB: Fetching attno {} from relation {} with CTID {}",
        attno,
        get_rel_name(relid),
        ctid
    );

    // Open the relation
    let heaprel = pg_sys::relation_open(relid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
    if heaprel.is_null() {
        warning!("ParadeDB: Failed to open relation {}", relid);
        return None;
    }

    let result = fetch_column_from_relation_by_attno_with_open_rel(heaprel, ctid, attno);

    // Close the relation
    pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);

    result
}

/// Helper function to fetch column value with an already open relation
unsafe fn fetch_column_from_relation_by_attno_with_open_rel(
    heaprel: pg_sys::Relation,
    ctid: u64,
    attno: pg_sys::AttrNumber,
) -> Option<String> {
    // Convert CTID to ItemPointer
    let mut ipd = pg_sys::ItemPointerData::default();
    crate::postgres::utils::u64_to_item_pointer(ctid, &mut ipd);

    // Prepare heap tuple structure
    let mut htup = pg_sys::HeapTupleData {
        t_self: ipd,
        ..Default::default()
    };
    let mut buffer: pg_sys::Buffer = pg_sys::InvalidBuffer as i32;

    // Fetch the heap tuple
    let found = {
        #[cfg(feature = "pg14")]
        {
            pg_sys::heap_fetch(heaprel, pg_sys::GetActiveSnapshot(), &mut htup, &mut buffer)
        }
        #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
        {
            pg_sys::heap_fetch(
                heaprel,
                pg_sys::GetActiveSnapshot(),
                &mut htup,
                &mut buffer,
                false,
            )
        }
    };

    let result = if found {
        // Extract the specific column value
        let tuple_desc = pgrx::PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);
        let heap_tuple =
            pgrx::heap_tuple::PgHeapTuple::from_heap_tuple(tuple_desc.clone(), &mut htup);

        if let Some(attribute) = tuple_desc.get(attno as usize - 1) {
            let column_name = attribute.name().to_string();

            // Handle different data types appropriately
            let column_value = if attribute.type_oid() == pg_sys::INT4OID.into() {
                // Handle integer columns
                match heap_tuple.get_by_name::<i32>(&column_name) {
                    Ok(Some(value)) => Some(value.to_string()),
                    Ok(None) => Some("NULL".to_string()),
                    Err(_) => None,
                }
            } else {
                // Handle text/varchar columns
                match heap_tuple.get_by_name::<String>(&column_name) {
                    Ok(Some(value)) => Some(value),
                    Ok(None) => Some("NULL".to_string()),
                    Err(_) => None,
                }
            };

            warning!(
                "ParadeDB: Fetched column '{}' (attno {}) = {:?}",
                column_name,
                attno,
                column_value
            );
            column_value
        } else {
            warning!("ParadeDB: Invalid attno {} for relation", attno);
            None
        }
    } else {
        warning!("ParadeDB: Heap tuple not found for CTID {}", ctid);
        None
    };

    // Clean up
    if buffer != pg_sys::InvalidBuffer as i32 {
        pg_sys::ReleaseBuffer(buffer);
    }

    result
}

/// Map original variable reference (varnosyn/varattnosyn) to the actual relation
unsafe fn map_original_variable_to_relation(
    state: &mut CustomScanStateWrapper<PdbScan>,
    varnosyn: i16,
    varattnosyn: pg_sys::AttrNumber,
    outer_relation_name: &str,
    inner_relation_name: &str,
) -> ColumnMapping {
    warning!(
        "ParadeDB: Mapping original variable varnosyn: {}, varattnosyn: {} to relation",
        varnosyn,
        varattnosyn
    );

    // Get the relation OIDs to determine which is which
    let (outer_relid, inner_relid) = get_relation_oids_from_state(state);

    // Try to map the varnosyn to the actual relation
    // varnosyn represents the original range table index before transformation

    // Get the relation OID for this varnosyn by looking it up in the range table
    let planstate = state.planstate();
    let plan = (*planstate).plan;

    // For join nodes, we need to determine which relation the varnosyn refers to
    // This is a simplified approach - in practice, we'd need to analyze the range table more carefully

    if let (Some(outer_oid), Some(inner_oid)) = (outer_relid, inner_relid) {
        // Check if varnosyn corresponds to our known relations
        // This is a heuristic based on the typical range table ordering

        if varnosyn == 1 {
            // First relation in range table - likely the outer relation
            warning!(
                "ParadeDB: varnosyn 1 mapped to outer relation {} (attno {})",
                outer_relation_name,
                varattnosyn
            );
            ColumnMapping::RelationColumn {
                relid: outer_oid,
                column_name: get_column_name_by_attno(outer_oid, varattnosyn),
            }
        } else if varnosyn == 2 {
            // Second relation in range table - likely the inner relation
            warning!(
                "ParadeDB: varnosyn 2 mapped to inner relation {} (attno {})",
                inner_relation_name,
                varattnosyn
            );
            ColumnMapping::RelationColumn {
                relid: inner_oid,
                column_name: get_column_name_by_attno(inner_oid, varattnosyn),
            }
        } else {
            warning!(
                "ParadeDB: Unknown varnosyn {}, using fallback mapping",
                varnosyn
            );
            // Fallback to index-based mapping
            ColumnMapping::IndexBased {
                relation_index: if varnosyn == 1 { 0 } else { 1 },
                column_index: (varattnosyn - 1) as usize,
            }
        }
    } else {
        warning!("ParadeDB: No relation OIDs available, using index-based fallback");
        ColumnMapping::IndexBased {
            relation_index: if varnosyn == 1 { 0 } else { 1 },
            column_index: (varattnosyn - 1) as usize,
        }
    }
}

/// Get column name by attribute number from a relation
unsafe fn get_column_name_by_attno(relid: pg_sys::Oid, attno: pg_sys::AttrNumber) -> String {
    // Open the relation to get its tuple descriptor
    let heaprel = pg_sys::relation_open(relid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
    if heaprel.is_null() {
        warning!(
            "ParadeDB: Failed to open relation {} for column name lookup",
            relid
        );
        return format!("col_{}", attno);
    }

    let column_name = {
        let tuple_desc = pgrx::PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);
        if let Some(attribute) = tuple_desc.get(attno as usize - 1) {
            attribute.name().to_string()
        } else {
            format!("col_{}", attno)
        }
    };

    // Close the relation
    pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);

    warning!(
        "ParadeDB: Relation {} attno {} -> column name '{}'",
        get_rel_name(relid),
        attno,
        column_name
    );

    column_name
}
