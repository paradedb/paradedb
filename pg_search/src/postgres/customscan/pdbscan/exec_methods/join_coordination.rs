//! JOIN Coordination Execution Method
//!
//! This module implements a specialized execution method for optimizing JOIN queries
//! with search predicates on multiple tables. The key optimization is to:
//!
//! 1. Execute searches on all tables in parallel
//! 2. Perform early intersection using fast fields (CTIDs, scores)
//! 3. Apply LIMIT early using combined scores
//! 4. Load non-fast fields only for final results
//!
//! Target query pattern:
//! ```sql
//! SELECT d.title, f.filename, p.content
//! FROM documents d
//! JOIN files f ON d.id = f.document_id
//! JOIN pages p ON f.id = p.file_id  
//! WHERE d.content @@@ 'search' AND f.title @@@ 'report' AND p.content @@@ 'analysis'
//! ORDER BY paradedb.score(d.id) + paradedb.score(f.id) + paradedb.score(p.id) DESC
//! LIMIT 100;
//! ```

use crate::postgres::customscan::pdbscan::exec_methods::lazy_fields::{
    FallbackStrategy, LazyFieldLoaderWithFallback, LazyResult, TableFieldMap,
};
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::postgres::utils;
use crate::schema::SearchIndexSchema;
use pgrx::{pg_sys, PgList, PgRelation, PgTupleDesc};
use std::collections::HashMap;
use std::collections::HashSet;
use tantivy::{DocAddress, Score};

/// Information about a table participating in the JOIN
#[derive(Debug, Clone)]
pub struct JoinTable {
    /// Table OID
    pub table_oid: pg_sys::Oid,

    /// Heap relation
    pub heaprel: pg_sys::Relation,

    /// Index relation  
    pub indexrel: pg_sys::Relation,

    /// Search query for this table
    pub search_query: String,

    /// Field mapping for this table
    pub field_map: TableFieldMap,

    /// Fast field attribute numbers
    pub fast_field_attnos: HashSet<pg_sys::AttrNumber>,

    /// Non-fast field attribute numbers  
    pub non_fast_field_attnos: HashSet<pg_sys::AttrNumber>,
}

/// A search result from a single table
#[derive(Debug, Clone)]
pub struct TableSearchResult {
    /// Table this result came from
    pub table_oid: pg_sys::Oid,

    /// CTID of the tuple
    pub ctid: u64,

    /// BM25 score
    pub score: Score,

    /// Document address in Tantivy
    pub doc_address: DocAddress,

    /// Fast field values (immediately available)
    pub fast_fields: HashMap<pg_sys::AttrNumber, pg_sys::Datum>,
}

/// A joined result combining multiple table results
#[derive(Debug)]
pub struct JoinedResult {
    /// Results from each table (keyed by table OID)
    pub table_results: HashMap<pg_sys::Oid, TableSearchResult>,

    /// Combined score across all tables
    pub combined_score: Score,

    /// Lazy loading state for non-fast fields
    pub lazy_results: HashMap<pg_sys::Oid, LazyResult>,
}

impl JoinedResult {
    /// Create a new joined result
    pub fn new() -> Self {
        Self {
            table_results: HashMap::new(),
            combined_score: 0.0,
            lazy_results: HashMap::new(),
        }
    }

    /// Add a table result
    pub fn add_table_result(&mut self, result: TableSearchResult) {
        self.combined_score += result.score;
        self.table_results.insert(result.table_oid, result);
    }

    /// Check if this joined result satisfies the JOIN conditions
    /// For now, this is a placeholder - real implementation would check foreign key relationships
    pub fn satisfies_join_conditions(&self, _join_tables: &[JoinTable]) -> bool {
        // Placeholder: assume all results satisfy JOIN conditions
        // Real implementation would check:
        // - documents.id = files.document_id
        // - files.id = pages.file_id
        true
    }
}

/// JOIN coordination execution state
pub struct JoinCoordinationExecState {
    /// Tables participating in the JOIN
    join_tables: Vec<JoinTable>,

    /// Search results from each table (before joining)
    table_search_results: HashMap<pg_sys::Oid, Vec<TableSearchResult>>,

