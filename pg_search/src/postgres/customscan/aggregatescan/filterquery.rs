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

//! FilterQuery - a tantivy QueryBuilder for PostgreSQL filter aggregations.
//!
//! Uses a function pointer to defer PostgreSQL-dependent query building to runtime,
//! which is required to avoid linker errors in pgrx_embed.

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

/// Type alias for the filter query builder function.
/// This function is set at runtime to avoid pulling PostgreSQL symbols into pgrx_embed.
/// Takes the JSON-serialized query and indexrelid.
type BuildFilterQueryFn = fn(serde_json::Value, u32) -> anyhow::Result<Box<dyn Query>>;

/// Global function pointer for building filter queries.
/// This is initialized at extension load time via `init_filter_query_builder()`.
/// Using a function pointer breaks the link-time dependency on PostgreSQL symbols,
/// allowing the pgrx_embed binary to be built without them.
static BUILD_FILTER_QUERY_FN: std::sync::OnceLock<BuildFilterQueryFn> = std::sync::OnceLock::new();

/// Initialize the query builder. Call from `_PG_init`.
pub fn init_filter_query_builder() {
    BUILD_FILTER_QUERY_FN.get_or_init(|| build_query);
}

/// Create a FilterQuery from SearchQueryInput.
pub fn new_filter_query(
    query: SearchQueryInput,
    indexrelid: pg_sys::Oid,
) -> anyhow::Result<FilterQuery> {
    Ok(FilterQuery {
        query: serde_json::to_value(&query)?,
        indexrelid: indexrelid.to_u32(),
    })
}

/// A QueryBuilder wrapping SearchQueryInput as JSON to avoid link-time PostgreSQL dependencies.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FilterQuery {
    /// The SearchQueryInput serialized as JSON.
    /// We store it as JSON to avoid importing SearchQueryInput which has PostgreSQL dependencies.
    query: serde_json::Value,
    /// Index OID stored as u32 to avoid pg_sys::Oid which would pull in PostgreSQL symbols.
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
        // Get the builder function that was initialized at extension load time.
        // This indirection via function pointer avoids pulling PostgreSQL symbols
        // into the pgrx_embed binary at link time.
        let build_fn = BUILD_FILTER_QUERY_FN
            .get()
            .expect("call init_filter_query_builder() in _PG_init");
        build_fn(self.query.clone(), self.indexrelid)
            .map_err(|e| TantivyError::InvalidArgument(e.to_string()))
    }

    fn box_clone(&self) -> Box<dyn QueryBuilder> {
        Box::new(self.clone())
    }
}

fn build_query(query_json: serde_json::Value, indexrelid: u32) -> anyhow::Result<Box<dyn Query>> {
    let query: SearchQueryInput = serde_json::from_value(query_json)?;
    let indexrelid = pg_sys::Oid::from(indexrelid);
    let context = ExprContextGuard::new();

    let index = PgSearchRelation::with_lock(indexrelid, pg_sys::AccessShareLock as _);
    let schema = index.schema()?;
    let reader = SearchIndexReader::open_with_context(
        &index,
        query.clone(),
        false,
        MvccSatisfies::Snapshot,
        NonNull::new(context.as_ptr()),
        None,
    )?;

    let tantivy_query = query.into_tantivy_query(
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
    )?;

    Ok(Box::new(QueryWithContext {
        query: Box::new(tantivy_query),
        _context: Arc::new(context),
    }))
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
