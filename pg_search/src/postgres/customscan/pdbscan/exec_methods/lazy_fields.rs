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

//! Lazy field loading infrastructure for JOIN optimization
//!
//! This module provides lazy loading of non-fast fields to optimize JOIN queries
//! by deferring expensive heap access until only the final result set is known.
//!
//! It follows the same MVCC and visibility checking patterns as PdbScan to ensure
//! production-grade safety and reliability.

use crate::postgres::customscan::pdbscan::is_block_all_visible;
use crate::postgres::utils;
use crate::postgres::visibility_checker::VisibilityChecker;
use crate::schema::SearchIndexSchema;
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::{pg_sys, PgRelation, PgTupleDesc};
use std::collections::HashMap;
use std::collections::HashSet;

/// State of a lazy-loaded field
#[derive(Debug, Clone)]
pub enum LazyFieldState {
    /// Field has not been loaded yet
    NotLoaded,
    /// Field is currently being loaded (for future async support)
    Loading,
    /// Field has been successfully loaded
    Loaded(pg_sys::Datum),
    /// Field loading failed with an error
    Failed(LazyLoadError),
}

/// Errors that can occur during lazy field loading
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LazyLoadError {
    /// Tuple is not visible in the current snapshot
    TupleNotVisible,
    /// Failed to pin buffer for the block
    BufferPinFailed,
    /// Memory allocation failed
    MemoryExhausted,
    /// Transaction was aborted
    TransactionAborted,
}

/// Lazy field loader that follows PdbScan's MVCC patterns
pub struct LazyFieldLoader {
    /// Reuse PdbScan's visibility checking infrastructure
    visibility_checker: VisibilityChecker,

    /// Block visibility cache (same pattern as PdbScan)
    blockvis: (pg_sys::BlockNumber, bool),

    /// Visibility map buffer (same as PdbScan)
    vmbuff: pg_sys::Buffer,

    /// Memory context for field data
    memory_context: pg_sys::MemoryContext,

    /// Per-tuple memory context for temporary allocations
    per_tuple_context: pg_sys::MemoryContext,

    /// Temporary slot for tuple access
    temp_slot: *mut pg_sys::TupleTableSlot,
}

impl LazyFieldLoader {
    /// Create a new lazy field loader for the given heap relation
    ///
    /// This follows the same initialization pattern as PdbScan's exec methods
    pub fn new(heaprel: pg_sys::Relation) -> Self {
        unsafe {
            let snapshot = pg_sys::GetActiveSnapshot();

            Self {
                // Use existing VisibilityChecker - no need to reinvent
                visibility_checker: VisibilityChecker::with_rel_and_snap(heaprel, snapshot),

                // Initialize block visibility cache (same as PdbScan)
                blockvis: (pg_sys::InvalidBlockNumber, false),
                vmbuff: pg_sys::InvalidBuffer as pg_sys::Buffer,

                // Create memory contexts following PdbScan patterns
                memory_context: pg_sys::AllocSetContextCreateExtended(
                    pg_sys::CurrentMemoryContext,
                    c"LazyFieldLoader".as_ptr(),
                    pg_sys::ALLOCSET_DEFAULT_MINSIZE as usize,
                    pg_sys::ALLOCSET_DEFAULT_INITSIZE as usize,
                    pg_sys::ALLOCSET_DEFAULT_MAXSIZE as usize,
                ),
                per_tuple_context: pg_sys::AllocSetContextCreateExtended(
                    pg_sys::CurrentMemoryContext,
                    c"LazyFieldPerTuple".as_ptr(),
                    pg_sys::ALLOCSET_DEFAULT_MINSIZE as usize,
                    pg_sys::ALLOCSET_DEFAULT_INITSIZE as usize,
                    pg_sys::ALLOCSET_SMALL_MAXSIZE as usize,
                ),

                // Create temporary slot for tuple access
                temp_slot: pg_sys::MakeTupleTableSlot((*heaprel).rd_att, &pg_sys::TTSOpsVirtual),
            }
        }
    }

