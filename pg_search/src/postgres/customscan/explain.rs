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

            // Remove any field named "postgres_expression" (contains pointers)
            obj.remove("postgres_expression");

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
    use serde_json::Value;

    // Inject estimate at this node if available
    if let Some(estimated_docs) = estimate_tree.estimated_docs {
        if let Value::Object(obj) = json_value {
            obj.insert(
                "estimated_docs".to_string(),
                Value::Number(estimated_docs.into()),
            );
        }
    }

    // Recursively process children based on query structure
    if let Value::Object(obj) = json_value {
        // Handle boolean queries (must, should, must_not)
        if let Some(Value::Object(boolean)) = obj.get_mut("boolean") {
            inject_into_boolean_query(boolean, estimate_tree);
        }
        // Handle with_index wrapper
        else if let Some(Value::Object(with_index)) = obj.get_mut("with_index") {
            if let Some(first_child) = estimate_tree.children().first() {
                if let Some(query) = with_index.get_mut("query") {
                    inject_estimates_into_json(query, first_child);
                }
            }
        }
        // Handle score_adjusted
        else if let Some(Value::Object(score_adjusted)) = obj.get_mut("score_adjusted") {
            if let Some(first_child) = estimate_tree.children().first() {
                if let Some(query) = score_adjusted.get_mut("query") {
                    inject_estimates_into_json(query, first_child);
                }
            }
        }
        // Handle heap_filter
        else if let Some(Value::Object(heap_filter)) = obj.get_mut("heap_filter") {
            if let Some(first_child) = estimate_tree.children().first() {
                if let Some(indexed_query) = heap_filter.get_mut("indexed_query") {
                    inject_estimates_into_json(indexed_query, first_child);
                }
            }
        }
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
                inject_estimates_into_json(must_item, &children[child_idx]);
                child_idx += 1;
            }
        }
    }

    // Process "should" clauses
    if let Some(Value::Array(should)) = boolean.get_mut("should") {
        for should_item in should.iter_mut() {
            if child_idx < children.len() {
                inject_estimates_into_json(should_item, &children[child_idx]);
                child_idx += 1;
            }
        }
    }

    // Process "must_not" clauses
    if let Some(Value::Array(must_not)) = boolean.get_mut("must_not") {
        for must_not_item in must_not.iter_mut() {
            if child_idx < children.len() {
                inject_estimates_into_json(must_not_item, &children[child_idx]);
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
}
