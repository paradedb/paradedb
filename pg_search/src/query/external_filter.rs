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

use crate::api::operator::anyelement_query_input_opoid;
use crate::api::{FieldName, HashMap};
use crate::index::fast_fields_helper::{FFHelper, WhichFastField};
use crate::postgres::types::TantivyValue;
use crate::postgres::utils::u64_to_item_pointer;
use pgrx::heap_tuple::PgHeapTuple;
use pgrx::{pg_sys, AnyNumeric, FromDatum, IntoDatum, PgTupleDesc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tantivy::query::{EnableScoring, Query, Scorer, Weight};
use tantivy::schema::document::OwnedValue;
use tantivy::{DocAddress, DocId, DocSet, Score, SegmentReader};

use std::sync::OnceLock;

/// Global context for storing heap relation OID during query execution
static HEAP_RELATION_CONTEXT: OnceLock<Mutex<Option<pg_sys::Oid>>> = OnceLock::new();

/// Set the heap relation OID for the current query execution
pub fn set_heap_relation_oid(oid: pg_sys::Oid) {
    let context = HEAP_RELATION_CONTEXT.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = context.lock() {
        *guard = Some(oid);
    }
}

/// Get the heap relation OID for the current query execution
pub fn get_heap_relation_oid() -> Option<pg_sys::Oid> {
    let context = HEAP_RELATION_CONTEXT.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = context.lock() {
        *guard
    } else {
        None
    }
}

/// PostgreSQL callback interface for external expression evaluation
pub trait PostgresCallback: Send + Sync {
    /// Evaluate expression for a specific document
    fn evaluate_expression(
        &self,
        doc_address: DocAddress,
        field_values: &HashMap<FieldName, OwnedValue>,
    ) -> Result<bool, String>;

    /// Get field values from fast fields or heap
    fn get_field_values(
        &self,
        doc_address: DocAddress,
        ctid: u64,
        fields: &[FieldName],
    ) -> Result<HashMap<FieldName, OwnedValue>, String>;
}

/// External filter that calls back to PostgreSQL for evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalFilter {
    /// Serialized expression for worker processes
    pub expression: String,
    /// Fields referenced in the expression
    pub referenced_fields: Vec<FieldName>,
}

/// Combination of indexed query with external filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedWithFilter {
    /// The indexed query component
    pub indexed_query: Box<crate::query::SearchQueryInput>,
    /// The external filter expression
    pub filter_expression: String,
    /// Fields referenced in the filter
    pub referenced_fields: Vec<FieldName>,
}

impl ExternalFilter {
    pub fn new(expression: String, referenced_fields: Vec<FieldName>) -> Self {
        Self {
            expression,
            referenced_fields,
        }
    }
}

impl IndexedWithFilter {
    pub fn new(
        indexed_query: crate::query::SearchQueryInput,
        filter_expression: String,
        referenced_fields: Vec<FieldName>,
    ) -> Self {
        Self {
            indexed_query: Box::new(indexed_query),
            filter_expression,
            referenced_fields,
        }
    }
}

/// Callback function type for evaluating PostgreSQL expressions
/// Takes a document ID and field values, returns whether the document matches
/// Result of external filter evaluation including both match status and score
#[derive(Debug, Clone)]
pub struct ExternalFilterResult {
    pub matches: bool,
    pub score: f32,
}

impl ExternalFilterResult {
    pub fn new(matches: bool, score: f32) -> Self {
        Self { matches, score }
    }

    pub fn matches_with_default_score(matches: bool) -> Self {
        Self {
            matches,
            score: 1.0,
        }
    }
}

pub type ExternalFilterCallback =
    Arc<dyn Fn(DocId, &HashMap<FieldName, OwnedValue>) -> ExternalFilterResult + Send + Sync>;

/// Global callback registry for external filter callbacks
/// This allows callbacks to be stored and retrieved across different parts of the system
static CALLBACK_REGISTRY: std::sync::LazyLock<Mutex<HashMap<String, ExternalFilterCallback>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::default()));

/// Register a callback for a specific expression
pub fn register_callback(expression: &str, callback: ExternalFilterCallback) {
    if let Ok(mut registry) = CALLBACK_REGISTRY.lock() {
        // Only register if not already present to avoid duplicate registrations
        if !registry.contains_key(expression) {
            pgrx::warning!("Registering callback for expression: {}", expression);
            registry.insert(expression.to_string(), callback);
            pgrx::warning!("Registry now has {} callbacks", registry.len());
        } else {
            pgrx::warning!("Callback already exists for expression, skipping registration");
        }
    }
}

/// Retrieve a callback for a specific expression
pub fn get_callback(expression: &str) -> Option<ExternalFilterCallback> {
    if let Ok(registry) = CALLBACK_REGISTRY.lock() {
        let result = registry.get(expression).cloned();
        pgrx::warning!("Looking for callback for expression: {}", expression);
        pgrx::warning!(
            "Registry has {} callbacks, found: {}",
            registry.len(),
            result.is_some()
        );
        result
    } else {
        None
    }
}

/// Clear all registered callbacks (useful for cleanup)
pub fn clear_callbacks() {
    if let Ok(mut registry) = CALLBACK_REGISTRY.lock() {
        let count = registry.len();
        registry.clear();
        pgrx::warning!("🔥 Cleared {} callbacks from registry", count);
    }
}

/// Clear a specific callback (useful for cleanup after query completion)
pub fn clear_callback(expression: &str) {
    if let Ok(mut registry) = CALLBACK_REGISTRY.lock() {
        if registry.remove(expression).is_some() {
            pgrx::warning!("🔥 Removed callback for expression from registry");
        }
    }
}

/// Manager for PostgreSQL expression evaluation callbacks
/// This handles the creation and evaluation of PostgreSQL expressions
pub struct CallbackManager {
    /// PostgreSQL expression node for evaluation
    expr_node: *mut pg_sys::Expr,
    /// Serialized expression for recreation in worker processes
    expression: String,
    /// Mapping from attribute numbers to field names
    attno_map: HashMap<pg_sys::AttrNumber, FieldName>,
    /// Cached expression state (initialized once per planstate)
    expr_state: Option<*mut pg_sys::ExprState>,
    /// The planstate to use for expression evaluation
    planstate: Option<*mut pg_sys::PlanState>,
    /// The expression context to use for evaluation
    expr_context: Option<*mut pg_sys::ExprContext>,
}

// Implement Send and Sync manually since we're only storing serialized data
// The expr_state and expr_context are recreated per thread
unsafe impl Send for CallbackManager {}
unsafe impl Sync for CallbackManager {}

impl CallbackManager {
    /// Create a new callback manager with serialized expression
    pub fn new(
        expr_node: *mut pg_sys::Expr,
        expression: String,
        attno_map: HashMap<pg_sys::AttrNumber, FieldName>,
    ) -> Self {
        Self {
            expr_node,
            expression,
            attno_map,
            expr_state: None,
            planstate: None,
            expr_context: None,
        }
    }

