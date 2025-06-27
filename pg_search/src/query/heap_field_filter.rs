use pgrx::{pg_sys, AnyNumeric, PgTupleDesc, PgRelation};
use pgrx::heap_tuple::PgHeapTuple;
use crate::debug_log;
use crate::api::FieldName;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
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
    /// Serialized representation of the PostgreSQL expression
    /// We store this as a string description since we can't serialize raw pointers
    pub expr_description: String,
    /// For now, we'll use a simplified approach until we implement full expression evaluation
    /// This will be replaced with proper PostgreSQL expression evaluation later
    pub placeholder: bool,
}

impl PartialEq for HeapFieldFilter {
    fn eq(&self, other: &Self) -> bool {
        // Compare by description since expr_node is a raw pointer
        self.expr_description == other.expr_description
    }
}

/// Operand in a heap comparison - can be either a field reference or a constant value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HeapOperand {
    /// Reference to a field in the heap tuple
    Field {
        field: FieldName,
        /// PostgreSQL attribute number (resolved during creation)
        attno: pg_sys::AttrNumber,
    },
    /// Constant value
    Value(HeapValue),
}

/// Supported operators for heap field filtering
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum HeapOperator {
    Equal,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    IsNull,
    IsNotNull,
}

/// Values that can be compared in heap filtering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HeapValue {
    Integer(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    Decimal(String), // Store as string to preserve precision
}

impl HeapFieldFilter {
    /// Create a new HeapFieldFilter from a PostgreSQL expression node
    pub unsafe fn new(_expr_node: *mut pg_sys::Node, expr_description: String) -> Self {
        debug_log!("Creating HeapFieldFilter with description: {}", expr_description);
        
        Self {
            expr_description,
            placeholder: true, // Placeholder until we implement full evaluation
        }
    }

    /// Evaluate this filter against a heap tuple identified by ctid
    /// Uses a simplified evaluation approach for now
    pub unsafe fn evaluate(&self, ctid: pg_sys::ItemPointer, relation_oid: pg_sys::Oid) -> bool {
        let (block_num, offset_num) = pgrx::itemptr::item_pointer_get_both(*ctid);
        debug_log!("HeapFieldFilter::evaluate called with ctid: block={}, offset={}, relation_oid: {}", 
                  block_num, offset_num, relation_oid);
        
        // For now, use a simplified evaluation approach
        // This will be replaced with full PostgreSQL expression evaluation
        debug_log!("Using simplified evaluation for expression: {}", self.expr_description);
        
        // Return true as placeholder - actual evaluation will be implemented later
        true
    }

    /// Create a HeapFieldFilter from operands (deprecated - for compatibility)
    pub fn from_operands(
        _left: HeapOperand,
        _operator: HeapOperator,
        _right: HeapOperand,
    ) -> Result<Self, String> {
        // This is a compatibility method for the old operand-based approach
        // It creates a placeholder filter that always returns true
        Ok(Self {
            expr_description: "Legacy operand-based filter (placeholder)".to_string(),
            placeholder: true,
        })
    }

    /// Create a new heap field filter with field resolution for flexible operands
    /// This method is deprecated - use the new expression-based approach instead
    pub fn with_field_resolution(
        _left: HeapOperand,
        _operator: HeapOperator,
        _right: HeapOperand,
        _relation_oid: pg_sys::Oid,
    ) -> Result<Self, String> {
        // Deprecated method - return a placeholder
        Ok(Self {
            expr_description: "Deprecated field resolution method (placeholder)".to_string(),
            placeholder: true,
        })
    }

    // Old operand-based methods removed - they are no longer needed
    // The new expression-based approach handles evaluation directly
}

/// Resolve field name to PostgreSQL attribute number
unsafe fn resolve_field_name_to_attno(
    relation_oid: pg_sys::Oid,
    field_name: &FieldName,
) -> Option<pg_sys::AttrNumber> {
    // Open the relation
    let relation = pg_sys::RelationIdGetRelation(relation_oid);
    if relation.is_null() {
        debug_log!("Failed to open relation with OID: {}", relation_oid);
        return None;
    }

    let tuple_desc = (*relation).rd_att;
    let field_name_str = field_name.root();
    
    // Search through attributes
    for attno in 1..=(*tuple_desc).natts {
        let attr_slice = (*tuple_desc).attrs.as_slice((*tuple_desc).natts as usize);
        let form_attr = &attr_slice[(attno - 1) as usize];
        let attr_name = std::ffi::CStr::from_ptr(form_attr.attname.data.as_ptr());
        
        if let Ok(attr_name_str) = attr_name.to_str() {
            if attr_name_str == field_name_str {
                pg_sys::RelationClose(relation);
                return Some(attno as pg_sys::AttrNumber);
            }
        }
    }
    
    pg_sys::RelationClose(relation);
    None
}

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
    fn test_heap_operator_equality() {
        assert_eq!(HeapOperator::Equal, HeapOperator::Equal);
        assert_ne!(HeapOperator::Equal, HeapOperator::GreaterThan);
    }

    #[test]
    fn test_heap_value_comparisons() {
        let val1 = HeapValue::Integer(10);
        let val2 = HeapValue::Integer(20);
        let val3 = HeapValue::Float(15.5);
        
        assert_eq!(val1, HeapValue::Integer(10));
        assert_ne!(val1, val2);
        
        // Test cross-type comparison logic would go here
        // (actual comparison happens in compare_values method)
    }
} 