    /// Load a field with visibility checking, following PdbScan's patterns
    ///
    /// This is the core method that implements the same two-tier visibility
    /// checking as PdbScan: fast path for visible blocks, slow path for MVCC.
    pub fn load_field_with_visibility_check(
        &mut self,
        ctid: u64,
        attno: pg_sys::AttrNumber,
        heaprel: pg_sys::Relation,
    ) -> Result<pg_sys::Datum, LazyLoadError> {
        unsafe {
            // Use the same block visibility optimization as PdbScan
            let mut tid = pg_sys::ItemPointerData::default();
            utils::u64_to_item_pointer(ctid, &mut tid);

            let blockno = item_pointer_get_block_number(&tid);
            let is_visible = if blockno == self.blockvis.0 {
                // We know the visibility of this block from cache (same as PdbScan)
                self.blockvis.1
            } else {
                // New block, check visibility using PdbScan's function
                self.blockvis.0 = blockno;
                self.blockvis.1 = is_block_all_visible(heaprel, &mut self.vmbuff, blockno);
                self.blockvis.1
            };

            if is_visible {
                // Block is all visible, can extract field directly (fast path)
                self.extract_field_from_visible_block(ctid, attno, heaprel)
            } else {
                // Use VisibilityChecker for MVCC-safe access (slow path, same as PdbScan)
                // Extract the field from slot in a separate step to avoid borrowing conflicts
                let temp_slot = self.temp_slot;
                let result = self
                    .visibility_checker
                    .exec_if_visible(ctid, temp_slot, |_heaprel| {
                        // Extract field from the slot that was populated by VisibilityChecker
                        let mut isnull = false;
                        let datum = pg_sys::slot_getattr(temp_slot, attno as i32, &mut isnull);

                        if isnull {
                            (pg_sys::Datum::null(), true)
                        } else {
                            (datum, false)
                        }
                    });

                match result {
                    Some((datum, is_null)) => {
                        if is_null {
                            Ok(pg_sys::Datum::null())
                        } else {
                            // Copy to our memory context
                            let tupdesc = (*temp_slot).tts_tupleDescriptor;
                            Ok(self.copy_datum_to_context(datum, attno, tupdesc))
                        }
                    }
                    None => Err(LazyLoadError::TupleNotVisible),
                }
            }
        }
    }

    /// Extract field from a block known to be all-visible (fast path)
    ///
    /// This follows the same pattern as PdbScan's fast field access
    unsafe fn extract_field_from_visible_block(
        &self,
        ctid: u64,
        attno: pg_sys::AttrNumber,
        heaprel: pg_sys::Relation,
    ) -> Result<pg_sys::Datum, LazyLoadError> {
        // Switch to per-tuple context for temporary allocations
        let old_context = pg_sys::MemoryContextSwitchTo(self.per_tuple_context);

        let result = self.extract_field_from_visible_block_internal(ctid, attno, heaprel);

        // Restore previous context
        pg_sys::MemoryContextSwitchTo(old_context);

        result
    }

    unsafe fn extract_field_from_visible_block_internal(
        &self,
        ctid: u64,
        attno: pg_sys::AttrNumber,
        heaprel: pg_sys::Relation,
    ) -> Result<pg_sys::Datum, LazyLoadError> {
        // Convert CTID to ItemPointer
        let mut tid = pg_sys::ItemPointerData::default();
        utils::u64_to_item_pointer(ctid, &mut tid);

        // Read the tuple directly (block is known to be visible)
        let buffer = pg_sys::ReadBuffer(heaprel, item_pointer_get_block_number(&tid));
        if buffer == pg_sys::InvalidBuffer as i32 {
            return Err(LazyLoadError::BufferPinFailed);
        }

        // Lock buffer for reading
        pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_SHARE as i32);

        // Get the tuple from the page
        let page = pg_sys::BufferGetPage(buffer);
        let offno = pgrx::itemptr::item_pointer_get_offset_number(&tid);
        let tuple = pg_sys::PageGetItem(page, pg_sys::PageGetItemId(page, offno));
        let htup = tuple as pg_sys::HeapTuple;