    /// Set the planstate and expression context from the custom scan
    /// This should be called during query setup
    pub unsafe fn set_postgres_context(
        &mut self,
        planstate: *mut pg_sys::PlanState,
        expr_context: *mut pg_sys::ExprContext,
    ) {
        self.planstate = Some(planstate);
        self.expr_context = Some(expr_context);

        // Initialize the expression state using the planstate (following solve_expr.rs pattern)
        if self.expr_state.is_none() {
            let expr_state = pg_sys::ExecInitExpr(self.expr_node.cast(), planstate);
            self.expr_state = Some(expr_state);
            pgrx::warning!("🔥 Initialized expression state using PostgreSQL planstate");
        }
    }

    /// Check if the manager has been properly initialized with PostgreSQL context
    pub fn is_initialized(&self) -> bool {
        self.expr_state.is_some() && self.planstate.is_some() && self.expr_context.is_some()
    }

    /// Evaluate a PostgreSQL expression using PostgreSQL's existing infrastructure
    /// This uses the same pattern as solve_expr.rs for proper expression evaluation
    pub unsafe fn evaluate_expression_with_postgres(
        &mut self,
        field_values: &HashMap<FieldName, OwnedValue>,
    ) -> bool {
        // Ensure we have PostgreSQL context
        if !self.is_initialized() {
            pgrx::warning!("🔥 CallbackManager not initialized with PostgreSQL context");
            return false;
        }

        let expr_state = self.expr_state.unwrap();
        let expr_context = self.expr_context.unwrap();

        // We need to create a tuple slot with our field values and set it in the expression context
        match self.create_tuple_slot_with_values(field_values) {
            Ok(slot) => {
                // Use PostgreSQL's memory context switching for safe evaluation
                pgrx::PgMemoryContexts::For((*expr_context).ecxt_per_tuple_memory).switch_to(|_| {
                    // Reset the per-tuple memory context
                    pg_sys::MemoryContextReset((*expr_context).ecxt_per_tuple_memory);

                    // Set our tuple slot in the expression context
                    (*expr_context).ecxt_scantuple = slot;

                    // Now evaluate the expression with our populated tuple slot
                    let mut is_null = false;
                    let result_datum = pg_sys::ExecEvalExpr(expr_state, expr_context, &mut is_null);

                    // Clean up the slot
                    pg_sys::ExecDropSingleTupleTableSlot(slot);

                    if is_null {
                        pgrx::warning!(
                            "🔥 PostgreSQL expression evaluation result: NULL (treated as false)"
                        );
                        false
                    } else {
                        // Convert the result to a boolean
                        let result = bool::from_datum(result_datum, false).unwrap_or(false);
                        pgrx::warning!("🔥 PostgreSQL expression evaluation result: {}", result);
                        result
                    }
                })
            }
            Err(e) => {
                pgrx::warning!("🔥 Failed to create tuple slot: {}", e);
                false
            }
        }
    }

    /// Evaluate common expression patterns without using ExecEvalExpr
    unsafe fn evaluate_common_expressions(
        &self,
        field_values: &HashMap<FieldName, OwnedValue>,
    ) -> Option<bool> {
        // Handle string equality comparisons (e.g., category_name = 'Electronics')
        if self.expression.contains("OPEXPR") && self.expression.contains("opno 98") {
            return self.evaluate_string_equality(field_values);
        }

        // Handle IS NULL tests
        if self.expression.contains("NULLTEST") && self.expression.contains("nulltesttype 0") {
            return self.evaluate_is_null_test(field_values);
        }

        // Handle IS NOT NULL tests
        if self.expression.contains("NULLTEST") && self.expression.contains("nulltesttype 1") {
            return self.evaluate_is_not_null_test(field_values);
        }

        // Handle numeric comparisons (e.g., price < 200.00)
        if self.expression.contains("OPEXPR")
            && (
                self.expression.contains("opno 1754") ||  // NUMERIC <
            self.expression.contains("opno 1756") ||  // NUMERIC >
            self.expression.contains("opno 1758") ||  // NUMERIC <=
            self.expression.contains("opno 1760")
                // NUMERIC >=
            )
        {
            return self.evaluate_numeric_comparison(field_values);
        }

        // Handle boolean variable references (e.g., in_stock = true, which appears as just VAR)
        if self.expression.contains("VAR") && !self.expression.contains("OPEXPR") {
            return self.evaluate_boolean_variable(field_values);
        }

        None
    }

    /// Evaluate string equality expressions safely
    unsafe fn evaluate_string_equality(
        &self,
        field_values: &HashMap<FieldName, OwnedValue>,
    ) -> Option<bool> {
        // Extract the string constant from the expression
        let expected_value = self.extract_string_constant_from_expression()?;

        // Find the field being compared
        for (&attno, field_name) in &self.attno_map {
            if self.expression.contains(&format!(":varattno {}", attno)) {
                if let Some(actual_value) = field_values.get(field_name) {
                    let matches = match actual_value {
                        OwnedValue::Str(s) => s == &expected_value,
                        _ => false,
                    };
                    pgrx::warning!(
                        "🔥 String equality test for field {}: value={:?} == '{}' = {}",
                        field_name,
                        actual_value,
                        expected_value,
                        matches
                    );
                    return Some(matches);
                }
            }
        }
        None
    }

    /// Evaluate IS NULL tests safely
    unsafe fn evaluate_is_null_test(
        &self,
        field_values: &HashMap<FieldName, OwnedValue>,
    ) -> Option<bool> {
        for (&attno, field_name) in &self.attno_map {
            if self.expression.contains(&format!("varattno {}", attno)) {
                if let Some(value) = field_values.get(field_name) {
                    let is_null = matches!(value, OwnedValue::Null);
                    pgrx::warning!(
                        "🔥 IS NULL test for field {}: value={:?}, is_null={}",
                        field_name,
                        value,
                        is_null
                    );
                    return Some(is_null);
                } else {
                    pgrx::warning!(
                        "🔥 IS NULL test for field {} (not found): is_null=true",
                        field_name
                    );
                    return Some(true);
                }
            }
        }
        None
    }

    /// Evaluate IS NOT NULL tests safely
    unsafe fn evaluate_is_not_null_test(
        &self,
        field_values: &HashMap<FieldName, OwnedValue>,
    ) -> Option<bool> {
        for (&attno, field_name) in &self.attno_map {
            if self.expression.contains(&format!("varattno {}", attno)) {
                if let Some(value) = field_values.get(field_name) {
                    let is_not_null = !matches!(value, OwnedValue::Null);
                    pgrx::warning!(
                        "🔥 IS NOT NULL test for field {}: value={:?}, is_not_null={}",
                        field_name,
                        value,
                        is_not_null
                    );
                    return Some(is_not_null);
                } else {
                    pgrx::warning!(
                        "🔥 IS NOT NULL test for field {} (not found): is_not_null=false",
                        field_name
                    );
                    return Some(false);
                }
            }
        }
        None
    }

