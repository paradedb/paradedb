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
//! ## User-Facing Function: `pdb.agg(jsonb)`, `pdb.agg(jsonb, text)` and `pdb.agg(jsonb, bool)`
//!
//! This is the public API for users to specify custom Tantivy aggregations.
//! When used in window function context (`OVER ()`), it gets intercepted at planning
//! time and replaced with `window_agg()` placeholder. The actual execution happens
//! in the custom scan using Tantivy's aggregation collectors.
//!
//! Example: `SELECT *, pdb.agg('{"avg": {"field": "price"}}'::jsonb) OVER () FROM products`
//!
//! The optional second argument is the `visibility` mode:
//! - 'transaction' (default): check transaction visibility, matching Postgres semantics
//! - 'raw': skip the checks and aggregate raw index data, faster but approximate
//! - 'threshold': check only when the estimated matching row count is below
//!   `paradedb.visibility_threshold`
//!
//! Example opting into raw index data:
//! `SELECT *, pdb.agg('{"avg": {"field": "price"}}'::jsonb, 'raw') OVER () FROM products`
//!
//! The `bool` overload is the deprecated `solve_mvcc` spelling, kept so existing
//! queries keep working: `true` means 'transaction' and `false` means 'raw'.
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

use pgrx::{FromDatum, Json, JsonB, PgRelation, default, pg_extern};
use serde::{Deserialize, Serialize};

use crate::aggregate::{AggregateRequest, execute_aggregate};
use crate::api::operator::estimate_matching_rows;
use crate::api::version::VersionInfo;
use crate::gucs;
use crate::nodecast;
use crate::postgres::customscan::aggregatescan::aggregate_type::validate_agg_json_fields;
use crate::postgres::customscan::aggregatescan::json_rewrite::rewrite_aggregate_result_json_timestamps;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::{ExprContextGuard, lookup_pdb_function};
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;

fn aggregate_impl(
    index: PgRelation,
    query: SearchQueryInput,
    agg: Json,
    visibility: MvccVisibility,
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

    // Validate aggregation fields exist and are supported before executing.
    // This path bypasses the planner, so we validate here directly.
    let schema = SearchIndexSchema::open(&relation).ok();
    if let Some(schema) = schema.as_ref()
        && let Err(e) = validate_agg_json_fields(&agg.0, schema)
    {
        pgrx::error!("{}", e);
    }

    let standalone_context = ExprContextGuard::new();
    // need a copy of the original request json for rewriting later
    let agg_json = agg.0.clone();

    let aggregate = execute_aggregate(
        &relation,
        query,
        AggregateRequest::Json(serde_json::from_value(agg.0)?),
        visibility,
        memory_limit.try_into()?,
        bucket_limit_u32,
        standalone_context.as_ptr(),
        std::ptr::null_mut(), // No planstate in API context
        None,                 // No bitmap intersection in API context
    )?;

    if aggregate.0.is_empty() {
        return Ok(JsonB(serde_json::Value::Null));
    }

    let mut output = serde_json::to_value(aggregate)?;
    // rewrite the aggregate results so we get human readable datetime values
    if relation.created_by_version().stores_datetimes_in_i64()
        && let (Some(schema), Some(request_obj), Some(output_obj)) = (
            schema.as_ref(),
            agg_json.as_object(),
            output.as_object_mut(),
        )
    {
        for (name, request) in request_obj.iter() {
            if let Some(response) = output_obj.get_mut(name) {
                rewrite_aggregate_result_json_timestamps(response, request, schema);
            }
        }
    }

    Ok(JsonB(output))
}

/// Resolve the UDF's visibility from its two spellings.
///
/// `visibility` is the supported parameter; `solve_mvcc` is deprecated and maps onto
/// it. Supplying both is an error rather than a precedence rule, because either
/// choice of winner would silently ignore something the caller asked for.
fn resolve_udf_visibility(solve_mvcc: Option<bool>, visibility: Option<String>) -> MvccVisibility {
    match (solve_mvcc, visibility) {
        (Some(_), Some(_)) => pgrx::error!(
            "cannot specify both `solve_mvcc` and `visibility`. \
             `solve_mvcc` is deprecated: pass `visibility` alone."
        ),
        (Some(solve_mvcc), None) => {
            let visibility = if solve_mvcc {
                MvccVisibility::Transaction
            } else {
                MvccVisibility::Raw
            };
            pgrx::warning!(
                "`solve_mvcc` is deprecated and will be removed in a future release. \
                 Use `visibility => '{}'` instead.",
                visibility.as_sql_value()
            );
            visibility
        }
        (None, Some(visibility)) => MvccVisibility::from_sql_value(&visibility),
        (None, None) => MvccVisibility::default(),
    }
}

