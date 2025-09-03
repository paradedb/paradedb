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

use std::ptr::NonNull;

use pgrx::pg_sys;
use pgrx::pg_sys::AsPgCStr;

use crate::query::SearchQueryInput;

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
        let mut json_value = serde_json::to_value(query).expect("query should serialize to json");
        cleanup_variabilities_from_tantivy_query(&mut json_value);
        self.add_json("Tantivy Query", json_value)
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

/// Remove the oid from the with_index object
/// This helps to reduce the variability of the explain output used in regression tests
fn cleanup_variabilities_from_tantivy_query(json_value: &mut serde_json::Value) {
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

            // Remove any field named "postgres_expression"
            obj.remove("postgres_expression");

            // Recursively process all values in the object
            for (_, value) in obj.iter_mut() {
                cleanup_variabilities_from_tantivy_query(value);
            }
        }
        serde_json::Value::Array(arr) => {
            // Recursively process all elements in the array
            for item in arr.iter_mut() {
                cleanup_variabilities_from_tantivy_query(item);
            }
        }
        // Base cases: primitive values don't need processing
        _ => {}
    }
}