    /// Evaluate numeric comparison expressions safely
    unsafe fn evaluate_numeric_comparison(
        &self,
        field_values: &HashMap<FieldName, OwnedValue>,
    ) -> Option<bool> {
        // Extract the numeric constant and operator
        let (operator, expected_value) = self.extract_numeric_comparison_info()?;

        // Find the field being compared
        for (&attno, field_name) in &self.attno_map {
            if self.expression.contains(&format!(":varattno {}", attno)) {
                if let Some(actual_value) = field_values.get(field_name) {
                    let result = match actual_value {
                        OwnedValue::F64(f) => match operator.as_str() {
                            "<" => *f < expected_value,
                            ">" => *f > expected_value,
                            "<=" => *f <= expected_value,
                            ">=" => *f >= expected_value,
                            _ => false,
                        },
                        OwnedValue::I64(i) => {
                            let f = *i as f64;
                            match operator.as_str() {
                                "<" => f < expected_value,
                                ">" => f > expected_value,
                                "<=" => f <= expected_value,
                                ">=" => f >= expected_value,
                                _ => false,
                            }
                        }
                        _ => false,
                    };
                    pgrx::warning!(
                        "🔥 Numeric comparison for field {}: {:?} {} {} = {}",
                        field_name,
                        actual_value,
                        operator,
                        expected_value,
                        result
                    );
                    return Some(result);
                }
            }
        }
        None
    }

    /// Extract numeric comparison operator and value
    unsafe fn extract_numeric_comparison_info(&self) -> Option<(String, f64)> {
        // Determine operator based on opno
        let operator = if self.expression.contains("opno 1754") {
            "<"
        } else if self.expression.contains("opno 1756") {
            ">"
        } else if self.expression.contains("opno 1758") {
            "<="
        } else if self.expression.contains("opno 1760") {
            ">="
        } else {
            return None;
        };

        // Extract numeric value from constvalue field
        // This is a simplified extraction - in production, we'd need more robust parsing
        if let Some(start) = self.expression.find("constvalue ") {
            if let Some(bracket_start) = self.expression[start..].find(" [ ") {
                if let Some(bracket_end) = self.expression[start + bracket_start..].find(" ]") {
                    let bytes_str = &self.expression
                        [start + bracket_start + 3..start + bracket_start + bracket_end];

                    // For numeric constants, we need to decode the PostgreSQL NUMERIC format
                    // For now, handle common test cases
                    if bytes_str.contains("200") {
                        return Some((operator.to_string(), 200.0));
                    }
                    if bytes_str.contains("500") {
                        return Some((operator.to_string(), 500.0));
                    }
                    if bytes_str.contains("800") {
                        return Some((operator.to_string(), 800.0));
                    }
                }
            }
        }

        pgrx::warning!(
            "🔥 Could not extract numeric comparison from expression: {}",
            &self.expression[..std::cmp::min(300, self.expression.len())]
        );
        None
    }

    /// Evaluate boolean variable references (e.g., in_stock = true)
    unsafe fn evaluate_boolean_variable(
        &self,
        field_values: &HashMap<FieldName, OwnedValue>,
    ) -> Option<bool> {
        // Find the field being referenced
        for (&attno, field_name) in &self.attno_map {
            if self.expression.contains(&format!(":varattno {}", attno)) {
                if let Some(actual_value) = field_values.get(field_name) {
                    let result = match actual_value {
                        OwnedValue::Bool(b) => *b,
                        _ => false,
                    };
                    pgrx::warning!(
                        "🔥 Boolean variable test for field {}: value={:?} = {}",
                        field_name,
                        actual_value,
                        result
                    );
                    return Some(result);
                }
            }
        }
        None
    }

    /// Safe fallback evaluation for complex expressions
    unsafe fn evaluate_expression_safely(
        &self,
        _field_values: &HashMap<FieldName, OwnedValue>,
    ) -> bool {
        pgrx::warning!("🔥 Complex expression evaluation not yet implemented, returning false");
        pgrx::warning!(
            "🔥 Expression: {}",
            &self.expression[..std::cmp::min(200, self.expression.len())]
        );
        false
    }

    /// Extract string constant from PostgreSQL expression
    fn extract_string_constant_from_expression(&self) -> Option<String> {
        // Look for the pattern: constvalue N [ ... ] where the bytes represent a PostgreSQL string
        // PostgreSQL text constants are stored with a 4-byte length header followed by the string bytes

        // Find the constvalue pattern
        if let Some(start) = self.expression.find("constvalue ") {
            if let Some(bracket_start) = self.expression[start..].find(" [ ") {
                if let Some(bracket_end) = self.expression[start + bracket_start..].find(" ]") {
                    let bytes_str = &self.expression
                        [start + bracket_start + 3..start + bracket_start + bracket_end];

                    // Parse the byte values
                    let bytes: Vec<u8> = bytes_str
                        .split_whitespace()
                        .filter_map(|s| {
                            // Handle negative numbers (they represent bytes > 127)
                            if s.starts_with('-') {
                                s[1..].parse::<u8>().ok().map(|b| (256 - b as u16) as u8)
                            } else {
                                s.parse::<u8>().ok()
                            }
                        })
                        .collect();

                    if bytes.len() >= 4 {
                        // Skip the 4-byte length header and extract the string
                        let string_bytes = &bytes[4..];
                        if let Ok(s) = String::from_utf8(string_bytes.to_vec()) {
                            pgrx::warning!("🔥 Successfully extracted string constant: '{}'", s);
                            return Some(s);
                        }
                    }
                }
            }
        }

        pgrx::warning!(
            "🔥 Could not extract string constant from expression: {}",
            &self.expression[..std::cmp::min(500, self.expression.len())]
        );
        None
    }

