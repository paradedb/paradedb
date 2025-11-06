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

use super::estimate_tree::QueryWithEstimates;
use super::SearchQueryInput;
use tantivy::query::Query as TantivyQuery;

/// Build different output types from Tantivy queries.
pub trait QueryBuilder {
    type Output;

    /// Build a leaf node with no children.
    fn build_leaf(
        &self,
        query_input: SearchQueryInput,
        tantivy_query: Box<dyn TantivyQuery>,
        label: String,
    ) -> Self::Output;

    /// Build a node with children.
    fn build_with_children(
        &self,
        query_input: SearchQueryInput,
        tantivy_query: Box<dyn TantivyQuery>,
        label: String,
        children: Vec<Self::Output>,
    ) -> Self::Output;

    /// Extract the query from output.
    fn extract_query(output: &Self::Output) -> &dyn TantivyQuery;

    /// Clone the query from output.
    fn clone_query(output: &Self::Output) -> Box<dyn TantivyQuery>;
}

/// Builder that returns only the Tantivy Query.
pub struct QueryOnlyBuilder;

impl QueryBuilder for QueryOnlyBuilder {
    type Output = Box<dyn TantivyQuery>;

    fn build_leaf(
        &self,
        _query_input: SearchQueryInput,
        tantivy_query: Box<dyn TantivyQuery>,
        _label: String,
    ) -> Self::Output {
        tantivy_query
    }

    fn build_with_children(
        &self,
        _query_input: SearchQueryInput,
        tantivy_query: Box<dyn TantivyQuery>,
        _label: String,
        _children: Vec<Self::Output>,
    ) -> Self::Output {
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
pub struct QueryTreeBuilder;

impl QueryBuilder for QueryTreeBuilder {
    type Output = (Box<dyn TantivyQuery>, QueryWithEstimates);

    fn build_leaf(
        &self,
        query_input: SearchQueryInput,
        tantivy_query: Box<dyn TantivyQuery>,
        label: String,
    ) -> Self::Output {
        let tree = QueryWithEstimates::new(query_input, label);
        (tantivy_query, tree)
    }

    fn build_with_children(
        &self,
        query_input: SearchQueryInput,
        tantivy_query: Box<dyn TantivyQuery>,
        label: String,
        children: Vec<Self::Output>,
    ) -> Self::Output {
        let child_trees: Vec<_> = children.into_iter().map(|(_, tree)| tree).collect();
        let tree = QueryWithEstimates::with_children(query_input, label, child_trees);
        (tantivy_query, tree)
    }

    fn extract_query(output: &Self::Output) -> &dyn TantivyQuery {
        output.0.as_ref()
    }

    fn clone_query(output: &Self::Output) -> Box<dyn TantivyQuery> {
        output.0.box_clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::query::AllQuery;

    #[test]
    fn test_query_only_builder() {
        let builder = QueryOnlyBuilder;
        let query: Box<dyn TantivyQuery> = Box::new(AllQuery);

        let output = builder.build_leaf(
            SearchQueryInput::All,
            query.box_clone(),
            "Test Query".to_string(),
        );

        let _extracted = QueryOnlyBuilder::extract_query(&output);
    }

    #[test]
    fn test_query_tree_builder() {
        let builder = QueryTreeBuilder;
        let query: Box<dyn TantivyQuery> = Box::new(AllQuery);

        let output = builder.build_leaf(
            SearchQueryInput::All,
            query.box_clone(),
            "Test Query".to_string(),
        );

        let _extracted = QueryTreeBuilder::extract_query(&output);
        let tree = &output.1;
        assert_eq!(tree.query_type, "Test Query");
    }
}
