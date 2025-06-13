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

use crate::api::{FieldName, HashMap};
use crate::index::fast_fields_helper::FFHelper;
use crate::postgres::utils::u64_to_item_pointer;
use pgrx::heap_tuple::PgHeapTuple;
use pgrx::{pg_sys, FromDatum, IntoDatum, PgTupleDesc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tantivy::query::{EnableScoring, Query, Scorer, Weight};
use tantivy::schema::OwnedValue;
use tantivy::DocAddress;
use tantivy::{DocId, Score, SegmentReader};

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
pub type ExternalFilterCallback =
    Arc<dyn Fn(DocId, &HashMap<FieldName, OwnedValue>) -> bool + Send + Sync>;

/// Manager for PostgreSQL expression evaluation callbacks
/// This handles the creation and evaluation of PostgreSQL expressions
pub struct CallbackManager {
    /// Serialized expression for recreation in worker processes
    expression: String,
    /// Mapping from attribute numbers to field names
    attno_map: HashMap<pg_sys::AttrNumber, FieldName>,
    /// Cached expression state (not thread-safe, recreated per thread)
    expr_state: Option<*mut pg_sys::ExprState>,
    /// Cached expression context (not thread-safe, recreated per thread)
    expr_context: Option<*mut pg_sys::ExprContext>,
}

// Implement Send and Sync manually since we're only storing serialized data
// The expr_state and expr_context are recreated per thread
unsafe impl Send for CallbackManager {}
unsafe impl Sync for CallbackManager {}

impl CallbackManager {
    /// Create a new callback manager with serialized expression
    pub fn new(expression: String, attno_map: HashMap<pg_sys::AttrNumber, FieldName>) -> Self {
        Self {
            expression,
            attno_map,
            expr_state: None,
            expr_context: None,
        }
    }

    /// Check if the callback manager is initialized for the current thread
    pub fn is_initialized(&self) -> bool {
        self.expr_state.is_some() && self.expr_context.is_some()
    }

    /// Initialize the expression state and context for the current thread
    pub unsafe fn initialize(&mut self) -> Result<(), String> {
        // Parse the expression string back to a PostgreSQL node
        let expr_str: &str = &self.expression;
        let expression_cstr = std::ffi::CString::new(expr_str)
            .map_err(|_| "Failed to create CString from expression")?;
        let expr_node = pg_sys::stringToNode(expression_cstr.as_ptr());

        if expr_node.is_null() {
            return Err("Failed to parse expression".to_string());
        }

        // Create expression state and context
        let expr_state = pg_sys::ExecInitExpr(expr_node.cast(), std::ptr::null_mut());
        let expr_context = pg_sys::CreateStandaloneExprContext();

        if expr_state.is_null() || expr_context.is_null() {
            return Err("Failed to initialize expression state or context".to_string());
        }

        self.expr_state = Some(expr_state);
        self.expr_context = Some(expr_context);

        Ok(())
    }

    /// Evaluate the expression with the provided field values
    pub unsafe fn evaluate_expression(
        &mut self,
        field_values: &HashMap<FieldName, OwnedValue>,
    ) -> Result<bool, String> {
        let expr_state = self.expr_state.ok_or("Expression state not initialized")?;
        let expr_context = self
            .expr_context
            .ok_or("Expression context not initialized")?;

        // For now, we'll implement a simple evaluation that always returns true
        // In a full implementation, this would:
        // 1. Set up the expression context with the field values
        // 2. Evaluate the expression using ExecEvalExpr
        // 3. Convert the result to a boolean

        // TODO: Implement proper field value binding and expression evaluation
        // This requires setting up the expression context's tuple slot with the field values

        let mut is_null = false;
        let result = pg_sys::ExecEvalExpr(expr_state, expr_context, &mut is_null);

        if is_null {
            Ok(false)
        } else {
            // Convert the result to boolean
            match pg_sys::DatumGetBool(result) {
                true => Ok(true),
                false => Ok(false),
            }
        }
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
    expression: String,
    attno_map: HashMap<pg_sys::AttrNumber, FieldName>,
) -> ExternalFilterCallback {
    let callback_manager = Arc::new(std::sync::Mutex::new(CallbackManager::new(
        expression, attno_map,
    )));

    Arc::new(
        move |doc_id: DocId, field_values: &HashMap<FieldName, OwnedValue>| {
            // Lock the callback manager for thread-safe access
            let mut manager = match callback_manager.lock() {
                Ok(manager) => manager,
                Err(_) => {
                    // If we can't lock, assume the expression doesn't match
                    return false;
                }
            };

            // Ensure the manager is initialized for this thread
            if !manager.is_initialized() {
                unsafe {
                    if let Err(_) = manager.initialize() {
                        // If initialization fails, assume the expression doesn't match
                        return false;
                    }
                }
            }

            // Evaluate the expression with the provided field values
            unsafe {
                match manager.evaluate_expression(field_values) {
                    Ok(result) => result,
                    Err(_) => {
                        // If evaluation fails, assume the expression doesn't match
                        false
                    }
                }
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
    /// The callback will be set later during execution
    pub fn new(config: ExternalFilterConfig) -> Self {
        Self {
            config,
            callback: None,
        }
    }

    /// Create a new external filter query with both configuration and callback
    pub fn with_callback<F>(config: ExternalFilterConfig, callback: F) -> Self
    where
        F: Fn(DocId, &HashMap<FieldName, OwnedValue>) -> bool + Send + Sync + 'static,
    {
        Self {
            config,
            callback: Some(Arc::new(callback)),
        }
    }

    /// Set the callback function for this query
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: Fn(DocId, &HashMap<FieldName, OwnedValue>) -> bool + Send + Sync + 'static,
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
    fn scorer(&self, reader: &SegmentReader, boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
        Ok(Box::new(ExternalFilterScorer {
            config: self.config.clone(),
            callback: self.callback.clone(),
            reader: reader.clone(),
            boost,
            doc_id: 0,
            max_doc: reader.max_doc(),
            ff_helper: None,
            heaprel_oid: pg_sys::InvalidOid,
        }))
    }

    fn explain(
        &self,
        _reader: &SegmentReader,
        _doc: DocId,
    ) -> tantivy::Result<tantivy::query::Explanation> {
        Ok(tantivy::query::Explanation::new("ExternalFilter", 1.0))
    }
}

/// Scorer implementation for external filter queries
struct ExternalFilterScorer {
    config: ExternalFilterConfig,
    callback: Option<ExternalFilterCallback>,
    reader: SegmentReader,
    boost: Score,
    doc_id: DocId,
    max_doc: DocId,
    ff_helper: Option<FFHelper>,
    heaprel_oid: pg_sys::Oid, // Store OID instead of raw pointer for thread safety
}

impl Scorer for ExternalFilterScorer {
    fn score(&mut self) -> Score {
        self.boost
    }
}

impl tantivy::DocSet for ExternalFilterScorer {
    fn advance(&mut self) -> DocId {
        // For now, let's make this always return TERMINATED to test if filtering is working
        // This should result in no documents being returned when external filters are used
        tantivy::TERMINATED
    }

    fn doc(&self) -> DocId {
        tantivy::TERMINATED
    }

    fn size_hint(&self) -> u32 {
        0
    }
}

impl ExternalFilterScorer {
    /// Set the heap relation OID for field value extraction
    pub fn set_heaprel_oid(&mut self, heaprel_oid: pg_sys::Oid) {
        self.heaprel_oid = heaprel_oid;
    }

    /// Set the fast field helper for field value extraction
    pub fn set_ff_helper(&mut self, ff_helper: FFHelper) {
        self.ff_helper = Some(ff_helper);
    }

    /// Extract field values for the given document
    fn extract_field_values(&self, doc_id: DocId) -> HashMap<FieldName, OwnedValue> {
        let mut field_values = HashMap::default();

        // Try to extract values from fast fields first
        if let Some(ref ff_helper) = self.ff_helper {
            for (field_idx, field_name) in self.config.referenced_fields.iter().enumerate() {
                let doc_address = DocAddress::new(0, doc_id); // Assuming segment 0 for now

                if let Some(tantivy_value) = ff_helper.value(field_idx, doc_address) {
                    field_values.insert(field_name.clone(), tantivy_value.0);
                }
            }
        }

        // If we couldn't get all values from fast fields, try heap access
        if field_values.len() < self.config.referenced_fields.len()
            && self.heaprel_oid != pg_sys::InvalidOid
        {
            // Extract ctid for heap access
            let ctid = self.extract_ctid(doc_id);
            if let Some(ctid) = ctid {
                for field_name in &self.config.referenced_fields {
                    if !field_values.contains_key(field_name) {
                        if let Some(value) = self.extract_field_from_heap(ctid, field_name) {
                            field_values.insert(field_name.clone(), value);
                        }
                    }
                }
            }
        }

        field_values
    }

    /// Extract ctid from the document for heap access
    fn extract_ctid(&self, doc_id: DocId) -> Option<u64> {
        // Try to get ctid from fast fields
        let fast_fields = self.reader.fast_fields();
        if let Ok(ctid_ff) = fast_fields.u64("ctid") {
            ctid_ff.first(doc_id)
        } else {
            None
        }
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
            cached_indexed_query: None, // Don't clone the cached query
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
        // For now, just use the external filter
        // In a full implementation, this would combine both queries
        self.external_filter.weight(enable_scoring)
    }
}
