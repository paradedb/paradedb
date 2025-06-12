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

/// A Tantivy query that evaluates external PostgreSQL expressions via callback
#[derive(Clone)]
pub struct ExternalFilterQuery {
    /// Callback function to evaluate the expression for a given document
    callback: Arc<dyn Fn(DocId) -> bool + Send + Sync>,
}

impl ExternalFilterQuery {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(DocId) -> bool + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }
}

impl std::fmt::Debug for ExternalFilterQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExternalFilterQuery")
            .field("callback", &"<callback function>")
            .finish()
    }
}

impl Query for ExternalFilterQuery {
    fn weight(&self, _enable_scoring: EnableScoring) -> tantivy::Result<Box<dyn Weight>> {
        Ok(Box::new(ExternalFilterWeight {
            callback: self.callback.clone(),
        }))
    }
}

#[derive(Clone)]
struct ExternalFilterWeight {
    callback: Arc<dyn Fn(DocId) -> bool + Send + Sync>,
}

impl Weight for ExternalFilterWeight {
    fn scorer(&self, _reader: &SegmentReader, _boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
        Ok(Box::new(ExternalFilterScorer {
            callback: self.callback.clone(),
            doc: 0,
            max_doc: _reader.max_doc(),
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

struct ExternalFilterScorer {
    callback: Arc<dyn Fn(DocId) -> bool + Send + Sync>,
    doc: DocId,
    max_doc: DocId,
}

impl Scorer for ExternalFilterScorer {
    fn score(&mut self) -> Score {
        1.0 // External filters don't contribute to scoring
    }
}

impl tantivy::DocSet for ExternalFilterScorer {
    fn advance(&mut self) -> DocId {
        loop {
            self.doc += 1;
            if self.doc >= self.max_doc {
                return tantivy::TERMINATED;
            }
            if (self.callback)(self.doc) {
                return self.doc;
            }
        }
    }

    fn doc(&self) -> DocId {
        self.doc
    }

    fn size_hint(&self) -> u32 {
        self.max_doc
    }
}
