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

//! Aggregate functions for ParadeDB search.
//!
//! ## User-Facing Function: `paradedb.agg(jsonb)`
//!
//! This is the public API for users to specify custom Tantivy aggregations.
//! When used in window function context (`OVER ()`), it gets intercepted at planning
//! time and replaced with `window_func()` placeholder. The actual execution happens
//! in the custom scan using Tantivy's aggregation collectors.
//!
//! Example: `SELECT *, paradedb.agg('{"avg": {"field": "price"}}'::jsonb) OVER () FROM products`
//!
//! When used with GROUP BY, the aggregate currently returns an error indicating it's not supported.
//! The window function variant is the primary use case.
//!
//! ## Internal Function: `window_func(text)`
//!
//! This is an internal placeholder function (in `window_function.rs`) that replaces
//! `paradedb.agg()` calls when they appear in window function context during planning.
//! It should never be called by users directly.
//!
//! ## Placeholder Functions: `agg_sfunc` and `agg_finalfunc`
//!
//! These are PostgreSQL aggregate state transition and finalization functions that
//! should never actually execute. They exist only to satisfy PostgreSQL's aggregate
//! function requirements. If they do execute, it means the custom scan failed to
//! intercept the aggregate, and they will error immediately.

use std::error::Error;

use pgrx::{default, pg_extern, Internal, Json, JsonB, PgRelation};

use crate::aggregate::{execute_aggregate, AggregateRequest};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::ExprContextGuard;
use crate::query::SearchQueryInput;

#[pg_extern]
pub fn aggregate(
    index: PgRelation,
    query: SearchQueryInput,
    agg: Json,
    solve_mvcc: default!(bool, true),
    memory_limit: default!(i64, 500000000),
    bucket_limit: default!(i64, 65000),
) -> Result<JsonB, Box<dyn Error>> {
    let relation = unsafe { PgSearchRelation::from_pg(index.as_ptr()) };
    let standalone_context = ExprContextGuard::new();
    let aggregate = execute_aggregate(
        &relation,
        query,
        AggregateRequest::Json(serde_json::from_value(agg.0)?),
        solve_mvcc,
        memory_limit.try_into()?,
        bucket_limit.try_into()?,
        standalone_context.as_ptr(),
    )?;
    if aggregate.0.is_empty() {
        Ok(JsonB(serde_json::Value::Null))
    } else {
        Ok(JsonB(serde_json::to_value(aggregate)?))
    }
}

/// Placeholder state transition function for `paradedb.agg` aggregate.
///
/// This function should never actually execute - it exists only to satisfy PostgreSQL's
/// aggregate function requirements. The custom scan intercepts `paradedb.agg()` calls
/// during planning and handles them directly using Tantivy collectors.
///
/// If this function executes, it means the custom scan failed to intercept the aggregate,
/// which indicates a bug in the planning logic.
#[pg_extern(stable, parallel_safe)]
pub fn agg_sfunc(_state: Option<Internal>, _agg_definition: JsonB) -> Option<Internal> {
    pgrx::error!("paradedb.agg() placeholder function should not be executed.")
}

/// Placeholder final function for `paradedb.agg` aggregate.
///
/// This function should never actually execute - it exists only to satisfy PostgreSQL's
/// aggregate function requirements. See `agg_sfunc` documentation for details.
#[pg_extern(stable, parallel_safe)]
pub fn agg_finalfunc(_state: Option<Internal>) -> JsonB {
    pgrx::error!("paradedb.agg() placeholder function should not be executed.")
}
