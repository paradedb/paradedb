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
//! # Overview
//!
//! This module provides a builder pattern for constructing query outputs that can
//! either return just the Tantivy query (`QueryOnlyBuilder`) or both the query and
//! a tree structure for estimates (`QueryTreeBuilder`).
//!
//! # Two Builders, Two Use Cases
//!
//! - **`QueryOnlyBuilder`**: Used for normal query execution.
//!   Returns only `Box<dyn TantivyQuery>`. Ignores all closures for zero overhead.
//!
//! - **`QueryTreeBuilder`**: Used for `EXPLAIN VERBOSE` output.
//!   Returns `(Box<dyn TantivyQuery>, QueryWithEstimates)`. Calls all closures
//!   to build the estimate tree structure.
//!
//! # Performance Optimizations
//!
//! The trait is designed for zero-cost abstraction on the hot path:
//!
//! 1. **Lazy Labels**: `label_fn` closure only called by `QueryTreeBuilder`
//! 2. **Lazy Children**: `children_fn` closure only called by `QueryTreeBuilder`
//! 3. **Lazy Placeholders**: `query_input_fn` closure only called by `QueryTreeBuilder`
//! 4. **Zero-Clone Splitting**: `split_for_parent()` avoids cloning for `QueryOnlyBuilder`
//!
//! # Key Method: `split_for_parent`
//!
//! When building composite queries (Boolean, Boost, etc.), we need the child's
//! Tantivy query for the parent, and optionally the full output for the children
//! closure. `split_for_parent` handles this differently per builder:
//!
//! ```text
//! QueryOnlyBuilder:
//!   output ──────────────────> (query, None)
//!                              No clone! Output consumed.
//!
//! QueryTreeBuilder:
//!   output ──┬── clone ──────> (cloned_query, Some(output))
//!            └── keep ───────> Must keep output for children closure.
//! ```
//!
//! # Usage Pattern in `into_tantivy_query_generic`
//!
//! ```ignore
//! // Process child query
//! let output = recurse(child_input)?;
//!
//! // Split: zero-cost for QueryOnlyBuilder, clone for QueryTreeBuilder
//! let (child_query, opt_output) = B::split_for_parent(output);
//!
//! // Use child_query for parent construction
//! parent_subqueries.push(child_query);
//!
//! // Store output for children closure (only Some for QueryTreeBuilder)
//! if let Some(out) = opt_output {
//!     child_outputs.push(out);
//! }
//! ```

use super::estimate_tree::QueryWithEstimates;
use super::SearchQueryInput;
use tantivy::query::Query as TantivyQuery;

/// Build different output types from Tantivy queries.
///
/// The trait uses closures for all parameters to enable lazy evaluation:
/// - `QueryOnlyBuilder` never calls any closures, achieving zero allocation overhead
/// - `QueryTreeBuilder` calls all closures to build the estimate tree
pub trait QueryBuilder {
    type Output;

    /// Whether this builder needs the actual `SearchQueryInput` for estimation.
    ///
    /// - `QueryOnlyBuilder`: `false` - closures are never called, no clone needed
    /// - `QueryTreeBuilder`: `true` - closures are called, need real query for estimation
    ///
    /// Use this to conditionally clone before destructuring:
    /// ```ignore
    /// let cloned = if B::NEEDS_QUERY_INPUT { Some(self.clone()) } else { None };
    /// let SearchQueryInput::Boost { query, factor } = self else { ... };
    /// // `self` is consumed, but `cloned` has the full query for estimation
    /// ```
    const NEEDS_QUERY_INPUT: bool;

    /// Build a leaf node with no children.
    ///
    /// Label closure is lazy - only called by builders that need it.
    /// `query_input` is `Some` for `QueryTreeBuilder`, `None` for `QueryOnlyBuilder`.
    fn build_leaf<F>(
        &self,
        tantivy_query: Box<dyn TantivyQuery>,
        label_fn: F,
        query_input: Option<SearchQueryInput>,
    ) -> Self::Output
    where
        F: FnOnce() -> String;

    /// Build a node with children.
    ///
    /// Label and children closures are lazy - only called by builders that need them.
    /// `query_input` is `Some` for `QueryTreeBuilder`, `None` for `QueryOnlyBuilder`.
    fn build_with_children<F, C>(
        &self,
        tantivy_query: Box<dyn TantivyQuery>,
        label_fn: F,
        children_fn: C,
        query_input: Option<SearchQueryInput>,
    ) -> Self::Output
    where
        F: FnOnce() -> String,
        C: FnOnce(&Self) -> Vec<Self::Output>;

    /// Extract the query from output (by reference).
    fn extract_query(output: &Self::Output) -> &dyn TantivyQuery;

    /// Clone the query from output.
    fn clone_query(output: &Self::Output) -> Box<dyn TantivyQuery>;

    /// Take the query from output, consuming the output.
    ///
    /// For `QueryOnlyBuilder`: Returns the query directly (zero-cost, no clone).
    /// For `QueryTreeBuilder`: Clones the query (output is still needed for children).
    ///
    /// Use this when you need the query for a parent node and either:
    /// - Don't need the output for children (`QueryOnlyBuilder` case), or
    /// - Will store the output separately for lazy children processing
    fn take_query(output: Self::Output) -> Box<dyn TantivyQuery>;