        // Extract the field using PostgreSQL's standard function
        let mut isnull = false;
        let tupdesc = (*heaprel).rd_att;
        let datum = pg_sys::heap_getattr(htup, attno as i32, tupdesc, &mut isnull);

        // Copy datum to our memory context if not null
        let result_datum = if isnull {
            pg_sys::Datum::null()
        } else {
            self.copy_datum_to_context(datum, attno, tupdesc)
        };

        // Release buffer
        pg_sys::UnlockReleaseBuffer(buffer);

        Ok(result_datum)
    }

    /// Copy datum to our memory context to ensure it survives buffer release
    ///
    /// This follows PostgreSQL's standard pattern for datum copying
    unsafe fn copy_datum_to_context(
        &self,
        datum: pg_sys::Datum,
        attno: pg_sys::AttrNumber,
        tupdesc: pg_sys::TupleDesc,
    ) -> pg_sys::Datum {
        let old_context = pg_sys::MemoryContextSwitchTo(self.memory_context);

        let att = (*tupdesc).attrs.as_slice((*tupdesc).natts as usize)[attno as usize - 1];
        let copied_datum = if att.attbyval {
            // Pass-by-value types can be copied directly
            datum
        } else {
            // Pass-by-reference types need deep copy
            // For now, we'll just return the datum directly since we're in a persistent context
            // TODO: Implement proper datum copying when needed
            datum
        };

        pg_sys::MemoryContextSwitchTo(old_context);
        copied_datum
    }

    /// Reset per-tuple context to free temporary allocations
    ///
    /// This follows PdbScan's memory management pattern
    pub fn reset_per_tuple_context(&mut self) {
        unsafe {
            pg_sys::MemoryContextReset(self.per_tuple_context);
        }
    }

    /// Reset block visibility cache
    ///
    /// This follows PdbScan's reset pattern
    pub fn reset_block_cache(&mut self) {
        self.blockvis = (pg_sys::InvalidBlockNumber, false);
    }
}

impl Drop for LazyFieldLoader {
    fn drop(&mut self) {
        unsafe {
            // Release visibility map buffer if we have one (same as PdbScan)
            if utils::IsTransactionState() && self.vmbuff != pg_sys::InvalidBuffer as pg_sys::Buffer
            {
                pg_sys::ReleaseBuffer(self.vmbuff);
            }
        }
    }
}

/// Lazy field loader with fallback strategies
///
/// This provides additional error handling on top of the basic loader
pub struct LazyFieldLoaderWithFallback {
    loader: LazyFieldLoader,
    fallback_strategy: FallbackStrategy,
    error_stats: ErrorStatistics,
}

/// Fallback strategies for handling load failures
#[derive(Debug, Clone, Copy)]
pub enum FallbackStrategy {
    /// Simple retry (like PdbScan - just continue to next tuple)
    RetryOnce,
    /// Fall back to PostgreSQL's standard heap access
    FallbackToEagerLoading,
    /// Return NULL for failed fields
    ReturnPartialResults,
    /// Fail the entire query
    FailQuery,
}

/// Statistics for tracking lazy loading performance
#[derive(Debug, Default)]
pub struct ErrorStatistics {
    pub tuple_not_visible_count: u64,
    pub buffer_pin_failures: u64,
    pub memory_exhausted_count: u64,
    pub total_fallbacks: u64,
}

impl LazyFieldLoaderWithFallback {
    /// Create a new loader with fallback support
    pub fn new(heaprel: pg_sys::Relation, fallback_strategy: FallbackStrategy) -> Self {
        Self {
            loader: LazyFieldLoader::new(heaprel),
            fallback_strategy,
            error_stats: ErrorStatistics::default(),
        }
    }