/// SQL: aggregate(index, query, agg, solve_mvcc=NULL, memory_limit=..., bucket_limit=GUC, visibility=NULL)
/// - bucket_limit=NULL => use GUC paradedb.max_term_agg_buckets
/// - solve_mvcc is deprecated in favor of visibility; see `resolve_udf_visibility`
///
/// `visibility` is appended after `bucket_limit` rather than replacing `solve_mvcc`
/// in place so that positional calls written against the old signature still resolve.
#[pg_extern]
pub fn aggregate(
    index: PgRelation,
    query: SearchQueryInput,
    agg: Json,
    solve_mvcc: default!(Option<bool>, "NULL"),
    memory_limit: default!(i64, 500000000),
    bucket_limit: default!(Option<i64>, "NULL"),
    visibility: default!(Option<String>, "NULL"),
) -> Result<JsonB, Box<dyn Error>> {
    // bucket_limit NULL => use GUC
    let bucket_limit = bucket_limit.unwrap_or_else(|| gucs::max_term_agg_buckets() as i64);
    let visibility = resolve_udf_visibility(solve_mvcc, visibility);

    aggregate_impl(index, query, agg, visibility, memory_limit, bucket_limit)
}

#[pgrx::pg_schema]
mod pdb {
    use pgrx::aggregate::Aggregate;
    use pgrx::{Internal, JsonB, pg_extern};

    /// Placeholder aggregate for `pdb.agg(jsonb)`.
    ///
    /// This aggregate should never actually execute - it's intercepted at planning time
    /// for window functions or by AggregateScan for (GROUP BY) aggregate queries.
    ///
    /// Usage:
    /// ```sql
    /// -- Default (visibility => 'transaction')
    /// pdb.agg('{"avg": {"field": "price"}}'::jsonb)
    ///
    /// -- Aggregate raw index data for performance
    /// pdb.agg('{"avg": {"field": "price"}}'::jsonb, 'raw')
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

    /// Placeholder aggregate for the deprecated `pdb.agg(jsonb, bool)` overload.
    ///
    /// The second parameter is the old `solve_mvcc` boolean:
    /// - `true`: equivalent to `visibility => 'transaction'`
    /// - `false`: equivalent to `visibility => 'raw'`
    ///
    /// Kept so that queries written against the old signature keep working.
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

    /// Placeholder aggregate for `pdb.agg(jsonb, text)`, the `visibility` overload.
    ///
    /// The second parameter is the visibility mode: `'transaction'` (the default),
    /// `'raw'`, or `'threshold'`. An unknown-typed literal such as `'threshold'`
    /// resolves here rather than to the `bool` overload, because Postgres prefers
    /// the string category when disambiguating an untyped literal.
    ///
    /// Keep the struct name short. `pg_aggregate` derives `<snake>_<snake>_finalize`
    /// from it, so each character costs two and Postgres truncates past 63. The
    /// current name lands at 61.
    #[derive(pgrx::AggregateName, Default)]
    #[aggregate_name = "agg"]
    pub struct AggPlaceholderVisibility;

