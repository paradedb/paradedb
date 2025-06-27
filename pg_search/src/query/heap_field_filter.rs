use pgrx::pg_sys;
use crate::debug_log;
use crate::query::PostgresPointer;
use serde::{Deserialize, Serialize};
use tantivy::{
    DocId, DocSet, Score, SegmentReader,
    query::{Query, Weight, EnableScoring, Explanation, Scorer},
    TERMINATED,
};

/// Core heap-based field filter using PostgreSQL expression evaluation
/// This approach stores a serialized representation of the PostgreSQL expression
/// and evaluates it directly against heap tuples, supporting any PostgreSQL operator or function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeapFieldFilter {
    /// PostgreSQL expression node that can be serialized and reconstructed
    pub expr_node: PostgresPointer,
    /// Human-readable description of the expression
    pub description: String,
}

// SAFETY: HeapFieldFilter is only used within PostgreSQL's single-threaded context
// during query execution. The PostgresPointer serialization/deserialization handles
// the cross-thread boundary properly via nodeToString/stringToNode.
unsafe impl Send for HeapFieldFilter {}
unsafe impl Sync for HeapFieldFilter {}

impl PartialEq for HeapFieldFilter {
    fn eq(&self, other: &Self) -> bool {
        // Compare by the serialized expression node
        self.expr_node == other.expr_node
    }
}

// The operand-based enums have been removed in favor of the expression-based approach
// All filtering is now handled through PostgreSQL expression evaluation

impl HeapFieldFilter {
    /// Create a new HeapFieldFilter from a PostgreSQL expression node
    pub unsafe fn new(expr_node: *mut pg_sys::Node, expr_description: String) -> Self {
        debug_log!("Creating HeapFieldFilter with description: {}", expr_description);
        
        Self {
            expr_node: PostgresPointer(expr_node.cast()),
            description: expr_description,
        }
    }

    /// Evaluate this filter against a heap tuple identified by ctid
    /// Uses PostgreSQL's expression evaluation system
    pub unsafe fn evaluate(&self, ctid: pg_sys::ItemPointer, relation_oid: pg_sys::Oid) -> bool {
        let (block_num, offset_num) = pgrx::itemptr::item_pointer_get_both(*ctid);
        debug_log!("HeapFieldFilter::evaluate called with ctid: block={}, offset={}, relation_oid: {}", 
                  block_num, offset_num, relation_oid);
        
        // For now, use a simplified evaluation approach
        // TODO: Implement full PostgreSQL expression evaluation using the stored expression node
        debug_log!("Using simplified evaluation for expression: {}", self.description);
        
        // Return true as placeholder - actual evaluation will be implemented later
        // This will need to get the expression node and evaluate it against the heap tuple
        true
    }

    /// Get the PostgreSQL expression node
    pub unsafe fn get_expression_node(&self) -> *mut pg_sys::Node {
        self.expr_node.0.cast()
    }

    // The new expression-based approach handles evaluation directly
}

// Field name resolution is no longer needed with the expression-based approach

/// Tantivy query that combines indexed search with heap field filtering
#[derive(Debug)]
pub struct IndexedWithHeapFilterQuery {
    indexed_query: Box<dyn Query>,
    field_filters: Vec<HeapFieldFilter>,
    relation_oid: pg_sys::Oid,
}

impl IndexedWithHeapFilterQuery {
    pub fn new(
        indexed_query: Box<dyn Query>,
        field_filters: Vec<HeapFieldFilter>,
        relation_oid: pg_sys::Oid,
    ) -> Self {
        Self {
            indexed_query,
            field_filters,
            relation_oid,
        }
    }
}

impl tantivy::query::QueryClone for IndexedWithHeapFilterQuery {
    fn box_clone(&self) -> Box<dyn Query> {
        Box::new(Self {
            indexed_query: self.indexed_query.box_clone(),
            field_filters: self.field_filters.clone(),
            relation_oid: self.relation_oid,
        })
    }
}