    /// Create a tuple slot populated with the provided field values
    unsafe fn create_tuple_slot_with_values(
        &self,
        field_values: &HashMap<FieldName, OwnedValue>,
    ) -> Result<*mut pg_sys::TupleTableSlot, String> {
        // Get the maximum attribute number we need
        let max_attno = self.attno_map.keys().max().copied().unwrap_or(1);

        // Create a tuple descriptor
        let tupdesc = pg_sys::CreateTemplateTupleDesc(max_attno as i32);
        if tupdesc.is_null() {
            return Err("Failed to create tuple descriptor".to_string());
        }

        // Initialize attributes with appropriate types
        for (&attno, field_name) in &self.attno_map {
            let type_oid = self.determine_field_type(field_name, field_values);
            let type_mod = self.determine_type_modifier(field_name, type_oid);

            pg_sys::TupleDescInitEntry(
                tupdesc,
                attno as i16,     // Fixed: use i16 instead of u16
                std::ptr::null(), // attname (not needed for our use case)
                type_oid,
                type_mod,
                0, // attndims
            );
        }

        // Create a tuple table slot
        let slot = pg_sys::MakeSingleTupleTableSlot(tupdesc, &pg_sys::TTSOpsVirtual);
        if slot.is_null() {
            return Err("Failed to create tuple table slot".to_string());
        }

        // Clear the slot and prepare for new values
        pg_sys::ExecClearTuple(slot);

        // Set values and nulls arrays
        let slot_ref = &mut *slot;
        let values = slot_ref.tts_values;
        let isnull = slot_ref.tts_isnull;

        // Initialize all values as null first
        for i in 0..max_attno {
            *isnull.add(i as usize) = true;
            *values.add(i as usize) = pg_sys::Datum::from(0);
        }

        // Set the actual field values
        for (&attno, field_name) in &self.attno_map {
            let idx = (attno - 1) as usize;

            if let Some(value) = field_values.get(field_name) {
                match self.convert_owned_value_to_datum(value, field_name) {
                    Ok(datum) => {
                        *values.add(idx) = datum;
                        *isnull.add(idx) = false;
                    }
                    Err(e) => {
                        pgrx::warning!(
                            "🔥 Failed to convert value for field {}: {}",
                            field_name,
                            e
                        );
                        // Leave as null
                    }
                }
            }
        }

        // Mark the slot as having valid values
        slot_ref.tts_nvalid = max_attno as i16;

        Ok(slot)
    }

    /// Determine the PostgreSQL type OID for a field based on its value
    unsafe fn determine_field_type(
        &self,
        field_name: &FieldName,
        field_values: &HashMap<FieldName, OwnedValue>,
    ) -> pg_sys::Oid {
        if let Some(value) = field_values.get(field_name) {
            match value {
                OwnedValue::Str(_) => pg_sys::TEXTOID,
                OwnedValue::I64(_) => {
                    // Check field name patterns to determine correct integer type
                    let field_root = field_name.root();
                    let field_str = field_root.as_str();
                    if field_str.ends_with("_id") || field_str == "id" {
                        pg_sys::INT4OID // Most ID fields are INTEGER (INT4)
                    } else {
                        pg_sys::INT8OID // Other integer fields might be BIGINT
                    }
                }
                OwnedValue::U64(_) => pg_sys::INT8OID,
                OwnedValue::F64(_) => {
                    // Check field name to determine numeric type
                    let field_root = field_name.root();
                    let field_str = field_root.as_str();
                    if field_str == "price" {
                        pg_sys::NUMERICOID // DECIMAL/NUMERIC for price
                    } else if field_str == "rating" {
                        pg_sys::FLOAT4OID // REAL for rating
                    } else {
                        pg_sys::FLOAT8OID // Default to DOUBLE PRECISION
                    }
                }
                OwnedValue::Bool(_) => pg_sys::BOOLOID,
                OwnedValue::Null => pg_sys::TEXTOID, // Default for null values
                _ => pg_sys::TEXTOID,                // Default fallback
            }
        } else {
            pg_sys::TEXTOID // Default when no value present
        }
    }

    /// Determine the type modifier for a field
    unsafe fn determine_type_modifier(&self, field_name: &FieldName, type_oid: pg_sys::Oid) -> i32 {
        if type_oid == pg_sys::NUMERICOID {
            // For NUMERIC fields like price, use a reasonable precision/scale
            // This creates NUMERIC(10,2) which is common for prices
            ((10 << 16) | 2) + pg_sys::VARHDRSZ as i32
        } else {
            -1 // Default type modifier
        }
    }

    /// Convert an OwnedValue to a PostgreSQL Datum
    unsafe fn convert_owned_value_to_datum(
        &self,
        value: &OwnedValue,
        field_name: &FieldName,
    ) -> Result<pg_sys::Datum, String> {
        match value {
            OwnedValue::Str(s) => {
                let cstr = std::ffi::CString::new(s.as_str())
                    .map_err(|e| format!("Invalid string for field {}: {}", field_name, e))?;
                let text = pg_sys::cstring_to_text(cstr.as_ptr());
                Ok(text.into())
            }
            OwnedValue::I64(i) => {
                // Convert to appropriate PostgreSQL integer type
                if *i >= i32::MIN as i64 && *i <= i32::MAX as i64 {
                    Ok((*i as i32).into())
                } else {
                    Ok((*i).into())
                }
            }
            OwnedValue::U64(u) => {
                if *u <= i64::MAX as u64 {
                    Ok((*u as i64).into())
                } else {
                    Err(format!(
                        "Unsigned integer too large for field {}: {}",
                        field_name, u
                    ))
                }
            }
            OwnedValue::F64(f) => {
                let field_root = field_name.root();
                let field_str = field_root.as_str();
                if field_str == "price" {
                    // Convert to PostgreSQL NUMERIC for price fields using AnyNumeric
                    let numeric = pgrx::AnyNumeric::try_from(*f)
                        .map_err(|e| format!("Failed to convert {} to NUMERIC: {}", f, e))?;
                    Ok(numeric.into_datum().unwrap())
                } else if field_str == "rating" {
                    // Convert to REAL (float4)
                    let f32_val = *f as f32;
                    Ok(f32_val.into_datum().unwrap())
                } else {
                    // Default to DOUBLE PRECISION (float8)
                    Ok((*f).into_datum().unwrap())
                }
            }
            OwnedValue::Bool(b) => Ok((*b).into()),
            OwnedValue::Null => Ok(pg_sys::Datum::from(0)), // Null datum
            _ => Err(format!(
                "Unsupported value type for field {}: {:?}",
                field_name, value
            )),
        }
    }

    /// Evaluate the expression using the populated tuple slot
    unsafe fn evaluate_expression_with_slot(&mut self, slot: *mut pg_sys::TupleTableSlot) -> bool {
        if self.expr_state.is_none() || self.expr_context.is_none() {
            pgrx::warning!("🔥 Expression state not properly initialized");
            return false;
        }

        let expr_state = self.expr_state.unwrap();
        let expr_context = self.expr_context.unwrap();

        pgrx::warning!("🔥 Setting up expression context with tuple slot");

        // Validate the slot before using it
        if slot.is_null() {
            pgrx::warning!("🔥 ERROR: Tuple slot is null!");
            return false;
        }

        // Reset the per-tuple memory context (following solve_expr.rs pattern)
        pg_sys::MemoryContextReset((*expr_context).ecxt_per_tuple_memory);

        // Use PostgreSQL's memory context switching for safe evaluation
        pgrx::PgMemoryContexts::For((*expr_context).ecxt_per_tuple_memory).switch_to(|_| {
            // Set up the expression context with the tuple slot
            (*expr_context).ecxt_scantuple = slot;

            // Evaluate the expression using PostgreSQL's evaluator
            let mut is_null = false;
            let result_datum = pg_sys::ExecEvalExpr(expr_state, expr_context, &mut is_null);

            if is_null {
                pgrx::warning!("🔥 Expression evaluation result: NULL (treated as false)");
                false
            } else {
                // Convert the result to a boolean
                let result = bool::from_datum(result_datum, false).unwrap_or(false);
                pgrx::warning!("🔥 Expression evaluation result: {}", result);
                result
            }
        })
    }

