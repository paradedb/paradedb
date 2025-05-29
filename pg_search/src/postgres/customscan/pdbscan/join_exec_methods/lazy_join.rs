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

//! Optimized join execution with lazy field loading
//!
//! This module implements the core optimization strategy: execute joins using only
//! fast fields, apply LIMIT early, then batch-load non-fast fields for final results.

use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::pdbscan::{get_rel_name, PdbScan};
use pgrx::{pg_sys, warning};
use std::collections::HashMap;

use super::field_map::{FieldLoadingStrategy, MultiTableFieldMap, TableFieldMap};
use super::lazy_loader::{
    FallbackStrategy, LazyFieldLoader, LazyFieldLoaderWithFallback, LazyResult,
};
use super::state::JoinExecState;

/// Execute join with lazy field loading optimization
pub unsafe fn execute_lazy_join(
    state: &mut CustomScanStateWrapper<PdbScan>,
) -> *mut pg_sys::TupleTableSlot {
    // Check if we should use lazy loading
    if !should_use_lazy_loading(state) {
        warning!("ParadeDB: Lazy loading not beneficial for this query, using standard execution");
        return super::match_and_return_next_tuple(state);
    }

    warning!("ParadeDB: Using lazy join execution with deferred field loading");

    // Phase 1: Fast field intersection to find matching tuples
    let matching_results = execute_fast_field_intersection(state);
    if matching_results.is_empty() {
        warning!("ParadeDB: No matching tuples found in fast field intersection");
        return std::ptr::null_mut();
    }

    // Phase 2: Apply early LIMIT if applicable
    let limited_results = apply_early_limit(state, matching_results);

    warning!(
        "ParadeDB: Fast field intersection found {} matches, limited to {}",
        matching_results.len(),
        limited_results.len()
    );

    // Phase 3: Batch load non-fast fields for final results
    match batch_load_and_return_tuple(state, limited_results) {
        Some(slot) => slot,
        None => {
            // Continue to next batch if available
            execute_lazy_join(state)
        }
    }
}

/// Check if lazy loading should be used for this join
unsafe fn should_use_lazy_loading(state: &mut CustomScanStateWrapper<PdbScan>) -> bool {
    let join_state = match state.custom_state().join_exec_state.as_ref() {
        Some(state) => state,
        None => return false,
    };

    // Check for LIMIT clause
    let limit = get_effective_limit(state);
    if limit.is_none() {
        warning!("ParadeDB: No LIMIT clause found, lazy loading may not provide benefit");
        return false;
    }

    // Check if we have search predicates (required for optimization)
    if !join_state.is_search_join() {
        return false;
    }

    // Check if GUC is enabled
    if !crate::gucs::is_lazy_join_loading_enabled() {
        return false;
    }

    true
}

/// Execute fast field intersection to find matching tuples
unsafe fn execute_fast_field_intersection(
    state: &mut CustomScanStateWrapper<PdbScan>,
) -> Vec<FastFieldMatch> {
    let mut matches = Vec::new();

    let join_state = match state.custom_state_mut().join_exec_state.as_mut() {
        Some(state) => state,
        None => return matches,
    };

    // Get search results from both sides
    let outer_results = join_state.outer_results.as_ref().unwrap_or(&Vec::new());
    let inner_results = join_state.inner_results.as_ref().unwrap_or(&Vec::new());

    warning!(
        "ParadeDB: Fast field intersection with {} outer and {} inner results",
        outer_results.len(),
        inner_results.len()
    );

    // Perform nested loop join on fast fields only
    // This is still O(n*m) but n and m are search result sets, not full tables
    for (outer_idx, (outer_ctid, outer_score)) in outer_results.iter().enumerate() {
        for (inner_idx, (inner_ctid, inner_score)) in inner_results.iter().enumerate() {
            // Evaluate join condition using only fast fields
            if evaluate_join_condition_fast_fields(state, *outer_ctid, *inner_ctid) {
                matches.push(FastFieldMatch {
                    outer_ctid: *outer_ctid,
                    inner_ctid: *inner_ctid,
                    combined_score: *outer_score + *inner_score,
                    outer_idx,
                    inner_idx,
                });
            }
        }
    }

    // Sort by combined score for ranking
    matches.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());

    matches
}

/// Fast field match result
#[derive(Debug, Clone)]
struct FastFieldMatch {
    outer_ctid: u64,
    inner_ctid: u64,
    combined_score: f32,
    outer_idx: usize,
    inner_idx: usize,
}

