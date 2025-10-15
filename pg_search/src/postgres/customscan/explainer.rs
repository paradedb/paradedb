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

//! This module provides a consistent way to format objects for EXPLAIN output,
//! ensuring deterministic output for regression tests by removing variabilities
//! like OIDs and internal pointers.

use std::ptr::NonNull;

use pgrx::pg_sys;
use pgrx::pg_sys::AsPgCStr;
use serde::Serialize;

use crate::query::SearchQueryInput;

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

pub struct Explainer {
    state: NonNull<pg_sys::ExplainState>,
}

impl Explainer {
    pub fn new(state: *mut pg_sys::ExplainState) -> Option<Self> {
        NonNull::new(state).map(|state| Self { state })
    }

    pub fn is_verbose(&self) -> bool {
        unsafe { (*self.state.as_ptr()).verbose }
    }

    pub fn is_analyze(&self) -> bool {
        unsafe { (*self.state.as_ptr()).analyze }
    }

    pub fn is_costs(&self) -> bool {
        unsafe { (*self.state.as_ptr()).costs }
    }

    pub fn add_query(&mut self, query: &SearchQueryInput) {
        self.add_explainable("Tantivy Query", query);
    }

    /// Add an explainable object to the output
    pub fn add_explainable<T: ExplainFormat>(&mut self, key: &str, value: &T) {
        self.add_text(key, value.explain_format());
    }

    pub fn add_json<T: serde::Serialize>(&mut self, key: &str, value: T) {
        self.add_text(
            key,
            serde_json::to_string(&value)
                .unwrap_or_else(|e| panic!("{key} should serialize to json: {e}")),
        );
    }

    pub fn add_text<S: AsRef<str>>(&mut self, key: &str, value: S) {
        unsafe {
            pg_sys::ExplainPropertyText(
                key.as_pg_cstr(),
                value.as_ref().as_pg_cstr(),
                self.state.as_ptr(),
            );
        }
    }

    #[allow(dead_code)]
    pub fn add_integer(&mut self, key: &str, value: i64, unit: Option<&str>) {
        unsafe {
            pg_sys::ExplainPropertyInteger(
                key.as_pg_cstr(),
                unit.as_pg_cstr(),
                value,
                self.state.as_ptr(),
            );
        }
    }

    pub fn add_unsigned_integer(&mut self, key: &str, value: u64, unit: Option<&str>) {
        unsafe {
            pg_sys::ExplainPropertyUInteger(
                key.as_pg_cstr(),
                unit.as_pg_cstr(),
                value,
                self.state.as_ptr(),
            );
        }
    }

    #[allow(dead_code)]
    pub fn add_float(&mut self, key: &str, value: f64, unit: Option<&str>, ndigits: i32) {
        unsafe {
            pg_sys::ExplainPropertyFloat(
                key.as_pg_cstr(),
                unit.as_pg_cstr(),
                value,
                ndigits,
                self.state.as_ptr(),
            );
        }
    }

    pub fn add_bool(&mut self, key: &str, value: bool) {
        unsafe {
            pg_sys::ExplainPropertyBool(key.as_pg_cstr(), value, self.state.as_ptr());
        }
    }

    #[allow(dead_code)]
    pub fn add_list(&mut self, key: &str, values: &mut pgrx::list::List<*mut std::ffi::c_char>) {
        unsafe {
            pg_sys::ExplainPropertyList(key.as_pg_cstr(), values.as_mut_ptr(), self.state.as_ptr())
        }
    }
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
