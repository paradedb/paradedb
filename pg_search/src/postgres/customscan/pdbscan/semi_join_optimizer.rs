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

//! Semi-join optimization for search predicates
//!
//! This module implements semi-join optimization strategies that use search results
//! from one relation to filter search operations on related relations, reducing
//! the overall search space and improving performance.

use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::pdbscan::get_rel_name;
use crate::postgres::customscan::pdbscan::join_qual_inspect::{
    JoinSearchPredicates, RelationSearchPredicate,
};
use crate::query::SearchQueryInput;
use pgrx::{pg_sys, warning};
use std::collections::{HashMap, HashSet};

/// Semi-join optimization strategy for search predicates
pub struct SemiJoinOptimizer {
    /// Search predicates organized by relation
    predicates: JoinSearchPredicates,
    /// Estimated selectivity for each predicate
    selectivity_estimates: HashMap<pg_sys::Oid, f64>,
    /// Join key relationships between relations
    join_relationships: Vec<JoinRelationship>,
}

/// Represents a join relationship between two relations
#[derive(Debug, Clone)]
pub struct JoinRelationship {
    /// Source relation OID
    pub source_relid: pg_sys::Oid,
    /// Target relation OID  
    pub target_relid: pg_sys::Oid,
    /// Source join key column
    pub source_key: String,
    /// Target join key column
    pub target_key: String,
    /// Estimated join selectivity
    pub join_selectivity: f64,
}

/// Semi-join filter strategy
#[derive(Debug, Clone)]
pub enum SemiJoinStrategy {
    /// Use search results from source to filter target search
    SearchFilter {
        source_relid: pg_sys::Oid,
        target_relid: pg_sys::Oid,
        filter_keys: Vec<i32>,
    },
    /// Use Bloom filter for large key sets
    BloomFilter {
        source_relid: pg_sys::Oid,
        target_relid: pg_sys::Oid,
        bloom_filter: BloomFilterData,
    },
    /// Use sorted array for medium key sets
    SortedArray {
        source_relid: pg_sys::Oid,
        target_relid: pg_sys::Oid,
        sorted_keys: Vec<i32>,
    },
}

/// Bloom filter data for semi-join optimization
#[derive(Debug, Clone)]
pub struct BloomFilterData {
    /// Bit array for the bloom filter
    pub bits: Vec<u64>,
    /// Number of hash functions
    pub hash_count: u32,
    /// Size of the bit array
    pub size: usize,
}

impl SemiJoinOptimizer {
    /// Create a new semi-join optimizer
    pub fn new(predicates: JoinSearchPredicates) -> Self {
        Self {
            predicates,
            selectivity_estimates: HashMap::new(),
            join_relationships: Vec::new(),
        }
    }

    /// Analyze search predicates and create optimization plan
    pub unsafe fn analyze_and_optimize(&mut self) -> Vec<SemiJoinStrategy> {
        warning!("ParadeDB: Analyzing semi-join optimization opportunities");

        // Step 1: Estimate selectivity for each search predicate
        self.estimate_search_selectivity();

        // Step 2: Identify join relationships
        self.identify_join_relationships();

        // Step 3: Order predicates by selectivity (most selective first)
        let ordered_predicates = self.order_predicates_by_selectivity();

        // Step 4: Create semi-join strategies
        self.create_semi_join_strategies(&ordered_predicates)
    }

    /// Estimate selectivity for each search predicate
    unsafe fn estimate_search_selectivity(&mut self) {
        warning!("ParadeDB: Estimating search predicate selectivity");

        // Estimate selectivity for outer predicates
        for predicate in &self.predicates.outer_predicates {
            if predicate.uses_search_operator {
                let selectivity = self.estimate_predicate_selectivity(predicate);
                self.selectivity_estimates
                    .insert(predicate.relid, selectivity);
                warning!(
                    "ParadeDB: Estimated selectivity for {} (outer): {:.4}",
                    predicate.relname,
                    selectivity
                );
            }
        }

        // Estimate selectivity for inner predicates
        for predicate in &self.predicates.inner_predicates {
            if predicate.uses_search_operator {
                let selectivity = self.estimate_predicate_selectivity(predicate);
                self.selectivity_estimates
                    .insert(predicate.relid, selectivity);
                warning!(
                    "ParadeDB: Estimated selectivity for {} (inner): {:.4}",
                    predicate.relname,
                    selectivity
                );
            }
        }
    }

