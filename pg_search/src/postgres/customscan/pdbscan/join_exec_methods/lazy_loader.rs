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

//! Batch-optimized lazy field loading for join execution
//!
//! This module implements the core optimization from the design document:
//! deferring expensive heap access for non-fast fields until after LIMIT application.

use crate::postgres::customscan::pdbscan::{get_rel_name, is_block_all_visible};
use crate::postgres::visibility_checker::VisibilityChecker;
use pgrx::{pg_sys, warning, FromDatum, IntoDatum, PgMemoryContexts, PgRelation, PgTupleDesc};
use std::collections::HashMap;
use std::ptr;

use super::field_map::{FieldInfo, FieldLoadingStrategy, MultiTableFieldMap};

/// Result of a lazy field load operation
#[derive(Debug)]
pub struct LazyFieldValue {
    pub datum: pg_sys::Datum,
    pub is_null: bool,
}

/// Batch-optimized lazy field loader
pub struct LazyFieldLoader {
    /// Visibility checker for MVCC safety
    visibility_checker: VisibilityChecker,
    /// Cache for block visibility to optimize repeated access
    block_visibility_cache: HashMap<pg_sys::BlockNumber, bool>,
    /// Statistics for monitoring
    stats: LoaderStats,
}

/// Statistics for lazy loading operations
#[derive(Debug, Default)]
struct LoaderStats {
    heap_accesses: u64,
    batch_loads: u64,
    cache_hits: u64,
    failed_loads: u64,
}

impl LazyFieldLoader {
    /// Create a new lazy field loader
    pub fn new(heaprel: pg_sys::Relation, snapshot: pg_sys::Snapshot) -> Self {
        Self {
            visibility_checker: unsafe { VisibilityChecker::with_rel_and_snap(heaprel, snapshot) },
            block_visibility_cache: HashMap::new(),
            stats: LoaderStats::default(),
        }
    }

    /// Batch load multiple fields from a single heap tuple
    /// This is the core optimization - load all non-fast fields in one heap access
    pub unsafe fn load_fields_batch(
        &mut self,
        heaprel: pg_sys::Relation,
        ctid: u64,
        field_infos: &[&FieldInfo],
    ) -> Result<Vec<LazyFieldValue>, String> {
        self.stats.batch_loads += 1;
        self.stats.heap_accesses += 1;

        // Convert CTID to ItemPointer
        let mut ipd = pg_sys::ItemPointerData::default();
        crate::postgres::utils::u64_to_item_pointer(ctid, &mut ipd);

        // Check block visibility cache first
        let block_num = pg_sys::ItemPointerGetBlockNumber(&ipd);
        if let Some(&is_visible) = self.block_visibility_cache.get(&block_num) {
            if is_visible {
                self.stats.cache_hits += 1;
                // Fast path: entire block is visible, skip visibility check
                return self.fetch_tuple_fields_fast(heaprel, &ipd, field_infos);
            }
        }

        // Slow path: need visibility check
        self.fetch_tuple_fields_with_visibility(heaprel, &ipd, field_infos, block_num)
    }

    /// Fast path for visible blocks - no visibility check needed
    unsafe fn fetch_tuple_fields_fast(
        &mut self,
        heaprel: pg_sys::Relation,
        ipd: &pg_sys::ItemPointerData,
        field_infos: &[&FieldInfo],
    ) -> Result<Vec<LazyFieldValue>, String> {
        let mut htup = pg_sys::HeapTupleData {
            t_self: *ipd,
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

        if !found {
            if buffer != pg_sys::InvalidBuffer as i32 {
                pg_sys::ReleaseBuffer(buffer);
            }
            self.stats.failed_loads += 1;
            return Err(format!(
                "Tuple not found at CTID {}",
                crate::postgres::utils::item_pointer_to_u64(*ipd)
            ));
        }

        // Extract all requested fields in a single pass
        let results = self.extract_fields_from_tuple(heaprel, &mut htup, field_infos)?;

        if buffer != pg_sys::InvalidBuffer as i32 {
            pg_sys::ReleaseBuffer(buffer);
        }

        Ok(results)
    }

    /// Slow path with full visibility checking
    unsafe fn fetch_tuple_fields_with_visibility(
        &mut self,
        heaprel: pg_sys::Relation,
        ipd: &pg_sys::ItemPointerData,
        field_infos: &[&FieldInfo],
        block_num: pg_sys::BlockNumber,
    ) -> Result<Vec<LazyFieldValue>, String> {
        let ctid = crate::postgres::utils::item_pointer_to_u64(*ipd);

        // Use visibility checker to ensure MVCC safety
        let mut results = Vec::new();
        let found = self
            .visibility_checker
            .exec_if_visible(ctid, ptr::null_mut(), |_| {
                // Tuple is visible, fetch the fields
                match self.fetch_tuple_fields_fast(heaprel, ipd, field_infos) {
                    Ok(values) => {
                        results = values;
                        true
                    }
                    Err(e) => {
                        warning!("ParadeDB: Failed to fetch fields: {}", e);
                        false
                    }
                }
            });

        if found.is_some() {
            // Update block visibility cache
            let mut vmbuff: pg_sys::Buffer = pg_sys::InvalidBuffer as i32;
            let is_all_visible = is_block_all_visible(heaprel, &mut vmbuff, block_num);
            self.block_visibility_cache
                .insert(block_num, is_all_visible);

            Ok(results)
        } else {
            self.stats.failed_loads += 1;
            Err("Tuple not visible".to_string())
        }
    }

    /// Extract multiple fields from a heap tuple in one pass
    unsafe fn extract_fields_from_tuple(
        &self,
        heaprel: pg_sys::Relation,
        htup: &mut pg_sys::HeapTupleData,
        field_infos: &[&FieldInfo],
    ) -> Result<Vec<LazyFieldValue>, String> {
        let tuple_desc = PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);
        let mut results = Vec::with_capacity(field_infos.len());

        for field_info in field_infos {
            let mut is_null = false;
            let datum = pg_sys::heap_getattr(
                htup,
                field_info.attnum as u32,
                (*heaprel).rd_att,
                &mut is_null,
            );

            results.push(LazyFieldValue { datum, is_null });
        }

        Ok(results)
    }

