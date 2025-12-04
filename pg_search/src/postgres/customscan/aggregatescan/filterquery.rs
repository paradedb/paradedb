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

//! FilterQuery implementation for tantivy's QueryBuilder trait.
//!
//! This module is carefully structured to avoid pulling PostgreSQL symbols into
//! the pgrx_embed binary. The PostgreSQL-dependent implementation is in a separate
//! module (`filterquery_impl`) and registered via function pointer at runtime.

use crate::query::SearchQueryInput;

use tantivy::aggregation::agg_req::AggregationVariants;
use tantivy::aggregation::bucket::{FilterAggregation, QueryBuilder};
use tantivy::query::Query;
use tantivy::schema::Schema;
use tantivy::tokenizer::TokenizerManager;
use tantivy::TantivyError;

/// Type alias for the filter query builder function.
/// This function is set at runtime to avoid pulling PostgreSQL symbols into pgrx_embed.
pub type BuildFilterQueryFn = fn(&SearchQueryInput, u32) -> anyhow::Result<Box<dyn Query>>;

/// Global function pointer for building filter queries.
/// This is initialized at extension load time via `init_filter_query_builder()`.
/// Using a function pointer breaks the link-time dependency on PostgreSQL symbols,
/// allowing the pgrx_embed binary to be built without them.
pub static BUILD_FILTER_QUERY_FN: std::sync::OnceLock<BuildFilterQueryFn> =
    std::sync::OnceLock::new();

/// FilterQuery is a QueryBuilder that builds tantivy queries from SearchQueryInput.
/// The actual query building is deferred until `build_query()` is called, which allows
/// proper serialization/deserialization for distributed aggregation scenarios.
///
/// IMPORTANT: This struct must NOT contain any PostgreSQL types (like pg_sys::Oid)
/// to avoid pulling PostgreSQL symbols into the pgrx_embed binary via typetag.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FilterQuery {
    query: SearchQueryInput,
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

        build_fn(&self.query, self.indexrelid)
            .map_err(|e| TantivyError::InvalidArgument(e.to_string()))
    }

    fn box_clone(&self) -> Box<dyn QueryBuilder> {
        Box::new(self.clone())
    }
}

impl FilterQuery {
    /// Create a new FilterQuery.
    ///
    /// Takes the indexrelid as u32 to avoid storing pg_sys::Oid which would pull
    /// PostgreSQL symbols into the pgrx_embed binary. Callers should use
    /// `indexrelid.to_u32()` to convert from pg_sys::Oid.
    ///
    /// Returns `Ok(Self)` for API compatibility (was previously fallible when
    /// building the query eagerly).
    pub fn new(
        query: SearchQueryInput,
        indexrelid: u32,
        _is_execution_time: bool,
    ) -> anyhow::Result<Self> {
        Ok(Self { query, indexrelid })
    }
}
