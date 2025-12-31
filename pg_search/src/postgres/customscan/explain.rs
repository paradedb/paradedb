// Copyright (C) 2023-2026 ParadeDB, Inc.
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

//! Unified formatting for EXPLAIN output and display strings
//!
//! This module provides a consistent way to format objects for EXPLAIN output,
//! ensuring deterministic output for regression tests by removing variabilities
//! like OIDs and internal pointers.

use crate::query::estimate_tree::QueryWithEstimates;
use serde::Serialize;

/// Trait for objects that can be formatted for EXPLAIN output
///
/// Implementers should provide a deterministic string representation
/// suitable for display to users and regression tests.
pub trait ExplainFormat {
    /// Format this object for EXPLAIN output (deterministic, user-friendly)
    fn explain_format(&self) -> String;
}

/// Clean JSON value for deterministic EXPLAIN output
///
/// Removes variabilities like:
/// - `oid` fields from `with_index` objects
/// - `postgres_expression` fields (contain pointers)
/// - `expr_node` fields from `field_filters` (internal node representation)
/// - Any other non-deterministic data
pub fn cleanup_json_for_explain(json_value: &mut serde_json::Value) {
    match json_value {
        serde_json::Value::Object(obj) => {
            // Check if this is a "with_index" object and remove its "oid" if present
            if obj.contains_key("with_index") {
                if let Some(with_index) = obj.get_mut("with_index") {
                    if let Some(with_index_obj) = with_index.as_object_mut() {
                        with_index_obj.remove("oid");
                    }
                }
            }

            // Clean up HeapFieldFilter objects: remove raw expr_node (internal node representation)
            // Keep the heap_filter field which contains the human-readable SQL expression
            if obj.contains_key("expr_node") && obj.contains_key("heap_filter") {
                obj.remove("expr_node");
            }

            // Remove any field named "postgres_expression" (contains pointers)
            obj.remove("postgres_expression");
            obj.remove("indexrelid");

            // Recursively process all values in the object
            for (_, value) in obj.iter_mut() {
                cleanup_json_for_explain(value);
            }
        }
        serde_json::Value::Array(arr) => {
            // Recursively process all elements in the array
            for item in arr.iter_mut() {
                cleanup_json_for_explain(item);
            }
        }
        // Base cases: primitive values don't need processing
        _ => {}
    }
}

/// Format a serializable object for EXPLAIN output
///
/// This is a convenience function that:
/// 1. Serializes to JSON
/// 2. Cleans up variabilities
/// 3. Returns a deterministic string
pub fn format_for_explain<T: Serialize>(value: &T) -> String {
    let mut json_value = serde_json::to_value(value)
        .unwrap_or_else(|_| serde_json::Value::String("Error serializing".to_string()));
    cleanup_json_for_explain(&mut json_value);
    serde_json::to_string(&json_value).unwrap_or_else(|_| "Error".to_string())
}