/// Evaluate join condition using only fast fields
unsafe fn evaluate_join_condition_fast_fields(
    state: &mut CustomScanStateWrapper<PdbScan>,
    outer_ctid: u64,
    inner_ctid: u64,
) -> bool {
    // This is a simplified version that assumes join keys are fast fields
    // In a complete implementation, we would:
    // 1. Check if join key columns are fast fields
    // 2. Load values from Tantivy index
    // 3. Compare them

    // For now, delegate to the existing evaluation logic
    // TODO: Optimize to use only fast fields when possible
    super::evaluate_join_condition(state, outer_ctid, inner_ctid)
}

/// Apply LIMIT early to reduce the number of tuples needing non-fast field loading
fn apply_early_limit(
    state: &mut CustomScanStateWrapper<PdbScan>,
    matches: Vec<FastFieldMatch>,
) -> Vec<FastFieldMatch> {
    let limit = get_effective_limit(state).unwrap_or(matches.len() as f64) as usize;

    matches.into_iter().take(limit).collect()
}

/// Get the effective LIMIT for this query
fn get_effective_limit(state: &mut CustomScanStateWrapper<PdbScan>) -> Option<f64> {
    // Check join state for limit
    if let Some(ref join_state) = state.custom_state().join_exec_state {
        join_state.limit
    } else {
        None
    }
}

/// Batch load non-fast fields and return next tuple
unsafe fn batch_load_and_return_tuple(
    state: &mut CustomScanStateWrapper<PdbScan>,
    matches: Vec<FastFieldMatch>,
) -> Option<*mut pg_sys::TupleTableSlot> {
    if matches.is_empty() {
        return None;
    }

    let join_state = state.custom_state_mut().join_exec_state.as_mut()?;

    // Initialize field maps if not already done
    let field_map = get_or_create_field_map(state)?;

    // Create lazy loaders for each relation
    let mut loaders = create_lazy_loaders(state)?;

    // Process matches in batches for memory efficiency
    const BATCH_SIZE: usize = 100;
    let batch = matches.into_iter().take(BATCH_SIZE).collect::<Vec<_>>();

    warning!(
        "ParadeDB: Processing batch of {} matches for lazy loading",
        batch.len()
    );

    // Create lazy results for the batch
    let mut lazy_results = Vec::new();
    for match_info in &batch {
        let mut lazy_result = LazyResult::new(field_map.clone());

        // Add fast field values (already available)
        add_fast_field_values(state, &mut lazy_result, match_info);

        // Add non-fast field references for lazy loading
        add_non_fast_field_refs(state, &mut lazy_result, match_info, &field_map);

        lazy_results.push(lazy_result);
    }

    // Batch load all non-fast fields
    let mut load_stats = (0u64, 0u64, 0u64, 0u64);
    for lazy_result in &mut lazy_results {
        if let Err(e) = lazy_result.load_non_fast_fields_batch(&mut loaders) {
            warning!("ParadeDB: Failed to load non-fast fields: {}", e);
        }
    }

    // Log loading statistics
    for (relid, loader) in &loaders {
        let stats = loader.get_stats();
        load_stats.0 += stats.0; // heap_accesses
        load_stats.1 += stats.1; // batch_loads
        load_stats.2 += stats.2; // cache_hits
        load_stats.3 += stats.3; // failed_loads
    }

    warning!(
        "ParadeDB: Lazy loading stats - {} heap accesses, {} batch loads, {} cache hits, {} failures",
        load_stats.0, load_stats.1, load_stats.2, load_stats.3
    );

    // Return the first result tuple
    // In a complete implementation, we would store the remaining results
    // for subsequent calls
    if let Some(lazy_result) = lazy_results.into_iter().next() {
        Some(create_result_tuple_from_lazy(state, lazy_result))
    } else {
        None
    }
}

/// Get or create field map for the relations
unsafe fn get_or_create_field_map(
    state: &mut CustomScanStateWrapper<PdbScan>,
) -> Option<MultiTableFieldMap> {
    let join_state = state.custom_state_mut().join_exec_state.as_mut()?;

    // Check if we already have a cached field map
    if let Some(ref field_map) = join_state.field_map {
        return Some(field_map.clone());
    }

    // Create a new field map
    let mut field_map = MultiTableFieldMap::new();

    // Add outer relation
    if join_state.outer_relid != pg_sys::InvalidOid {
        // TODO: Get actual fast fields from index
        let fast_fields = std::collections::HashSet::new();
        if let Ok(table_map) = TableFieldMap::new(join_state.outer_relid, &fast_fields) {
            field_map.add_table(table_map);
        }
    }

    // Add inner relation
    if join_state.inner_relid != pg_sys::InvalidOid {
        // TODO: Get actual fast fields from index
        let fast_fields = std::collections::HashSet::new();
        if let Ok(table_map) = TableFieldMap::new(join_state.inner_relid, &fast_fields) {
            field_map.add_table(table_map);
        }
    }

    field_map.log_statistics();

    // Cache the field map
    join_state.field_map = Some(field_map.clone());

    Some(field_map)
}

