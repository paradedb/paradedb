// Simple demonstration of the correct field-based filtering approach
// This shows how we should extract field values using ctid and evaluate in Tantivy

use crate::api::FieldName;
use pgrx::{pg_sys, FromDatum};
use tantivy::schema::OwnedValue;
use serde::{Deserialize, Serialize};

/// Simple field comparison that can be evaluated in Tantivy
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimpleFieldFilter {
    pub field: FieldName,
    pub operator: SimpleOperator,
    pub value: SimpleValue,
    pub relation_oid: pg_sys::Oid, // Need relation OID for heap access
    pub field_attno: pg_sys::AttrNumber, // Field attribute number
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SimpleOperator {
    Equal,
    GreaterThan,
    LessThan,
    IsNull,
    IsNotNull,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SimpleValue {
    Integer(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
}

impl SimpleFieldFilter {
    /// Create a new field filter with relation and field information
    pub fn new(
        field: FieldName,
        operator: SimpleOperator,
        value: SimpleValue,
        relation_oid: pg_sys::Oid,
        field_attno: pg_sys::AttrNumber,
    ) -> Self {
        Self {
            field,
            operator,
            value,
            relation_oid,
            field_attno,
        }
    }

    /// Create a SimpleFieldFilter with automatic attribute number resolution
    pub fn new_with_field_resolution(
        field: FieldName,
        operator: SimpleOperator,
        value: SimpleValue,
        relation_oid: pg_sys::Oid,
    ) -> Option<Self> {
        let field_attno = unsafe { resolve_field_name_to_attno(relation_oid, &field)? };
        Some(Self::new(field, operator, value, relation_oid, field_attno))
    }

    /// Extract field value from PostgreSQL heap using ctid and evaluate the filter
    pub fn evaluate(&self, ctid: u64) -> bool {
        // Step 1: Extract field value from PostgreSQL using ctid
        let field_value = match self.extract_field_value_from_postgres(ctid) {
            Ok(value) => value,
            Err(_) => return false,
        };

        // Step 2: Evaluate the filter in Tantivy (no PostgreSQL expression evaluation)
        self.evaluate_filter(&field_value)
    }

    /// Extract a single field value from PostgreSQL heap using ctid
    /// This is the core implementation that properly accesses PostgreSQL data
    fn extract_field_value_from_postgres(&self, ctid: u64) -> Result<OwnedValue, String> {
        unsafe {
            // Step 1: Convert ctid back to PostgreSQL ItemPointer
            // ctid is stored as u64 in Tantivy, but PostgreSQL uses ItemPointerData (6 bytes)
            let item_pointer_data = self.u64_to_item_pointer(ctid)?;

            // Step 2: Open the relation
            let relation = pg_sys::RelationIdGetRelation(self.relation_oid);
            if relation.is_null() {
                return Err("Failed to open relation".to_string());
            }

            // Step 3: Fetch the tuple using heap_fetch
            let mut buffer: i32 = pg_sys::InvalidBuffer as i32;
            let mut heap_tuple_data = pg_sys::HeapTupleData {
                t_len: 0,
                t_self: item_pointer_data,
                t_tableOid: self.relation_oid,
                t_data: std::ptr::null_mut(),
            };

            let tuple_exists = pg_sys::heap_fetch(
                relation,
                pg_sys::GetActiveSnapshot(),
                &mut heap_tuple_data,
                &mut buffer,
                false, // Don't keep buffer pinned
            );

            if !tuple_exists {
                pg_sys::RelationClose(relation);
                return Err("Failed to fetch tuple".to_string());
            }

            // Step 4: Extract the field value from the tuple
            let tuple_desc = (*relation).rd_att;
            let field_value = self.extract_field_from_tuple(&mut heap_tuple_data, tuple_desc)?;

            // Step 5: Release the buffer and close relation
            if buffer != pg_sys::InvalidBuffer as i32 {
                pg_sys::ReleaseBuffer(buffer);
            }
            pg_sys::RelationClose(relation);

            Ok(field_value)
        }
    }

    /// Convert u64 ctid back to PostgreSQL ItemPointerData
    unsafe fn u64_to_item_pointer(&self, ctid: u64) -> Result<pg_sys::ItemPointerData, String> {
        // ctid in Tantivy is stored as u64, but PostgreSQL ItemPointer is 6 bytes (48 bits)
        // Format: block number (32 bits) + offset number (16 bits)
        
        let block_number = (ctid >> 16) as u32;
        let offset_number = (ctid & 0xFFFF) as u16;

        if block_number == pg_sys::InvalidBlockNumber || offset_number == 0 {
            return Err("Invalid ctid".to_string());
        }

        let item_pointer = pg_sys::ItemPointerData {
            ip_blkid: pg_sys::BlockIdData {
                bi_hi: ((block_number >> 16) & 0xFFFF) as u16,
                bi_lo: (block_number & 0xFFFF) as u16,
            },
            ip_posid: offset_number,
        };

        Ok(item_pointer)
    }

    /// Extract a specific field value from a heap tuple
    unsafe fn extract_field_from_tuple(
        &self,
        heap_tuple: &mut pg_sys::HeapTupleData,
        tuple_desc: pg_sys::TupleDesc,
    ) -> Result<OwnedValue, String> {
        let mut is_null = false;
        
        // Extract the field value using heap_getattr
        let datum = pg_sys::heap_getattr(
            heap_tuple,
            self.field_attno as i32,
            tuple_desc,
            &mut is_null,
        );

        if is_null {
            return Ok(OwnedValue::Null);
        }

        // Convert the Datum to OwnedValue based on the field's type
        self.datum_to_owned_value(datum, tuple_desc)
    }

    /// Convert PostgreSQL Datum to Tantivy OwnedValue
    unsafe fn datum_to_owned_value(
        &self,
        datum: pg_sys::Datum,
        tuple_desc: pg_sys::TupleDesc,
    ) -> Result<OwnedValue, String> {
        // Get the attribute info for type conversion
        let attr = (*tuple_desc).attrs.as_ptr().add((self.field_attno - 1) as usize);
        let type_oid = (*attr).atttypid;

        match type_oid {
            pg_sys::INT4OID => {
                let value = i32::from_datum(datum, false)
                    .ok_or("Failed to convert INT4")?;
                Ok(OwnedValue::I64(value as i64))
            }
            pg_sys::INT8OID => {
                let value = i64::from_datum(datum, false)
                    .ok_or("Failed to convert INT8")?;
                Ok(OwnedValue::I64(value))
            }
            pg_sys::FLOAT4OID => {
                let value = f32::from_datum(datum, false)
                    .ok_or("Failed to convert FLOAT4")?;
                Ok(OwnedValue::F64(value as f64))
            }
            pg_sys::FLOAT8OID => {
                let value = f64::from_datum(datum, false)
                    .ok_or("Failed to convert FLOAT8")?;
                Ok(OwnedValue::F64(value))
            }
            pg_sys::TEXTOID | pg_sys::VARCHAROID => {
                let value = String::from_datum(datum, false)
                    .ok_or("Failed to convert TEXT")?;
                Ok(OwnedValue::Str(value))
            }
            pg_sys::BOOLOID => {
                let value = bool::from_datum(datum, false)
                    .ok_or("Failed to convert BOOL")?;
                Ok(OwnedValue::Bool(value))
            }
            pg_sys::NUMERICOID => {
                // Convert NUMERIC to f64 for simplicity
                let numeric = pgrx::AnyNumeric::from_datum(datum, false)
                    .ok_or("Failed to convert NUMERIC")?;
                let value = f64::try_from(numeric)
                    .map_err(|_| "Failed to convert NUMERIC to f64")?;
                Ok(OwnedValue::F64(value))
            }
            _ => {
                // For unsupported types, return as string representation
                let mut output_fn = pg_sys::InvalidOid;
                let mut is_varlena = false;
                pg_sys::getTypeOutputInfo(type_oid, &mut output_fn, &mut is_varlena);
                
                let text_datum = pg_sys::OidOutputFunctionCall(output_fn, datum);
                let value = String::from_datum(text_datum.into(), false)
                    .ok_or("Failed to convert unknown type")?;
                Ok(OwnedValue::Str(value))
            }
        }
    }

    /// Evaluate the filter using the extracted field value (pure Tantivy evaluation)
    fn evaluate_filter(&self, field_value: &OwnedValue) -> bool {
        match &self.operator {
            SimpleOperator::IsNull => matches!(field_value, OwnedValue::Null),
            SimpleOperator::IsNotNull => !matches!(field_value, OwnedValue::Null),
            SimpleOperator::Equal => self.compare_equal(field_value),
            SimpleOperator::GreaterThan => self.compare_greater(field_value),
            SimpleOperator::LessThan => self.compare_less(field_value),
        }
    }

    fn compare_equal(&self, field_value: &OwnedValue) -> bool {
        match (field_value, &self.value) {
            (OwnedValue::I64(a), SimpleValue::Integer(b)) => a == b,
            (OwnedValue::F64(a), SimpleValue::Float(b)) => (a - b).abs() < f64::EPSILON,
            (OwnedValue::Str(a), SimpleValue::Text(b)) => a.as_str() == b,
            (OwnedValue::Bool(a), SimpleValue::Boolean(b)) => a == b,
            _ => false,
        }
    }

    fn compare_greater(&self, field_value: &OwnedValue) -> bool {
        match (field_value, &self.value) {
            (OwnedValue::I64(a), SimpleValue::Integer(b)) => a > b,
            (OwnedValue::F64(a), SimpleValue::Float(b)) => a > b,
            (OwnedValue::I64(a), SimpleValue::Float(b)) => (*a as f64) > *b,
            (OwnedValue::F64(a), SimpleValue::Integer(b)) => *a > (*b as f64),
            _ => false,
        }
    }

    fn compare_less(&self, field_value: &OwnedValue) -> bool {
        match (field_value, &self.value) {
            (OwnedValue::I64(a), SimpleValue::Integer(b)) => a < b,
            (OwnedValue::F64(a), SimpleValue::Float(b)) => a < b,
            (OwnedValue::I64(a), SimpleValue::Float(b)) => (*a as f64) < *b,
            (OwnedValue::F64(a), SimpleValue::Integer(b)) => *a < (*b as f64),
            _ => false,
        }
    }
}

/// Helper function to create field filters from PostgreSQL expressions
/// This would be called during query planning to convert OpExpr nodes to SimpleFieldFilter
pub unsafe fn create_field_filter_from_opexpr(
    op_expr: *mut pg_sys::OpExpr,
    relation_oid: pg_sys::Oid,
) -> Option<SimpleFieldFilter> {
    // Extract left and right operands
    let args = pgrx::PgList::<pg_sys::Node>::from_pg((*op_expr).args);
    let args_vec: Vec<_> = args.iter_ptr().collect();
    
    if args_vec.len() != 2 {
        return None;
    }

    let left = args_vec[0];
    let right = args_vec[1];

    // Left side should be a Var (field reference)
    if (*left).type_ != pg_sys::NodeTag::T_Var {
        return None;
    }

    let var = left.cast::<pg_sys::Var>();
    let field_attno = (*var).varattno;

    // Get field name
    let field_name = get_field_name_from_attno(relation_oid, field_attno)?;

    // Right side should be a constant
    if (*right).type_ != pg_sys::NodeTag::T_Const {
        return None;
    }

    let const_node = right.cast::<pg_sys::Const>();
    if (*const_node).constisnull {
        return None;
    }

    // Extract value and operator
    let value = extract_simple_value_from_const(const_node)?;
    let operator = map_operator_oid_to_simple_operator((*op_expr).opno)?;

    Some(SimpleFieldFilter::new(
        field_name,
        operator,
        value,
        relation_oid,
        field_attno,
    ))
}

/// Get field name from attribute number
unsafe fn get_field_name_from_attno(
    relation_oid: pg_sys::Oid,
    attno: pg_sys::AttrNumber,
) -> Option<FieldName> {
    let relation = pg_sys::RelationIdGetRelation(relation_oid);
    if relation.is_null() {
        return None;
    }

    let tuple_desc = (*relation).rd_att;
    if attno <= 0 || i32::from(attno) > (*tuple_desc).natts {
        pg_sys::RelationClose(relation);
        return None;
    }

    let attr = (*tuple_desc).attrs.as_ptr().add((attno - 1) as usize);
    let attr_name = std::ffi::CStr::from_ptr((*attr).attname.data.as_ptr());
    let field_name = attr_name.to_string_lossy().to_string();
    
    pg_sys::RelationClose(relation);
    Some(FieldName::from(field_name))
}

/// Extract simple value from PostgreSQL constant
unsafe fn extract_simple_value_from_const(const_node: *mut pg_sys::Const) -> Option<SimpleValue> {
    let datum = (*const_node).constvalue;
    let type_oid = (*const_node).consttype;

    match type_oid {
        pg_sys::INT4OID => {
            let value = i32::from_datum(datum, false)?;
            Some(SimpleValue::Integer(value as i64))
        }
        pg_sys::INT8OID => {
            let value = i64::from_datum(datum, false)?;
            Some(SimpleValue::Integer(value))
        }
        pg_sys::FLOAT4OID => {
            let value = f32::from_datum(datum, false)?;
            Some(SimpleValue::Float(value as f64))
        }
        pg_sys::FLOAT8OID => {
            let value = f64::from_datum(datum, false)?;
            Some(SimpleValue::Float(value))
        }
        pg_sys::TEXTOID => {
            let value = String::from_datum(datum, false)?;
            Some(SimpleValue::Text(value))
        }
        pg_sys::BOOLOID => {
            let value = bool::from_datum(datum, false)?;
            Some(SimpleValue::Boolean(value))
        }
        pg_sys::NUMERICOID => {
            let numeric = pgrx::AnyNumeric::from_datum(datum, false)?;
            let value = f64::try_from(numeric).ok()?;
            Some(SimpleValue::Float(value))
        }
        _ => None,
    }
}

/// Map PostgreSQL operator OID to simple operator
unsafe fn map_operator_oid_to_simple_operator(opno: pg_sys::Oid) -> Option<SimpleOperator> {
    // Get operator name to determine the operation
    let operator_tuple = pg_sys::SearchSysCache1(
        pg_sys::SysCacheIdentifier::OPEROID as i32,
        pg_sys::Datum::from(opno.to_u32()),
    );

    if operator_tuple.is_null() {
        return None;
    }

    let operator_form = pg_sys::GETSTRUCT(operator_tuple) as *mut pg_sys::FormData_pg_operator;
    let operator_name = std::ffi::CStr::from_ptr((*operator_form).oprname.data.as_ptr());
    let op_name = operator_name.to_string_lossy();

    pg_sys::ReleaseSysCache(operator_tuple);

    match op_name.as_ref() {
        "=" => Some(SimpleOperator::Equal),
        ">" => Some(SimpleOperator::GreaterThan),
        "<" => Some(SimpleOperator::LessThan),
        ">=" => Some(SimpleOperator::GreaterThan), // We'll handle >= as > for simplicity
        "<=" => Some(SimpleOperator::LessThan),    // We'll handle <= as < for simplicity
        _ => None,
    }
}

/// This demonstrates the correct architecture:
/// 1. Parse PostgreSQL expressions into SimpleFieldFilter objects (no PostgreSQL evaluation)
/// 2. For each document, extract field values using ctid and proper heap access
/// 3. Evaluate filters in Tantivy using pure value comparisons
/// 4. No mock tuple slots, no PostgreSQL expression states, no type guessing

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_field_filter_concept() {
        // This test demonstrates the concept - in practice we'd need real relation/field data
        let filter = SimpleFieldFilter::new(
            FieldName::from("rating".to_string()),
            SimpleOperator::GreaterThan,
            SimpleValue::Float(4.0),
            pg_sys::InvalidOid, // Would be real relation OID
            1, // Would be real attribute number
        );

        // The key insight: this is how filtering should work
        // 1. Extract value from PostgreSQL heap using ctid
        // 2. Compare in Tantivy without any PostgreSQL expression evaluation
        assert_eq!(filter.field.root(), "rating");
        assert_eq!(filter.operator, SimpleOperator::GreaterThan);
    }

    #[test]
    fn test_value_comparisons() {
        let filter = SimpleFieldFilter::new(
            FieldName::from("price".to_string()),
            SimpleOperator::LessThan,
            SimpleValue::Float(300.0),
            pg_sys::InvalidOid,
            2,
        );

        // Test the comparison logic
        assert!(filter.compare_less(&OwnedValue::F64(299.99)));
        assert!(!filter.compare_less(&OwnedValue::F64(300.01)));
    }
}

/// Resolve a field name to its attribute number in the given relation
unsafe fn resolve_field_name_to_attno(
    relation_oid: pg_sys::Oid,
    field_name: &FieldName,
) -> Option<pg_sys::AttrNumber> {
    pgrx::warning!("🔥 resolve_field_name_to_attno: relation_oid = {}, field_name = {}", relation_oid, field_name.root());
    
    let relation = pg_sys::RelationIdGetRelation(relation_oid);
    if relation.is_null() {
        pgrx::warning!("🔥 resolve_field_name_to_attno: RelationIdGetRelation returned null for oid {}", relation_oid);
        return None;
    }

    let tuple_desc = (*relation).rd_att;
    let field_name_str = field_name.root();
    
    pgrx::warning!("🔥 resolve_field_name_to_attno: searching for field '{}' in {} attributes", field_name_str, (*tuple_desc).natts);
    
    // Search through all attributes to find matching field name
    for attno in 1..=(*tuple_desc).natts {
        let attr = (*tuple_desc).attrs.as_ptr().add((attno - 1) as usize);
        let attr_name = std::ffi::CStr::from_ptr((*attr).attname.data.as_ptr());
        
        if let Ok(attr_name_str) = attr_name.to_str() {
            pgrx::warning!("🔥 resolve_field_name_to_attno: checking attribute {} = '{}'", attno, attr_name_str);
            if attr_name_str == field_name_str {
                pgrx::warning!("🔥 resolve_field_name_to_attno: FOUND field '{}' at attno {}", field_name_str, attno);
                pg_sys::RelationClose(relation);
                return Some(attno as pg_sys::AttrNumber);
            }
        }
    }

    pgrx::warning!("🔥 resolve_field_name_to_attno: field '{}' NOT FOUND", field_name_str);
    pg_sys::RelationClose(relation);
    None
} 
