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

//! Aggregate functions for ParadeDB search.
//!
//! ## User-Facing Function: `pdb.agg(jsonb)` and `pdb.agg(jsonb, bool)`
//!
//! This is the public API for users to specify custom Tantivy aggregations.
//! When used in window function context (`OVER ()`), it gets intercepted at planning
//! time and replaced with `window_agg()` placeholder. The actual execution happens
//! in the custom scan using Tantivy's aggregation collectors.
//!
//! Example: `SELECT *, pdb.agg('{"avg": {"field": "price"}}'::jsonb) OVER () FROM products`
//!
//! The optional second argument controls MVCC visibility filtering:
//! - 'enabled' (default): Apply MVCC filtering for transaction-accurate aggregates
//! - 'disabled': Skip MVCC filtering for faster but potentially stale aggregates
//!
//! Example with MVCC disabled:
//! `SELECT *, pdb.agg('{"avg": {"field": "price"}}'::jsonb, false) OVER () FROM products`
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
use serde::{Deserialize, Serialize};

use crate::aggregate::{execute_aggregate, AggregateRequest};
use crate::api::HashMap;
use crate::gucs;
use crate::postgres::customscan::aggregatescan::{
    descale_numeric_values_in_json, extract_agg_name_to_field,
};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::{lookup_pdb_function, ExprContextGuard};
use crate::query::SearchQueryInput;
use crate::schema::SearchFieldType;

fn aggregate_impl(
    index: PgRelation,
    query: SearchQueryInput,
    agg: Json,
    solve_mvcc: bool,
    memory_limit: i64,
    bucket_limit: i64,
) -> Result<JsonB, Box<dyn Error>> {
    // Explicit bucket_limit must be semantically valid.
    if bucket_limit <= 0 {
        pgrx::error!("bucket_limit must be a positive integer");
    }

    // Convert with a clearer error for huge values.
    let bucket_limit_u32: u32 = bucket_limit
        .try_into()
        .unwrap_or_else(|_| pgrx::error!("bucket_limit must be <= {}", u32::MAX));

    let relation = unsafe { PgSearchRelation::from_pg(index.as_ptr()) };
    let standalone_context = ExprContextGuard::new();

    // Extract aggregate name to field mappings for descaling Numeric64 fields
    let agg_name_to_field = extract_agg_name_to_field(&agg.0);

    // Build a mapping of aggregate names to their numeric scales
    let mut numeric_field_scales = HashMap::default();
    if let Ok(schema) = relation.schema() {
        for (agg_name, field_name) in &agg_name_to_field {
            if let Some(search_field) = schema.search_field(field_name) {
                if let SearchFieldType::Numeric64(_, scale) = search_field.field_type() {
                    numeric_field_scales.insert(agg_name.clone(), scale);
                }
            }
        }
    }

    let aggregate = execute_aggregate(
        &relation,
        query,
        AggregateRequest::Json(serde_json::from_value(agg.0)?),
        solve_mvcc,
        memory_limit.try_into()?,
        bucket_limit_u32,
        standalone_context.as_ptr(),
        std::ptr::null_mut(), // No planstate in API context
    )?;

    if aggregate.0.is_empty() {
        Ok(JsonB(serde_json::Value::Null))
    } else {
        // Convert to JSON and descale Numeric64 fields
        let json_value = serde_json::to_value(aggregate)?;
        let descaled_json = if numeric_field_scales.is_empty() {
            json_value
        } else {
            descale_numeric_values_in_json(json_value, &numeric_field_scales)
        };
        Ok(JsonB(descaled_json))
    }
}

/// SQL: aggregate(index, query, agg, solve_mvcc=true, memory_limit=..., bucket_limit=GUC)
/// SQL: aggregate(index, query, agg, solve_mvcc=true, memory_limit=..., bucket_limit=NULL)
/// - bucket_limit=NULL => use GUC paradedb.max_term_agg_buckets
#[pg_extern]
pub fn aggregate(
    index: PgRelation,
    query: SearchQueryInput,
    agg: Json,
    solve_mvcc: default!(bool, true),
    memory_limit: default!(i64, 500000000),
    bucket_limit: default!(Option<i64>, "NULL"),
) -> Result<JsonB, Box<dyn Error>> {
    // bucket_limit NULL => use GUC
    let bucket_limit = bucket_limit.unwrap_or_else(|| gucs::max_term_agg_buckets() as i64);

    aggregate_impl(index, query, agg, solve_mvcc, memory_limit, bucket_limit)
}

#[pgrx::pg_schema]
mod pdb {
    use pgrx::aggregate::Aggregate;
    use pgrx::{pg_extern, Internal, JsonB};