    #[pgrx::pg_aggregate(parallel_safe)]
    impl Aggregate<AggPlaceholderVisibility> for AggPlaceholderVisibility {
        type Args = (JsonB, String);
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

/// Get the OID of the deprecated pdb.agg(jsonb, bool) aggregate function
/// Returns InvalidOid if the function doesn't exist yet (e.g., during extension creation)
fn agg_with_solve_mvcc_funcoid() -> pgrx::pg_sys::Oid {
    lookup_pdb_function("agg", &[pgrx::pg_sys::JSONBOID, pgrx::pg_sys::BOOLOID])
}

/// Get the OID of the pdb.agg(jsonb, text) aggregate function with the `visibility` parameter
/// Returns InvalidOid if the function doesn't exist yet (e.g., during extension creation)
fn agg_with_visibility_funcoid() -> pgrx::pg_sys::Oid {
    lookup_pdb_function("agg", &[pgrx::pg_sys::JSONBOID, pgrx::pg_sys::TEXTOID])
}

/// The OIDs of every `pdb.agg()` overload: the one-argument form, the deprecated
/// `(jsonb, bool)` form, and the `(jsonb, text)` form.
///
/// Each entry is a catalog lookup, so hoist this out of any loop that walks an
/// expression tree rather than calling `is_agg_funcoid` per node.
pub fn agg_funcoids() -> [u32; 3] {
    [
        agg_funcoid().to_u32(),
        agg_with_solve_mvcc_funcoid().to_u32(),
        agg_with_visibility_funcoid().to_u32(),
    ]
}

/// True for any `pdb.agg()` overload. Convenient for a one-off check; use
/// `agg_funcoids()` when the check is inside a tree walk.
pub fn is_agg_funcoid(aggfnoid: u32) -> bool {
    agg_funcoids().contains(&aggfnoid)
}

/// Decode the visibility argument of a `pdb.agg()` call.
///
/// `second_arg` is the second argument's expression, or `None` for the one-argument
/// overload. A non-`Const` or NULL argument, and any overload without a visibility
/// argument, yield the default: an undecodable mode must not silently downgrade
/// accuracy.
///
/// # Safety
/// The caller must ensure `second_arg`, when present, is a valid `Node` pointer.
pub unsafe fn visibility_from_agg_arg(
    aggfnoid: u32,
    second_arg: Option<*mut pgrx::pg_sys::Node>,
) -> MvccVisibility {
    let Some(const_node) = second_arg.and_then(|node| nodecast!(Const, T_Const, node)) else {
        return MvccVisibility::default();
    };

    if aggfnoid == agg_with_solve_mvcc_funcoid().to_u32() {
        if extract_solve_mvcc_from_const(const_node) {
            MvccVisibility::Transaction
        } else {
            MvccVisibility::Raw
        }
    } else if aggfnoid == agg_with_visibility_funcoid().to_u32() {
        extract_visibility_from_const(const_node)
    } else {
        MvccVisibility::default()
    }
}

/// The spec and visibility of a `pdb.agg()` call, or `None` when the spec is not
/// a non-null constant. Field validation and the plan shape both need the spec at
/// plan time, so a parameter cannot stand in for it.
///
/// # Safety
/// The caller must ensure `spec_arg` and `visibility_arg`, when present, are valid
/// `Node` pointers.
pub unsafe fn pdb_agg_spec(
    aggfnoid: u32,
    spec_arg: *mut pgrx::pg_sys::Node,
    visibility_arg: Option<*mut pgrx::pg_sys::Node>,
) -> Option<(serde_json::Value, MvccVisibility)> {
    let const_node = nodecast!(Const, T_Const, spec_arg)?;
    let spec = JsonB::from_datum((*const_node).constvalue, (*const_node).constisnull)?.0;
    Some((spec, visibility_from_agg_arg(aggfnoid, visibility_arg)))
}

/// Extract solve_mvcc boolean from a Const node.
/// Returns true (MVCC enabled) if the value can't be extracted or is null.
///
/// # Safety
/// The caller must ensure `const_node` is a valid pointer to a Const node.
unsafe fn extract_solve_mvcc_from_const(const_node: *mut pgrx::pg_sys::Const) -> bool {
    if const_node.is_null() || (*const_node).constisnull {
        return true;
    }
    let bool_datum = (*const_node).constvalue;
    pgrx::FromDatum::from_datum(bool_datum, false).unwrap_or(true)
}

/// Extract a `visibility` mode from a `text` Const node. A NULL or undecodable
/// value yields the default; an unrecognized string is an error.
///
/// # Safety
/// The caller must ensure `const_node` is a valid pointer to a Const node.
unsafe fn extract_visibility_from_const(const_node: *mut pgrx::pg_sys::Const) -> MvccVisibility {
    if const_node.is_null() || (*const_node).constisnull {
        return MvccVisibility::default();
    }
    let datum = (*const_node).constvalue;
    match <String as pgrx::FromDatum>::from_datum(datum, false) {
        Some(value) => MvccVisibility::from_sql_value(&value),
        None => MvccVisibility::default(),
    }
}

/// Controls transaction visibility filtering for aggregate computations.
///
/// Aggregates read the index rather than the heap, so an index entry for a row that
/// is dead or invisible to the current snapshot is only excluded if it is checked
/// against the heap. This enum decides whether that check runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MvccVisibility {
    /// Apply visibility checking, so aggregates count only rows visible to the
    /// current transaction. The default, and the only mode that matches vanilla
    /// Postgres semantics.
    #[default]
    Transaction,
    /// Skip visibility checking and aggregate raw index data. Faster, and
    /// approximate whenever the index holds entries for rows the transaction
    /// cannot see.
    Raw,
    /// Apply visibility checking only when the query's estimated matching row
    /// count is below `paradedb.visibility_threshold`. Resolved per execution,
    /// not per plan.
    Threshold,
}

impl MvccVisibility {
    /// Parse a SQL-level `visibility` value (case-insensitive), or `None` when the
    /// value is not one of the recognized spellings.
    ///
    /// The legacy `solve_mvcc` spellings are accepted as aliases so that queries
    /// written against the old parameter keep working.
    ///
    /// Kept free of `pgrx::error!` so it stays a pure function. Reporting the error
    /// is `from_sql_value`'s job: a unit test that reached the reporting path would
    /// pull Postgres's `ereport` symbols into the test binary, which does not link
    /// on Linux.
    pub fn try_from_sql_value(value: &str) -> Option<Self> {
        Some(match value.trim().to_lowercase().as_str() {
            // The boolean spellings are the ones Postgres itself accepts for a
            // `boolean` input, since this value used to be one.
            "transaction" | "true" | "t" | "yes" | "y" | "on" | "1" | "enabled" | "always" => {
                MvccVisibility::Transaction
            }
            "raw" | "false" | "f" | "no" | "n" | "off" | "0" | "disabled" | "never" => {
                MvccVisibility::Raw
            }
            "threshold" | "estimated" => MvccVisibility::Threshold,
            _ => return None,
        })
    }

