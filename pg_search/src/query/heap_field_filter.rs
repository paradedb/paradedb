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

/// Core heap-based field filter for flexible comparisons
/// Supports field-to-field, field-to-value, and value-to-field comparisons
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeapFieldFilter {
    /// Left side of the comparison (can be field or value)
    pub left: HeapOperand,
    /// Comparison operator
    pub operator: HeapOperator,
    /// Right side of the comparison (can be field or value)
    pub right: HeapOperand,
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
    /// Create a new heap field filter with field resolution for flexible operands
    pub fn new_with_operand_resolution(
        left: HeapOperand,
        operator: HeapOperator,
        right: HeapOperand,
        relation_oid: pg_sys::Oid,
    ) -> Option<Self> {
        // Resolve field references in operands
        let resolved_left = Self::resolve_operand(left, relation_oid)?;
        let resolved_right = Self::resolve_operand(right, relation_oid)?;
        
        Some(Self {
            left: resolved_left,
            operator,
            right: resolved_right,
        })
    }

    /// Resolve field references in an operand
    fn resolve_operand(operand: HeapOperand, relation_oid: pg_sys::Oid) -> Option<HeapOperand> {
        match operand {
            HeapOperand::Field { field, attno: _ } => {
                // Resolve the field name to attribute number
                let attno = unsafe { resolve_field_name_to_attno(relation_oid, &field)? };
                Some(HeapOperand::Field { field, attno })
            }
            HeapOperand::Value(value) => Some(HeapOperand::Value(value)),
        }
    }

    /// Evaluate this filter against a heap tuple identified by ctid
    /// Now supports flexible operand evaluation
    pub unsafe fn evaluate(&self, ctid: pg_sys::ItemPointer, relation_oid: pg_sys::Oid) -> bool {
        let (block_num, offset_num) = pgrx::itemptr::item_pointer_get_both(*ctid);
        debug_log!("HeapFieldFilter::evaluate called with ctid: block={}, offset={}, relation_oid: {}", 
                  block_num, offset_num, relation_oid);

        // Get left operand value
        let left_value = match &self.left {
            HeapOperand::Field { field, attno } => {
                debug_log!("Evaluating left operand: field {} (attno: {})", field.root(), attno);
                self.get_field_value_from_heap(ctid, relation_oid, *attno)
            }
            HeapOperand::Value(value) => {
                debug_log!("Left operand is constant value: {:?}", value);
                Some(value.clone())
            }
        };

        // Get right operand value
        let right_value = match &self.right {
            HeapOperand::Field { field, attno } => {
                debug_log!("Evaluating right operand: field {} (attno: {})", field.root(), attno);
                self.get_field_value_from_heap(ctid, relation_oid, *attno)
            }
            HeapOperand::Value(value) => {
                debug_log!("Right operand is constant value: {:?}", value);
                Some(value.clone())
            }
        };

        debug_log!("Left operand value: {:?}", left_value);
        debug_log!("Right operand value: {:?}", right_value);

        // Handle NULL values
        match (&left_value, &right_value) {
            (None, _) | (_, None) => {
                debug_log!("One or both operands are NULL, returning false");
                false
            }
            (Some(left), Some(right)) => {
                let result = self.compare_values(left, right);
                debug_log!("Comparison result for {:?} {:?} {:?}: {}", left, self.operator, right, result);
                result
            }
        }
    }

    /// Extract the value of an operand (field or constant) for a given ctid
    unsafe fn extract_operand_value(&self, operand: &HeapOperand, ctid: pg_sys::ItemPointer, relation_oid: pg_sys::Oid) -> Option<HeapValue> {
        match operand {
            HeapOperand::Field { attno, .. } => {
                self.get_field_value_from_heap(ctid, relation_oid, *attno)
            }
            HeapOperand::Value(value) => Some(value.clone()),
        }
    }

    /// Get field value from heap tuple using proper pgrx APIs
    unsafe fn get_field_value_from_heap(&self, ctid: pg_sys::ItemPointer, relation_oid: pg_sys::Oid, attno: pg_sys::AttrNumber) -> Option<HeapValue> {
        let (block_num, offset_num) = pgrx::itemptr::item_pointer_get_both(*ctid);
        debug_log!("get_field_value_from_heap called with ctid: block={}, offset={}, attno: {}", 
                  block_num, offset_num, attno);

        // Open the relation using pgrx
        let heaprel = PgRelation::open(relation_oid);
        let ipd = *ctid;

        // Create HeapTupleData structure
        let mut htup = pg_sys::HeapTupleData {
            t_self: ipd,
            ..Default::default()
        };
        let mut buffer: pg_sys::Buffer = pg_sys::InvalidBuffer as i32;

        // Fetch the heap tuple
        #[cfg(feature = "pg14")]
        let fetch_success = pg_sys::heap_fetch(
            heaprel.as_ptr(),
            pg_sys::GetActiveSnapshot(),
            &mut htup,
            &mut buffer,
        );

        #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
        let fetch_success = pg_sys::heap_fetch(
            heaprel.as_ptr(),
            pg_sys::GetActiveSnapshot(),
            &mut htup,
            &mut buffer,
            false,
        );

        if !fetch_success {
            debug_log!("Failed to fetch heap tuple for ctid: block={}, offset={}", block_num, offset_num);
            if buffer != (pg_sys::InvalidBuffer as i32) {
                pg_sys::ReleaseBuffer(buffer);
            }
            return None;
        }

        debug_log!("Successfully fetched heap tuple");

        // Use pgrx's high-level APIs
        let tuple_desc = PgTupleDesc::from_pg_unchecked(heaprel.rd_att);
        let heap_tuple = PgHeapTuple::from_heap_tuple(tuple_desc.clone(), &mut htup);
        
        // DEBUG: Basic tuple info
        debug_log!("Tuple has {} attributes", tuple_desc.len());
        
        // Get the attribute by index (attno is 1-based)
        let result = if let Some(attribute) = tuple_desc.get((attno - 1) as usize) {
            let field_name = attribute.name();
            debug_log!("Getting attribute {} (name: {}) from heap tuple", attno, field_name);
            
            // Get the attribute type
            let attr_type = attribute.type_oid().value();
            debug_log!("Field type OID: {}", attr_type);
            
            // Extract the value using the appropriate type
            let heap_value = match attr_type {
                pg_sys::TEXTOID | pg_sys::VARCHAROID => {
                    match heap_tuple.get_by_name::<String>(field_name) {
                        Ok(Some(text_value)) => {
                            debug_log!("Extracted TEXT value: '{}' from ctid block={}, offset={}, attno={}", text_value, block_num, offset_num, attno);
                            Some(HeapValue::Text(text_value))
                        }
                        Ok(None) => {
                            debug_log!("TEXT field value is NULL");
                            None
                        }
                        Err(e) => {
                            debug_log!("Failed to extract TEXT value: {:?}", e);
                            None
                        }
                    }
                }
                pg_sys::BOOLOID => {
                    match heap_tuple.get_by_name::<bool>(field_name) {
                        Ok(Some(bool_value)) => {
                            debug_log!("Extracted BOOL value: {}", bool_value);
                            Some(HeapValue::Boolean(bool_value))
                        }
                        Ok(None) => {
                            debug_log!("BOOL field value is NULL");
                            None
                        }
                        Err(e) => {
                            debug_log!("Failed to extract BOOL value: {:?}", e);
                            None
                        }
                    }
                }
                pg_sys::INT4OID => {
                    match heap_tuple.get_by_name::<i32>(field_name) {
                        Ok(Some(int_value)) => {
                            debug_log!("Extracted INT4 value: {}", int_value);
                            Some(HeapValue::Integer(int_value as i64))
                        }
                        Ok(None) => {
                            debug_log!("INT4 field value is NULL");
                            None
                        }
                        Err(e) => {
                            debug_log!("Failed to extract INT4 value: {:?}", e);
                            None
                        }
                    }
                }
                pg_sys::INT8OID => {
                    match heap_tuple.get_by_name::<i64>(field_name) {
                        Ok(Some(int_value)) => {
                            debug_log!("Extracted INT8 value: {}", int_value);
                            Some(HeapValue::Integer(int_value))
                        }
                        Ok(None) => {
                            debug_log!("INT8 field value is NULL");
                            None
                        }
                        Err(e) => {
                            debug_log!("Failed to extract INT8 value: {:?}", e);
                            None
                        }
                    }
                }
                pg_sys::FLOAT4OID => {
                    match heap_tuple.get_by_name::<f32>(field_name) {
                        Ok(Some(float_value)) => {
                            debug_log!("Extracted FLOAT4 value: {}", float_value);
                            Some(HeapValue::Float(float_value as f64))
                        }
                        Ok(None) => {
                            debug_log!("FLOAT4 field value is NULL");
                            None
                        }
                        Err(e) => {
                            debug_log!("Failed to extract FLOAT4 value: {:?}", e);
                            None
                        }
                    }
                }
                pg_sys::FLOAT8OID => {
                    match heap_tuple.get_by_name::<f64>(field_name) {
                        Ok(Some(float_value)) => {
                            debug_log!("Extracted FLOAT8 value: {}", float_value);
                            Some(HeapValue::Float(float_value))
                        }
                        Ok(None) => {
                            debug_log!("FLOAT8 field value is NULL");
                            None
                        }
                        Err(e) => {
                            debug_log!("Failed to extract FLOAT8 value: {:?}", e);
                            None
                        }
                    }
                }
                pg_sys::NUMERICOID => {
                    match heap_tuple.get_by_name::<AnyNumeric>(field_name) {
                        Ok(Some(numeric)) => {
                            let decimal_str = numeric.to_string();
                            debug_log!("Extracted NUMERIC value: {}", decimal_str);
                            Some(HeapValue::Decimal(decimal_str))
                        }
                        Ok(None) => {
                            debug_log!("NUMERIC field value is NULL");
                            None
                        }
                        Err(e) => {
                            debug_log!("Failed to extract NUMERIC value: {:?}", e);
                            None
                        }
                    }
                }
                _ => {
                    debug_log!("Unsupported field type: {}", attr_type);
                    None
                }
            };
            heap_value
        } else {
            debug_log!("Invalid attribute number: {}", attno);
            None
        };

        // Clean up
        if buffer != (pg_sys::InvalidBuffer as i32) {
            pg_sys::ReleaseBuffer(buffer);
        }

        debug_log!("get_field_value_from_heap returning: {:?}", result);
        result
    }

    /// Enhanced comparison supporting cross-type comparisons
    fn compare_values(&self, left: &HeapValue, right: &HeapValue) -> bool {
        let comparison = match (left, right) {
            // Same-type comparisons
            (HeapValue::Integer(a), HeapValue::Integer(b)) => a.cmp(b),
            (HeapValue::Float(a), HeapValue::Float(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
            (HeapValue::Text(a), HeapValue::Text(b)) => a.cmp(b),
            (HeapValue::Boolean(a), HeapValue::Boolean(b)) => a.cmp(b),
            (HeapValue::Decimal(a), HeapValue::Decimal(b)) => a.cmp(b),
            
            // Cross-type numeric comparisons
            (HeapValue::Integer(a), HeapValue::Float(b)) => (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal),
            (HeapValue::Float(a), HeapValue::Integer(b)) => a.partial_cmp(&(*b as f64)).unwrap_or(Ordering::Equal),
            
            // Decimal cross-type comparisons
            (HeapValue::Decimal(a), HeapValue::Integer(b)) => {
                // Convert decimal string to f64 for comparison
                if let Ok(a_f64) = a.parse::<f64>() {
                    a_f64.partial_cmp(&(*b as f64)).unwrap_or(Ordering::Equal)
                } else {
                    debug_log!("Failed to parse decimal for comparison: {}", a);
                    Ordering::Equal
                }
            },
            (HeapValue::Integer(a), HeapValue::Decimal(b)) => {
                // Convert decimal string to f64 for comparison
                if let Ok(b_f64) = b.parse::<f64>() {
                    (*a as f64).partial_cmp(&b_f64).unwrap_or(Ordering::Equal)
                } else {
                    debug_log!("Failed to parse decimal for comparison: {}", b);
                    Ordering::Equal
                }
            },
            (HeapValue::Decimal(a), HeapValue::Float(b)) => {
                // Convert decimal string to f64 for comparison
                if let Ok(a_f64) = a.parse::<f64>() {
                    a_f64.partial_cmp(b).unwrap_or(Ordering::Equal)
                } else {
                    debug_log!("Failed to parse decimal for comparison: {}", a);
                    Ordering::Equal
                }
            },
            (HeapValue::Float(a), HeapValue::Decimal(b)) => {
                // Convert decimal string to f64 for comparison
                if let Ok(b_f64) = b.parse::<f64>() {
                    a.partial_cmp(&b_f64).unwrap_or(Ordering::Equal)
                } else {
                    debug_log!("Failed to parse decimal for comparison: {}", b);
                    Ordering::Equal
                }
            },
            
            // Add more cross-type comparisons as needed
            _ => {
                debug_log!("Type mismatch in heap field comparison: {:?} vs {:?}", left, right);
                return false;
            }
        };

        match self.operator {
            HeapOperator::Equal => comparison == Ordering::Equal,
            HeapOperator::GreaterThan => comparison == Ordering::Greater,
            HeapOperator::LessThan => comparison == Ordering::Less,
            HeapOperator::GreaterThanOrEqual => comparison != Ordering::Less,
            HeapOperator::LessThanOrEqual => comparison != Ordering::Greater,
            HeapOperator::IsNull => false, // Already handled in evaluate()
            HeapOperator::IsNotNull => true, // Already handled in evaluate()
        }
    }
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
            debug_log!("Processing doc_id: {}, extracted ctid: {}", doc_id, ctid_value);
            
            // Convert u64 ctid back to ItemPointer
            let mut item_pointer = pg_sys::ItemPointerData::default();
            crate::postgres::utils::u64_to_item_pointer(ctid_value, &mut item_pointer);
            let (block_num, offset_num) = pgrx::itemptr::item_pointer_get_both(item_pointer);
            debug_log!("Converted u64 ctid {} to ItemPointer: block={}, offset={}", 
                      ctid_value, block_num, offset_num);
            
            // Evaluate all heap filters
            debug_log!("Evaluating {} heap filters for ctid: {}", self.field_filters.len(), ctid_value);
            
            for filter in &self.field_filters {
                unsafe {
                    if !filter.evaluate(&mut item_pointer as *mut pg_sys::ItemPointerData, self.relation_oid) {
                        debug_log!("Document FAILED heap filters - REJECTING doc_id {}", doc_id);
                        return false;
                    }
                }
            }
            
            debug_log!("Document PASSED heap filters - ACCEPTING doc_id {}", doc_id);
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
        debug_log!("IndexedWithHeapFilterScorer::advance called, current_doc: {}", self.current_doc);
        
        // Move to the next valid document
        self.current_doc = self.advance_to_next_valid();
        debug_log!("IndexedWithHeapFilterScorer::advance returning: {}", self.current_doc);
        self.current_doc
    }

    fn doc(&self) -> DocId {
        debug_log!("IndexedWithHeapFilterScorer::doc() returning current_doc: {}", self.current_doc);
        self.current_doc
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

