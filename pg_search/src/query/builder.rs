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

//! Generic query builder trait for different output types.
//!
//! This module provides a builder pattern for constructing query outputs that can
//! either return just the Tantivy query (`QueryOnlyBuilder`) or both the query and
//! a tree structure for estimates (`QueryTreeBuilder`).
//!
//! The trait methods use closures for labels and children to enable lazy evaluation,
//! avoiding unnecessary allocations when the builder doesn't need them.

use super::estimate_tree::QueryWithEstimates;
use super::SearchQueryInput;
use tantivy::query::Query as TantivyQuery;

/// Build different output types from Tantivy queries.
///
/// The trait uses closures for `label` and `children` parameters to enable lazy
/// evaluation - `QueryOnlyBuilder` doesn't need labels or children, so it never
/// calls these closures, avoiding unnecessary allocations.
pub trait QueryBuilder {
    type Output;

    /// Build a leaf node with no children.
    ///
    /// `label_fn` is a closure that generates the label lazily.
    /// Takes a reference to avoid cloning for `QueryOnlyBuilder` which ignores the input.
    fn build_leaf<F>(
        &self,
        query_input: &SearchQueryInput,
        tantivy_query: Box<dyn TantivyQuery>,
        label_fn: F,
    ) -> Self::Output
    where
        F: FnOnce() -> String;

    /// Build a node with children.
    ///
    /// `label_fn` generates the label lazily.
    /// `children_fn` generates the children lazily.
    /// Takes a reference to avoid cloning for `QueryOnlyBuilder` which ignores the input.
    fn build_with_children<F, C>(
        &self,
        query_input: &SearchQueryInput,
        tantivy_query: Box<dyn TantivyQuery>,
        label_fn: F,
        children_fn: C,
    ) -> Self::Output
    where
        F: FnOnce() -> String,
        C: FnOnce() -> Vec<Self::Output>;

    /// Extract the query from output.
    fn extract_query(output: &Self::Output) -> &dyn TantivyQuery;

    /// Clone the query from output.
    fn clone_query(output: &Self::Output) -> Box<dyn TantivyQuery>;
}

/// Builder that returns only the Tantivy Query.
///
/// This builder ignores labels, children, and query_input since it only
/// needs to return the Tantivy query. The lazy closures are never called,
/// avoiding unnecessary allocations for large queries.
pub struct QueryOnlyBuilder;

impl QueryBuilder for QueryOnlyBuilder {
    type Output = Box<dyn TantivyQuery>;

    fn build_leaf<F>(
        &self,
        _query_input: &SearchQueryInput,
        tantivy_query: Box<dyn TantivyQuery>,
        _label_fn: F,
    ) -> Self::Output
    where
        F: FnOnce() -> String,
    {
        // Don't call label_fn - we don't need it
        // query_input is ignored - no clone needed on this path
        tantivy_query
    }

    fn build_with_children<F, C>(
        &self,
        _query_input: &SearchQueryInput,
        tantivy_query: Box<dyn TantivyQuery>,
        _label_fn: F,
        _children_fn: C,
    ) -> Self::Output
    where
        F: FnOnce() -> String,
        C: FnOnce() -> Vec<Self::Output>,
    {
        // Don't call label_fn or children_fn - we don't need them
        // query_input is ignored - no clone needed on this path
        tantivy_query
    }

    fn extract_query(output: &Self::Output) -> &dyn TantivyQuery {
        output.as_ref()
    }

    fn clone_query(output: &Self::Output) -> Box<dyn TantivyQuery> {
        output.box_clone()
    }
}

/// Builder that returns both Tantivy Query and QueryWithEstimates tree.
///
/// This builder calls all closures to construct the full tree structure
/// needed for recursive cost estimates in EXPLAIN output.
pub struct QueryTreeBuilder;

impl QueryBuilder for QueryTreeBuilder {
    type Output = (Box<dyn TantivyQuery>, QueryWithEstimates);

    fn build_leaf<F>(
        &self,
        query_input: &SearchQueryInput,
        tantivy_query: Box<dyn TantivyQuery>,
        label_fn: F,
    ) -> Self::Output
    where
        F: FnOnce() -> String,
    {
        // Clone the query_input for the estimate tree (needed for cost estimation)
        let tree = QueryWithEstimates::new(query_input.clone(), label_fn());
        (tantivy_query, tree)
    }

    fn build_with_children<F, C>(
        &self,
        query_input: &SearchQueryInput,
        tantivy_query: Box<dyn TantivyQuery>,
        label_fn: F,
        children_fn: C,
    ) -> Self::Output
    where
        F: FnOnce() -> String,
        C: FnOnce() -> Vec<Self::Output>,
    {
        let children = children_fn();
        let child_trees: Vec<_> = children.into_iter().map(|(_, tree)| tree).collect();
        // Clone the query_input for the estimate tree (needed for cost estimation)
        let tree = QueryWithEstimates::with_children(query_input.clone(), label_fn(), child_trees);
        (tantivy_query, tree)
    }

    fn extract_query(output: &Self::Output) -> &dyn TantivyQuery {
        output.0.as_ref()
    }

    fn clone_query(output: &Self::Output) -> Box<dyn TantivyQuery> {
        output.0.box_clone()
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use tantivy::query::AllQuery;

    #[pgrx::pg_test]
    fn test_query_only_builder() {
        let builder = QueryOnlyBuilder;
        let query: Box<dyn TantivyQuery> = Box::new(AllQuery);

        let output = builder.build_leaf(&SearchQueryInput::All, query.box_clone(), || {
            "Test Query".to_string()
        });

        let _extracted = QueryOnlyBuilder::extract_query(&output);
    }

    #[pgrx::pg_test]
    fn test_query_tree_builder() {
        let builder = QueryTreeBuilder;
        let query: Box<dyn TantivyQuery> = Box::new(AllQuery);

        let output = builder.build_leaf(&SearchQueryInput::All, query.box_clone(), || {
            "Test Query".to_string()
        });

        let _extracted = QueryTreeBuilder::extract_query(&output);
        let tree = &output.1;
        assert_eq!(tree.query_type, "Test Query");
    }
}
