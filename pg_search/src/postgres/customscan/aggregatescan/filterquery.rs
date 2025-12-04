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

use crate::query::SearchQueryInput;

use pgrx::pg_sys;
use tantivy::aggregation::agg_req::AggregationVariants;
use tantivy::aggregation::bucket::{FilterAggregation, QueryBuilder};
use tantivy::query::Query;
use tantivy::schema::Schema;
use tantivy::tokenizer::TokenizerManager;
use tantivy::TantivyError;

/// Type alias for the filter query builder function.
/// This function is set at runtime to avoid pulling PostgreSQL symbols into pgrx_embed.
type BuildFilterQueryFn = fn(&SearchQueryInput, pg_sys::Oid) -> anyhow::Result<Box<dyn Query>>;

/// Global function pointer for building filter queries.
/// This is initialized at extension load time via `init_filter_query_builder()`.
/// Using a function pointer breaks the link-time dependency on PostgreSQL symbols,
/// allowing the pgrx_embed binary to be built without them.
static BUILD_FILTER_QUERY_FN: std::sync::OnceLock<BuildFilterQueryFn> = std::sync::OnceLock::new();

/// Initialize the filter query builder function. Must be called at extension load time.
pub fn init_filter_query_builder() {
    BUILD_FILTER_QUERY_FN.get_or_init(|| build_filter_query_impl);
}

/// FilterQuery is a QueryBuilder that builds tantivy queries from SearchQueryInput.
/// The actual query building is deferred until `build_query()` is called, which allows
/// proper serialization/deserialization for distributed aggregation scenarios.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FilterQuery {
    query: SearchQueryInput,
    indexrelid: u32,
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
        // Get the builder function that was initialized at extension load time.
        // This indirection via function pointer avoids pulling PostgreSQL symbols
        // into the pgrx_embed binary at link time.
        let build_fn = BUILD_FILTER_QUERY_FN
            .get()
            .expect("FilterQuery builder not initialized - call init_filter_query_builder() first");

        build_fn(&self.query, pg_sys::Oid::from(self.indexrelid))
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
    ) -> anyhow::Result<Self> {
        // We no longer build the query at construction time.
        // The query will be built lazily when build_query() is called.
        Ok(Self {
            query,
            indexrelid: indexrelid.to_u32(),
        })
    }
}

// ============================================================================
// PostgreSQL-dependent implementation
// ============================================================================
// This code is in a separate function to be called via function pointer,
// breaking the link-time dependency on PostgreSQL symbols.

use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::utils::ExprContextGuard;
use crate::postgres::PgSearchRelation;
use std::ptr::NonNull;
use std::sync::Arc;
use tantivy::query::{EnableScoring, QueryParser, Weight};

/// A wrapper that holds both a tantivy Query and the ExprContextGuard that must
/// stay alive as long as the query exists (for queries that hold raw pointers to
/// the ExprContext, such as HeapFilterQuery for correlated subqueries).
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

/// The actual implementation that builds a tantivy query from SearchQueryInput.
/// This function contains all PostgreSQL-dependent code.
fn build_filter_query_impl(
    query: &SearchQueryInput,
    indexrelid: pg_sys::Oid,
) -> anyhow::Result<Box<dyn Query>> {
    let standalone_context = ExprContextGuard::new();
    let index = PgSearchRelation::with_lock(indexrelid, pg_sys::AccessShareLock as _);
    let schema = index.schema()?;
    let reader = SearchIndexReader::open_with_context(
        &index,
        query.clone(),
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
    let tantivy_query = Box::new(query.clone().into_tantivy_query(
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
