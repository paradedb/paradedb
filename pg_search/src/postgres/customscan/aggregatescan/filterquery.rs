// Copyright (c) 2023-2026 ParadeDB, Inc.
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

//! FilterQuery - a tantivy QueryBuilder for PostgreSQL filter aggregations.

use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::utils::ExprContextGuard;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;

use pgrx::pg_sys;
use std::fmt::Debug;
use std::ptr::NonNull;
use std::sync::Arc;
use tantivy::aggregation::agg_req::AggregationVariants;
use tantivy::aggregation::bucket::{FilterAggregation, QueryBuilder};
use tantivy::query::{EnableScoring, Query, QueryParser, Weight};
use tantivy::schema::Schema;
use tantivy::tokenizer::TokenizerManager;
use tantivy::TantivyError;

/// Create a FilterQuery from SearchQueryInput.
pub fn new_filter_query(query: SearchQueryInput, indexrelid: pg_sys::Oid) -> FilterQuery {
    FilterQuery {
        query,
        indexrelid: indexrelid.to_u32(),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FilterQuery {
    query: SearchQueryInput,
    /// Index OID stored as u32 because `pg_sys::Oid` is not `Serialize`.
    indexrelid: u32,
}

impl From<FilterQuery> for AggregationVariants {
    fn from(val: FilterQuery) -> Self {
        AggregationVariants::Filter(FilterAggregation::new_with_builder(Box::new(val)))
    }
}

#[typetag::serde]
impl QueryBuilder for FilterQuery {
    fn build_query(&self, _: &Schema, _: &TokenizerManager) -> tantivy::Result<Box<dyn Query>> {
        let indexrelid = pg_sys::Oid::from(self.indexrelid);
        let context = ExprContextGuard::new();
        let index = PgSearchRelation::with_lock(indexrelid, pg_sys::AccessShareLock as _);
        let schema = index
            .schema()
            .map_err(|e| TantivyError::InvalidArgument(e.to_string()))?;
        let reader = SearchIndexReader::open_with_context(
            &index,
            self.query.clone(),
            false,
            MvccSatisfies::Snapshot,
            NonNull::new(context.as_ptr()),
            None,
            self.query.needs_tokenizer(),
        )
        .map_err(|e| TantivyError::InvalidArgument(e.to_string()))?;

        let tantivy_query = self
            .query
            .clone()
            .into_tantivy_query(
                &schema,
                &|| {
                    QueryParser::for_index(
                        reader.searcher().index(),
                        schema.fields().map(|(f, _)| f).collect(),
                    )
                },
                reader.searcher(),
                index.oid(),
                index.heap_relation().map(|r| r.oid()),
                NonNull::new(context.as_ptr()),
                None,
            )
            .map_err(|e| TantivyError::InvalidArgument(e.to_string()))?;

        Ok(Box::new(QueryWithContext {
            query: Box::new(tantivy_query),
            _context: Arc::new(context),
        }))
    }

    fn box_clone(&self) -> Box<dyn QueryBuilder> {
        Box::new(self.clone())
    }
}

/// Wraps a Query with its ExprContextGuard to extend the context's lifetime.
struct QueryWithContext {
    query: Box<dyn Query>,
    _context: Arc<ExprContextGuard>,
}

impl Query for QueryWithContext {
    fn weight(&self, scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        self.query.weight(scoring)
    }
}

impl Clone for QueryWithContext {
    fn clone(&self) -> Self {
        Self {
            query: self.query.box_clone(),
            _context: self._context.clone(),
        }
    }
}

impl Debug for QueryWithContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.query.fmt(f)
    }
}