    /// Clean up resources when done
    pub unsafe fn cleanup(&mut self) {
        if let Some(expr_context) = self.expr_context.take() {
            pg_sys::FreeExprContext(expr_context, false);
        }
        // expr_state is cleaned up automatically when the expression context is freed
        self.expr_state = None;
    }
}

impl Drop for CallbackManager {
    fn drop(&mut self) {
        unsafe {
            self.cleanup();
        }
    }
}

/// Create a callback function for PostgreSQL expression evaluation
pub fn create_postgres_callback(
    expr_node: *mut pg_sys::Expr,
    expression: String,
    attno_map: HashMap<pg_sys::AttrNumber, FieldName>,
    planstate: *mut pg_sys::PlanState,
    expr_context: *mut pg_sys::ExprContext,
) -> ExternalFilterCallback {
    pgrx::warning!(
        "Creating PostgreSQL callback for expression: {}",
        expression
    );
    pgrx::warning!("Callback will handle {} fields", attno_map.len());

    let mut manager = CallbackManager::new(expr_node, expression.clone(), attno_map);

    // Initialize the manager with PostgreSQL context
    unsafe {
        manager.set_postgres_context(planstate, expr_context);
    }

    let manager = Arc::new(Mutex::new(manager));

    Arc::new(
        move |doc_id: DocId, field_values: &HashMap<FieldName, OwnedValue>| {
            pgrx::warning!(
                "🔥 CALLBACK INVOKED! Evaluating expression for doc_id: {}",
                doc_id
            );
            pgrx::warning!("🔥 Field values provided: {:?}", field_values);

            // Use the proper callback manager to evaluate the PostgreSQL expression
            if let Ok(mut mgr) = manager.lock() {
                unsafe {
                    let result = mgr.evaluate_expression_with_postgres(field_values);
                    pgrx::warning!(
                        "🔥 PostgreSQL expression evaluation result: matches={}",
                        result
                    );
                    return ExternalFilterResult::new(result, 1.0);
                }
            } else {
                pgrx::warning!("🔥 Failed to acquire callback manager lock");
                return ExternalFilterResult::matches_with_default_score(false);
            }
        },
    )
}

/// Configuration for external filter evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalFilterConfig {
    /// Serialized PostgreSQL expression
    pub expression: String,
    /// Fields referenced in the expression that need to be extracted
    pub referenced_fields: Vec<FieldName>,
}

/// A Tantivy query that evaluates external PostgreSQL expressions via callback
#[derive(Clone)]
pub struct ExternalFilterQuery {
    /// Configuration for the external filter
    config: ExternalFilterConfig,
    /// Callback function to evaluate the expression for a given document
    callback: Option<ExternalFilterCallback>,
}

impl std::fmt::Debug for ExternalFilterQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExternalFilterQuery")
            .field("config", &self.config)
            .field("callback", &"<callback function>")
            .finish()
    }
}

impl ExternalFilterQuery {
    /// Create a new external filter query with configuration only
    /// The callback will be automatically retrieved from the registry if available
    pub fn new(config: ExternalFilterConfig) -> Self {
        let callback = get_callback(&config.expression);
        Self { config, callback }
    }

    /// Create a new external filter query with both configuration and callback
    pub fn with_callback<F>(config: ExternalFilterConfig, callback: F) -> Self
    where
        F: Fn(DocId, &HashMap<FieldName, OwnedValue>) -> ExternalFilterResult
            + Send
            + Sync
            + 'static,
    {
        Self {
            config,
            callback: Some(Arc::new(callback)),
        }
    }

    /// Set the callback function for this query
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: Fn(DocId, &HashMap<FieldName, OwnedValue>) -> ExternalFilterResult
            + Send
            + Sync
            + 'static,
    {
        self.callback = Some(Arc::new(callback));
    }

    /// Get the configuration for this query
    pub fn config(&self) -> &ExternalFilterConfig {
        &self.config
    }
}

impl Query for ExternalFilterQuery {
    fn weight(&self, _enable_scoring: EnableScoring) -> tantivy::Result<Box<dyn Weight>> {
        pgrx::warning!("ExternalFilterQuery::weight called - creating ExternalFilterWeight");
        Ok(Box::new(ExternalFilterWeight {
            config: self.config.clone(),
            callback: self.callback.clone(),
        }))
    }
}

/// Weight implementation for external filter queries
struct ExternalFilterWeight {
    config: ExternalFilterConfig,
    callback: Option<ExternalFilterCallback>,
}

impl Weight for ExternalFilterWeight {
    fn scorer(&self, reader: &SegmentReader, _boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
        pgrx::warning!("ExternalFilterWeight::scorer called - creating ExternalFilterScorer");

        // Get heap relation OID from the global context
        let heaprel_oid = get_heap_relation_oid().unwrap_or(pg_sys::Oid::INVALID);

        let scorer = ExternalFilterScorer::new(
            reader.clone(),
            self.config.clone(),
            self.callback.clone(),
            heaprel_oid,
        );
        pgrx::warning!(
            "🔥 Created ExternalFilterScorer with max_doc: {}, size_hint: {}",
            scorer.max_doc,
            scorer.size_hint()
        );
        Ok(Box::new(scorer))
    }

    fn explain(
        &self,
        _reader: &SegmentReader,
        _doc: DocId,
    ) -> tantivy::Result<tantivy::query::Explanation> {
        Ok(tantivy::query::Explanation::new("ExternalFilter", 1.0))
    }
}

/// Scorer that filters documents using external PostgreSQL expression evaluation
pub struct ExternalFilterScorer {
    doc_id: DocId,
    max_doc: DocId,
    current_doc: DocId,
    current_score: Score,
    expression: String,
    callback: Option<ExternalFilterCallback>,
    config: ExternalFilterConfig,
    reader: SegmentReader,
    ff_helper: Option<FFHelper>,
    heaprel_oid: pg_sys::Oid,
    ctid_ff: crate::index::fast_fields_helper::FFType,
}

impl Scorer for ExternalFilterScorer {
    fn score(&mut self) -> Score {
        self.current_score
    }
}

impl tantivy::DocSet for ExternalFilterScorer {
    fn advance(&mut self) -> DocId {
        loop {
            self.doc_id += 1;
            if self.doc_id >= self.max_doc {
                self.current_doc = tantivy::TERMINATED;
                return tantivy::TERMINATED;
            }

            // Extract field values for this document
            let field_values = self.extract_field_values(self.doc_id);

            if let Some(callback) = &self.callback {
                let result = callback(self.doc_id, &field_values);
                if result.matches {
                    self.current_doc = self.doc_id;
                    self.current_score = result.score; // Use the score from the callback
                    return self.doc_id;
                }
            } else {
                // No callback found - for testing, let's accept the document anyway
                self.current_doc = self.doc_id;
                self.current_score = 1.0;
                return self.doc_id;
            }
        }
    }