/// Recursively inject estimated_docs from QueryWithEstimates tree into JSON
///
/// This function walks both the QueryWithEstimates tree and the corresponding JSON
/// structure in parallel, injecting `estimated_docs` fields at matching query nodes.
fn inject_estimates_into_json(
    json_value: &mut serde_json::Value,
    estimate_tree: &QueryWithEstimates,
) {
    use crate::query::SearchQueryInput;
    use serde_json::Value;

    // Try to get the JSON value as an object
    // Some unit variants like `All` and `Empty` serialize as strings ("all", "empty")
    // rather than objects, so we can't inject estimates into them directly
    let obj = match json_value.as_object_mut() {
        Some(obj) => obj,
        None => return, // Primitive value (string like "all"), can't inject estimate
    };

    // Inject estimate at this node if available
    if let Some(estimated_docs) = estimate_tree.estimated_docs {
        obj.insert(
            "estimated_docs".to_string(),
            Value::Number(estimated_docs.into()),
        );
    }

    // Match on the query type first for type safety
    match &estimate_tree.query {
        SearchQueryInput::Boolean { .. } => {
            let boolean = obj
                .get_mut("boolean")
                .expect("expected 'boolean' key in JSON for Boolean query")
                .as_object_mut()
                .expect("'boolean' value should be an object");
            inject_into_boolean_query(boolean, estimate_tree);
        }
        SearchQueryInput::WithIndex { .. } => {
            let first_child = estimate_tree
                .children()
                .first()
                .expect("WithIndex query should have a child");
            let with_index = obj
                .get_mut("with_index")
                .expect("expected 'with_index' key in JSON for WithIndex query")
                .as_object_mut()
                .expect("'with_index' value should be an object");
            let query = with_index
                .get_mut("query")
                .expect("'with_index' should have a 'query' field");
            inject_estimates_into_json(query, first_child);
        }
        SearchQueryInput::Boost { .. } => {
            let first_child = estimate_tree
                .children()
                .first()
                .expect("Boost query should have a child");
            let boost = obj
                .get_mut("boost")
                .expect("expected 'boost' key in JSON for Boost query")
                .as_object_mut()
                .expect("'boost' value should be an object");
            let query = boost
                .get_mut("query")
                .expect("'boost' should have a 'query' field");
            inject_estimates_into_json(query, first_child);
        }
        SearchQueryInput::ConstScore { .. } => {
            let first_child = estimate_tree
                .children()
                .first()
                .expect("ConstScore query should have a child");
            let const_score = obj
                .get_mut("const_score")
                .expect("expected 'const_score' key in JSON for ConstScore query")
                .as_object_mut()
                .expect("'const_score' value should be an object");
            let query = const_score
                .get_mut("query")
                .expect("'const_score' should have a 'query' field");
            inject_estimates_into_json(query, first_child);
        }
        SearchQueryInput::HeapFilter { .. } => {
            let first_child = estimate_tree
                .children()
                .first()
                .expect("HeapFilter query should have a child");
            let heap_filter = obj
                .get_mut("heap_filter")
                .expect("expected 'heap_filter' key in JSON for HeapFilter query")
                .as_object_mut()
                .expect("'heap_filter' value should be an object");
            let indexed_query = heap_filter
                .get_mut("indexed_query")
                .expect("'heap_filter' should have an 'indexed_query' field");
            inject_estimates_into_json(indexed_query, first_child);
        }
        SearchQueryInput::DisjunctionMax { .. } => {
            let disjunction_max = obj
                .get_mut("disjunction_max")
                .expect("expected 'disjunction_max' key in JSON for DisjunctionMax query")
                .as_object_mut()
                .expect("'disjunction_max' value should be an object");
            let disjuncts = disjunction_max
                .get_mut("disjuncts")
                .expect("'disjunction_max' should have a 'disjuncts' field")
                .as_array_mut()
                .expect("'disjuncts' should be an array");
            for (idx, child) in estimate_tree.children().iter().enumerate() {
                if idx < disjuncts.len() {
                    inject_estimates_into_json(&mut disjuncts[idx], child);
                }
            }
        }
        SearchQueryInput::ScoreFilter { query, .. } => {
            if query.is_some() {
                let first_child = estimate_tree
                    .children()
                    .first()
                    .expect("ScoreFilter with query should have a child");
                let score_filter = obj
                    .get_mut("score_filter")
                    .expect("expected 'score_filter' key in JSON for ScoreFilter query")
                    .as_object_mut()
                    .expect("'score_filter' value should be an object");
                let query = score_filter
                    .get_mut("query")
                    .expect("'score_filter' should have a 'query' field");
                inject_estimates_into_json(query, first_child);
            }
        }
        // Leaf query types - no children to process
        SearchQueryInput::All
        | SearchQueryInput::Empty
        | SearchQueryInput::MoreLikeThis { .. }
        | SearchQueryInput::Parse { .. }
        | SearchQueryInput::TermSet { .. }
        | SearchQueryInput::PostgresExpression { .. }
        | SearchQueryInput::FieldedQuery { .. }
        | SearchQueryInput::Uninitialized => {
            // These are leaf nodes, no children to process
        }
    }
}

/// Unwrap the Empty wrapper that Boolean clause children are wrapped in.
/// The estimate tree wraps each clause in an Empty wrapper for labeling purposes,
/// but the JSON doesn't have this wrapper, so we need to skip it.
fn unwrap_boolean_clause_child(child: &QueryWithEstimates) -> &QueryWithEstimates {
    use crate::query::SearchQueryInput;
    match &child.query {
        SearchQueryInput::Empty => {
            // This is the wrapper - use the first (and only) child
            child.children().first().unwrap_or(child)
        }
        _ => child,
    }
}

