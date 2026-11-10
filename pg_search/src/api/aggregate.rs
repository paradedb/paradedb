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

use std::error::Error;

use pgrx::{Json, JsonB, default, pg_extern};

use crate::aggregate::{AggregateRequest, execute_aggregate};
use crate::api::version::VersionInfo;
use crate::gucs;
use crate::postgres::access::IndexOid;
use crate::postgres::customscan::aggregatescan::aggregate_type::validate_agg_json_fields;
use crate::postgres::customscan::aggregatescan::json_rewrite::rewrite_aggregate_result_json_timestamps;
use crate::postgres::utils::ExprContextGuard;
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;

use planner::api::aggregate::MvccVisibility;

fn aggregate_impl(
    index: IndexOid,
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

    let relation = index.read();

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
        None,                 // No EXPLAIN instrumentation
        None,
        None,
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
    index: IndexOid,
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
    use pgrx::aggregate::{Aggregate, ParallelOption};
    use pgrx::{Internal, JsonB, pg_extern};

    #[derive(pgrx::AggregateName, Default)]
    #[aggregate_name = "agg"]
    pub struct AggPlaceholder;

    #[pgrx::pg_aggregate]
    impl Aggregate<AggPlaceholder> for AggPlaceholder {
        const PARALLEL: Option<ParallelOption> = Some(ParallelOption::Safe);
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

    #[derive(pgrx::AggregateName, Default)]
    #[aggregate_name = "agg"]
    pub struct AggPlaceholderWithMvcc;

    #[pgrx::pg_aggregate]
    impl Aggregate<AggPlaceholderWithMvcc> for AggPlaceholderWithMvcc {
        const PARALLEL: Option<ParallelOption> = Some(ParallelOption::Safe);
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

    #[derive(pgrx::AggregateName, Default)]
    #[aggregate_name = "agg"]
    pub struct AggPlaceholderVisibility;

    #[pgrx::pg_aggregate]
    impl Aggregate<AggPlaceholderVisibility> for AggPlaceholderVisibility {
        const PARALLEL: Option<ParallelOption> = Some(ParallelOption::Safe);
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

    #[derive(pgrx::AggregateName, Default)]
    #[aggregate_name = "combine_agg"]
    pub struct CombineAggStub;

    #[pgrx::pg_aggregate]
    impl Aggregate<CombineAggStub> for CombineAggStub {
        const PARALLEL: Option<ParallelOption> = Some(ParallelOption::Safe);
        type Args = JsonB;
        type State = Internal;
        type Finalize = JsonB;

        fn state(
            _current: Self::State,
            _arg: Self::Args,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::State {
            pgrx::error!(
                "`pdb.combine_agg()` must be used as `pdb.combine_agg(pdb.agg(...)) OVER ()` \
                 in a `GROUP BY` query handled by ParadeDB's custom scan."
            )
        }

        fn finalize(
            _current: Self::State,
            _direct_arg: Self::OrderedSetArgs,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::Finalize {
            pgrx::error!(
                "`pdb.combine_agg()` must be used as `pdb.combine_agg(pdb.agg(...)) OVER ()` \
                 in a `GROUP BY` query handled by ParadeDB's custom scan."
            )
        }
    }

    #[pg_extern(volatile, parallel_safe, name = "agg_fn")]
    pub fn agg_fn_placeholder(_agg_name: &str) -> JsonB {
        pgrx::error!(
            "pdb.agg_fn() placeholder should not be executed - \
             custom scan should have intercepted this."
        )
    }
}
