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

use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::utils::ExprContextGuard;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;

use anyhow::Result;
use pgrx::pg_sys;
use std::ptr::NonNull;
use std::sync::Arc;
use tantivy::aggregation::agg_req::AggregationVariants;
use tantivy::aggregation::bucket::{FilterAggregation, QueryBuilder};
use tantivy::query::{EnableScoring, Query, QueryParser, Weight};
use tantivy::schema::Schema;
use tantivy::tokenizer::TokenizerManager;
use tantivy::TantivyError;

/// A wrapper that holds both a tantivy Query and the ExprContextGuard that must
/// stay alive as long as the query exists (for queries that hold raw pointers to
/// the ExprContext, such as HeapFilterQuery for correlated subqueries).
/// Uses Arc for the context so it can be cloned (Query trait requires Clone).
struct QueryWithContext {
    tantivy_query: Box<dyn Query>,
    #[allow(dead_code)]
    expr_context_guard: Arc<ExprContextGuard>,
}

impl Clone for QueryWithContext {
    fn clone(&self) -> Self {
        Self {
            tantivy_query: self.tantivy_query.box_clone(),
            expr_context_guard: self.expr_context_guard.clone(),
        }
    }
}

impl std::fmt::Debug for QueryWithContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryWithContext")
            .field("tantivy_query", &self.tantivy_query)
            .finish()
    }
}

impl Query for QueryWithContext {
    fn weight(&self, enable_scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        self.tantivy_query.weight(enable_scoring)
    }
}

/// FilterQuery is a QueryBuilder that builds tantivy queries from SearchQueryInput.
/// The actual query building is deferred until `build_query()` is called, which allows
/// proper serialization/deserialization for distributed aggregation scenarios.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FilterQuery {
    query: SearchQueryInput,
    indexrelid: pg_sys::Oid,
}

impl From<FilterQuery> for AggregationVariants {
    fn from(val: FilterQuery) -> Self {
        AggregationVariants::Filter(FilterAggregation::new_with_builder(Box::new(val)))
    }
}

#[typetag::serde]
impl QueryBuilder for FilterQuery {
    fn build_query(
        &self,
        _schema: &Schema,
        _tokenizers: &TokenizerManager,
    ) -> tantivy::Result<Box<dyn Query>> {
        // Build the tantivy query at execution time using our stored SearchQueryInput
        // and indexrelid. This is called by tantivy when it needs the actual query.
        self.build_tantivy_query()
            .map_err(|e| TantivyError::InvalidArgument(e.to_string()))
    }

    fn box_clone(&self) -> Box<dyn QueryBuilder> {
        Box::new(self.clone())
    }
}

impl FilterQuery {
    pub fn new(
        query: SearchQueryInput,
        indexrelid: pg_sys::Oid,
        _is_execution_time: bool,
    ) -> Result<Self> {
        // We no longer build the query at construction time.
        // The query will be built lazily when build_query() is called.
        Ok(Self { query, indexrelid })
    }

    /// Build the actual tantivy query from the stored SearchQueryInput.
    /// This is called from build_query() at execution time.
    fn build_tantivy_query(&self) -> Result<Box<dyn Query>> {
        let standalone_context = ExprContextGuard::new();
        let index = PgSearchRelation::with_lock(self.indexrelid, pg_sys::AccessShareLock as _);
        let schema = index.schema()?;
        let reader = SearchIndexReader::open_with_context(
            &index,
            self.query.clone(),
            false,
            MvccSatisfies::Snapshot,
            NonNull::new(standalone_context.as_ptr()),
            None,
        )?;
        let parser = || {
            QueryParser::for_index(
                reader.searcher().index(),
                schema.fields().map(|(f, _)| f).collect(),
            )
        };
        let heap_oid = index.heap_relation().map(|r| r.oid());
        let tantivy_query = Box::new(self.query.clone().into_tantivy_query(
            &schema,
            &parser,
            reader.searcher(),
            index.oid(),
            heap_oid,
            NonNull::new(standalone_context.as_ptr()),
            None,
        )?);

        // Wrap the query with its ExprContextGuard to keep the context alive
        // as long as the query exists (needed for HeapFilterQuery and similar
        // queries that hold raw pointers to the ExprContext).
        Ok(Box::new(QueryWithContext {
            tantivy_query,
            expr_context_guard: Arc::new(standalone_context),
        }))
    }
}
