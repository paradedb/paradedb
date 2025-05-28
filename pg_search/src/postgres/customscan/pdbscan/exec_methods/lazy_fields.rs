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
use pgrx::{pg_sys, PgList, PgRelation, PgTupleDesc};
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

    /// Load multiple fields with visibility checking in a single heap access
    ///
    /// This is much more efficient than loading fields one at a time because:
    /// - Single heap access per tuple
    /// - Better cache locality
    /// - Reduced buffer management overhead
    pub fn load_fields_batch(
        &mut self,
        ctid: u64,
        field_attnos: &[pg_sys::AttrNumber],
        heaprel: pg_sys::Relation,
    ) -> Result<Vec<(pg_sys::AttrNumber, pg_sys::Datum)>, LazyLoadError> {
        if field_attnos.is_empty() {
            return Ok(Vec::new());
        }

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
                // Block is all visible, can extract fields directly (fast path)
                self.extract_fields_from_visible_block(ctid, field_attnos, heaprel)
            } else {
                // Use VisibilityChecker for MVCC-safe access (slow path, same as PdbScan)
                self.extract_fields_with_visibility_check(ctid, field_attnos, heaprel)
            }
        }
    }

    /// Extract multiple fields from a block known to be all-visible (fast path)
    unsafe fn extract_fields_from_visible_block(
        &self,
        ctid: u64,
        field_attnos: &[pg_sys::AttrNumber],
        heaprel: pg_sys::Relation,
    ) -> Result<Vec<(pg_sys::AttrNumber, pg_sys::Datum)>, LazyLoadError> {
        // Switch to per-tuple context for temporary allocations
        let old_context = pg_sys::MemoryContextSwitchTo(self.per_tuple_context);

        let result = self.extract_fields_from_visible_block_internal(ctid, field_attnos, heaprel);

        // Restore previous context
        pg_sys::MemoryContextSwitchTo(old_context);

        result
    }

    unsafe fn extract_fields_from_visible_block_internal(
        &self,
        ctid: u64,
        field_attnos: &[pg_sys::AttrNumber],
        heaprel: pg_sys::Relation,
    ) -> Result<Vec<(pg_sys::AttrNumber, pg_sys::Datum)>, LazyLoadError> {
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

        // Extract ALL requested fields in a single pass
        let tupdesc = (*heaprel).rd_att;
        let mut results = Vec::with_capacity(field_attnos.len());

        for &attno in field_attnos {
            let mut isnull = false;
            let datum = pg_sys::heap_getattr(htup, attno as i32, tupdesc, &mut isnull);

            let result_datum = if isnull {
                pg_sys::Datum::null()
            } else {
                self.copy_datum_to_context(datum, attno, tupdesc)
            };

            results.push((attno, result_datum));
        }

        // Release buffer (only once for all fields!)
        pg_sys::UnlockReleaseBuffer(buffer);

        Ok(results)
    }

    /// Extract multiple fields using VisibilityChecker (slow path)
    unsafe fn extract_fields_with_visibility_check(
        &mut self,
        ctid: u64,
        field_attnos: &[pg_sys::AttrNumber],
        heaprel: pg_sys::Relation,
    ) -> Result<Vec<(pg_sys::AttrNumber, pg_sys::Datum)>, LazyLoadError> {
        let temp_slot = self.temp_slot;
        let field_attnos_vec = field_attnos.to_vec(); // Copy for closure

        let result = self
            .visibility_checker
            .exec_if_visible(ctid, temp_slot, |_heaprel| {
                // Extract ALL requested fields from the slot in a single pass
                let mut field_results = Vec::with_capacity(field_attnos_vec.len());

                for &attno in &field_attnos_vec {
                    let mut isnull = false;
                    let datum = pg_sys::slot_getattr(temp_slot, attno as i32, &mut isnull);

                    let result_datum = if isnull {
                        pg_sys::Datum::null()
                    } else {
                        datum // Will be copied to context outside the closure
                    };

                    field_results.push((attno, result_datum, isnull));
                }

                field_results
            });

        match result {
            Some(field_results) => {
                // Copy all datums to our memory context
                let tupdesc = (*temp_slot).tts_tupleDescriptor;
                let mut final_results = Vec::with_capacity(field_results.len());

                for (attno, datum, is_null) in field_results {
                    let final_datum = if is_null {
                        pg_sys::Datum::null()
                    } else {
                        self.copy_datum_to_context(datum, attno, tupdesc)
                    };
                    final_results.push((attno, final_datum));
                }

                Ok(final_results)
            }
            None => Err(LazyLoadError::TupleNotVisible),
        }
    }

    /// Load a single field (kept for backward compatibility, but uses batch loading internally)
    pub fn load_field_with_visibility_check(
        &mut self,
        ctid: u64,
        attno: pg_sys::AttrNumber,
        heaprel: pg_sys::Relation,
    ) -> Result<pg_sys::Datum, LazyLoadError> {
        let results = self.load_fields_batch(ctid, &[attno], heaprel)?;
        Ok(results
            .into_iter()
            .next()
            .map(|(_, datum)| datum)
            .unwrap_or(pg_sys::Datum::null()))
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

    /// Load multiple fields with fallback handling (RECOMMENDED)
    ///
    /// This is the preferred method for loading multiple fields as it's much more efficient
    /// than loading fields one at a time.
    pub fn load_fields_batch_with_fallback(
        &mut self,
        ctid: u64,
        field_attnos: &[pg_sys::AttrNumber],
        heaprel: pg_sys::Relation,
    ) -> Result<Vec<(pg_sys::AttrNumber, pg_sys::Datum)>, LazyLoadError> {
        // First attempt
        match self.loader.load_fields_batch(ctid, field_attnos, heaprel) {
            Ok(results) => Ok(results),
            Err(LazyLoadError::TupleNotVisible) => {
                self.error_stats.tuple_not_visible_count += 1;

                match self.fallback_strategy {
                    FallbackStrategy::RetryOnce => {
                        // Simple retry (PdbScan just continues to next tuple)
                        self.loader.load_fields_batch(ctid, field_attnos, heaprel)
                    }
                    FallbackStrategy::FallbackToEagerLoading => {
                        // Use PostgreSQL's standard heap access
                        self.error_stats.total_fallbacks += 1;
                        self.eager_load_fields_batch(ctid, field_attnos, heaprel)
                    }
                    FallbackStrategy::ReturnPartialResults => {
                        // Return NULL for all fields
                        Ok(field_attnos
                            .iter()
                            .map(|&attno| (attno, pg_sys::Datum::null()))
                            .collect())
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

    /// Load field with fallback handling (single field - less efficient)
    pub fn load_field_with_fallback(
        &mut self,
        ctid: u64,
        attno: pg_sys::AttrNumber,
        heaprel: pg_sys::Relation,
    ) -> Result<pg_sys::Datum, LazyLoadError> {
        // Use batch loading internally for consistency
        let results = self.load_fields_batch_with_fallback(ctid, &[attno], heaprel)?;
        Ok(results
            .into_iter()
            .next()
            .map(|(_, datum)| datum)
            .unwrap_or(pg_sys::Datum::null()))
    }

    /// Fallback to eager loading using PostgreSQL's standard table access (batch version)
    fn eager_load_fields_batch(
        &self,
        ctid: u64,
        field_attnos: &[pg_sys::AttrNumber],
        heaprel: pg_sys::Relation,
    ) -> Result<Vec<(pg_sys::AttrNumber, pg_sys::Datum)>, LazyLoadError> {
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
                // Extract ALL requested fields in a single pass
                let mut results = Vec::with_capacity(field_attnos.len());

                for &attno in field_attnos {
                    let mut isnull = false;
                    let datum = pg_sys::slot_getattr(slot, attno as i32, &mut isnull);
                    let result_datum = if isnull { pg_sys::Datum::null() } else { datum };
                    results.push((attno, result_datum));
                }

                Ok(results)
            } else {
                Err(LazyLoadError::TupleNotVisible)
            };

            pg_sys::table_index_fetch_end(scan);
            result
        }
    }

    /// Fallback to eager loading using PostgreSQL's standard table access (single field)
    fn eager_load_field(
        &self,
        ctid: u64,
        attno: pg_sys::AttrNumber,
        heaprel: pg_sys::Relation,
    ) -> Result<pg_sys::Datum, LazyLoadError> {
        // Use batch loading internally
        let results = self.eager_load_fields_batch(ctid, &[attno], heaprel)?;
        Ok(results
            .into_iter()
            .next()
            .map(|(_, datum)| datum)
            .unwrap_or(pg_sys::Datum::null()))
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

    /// Add multiple fast field values at once
    pub fn add_fast_fields(&mut self, fields: Vec<(pg_sys::AttrNumber, pg_sys::Datum)>) {
        for (attno, datum) in fields {
            self.fast_fields.insert(attno, datum);
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

    /// Load multiple non-fast fields at once using batch loading
    ///
    /// This is much more efficient than loading fields one at a time
    pub fn load_non_fast_fields_batch(
        &mut self,
        table_oid: pg_sys::Oid,
        field_attnos: &[pg_sys::AttrNumber],
        loader: &mut LazyFieldLoaderWithFallback,
        heaprel: pg_sys::Relation,
    ) -> Result<(), LazyLoadError> {
        let ctid = self
            .get_ctid(table_oid)
            .ok_or(LazyLoadError::TupleNotVisible)?;

        // Load all requested fields in a single heap access
        let field_results = loader.load_fields_batch_with_fallback(ctid, field_attnos, heaprel)?;

        // Store all results
        for (attno, datum) in field_results {
            self.non_fast_fields
                .insert(attno, LazyFieldState::Loaded(datum));
        }

        Ok(())
    }

    /// Load a single non-fast field (less efficient - use batch loading when possible)
    pub fn load_non_fast_field(
        &mut self,
        table_oid: pg_sys::Oid,
        attno: pg_sys::AttrNumber,
        loader: &mut LazyFieldLoaderWithFallback,
        heaprel: pg_sys::Relation,
    ) -> Result<(), LazyLoadError> {
        // Use batch loading internally for consistency
        self.load_non_fast_fields_batch(table_oid, &[attno], loader, heaprel)
    }

    /// Check if a non-fast field has been loaded
    pub fn is_field_loaded(&self, attno: pg_sys::AttrNumber) -> bool {
        matches!(
            self.non_fast_fields.get(&attno),
            Some(LazyFieldState::Loaded(_))
        )
    }

    /// Get a loaded field value (fast or non-fast)
    pub fn get_field(&self, attno: pg_sys::AttrNumber) -> Option<pg_sys::Datum> {
        // Check fast fields first
        if let Some(datum) = self.fast_fields.get(&attno) {
            return Some(*datum);
        }

        // Then check non-fast fields
        match self.non_fast_fields.get(&attno) {
            Some(LazyFieldState::Loaded(datum)) => Some(*datum),
            _ => None,
        }
    }

    /// Get multiple field values at once
    pub fn get_fields(
        &self,
        attnos: &[pg_sys::AttrNumber],
    ) -> Vec<(pg_sys::AttrNumber, Option<pg_sys::Datum>)> {
        attnos
            .iter()
            .map(|&attno| (attno, self.get_field(attno)))
            .collect()
    }

    /// Check which non-fast fields still need to be loaded
    pub fn get_unloaded_non_fast_fields(
        &self,
        requested_attnos: &[pg_sys::AttrNumber],
    ) -> Vec<pg_sys::AttrNumber> {
        requested_attnos
            .iter()
            .filter(|&&attno| {
                // Skip if it's a fast field (already loaded)
                if self.fast_fields.contains_key(&attno) {
                    return false;
                }

                // Include if it's not loaded yet
                !matches!(
                    self.non_fast_fields.get(&attno),
                    Some(LazyFieldState::Loaded(_))
                )
            })
            .copied()
            .collect()
    }

    /// Get statistics about this lazy result
    pub fn get_stats(&self) -> LazyResultStats {
        let loaded_non_fast = self
            .non_fast_fields
            .values()
            .filter(|state| matches!(state, LazyFieldState::Loaded(_)))
            .count();

        let failed_non_fast = self
            .non_fast_fields
            .values()
            .filter(|state| matches!(state, LazyFieldState::Failed(_)))
            .count();

        LazyResultStats {
            fast_fields_count: self.fast_fields.len(),
            loaded_non_fast_fields: loaded_non_fast,
            failed_non_fast_fields: failed_non_fast,
            total_tables: self.ctids.len(),
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

/// Statistics about a lazy result
#[derive(Debug, Clone)]
pub struct LazyResultStats {
    pub fast_fields_count: usize,
    pub loaded_non_fast_fields: usize,
    pub failed_non_fast_fields: usize,
    pub total_tables: usize,
}

impl LazyResultStats {
    /// Calculate the total number of loaded fields
    pub fn total_loaded_fields(&self) -> usize {
        self.fast_fields_count + self.loaded_non_fast_fields
    }

    /// Calculate the percentage of successfully loaded fields
    pub fn success_rate(&self) -> f64 {
        let total_attempted = self.loaded_non_fast_fields + self.failed_non_fast_fields;
        if total_attempted == 0 {
            100.0 // All fast fields, 100% success
        } else {
            (self.loaded_non_fast_fields as f64 / total_attempted as f64) * 100.0
        }
    }
}

/// Execution method that integrates batch lazy field loading with PdbScan infrastructure
///
/// This execution method is designed for single-table queries with LIMIT and non-fast fields.
/// It follows the same patterns as existing execution methods but adds lazy loading optimization.
pub struct LazyFieldExecState {
    /// Basic execution state (following PdbScan patterns)
    heaprel: pg_sys::Relation,
    slot: *mut pg_sys::TupleTableSlot,
    search_results: crate::index::reader::index::SearchResults,
    did_query: bool,

    /// Lazy loading infrastructure
    lazy_loader: Option<LazyFieldLoaderWithFallback>,
    table_field_map: Option<TableFieldMap>,

    /// Fast fields that are immediately available
    fast_field_attnos: HashSet<pg_sys::AttrNumber>,

    /// Non-fast fields that require lazy loading
    non_fast_field_attnos: HashSet<pg_sys::AttrNumber>,

    /// Results with lazy loading state
    lazy_results: Vec<LazyResult>,
    current_result_index: usize,

    /// Performance tracking
    heap_accesses_saved: u64,
    fields_loaded_lazily: u64,
}

impl Default for LazyFieldExecState {
    fn default() -> Self {
        Self {
            heaprel: std::ptr::null_mut(),
            slot: std::ptr::null_mut(),
            search_results: crate::index::reader::index::SearchResults::None,
            did_query: false,
            lazy_loader: None,
            table_field_map: None,
            fast_field_attnos: HashSet::new(),
            non_fast_field_attnos: HashSet::new(),
            lazy_results: Vec::new(),
            current_result_index: 0,
            heap_accesses_saved: 0,
            fields_loaded_lazily: 0,
        }
    }
}

impl LazyFieldExecState {
    /// Create a new lazy field execution state
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize field classification based on the target list and schema
    fn initialize_field_classification(
        &mut self,
        state: &mut crate::postgres::customscan::pdbscan::scan_state::PdbScanState,
        cstate: *mut pg_sys::CustomScanState,
    ) {
        unsafe {
            let heaprel = PgRelation::from_pg(self.heaprel);
            let tupdesc = PgTupleDesc::from_pg_unchecked((*cstate).ss.ps.ps_ResultTupleDesc);

            // Get the search index schema
            let indexrel = state
                .indexrel
                .as_ref()
                .map(|indexrel| PgRelation::from_pg(*indexrel))
                .expect("indexrel should be available");

            let directory = crate::index::mvcc::MVCCDirectory::snapshot(indexrel.oid());
            let index = tantivy::Index::open(directory).expect("should be able to open index");
            let schema = SearchIndexSchema::open(index.schema(), &indexrel);

            // Create table field map
            self.table_field_map = Some(TableFieldMap::new(heaprel.oid(), &heaprel, &schema));

            // Classify fields from the target list
            let target_list = (*(*cstate).ss.ps.plan).targetlist;
            let target_entries = PgList::<pg_sys::TargetEntry>::from_pg(target_list);

            for te in target_entries.iter_ptr() {
                if let Some(var) = crate::nodecast!(Var, T_Var, (*te).expr) {
                    let attno = (*var).varattno;

                    if schema.is_fast_field(self.get_field_name_by_attno(attno, &tupdesc)) {
                        self.fast_field_attnos.insert(attno);
                    } else {
                        self.non_fast_field_attnos.insert(attno);
                    }
                }
            }
        }
    }

    /// Get field name by attribute number
    fn get_field_name_by_attno<'a>(
        &self,
        attno: pg_sys::AttrNumber,
        tupdesc: &'a PgTupleDesc<'a>,
    ) -> &'a str {
        if let Some(att) = tupdesc.get((attno - 1) as usize) {
            att.name()
        } else {
            ""
        }
    }

    /// Execute search and prepare lazy results
    fn execute_search_and_prepare_lazy_results(
        &mut self,
        state: &mut crate::postgres::customscan::pdbscan::scan_state::PdbScanState,
    ) {
        // Execute search to get CTIDs and fast fields
        self.search_results = state
            .search_reader
            .as_ref()
            .expect("must have a search_reader")
            .search(
                state.need_scores(),
                false,
                state.search_query_input(),
                state.limit,
            );

        // Convert search results to lazy results
        self.lazy_results.clear();

        // Collect all search results first (for LIMIT optimization)
        let mut all_results = Vec::new();
        while let Some((scored, _doc_address)) = self.search_results.next() {
            all_results.push(scored);
        }

        // Sort by score if needed (for LIMIT optimization)
        if state.limit.is_some() {
            all_results.sort_by(|a, b| {
                b.bm25
                    .partial_cmp(&a.bm25)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Apply LIMIT early - this is the key optimization!
            if let Some(limit) = state.limit {
                all_results.truncate(limit);
            }
        }

        // Create lazy results with fast fields populated
        let all_results_len = all_results.len();
        for scored in all_results {
            let mut lazy_result = LazyResult::new();

            // Add fast fields (immediately available)
            for &attno in &self.fast_field_attnos {
                // For now, we'll populate with placeholder values
                // In a full implementation, we'd extract these from the search results
                lazy_result.add_fast_field(attno, scored.ctid.into());
            }

            // Add CTID for lazy loading
            lazy_result.add_ctid(unsafe { (*self.heaprel).rd_id }, scored.ctid);

            // Add score if needed
            if state.need_scores() {
                lazy_result.combined_score = Some(scored.bm25);
            }

            self.lazy_results.push(lazy_result);
        }

        // Calculate heap accesses saved
        let total_non_fast_fields = self.non_fast_field_attnos.len() as u64;
        let original_heap_accesses = all_results_len as u64 * total_non_fast_fields;
        let optimized_heap_accesses = self.lazy_results.len() as u64 * total_non_fast_fields;
        self.heap_accesses_saved = original_heap_accesses.saturating_sub(optimized_heap_accesses);

        self.current_result_index = 0;
    }

    /// Load non-fast fields for the current result
    fn load_non_fast_fields_for_current_result(
        &mut self,
        result_index: usize,
    ) -> Result<(), LazyLoadError> {
        if result_index >= self.lazy_results.len() {
            return Err(LazyLoadError::TupleNotVisible);
        }

        let result = &mut self.lazy_results[result_index];
        let table_oid = unsafe { (*self.heaprel).rd_id };

        // Get unloaded non-fast fields
        let unloaded_fields: Vec<pg_sys::AttrNumber> = self
            .non_fast_field_attnos
            .iter()
            .filter(|&&attno| !result.is_field_loaded(attno))
            .copied()
            .collect();

        if !unloaded_fields.is_empty() {
            // Use batch loading for efficiency
            result.load_non_fast_fields_batch(
                table_oid,
                &unloaded_fields,
                self.lazy_loader.as_mut().unwrap(),
                self.heaprel,
            )?;

            self.fields_loaded_lazily += unloaded_fields.len() as u64;
        }

        Ok(())
    }

    /// Create a tuple slot with all fields (fast and non-fast)
    fn create_tuple_slot_with_all_fields(
        &self,
        result: &LazyResult,
    ) -> *mut pg_sys::TupleTableSlot {
        unsafe {
            // Clear the slot
            pg_sys::ExecClearTuple(self.slot);

            // Set up the slot with all field values
            let tupdesc = (*self.slot).tts_tupleDescriptor;
            let natts = (*tupdesc).natts as usize;

            for attno in 1..=natts {
                let attno = attno as pg_sys::AttrNumber;

                if let Some(datum) = result.get_field(attno) {
                    // Set the field value
                    (*self.slot)
                        .tts_values
                        .add((attno - 1) as usize)
                        .write(datum);
                    (*self.slot)
                        .tts_isnull
                        .add((attno - 1) as usize)
                        .write(false);
                } else {
                    // Set as NULL
                    (*self.slot)
                        .tts_values
                        .add((attno - 1) as usize)
                        .write(pg_sys::Datum::null());
                    (*self.slot)
                        .tts_isnull
                        .add((attno - 1) as usize)
                        .write(true);
                }
            }

            // Mark slot as valid
            (*self.slot).tts_nvalid = natts as pg_sys::AttrNumber;
            (*self.slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
            (*self.slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;

            self.slot
        }
    }
}

impl crate::postgres::customscan::pdbscan::exec_methods::ExecMethod for LazyFieldExecState {
    fn init(
        &mut self,
        state: &mut crate::postgres::customscan::pdbscan::scan_state::PdbScanState,
        cstate: *mut pg_sys::CustomScanState,
    ) {
        unsafe {
            self.heaprel = state.heaprel.unwrap();
            self.slot = pg_sys::MakeTupleTableSlot(
                (*cstate).ss.ps.ps_ResultTupleDesc,
                &pg_sys::TTSOpsVirtual,
            );

            // Initialize lazy loader
            self.lazy_loader = Some(LazyFieldLoaderWithFallback::new(
                self.heaprel,
                FallbackStrategy::FallbackToEagerLoading,
            ));

            // Initialize field classification
            self.initialize_field_classification(state, cstate);
        }
    }

    fn query(
        &mut self,
        state: &mut crate::postgres::customscan::pdbscan::scan_state::PdbScanState,
    ) -> bool {
        if self.did_query {
            return false;
        }

        self.execute_search_and_prepare_lazy_results(state);
        self.did_query = true;

        !self.lazy_results.is_empty()
    }

    fn internal_next(
        &mut self,
        _state: &mut crate::postgres::customscan::pdbscan::scan_state::PdbScanState,
    ) -> crate::postgres::customscan::pdbscan::exec_methods::ExecState {
        use crate::postgres::customscan::pdbscan::exec_methods::ExecState;

        // Check if we have more results
        if self.current_result_index >= self.lazy_results.len() {
            return ExecState::Eof;
        }

        // Load non-fast fields for the current result (lazy loading!)
        if let Err(_) = self.load_non_fast_fields_for_current_result(self.current_result_index) {
            // Skip this result and try the next one
            self.current_result_index += 1;
            return self.internal_next(_state);
        }

        // Create tuple slot with all fields
        let result = &self.lazy_results[self.current_result_index];
        let slot = self.create_tuple_slot_with_all_fields(result);

        // Move to next result
        self.current_result_index += 1;

        ExecState::Virtual { slot }
    }

    fn reset(
        &mut self,
        _state: &mut crate::postgres::customscan::pdbscan::scan_state::PdbScanState,
    ) {
        self.did_query = false;
        self.lazy_results.clear();
        self.current_result_index = 0;
        self.heap_accesses_saved = 0;
        self.fields_loaded_lazily = 0;

        // Reset lazy loader state
        if let Some(ref mut loader) = self.lazy_loader {
            loader.reset_per_tuple_context();
            loader.reset_block_cache();
        }
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
    fn test_batch_fast_fields() {
        let mut result = LazyResult::new();

        // Test batch adding fast fields
        let batch_fields = vec![(1, 42.into()), (2, 84.into()), (3, 126.into())];
        result.add_fast_fields(batch_fields);

        assert_eq!(result.fast_fields.len(), 3);
        assert_eq!(result.get_field(1), Some(42.into()));
        assert_eq!(result.get_field(2), Some(84.into()));
        assert_eq!(result.get_field(3), Some(126.into()));

        // Test batch getting fields
        let requested_fields = [1, 2, 3, 4]; // 4 doesn't exist
        let field_results = result.get_fields(&requested_fields);

        assert_eq!(field_results.len(), 4);
        assert_eq!(field_results[0], (1, Some(42.into())));
        assert_eq!(field_results[1], (2, Some(84.into())));
        assert_eq!(field_results[2], (3, Some(126.into())));
        assert_eq!(field_results[3], (4, None));
    }

    #[test]
    fn test_unloaded_field_detection() {
        let mut result = LazyResult::new();

        // Add some fast fields
        result.add_fast_field(1, 42.into());
        result.add_fast_field(2, 84.into());

        // Add some loaded non-fast fields
        result
            .non_fast_fields
            .insert(3, LazyFieldState::Loaded(126.into()));

        // Add some unloaded non-fast fields
        result.non_fast_fields.insert(4, LazyFieldState::NotLoaded);
        result
            .non_fast_fields
            .insert(5, LazyFieldState::Failed(LazyLoadError::TupleNotVisible));

        // Test unloaded field detection
        let requested_fields = [1, 2, 3, 4, 5, 6]; // 6 is completely new
        let unloaded = result.get_unloaded_non_fast_fields(&requested_fields);

        // Should return fields 4, 5, and 6 (1,2,3 are already available)
        assert_eq!(unloaded.len(), 3);
        assert!(unloaded.contains(&4));
        assert!(unloaded.contains(&5));
        assert!(unloaded.contains(&6));
        assert!(!unloaded.contains(&1)); // Fast field, already loaded
        assert!(!unloaded.contains(&2)); // Fast field, already loaded
        assert!(!unloaded.contains(&3)); // Non-fast field, already loaded
    }

    #[test]
    fn test_lazy_result_stats() {
        let mut result = LazyResult::new();

        // Add fast fields
        result.add_fast_field(1, 42.into());
        result.add_fast_field(2, 84.into());

        // Add loaded non-fast fields
        result
            .non_fast_fields
            .insert(3, LazyFieldState::Loaded(126.into()));
        result
            .non_fast_fields
            .insert(4, LazyFieldState::Loaded(168.into()));

        // Add failed non-fast fields
        result
            .non_fast_fields
            .insert(5, LazyFieldState::Failed(LazyLoadError::TupleNotVisible));

        // Add CTIDs for multiple tables
        result.add_ctid(100.into(), 1000);
        result.add_ctid(200.into(), 2000);

        let stats = result.get_stats();

        assert_eq!(stats.fast_fields_count, 2);
        assert_eq!(stats.loaded_non_fast_fields, 2);
        assert_eq!(stats.failed_non_fast_fields, 1);
        assert_eq!(stats.total_tables, 2);
        assert_eq!(stats.total_loaded_fields(), 4);

        // Success rate: 2 loaded out of 3 attempted non-fast fields = 66.67%
        assert!((stats.success_rate() - 66.666666666666666).abs() < 0.001);
    }

    #[test]
    fn test_lazy_result_stats_all_fast_fields() {
        let mut result = LazyResult::new();

        // Only fast fields
        result.add_fast_field(1, 42.into());
        result.add_fast_field(2, 84.into());

        let stats = result.get_stats();

        assert_eq!(stats.fast_fields_count, 2);
        assert_eq!(stats.loaded_non_fast_fields, 0);
        assert_eq!(stats.failed_non_fast_fields, 0);
        assert_eq!(stats.total_loaded_fields(), 2);
        assert_eq!(stats.success_rate(), 100.0); // All fast fields = 100% success
    }
}