    /// Joined results (after intersection and JOIN condition checking)
    joined_results: Vec<JoinedResult>,

    /// Current result index for iteration
    current_result_index: usize,

    /// Lazy field loaders for each table
    lazy_loaders: HashMap<pg_sys::Oid, LazyFieldLoaderWithFallback>,

    /// Target list slot for result projection
    result_slot: *mut pg_sys::TupleTableSlot,

    /// LIMIT for early termination
    limit: Option<usize>,

    /// Whether we've executed the searches
    did_search: bool,

    /// Performance statistics
    total_search_results: usize,
    joined_results_count: usize,
    heap_accesses_saved: u64,
}

impl Default for JoinCoordinationExecState {
    fn default() -> Self {
        Self {
            join_tables: Vec::new(),
            table_search_results: HashMap::new(),
            joined_results: Vec::new(),
            current_result_index: 0,
            lazy_loaders: HashMap::new(),
            result_slot: std::ptr::null_mut(),
            limit: None,
            did_search: false,
            total_search_results: 0,
            joined_results_count: 0,
            heap_accesses_saved: 0,
        }
    }
}

impl JoinCoordinationExecState {
    /// Create a new JOIN coordination execution state
    pub fn new(limit: Option<usize>) -> Self {
        Self {
            limit,
            ..Default::default()
        }
    }

    /// Initialize JOIN tables from the query plan
    /// This is a simplified version - real implementation would parse the JOIN tree
    fn initialize_join_tables(
        &mut self,
        state: &mut PdbScanState,
        cstate: *mut pg_sys::CustomScanState,
    ) {
        unsafe {
            // For now, we'll just handle the single table case as a starting point
            // Real implementation would parse the JOIN tree and identify all participating tables

            let heaprel = state.heaprel.unwrap();
            let indexrel = state.indexrel.unwrap();

            let heaprel_pg = PgRelation::from_pg(heaprel);
            let indexrel_pg = PgRelation::from_pg(indexrel);

            // Create search index schema
            let directory = crate::index::mvcc::MVCCDirectory::snapshot(indexrel_pg.oid());
            let index = tantivy::Index::open(directory).expect("should be able to open index");
            let schema = SearchIndexSchema::open(index.schema(), &indexrel_pg);

            // Create table field map
            let field_map = TableFieldMap::new(heaprel_pg.oid(), &heaprel_pg, &schema);

            // Classify fields from target list
            let target_list = (*(*cstate).ss.ps.plan).targetlist;
            let target_entries = PgList::<pg_sys::TargetEntry>::from_pg(target_list);
            let tupdesc = PgTupleDesc::from_pg_unchecked((*cstate).ss.ps.ps_ResultTupleDesc);

            let mut fast_field_attnos = HashSet::new();
            let mut non_fast_field_attnos = HashSet::new();

            for te in target_entries.iter_ptr() {
                if let Some(var) = crate::nodecast!(Var, T_Var, (*te).expr) {
                    let attno = (*var).varattno;

                    if let Some(att) = tupdesc.get((attno - 1) as usize) {
                        if schema.is_fast_field(att.name()) {
                            fast_field_attnos.insert(attno);
                        } else {
                            non_fast_field_attnos.insert(attno);
                        }
                    }
                }
            }

            // Create JOIN table entry
            let join_table = JoinTable {
                table_oid: heaprel_pg.oid(),
                heaprel,
                indexrel,
                search_query: "placeholder".to_string(), // Real implementation would extract from quals
                field_map,
                fast_field_attnos,
                non_fast_field_attnos,
            };

            self.join_tables.push(join_table);

            // Initialize lazy loaders
            self.lazy_loaders.insert(
                heaprel_pg.oid(),
                LazyFieldLoaderWithFallback::new(heaprel, FallbackStrategy::FallbackToEagerLoading),
            );
        }
    }

