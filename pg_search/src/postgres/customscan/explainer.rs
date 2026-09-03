// Copyright (c) 2023-2026 ParadeDB, Inc.
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

use crate::postgres::customscan::explain;
use crate::postgres::customscan::explain::ExplainFormat;
use crate::query::SearchQueryInput;
use crate::query::estimate_tree::QueryWithEstimates;

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

    /// Add a query with recursive estimates to the EXPLAIN output
    pub fn add_query_with_estimates(&mut self, query_tree: &QueryWithEstimates) {
        self.add_text(
            "Tantivy Query",
            explain::format_for_explain_with_estimates(query_tree),
        );
    }

    /// Deparse a PostgreSQL expression using PostgreSQL's active EXPLAIN context (`deparse_cxt`).
    pub fn deparse_expr(&self, node: *mut pg_sys::Node) -> Option<String> {
        use std::panic::AssertUnwindSafe;
        if node.is_null() {
            return None;
        }
        unsafe {
            let es = self.state.as_ptr();
            let cxt = (*es).deparse_cxt;
            if cxt.is_null() {
                return None;
            }
            pgrx::PgTryBuilder::new(AssertUnwindSafe(|| {
                let deparsed = pg_sys::deparse_expression(node.cast(), cxt, true, false);
                if deparsed.is_null() {
                    None
                } else {
                    Some(
                        std::ffi::CStr::from_ptr(deparsed)
                            .to_string_lossy()
                            .into_owned(),
                    )
                }
            }))
            .catch_others(|_| None)
            .execute()
        }
    }

    /// Deserializes a PostgreSQL node from string format and deparses it using the active context.
    /// Falls back to the raw string if deserialization or deparsing fails.
    pub fn deparse_serialized(&self, pg_node_string: &str) -> String {
        let Ok(c_str) = std::ffi::CString::new(pg_node_string) else {
            return pg_node_string.to_string();
        };
        let node = unsafe { pg_sys::stringToNode(c_str.as_ptr().cast_mut()) };
        if node.is_null() {
            return pg_node_string.to_string();
        }
        self.deparse_expr(node.cast())
            .unwrap_or_else(|| pg_node_string.to_string())
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

    pub fn add_bool(&mut self, key: &str, value: bool) {
        unsafe {
            pg_sys::ExplainPropertyBool(key.as_pg_cstr(), value, self.state.as_ptr());
        }
    }
}