/// Helper to inject estimates into boolean query clauses
fn inject_into_boolean_query(
    boolean: &mut serde_json::Map<String, serde_json::Value>,
    estimate_tree: &QueryWithEstimates,
) {
    use serde_json::Value;

    let children = estimate_tree.children();
    let mut child_idx = 0;

    // Process "must" clauses
    if let Some(Value::Array(must)) = boolean.get_mut("must") {
        for must_item in must.iter_mut() {
            if child_idx < children.len() {
                let actual_child = unwrap_boolean_clause_child(&children[child_idx]);
                inject_estimates_into_json(must_item, actual_child);
                child_idx += 1;
            }
        }
    }

    // Process "should" clauses
    if let Some(Value::Array(should)) = boolean.get_mut("should") {
        for should_item in should.iter_mut() {
            if child_idx < children.len() {
                let actual_child = unwrap_boolean_clause_child(&children[child_idx]);
                inject_estimates_into_json(should_item, actual_child);
                child_idx += 1;
            }
        }
    }

    // Process "must_not" clauses
    if let Some(Value::Array(must_not)) = boolean.get_mut("must_not") {
        for must_not_item in must_not.iter_mut() {
            if child_idx < children.len() {
                let actual_child = unwrap_boolean_clause_child(&children[child_idx]);
                inject_estimates_into_json(must_not_item, actual_child);
                child_idx += 1;
            }
        }
    }
}

/// Format a query with recursive cost estimates for EXPLAIN output
///
/// This function:
/// 1. Serializes the query to JSON
/// 2. Cleans up variabilities
/// 3. Injects estimated_docs fields from the estimate tree
/// 4. Returns a deterministic string with embedded estimates
pub fn format_for_explain_with_estimates(estimate_tree: &QueryWithEstimates) -> String {
    let mut json_value = serde_json::to_value(&estimate_tree.query)
        .unwrap_or_else(|_| serde_json::Value::String("Error serializing".to_string()));
    cleanup_json_for_explain(&mut json_value);
    inject_estimates_into_json(&mut json_value, estimate_tree);
    serde_json::to_string(&json_value).unwrap_or_else(|_| "Error".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cleanup_removes_oid() {
        let mut value = json!({
            "with_index": {
                "oid": 12345,
                "name": "test_index"
            },
            "field": "value"
        });

        cleanup_json_for_explain(&mut value);

        assert_eq!(
            value,
            json!({
                "with_index": {
                    "name": "test_index"
                },
                "field": "value"
            })
        );
    }

    #[test]
    fn test_cleanup_removes_postgres_expression() {
        let mut value = json!({
            "query": "test",
            "postgres_expression": "0x123456",
            "nested": {
                "postgres_expression": "0xABCDEF",
                "data": "value"
            }
        });

        cleanup_json_for_explain(&mut value);

        assert_eq!(
            value,
            json!({
                "query": "test",
                "nested": {
                    "data": "value"
                }
            })
        );
    }

    #[test]
    fn test_cleanup_handles_arrays() {
        let mut value = json!([
            { "oid": 123, "name": "a" },
            { "postgres_expression": "ptr", "value": "b" }
        ]);

        cleanup_json_for_explain(&mut value);

        assert_eq!(
            value,
            json!([
                { "oid": 123, "name": "a" },
                { "value": "b" }
            ])
        );
    }

    #[test]
    fn test_cleanup_heap_filter_field_filters() {
        // Test that expr_node is removed and heap_filter is kept
        let mut value = json!({
            "heap_filter": {
                "indexed_query": "all",
                "field_filters": [
                    {
                        "expr_node": "{OPEXPR :opno 98 :opfuncid 67 ...}",
                        "heap_filter": "(category = 'Electronics'::text)"
                    },
                    {
                        "expr_node": "{OPEXPR :opno 1754 ...}",
                        "heap_filter": "(price > 500.00)"
                    }
                ]
            }
        });

        cleanup_json_for_explain(&mut value);

        assert_eq!(
            value,
            json!({
                "heap_filter": {
                    "indexed_query": "all",
                    "field_filters": [
                        {
                            "heap_filter": "(category = 'Electronics'::text)"
                        },
                        {
                            "heap_filter": "(price > 500.00)"
                        }
                    ]
                }
            })
        );
    }
}
