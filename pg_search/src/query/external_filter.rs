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
use pgrx::pg_sys;
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
/// Note: This is not thread-safe and should only be used within a single thread
pub struct CallbackManager {
    /// Serialized expression for recreation in worker processes
    expression: String,
    /// Mapping from attribute numbers to field names
    attno_map: HashMap<pg_sys::AttrNumber, FieldName>,
}

// Implement Send and Sync manually since we're only storing serialized data
unsafe impl Send for CallbackManager {}
unsafe impl Sync for CallbackManager {}

impl CallbackManager {
    /// Create a new callback manager with serialized expression
    pub fn new(expression: String, attno_map: HashMap<pg_sys::AttrNumber, FieldName>) -> Self {
        Self {
            expression,
            attno_map,
        }
    }

    /// Evaluate the expression for the given field values
    /// This is a placeholder implementation - in a full implementation,
    /// this would recreate the PostgreSQL expression state and evaluate it
    pub fn evaluate(&self, _field_values: &HashMap<FieldName, OwnedValue>) -> bool {
        // TODO: Implement proper PostgreSQL expression evaluation
        // For now, return true as a placeholder
        true
    }
}

/// Create a callback function for PostgreSQL expression evaluation
pub fn create_postgres_callback(
    expression: String,
    attno_map: HashMap<pg_sys::AttrNumber, FieldName>,
) -> ExternalFilterCallback {
    let callback_manager = CallbackManager::new(expression, attno_map);

    Arc::new(
        move |_doc_id: DocId, field_values: &HashMap<FieldName, OwnedValue>| {
            callback_manager.evaluate(field_values)
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
}

impl Scorer for ExternalFilterScorer {
    fn score(&mut self) -> Score {
        self.boost
    }
}

impl tantivy::DocSet for ExternalFilterScorer {
    fn advance(&mut self) -> DocId {
        if let Some(ref callback) = self.callback {
            // Find the next document that matches the external filter
            while self.doc_id < self.max_doc {
                // Extract field values for this document
                let field_values = self.extract_field_values(self.doc_id);

                // Evaluate the callback
                if callback(self.doc_id, &field_values) {
                    let current_doc = self.doc_id;
                    self.doc_id += 1;
                    return current_doc;
                }

                self.doc_id += 1;
            }
        } else {
            // No callback available - skip all documents
            self.doc_id = self.max_doc;
        }

        tantivy::TERMINATED
    }

    fn doc(&self) -> DocId {
        self.doc_id.saturating_sub(1)
    }

    fn size_hint(&self) -> u32 {
        // Conservative estimate - we don't know how many documents will match
        self.max_doc.saturating_sub(self.doc_id)
    }
}

impl ExternalFilterScorer {
    /// Extract field values for the given document
    fn extract_field_values(&self, _doc_id: DocId) -> HashMap<FieldName, OwnedValue> {
        let field_values = HashMap::default();

        // For now, return empty map - this will be implemented when we have
        // the full field extraction infrastructure
        // TODO: Implement field value extraction from fast fields and stored fields

        field_values
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
