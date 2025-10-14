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

use pgrx::prelude::*;
use pgrx::{direct_function_call, pg_sys, IntoDatum};

/// Internal placeholder function for window aggregates
///
/// This function is used as a replacement for WindowFunc nodes during planning.
/// The JSON parameter contains the serialized WindowSpecification that will be
/// deserialized during custom scan planning.
///
/// Users should never call this directly - it's injected by the planner hook.
#[pg_extern(volatile, parallel_safe, name = "window_func")]
pub fn window_func_placeholder(window_aggregate_json: &str) -> i64 {
    // This is just a placeholder that should never actually execute
    // If it does execute, it means our custom scan didn't intercept it
    panic!(
        "window_func placeholder executed - custom scan should have intercepted this. JSON: {}",
        window_aggregate_json
    );
}

/// Get the OID of the window_func placeholder function
pub fn window_func_oid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.window_func(text)".into_datum()],
        )
        .expect("the `paradedb.window_func` function should exist")
    }
}