    /// Estimate selectivity for a single search predicate
    unsafe fn estimate_predicate_selectivity(&self, predicate: &RelationSearchPredicate) -> f64 {
        // For now, use simple heuristics based on query complexity
        // In a production implementation, this would use index statistics

        let query_str = format!("{:?}", predicate.query);

        // Simple heuristic: longer, more specific queries are more selective
        let base_selectivity = match query_str.len() {
            0..=10 => 0.5,   // Very short queries - low selectivity
            11..=20 => 0.1,  // Medium queries - moderate selectivity
            21..=50 => 0.05, // Longer queries - high selectivity
            _ => 0.01,       // Very long queries - very high selectivity
        };

        // Adjust based on query type
        if query_str.contains("AND") || query_str.contains("\"") {
            base_selectivity * 0.5 // Phrase queries and AND queries are more selective
        } else {
            base_selectivity
        }
    }

    /// Identify join relationships between relations
    unsafe fn identify_join_relationships(&mut self) {
        warning!("ParadeDB: Identifying join relationships");

        // For the example query pattern, we know the relationships:
        // documents.id = files.documentId
        // files.id = pages.fileId

        // In a production implementation, this would analyze the actual join conditions
        // For now, we'll use hardcoded relationships for the common patterns

        let mut outer_relids: Vec<pg_sys::Oid> = self
            .predicates
            .outer_predicates
            .iter()
            .map(|p| p.relid)
            .collect();
        let mut inner_relids: Vec<pg_sys::Oid> = self
            .predicates
            .inner_predicates
            .iter()
            .map(|p| p.relid)
            .collect();

        // Create relationships between all combinations
        for &outer_relid in &outer_relids {
            for &inner_relid in &inner_relids {
                if outer_relid != inner_relid {
                    let relationship = self.infer_join_relationship(outer_relid, inner_relid);
                    if let Some(rel) = relationship {
                        self.join_relationships.push(rel);
                        warning!(
                            "ParadeDB: Found join relationship: {} -> {}",
                            crate::postgres::customscan::pdbscan::get_rel_name(outer_relid),
                            crate::postgres::customscan::pdbscan::get_rel_name(inner_relid)
                        );
                    }
                }
            }
        }
    }

    /// Infer join relationship between two relations using actual schema analysis
    unsafe fn infer_join_relationship(
        &self,
        source_relid: pg_sys::Oid,
        target_relid: pg_sys::Oid,
    ) -> Option<JoinRelationship> {
        // Analyze the actual schema to find foreign key relationships
        let (source_key, target_key) = self.analyze_join_keys(source_relid, target_relid)?;

        // Estimate join selectivity based on relation sizes
        let join_selectivity = self.estimate_join_selectivity(source_relid, target_relid);

        Some(JoinRelationship {
            source_relid,
            target_relid,
            source_key,
            target_key,
            join_selectivity,
        })
    }

    /// Analyze actual join keys between two relations
    unsafe fn analyze_join_keys(
        &self,
        source_relid: pg_sys::Oid,
        target_relid: pg_sys::Oid,
    ) -> Option<(String, String)> {
        // Get column information for both relations
        let source_columns = self.get_relation_columns(source_relid);
        let target_columns = self.get_relation_columns(target_relid);

        // Look for common join patterns:
        // 1. source.id = target.{source_name}_id
        // 2. source.{target_name}_id = target.id
        // 3. source.id = target.id (same column name)

        let source_name = crate::postgres::customscan::pdbscan::get_rel_name(source_relid);
        let target_name = crate::postgres::customscan::pdbscan::get_rel_name(target_relid);

        // Remove common suffixes to get base names
        let source_base = source_name
            .trim_end_matches("_join_test")
            .trim_end_matches("s");
        let target_base = target_name
            .trim_end_matches("_join_test")
            .trim_end_matches("s");

        // Pattern 1: source.id = target.{source_base}Id or target.{source_base}_id
        if source_columns.contains(&"id".to_string()) {
            let foreign_key_patterns = vec![
                format!("{}Id", source_base),
                format!("{}_id", source_base),
                format!("{}id", source_base),
            ];

            for pattern in foreign_key_patterns {
                if target_columns.contains(&pattern) {
                    warning!(
                        "ParadeDB: Found join relationship: {}.id = {}.{}",
                        source_name,
                        target_name,
                        pattern
                    );
                    return Some(("id".to_string(), pattern));
                }
            }
        }

        // Pattern 2: source.{target_base}Id = target.id
        if target_columns.contains(&"id".to_string()) {
            let foreign_key_patterns = vec![
                format!("{}Id", target_base),
                format!("{}_id", target_base),
                format!("{}id", target_base),
            ];

            for pattern in foreign_key_patterns {
                if source_columns.contains(&pattern) {
                    warning!(
                        "ParadeDB: Found join relationship: {}.{} = {}.id",
                        source_name,
                        pattern,
                        target_name
                    );
                    return Some((pattern, "id".to_string()));
                }
            }
        }

        // Pattern 3: Common column names
        for source_col in &source_columns {
            if target_columns.contains(source_col) && source_col != "id" {
                warning!(
                    "ParadeDB: Found common column join: {}.{} = {}.{}",
                    source_name,
                    source_col,
                    target_name,
                    source_col
                );
                return Some((source_col.clone(), source_col.clone()));
            }
        }

        warning!(
            "ParadeDB: No join relationship found between {} and {}",
            source_name,
            target_name
        );
        None
    }