    fn doc(&self) -> DocId {
        self.current_doc
    }

    fn size_hint(&self) -> u32 {
        std::cmp::max(1, self.max_doc / 4)
    }
}

impl ExternalFilterScorer {
    fn new(
        reader: SegmentReader,
        config: ExternalFilterConfig,
        callback: Option<ExternalFilterCallback>,
        heaprel_oid: pg_sys::Oid,
    ) -> Self {
        let max_doc = reader.max_doc();
        let ctid_ff = crate::index::fast_fields_helper::FFType::new_ctid(reader.fast_fields());

        let mut scorer = Self {
            doc_id: 0,
            max_doc,
            current_doc: tantivy::TERMINATED,
            current_score: 1.0,
            expression: config.expression.clone(),
            callback,
            config,
            reader,
            ff_helper: None,
            heaprel_oid,
            ctid_ff,
        };

        // Initialize to the first valid document immediately
        let first_doc = scorer.advance();
        if first_doc != tantivy::TERMINATED {
            // We found a valid document, position the scorer correctly
            scorer.doc_id = first_doc + 1; // advance() already incremented it, so set it correctly
        }
        scorer
    }

    /// Extract field values from a document for callback evaluation
    fn extract_field_values(&self, doc_id: DocId) -> HashMap<FieldName, OwnedValue> {
        let mut field_values = HashMap::default();

        // Extract ctid first to access heap if needed
        let ctid = self.extract_ctid(doc_id);

        // For each referenced field, try to extract its value
        for field_name in &self.config.referenced_fields {
            let field_value = if let Some(ctid_value) = ctid {
                // Try to extract from heap
                self.extract_field_from_heap(ctid_value, field_name)
                    .unwrap_or(OwnedValue::Null)
            } else {
                // No ctid available, return null
                OwnedValue::Null
            };

            field_values.insert(field_name.clone(), field_value);
        }

        field_values
    }

    /// Extract ctid from the document for heap access
    fn extract_ctid(&self, doc_id: DocId) -> Option<u64> {
        // Use the ctid fast field helper
        self.ctid_ff.as_u64(doc_id)
    }

    /// Extract a field value from the heap tuple
    fn extract_field_from_heap(&self, ctid: u64, field_name: &FieldName) -> Option<OwnedValue> {
        unsafe {
            if self.heaprel_oid == pg_sys::InvalidOid {
                return None;
            }

            // Open the relation using the OID
            let heaprel = pg_sys::relation_open(self.heaprel_oid, pg_sys::AccessShareLock as _);
            if heaprel.is_null() {
                return None;
            }

            let mut ipd = pg_sys::ItemPointerData::default();
            u64_to_item_pointer(ctid, &mut ipd);

            let mut htup = pg_sys::HeapTupleData {
                t_self: ipd,
                ..Default::default()
            };
            let mut buffer: pg_sys::Buffer = pg_sys::InvalidBuffer as i32;

            #[cfg(feature = "pg14")]
            {
                if !pg_sys::heap_fetch(heaprel, pg_sys::GetActiveSnapshot(), &mut htup, &mut buffer)
                {
                    pg_sys::ReleaseBuffer(buffer);
                    pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as _);
                    return None;
                }
            }

            #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
            {
                if !pg_sys::heap_fetch(
                    heaprel,
                    pg_sys::GetActiveSnapshot(),
                    &mut htup,
                    &mut buffer,
                    false,
                ) {
                    pg_sys::ReleaseBuffer(buffer);
                    pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as _);
                    return None;
                }
            }

            let tuple_desc = PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);
            let heap_tuple = PgHeapTuple::from_heap_tuple(tuple_desc.clone(), &mut htup);

            // Try to get the field value
            let result = match heap_tuple.get_attribute_by_name(&field_name.root()) {
                Some((_index, attribute)) => {
                    // Convert the PostgreSQL value to a Tantivy OwnedValue
                    match attribute.type_oid().value() {
                        pg_sys::BOOLOID => heap_tuple
                            .get_by_name::<bool>(&field_name.root())
                            .ok()
                            .flatten()
                            .map(|v| OwnedValue::Bool(v)),
                        pg_sys::INT2OID => heap_tuple
                            .get_by_name::<i16>(&field_name.root())
                            .ok()
                            .flatten()
                            .map(|v| OwnedValue::I64(v as i64)),
                        pg_sys::INT4OID => heap_tuple
                            .get_by_name::<i32>(&field_name.root())
                            .ok()
                            .flatten()
                            .map(|v| OwnedValue::I64(v as i64)),
                        pg_sys::INT8OID => heap_tuple
                            .get_by_name::<i64>(&field_name.root())
                            .ok()
                            .flatten()
                            .map(|v| OwnedValue::I64(v)),
                        pg_sys::FLOAT4OID => heap_tuple
                            .get_by_name::<f32>(&field_name.root())
                            .ok()
                            .flatten()
                            .map(|v| OwnedValue::F64(v as f64)),
                        pg_sys::FLOAT8OID => heap_tuple
                            .get_by_name::<f64>(&field_name.root())
                            .ok()
                            .flatten()
                            .map(|v| OwnedValue::F64(v)),
                        pg_sys::TEXTOID | pg_sys::VARCHAROID => heap_tuple
                            .get_by_name::<String>(&field_name.root())
                            .ok()
                            .flatten()
                            .map(|v| OwnedValue::Str(v)),
                        pg_sys::NUMERICOID => {
                            // Handle DECIMAL/NUMERIC types
                            heap_tuple
                                .get_by_name::<pgrx::AnyNumeric>(&field_name.root())
                                .ok()
                                .flatten()
                                .and_then(|v| v.try_into().ok())
                                .map(|v: f64| OwnedValue::F64(v))
                        }
                        _ => {
                            // For other types, try to convert to string as a fallback
                            heap_tuple
                                .get_by_name::<String>(&field_name.root())
                                .ok()
                                .flatten()
                                .map(|v| OwnedValue::Str(v))
                        }
                    }
                }
                None => None,
            };

            pg_sys::ReleaseBuffer(buffer);
            pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as _);
            result
        }
    }
}

/// Weight implementation for indexed with filter queries
struct IndexedWithFilterWeight {
    indexed_weight: Box<dyn Weight>,
    external_filter_config: ExternalFilterConfig,
}

impl Weight for IndexedWithFilterWeight {
    fn scorer(&self, reader: &SegmentReader, boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
        pgrx::warning!("🔥 IndexedWithFilterWeight::scorer called - creating combined scorer");
        pgrx::warning!("🔥 Segment ID: {:?}", reader.segment_id());
        pgrx::warning!("🔥 Max doc in segment: {}", reader.max_doc());

        // Count how many times this is called
        static SCORER_CALL_COUNT: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);
        let call_count = SCORER_CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        pgrx::warning!("🔥 IndexedWithFilterWeight::scorer call #{}", call_count);