    /// Get loading statistics
    pub fn get_stats(&self) -> (u64, u64, u64, u64) {
        (
            self.stats.heap_accesses,
            self.stats.batch_loads,
            self.stats.cache_hits,
            self.stats.failed_loads,
        )
    }
}

/// Lazy result container that stores fast fields immediately and defers non-fast fields
#[derive(Debug)]
pub struct LazyResult {
    /// Fast field values (available immediately)
    fast_fields: HashMap<(pg_sys::Oid, pg_sys::AttrNumber), LazyFieldValue>,
    /// References to non-fast fields (loaded on demand)
    non_fast_refs: Vec<(pg_sys::Oid, pg_sys::AttrNumber, u64)>, // (relid, attnum, ctid)
    /// Cached non-fast field values (after loading)
    cached_values: HashMap<(pg_sys::Oid, pg_sys::AttrNumber), LazyFieldValue>,
    /// Field map for loading strategy information
    field_map: MultiTableFieldMap,
}

impl LazyResult {
    /// Create a new lazy result
    pub fn new(field_map: MultiTableFieldMap) -> Self {
        Self {
            fast_fields: HashMap::new(),
            non_fast_refs: Vec::new(),
            cached_values: HashMap::new(),
            field_map,
        }
    }

    /// Add fast field values
    pub fn add_fast_fields(
        &mut self,
        relid: pg_sys::Oid,
        fields: Vec<(pg_sys::AttrNumber, LazyFieldValue)>,
    ) {
        for (attnum, value) in fields {
            self.fast_fields.insert((relid, attnum), value);
        }
    }

    /// Add non-fast field reference for lazy loading
    pub fn add_non_fast_ref(&mut self, relid: pg_sys::Oid, attnum: pg_sys::AttrNumber, ctid: u64) {
        self.non_fast_refs.push((relid, attnum, ctid));
    }

    /// Get all unloaded non-fast fields for batch loading
    pub fn get_unloaded_non_fast_fields(&self) -> Vec<(pg_sys::Oid, pg_sys::AttrNumber, u64)> {
        self.non_fast_refs
            .iter()
            .filter(|(relid, attnum, _)| !self.cached_values.contains_key(&(*relid, *attnum)))
            .cloned()
            .collect()
    }

