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
//! ## User-Facing Function: `pdb.agg(jsonb)`
//!
//! This is the public API for users to specify custom Tantivy aggregations.
//! When used in window function context (`OVER ()`), it gets intercepted at planning
//! time and replaced with `window_agg()` placeholder. The actual execution happens
//! in the custom scan using Tantivy's aggregation collectors.
//!
//! Example: `SELECT *, pdb.agg('{"avg": {"field": "price"}}'::jsonb) OVER () FROM products`
//!
//! When used with GROUP BY, the aggregate currently returns an error indicating it's not supported.
//! The window function variant is the primary use case.
//!
//! ## Internal Function: `window_agg(text)`
//!
//! This is an internal placeholder function (in `window_aggregate.rs`) that replaces
//! `pdb.agg()` calls when they appear in window function context during planning.
//! It should never be called by users directly.
//!
//! ## Placeholder Aggregate: `AggPlaceholder`
//!
//! This implements the `pdb.agg()` aggregate using pgrx's native aggregate API.
//! It should never actually execute - if it does, it will error immediately with a
//! clear message. This is similar to how `paradedb.score()` works as a placeholder
//! that the custom scan intercepts and handles.

use std::error::Error;

use pgrx::{default, pg_extern, Json, JsonB, PgRelation};

use crate::aggregate::{execute_aggregate, AggregateRequest};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::{lookup_pdb_function, ExprContextGuard};
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

#[pgrx::pg_schema]
mod pdb {
    use pgrx::aggregate::Aggregate;
    use pgrx::{pg_extern, Internal, JsonB};

    /// Placeholder aggregate for `pdb.agg()`.
    ///
    /// This aggregate should never actually execute - it's intercepted at planning time
    /// for window functions or by AggregateScan for (GROUP BY) aggregate queries.
    #[derive(pgrx::AggregateName, Default)]
    #[aggregate_name = "agg"]
    pub struct AggPlaceholder;

    #[pgrx::pg_aggregate(parallel_safe)]
    impl Aggregate<AggPlaceholder> for AggPlaceholder {
        type Args = JsonB;
        type State = Internal;
        type Finalize = JsonB;

        fn state(
            _current: Self::State,
            _arg: Self::Args,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::State {
            // This should never execute - if it does, the query wasn't handled by our custom scan
            pgrx::error!(
            "pdb.agg() must be handled by ParadeDB's custom scan. \
             This error usually means the query syntax is not supported. \
             Try adding '@@@ paradedb.all()' to your WHERE clause to force custom scan usage, \
             or file an issue at https://github.com/paradedb/paradedb/issues if this should be supported."
        )
        }

        fn finalize(
            _current: Self::State,
            _direct_arg: Self::OrderedSetArgs,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::Finalize {
            // This should never execute - if it does, the query wasn't handled by our custom scan
            pgrx::error!(
            "pdb.agg() must be handled by ParadeDB's custom scan. \
             This error usually means the query syntax is not supported. \
             Try adding '@@@ paradedb.all()' to your WHERE clause to force custom scan usage, \
             or file an issue at https://github.com/paradedb/paradedb/issues if this should be supported."
        )
        }
    }

    /// Placeholder function for aggregate replacement in custom scans.
    ///
    /// This function should never execute - it's used to replace Aggref nodes
    /// in the plan tree to avoid "Aggref found in non-Agg plan node" errors.
    /// The actual aggregation is performed by the custom scan.
    ///
    /// The string argument is used to identify the aggregate in EXPLAIN output.
    #[pg_extern(volatile, parallel_safe, name = "agg_fn")]
    pub fn agg_fn_placeholder(_agg_name: &str) -> i64 {
        pgrx::error!(
            "pdb.agg_fn() placeholder should not be executed - \
             custom scan should have intercepted this."
        )
    }
}

/// Get the OID of the pdb.agg_fn() placeholder function
/// Returns InvalidOid if the function doesn't exist yet (e.g., during extension creation)
pub fn agg_fn_oid() -> pgrx::pg_sys::Oid {
    lookup_pdb_function("agg_fn", &[pgrx::pg_sys::TEXTOID])
}