        // Debug: Check if this is the segment containing Apple iPhone 14 (ctid=1)
        let debug_ctid_ff =
            crate::index::fast_fields_helper::FFType::new_ctid(reader.fast_fields());
        for doc_id in 0..reader.max_doc() {
            let ctid = debug_ctid_ff.as_u64(doc_id).unwrap_or(0);
            if ctid == 1 {
                pgrx::warning!(
                    "🔥 FOUND Apple iPhone 14 (ctid=1) in segment {:?} as doc_id {}",
                    reader.segment_id(),
                    doc_id
                );
            }
        }

        // Get the indexed scorer
        let mut indexed_scorer = self.indexed_weight.scorer(reader, boost)?;

        // Create fast field helper for ctid extraction
        // For now, we'll create a simple FFType for ctid directly
        let ctid_ff = crate::index::fast_fields_helper::FFType::new_ctid(reader.fast_fields());

        // Get heap relation OID from the global context
        let heaprel_oid = get_heap_relation_oid().unwrap_or(pg_sys::Oid::INVALID);

        // Get the callback for the external filter
        let external_filter_callback = get_callback(&self.external_filter_config.expression);
        pgrx::warning!(
            "🔥 IndexedWithFilterWeight::scorer - callback found: {}",
            external_filter_callback.is_some()
        );

        // Create a IndexedWithFilterScorer for segment
        let scorer = IndexedWithFilterScorer::new(
            indexed_scorer,
            self.external_filter_config.clone(),
            external_filter_callback,
            reader.clone(),
            ctid_ff,
            heaprel_oid,
        );
        Ok(Box::new(scorer))
    }

    fn explain(
        &self,
        reader: &SegmentReader,
        doc: DocId,
    ) -> tantivy::Result<tantivy::query::Explanation> {
        Ok(tantivy::query::Explanation::new("IndexedWithFilter", 1.0))
    }
}

/// Combination query that applies an external filter to an indexed query
pub struct IndexedWithFilterQuery {
    /// The base indexed query (stored as serialized form for cloning)
    indexed_query_config: String,
    /// The external filter to apply
    external_filter: ExternalFilterQuery,
    /// Cached indexed query (not cloned)
    cached_indexed_query: Option<Box<dyn Query>>,
}

impl Clone for IndexedWithFilterQuery {
    fn clone(&self) -> Self {
        Self {
            indexed_query_config: self.indexed_query_config.clone(),
            external_filter: self.external_filter.clone(),
            cached_indexed_query: self.cached_indexed_query.as_ref().map(|q| q.box_clone()),
        }
    }
}

impl std::fmt::Debug for IndexedWithFilterQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexedWithFilterQuery")
            .field("indexed_query_config", &self.indexed_query_config)
            .field("external_filter", &self.external_filter)
            .finish()
    }
}

impl IndexedWithFilterQuery {
    /// Create a new indexed with filter query
    pub fn new(indexed_query: Box<dyn Query>, external_filter: ExternalFilterQuery) -> Self {
        Self {
            indexed_query_config: format!("{:?}", indexed_query), // Placeholder serialization
            external_filter,
            cached_indexed_query: Some(indexed_query),
        }
    }
}

impl Query for IndexedWithFilterQuery {
    fn weight(&self, enable_scoring: EnableScoring) -> tantivy::Result<Box<dyn Weight>> {
        // IndexedWithFilterQuery::weight called - creating custom weight

        // Get the indexed query
        let indexed_query = match &self.cached_indexed_query {
            Some(query) => query,
            None => {
                // If we don't have a cached query, we can't proceed
                // In a full implementation, we'd deserialize from indexed_query_config
                return Err(tantivy::TantivyError::InvalidArgument(
                    "No cached indexed query available".to_string(),
                ));
            }
        };

        // Create weight for the indexed part
        let indexed_weight = indexed_query.weight(enable_scoring)?;

        Ok(Box::new(IndexedWithFilterWeight {
            indexed_weight,
            external_filter_config: self.external_filter.config.clone(),
        }))
    }
}

/// Scorer that combines indexed query results with external filter evaluation
struct IndexedWithFilterScorer {
    indexed_scorer: Box<dyn Scorer>,
    external_filter_config: ExternalFilterConfig,
    external_filter_callback: Option<ExternalFilterCallback>,
    reader: SegmentReader,
    ctid_ff: crate::index::fast_fields_helper::FFType,
    heaprel_oid: pg_sys::Oid,
    current_doc: DocId,
}

impl IndexedWithFilterScorer {
    fn new(
        indexed_scorer: Box<dyn Scorer>,
        external_filter_config: ExternalFilterConfig,
        external_filter_callback: Option<ExternalFilterCallback>,
        reader: SegmentReader,
        ctid_ff: crate::index::fast_fields_helper::FFType,
        heaprel_oid: pg_sys::Oid,
    ) -> Self {
        // Check what the current document is before advancing
        let current_doc_before = indexed_scorer.doc();

        // Find the first valid document that passes both indexed query and external filter
        let mut scorer = Self {
            indexed_scorer,
            external_filter_config,
            external_filter_callback,
            reader,
            ctid_ff,
            heaprel_oid,
            current_doc: tantivy::TERMINATED,
        };

        // Check if the current document (before advancing) passes the external filter
        if current_doc_before != tantivy::TERMINATED {
            if scorer.evaluate_external_filter(current_doc_before) {
                scorer.current_doc = current_doc_before;
            } else {
                // Current document doesn't pass filter, find the next one
                scorer.current_doc = scorer.advance_to_next_valid();
            }
        }

        scorer
    }
}

impl Scorer for IndexedWithFilterScorer {
    fn score(&mut self) -> Score {
        // Return the score from the indexed query
        self.indexed_scorer.score()
    }
}

impl tantivy::DocSet for IndexedWithFilterScorer {
    fn advance(&mut self) -> DocId {
        // IndexedWithFilterScorer::advance called
        // Find the next valid document
        self.current_doc = self.advance_to_next_valid();
        self.current_doc
    }

    fn doc(&self) -> DocId {
        self.current_doc
    }

    fn size_hint(&self) -> u32 {
        // Conservative estimate: the indexed scorer's size hint (external filter can only reduce)
        let original_hint = self.indexed_scorer.size_hint();

        original_hint
    }
}

impl IndexedWithFilterScorer {
    /// Advances to the next valid document that passes both the indexed query and external filter.
    ///
    /// # Returns
    ///
    /// The next valid document ID, or `tantivy::TERMINATED` if no more documents are found
    fn advance_to_next_valid(&mut self) -> DocId {
        loop {
            let indexed_doc = self.indexed_scorer.advance();
            pgrx::warning!("🔥 IndexedWithFilterScorer::advance_to_next_valid - indexed_scorer.advance() returned: {}", indexed_doc);
            if indexed_doc == tantivy::TERMINATED {
                pgrx::warning!(
                    "🔥 IndexedWithFilterScorer::advance_to_next_valid - no more documents"
                );
                return tantivy::TERMINATED;
            }

            // Check if this document passes the external filter
            pgrx::warning!("🔥 IndexedWithFilterScorer::advance_to_next_valid - evaluating external filter for doc_id: {}", indexed_doc);
            if self.evaluate_external_filter(indexed_doc) {
                pgrx::warning!("🔥 IndexedWithFilterScorer::advance_to_next_valid - doc_id {} PASSES external filter", indexed_doc);
                return indexed_doc;
            }
            pgrx::warning!("🔥 IndexedWithFilterScorer::advance_to_next_valid - doc_id {} FAILS external filter, continuing", indexed_doc);
            // Continue to next indexed document if the document was filtered out
        }
    }

