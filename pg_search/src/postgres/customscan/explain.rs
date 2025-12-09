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

            // Clean up field_filters in heap_filter: remove raw expr_node and rename description to filter
            // The description field contains human-readable SQL expression
            if obj.contains_key("expr_node") && obj.contains_key("description") {
                // This is a HeapFieldFilter object - remove internal node representation
                // and rename "description" to "filter" for cleaner output
                obj.remove("expr_node");
                if let Some(description) = obj.remove("description") {
                    obj.insert("heap_filter".to_string(), description);
                }
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
        // Test that expr_node is removed and description is renamed to filter
        let mut value = json!({
            "heap_filter": {
                "indexed_query": "all",
                "field_filters": [
                    {
                        "expr_node": "{OPEXPR :opno 98 :opfuncid 67 ...}",
                        "description": "(category = 'Electronics'::text)"
                    },
                    {
                        "expr_node": "{OPEXPR :opno 1754 ...}",
                        "description": "(price > 500.00)"
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
                            "filter": "(category = 'Electronics'::text)"
                        },
                        {
                            "filter": "(price > 500.00)"
                        }
                    ]
                }
            })
        );
    }
}