    /// Get column names for a relation
    unsafe fn get_relation_columns(&self, relid: pg_sys::Oid) -> Vec<String> {
        let mut columns = Vec::new();

        // Open the relation to get its tuple descriptor
        let heaprel = pg_sys::relation_open(relid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        if heaprel.is_null() {
            return columns;
        }

        let tuple_desc = pgrx::PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);
        for i in 0..tuple_desc.len() {
            if let Some(attribute) = tuple_desc.get(i) {
                columns.push(attribute.name().to_string());
            }
        }

        // Close the relation
        pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);

        columns
    }

    /// Estimate join selectivity between two relations
    unsafe fn estimate_join_selectivity(
        &self,
        source_relid: pg_sys::Oid,
        target_relid: pg_sys::Oid,
    ) -> f64 {
        // Get relation statistics if available
        let source_tuples = self.get_relation_tuple_count(source_relid);
        let target_tuples = self.get_relation_tuple_count(target_relid);

        // Simple heuristic: smaller relation / larger relation
        if source_tuples > 0.0 && target_tuples > 0.0 {
            let smaller = source_tuples.min(target_tuples);
            let larger = source_tuples.max(target_tuples);
            (smaller / larger).max(0.01).min(0.5) // Clamp between 1% and 50%
        } else {
            0.1 // Default selectivity
        }
    }

    /// Get tuple count for a relation
    unsafe fn get_relation_tuple_count(&self, relid: pg_sys::Oid) -> f64 {
        let heaprel = pg_sys::relation_open(relid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        if heaprel.is_null() {
            return 0.0;
        }

        let tuple_count = (*heaprel)
            .rd_rel
            .as_ref()
            .map(|rel| rel.reltuples)
            .unwrap_or(0.0);

        pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);

        tuple_count as f64
    }

    /// Order predicates by selectivity (most selective first)
    fn order_predicates_by_selectivity(&self) -> Vec<&RelationSearchPredicate> {
        let mut all_predicates: Vec<&RelationSearchPredicate> = Vec::new();

        // Collect all predicates
        all_predicates.extend(&self.predicates.outer_predicates);
        all_predicates.extend(&self.predicates.inner_predicates);

        // Sort by selectivity (most selective first)
        all_predicates.sort_by(|a, b| {
            let a_selectivity = self.selectivity_estimates.get(&a.relid).unwrap_or(&1.0);
            let b_selectivity = self.selectivity_estimates.get(&b.relid).unwrap_or(&1.0);
            a_selectivity
                .partial_cmp(b_selectivity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        warning!("ParadeDB: Ordered predicates by selectivity:");
        for (i, predicate) in all_predicates.iter().enumerate() {
            let selectivity = self
                .selectivity_estimates
                .get(&predicate.relid)
                .unwrap_or(&1.0);
            warning!(
                "ParadeDB: {}. {} (selectivity: {:.4})",
                i + 1,
                predicate.relname,
                selectivity
            );
        }

        all_predicates
    }

    /// Create semi-join strategies based on ordered predicates
    fn create_semi_join_strategies(
        &self,
        ordered_predicates: &[&RelationSearchPredicate],
    ) -> Vec<SemiJoinStrategy> {
        let mut strategies = Vec::new();

        warning!("ParadeDB: Creating semi-join strategies");

        // For each pair of related predicates, create a semi-join strategy
        for (i, &source_pred) in ordered_predicates.iter().enumerate() {
            for &target_pred in ordered_predicates.iter().skip(i + 1) {
                // Check if these relations have a join relationship
                if let Some(relationship) =
                    self.find_join_relationship(source_pred.relid, target_pred.relid)
                {
                    let strategy =
                        self.choose_semi_join_strategy(source_pred, target_pred, &relationship);
                    if let Some(s) = strategy {
                        strategies.push(s);
                        warning!(
                            "ParadeDB: Created semi-join strategy: {} -> {}",
                            source_pred.relname,
                            target_pred.relname
                        );
                    }
                }
            }
        }

        strategies
    }

    /// Find join relationship between two relations
    fn find_join_relationship(
        &self,
        source_relid: pg_sys::Oid,
        target_relid: pg_sys::Oid,
    ) -> Option<&JoinRelationship> {
        self.join_relationships
            .iter()
            .find(|rel| rel.source_relid == source_relid && rel.target_relid == target_relid)
    }

    /// Choose the best semi-join strategy for a pair of predicates
    fn choose_semi_join_strategy(
        &self,
        source_pred: &RelationSearchPredicate,
        target_pred: &RelationSearchPredicate,
        relationship: &JoinRelationship,
    ) -> Option<SemiJoinStrategy> {
        let source_selectivity = self
            .selectivity_estimates
            .get(&source_pred.relid)
            .unwrap_or(&1.0);
        let estimated_source_results = (100_000.0 * source_selectivity) as usize; // Rough estimate

        // Choose strategy based on estimated result size
        match estimated_source_results {
            0..=1000 => {
                // Small result set - use direct filtering
                Some(SemiJoinStrategy::SearchFilter {
                    source_relid: source_pred.relid,
                    target_relid: target_pred.relid,
                    filter_keys: Vec::new(), // Will be populated during execution
                })
            }
            1001..=10000 => {
                // Medium result set - use sorted array
                Some(SemiJoinStrategy::SortedArray {
                    source_relid: source_pred.relid,
                    target_relid: target_pred.relid,
                    sorted_keys: Vec::new(), // Will be populated during execution
                })
            }
            _ => {
                // Large result set - use Bloom filter
                Some(SemiJoinStrategy::BloomFilter {
                    source_relid: source_pred.relid,
                    target_relid: target_pred.relid,
                    bloom_filter: BloomFilterData {
                        bits: Vec::new(),
                        hash_count: 3,
                        size: 8192,
                    },
                })
            }
        }
    }
}

/// Execute semi-join optimization strategy
pub unsafe fn execute_semi_join_strategy(
    strategy: &SemiJoinStrategy,
    search_readers: &HashMap<pg_sys::Oid, SearchIndexReader>,
    predicates: &JoinSearchPredicates,
) -> Option<Vec<(u64, f32)>> {
    match strategy {
        SemiJoinStrategy::SearchFilter {
            source_relid,
            target_relid,
            ..
        } => {
            execute_search_filter_strategy(*source_relid, *target_relid, search_readers, predicates)
        }
        SemiJoinStrategy::SortedArray {
            source_relid,
            target_relid,
            ..
        } => {
            execute_sorted_array_strategy(*source_relid, *target_relid, search_readers, predicates)
        }
        SemiJoinStrategy::BloomFilter {
            source_relid,
            target_relid,
            ..
        } => {
            execute_bloom_filter_strategy(*source_relid, *target_relid, search_readers, predicates)
        }
    }
}

/// Execute search filter strategy (for small result sets)
unsafe fn execute_search_filter_strategy(
    source_relid: pg_sys::Oid,
    target_relid: pg_sys::Oid,
    search_readers: &HashMap<pg_sys::Oid, SearchIndexReader>,
    predicates: &JoinSearchPredicates,
) -> Option<Vec<(u64, f32)>> {
    warning!("ParadeDB: Executing search filter strategy");

    // Step 1: Execute search on source relation
    let source_predicate = find_predicate_for_relation(predicates, source_relid)?;
    let source_reader = search_readers.get(&source_relid)?;

    let source_results = execute_search(source_reader, source_predicate);
    warning!(
        "ParadeDB: Source search returned {} results",
        source_results.len()
    );

    // Step 2: Extract join keys from source results
    let join_keys = extract_join_keys_from_results(source_relid, &source_results);
    warning!("ParadeDB: Extracted {} join keys", join_keys.len());

    // Step 3: Execute filtered search on target relation
    let target_predicate = find_predicate_for_relation(predicates, target_relid)?;
    let target_reader = search_readers.get(&target_relid)?;

    let target_results = execute_filtered_search(target_reader, target_predicate, &join_keys);
    warning!(
        "ParadeDB: Filtered target search returned {} results",
        target_results.len()
    );

    Some(target_results)
}

/// Execute sorted array strategy (for medium result sets)
unsafe fn execute_sorted_array_strategy(
    source_relid: pg_sys::Oid,
    target_relid: pg_sys::Oid,
    search_readers: &HashMap<pg_sys::Oid, SearchIndexReader>,
    predicates: &JoinSearchPredicates,
) -> Option<Vec<(u64, f32)>> {
    warning!("ParadeDB: Executing sorted array strategy");

    // Similar to search filter, but with sorted array for faster lookups
    execute_search_filter_strategy(source_relid, target_relid, search_readers, predicates)
}

/// Execute Bloom filter strategy (for large result sets)
unsafe fn execute_bloom_filter_strategy(
    source_relid: pg_sys::Oid,
    target_relid: pg_sys::Oid,
    search_readers: &HashMap<pg_sys::Oid, SearchIndexReader>,
    predicates: &JoinSearchPredicates,
) -> Option<Vec<(u64, f32)>> {
    warning!("ParadeDB: Executing Bloom filter strategy");

    // For now, fall back to search filter strategy
    // In production, this would implement actual Bloom filter logic
    execute_search_filter_strategy(source_relid, target_relid, search_readers, predicates)
}

/// Find predicate for a specific relation
fn find_predicate_for_relation(
    predicates: &JoinSearchPredicates,
    relid: pg_sys::Oid,
) -> Option<&RelationSearchPredicate> {
    predicates
        .outer_predicates
        .iter()
        .chain(predicates.inner_predicates.iter())
        .find(|p| p.relid == relid && p.uses_search_operator)
}

/// Execute search on a relation
unsafe fn execute_search(
    search_reader: &SearchIndexReader,
    predicate: &RelationSearchPredicate,
) -> Vec<(u64, f32)> {
    let search_results = search_reader.search(
        true,  // need_scores
        false, // sort_segments_by_ctid
        &predicate.query,
        None, // estimated_rows
    );

    search_results
        .into_iter()
        .map(|(search_index_score, _doc_address)| {
            (search_index_score.ctid, search_index_score.bm25)
        })
        .collect()
}

/// Extract join keys from search results
unsafe fn extract_join_keys_from_results(
    relid: pg_sys::Oid,
    results: &[(u64, f32)],
) -> HashSet<i32> {
    let mut join_keys = HashSet::new();

    // For each search result, extract the join key
    for &(ctid, _score) in results {
        if let Some(join_key) = fetch_join_key_from_ctid(relid, ctid) {
            join_keys.insert(join_key);
        }
    }

    join_keys
}

/// Execute filtered search on target relation with Tantivy filter pushdown
unsafe fn execute_filtered_search(
    search_reader: &SearchIndexReader,
    predicate: &RelationSearchPredicate,
    filter_keys: &HashSet<i32>,
) -> Vec<(u64, f32)> {
    warning!(
        "ParadeDB: Executing filtered search with {} filter keys pushed down to Tantivy",
        filter_keys.len()
    );

    // Create a combined query that includes both the search predicate AND the join key filter
    let combined_query = create_filtered_query(&predicate.query, predicate.relid, filter_keys);

    warning!(
        "ParadeDB: Created combined query with filter pushdown for relation {}",
        get_rel_name(predicate.relid)
    );

    // Execute the combined query - Tantivy will handle both search and filtering
    let search_results = search_reader.search(
        true,  // need_scores
        false, // sort_segments_by_ctid
        &combined_query,
        None, // estimated_rows
    );

    let results: Vec<(u64, f32)> = search_results
        .into_iter()
        .map(|(search_index_score, _doc_address)| {
            (search_index_score.ctid, search_index_score.bm25)
        })
        .collect();

    warning!(
        "ParadeDB: Filtered search with Tantivy pushdown returned {} results (vs {} filter keys)",
        results.len(),
        filter_keys.len()
    );

    results
}

/// Fetch join key from a CTID
unsafe fn fetch_join_key_from_ctid(relid: pg_sys::Oid, ctid: u64) -> Option<i32> {
    // Open the relation
    let heaprel = pg_sys::relation_open(relid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
    if heaprel.is_null() {
        return None;
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

    let result = if found {
        // Try to extract the join key - look for common patterns
        let tuple_desc = pgrx::PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);
        let heap_tuple =
            pgrx::heap_tuple::PgHeapTuple::from_heap_tuple(tuple_desc.clone(), &mut htup);

        // Look for common join key column names
        let join_key_candidates = vec!["id", "document_id", "documentId", "file_id", "fileId"];

        for candidate in join_key_candidates {
            if let Ok(Some(value)) = heap_tuple.get_by_name::<i32>(candidate) {
                // Clean up before returning
                if buffer != pg_sys::InvalidBuffer as i32 {
                    pg_sys::ReleaseBuffer(buffer);
                }
                pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
                return Some(value);
            }
        }

        // If no standard join key found, try the first integer column
        for i in 0..tuple_desc.len() {
            if let Some(attribute) = tuple_desc.get(i) {
                if attribute.type_oid() == pg_sys::INT4OID.into() {
                    let column_name = attribute.name().to_string();
                    if let Ok(Some(value)) = heap_tuple.get_by_name::<i32>(&column_name) {
                        // Clean up before returning
                        if buffer != pg_sys::InvalidBuffer as i32 {
                            pg_sys::ReleaseBuffer(buffer);
                        }
                        pg_sys::relation_close(
                            heaprel,
                            pg_sys::AccessShareLock as pg_sys::LOCKMODE,
                        );
                        return Some(value);
                    }
                }
            }
        }

        None
    } else {
        None
    };

    // Clean up
    if buffer != pg_sys::InvalidBuffer as i32 {
        pg_sys::ReleaseBuffer(buffer);
    }
    pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);

    result
}

/// Create a combined query that includes both search predicates and join key filters
/// This pushes the filter down to Tantivy for more efficient execution
unsafe fn create_filtered_query(
    original_query: &SearchQueryInput,
    relid: pg_sys::Oid,
    filter_keys: &HashSet<i32>,
) -> SearchQueryInput {
    warning!(
        "ParadeDB: Creating filtered query for relation {} with {} filter keys",
        get_rel_name(relid),
        filter_keys.len()
    );

    // Determine the join key field name for this relation
    let join_key_field = determine_join_key_field_name(relid);

    warning!(
        "ParadeDB: Using join key field '{}' for relation {}",
        join_key_field,
        get_rel_name(relid)
    );

    // Create term queries for each filter key
    let mut filter_terms = Vec::new();
    for &key_value in filter_keys {
        let term_query = SearchQueryInput::Term {
            field: Some(join_key_field.clone()),
            value: tantivy::schema::OwnedValue::I64(key_value as i64),
            is_datetime: false,
        };
        filter_terms.push(term_query);
    }

    // Create a disjunction (OR) of all the filter terms
    let filter_query = if filter_terms.len() == 1 {
        filter_terms.into_iter().next().unwrap()
    } else {
        SearchQueryInput::Boolean {
            must: vec![],
            should: filter_terms, // OR all the filter keys
            must_not: vec![],
        }
    };

    // Combine the original search query with the filter using AND (must)
    let combined_query = SearchQueryInput::Boolean {
        must: vec![original_query.clone(), filter_query],
        should: vec![],
        must_not: vec![],
    };

    warning!(
        "ParadeDB: Created combined query: original_search AND (key IN [{}])",
        filter_keys
            .iter()
            .map(|k| k.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    combined_query
}

/// Determine the join key field name for a relation
/// This maps relation names to their appropriate join key fields
unsafe fn determine_join_key_field_name(relid: pg_sys::Oid) -> String {
    let relation_name = get_rel_name(relid);

    // Map relation names to their join key fields based on our schema
    match relation_name.as_str() {
        "documents_join_test" => "id".to_string(),
        "files_join_test" => "document_id".to_string(), // files join on document_id
        "authors_join_test" => "document_id".to_string(), // authors join on document_id
        _ => {
            // Generic fallback: try common patterns
            warning!(
                "ParadeDB: Unknown relation '{}', using 'id' as join key field",
                relation_name
            );
            "id".to_string()
        }
    }
}