    /// Load non-fast fields in batch
    pub unsafe fn load_non_fast_fields_batch(
        &mut self,
        loaders: &mut HashMap<pg_sys::Oid, LazyFieldLoader>,
    ) -> Result<(), String> {
        // Group fields by relation and CTID for optimal batch loading
        let mut loads_by_relation: HashMap<pg_sys::Oid, HashMap<u64, Vec<pg_sys::AttrNumber>>> =
            HashMap::new();

        for (relid, attnum, ctid) in &self.non_fast_refs {
            if !self.cached_values.contains_key(&(*relid, *attnum)) {
                loads_by_relation
                    .entry(*relid)
                    .or_insert_with(HashMap::new)
                    .entry(*ctid)
                    .or_insert_with(Vec::new)
                    .push(*attnum);
            }
        }

        // Perform batch loads
        for (relid, ctid_fields) in loads_by_relation {
            let loader = loaders
                .get_mut(&relid)
                .ok_or_else(|| format!("No loader for relation {}", relid))?;

            let table_map = self
                .field_map
                .get_table(relid)
                .ok_or_else(|| format!("No field map for relation {}", relid))?;

            // Open the relation
            let heaprel = pg_sys::relation_open(relid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
            if heaprel.is_null() {
                return Err(format!("Failed to open relation {}", relid));
            }

            // Load fields for each CTID
            for (ctid, attnums) in ctid_fields {
                let field_infos: Vec<&FieldInfo> = attnums
                    .iter()
                    .filter_map(|attnum| table_map.fields.get(attnum))
                    .collect();

                match loader.load_fields_batch(heaprel, ctid, &field_infos) {
                    Ok(values) => {
                        // Cache the loaded values
                        for (i, attnum) in attnums.iter().enumerate() {
                            if let Some(value) = values.get(i) {
                                self.cached_values.insert(
                                    (relid, *attnum),
                                    LazyFieldValue {
                                        datum: value.datum,
                                        is_null: value.is_null,
                                    },
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warning!(
                            "ParadeDB: Failed to load fields from relation {}: {}",
                            get_rel_name(relid),
                            e
                        );
                    }
                }
            }

            pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        }

        Ok(())
    }

    /// Get a field value (from fast fields or cached values)
    pub fn get_field(
        &self,
        relid: pg_sys::Oid,
        attnum: pg_sys::AttrNumber,
    ) -> Option<&LazyFieldValue> {
        self.fast_fields
            .get(&(relid, attnum))
            .or_else(|| self.cached_values.get(&(relid, attnum)))
    }

    /// Get all field values as a vector suitable for tuple construction
    pub fn get_all_fields(
        &self,
        field_order: &[(pg_sys::Oid, pg_sys::AttrNumber)],
    ) -> Vec<(pg_sys::Datum, bool)> {
        field_order
            .iter()
            .map(|(relid, attnum)| {
                if let Some(value) = self.get_field(*relid, *attnum) {
                    (value.datum, value.is_null)
                } else {
                    (pg_sys::Datum::null(), true)
                }
            })
            .collect()
    }
}

/// Lazy field loader with fallback strategies for production resilience
pub struct LazyFieldLoaderWithFallback {
    loader: LazyFieldLoader,
    fallback_strategy: FallbackStrategy,
    error_count: u64,
    error_threshold: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum FallbackStrategy {
    /// Retry the operation once
    Retry,
    /// Fall back to eager loading
    EagerLoad,
    /// Return partial results
    PartialResults,
    /// Fail the query
    FailQuery,
}

impl LazyFieldLoaderWithFallback {
    /// Create a new loader with fallback
    pub fn new(
        heaprel: pg_sys::Relation,
        snapshot: pg_sys::Snapshot,
        strategy: FallbackStrategy,
        error_threshold: u64,
    ) -> Self {
        Self {
            loader: LazyFieldLoader::new(heaprel, snapshot),
            fallback_strategy: strategy,
            error_count: 0,
            error_threshold,
        }
    }

    /// Load fields with fallback handling
    pub unsafe fn load_fields_batch_with_fallback(
        &mut self,
        heaprel: pg_sys::Relation,
        ctid: u64,
        field_infos: &[&FieldInfo],
    ) -> Result<Vec<LazyFieldValue>, String> {
        match self.loader.load_fields_batch(heaprel, ctid, field_infos) {
            Ok(values) => Ok(values),
            Err(e) => {
                self.error_count += 1;

                if self.error_count >= self.error_threshold {
                    warning!(
                        "ParadeDB: Lazy loading error threshold reached ({})",
                        self.error_count
                    );
                }

                match self.fallback_strategy {
                    FallbackStrategy::Retry => {
                        // Try once more
                        self.loader.load_fields_batch(heaprel, ctid, field_infos)
                    }
                    FallbackStrategy::EagerLoad => {
                        // This would trigger eager loading of all fields
                        Err(format!("Falling back to eager loading: {}", e))
                    }
                    FallbackStrategy::PartialResults => {
                        // Return null values for failed fields
                        Ok(field_infos
                            .iter()
                            .map(|_| LazyFieldValue {
                                datum: pg_sys::Datum::null(),
                                is_null: true,
                            })
                            .collect())
                    }
                    FallbackStrategy::FailQuery => Err(e),
                }
            }
        }
    }

    /// Get error statistics
    pub fn get_error_count(&self) -> u64 {
        self.error_count
    }

    /// Get loader statistics
    pub fn get_stats(&self) -> (u64, u64, u64, u64) {
        self.loader.get_stats()
    }
}