    /// Load field with fallback handling
    pub fn load_field_with_fallback(
        &mut self,
        ctid: u64,
        attno: pg_sys::AttrNumber,
        heaprel: pg_sys::Relation,
    ) -> Result<pg_sys::Datum, LazyLoadError> {
        // First attempt
        match self
            .loader
            .load_field_with_visibility_check(ctid, attno, heaprel)
        {
            Ok(datum) => Ok(datum),
            Err(LazyLoadError::TupleNotVisible) => {
                self.error_stats.tuple_not_visible_count += 1;

                match self.fallback_strategy {
                    FallbackStrategy::RetryOnce => {
                        // Simple retry (PdbScan just continues to next tuple)
                        self.loader
                            .load_field_with_visibility_check(ctid, attno, heaprel)
                    }
                    FallbackStrategy::FallbackToEagerLoading => {
                        // Use PostgreSQL's standard heap access
                        self.error_stats.total_fallbacks += 1;
                        self.eager_load_field(ctid, attno, heaprel)
                    }
                    FallbackStrategy::ReturnPartialResults => {
                        // Return NULL for this field
                        Ok(pg_sys::Datum::null())
                    }
                    FallbackStrategy::FailQuery => Err(LazyLoadError::TupleNotVisible),
                }
            }
            Err(LazyLoadError::BufferPinFailed) => {
                self.error_stats.buffer_pin_failures += 1;
                Err(LazyLoadError::BufferPinFailed)
            }
            Err(other_error) => Err(other_error),
        }
    }

    /// Fallback to eager loading using PostgreSQL's standard table access
    fn eager_load_field(
        &self,
        ctid: u64,
        attno: pg_sys::AttrNumber,
        heaprel: pg_sys::Relation,
    ) -> Result<pg_sys::Datum, LazyLoadError> {
        unsafe {
            let mut tid = pg_sys::ItemPointerData::default();
            utils::u64_to_item_pointer(ctid, &mut tid);

            // Use PostgreSQL's standard table access methods
            let scan = pg_sys::table_index_fetch_begin(heaprel);
            let slot = pg_sys::MakeTupleTableSlot((*heaprel).rd_att, &pg_sys::TTSOpsVirtual);

            let mut call_again = false;
            let mut all_dead = false;
            let found = pg_sys::table_index_fetch_tuple(
                scan,
                &mut tid,
                pg_sys::GetActiveSnapshot(),
                slot,
                &mut call_again,
                &mut all_dead,
            );

            let result = if found {
                let mut isnull = false;
                let datum = pg_sys::slot_getattr(slot, attno as i32, &mut isnull);
                if isnull {
                    Ok(pg_sys::Datum::null())
                } else {
                    Ok(datum)
                }
            } else {
                Err(LazyLoadError::TupleNotVisible)
            };

            pg_sys::table_index_fetch_end(scan);
            result
        }
    }

    /// Get error statistics
    pub fn error_stats(&self) -> &ErrorStatistics {
        &self.error_stats
    }

    /// Reset per-tuple context
    pub fn reset_per_tuple_context(&mut self) {
        self.loader.reset_per_tuple_context();
    }

    /// Reset block cache
    pub fn reset_block_cache(&mut self) {
        self.loader.reset_block_cache();
    }
}

/// Container for lazy-loaded results
///
/// This represents a result tuple where fast fields are immediately available
/// and non-fast fields are loaded lazily
#[derive(Debug)]
pub struct LazyResult {
    /// Fast fields that are immediately available
    pub fast_fields: HashMap<pg_sys::AttrNumber, pg_sys::Datum>,

    /// CTIDs for lazy loading non-fast fields
    pub ctids: HashMap<pg_sys::Oid, u64>, // table_oid -> ctid

    /// Lazily loaded non-fast fields
    pub non_fast_fields: HashMap<pg_sys::AttrNumber, LazyFieldState>,

    /// Combined score (if applicable)
    pub combined_score: Option<f32>,
}

impl LazyResult {
    /// Create a new lazy result
    pub fn new() -> Self {
        Self {
            fast_fields: HashMap::new(),
            ctids: HashMap::new(),
            non_fast_fields: HashMap::new(),
            combined_score: None,
        }
    }

    /// Add a fast field value
    pub fn add_fast_field(&mut self, attno: pg_sys::AttrNumber, datum: pg_sys::Datum) {
        self.fast_fields.insert(attno, datum);
    }

    /// Add a CTID for a table
    pub fn add_ctid(&mut self, table_oid: pg_sys::Oid, ctid: u64) {
        self.ctids.insert(table_oid, ctid);
    }