/// Create lazy loaders for each relation
unsafe fn create_lazy_loaders(
    state: &mut CustomScanStateWrapper<PdbScan>,
) -> Option<HashMap<pg_sys::Oid, LazyFieldLoaderWithFallback>> {
    let join_state = state.custom_state().join_exec_state.as_ref()?;
    let mut loaders = HashMap::new();

    // Create loader for outer relation
    if join_state.outer_relid != pg_sys::InvalidOid {
        let heaprel = pg_sys::relation_open(
            join_state.outer_relid,
            pg_sys::AccessShareLock as pg_sys::LOCKMODE,
        );
        if !heaprel.is_null() {
            let loader = LazyFieldLoaderWithFallback::new(
                heaprel,
                pg_sys::GetActiveSnapshot(),
                FallbackStrategy::PartialResults,
                100, // error threshold
            );
            loaders.insert(join_state.outer_relid, loader);
            pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        }
    }

    // Create loader for inner relation
    if join_state.inner_relid != pg_sys::InvalidOid {
        let heaprel = pg_sys::relation_open(
            join_state.inner_relid,
            pg_sys::AccessShareLock as pg_sys::LOCKMODE,
        );
        if !heaprel.is_null() {
            let loader = LazyFieldLoaderWithFallback::new(
                heaprel,
                pg_sys::GetActiveSnapshot(),
                FallbackStrategy::PartialResults,
                100, // error threshold
            );
            loaders.insert(join_state.inner_relid, loader);
            pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        }
    }

    Some(loaders)
}

/// Add fast field values to lazy result
unsafe fn add_fast_field_values(
    state: &mut CustomScanStateWrapper<PdbScan>,
    lazy_result: &mut LazyResult,
    match_info: &FastFieldMatch,
) {
    // TODO: Extract actual fast field values from search results or Tantivy
    // For now, add score as a fast field

    let join_state = state.custom_state().join_exec_state.as_ref().unwrap();

    // Add scores as fast fields
    lazy_result.add_fast_fields(
        join_state.outer_relid,
        vec![(
            1,
            super::lazy_loader::LazyFieldValue {
                datum: (match_info.combined_score as f64).into_datum().unwrap(),
                is_null: false,
            },
        )],
    );
}

/// Add non-fast field references to lazy result
unsafe fn add_non_fast_field_refs(
    state: &mut CustomScanStateWrapper<PdbScan>,
    lazy_result: &mut LazyResult,
    match_info: &FastFieldMatch,
    field_map: &MultiTableFieldMap,
) {
    let join_state = state.custom_state().join_exec_state.as_ref().unwrap();

    // Add references for outer relation non-fast fields
    if let Some(table_map) = field_map.get_table(join_state.outer_relid) {
        for field_info in table_map.get_non_fast_fields() {
            lazy_result.add_non_fast_ref(
                join_state.outer_relid,
                field_info.attnum,
                match_info.outer_ctid,
            );
        }
    }

    // Add references for inner relation non-fast fields
    if let Some(table_map) = field_map.get_table(join_state.inner_relid) {
        for field_info in table_map.get_non_fast_fields() {
            lazy_result.add_non_fast_ref(
                join_state.inner_relid,
                field_info.attnum,
                match_info.inner_ctid,
            );
        }
    }
}

/// Create result tuple from lazy result
unsafe fn create_result_tuple_from_lazy(
    state: &mut CustomScanStateWrapper<PdbScan>,
    lazy_result: LazyResult,
) -> *mut pg_sys::TupleTableSlot {
    // TODO: Implement proper tuple creation from lazy result
    // For now, delegate to existing implementation

    let slot = state.csstate.ss.ss_ScanTupleSlot;
    if slot.is_null() {
        return std::ptr::null_mut();
    }

    // Clear the slot
    pg_sys::ExecClearTuple(slot);

    // TODO: Populate slot with values from lazy_result

    slot
}