    /// Parse a SQL-level `visibility` value, erroring on an unrecognized one rather
    /// than guessing. Guessing wrong either costs accuracy silently or costs the
    /// performance the caller asked for.
    pub fn from_sql_value(value: &str) -> Self {
        Self::try_from_sql_value(value).unwrap_or_else(|| {
            pgrx::error!(
                "unrecognized visibility mode '{value}'. \
                 Valid values: 'transaction' (default), 'raw', 'threshold'."
            )
        })
    }

    /// Resolve to whether visibility checking actually runs for an execution
    /// that scans the given indexes. One decision covers all of them: a
    /// per-source decision could hide a deleted row on one side of a join and
    /// show it on the other.
    ///
    /// `Threshold` estimates each query's matching row count, which costs one
    /// extra single-segment index open per source. The largest estimate stands
    /// for the query, since that is where a heap check costs and where a dead
    /// tuple hides best. Anything that cannot be estimated falls back to
    /// checking: an unknown row count must not silently downgrade accuracy.
    ///
    /// The estimate opens the index without an expression context, so a query
    /// carrying heap filters or unsolved Postgres expressions is not estimated at
    /// all. Building a Tantivy query for those shapes requires the context, and
    /// the accurate side of the branch is the safe place to land.
    pub fn resolve_filtering_for_sources<'a>(
        &self,
        sources: impl IntoIterator<Item = (&'a PgSearchRelation, &'a SearchQueryInput)>,
    ) -> bool {
        match self {
            MvccVisibility::Transaction => true,
            MvccVisibility::Raw => false,
            MvccVisibility::Threshold => {
                let mut largest = 0;
                for (indexrel, query) in sources {
                    if query.has_heap_filters() || query.has_postgres_expressions() {
                        return true;
                    }
                    match estimate_matching_rows(indexrel, query.clone()) {
                        Some(rows) => largest = largest.max(rows),
                        None => return true,
                    }
                }
                largest < gucs::visibility_threshold()
            }
        }
    }