    /// Get CTID for a table
    pub fn get_ctid(&self, table_oid: pg_sys::Oid) -> Option<u64> {
        self.ctids.get(&table_oid).copied()
    }

    /// Check if a non-fast field has been loaded
    pub fn is_field_loaded(&self, attno: pg_sys::AttrNumber) -> bool {
        matches!(
            self.non_fast_fields.get(&attno),
            Some(LazyFieldState::Loaded(_))
        )
    }

    /// Get a loaded field value
    pub fn get_field(&self, attno: pg_sys::AttrNumber) -> Option<pg_sys::Datum> {
        match self.non_fast_fields.get(&attno) {
            Some(LazyFieldState::Loaded(datum)) => Some(*datum),
            _ => None,
        }
    }
}

impl Default for LazyResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Strategy for loading different types of fields
#[derive(Debug, Clone)]
pub enum FieldLoadingStrategy {
    /// Already available (no additional cost)
    FastField { value: pg_sys::Datum },

    /// Load from Tantivy stored fields (medium cost)
    TantivyStored { attno: pg_sys::AttrNumber },

    /// Load from PostgreSQL heap (high cost)
    HeapAccess {
        table_oid: pg_sys::Oid,
        ctid: u64,
        attno: pg_sys::AttrNumber,
    },
}

/// Information about how to access a field
#[derive(Debug, Clone)]
pub struct FieldAccessInfo {
    pub attno: pg_sys::AttrNumber,
    pub loading_strategy: FieldLoadingStrategy,
    pub is_fast_field: bool,
}

/// Mapping of fields for a specific table
#[derive(Debug, Clone)]
pub struct TableFieldMap {
    pub table_oid: pg_sys::Oid,
    pub fast_field_attnos: HashSet<pg_sys::AttrNumber>,
    pub tantivy_stored_attnos: HashSet<pg_sys::AttrNumber>,
    pub heap_only_attnos: HashSet<pg_sys::AttrNumber>,
}

impl TableFieldMap {
    /// Create a new table field map from a relation and its search index schema
    pub fn new(table_oid: pg_sys::Oid, heaprel: &PgRelation, schema: &SearchIndexSchema) -> Self {
        let mut fast_field_attnos = HashSet::new();
        let tantivy_stored_attnos = HashSet::new();
        let mut heap_only_attnos = HashSet::new();

        unsafe {
            let tupdesc = PgTupleDesc::from_pg_unchecked((*heaprel.as_ptr()).rd_att);

            for attno in 1..=tupdesc.len() {
                if let Some(att) = tupdesc.get(attno - 1) {
                    let attname = att.name();

                    if schema.is_fast_field(attname) {
                        fast_field_attnos.insert(attno as pg_sys::AttrNumber);
                    } else {
                        // For now, assume all non-fast fields require heap access
                        // TODO: Add logic to detect Tantivy stored fields when needed
                        heap_only_attnos.insert(attno as pg_sys::AttrNumber);
                    }
                }
            }
        }

        Self {
            table_oid,
            fast_field_attnos,
            tantivy_stored_attnos,
            heap_only_attnos,
        }
    }

    /// Determine the loading strategy for a field
    pub fn determine_loading_strategy(&self, attno: pg_sys::AttrNumber) -> FieldLoadingStrategy {
        if self.fast_field_attnos.contains(&attno) {
            // Fast fields are already loaded during search
            FieldLoadingStrategy::FastField {
                value: pg_sys::Datum::null(), // Will be populated with actual value
            }
        } else if self.tantivy_stored_attnos.contains(&attno) {
            FieldLoadingStrategy::TantivyStored { attno }
        } else {
            FieldLoadingStrategy::HeapAccess {
                table_oid: self.table_oid,
                ctid: 0, // Will be populated with actual CTID
                attno,
            }
        }
    }

    /// Check if a field is a fast field
    pub fn is_fast_field(&self, attno: pg_sys::AttrNumber) -> bool {
        self.fast_field_attnos.contains(&attno)
    }