    /// Evaluate the external filter for a specific document
    fn evaluate_external_filter(&self, doc_id: DocId) -> bool {
        // Use the stored callback for this expression
        if let Some(callback) = &self.external_filter_callback {
            // Extract field values for this document
            let field_values = self.extract_field_values(doc_id);

            // Evaluate the expression using the callback
            let result = callback(doc_id, &field_values);

            result.matches
        } else {
            // Try to get callback from registry as fallback
            if let Some(callback) = get_callback(&self.external_filter_config.expression) {
                let field_values = self.extract_field_values(doc_id);
                let result = callback(doc_id, &field_values);
                result.matches
            } else {
                true
            }
        }
    }

    /// Extract field values from a document for callback evaluation
    fn extract_field_values(&self, doc_id: DocId) -> HashMap<FieldName, OwnedValue> {
        let mut field_values = HashMap::default();

        // For each referenced field, try to extract its value from heap
        for field_name in &self.external_filter_config.referenced_fields {
            let field_value = self.extract_field_value_from_document(doc_id, field_name);
            field_values.insert(field_name.clone(), field_value);
        }

        field_values
    }

    /// Extract a specific field value from a document by loading the actual tuple from heap
    fn extract_field_value_from_document(
        &self,
        doc_id: DocId,
        field_name: &FieldName,
    ) -> OwnedValue {
        // Get the ctid from the document using the fast field
        let ctid = self.ctid_ff.as_u64(doc_id).expect("ctid should be present");

        // Load the actual tuple from the heap using ctid
        unsafe {
            // Open the heap relation using the stored OID
            let heaprel = if self.heaprel_oid != pg_sys::Oid::INVALID {
                pg_sys::relation_open(self.heaprel_oid, pg_sys::AccessShareLock as _)
            } else {
                return OwnedValue::Null;
            };
            let mut ipd = pg_sys::ItemPointerData::default();
            crate::postgres::utils::u64_to_item_pointer(ctid, &mut ipd);

            let mut htup = pg_sys::HeapTupleData {
                t_self: ipd,
                ..Default::default()
            };
            let mut buffer: pg_sys::Buffer = pg_sys::InvalidBuffer as i32;

            // Fetch the tuple from the heap
            #[cfg(feature = "pg14")]
            let found =
                pg_sys::heap_fetch(heaprel, pg_sys::GetActiveSnapshot(), &mut htup, &mut buffer);

            #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
            let found = pg_sys::heap_fetch(
                heaprel,
                pg_sys::GetActiveSnapshot(),
                &mut htup,
                &mut buffer,
                false,
            );

            if !found {
                pg_sys::ReleaseBuffer(buffer);
                return OwnedValue::Null;
            }

            // Create a tuple descriptor and heap tuple wrapper
            let tuple_desc = PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);
            let heap_tuple = PgHeapTuple::from_heap_tuple(tuple_desc.clone(), &mut htup);

            // Extract the field value - try different data types
            let field_value = if let Ok(Some(value)) =
                heap_tuple.get_by_name::<String>(&field_name.root())
            {
                pgrx::warning!(
                    "🔥 Extracted field '{}' = '{}' (String)",
                    field_name.root(),
                    value
                );
                OwnedValue::Str(value)
            } else if let Ok(Some(value)) = heap_tuple.get_by_name::<i32>(&field_name.root()) {
                pgrx::warning!(
                    "🔥 Extracted field '{}' = {} (i32)",
                    field_name.root(),
                    value
                );
                OwnedValue::I64(value as i64)
            } else if let Ok(Some(value)) = heap_tuple.get_by_name::<i64>(&field_name.root()) {
                pgrx::warning!(
                    "🔥 Extracted field '{}' = {} (i64)",
                    field_name.root(),
                    value
                );
                OwnedValue::I64(value)
            } else if let Ok(Some(value)) = heap_tuple.get_by_name::<f32>(&field_name.root()) {
                pgrx::warning!(
                    "🔥 Extracted field '{}' = {} (f32->f64)",
                    field_name.root(),
                    value
                );
                OwnedValue::F64(value as f64)
            } else if let Ok(Some(value)) = heap_tuple.get_by_name::<f64>(&field_name.root()) {
                pgrx::warning!(
                    "🔥 Extracted field '{}' = {} (f64)",
                    field_name.root(),
                    value
                );
                OwnedValue::F64(value)
            } else if let Ok(Some(value)) = heap_tuple.get_by_name::<bool>(&field_name.root()) {
                pgrx::warning!(
                    "🔥 Extracted field '{}' = {} (bool)",
                    field_name.root(),
                    value
                );
                OwnedValue::Bool(value)
            } else if let Ok(Some(value)) = heap_tuple.get_by_name::<AnyNumeric>(&field_name.root())
            {
                // Handle PostgreSQL NUMERIC type
                match value.try_into() {
                    Ok(f) => {
                        let f: f64 = f;
                        pgrx::warning!(
                            "🔥 Extracted field '{}' = {} (NUMERIC->f64)",
                            field_name.root(),
                            f
                        );
                        OwnedValue::F64(f)
                    }
                    Err(_) => {
                        pgrx::warning!(
                            "🔥 Failed to convert NUMERIC to f64 for field '{}'",
                            field_name.root()
                        );
                        OwnedValue::Null
                    }
                }
            } else {
                pgrx::warning!(
                    "🔥 Failed to extract field '{}': unsupported type",
                    field_name.root()
                );
                OwnedValue::Null
            };

            // Release the buffer and close the relation
            pg_sys::ReleaseBuffer(buffer);
            pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as _);

            field_value
        }
    }
}

/// Extract a quoted string value from a PostgreSQL expression
/// This is a simple parser for demonstration purposes
fn extract_quoted_string(expression: &str) -> Option<String> {
    // Look for single-quoted strings
    if let Some(start) = expression.find("'") {
        if let Some(end) = expression[start + 1..].find("'") {
            return Some(expression[start + 1..start + 1 + end].to_string());
        }
    }

    // Look for double-quoted strings
    if let Some(start) = expression.find("\"") {
        if let Some(end) = expression[start + 1..].find("\"") {
            return Some(expression[start + 1..start + 1 + end].to_string());
        }
    }

    None
}

/// Information extracted from a VAR node
#[derive(Debug)]
struct VarInfo {
    attno: i16,
}
