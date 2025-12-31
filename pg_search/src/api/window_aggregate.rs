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

//! Internal window function placeholder.
//!
//! This module provides `window_agg()`, which is used internally to replace
//! window aggregate calls (like `COUNT(*) OVER ()` or `pdb.agg(...) OVER ()`)
//! during query planning.
//!
//! ## How It Works
//!
//! 1. During planning, the planner hook detects window functions in queries with the `@@@` operator
//! 2. These window functions are replaced with calls to `window_agg(json)` where the JSON
//!    contains the serialized aggregation specification
//! 3. The custom scan intercepts these placeholder calls and executes the actual aggregations
//!    using Tantivy collectors
//! 4. This function should never actually execute - if it does, it indicates a bug
//!
//! ## User-Facing API
//!
//! Users should never call `window_agg()` directly. Instead, they should use:
//! - Standard SQL window functions: `COUNT(*) OVER ()`, `SUM(field) OVER ()`, etc.
//! - Custom aggregations: `pdb.agg('{"avg": {"field": "price"}}'::jsonb) OVER ()`
//!
//! Both of these get automatically converted to `window_agg()` calls during planning.

use pgrx::pg_sys;

use crate::postgres::utils::lookup_pdb_function;

#[pgrx::pg_schema]
mod pdb {
    use pgrx::prelude::*;

    /// Internal placeholder function for window aggregates.
    ///
    /// This function should never actually execute - it exists only as a placeholder
    /// that the custom scan replaces during execution. The JSON parameter contains
    /// the serialized aggregation specification.
    ///
    /// If this function executes, it means the custom scan failed to intercept it,
    /// which indicates a bug in the planning logic.
    #[pg_extern(volatile, parallel_safe, name = "window_agg")]
    pub fn window_agg_placeholder(window_aggregate_json: &str) -> i64 {
        pgrx::error!(
        "window_agg placeholder should not be executed - custom scan should have intercepted this. JSON: {}",
        window_aggregate_json
    )
    }
}

/// Get the OID of the window_agg placeholder function
/// Returns InvalidOid if the function doesn't exist yet (e.g., during extension creation)
pub fn window_agg_oid() -> pg_sys::Oid {
    lookup_pdb_function("window_agg", &[pg_sys::TEXTOID])
}