    /// The single visibility a query runs under, given every `pdb.agg()` setting in
    /// it. Two different settings are an error even when one of them came from an
    /// omitted argument, since an omitted argument is indistinguishable from an
    /// explicit `'transaction'` by the time it is seen here.
    pub fn resolve_shared(settings: impl Iterator<Item = MvccVisibility>) -> MvccVisibility {
        let mut resolved: Option<MvccVisibility> = None;
        for visibility in settings {
            match resolved {
                None => resolved = Some(visibility),
                Some(previous) if previous == visibility => {}
                Some(previous) => pgrx::error!(
                    "pdb.agg() calls have contradicting visibility settings: \
                     '{}' and '{}'. All pdb.agg() calls in a query must use the same \
                     visibility value, and omitting the argument selects '{}'.",
                    previous.as_sql_value(),
                    visibility.as_sql_value(),
                    MvccVisibility::default().as_sql_value()
                ),
            }
        }
        resolved.unwrap_or_default()
    }

    /// The canonical SQL spelling, for error and deprecation messages.
    pub fn as_sql_value(&self) -> &'static str {
        match self {
            MvccVisibility::Transaction => "transaction",
            MvccVisibility::Raw => "raw",
            MvccVisibility::Threshold => "threshold",
        }
    }

    /// [`Self::resolve_filtering_for_sources`] for a single index.
    pub fn resolve_filtering(&self, indexrel: &PgSearchRelation, query: &SearchQueryInput) -> bool {
        self.resolve_filtering_for_sources(std::iter::once((indexrel, query)))
    }
}

#[cfg(test)]
mod tests {
    use super::MvccVisibility;

    // These exercise `try_from_sql_value`, never `from_sql_value`. The erroring
    // wrapper calls `pgrx::error!`, and referencing it from a unit test pulls
    // Postgres's `ereport` symbols into the lib test binary, which does not link
    // on Linux. The SQL-level error is covered by the `visibility` regress test.

    #[test]
    fn parses_canonical_visibility_values() {
        assert_eq!(
            MvccVisibility::try_from_sql_value("transaction"),
            Some(MvccVisibility::Transaction)
        );
        assert_eq!(
            MvccVisibility::try_from_sql_value("raw"),
            Some(MvccVisibility::Raw)
        );
        assert_eq!(
            MvccVisibility::try_from_sql_value("threshold"),
            Some(MvccVisibility::Threshold)
        );
    }

    #[test]
    fn parses_legacy_solve_mvcc_spellings() {
        for value in ["true", "t", "yes", "y", "on", "1", "enabled", "always"] {
            assert_eq!(
                MvccVisibility::try_from_sql_value(value),
                Some(MvccVisibility::Transaction),
                "{value} should map to transaction"
            );
        }
        for value in ["false", "f", "no", "n", "off", "0", "disabled", "never"] {
            assert_eq!(
                MvccVisibility::try_from_sql_value(value),
                Some(MvccVisibility::Raw),
                "{value} should map to raw"
            );
        }
        assert_eq!(
            MvccVisibility::try_from_sql_value("estimated"),
            Some(MvccVisibility::Threshold)
        );
    }

    #[test]
    fn parsing_ignores_case_and_surrounding_whitespace() {
        assert_eq!(
            MvccVisibility::try_from_sql_value("  ThReSHoLd \t"),
            Some(MvccVisibility::Threshold)
        );
        assert_eq!(
            MvccVisibility::try_from_sql_value("RAW"),
            Some(MvccVisibility::Raw)
        );
    }

    #[test]
    fn rejects_unrecognized_values() {
        for value in ["nonsense", "", "  ", "transactional", "rawish", "10000"] {
            assert_eq!(
                MvccVisibility::try_from_sql_value(value),
                None,
                "{value:?} should not parse"
            );
        }
    }

    #[test]
    fn transaction_is_the_default() {
        assert_eq!(MvccVisibility::default(), MvccVisibility::Transaction);
    }

    #[test]
    fn sql_values_round_trip() {
        for visibility in [
            MvccVisibility::Transaction,
            MvccVisibility::Raw,
            MvccVisibility::Threshold,
        ] {
            assert_eq!(
                MvccVisibility::try_from_sql_value(visibility.as_sql_value()),
                Some(visibility)
            );
        }
    }
}
