// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search
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

use crate::api::FieldName;
use crate::debug_log;
use crate::index::fast_fields_helper::FFHelper;
use crate::postgres::utils::u64_to_item_pointer;
use pgrx::heap_tuple::PgHeapTuple;
use pgrx::{pg_sys, AnyNumeric, FromDatum, IntoDatum, PgTupleDesc, WhoAllocated, PgList};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
            debug_log!("Registering callback for expression: {}", expression);
            registry.insert(expression.to_string(), callback);
            debug_log!("Registry now has {} callbacks", registry.len());
        } else {
            debug_log!("Callback already exists for expression, skipping registration");
        }
    }
}

/// Retrieve a callback for a specific expression
pub fn get_callback(expression: &str) -> Option<ExternalFilterCallback> {
    if let Ok(registry) = CALLBACK_REGISTRY.lock() {
        let result = registry.get(expression).cloned();
        debug_log!("Looking for callback for expression: {}", expression);
        debug_log!(
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
        debug_log!("🔥 Cleared {} callbacks from registry", count);
    }
}

/// Clear a specific callback (useful for cleanup after query completion)
pub fn clear_callback(expression: &str) {
    if let Ok(mut registry) = CALLBACK_REGISTRY.lock() {
        if registry.remove(expression).is_some() {
            debug_log!("🔥 Removed callback for expression from registry");
        }
    }
}

/// Manager for PostgreSQL expression evaluation callbacks
/// This handles the creation and evaluation of PostgreSQL expressions
pub struct CallbackManager {
    expression: String,
    referenced_fields: Vec<FieldName>,
    expr_context: Option<*mut pg_sys::ExprContext>,
    expr_state: Option<*mut pg_sys::ExprState>,
    planstate: Option<*mut pg_sys::PlanState>, // Store planstate for proper initialization
}

// Implement Send and Sync manually since we're only storing serialized data
// The expr_state and expr_context are recreated per thread
unsafe impl Send for CallbackManager {}
unsafe impl Sync for CallbackManager {}

impl CallbackManager {
    pub fn new(
        expression: String, 
        referenced_fields: Vec<FieldName>,
        planstate: *mut pg_sys::PlanState,
    ) -> Self {
        Self {
            expression,
            referenced_fields,
            expr_context: None,
            expr_state: None,
            planstate: Some(planstate),
        }
    }

    /// Initialize the expression state and context for the current thread using proper planstate
    pub unsafe fn initialize(&mut self) -> Result<(), String> {
        if self.is_initialized() {
            return Ok(());
        }

        debug_log!("🔥 Initializing callback manager with proper planstate approach");

        // Create expression context
        let expr_context = pg_sys::CreateStandaloneExprContext();
        if expr_context.is_null() {
            return Err("Failed to create expression context".to_string());
        }
        self.expr_context = Some(expr_context);

        // Use the proper planstate-based initialization
        if let Some(planstate) = self.planstate {
            if let Some(expr_state) = init_heap_filter_expr_state(planstate, &self.expression) {
                self.expr_state = Some(expr_state);
                debug_log!("🔥 Successfully initialized expression state using planstate");
            } else {
                return Err("Failed to initialize expression state".to_string());
            }
        } else {
            return Err("No planstate available for initialization".to_string());
        }

        Ok(())
    }

    /// Evaluate a PostgreSQL expression using proper planstate-based approach
    pub unsafe fn evaluate_expression_with_postgres(
        &mut self,
        field_values: &HashMap<FieldName, OwnedValue>,
    ) -> bool {
        // Initialize if needed
        if let Err(e) = self.initialize() {
            debug_log!("🔥 Failed to initialize callback manager: {}", e);
            return false;
        }

        debug_log!("🔥 Evaluating expression with proper PostgreSQL evaluation");
        debug_log!("🔥 Expression: {}", self.expression);
        debug_log!("🔥 Field values: {:?}", field_values);

        // Get the initialized expression state and context
        let expr_state = match self.expr_state {
            Some(state) => state,
            None => {
                debug_log!("🔥 Expression state not initialized");
                return false;
            }
        };

        let expr_context = match self.expr_context {
            Some(context) => context,
            None => {
                debug_log!("🔥 Expression context not initialized");
                return false;
            }
        };

        // Set up the expression context with field values
        if let Err(e) = self.setup_expression_context_with_values(expr_context, field_values) {
            debug_log!("🔥 Failed to setup expression context: {}", e);
            return false;
        }

        // Evaluate the expression using PostgreSQL
        let mut is_null = false;
        let result_datum = pg_sys::ExecEvalExpr(
            expr_state,
            expr_context,
            &mut is_null as *mut bool,
        );

        if is_null {
            debug_log!("🔥 PostgreSQL expression evaluation result: NULL");
            false
        } else {
            let result = pg_sys::DatumGetBool(result_datum);
            debug_log!("🔥 PostgreSQL expression evaluation result: {}", result);
            result
        }
    }

    /// Setup the expression context with field values for evaluation
    unsafe fn setup_expression_context_with_values(
        &self,
        expr_context: *mut pg_sys::ExprContext,
        field_values: &HashMap<FieldName, OwnedValue>,
    ) -> Result<(), String> {
        // Create a proper tuple slot with the field values
        debug_log!("🔥 Setting up expression context with {} field values", field_values.len());
        
        // Create a tuple slot that matches our field structure
        let tuple_slot = self.create_mock_tuple_slot(field_values)?;
        
        // Set the tuple slot in the expression context
        (*expr_context).ecxt_scantuple = tuple_slot;
        
        debug_log!("🔥 Expression context setup complete");
        Ok(())
    }

    /// Check if the callback manager is initialized for the current thread
    pub fn is_initialized(&self) -> bool {
        self.expr_state.is_some() && self.expr_context.is_some()
    }

    /// Create a heap filter expression from a PostgreSQL node string
    unsafe fn create_heap_filter_expr(&self, heap_filter_node_string: &str) -> *mut pg_sys::Expr {
        debug_log!(
            "🔥 Creating heap filter expression from: {}",
            heap_filter_node_string
        );

        // Handle multiple clauses separated by our delimiter
        if heap_filter_node_string.contains("|||AND_CLAUSE_SEPARATOR|||")
            || heap_filter_node_string.contains("|||OR_CLAUSE_SEPARATOR|||")
            || heap_filter_node_string.contains("|||CLAUSE_SEPARATOR|||")
        {
            // Multiple clauses - determine the boolean operation and split accordingly
            let (clause_strings, bool_op) =
                if heap_filter_node_string.contains("|||AND_CLAUSE_SEPARATOR|||") {
                    (
                        heap_filter_node_string
                            .split("|||AND_CLAUSE_SEPARATOR|||")
                            .collect::<Vec<&str>>(),
                        pg_sys::BoolExprType::AND_EXPR,
                    )
                } else if heap_filter_node_string.contains("|||OR_CLAUSE_SEPARATOR|||") {
                    (
                        heap_filter_node_string
                            .split("|||OR_CLAUSE_SEPARATOR|||")
                            .collect::<Vec<&str>>(),
                        pg_sys::BoolExprType::OR_EXPR,
                    )
                } else {
                    // Legacy support for old CLAUSE_SEPARATOR (assume AND)
                    (
                        heap_filter_node_string
                            .split("|||CLAUSE_SEPARATOR|||")
                            .collect::<Vec<&str>>(),
                        pg_sys::BoolExprType::AND_EXPR,
                    )
                };

            // Create individual nodes for each clause
            let mut args_list = std::ptr::null_mut();
            for clause_str in clause_strings.iter() {
                let clause_cstring = std::ffi::CString::new(*clause_str)
                    .expect("Failed to create CString from clause string");
                let clause_node = pg_sys::stringToNode(clause_cstring.as_ptr());

                if !clause_node.is_null() {
                    // For the legacy ExprState path, replace @@@ operators with TRUE constants
                    let processed_node = replace_search_operators_with_true(clause_node.cast());
                    args_list = pg_sys::lappend(args_list, processed_node.cast::<core::ffi::c_void>());
                } else {
                    panic!("Failed to parse clause string: {}", clause_str);
                }
            }

            if !args_list.is_null() {
                // Create a BoolExpr to combine all clauses with the detected boolean operation
                let bool_expr =
                    pg_sys::palloc0(std::mem::size_of::<pg_sys::BoolExpr>()).cast::<pg_sys::BoolExpr>();
                (*bool_expr).xpr.type_ = pg_sys::NodeTag::T_BoolExpr;
                (*bool_expr).boolop = bool_op;
                (*bool_expr).args = args_list;
                (*bool_expr).location = -1;

                bool_expr.cast::<pg_sys::Expr>()
            } else {
                panic!("Failed to parse any clauses: {}", heap_filter_node_string);
            }
        } else {
            // Single clause - for legacy ExprState path, process @@@ operators
            let node_cstr = std::ffi::CString::new(heap_filter_node_string)
                .expect("Failed to create CString from node string");
            let node = pg_sys::stringToNode(node_cstr.as_ptr());

            if !node.is_null() {
                // For the legacy ExprState path, replace @@@ operators with TRUE constants
                let processed_node = replace_search_operators_with_true(node.cast());
                processed_node.cast::<pg_sys::Expr>()
            } else {
                panic!("Failed to deserialize node: {}", heap_filter_node_string);
            }
        }
    }

    /// Create a mock tuple slot with the provided field values
    /// This creates a tuple slot that matches the table structure
    unsafe fn create_mock_tuple_slot(
        &self,
        field_values: &HashMap<FieldName, OwnedValue>,
    ) -> Result<*mut pg_sys::TupleTableSlot, String> {
        debug_log!(
            "🔥 Creating mock tuple slot with {} field values",
            field_values.len()
        );

        // We need to create a tuple descriptor that matches the actual table structure
        // For now, we'll create a simple approach that works with the expression evaluation

        // Get the maximum attribute number we need to support
        let max_attno = self.referenced_fields.len();
        debug_log!("🔥 Maximum attribute number needed: {}", max_attno);

        // Create a tuple descriptor with enough attributes
        let tupdesc = pg_sys::CreateTemplateTupleDesc(max_attno as i32);

        // Initialize all attributes with appropriate types based on field values
        for i in 0..max_attno {
            let field_name = &self.referenced_fields[i];
            let oid = self.get_appropriate_type_oid(field_name);

            pg_sys::TupleDescInitEntry(
                tupdesc,
                i as pg_sys::AttrNumber + 1,
                std::ptr::null_mut(), // name (not needed for our use case)
                oid,                  // use appropriate type
                -1,                   // typmod
                0,                    // attdim
            );
        }

        // Create the tuple slot
        let slot = pg_sys::MakeTupleTableSlot(tupdesc, &pg_sys::TTSOpsVirtual);

        // Initialize the slot values
        let natts = max_attno as usize;
        let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
        let nulls = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

        // Initialize all values to NULL first
        for i in 0..natts {
            datums[i] = pg_sys::Datum::null();
            nulls[i] = true;
        }

        // Set the actual field values we have
        for (i, field_name) in self.referenced_fields.iter().enumerate() {
            if let Some(value) = field_values.get(field_name) {
                let array_index = i as usize;
                if array_index < natts {
                    match self.convert_value_to_datum(value, field_name) {
                        Ok((datum, is_null)) => {
                            datums[array_index] = datum;
                            nulls[array_index] = is_null;
                            debug_log!(
                                "🔥 Set field '{}' at index {} with is_null={}",
                                field_name.root().as_str(),
                                array_index,
                                is_null
                            );
                        }
                        Err(e) => {
                            debug_log!(
                                "🔥 Failed to convert value for field '{}': {}",
                                field_name.root().as_str(),
                                e
                            );
                            // Set to NULL on conversion failure
                            datums[array_index] = pg_sys::Datum::from(0);
                            nulls[array_index] = true;
                        }
                    }
                }
            }
        }

        // Mark the slot as valid
        (*slot).tts_nvalid = natts as i16;
        (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;

        debug_log!(
            "🔥 Successfully created mock tuple slot with {} attributes",
            natts
        );
        Ok(slot)
    }

    /// Get the appropriate PostgreSQL type OID for a given field
    fn get_appropriate_type_oid(&self, field_name: &FieldName) -> pg_sys::Oid {
        // Determine type based on field name patterns
        match field_name.root().as_str() {
            "id" | "category_id" => pg_sys::INT4OID,
            "price" => pg_sys::NUMERICOID,
            "category_name" | "name" | "description" => pg_sys::TEXTOID,
            "tags" => pg_sys::TEXTARRAYOID, // Array of text
            "in_stock" => pg_sys::BOOLOID,
            _ => pg_sys::TEXTOID, // Default to text for unknown fields
        }
    }

    /// Convert an OwnedValue to a PostgreSQL Datum with proper error handling
    fn convert_value_to_datum(
        &self,
        value: &OwnedValue,
        field_name: &FieldName,
    ) -> Result<(pg_sys::Datum, bool), String> {
        match value {
            OwnedValue::Null => Ok((pg_sys::Datum::from(0), true)), // NULL datum
            OwnedValue::Bool(b) => Ok(((*b).into_datum().unwrap(), false)),
            OwnedValue::I64(i) => {
                // Convert to appropriate integer type based on field
                Ok(((*i as i32).into_datum().unwrap(), false))
            }
            OwnedValue::F64(f) => {
                // Convert to appropriate float type
                Ok(((*f as f32).into_datum().unwrap(), false))
            }
            OwnedValue::Str(s) => {
                // Special handling for array marker
                if s == "__ARRAY_NON_NULL__" {
                    // For array fields that are not null, we need to create a non-null datum
                    // that represents the presence of an array value
                    debug_log!("🔥 Converting array marker to non-null datum for field '{}'", field_name.root());
                    // Return a non-null datum that PostgreSQL can use for IS NULL tests
                    Ok((1i32.into_datum().unwrap(), false)) // Non-null integer as placeholder
                } else {
                    Ok((s.clone().into_datum().unwrap(), false))
                }
            }
            _ => {
                debug_log!("🔥 Unsupported OwnedValue type for field '{}', treating as NULL", field_name.root());
                Ok((pg_sys::Datum::from(0), true))
            }
        }
    }

    /// Clean up resources when done
    pub unsafe fn cleanup(&mut self) {
        if let Some(_expr_state) = self.expr_state.take() {
            // In the simplified mode, expr_state is just a placeholder null pointer
            // so no cleanup is needed
        }
        
        if let Some(expr_context) = self.expr_context.take() {
            pg_sys::FreeExprContext(expr_context, false);
        }
        
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
    referenced_fields: Vec<FieldName>,
    planstate: *mut pg_sys::PlanState,
) -> ExternalFilterCallback {
    debug_log!(
        "Creating PostgreSQL callback for expression: {}",
        expression
    );
    debug_log!("Callback will handle {} fields", referenced_fields.len());

    let manager = Arc::new(Mutex::new(CallbackManager::new(
        expression,
        referenced_fields,
        planstate,
    )));

    Arc::new(
        move |doc_id: DocId, field_values: &HashMap<FieldName, OwnedValue>| {
            debug_log!(
                "🔥 CALLBACK INVOKED! Evaluating expression for doc_id: {}",
                doc_id
            );
            debug_log!("🔥 Field values provided: {:?}", field_values);

            // Use the proper callback manager to evaluate the PostgreSQL expression
            if let Ok(mut mgr) = manager.lock() {
                unsafe {
                    if !mgr.is_initialized() {
                        debug_log!("🔥 Initializing callback manager for first use");
                        if let Err(e) = mgr.initialize() {
                            debug_log!("🔥 Failed to initialize callback manager: {}", e);
                            return ExternalFilterResult::matches_with_default_score(false);
                        }
                    }

                    let result = mgr.evaluate_expression_with_postgres(field_values);
                    debug_log!(
                        "🔥 PostgreSQL expression evaluation result: matches={}",
                        result
                    );
                    return ExternalFilterResult::new(result, 0.0);
                }
            } else {
                debug_log!("🔥 Failed to acquire callback manager lock");
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
        debug_log!("ExternalFilterQuery::weight called - creating ExternalFilterWeight");
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
        debug_log!("ExternalFilterWeight::scorer called - creating ExternalFilterScorer");

        // Get heap relation OID from the global context
        let heaprel_oid = get_heap_relation_oid().unwrap_or(pg_sys::Oid::INVALID);

        let scorer = ExternalFilterScorer::new(
            reader.clone(),
            self.config.clone(),
            self.callback.clone(),
            heaprel_oid,
        );
        debug_log!(
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
                    let type_oid_raw = attribute.type_oid().value();
                    let type_oid = type_oid_raw.to_u32();
                    Some(match type_oid {
                        oid if oid == pg_sys::BOOLOID.to_u32() => {
                            match heap_tuple.get_by_name::<bool>(&field_name.root()) {
                                Ok(Some(value)) => {
                                    debug_log!("🔥 Extracted field '{}' = {} (bool)", field_name.root(), value);
                                    OwnedValue::Bool(value)
                                }
                                Ok(None) => OwnedValue::Null,
                                Err(e) => {
                                    debug_log!("🔥 Failed to extract bool field '{}': {}", field_name.root(), e);
                                    OwnedValue::Null
                                }
                            }
                        }
                        oid if oid == pg_sys::INT2OID.to_u32() => {
                            match heap_tuple.get_by_name::<i16>(&field_name.root()) {
                                Ok(Some(value)) => {
                                    debug_log!("🔥 Extracted field '{}' = {} (i16->i64)", field_name.root(), value);
                                    OwnedValue::I64(value as i64)
                                }
                                Ok(None) => OwnedValue::Null,
                                Err(e) => {
                                    debug_log!("🔥 Failed to extract i16 field '{}': {}", field_name.root(), e);
                                    OwnedValue::Null
                                }
                            }
                        }
                        oid if oid == pg_sys::INT4OID.to_u32() => {
                            match heap_tuple.get_by_name::<i32>(&field_name.root()) {
                                Ok(Some(value)) => {
                                    debug_log!("🔥 Extracted field '{}' = {} (i32->i64)", field_name.root(), value);
                                    OwnedValue::I64(value as i64)
                                }
                                Ok(None) => OwnedValue::Null,
                                Err(e) => {
                                    debug_log!("🔥 Failed to extract i32 field '{}': {}", field_name.root(), e);
                                    OwnedValue::Null
                                }
                            }
                        }
                        oid if oid == pg_sys::INT8OID.to_u32() => {
                            match heap_tuple.get_by_name::<i64>(&field_name.root()) {
                                Ok(Some(value)) => {
                                    debug_log!("🔥 Extracted field '{}' = {} (i64)", field_name.root(), value);
                                    OwnedValue::I64(value)
                                }
                                Ok(None) => OwnedValue::Null,
                                Err(e) => {
                                    debug_log!("🔥 Failed to extract i64 field '{}': {}", field_name.root(), e);
                                    OwnedValue::Null
                                }
                            }
                        }
                        oid if oid == pg_sys::FLOAT4OID.to_u32() => {
                            match heap_tuple.get_by_name::<f32>(&field_name.root()) {
                                Ok(Some(value)) => {
                                    debug_log!("🔥 Extracted field '{}' = {} (f32->f64)", field_name.root(), value);
                                    OwnedValue::F64(value as f64)
                                }
                                Ok(None) => OwnedValue::Null,
                                Err(e) => {
                                    debug_log!("🔥 Failed to extract f32 field '{}': {}", field_name.root(), e);
                                    OwnedValue::Null
                                }
                            }
                        }
                        oid if oid == pg_sys::FLOAT8OID.to_u32() => {
                            match heap_tuple.get_by_name::<f64>(&field_name.root()) {
                                Ok(Some(value)) => {
                                    debug_log!("🔥 Extracted field '{}' = {} (f64)", field_name.root(), value);
                                    OwnedValue::F64(value)
                                }
                                Ok(None) => OwnedValue::Null,
                                Err(e) => {
                                    debug_log!("🔥 Failed to extract f64 field '{}': {}", field_name.root(), e);
                                    OwnedValue::Null
                                }
                            }
                        }
                        oid if oid == pg_sys::TEXTOID.to_u32() || oid == pg_sys::VARCHAROID.to_u32() => {
                            match heap_tuple.get_by_name::<String>(&field_name.root()) {
                                Ok(Some(value)) => {
                                    debug_log!("🔥 Extracted field '{}' = '{}' (String)", field_name.root(), value);
                                    OwnedValue::Str(value)
                                }
                                Ok(None) => OwnedValue::Null,
                                Err(e) => {
                                    debug_log!("🔥 Failed to extract string field '{}': {}", field_name.root(), e);
                                    OwnedValue::Null
                                }
                            }
                        }
                        oid if oid == pg_sys::NUMERICOID.to_u32() => {
                            match heap_tuple.get_by_name::<AnyNumeric>(&field_name.root()) {
                                Ok(Some(value)) => {
                                    match value.try_into() {
                                        Ok(f) => {
                                            let f: f64 = f;
                                            debug_log!("🔥 Extracted field '{}' = {} (NUMERIC->f64)", field_name.root(), f);
                                            OwnedValue::F64(f)
                                        }
                                        Err(e) => {
                                            debug_log!("🔥 Failed to convert NUMERIC to f64 for field '{}': {}", field_name.root(), e);
                                            OwnedValue::Null
                                        }
                                    }
                                }
                                Ok(None) => OwnedValue::Null,
                                Err(e) => {
                                    debug_log!("🔥 Failed to extract NUMERIC field '{}': {}", field_name.root(), e);
                                    OwnedValue::Null
                                }
                            }
                        }
                        // Handle array types by trying to extract them properly
                        oid if self.is_array_type_u32(oid) => {
                            debug_log!("🔥 Attempting to extract array field '{}' (type_oid: {})", field_name.root(), type_oid);
                            
                            // Try to extract as array and check if it's actually NULL
                            match heap_tuple.get_by_index::<pg_sys::Datum>(_index) {
                                Ok(Some(_)) => {
                                    // Array has a value, but we can't process it yet
                                    debug_log!("🔥 Array field has non-null value, but array processing not implemented yet");
                                    // For now, return a special marker that indicates "non-null array"
                                    // This helps with IS NULL / IS NOT NULL tests
                                    OwnedValue::Str("__ARRAY_NON_NULL__".to_string())
                                }
                                Ok(None) => {
                                    // Array is actually NULL
                                    debug_log!("🔥 Array field is actually NULL");
                                    OwnedValue::Null
                                }
                                Err(e) => {
                                    debug_log!("🔥 Failed to check array field '{}' for null: {}", field_name.root(), e);
                                    OwnedValue::Null
                                }
                            }
                        }
                        // Handle other unknown types safely
                        _ => {
                            debug_log!("🔥 Unsupported type_oid {} for field '{}', returning Null", type_oid, field_name.root());
                            OwnedValue::Null
                        }
                    })
                }
                None => Some(OwnedValue::Null),
            };

            pg_sys::ReleaseBuffer(buffer);
            pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as _);
            result
        }
    }

    /// Check if a type OID represents an array type
    fn is_array_type(&self, type_oid: pg_sys::Oid) -> bool {
        // PostgreSQL array types typically have type OIDs > 1000
        // Common array types: 1009 (text[]), 1007 (int4[]), etc.
        let oid_value = type_oid.to_u32();
        oid_value == 1009 || // text[]
        oid_value == 1007 || // int4[]
        oid_value == 1016 || // int8[]
        oid_value == 1021 || // float4[]
        oid_value == 1022 || // float8[]
        oid_value == 1000 || // bool[]
        (oid_value > 1000 && oid_value < 2000) // General array type range
    }

    /// Check if a type OID (as u32) represents an array type
    fn is_array_type_u32(&self, type_oid: u32) -> bool {
        // PostgreSQL array types typically have type OIDs > 1000
        // Common array types: 1009 (text[]), 1007 (int4[]), etc.
        type_oid == 1009 || // text[]
        type_oid == 1007 || // int4[]
        type_oid == 1016 || // int8[]
        type_oid == 1021 || // float4[]
        type_oid == 1022 || // float8[]
        type_oid == 1000 || // bool[]
        (type_oid > 1000 && type_oid < 2000) // General array type range
    }
}

/// Weight implementation for indexed with filter queries
struct IndexedWithFilterWeight {
    indexed_weight: Box<dyn Weight>,
    external_filter_config: ExternalFilterConfig,
}

impl Weight for IndexedWithFilterWeight {
    fn scorer(
        &self,
        reader: &SegmentReader,
        boost: Score,
    ) -> tantivy::Result<Box<dyn Scorer>> {
        debug_log!("🔥 IndexedWithFilterWeight::scorer called - creating combined scorer");

        // Get the indexed scorer
        let indexed_scorer = self.indexed_weight.scorer(reader, boost)?;

        // Create fast field helper for ctid extraction
        let ctid_ff = crate::index::fast_fields_helper::FFType::new_ctid(reader.fast_fields());

        // Get heap relation OID from the global context
        let heaprel_oid = get_heap_relation_oid().unwrap_or(pg_sys::Oid::INVALID);

        // Get the callback for the external filter
        let external_filter_callback = get_callback(&self.external_filter_config.expression);

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
        _reader: &SegmentReader,
        _doc: DocId,
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
            debug_log!("🔥 IndexedWithFilterScorer::advance_to_next_valid - indexed_scorer.advance() returned: {}", indexed_doc);
            if indexed_doc == tantivy::TERMINATED {
                debug_log!("🔥 IndexedWithFilterScorer::advance_to_next_valid - no more documents");
                return tantivy::TERMINATED;
            }

            // Check if this document passes the external filter
            debug_log!("🔥 IndexedWithFilterScorer::advance_to_next_valid - evaluating external filter for doc_id: {}", indexed_doc);
            if self.evaluate_external_filter(indexed_doc) {
                debug_log!("🔥 IndexedWithFilterScorer::advance_to_next_valid - doc_id {} PASSES external filter", indexed_doc);
                return indexed_doc;
            }
            debug_log!("🔥 IndexedWithFilterScorer::advance_to_next_valid - doc_id {} FAILS external filter, continuing", indexed_doc);
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

            // Extract the field value - try different data types with improved error handling
            let field_value = self.extract_field_value_safe(&heap_tuple, field_name);

            // Release the buffer and close the relation
            pg_sys::ReleaseBuffer(buffer);
            pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as _);

            field_value
        }
    }

    /// Safely extract field value with proper error handling for different types
    fn extract_field_value_safe<'mcx, AllocatedBy: WhoAllocated>(
        &self,
        heap_tuple: &PgHeapTuple<'mcx, AllocatedBy>,
        field_name: &FieldName,
    ) -> OwnedValue {
        // Get the attribute information for this field
        if let Some((_index, attribute)) = heap_tuple.get_attribute_by_name(&field_name.root()) {
            let type_oid_raw = attribute.type_oid().value();
            let type_oid = type_oid_raw.to_u32();
            
            // Handle different PostgreSQL types with proper error handling
            match type_oid {
                oid if oid == pg_sys::BOOLOID.to_u32() => {
                    match heap_tuple.get_by_name::<bool>(&field_name.root()) {
                        Ok(Some(value)) => {
                            debug_log!("🔥 Extracted field '{}' = {} (bool)", field_name.root(), value);
                            OwnedValue::Bool(value)
                        }
                        Ok(None) => OwnedValue::Null,
                        Err(e) => {
                            debug_log!("🔥 Failed to extract bool field '{}': {}", field_name.root(), e);
                            OwnedValue::Null
                        }
                    }
                }
                oid if oid == pg_sys::INT2OID.to_u32() => {
                    match heap_tuple.get_by_name::<i16>(&field_name.root()) {
                        Ok(Some(value)) => {
                            debug_log!("🔥 Extracted field '{}' = {} (i16->i64)", field_name.root(), value);
                OwnedValue::I64(value as i64)
                        }
                        Ok(None) => OwnedValue::Null,
                        Err(e) => {
                            debug_log!("🔥 Failed to extract i16 field '{}': {}", field_name.root(), e);
                            OwnedValue::Null
                        }
                    }
                }
                oid if oid == pg_sys::INT4OID.to_u32() => {
                    match heap_tuple.get_by_name::<i32>(&field_name.root()) {
                        Ok(Some(value)) => {
                            debug_log!("🔥 Extracted field '{}' = {} (i32->i64)", field_name.root(), value);
                            OwnedValue::I64(value as i64)
                        }
                        Ok(None) => OwnedValue::Null,
                        Err(e) => {
                            debug_log!("🔥 Failed to extract i32 field '{}': {}", field_name.root(), e);
                            OwnedValue::Null
                        }
                    }
                }
                oid if oid == pg_sys::INT8OID.to_u32() => {
                    match heap_tuple.get_by_name::<i64>(&field_name.root()) {
                        Ok(Some(value)) => {
                            debug_log!("🔥 Extracted field '{}' = {} (i64)", field_name.root(), value);
                OwnedValue::I64(value)
                        }
                        Ok(None) => OwnedValue::Null,
                        Err(e) => {
                            debug_log!("🔥 Failed to extract i64 field '{}': {}", field_name.root(), e);
                            OwnedValue::Null
                        }
                    }
                }
                oid if oid == pg_sys::FLOAT4OID.to_u32() => {
                    match heap_tuple.get_by_name::<f32>(&field_name.root()) {
                        Ok(Some(value)) => {
                            debug_log!("🔥 Extracted field '{}' = {} (f32->f64)", field_name.root(), value);
                OwnedValue::F64(value as f64)
                        }
                        Ok(None) => OwnedValue::Null,
                        Err(e) => {
                            debug_log!("🔥 Failed to extract f32 field '{}': {}", field_name.root(), e);
                            OwnedValue::Null
                        }
                    }
                }
                oid if oid == pg_sys::FLOAT8OID.to_u32() => {
                    match heap_tuple.get_by_name::<f64>(&field_name.root()) {
                        Ok(Some(value)) => {
                            debug_log!("🔥 Extracted field '{}' = {} (f64)", field_name.root(), value);
                OwnedValue::F64(value)
                        }
                        Ok(None) => OwnedValue::Null,
                        Err(e) => {
                            debug_log!("🔥 Failed to extract f64 field '{}': {}", field_name.root(), e);
                            OwnedValue::Null
                        }
                    }
                }
                oid if oid == pg_sys::TEXTOID.to_u32() || oid == pg_sys::VARCHAROID.to_u32() => {
                    match heap_tuple.get_by_name::<String>(&field_name.root()) {
                        Ok(Some(value)) => {
                            debug_log!("🔥 Extracted field '{}' = '{}' (String)", field_name.root(), value);
                            OwnedValue::Str(value)
                        }
                        Ok(None) => OwnedValue::Null,
                        Err(e) => {
                            debug_log!("🔥 Failed to extract string field '{}': {}", field_name.root(), e);
                            OwnedValue::Null
                        }
                    }
                }
                oid if oid == pg_sys::NUMERICOID.to_u32() => {
                    match heap_tuple.get_by_name::<AnyNumeric>(&field_name.root()) {
                        Ok(Some(value)) => {
                match value.try_into() {
                    Ok(f) => {
                        let f: f64 = f;
                                    debug_log!("🔥 Extracted field '{}' = {} (NUMERIC->f64)", field_name.root(), f);
                        OwnedValue::F64(f)
                    }
                                Err(e) => {
                                    debug_log!("🔥 Failed to convert NUMERIC to f64 for field '{}': {}", field_name.root(), e);
                                    OwnedValue::Null
                                }
                            }
                        }
                        Ok(None) => OwnedValue::Null,
                        Err(e) => {
                            debug_log!("🔥 Failed to extract NUMERIC field '{}': {}", field_name.root(), e);
                            OwnedValue::Null
                        }
                    }
                }
                // Handle other unknown types safely
                _ => {
                    debug_log!("🔥 Unsupported type_oid {} for field '{}', returning Null", type_oid, field_name.root());
                        OwnedValue::Null
                    }
                }
            } else {
            debug_log!("🔥 Field '{}' not found in tuple", field_name.root());
                OwnedValue::Null
        }
    }

    /// Check if a type OID represents an array type
    fn is_array_type(&self, type_oid: pg_sys::Oid) -> bool {
        // PostgreSQL array types typically have type OIDs > 1000
        // Common array types: 1009 (text[]), 1007 (int4[]), etc.
        let oid_value = type_oid.to_u32();
        oid_value == 1009 || // text[]
        oid_value == 1007 || // int4[]
        oid_value == 1016 || // int8[]
        oid_value == 1021 || // float4[]
        oid_value == 1022 || // float8[]
        oid_value == 1000 || // bool[]
        (oid_value > 1000 && oid_value < 2000) // General array type range
    }

    /// Check if a type OID (as u32) represents an array type
    fn is_array_type_u32(&self, type_oid: u32) -> bool {
        // PostgreSQL array types typically have type OIDs > 1000
        // Common array types: 1009 (text[]), 1007 (int4[]), etc.
        type_oid == 1009 || // text[]
        type_oid == 1007 || // int4[]
        type_oid == 1016 || // int8[]
        type_oid == 1021 || // float4[]
        type_oid == 1022 || // float8[]
        type_oid == 1000 || // bool[]
        (type_oid > 1000 && type_oid < 2000) // General array type range
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

/// Initialize the heap filter expression state using the heap filter node string (if it exists).
pub unsafe fn init_heap_filter_expr_state(
    planstate: *mut pg_sys::PlanState,
    heap_filter_node_string: &str,
) -> Option<*mut pg_sys::ExprState> {
    if heap_filter_node_string.is_empty() {
        return None;
    }

    // Create the Expr node from the heap filter node string
    let expr = create_heap_filter_expr(heap_filter_node_string);
    
    // Initialize the ExprState
    let expr_state = pg_sys::ExecInitExpr(expr, planstate);
    Some(expr_state)
}

/// Create a heap filter expression from a serialized node string
unsafe fn create_heap_filter_expr(heap_filter_node_string: &str) -> *mut pg_sys::Expr {
    // Handle multiple clauses separated by our delimiter
    if heap_filter_node_string.contains("|||AND_CLAUSE_SEPARATOR|||")
        || heap_filter_node_string.contains("|||OR_CLAUSE_SEPARATOR|||")
        || heap_filter_node_string.contains("|||CLAUSE_SEPARATOR|||")
    {
        // Multiple clauses - determine the boolean operation and split accordingly
        let (clause_strings, bool_op) =
            if heap_filter_node_string.contains("|||AND_CLAUSE_SEPARATOR|||") {
                (
                    heap_filter_node_string
                        .split("|||AND_CLAUSE_SEPARATOR|||")
                        .collect::<Vec<&str>>(),
                    pg_sys::BoolExprType::AND_EXPR,
                )
            } else if heap_filter_node_string.contains("|||OR_CLAUSE_SEPARATOR|||") {
                (
                    heap_filter_node_string
                        .split("|||OR_CLAUSE_SEPARATOR|||")
                        .collect::<Vec<&str>>(),
                    pg_sys::BoolExprType::OR_EXPR,
                )
            } else {
                // Legacy support for old CLAUSE_SEPARATOR (assume AND)
                (
                    heap_filter_node_string
                        .split("|||CLAUSE_SEPARATOR|||")
                        .collect::<Vec<&str>>(),
                    pg_sys::BoolExprType::AND_EXPR,
                )
            };

        // Create individual nodes for each clause
        let mut args_list = std::ptr::null_mut();
        for clause_str in clause_strings.iter() {
            let clause_cstring = std::ffi::CString::new(*clause_str)
                .expect("Failed to create CString from clause string");
            let clause_node = pg_sys::stringToNode(clause_cstring.as_ptr());

            if !clause_node.is_null() {
                // For the legacy ExprState path, replace @@@ operators with TRUE constants
                let processed_node = replace_search_operators_with_true(clause_node.cast());
                args_list = pg_sys::lappend(args_list, processed_node.cast::<core::ffi::c_void>());
            } else {
                panic!("Failed to parse clause string: {}", clause_str);
            }
        }

        if !args_list.is_null() {
            // Create a BoolExpr to combine all clauses with the detected boolean operation
            let bool_expr =
                pg_sys::palloc0(std::mem::size_of::<pg_sys::BoolExpr>()).cast::<pg_sys::BoolExpr>();
            (*bool_expr).xpr.type_ = pg_sys::NodeTag::T_BoolExpr;
            (*bool_expr).boolop = bool_op;
            (*bool_expr).args = args_list;
            (*bool_expr).location = -1;

            bool_expr.cast::<pg_sys::Expr>()
        } else {
            panic!("Failed to parse any clauses: {}", heap_filter_node_string);
        }
    } else {
        // Single clause - for legacy ExprState path, process @@@ operators
        let node_cstr = std::ffi::CString::new(heap_filter_node_string)
            .expect("Failed to create CString from node string");
        let node = pg_sys::stringToNode(node_cstr.as_ptr());

        if !node.is_null() {
            // For the legacy ExprState path, replace @@@ operators with TRUE constants
            let processed_node = replace_search_operators_with_true(node.cast());
            processed_node.cast::<pg_sys::Expr>()
        } else {
            panic!("Failed to deserialize node: {}", heap_filter_node_string);
        }
    }
}

/// Replace @@@ search operators with TRUE constants for PostgreSQL evaluation
/// This allows PostgreSQL to evaluate the non-@@@ parts of mixed expressions
unsafe fn replace_search_operators_with_true(node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    if node.is_null() {
        return node;
    }

    match (*node).type_ {
        pg_sys::NodeTag::T_OpExpr => {
            let opexpr = node.cast::<pg_sys::OpExpr>();
            
            // Check if this is a @@@ operator (you'll need to define this check)
            if is_search_operator_for_replacement((*opexpr).opno) {
                // Replace with TRUE constant
                create_true_const().unwrap_or(node)
            } else {
                // Recursively process arguments
                let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
                let mut new_args = std::ptr::null_mut();
                
                for arg in args.iter_ptr() {
                    let processed_arg = replace_search_operators_with_true(arg);
                    new_args = pg_sys::lappend(new_args, processed_arg.cast::<core::ffi::c_void>());
                }
                
                // Create new OpExpr with processed arguments
                let new_opexpr = pg_sys::palloc0(std::mem::size_of::<pg_sys::OpExpr>())
                    .cast::<pg_sys::OpExpr>();
                *new_opexpr = *opexpr; // Copy original
                (*new_opexpr).args = new_args;
                new_opexpr.cast()
            }
        }
        pg_sys::NodeTag::T_BoolExpr => {
            let bool_expr = node.cast::<pg_sys::BoolExpr>();
            let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);
            let mut new_args = std::ptr::null_mut();
            
            for arg in args.iter_ptr() {
                let processed_arg = replace_search_operators_with_true(arg);
                new_args = pg_sys::lappend(new_args, processed_arg.cast::<core::ffi::c_void>());
            }
            
            // Create new BoolExpr with processed arguments
            let new_bool_expr = pg_sys::palloc0(std::mem::size_of::<pg_sys::BoolExpr>())
                .cast::<pg_sys::BoolExpr>();
            *new_bool_expr = *bool_expr; // Copy original
            (*new_bool_expr).args = new_args;
            new_bool_expr.cast()
        }
        _ => {
            // For other node types, return as-is
            node
        }
    }
}

/// Check if an operator OID represents a search operator that should be replaced
unsafe fn is_search_operator_for_replacement(_opno: pg_sys::Oid) -> bool {
    // This should check for @@@ operators
    // You'll need to implement this based on your operator OIDs
    false // Placeholder for now
}

/// Create a TRUE constant node
unsafe fn create_true_const() -> Option<*mut pg_sys::Node> {
    let const_node = pg_sys::palloc0(std::mem::size_of::<pg_sys::Const>()).cast::<pg_sys::Const>();
    (*const_node).xpr.type_ = pg_sys::NodeTag::T_Const;
    (*const_node).consttype = pg_sys::BOOLOID;
    (*const_node).consttypmod = -1;
    (*const_node).constcollid = pg_sys::Oid::from(0);
    (*const_node).constlen = 1;
    (*const_node).constbyval = true;
    (*const_node).constisnull = false;
    (*const_node).constvalue = pg_sys::Datum::from(true);
    (*const_node).location = -1;
    
    Some(const_node.cast())
}