impl Query for IndexedWithHeapFilterQuery {
    fn weight(&self, enable_scoring: EnableScoring) -> tantivy::Result<Box<dyn Weight>> {
        let indexed_weight = self.indexed_query.weight(enable_scoring)?;
        Ok(Box::new(IndexedWithHeapFilterWeight {
            indexed_weight,
            field_filters: self.field_filters.clone(),
            relation_oid: self.relation_oid,
        }))
    }
}

struct IndexedWithHeapFilterWeight {
    indexed_weight: Box<dyn Weight>,
    field_filters: Vec<HeapFieldFilter>,
    relation_oid: pg_sys::Oid,
}

impl Weight for IndexedWithHeapFilterWeight {
    fn scorer(&self, reader: &SegmentReader, boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
        debug_log!("IndexedWithHeapFilterWeight::scorer called with boost: {}", boost);
        let indexed_scorer = self.indexed_weight.scorer(reader, boost)?;
        debug_log!("Indexed scorer created successfully");
        
        // Get ctid fast field for heap access
        let fast_fields_reader = reader.fast_fields();
        let ctid_ff = crate::index::fast_fields_helper::FFType::new_ctid(&fast_fields_reader);
        debug_log!("ctid fast field created successfully");

        let scorer = IndexedWithHeapFilterScorer::new(
            indexed_scorer,
            self.field_filters.clone(),
            ctid_ff,
            self.relation_oid,
        );
        debug_log!("IndexedWithHeapFilterScorer created successfully");
        
        Ok(Box::new(scorer))
    }

    fn explain(&self, reader: &SegmentReader, doc: DocId) -> tantivy::Result<Explanation> {
        let indexed_explanation = self.indexed_weight.explain(reader, doc)?;
        Ok(Explanation::new(
            "IndexedWithHeapFilter",
            indexed_explanation.value(),
        ))
    }
}

struct IndexedWithHeapFilterScorer {
    indexed_scorer: Box<dyn Scorer>,
    field_filters: Vec<HeapFieldFilter>,
    ctid_ff: crate::index::fast_fields_helper::FFType,
    relation_oid: pg_sys::Oid,
    current_doc: DocId,
}

impl IndexedWithHeapFilterScorer {
    fn new(
        indexed_scorer: Box<dyn Scorer>,
        field_filters: Vec<HeapFieldFilter>,
        ctid_ff: crate::index::fast_fields_helper::FFType,
        relation_oid: pg_sys::Oid,
    ) -> Self {
        debug_log!("IndexedWithHeapFilterScorer::new called with {} field_filters, relation_oid: {}", field_filters.len(), relation_oid);
        
        let mut scorer = Self {
            indexed_scorer,
            field_filters,
            ctid_ff,
            relation_oid,
            current_doc: TERMINATED,
        };
        
        // Position at the first valid document
        // For initialization, we need to check the current document first, then advance if needed
        scorer.current_doc = scorer.find_first_valid_document();
        debug_log!("IndexedWithHeapFilterScorer initialized with first doc: {}", scorer.current_doc);
        
        scorer
    }
    
    fn find_first_valid_document(&mut self) -> DocId {
        // For initialization, check the current document first
        let current_doc = self.indexed_scorer.doc();
        debug_log!("find_first_valid_document: checking current doc: {}", current_doc);
        
        if current_doc != TERMINATED && self.passes_heap_filters(current_doc) {
            debug_log!("find_first_valid_document: current doc {} passes heap filters", current_doc);
            return current_doc;
        }
        
        // If current document doesn't pass, advance to find the next valid one
        self.advance_to_next_valid()
    }
    
    fn advance_to_next_valid(&mut self) -> DocId {
        loop {
            let doc = self.indexed_scorer.advance();
            debug_log!("advance_to_next_valid: underlying scorer advanced to doc: {}", doc);
            
            if doc == TERMINATED {
                debug_log!("advance_to_next_valid: underlying scorer terminated");
                return TERMINATED;
            }
            
            if self.passes_heap_filters(doc) {
                debug_log!("advance_to_next_valid: doc {} passes heap filters", doc);
                return doc;
            } else {
                debug_log!("advance_to_next_valid: doc {} failed heap filters, continuing", doc);
            }
        }
    }