    /// Execute searches on all tables in parallel
    /// This is the key optimization - instead of letting PostgreSQL do hash joins,
    /// we coordinate the searches ourselves
    fn execute_parallel_searches(&mut self, state: &mut PdbScanState) {
        // For now, implement single-table search as a starting point
        // Real implementation would execute searches on all tables in parallel

        if let Some(search_reader) = &state.search_reader {
            let search_results = search_reader.search(
                state.need_scores(),
                false,
                state.search_query_input(),
                None, // Don't apply LIMIT here - we'll do it after joining
            );

            let mut table_results = Vec::new();
            let mut result_count = 0;

            // Convert search results to table search results
            for (scored, doc_address) in search_results {
                let table_result = TableSearchResult {
                    table_oid: self.join_tables[0].table_oid,
                    ctid: scored.ctid,
                    score: scored.bm25,
                    doc_address,
                    fast_fields: HashMap::new(), // TODO: Extract fast fields from search results
                };

                table_results.push(table_result);
                result_count += 1;
            }

            self.total_search_results = result_count;
            self.table_search_results
                .insert(self.join_tables[0].table_oid, table_results);
        }
    }

    /// Perform early intersection and JOIN condition checking
    /// This is where we avoid the expensive PostgreSQL hash joins
    fn perform_early_intersection(&mut self) {
        // For single table, this is straightforward
        // Real implementation would perform intersection across multiple tables

        if let Some(table_results) = self
            .table_search_results
            .get(&self.join_tables[0].table_oid)
        {
            let mut joined_results = Vec::new();

            for table_result in table_results {
                let mut joined_result = JoinedResult::new();
                joined_result.add_table_result(table_result.clone());

                // Check JOIN conditions (placeholder for now)
                if joined_result.satisfies_join_conditions(&self.join_tables) {
                    joined_results.push(joined_result);
                }
            }

            // Sort by combined score (descending)
            joined_results.sort_by(|a, b| {
                b.combined_score
                    .partial_cmp(&a.combined_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Apply LIMIT early - this is the key optimization!
            if let Some(limit) = self.limit {
                joined_results.truncate(limit);
            }

            self.joined_results_count = joined_results.len();
            self.joined_results = joined_results;

            // Calculate heap accesses saved
            let total_non_fast_fields: usize = self
                .join_tables
                .iter()
                .map(|table| table.non_fast_field_attnos.len())
                .sum();

            let original_heap_accesses = self.total_search_results * total_non_fast_fields;
            let optimized_heap_accesses = self.joined_results_count * total_non_fast_fields;

            self.heap_accesses_saved =
                (original_heap_accesses.saturating_sub(optimized_heap_accesses)) as u64;
        }
    }

    /// Load non-fast fields for a joined result
    fn load_non_fast_fields_for_result(
        &mut self,
        result_index: usize,
    ) -> Result<(), crate::postgres::customscan::pdbscan::exec_methods::lazy_fields::LazyLoadError>
    {
        if result_index >= self.joined_results.len() {
            return Err(crate::postgres::customscan::pdbscan::exec_methods::lazy_fields::LazyLoadError::TupleNotVisible);
        }

        let joined_result = &mut self.joined_results[result_index];

        // Load non-fast fields for each table in the JOIN
        for join_table in &self.join_tables {
            if let Some(table_result) = joined_result.table_results.get(&join_table.table_oid) {
                // Get unloaded non-fast fields
                let unloaded_fields: Vec<pg_sys::AttrNumber> =
                    join_table.non_fast_field_attnos.iter().copied().collect();

                if !unloaded_fields.is_empty() {
                    // Create lazy result if not exists
                    if !joined_result
                        .lazy_results
                        .contains_key(&join_table.table_oid)
                    {
                        let mut lazy_result = LazyResult::new();
                        lazy_result.add_ctid(join_table.table_oid, table_result.ctid);
                        joined_result
                            .lazy_results
                            .insert(join_table.table_oid, lazy_result);
                    }

                    // Load fields using batch loading
                    if let Some(lazy_result) =
                        joined_result.lazy_results.get_mut(&join_table.table_oid)
                    {
                        if let Some(loader) = self.lazy_loaders.get_mut(&join_table.table_oid) {
                            lazy_result.load_non_fast_fields_batch(
                                join_table.table_oid,
                                &unloaded_fields,
                                loader,
                                join_table.heaprel,
                            )?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Create a result tuple slot with all fields from all tables
    fn create_result_tuple_slot(
        &self,
        joined_result: &JoinedResult,
    ) -> *mut pg_sys::TupleTableSlot {
        unsafe {
            // Clear the slot
            pg_sys::ExecClearTuple(self.result_slot);

            // For now, just handle single table case
            // Real implementation would project fields from all tables

            if let Some(join_table) = self.join_tables.first() {
                if let Some(table_result) = joined_result.table_results.get(&join_table.table_oid) {
                    let tupdesc = (*self.result_slot).tts_tupleDescriptor;
                    let natts = (*tupdesc).natts as usize;

                    for attno in 1..=natts {
                        let attno = attno as pg_sys::AttrNumber;

                        // Check fast fields first
                        if let Some(datum) = table_result.fast_fields.get(&attno) {
                            (*self.result_slot)
                                .tts_values
                                .add((attno - 1) as usize)
                                .write(*datum);
                            (*self.result_slot)
                                .tts_isnull
                                .add((attno - 1) as usize)
                                .write(false);
                        }
                        // Then check lazy loaded fields
                        else if let Some(lazy_result) =
                            joined_result.lazy_results.get(&join_table.table_oid)
                        {
                            if let Some(datum) = lazy_result.get_field(attno) {
                                (*self.result_slot)
                                    .tts_values
                                    .add((attno - 1) as usize)
                                    .write(datum);
                                (*self.result_slot)
                                    .tts_isnull
                                    .add((attno - 1) as usize)
                                    .write(false);
                            } else {
                                (*self.result_slot)
                                    .tts_values
                                    .add((attno - 1) as usize)
                                    .write(pg_sys::Datum::null());
                                (*self.result_slot)
                                    .tts_isnull
                                    .add((attno - 1) as usize)
                                    .write(true);
                            }
                        }
                        // Default to NULL
                        else {
                            (*self.result_slot)
                                .tts_values
                                .add((attno - 1) as usize)
                                .write(pg_sys::Datum::null());
                            (*self.result_slot)
                                .tts_isnull
                                .add((attno - 1) as usize)
                                .write(true);
                        }
                    }

                    // Mark slot as valid
                    (*self.result_slot).tts_nvalid = natts as pg_sys::AttrNumber;
                    (*self.result_slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
                    (*self.result_slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
                }
            }

            self.result_slot
        }
    }
}

impl ExecMethod for JoinCoordinationExecState {
    fn init(&mut self, state: &mut PdbScanState, cstate: *mut pg_sys::CustomScanState) {
        unsafe {
            self.result_slot = pg_sys::MakeTupleTableSlot(
                (*cstate).ss.ps.ps_ResultTupleDesc,
                &pg_sys::TTSOpsVirtual,
            );

            // Initialize JOIN tables
            self.initialize_join_tables(state, cstate);
        }
    }

    fn query(&mut self, state: &mut PdbScanState) -> bool {
        if self.did_search {
            return false;
        }

        // Execute searches on all tables
        self.execute_parallel_searches(state);

        // Perform early intersection and JOIN condition checking
        self.perform_early_intersection();

        self.did_search = true;

        !self.joined_results.is_empty()
    }

    fn internal_next(&mut self, _state: &mut PdbScanState) -> ExecState {
        // Check if we have more results
        if self.current_result_index >= self.joined_results.len() {
            return ExecState::Eof;
        }

        // Load non-fast fields for the current result (lazy loading!)
        if let Err(_) = self.load_non_fast_fields_for_result(self.current_result_index) {
            // Skip this result and try the next one
            self.current_result_index += 1;
            return self.internal_next(_state);
        }

        // Create result tuple slot
        let joined_result = &self.joined_results[self.current_result_index];
        let slot = self.create_result_tuple_slot(joined_result);

        // Move to next result
        self.current_result_index += 1;

        ExecState::Virtual { slot }
    }

    fn reset(&mut self, _state: &mut PdbScanState) {
        self.did_search = false;
        self.table_search_results.clear();
        self.joined_results.clear();
        self.current_result_index = 0;
        self.total_search_results = 0;
        self.joined_results_count = 0;
        self.heap_accesses_saved = 0;

        // Reset lazy loaders
        for loader in self.lazy_loaders.values_mut() {
            loader.reset_per_tuple_context();
            loader.reset_block_cache();
        }
    }
}