    /// Placeholder aggregate for `pdb.agg(jsonb)`.
    ///
    /// This aggregate should never actually execute - it's intercepted at planning time
    /// for window functions or by AggregateScan for (GROUP BY) aggregate queries.
    ///
    /// Usage:
    /// ```sql
    /// -- Default (solve_mvcc = true)
    /// pdb.agg('{"avg": {"field": "price"}}'::jsonb)
    ///
    /// -- Disable MVCC filtering for performance  
    /// pdb.agg('{"avg": {"field": "price"}}'::jsonb, false)
    /// ```
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
            pgrx::error!(
            "pdb.agg() must be handled by ParadeDB's custom scan. \
             This error usually means the query syntax is not supported. \
             Try adding '@@@ pdb.all()' to your WHERE clause to force custom scan usage, \
             or file an issue at https://github.com/paradedb/paradedb/issues if this should be supported."
        )
        }

        fn finalize(
            _current: Self::State,
            _direct_arg: Self::OrderedSetArgs,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::Finalize {
            pgrx::error!(
            "pdb.agg() must be handled by ParadeDB's custom scan. \
             This error usually means the query syntax is not supported. \
             Try adding '@@@ paradedb.all()' to your WHERE clause to force custom scan usage, \
             or file an issue at https://github.com/paradedb/paradedb/issues if this should be supported."
        )
        }
    }

    /// Placeholder aggregate for `pdb.agg(jsonb, bool)` with explicit MVCC control.
    ///
    /// The second parameter (solve_mvcc) controls MVCC visibility filtering:
    /// - `true`: Apply MVCC filtering for transaction-accurate aggregates
    /// - `false`: Skip MVCC filtering for faster but potentially stale aggregates
    #[derive(pgrx::AggregateName, Default)]
    #[aggregate_name = "agg"]
    pub struct AggPlaceholderWithMvcc;

    #[pgrx::pg_aggregate(parallel_safe)]
    impl Aggregate<AggPlaceholderWithMvcc> for AggPlaceholderWithMvcc {
        type Args = (JsonB, bool);
        type State = Internal;
        type Finalize = JsonB;

        fn state(
            _current: Self::State,
            _arg: Self::Args,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::State {
            pgrx::error!(
            "pdb.agg() must be handled by ParadeDB's custom scan. \
             This error usually means the query syntax is not supported. \
             Try adding '@@@ pdb.all()' to your WHERE clause to force custom scan usage, \
             or file an issue at https://github.com/paradedb/paradedb/issues if this should be supported."
        )
        }

        fn finalize(
            _current: Self::State,
            _direct_arg: Self::OrderedSetArgs,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::Finalize {
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
    pub fn agg_fn_placeholder(_agg_name: &str) -> JsonB {
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

/// Get the OID of the pdb.agg() aggregate function
/// Returns InvalidOid if the function doesn't exist yet (e.g., during extension creation)
pub fn agg_funcoid() -> pgrx::pg_sys::Oid {
    lookup_pdb_function("agg", &[pgrx::pg_sys::JSONBOID])
}

/// Get the OID of the pdb.agg(jsonb, bool) aggregate function with solve_mvcc parameter
/// Returns InvalidOid if the function doesn't exist yet (e.g., during extension creation)
pub fn agg_with_solve_mvcc_funcoid() -> pgrx::pg_sys::Oid {
    lookup_pdb_function("agg", &[pgrx::pg_sys::JSONBOID, pgrx::pg_sys::BOOLOID])
}

/// Extract solve_mvcc boolean from a Const node.
/// Returns true (MVCC enabled) if the value can't be extracted or is null.
///
/// # Safety
/// The caller must ensure `const_node` is a valid pointer to a Const node.
pub unsafe fn extract_solve_mvcc_from_const(const_node: *mut pgrx::pg_sys::Const) -> bool {
    if const_node.is_null() || (*const_node).constisnull {
        return true;
    }
    let bool_datum = (*const_node).constvalue;
    pgrx::FromDatum::from_datum(bool_datum, false).unwrap_or(true)
}

/// Controls MVCC visibility filtering for aggregate computations.
///
/// This enum determines whether aggregations should apply Postgres MVCC
/// (Multi-Version Concurrency Control) filtering to ensure transaction-consistent results.
///
/// The values are designed to be extensible for future enhancements, such as:
/// - Dynamic MVCC based on query estimate (for small result sets, accuracy matters more)
/// - Sampling-based MVCC for very large result sets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MvccVisibility {
    /// Apply MVCC filtering for transaction-accurate aggregates.
    /// This is the default behavior - aggregates will only include rows
    /// visible to the current transaction.
    #[default]
    Enabled,
    /// Skip MVCC filtering for performance.
    /// Aggregates may include rows that are not visible to the current transaction,
    /// but computation will be faster.
    Disabled,
}

impl MvccVisibility {
    /// Parse from a string value (case-insensitive).
    /// Returns the default (Enabled) for unrecognized values with a warning.
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "enabled" | "true" | "on" | "1" => MvccVisibility::Enabled,
            "disabled" | "false" | "off" | "0" => MvccVisibility::Disabled,
            other => {
                pgrx::warning!(
                    "Unknown MVCC visibility mode '{}'. Using 'enabled'. \
                     Valid values: 'enabled', 'disabled'.",
                    other
                );
                MvccVisibility::Enabled
            }
        }
    }

    /// Returns true if MVCC filtering should be applied
    pub fn should_filter(&self) -> bool {
        matches!(self, MvccVisibility::Enabled)
    }
}