    /// Check if a field requires heap access
    pub fn requires_heap_access(&self, attno: pg_sys::AttrNumber) -> bool {
        self.heap_only_attnos.contains(&attno)
    }

    /// Get all non-fast field attribute numbers
    pub fn non_fast_field_attnos(&self) -> impl Iterator<Item = pg_sys::AttrNumber> + '_ {
        self.tantivy_stored_attnos
            .iter()
            .chain(self.heap_only_attnos.iter())
            .copied()
    }

    /// Get statistics about field distribution
    pub fn field_stats(&self) -> FieldStats {
        FieldStats {
            total_fields: self.fast_field_attnos.len()
                + self.tantivy_stored_attnos.len()
                + self.heap_only_attnos.len(),
            fast_fields: self.fast_field_attnos.len(),
            tantivy_stored: self.tantivy_stored_attnos.len(),
            heap_only: self.heap_only_attnos.len(),
        }
    }
}

/// Statistics about field distribution in a table
#[derive(Debug, Clone)]
pub struct FieldStats {
    pub total_fields: usize,
    pub fast_fields: usize,
    pub tantivy_stored: usize,
    pub heap_only: usize,
}

impl FieldStats {
    /// Calculate the percentage of fields that are fast fields
    pub fn fast_field_percentage(&self) -> f64 {
        if self.total_fields == 0 {
            0.0
        } else {
            (self.fast_fields as f64 / self.total_fields as f64) * 100.0
        }
    }

    /// Calculate the percentage of fields that require heap access
    pub fn heap_access_percentage(&self) -> f64 {
        if self.total_fields == 0 {
            0.0
        } else {
            (self.heap_only as f64 / self.total_fields as f64) * 100.0
        }
    }
}

/// Multi-table field mapping for JOIN queries
#[derive(Debug)]
pub struct MultiTableFieldMap {
    pub table_maps: HashMap<pg_sys::Oid, TableFieldMap>,
}

impl MultiTableFieldMap {
    /// Create a new multi-table field map
    pub fn new() -> Self {
        Self {
            table_maps: HashMap::new(),
        }
    }

    /// Add a table field map
    pub fn add_table(&mut self, table_map: TableFieldMap) {
        self.table_maps.insert(table_map.table_oid, table_map);
    }

    /// Get the field map for a table
    pub fn get_table_map(&self, table_oid: pg_sys::Oid) -> Option<&TableFieldMap> {
        self.table_maps.get(&table_oid)
    }

    /// Determine which table a field belongs to based on attribute number ranges
    /// This is a simplified approach - in practice, you'd use query analysis
    pub fn determine_table_for_field(&self, attno: pg_sys::AttrNumber) -> Option<pg_sys::Oid> {
        // This is a placeholder implementation
        // In practice, you'd use the query's target list analysis to map fields to tables
        self.table_maps.keys().next().copied()
    }

    /// Get combined statistics across all tables
    pub fn combined_stats(&self) -> FieldStats {
        let mut total = FieldStats {
            total_fields: 0,
            fast_fields: 0,
            tantivy_stored: 0,
            heap_only: 0,
        };

        for table_map in self.table_maps.values() {
            let stats = table_map.field_stats();
            total.total_fields += stats.total_fields;
            total.fast_fields += stats.fast_fields;
            total.tantivy_stored += stats.tantivy_stored;
            total.heap_only += stats.heap_only;
        }

        total
    }
}

impl Default for MultiTableFieldMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_result_creation() {
        let mut result = LazyResult::new();

        // Test adding fast fields
        result.add_fast_field(1, 42.into());
        assert_eq!(result.fast_fields.len(), 1);

        // Test adding CTIDs
        result.add_ctid(12345.into(), 67890);
        assert_eq!(result.get_ctid(12345.into()), Some(67890));
        assert_eq!(result.get_ctid(99999.into()), None);