    /// Split output into query (for parent) and optional output (for children closure).
    ///
    /// For `QueryOnlyBuilder`: Returns (query, None) - no clone, output consumed.
    /// For `QueryTreeBuilder`: Returns (cloned_query, Some(output)) - needs both.
    ///
    /// This is the key optimization: `QueryOnlyBuilder` avoids cloning entirely.
    fn split_for_parent(output: Self::Output) -> (Box<dyn TantivyQuery>, Option<Self::Output>);
}

/// Builder that returns only the Tantivy Query.
///
/// This builder ignores all closures (label, children, query_input) since it only
/// needs to return the Tantivy query. By never calling any closures, we achieve
/// zero allocation overhead for the fast path.
pub struct QueryOnlyBuilder;

impl QueryBuilder for QueryOnlyBuilder {
    type Output = Box<dyn TantivyQuery>;

    /// No estimation needed - closures are never called, so no clone required.
    const NEEDS_QUERY_INPUT: bool = false;

    fn build_leaf<F>(
        &self,
        tantivy_query: Box<dyn TantivyQuery>,
        _label_fn: F,
        _query_input: Option<SearchQueryInput>,
    ) -> Self::Output
    where
        F: FnOnce() -> String,
    {
        // Normal query execution - ignore label_fn and query_input
        tantivy_query
    }

    fn build_with_children<F, C>(
        &self,
        tantivy_query: Box<dyn TantivyQuery>,
        _label_fn: F,
        _children_fn: C,
        _query_input: Option<SearchQueryInput>,
    ) -> Self::Output
    where
        F: FnOnce() -> String,
        C: FnOnce(&Self) -> Vec<Self::Output>,
    {
        // Normal query execution - ignore all closures and query_input.
        // children_fn is never called, so nested build calls inside it
        // (which may pass Some(...) placeholders) are never executed.
        tantivy_query
    }

    fn extract_query(output: &Self::Output) -> &dyn TantivyQuery {
        output.as_ref()
    }

    fn clone_query(output: &Self::Output) -> Box<dyn TantivyQuery> {
        output.box_clone()
    }

    fn take_query(output: Self::Output) -> Box<dyn TantivyQuery> {
        // Zero-cost: just return the query, no clone needed
        output
    }

    fn split_for_parent(output: Self::Output) -> (Box<dyn TantivyQuery>, Option<Self::Output>) {
        // Zero-cost: take the query, no output needed for children (closure never called)
        (output, None)
    }
}

/// Builder that returns both Tantivy Query and QueryWithEstimates tree.
///
/// This builder calls all closures to construct the full tree structure
/// needed for recursive cost estimates in EXPLAIN output.
pub struct QueryTreeBuilder;

impl QueryBuilder for QueryTreeBuilder {
    type Output = (Box<dyn TantivyQuery>, QueryWithEstimates);

    /// Estimation needed - closures are called, so clone the query before destructuring.
    const NEEDS_QUERY_INPUT: bool = true;

    fn build_leaf<F>(
        &self,
        tantivy_query: Box<dyn TantivyQuery>,
        label_fn: F,
        query_input: Option<SearchQueryInput>,
    ) -> Self::Output
    where
        F: FnOnce() -> String,
    {
        // query_input is always Some for QueryTreeBuilder
        let tree = QueryWithEstimates::new(query_input.unwrap(), label_fn());
        (tantivy_query, tree)
    }

    fn build_with_children<F, C>(
        &self,
        tantivy_query: Box<dyn TantivyQuery>,
        label_fn: F,
        children_fn: C,
        query_input: Option<SearchQueryInput>,
    ) -> Self::Output
    where
        F: FnOnce() -> String,
        C: FnOnce(&Self) -> Vec<Self::Output>,
    {
        let children = children_fn(self);
        let child_trees: Vec<_> = children.into_iter().map(|(_, tree)| tree).collect();
        // query_input is always Some for QueryTreeBuilder
        let tree = QueryWithEstimates::with_children(query_input.unwrap(), label_fn(), child_trees);
        (tantivy_query, tree)
    }

    fn extract_query(output: &Self::Output) -> &dyn TantivyQuery {
        output.0.as_ref()
    }

    fn clone_query(output: &Self::Output) -> Box<dyn TantivyQuery> {
        output.0.box_clone()
    }

    fn take_query(output: Self::Output) -> Box<dyn TantivyQuery> {
        // Take the query from the tuple, discard the tree
        output.0
    }

    fn split_for_parent(output: Self::Output) -> (Box<dyn TantivyQuery>, Option<Self::Output>) {
        // Must clone: we need query for parent AND output for children closure
        let query = output.0.box_clone();
        (query, Some(output))
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

        let output = builder.build_leaf(query.box_clone(), || "Test Query".to_string(), None);

        let _extracted = QueryOnlyBuilder::extract_query(&output);
    }

    #[pgrx::pg_test]
    fn test_query_tree_builder() {
        let builder = QueryTreeBuilder;
        let query: Box<dyn TantivyQuery> = Box::new(AllQuery);

        let output = builder.build_leaf(
            query.box_clone(),
            || "Test Query".to_string(),
            Some(SearchQueryInput::All),
        );

        let _extracted = QueryTreeBuilder::extract_query(&output);
        let tree = &output.1;
        assert_eq!(tree.query_type, "Test Query");
    }
}
