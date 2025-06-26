use crate::api::FieldName;
use pgrx::{pg_sys, FromDatum, AnyNumeric};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use tantivy::{
    DocId, DocSet, Score, SegmentReader,
    query::{Query, Weight, EnableScoring, Explanation, Scorer},
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
    pub fn evaluate(&self, ctid: u64, relation_oid: pg_sys::Oid) -> bool {
        // Extract values for both operands
        let left_value = self.extract_operand_value(&self.left, ctid, relation_oid);
        let right_value = self.extract_operand_value(&self.right, ctid, relation_oid);

        // Store the references for later use
        let left_is_none = left_value.is_none();
        let right_is_none = right_value.is_none();

        // Handle null values and perform comparison
        match (left_value, right_value) {
            (Some(left), Some(right)) => self.compare_values(&left, &right),
            (None, None) => {
                // Both null - only equal for equality checks
                matches!(self.operator, HeapOperator::Equal)
            }
            (None, Some(_)) | (Some(_), None) => {
                // One null, one not null
                match self.operator {
                    HeapOperator::IsNull => left_is_none || right_is_none,
                    HeapOperator::IsNotNull => !left_is_none && !right_is_none,
                    _ => false, // NULL doesn't match other comparisons
                }
            }
        }
    }

    /// Extract value from an operand (field reference or constant)
    fn extract_operand_value(
        &self,
        operand: &HeapOperand,
        ctid: u64,
        relation_oid: pg_sys::Oid,
    ) -> Option<HeapValue> {
        match operand {
            HeapOperand::Field { field: _, attno } => {
                // Extract field value from heap tuple
                unsafe { self.extract_field_value_by_attno(*attno, ctid, relation_oid) }
            }
            HeapOperand::Value(value) => Some(value.clone()),
        }
    }

    /// Extract field value by attribute number
    unsafe fn extract_field_value_by_attno(
        &self,
        attno: pg_sys::AttrNumber,
        ctid: u64,
        relation_oid: pg_sys::Oid,
    ) -> Option<HeapValue> {
        // Open relation and get heap tuple
        let relation = pg_sys::RelationIdGetRelation(relation_oid);
        if relation.is_null() {
            return None;
        }

        let mut ipd = pg_sys::ItemPointerData::default();
        crate::postgres::utils::u64_to_item_pointer(ctid, &mut ipd);

        let mut htup = pg_sys::HeapTupleData {
            t_self: ipd,
            ..Default::default()
        };
        let mut buffer: pg_sys::Buffer = pg_sys::InvalidBuffer as i32;

        // Use the appropriate heap_fetch based on PostgreSQL version
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
            pg_sys::RelationClose(relation);
            return None;
        }

        // Extract attribute value
        let tuple_desc = (*relation).rd_att;
        let mut is_null = false;
        let datum = pg_sys::heap_getattr(&mut htup, attno as i32, tuple_desc, &mut is_null);
        
        let result = if is_null {
            None
        } else {
            // Get the attribute type OID
            let attr_slice = (*tuple_desc).attrs.as_slice((*tuple_desc).natts as usize);
            let attr_type_oid = attr_slice[(attno - 1) as usize].atttypid;
            self.datum_to_heap_value(datum, attr_type_oid)
        };

        // Cleanup
        if buffer != pg_sys::InvalidBuffer as i32 {
            pg_sys::ReleaseBuffer(buffer);
        }
        pg_sys::RelationClose(relation);

        result
    }

    /// Convert PostgreSQL Datum to HeapValue based on type OID
    unsafe fn datum_to_heap_value(&self, datum: pg_sys::Datum, type_oid: pg_sys::Oid) -> Option<HeapValue> {
        match type_oid {
            pg_sys::TEXTOID | pg_sys::VARCHAROID => {
                String::from_datum(datum, false).map(|s| HeapValue::Text(s))
            }
            pg_sys::INT4OID => {
                i32::from_datum(datum, false).map(|i| HeapValue::Integer(i as i64))
            }
            pg_sys::INT8OID => {
                i64::from_datum(datum, false).map(|i| HeapValue::Integer(i))
            }
            pg_sys::FLOAT4OID => {
                f32::from_datum(datum, false).map(|f| HeapValue::Float(f as f64))
            }
            pg_sys::FLOAT8OID => {
                f64::from_datum(datum, false).map(|f| HeapValue::Float(f))
            }
            pg_sys::BOOLOID => {
                bool::from_datum(datum, false).map(|b| HeapValue::Boolean(b))
            }
            pg_sys::NUMERICOID => {
                // Handle DECIMAL/NUMERIC types using AnyNumeric
                AnyNumeric::from_datum(datum, false)
                    .map(|numeric| HeapValue::Decimal(numeric.to_string()))
            }
            _ => {
                pgrx::warning!("Unsupported type OID for heap filtering: {}", type_oid);
                None
            }
        }
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
            
            // Add more cross-type comparisons as needed
            _ => {
                pgrx::warning!("Type mismatch in heap field comparison: {:?} vs {:?}", left, right);
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
        pgrx::warning!("Failed to open relation with OID: {}", relation_oid);
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
        let indexed_scorer = self.indexed_weight.scorer(reader, boost)?;
        
        // Get ctid fast field for heap access
        let fast_fields_reader = reader.fast_fields();
        let ctid_ff = crate::index::fast_fields_helper::FFType::new_ctid(&fast_fields_reader);

        Ok(Box::new(IndexedWithHeapFilterScorer::new(
            indexed_scorer,
            self.field_filters.clone(),
            ctid_ff,
            self.relation_oid,
        )))
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
        Self {
            indexed_scorer,
            field_filters,
            ctid_ff,
            relation_oid,
            current_doc: tantivy::TERMINATED,
        }
    }

    fn advance_to_next_valid(&mut self) -> DocId {
        loop {
            let doc_id = self.indexed_scorer.advance();
            if doc_id == tantivy::TERMINATED {
                self.current_doc = tantivy::TERMINATED;
                return tantivy::TERMINATED;
            }

            // Extract ctid for this document
            let ctid = match self.extract_ctid(doc_id) {
                Some(ctid) => ctid,
                None => continue, // Skip documents without valid ctid
            };

            // Check if document passes all heap field filters
            if self.evaluate_heap_filters(ctid) {
                self.current_doc = doc_id;
                return doc_id;
            }
        }
    }

    fn extract_ctid(&self, doc_id: DocId) -> Option<u64> {
        match &self.ctid_ff {
            crate::index::fast_fields_helper::FFType::U64(ff) => ff.first(doc_id),
            _ => None,
        }
    }

    fn evaluate_heap_filters(&self, ctid: u64) -> bool {
        // All heap filters must pass (AND logic)
        for filter in &self.field_filters {
            if !filter.evaluate(ctid, self.relation_oid) {
                return false;
            }
        }
        true
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
        self.advance_to_next_valid()
    }

    fn doc(&self) -> DocId {
        self.current_doc
    }

    fn size_hint(&self) -> u32 {
        self.indexed_scorer.size_hint()
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