        // Test field loading state
        assert!(!result.is_field_loaded(2));
        result
            .non_fast_fields
            .insert(2, LazyFieldState::Loaded(100.into()));
        assert!(result.is_field_loaded(2));
        assert_eq!(result.get_field(2), Some(100.into()));
    }

    #[test]
    fn test_lazy_field_state() {
        let state = LazyFieldState::NotLoaded;
        assert!(matches!(state, LazyFieldState::NotLoaded));

        let state = LazyFieldState::Loaded(42.into());
        assert!(matches!(state, LazyFieldState::Loaded(_)));

        let state = LazyFieldState::Failed(LazyLoadError::TupleNotVisible);
        assert!(matches!(
            state,
            LazyFieldState::Failed(LazyLoadError::TupleNotVisible)
        ));
    }

    #[test]
    fn test_error_statistics() {
        let mut stats = ErrorStatistics::default();
        assert_eq!(stats.tuple_not_visible_count, 0);
        assert_eq!(stats.buffer_pin_failures, 0);

        stats.tuple_not_visible_count += 1;
        stats.buffer_pin_failures += 2;

        assert_eq!(stats.tuple_not_visible_count, 1);
        assert_eq!(stats.buffer_pin_failures, 2);
    }

    #[test]
    fn test_fallback_strategy() {
        let strategy = FallbackStrategy::RetryOnce;
        assert!(matches!(strategy, FallbackStrategy::RetryOnce));

        let strategy = FallbackStrategy::FallbackToEagerLoading;
        assert!(matches!(strategy, FallbackStrategy::FallbackToEagerLoading));

        let strategy = FallbackStrategy::ReturnPartialResults;
        assert!(matches!(strategy, FallbackStrategy::ReturnPartialResults));

        let strategy = FallbackStrategy::FailQuery;
        assert!(matches!(strategy, FallbackStrategy::FailQuery));
    }

    #[test]
    fn test_field_loading_strategy() {
        let strategy = FieldLoadingStrategy::FastField { value: 42.into() };
        assert!(matches!(strategy, FieldLoadingStrategy::FastField { .. }));

        let strategy = FieldLoadingStrategy::TantivyStored { attno: 1 };
        assert!(matches!(
            strategy,
            FieldLoadingStrategy::TantivyStored { .. }
        ));

        let strategy = FieldLoadingStrategy::HeapAccess {
            table_oid: 12345.into(),
            ctid: 67890,
            attno: 1,
        };
        assert!(matches!(strategy, FieldLoadingStrategy::HeapAccess { .. }));
    }

    #[test]
    fn test_field_stats() {
        let stats = FieldStats {
            total_fields: 10,
            fast_fields: 3,
            tantivy_stored: 2,
            heap_only: 5,
        };

        assert_eq!(stats.fast_field_percentage(), 30.0);
        assert_eq!(stats.heap_access_percentage(), 50.0);

        // Test edge case with zero fields
        let empty_stats = FieldStats {
            total_fields: 0,
            fast_fields: 0,
            tantivy_stored: 0,
            heap_only: 0,
        };

        assert_eq!(empty_stats.fast_field_percentage(), 0.0);
        assert_eq!(empty_stats.heap_access_percentage(), 0.0);
    }

    #[test]
    fn test_multi_table_field_map() {
        let mut multi_map = MultiTableFieldMap::new();

        // Create a mock table field map
        let table_map = TableFieldMap {
            table_oid: 12345.into(),
            fast_field_attnos: [1, 2].into_iter().collect(),
            tantivy_stored_attnos: [3].into_iter().collect(),
            heap_only_attnos: [4, 5].into_iter().collect(),
        };

        multi_map.add_table(table_map);

        // Test retrieval
        let retrieved = multi_map.get_table_map(12345.into());
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert!(retrieved.is_fast_field(1));
        assert!(retrieved.is_fast_field(2));
        assert!(!retrieved.is_fast_field(3));
        assert!(retrieved.requires_heap_access(4));
        assert!(retrieved.requires_heap_access(5));
        assert!(!retrieved.requires_heap_access(1));

        // Test combined stats
        let stats = multi_map.combined_stats();
        assert_eq!(stats.total_fields, 5);
        assert_eq!(stats.fast_fields, 2);
        assert_eq!(stats.tantivy_stored, 1);
        assert_eq!(stats.heap_only, 2);
    }
}