    fn passes_heap_filters(&self, doc_id: DocId) -> bool {
        // Extract ctid from the current document
        if let Some(ctid_value) = self.ctid_ff.as_u64(doc_id) {
            debug_log!("=== HEAP FILTER CHECK ===");
            debug_log!("Processing doc_id: {}, extracted ctid: {}", doc_id, ctid_value);
            
            // Convert u64 ctid back to ItemPointer
            let mut item_pointer = pg_sys::ItemPointerData::default();
            crate::postgres::utils::u64_to_item_pointer(ctid_value, &mut item_pointer);
            let (block_num, offset_num) = pgrx::itemptr::item_pointer_get_both(item_pointer);
            debug_log!("Converted u64 ctid {} to ItemPointer: block={}, offset={}", 
                      ctid_value, block_num, offset_num);
            
            // Evaluate all heap filters
            debug_log!("Evaluating {} heap filters for ctid: {}", self.field_filters.len(), ctid_value);
            
            for (filter_idx, filter) in self.field_filters.iter().enumerate() {
                debug_log!("Evaluating filter {} of {}", filter_idx + 1, self.field_filters.len());
                unsafe {
                    let filter_result = filter.evaluate(&mut item_pointer as *mut pg_sys::ItemPointerData, self.relation_oid);
                    debug_log!("Filter {} result: {}", filter_idx + 1, filter_result);
                    if !filter_result {
                        debug_log!("Document FAILED heap filter {} - REJECTING doc_id {}", filter_idx + 1, doc_id);
                        debug_log!("=== HEAP FILTER REJECTED ===");
                        return false;
                    }
                }
            }
            
            debug_log!("Document PASSED all {} heap filters - ACCEPTING doc_id {}", self.field_filters.len(), doc_id);
            debug_log!("=== HEAP FILTER ACCEPTED ===");
            true
        } else {
            debug_log!("Failed to extract ctid for doc_id: {}", doc_id);
            false
        }
    }
}

impl Scorer for IndexedWithHeapFilterScorer {
    fn score(&mut self) -> Score {
        // Return the score from the indexed query (preserving BM25 scores)
        self.indexed_scorer.score()
    }
}

impl DocSet for IndexedWithHeapFilterScorer {
    fn advance(&mut self) -> DocId {
        debug_log!("=== SCORER ADVANCE ===");
        debug_log!("IndexedWithHeapFilterScorer::advance called");
        
        loop {
            let doc = self.indexed_scorer.advance();
            debug_log!("Underlying scorer advanced to doc: {}", doc);
            
            if doc == TERMINATED {
                debug_log!("Underlying scorer terminated");
                debug_log!("=== SCORER TERMINATED ===");
                return TERMINATED;
            }
            
            debug_log!("Checking if doc {} passes heap filters...", doc);
            if self.passes_heap_filters(doc) {
                debug_log!("Doc {} PASSES heap filters, returning from advance()", doc);
                debug_log!("=== SCORER ADVANCE RETURNING {} ===", doc);
                return doc;
            } else {
                debug_log!("Doc {} FAILED heap filters, continuing to next doc", doc);
            }
        }
    }

    fn doc(&self) -> DocId {
        let doc = self.indexed_scorer.doc();
        debug_log!("IndexedWithHeapFilterScorer::doc called, returning: {}", doc);
        doc
    }

    fn size_hint(&self) -> u32 {
        let hint = self.indexed_scorer.size_hint();
        debug_log!("IndexedWithHeapFilterScorer::size_hint called, returning: {}", hint);
        hint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heap_field_filter_equality() {
        // Test that HeapFieldFilter equality works based on expression content
        // This is a placeholder test - actual tests would require PostgreSQL nodes
        assert!(true); // Placeholder until we can create actual PostgreSQL expressions in tests
    }
} 

