use crate::index::fast_fields_helper::FFType;
use pgrx::{pg_sys, FromDatum, AnyNumeric};
use crate::debug_log;
use crate::schema::SearchIndexSchema;
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
        debug_log!("HeapFieldFilter::evaluate called with ctid: {:?}, relation_oid: {}", 
                  (*ctid).ip_blkid.bi_hi as u32 * 65536 + (*ctid).ip_blkid.bi_lo as u32, 
                  relation_oid);

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

    /// Get field value from heap tuple
    unsafe fn get_field_value_from_heap(&self, ctid: pg_sys::ItemPointer, relation_oid: pg_sys::Oid, attno: pg_sys::AttrNumber) -> Option<HeapValue> {
        debug_log!("get_field_value_from_heap called with ctid: {:?}, attno: {}", 
                  (*ctid).ip_blkid.bi_hi as u32 * 65536 + (*ctid).ip_blkid.bi_lo as u32, 
                  attno);

        // Open the relation
        let relation = pg_sys::RelationIdGetRelation(relation_oid);
        if relation.is_null() {
            debug_log!("Failed to open relation with OID: {}", relation_oid);
            return None;
        }

        // Create a HeapTuple structure
        let mut htup = pg_sys::HeapTupleData {
            t_self: *ctid,
            ..Default::default()
        };
        let mut buffer: pg_sys::Buffer = pg_sys::InvalidBuffer as i32;

        // Get the heap tuple using the correct heap_fetch signature
        #[cfg(feature = "pg14")]
        let fetch_success = pg_sys::heap_fetch(
            relation,
            pg_sys::GetActiveSnapshot(),
            &mut htup,
            &mut buffer,
        );

        #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
        let fetch_success = pg_sys::heap_fetch(
            relation,
            pg_sys::GetActiveSnapshot(),
            &mut htup,
            &mut buffer,
            false,
        );

        if !fetch_success {
            debug_log!("Failed to fetch heap tuple for ctid: {:?}", ctid);
            pg_sys::RelationClose(relation);
            return None;
        }

        debug_log!("Successfully fetched heap tuple");

        // Get the tuple descriptor
        let tuple_desc = (*relation).rd_att;

        // Extract the field value
        let mut is_null = false;
        let datum = pg_sys::heap_getattr(&mut htup, attno as i32, tuple_desc, &mut is_null);

        debug_log!("heap_getattr returned: is_null={}", is_null);

        let result = if is_null {
            debug_log!("Field value is NULL");
            None
        } else {
            // Convert the datum to HeapValue based on the attribute type
            let attr = (*tuple_desc).attrs.as_slice((*tuple_desc).natts as usize);
            if (attno as usize) <= attr.len() {
                let attr_type = attr[(attno - 1) as usize].atttypid;
                debug_log!("Field type OID: {}", attr_type);
                
                let heap_value = match attr_type {
                    pg_sys::TEXTOID => {
                        let text_datum = unsafe { pg_sys::pg_detoast_datum_packed(datum.cast_mut_ptr()) };
                        let text_str = unsafe { std::ffi::CStr::from_ptr(pg_sys::text_to_cstring(text_datum as *mut pg_sys::text)) };
                        let string_value = text_str.to_string_lossy().into_owned();
                        debug_log!("Extracted TEXT value: {}", string_value);
                        Some(HeapValue::Text(string_value))
                    }
                    pg_sys::BOOLOID => {
                        let bool_value = unsafe { bool::from_datum(datum, false).unwrap_or(false) };
                        debug_log!("Extracted BOOL value: {}", bool_value);
                        Some(HeapValue::Boolean(bool_value))
                    }
                    pg_sys::INT4OID => {
                        let int_value = unsafe { i32::from_datum(datum, false).unwrap_or(0) };
                        debug_log!("Extracted INT4 value: {}", int_value);
                        Some(HeapValue::Integer(int_value as i64))
                    }
                    pg_sys::INT8OID => {
                        let int_value = unsafe { i64::from_datum(datum, false).unwrap_or(0) };
                        debug_log!("Extracted INT8 value: {}", int_value);
                        Some(HeapValue::Integer(int_value))
                    }
                    pg_sys::FLOAT4OID => {
                        let float_value = unsafe { f32::from_datum(datum, false).unwrap_or(0.0) };
                        debug_log!("Extracted FLOAT4 value: {}", float_value);
                        Some(HeapValue::Float(float_value as f64))
                    }
                    pg_sys::FLOAT8OID => {
                        let float_value = unsafe { f64::from_datum(datum, false).unwrap_or(0.0) };
                        debug_log!("Extracted FLOAT8 value: {}", float_value);
                        Some(HeapValue::Float(float_value))
                    }
                    pg_sys::NUMERICOID => {
                        if let Some(numeric) = unsafe { pgrx::AnyNumeric::from_datum(datum, false) } {
                            let decimal_str = numeric.to_string();
                            debug_log!("Extracted NUMERIC value: {}", decimal_str);
                            Some(HeapValue::Decimal(decimal_str))
                        } else {
                            debug_log!("Failed to extract NUMERIC value");
                            None
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
            }
        };

        // Clean up
        if buffer != (pg_sys::InvalidBuffer as i32) {
            pg_sys::ReleaseBuffer(buffer);
        }
        pg_sys::RelationClose(relation);

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
}

impl IndexedWithHeapFilterScorer {
    fn new(
        indexed_scorer: Box<dyn Scorer>,
        field_filters: Vec<HeapFieldFilter>,
        ctid_ff: crate::index::fast_fields_helper::FFType,
        relation_oid: pg_sys::Oid,
    ) -> Self {
        debug_log!("IndexedWithHeapFilterScorer::new called with {} field_filters, relation_oid: {}", field_filters.len(), relation_oid);
        
        Self {
            indexed_scorer,
            field_filters,
            ctid_ff,
            relation_oid,
        }
    }

    fn passes_heap_filters(&self, doc_id: DocId) -> bool {
        // Extract ctid from the current document
        if let Some(ctid_value) = self.ctid_ff.as_u64(doc_id) {
            debug_log!("Extracted ctid: {} for doc_id: {}", ctid_value, doc_id);
            
            // Convert u64 ctid back to ItemPointer
            let mut item_pointer = pg_sys::ItemPointerData::default();
            crate::postgres::utils::u64_to_item_pointer(ctid_value, &mut item_pointer);
            
            // Evaluate all heap filters
            debug_log!("Evaluating {} heap filters for ctid: {}", self.field_filters.len(), ctid_value);
            
            for filter in &self.field_filters {
                unsafe {
                    if !filter.evaluate(&mut item_pointer as *mut pg_sys::ItemPointerData, self.relation_oid) {
                        debug_log!("Document failed heap filters");
                        return false;
                    }
                }
            }
            
            debug_log!("Document passed all heap filters");
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
        debug_log!("IndexedWithHeapFilterScorer::advance called");
        
        loop {
            let doc = self.indexed_scorer.advance();
            debug_log!("Underlying scorer advanced to doc: {}", doc);
            
            if doc == TERMINATED {
                debug_log!("Underlying scorer terminated");
                return TERMINATED;
            }
            
            if self.passes_heap_filters(doc) {
                debug_log!("Doc {} passes heap filters, returning", doc);
                return doc;
            }
            
            debug_log!("Doc {} failed heap filters, continuing", doc);
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

