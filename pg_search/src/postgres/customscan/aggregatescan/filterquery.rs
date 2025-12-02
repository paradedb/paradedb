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
use crate::postgres::customscan::explain::ExplainFormat;
use crate::postgres::utils::ExprContextGuard;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;

use anyhow::Result;
use pgrx::pg_sys;
use std::ptr::NonNull;
use std::sync::Arc;
use tantivy::aggregation::agg_req::AggregationVariants;
use tantivy::aggregation::bucket::{FilterAggregation, SerializableQuery};
use tantivy::query::{EmptyQuery, EnableScoring, Query, QueryParser, Weight};

#[derive(Debug)]
pub struct FilterQuery {
    query: SearchQueryInput,
    indexrelid: pg_sys::Oid,
    tantivy_query: Box<dyn Query>,
    // Keep the ExprContextGuard alive as long as the FilterQuery (and any clones) live.
    // This is needed because the tantivy_query (HeapFilterQuery) holds a raw pointer
    // to the ExprContext, and we need to ensure the context is not freed while the query exists.
    // We use Arc so that clones share ownership - the context is only freed when all
    // FilterQuery instances (original and clones) are dropped.
    #[allow(dead_code)]
    expr_context_guard: Option<Arc<ExprContextGuard>>,
}

impl From<FilterQuery> for AggregationVariants {
    fn from(val: FilterQuery) -> Self {
        AggregationVariants::Filter(FilterAggregation::new_with_query(Box::new(val)))
    }
}

impl Clone for FilterQuery {
    fn clone(&self) -> Self {
        Self {
            query: self.query.clone(),
            indexrelid: self.indexrelid,
            tantivy_query: self.tantivy_query.box_clone(),
            // Clone the Arc to share ownership of the ExprContextGuard.
            // The cloned tantivy_query holds the same raw pointer to the ExprContext,
            // so we must keep the guard alive until all clones are dropped.
            expr_context_guard: self.expr_context_guard.clone(),
        }
    }
}

impl Query for FilterQuery {
    fn weight(&self, enable_scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        // todo: assert once that we are in execution time
        self.tantivy_query.weight(enable_scoring)
    }
}

impl SerializableQuery for FilterQuery {
    fn clone_box(&self) -> Box<dyn SerializableQuery> {
        Box::new(self.clone())
    }
}

impl serde::Serialize for FilterQuery {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let raw = self.query.explain_format();
        serde_json::from_str::<serde_json::Value>(&raw)
            .expect("should be able to serialize searchqueryinput")
            .serialize(serializer)
    }
}

impl FilterQuery {
    pub fn new(
        query: SearchQueryInput,
        indexrelid: pg_sys::Oid,
        is_execution_time: bool,
    ) -> Result<Self> {
        // If not called at execution time, Postgres expressions in the `SearchQueryInput`
        // have not been solved and generating the Tantivy query will fail. To get around this,
        // we produce a junk Tantivy query (which doesn't matter since we're not in execution time).
        if !is_execution_time {
            return Ok(Self {
                query,
                indexrelid,
                tantivy_query: Box::new(EmptyQuery),
                expr_context_guard: None,
            });
        }

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

        Ok(Self {
            query,
            indexrelid,
            tantivy_query,
            // Store the ExprContextGuard in an Arc so it lives as long as
            // the FilterQuery and any of its clones
            expr_context_guard: Some(Arc::new(standalone_context)),
        })
    }
}
