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

use std::error::Error;

use pgrx::{default, pg_extern, FromDatum, Internal, Json, JsonB, PgRelation};
use tantivy::aggregation::agg_req::Aggregation;

use crate::aggregate::execute_aggregate_json;
use crate::postgres::rel::PgSearchRelation;
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
    Ok(JsonB(execute_aggregate_json(
        &relation,
        &query,
        agg.0,
        solve_mvcc,
        memory_limit.try_into()?,
        bucket_limit.try_into()?,
    )?))
}

/// State transition function for agg aggregate
/// This accumulates the agg definition (which should be the same across all rows)
#[pg_extern(stable, parallel_safe)]
pub fn agg_sfunc(state: Option<Internal>, agg_definition: JsonB) -> Option<Internal> {
    // On first call, validate and store the agg definition
    if state.is_none() {
        let agg_value: serde_json::Value = agg_definition.0;

        // Validate it's a valid Tantivy aggregation by attempting to deserialize
        if let Err(e) = serde_json::from_value::<Aggregation>(agg_value.clone()) {
            pgrx::error!("Invalid Tantivy aggregation definition: {}", e);
        }

        // Store the validated JSON in an Internal datum
        // We use a Box<serde_json::Value> to store it
        unsafe {
            let boxed = Box::new(agg_value);
            Some(
                pgrx::Internal::from_datum(
                    pgrx::pg_sys::Datum::from(Box::into_raw(boxed) as *mut std::ffi::c_void),
                    false,
                )
                .unwrap(),
            )
        }
    } else {
        // Just return the existing state
        state
    }
}

/// Final function for agg aggregate
/// Returns the stored agg definition
#[pg_extern(stable, parallel_safe)]
pub fn agg_finalfunc(_state: Option<Internal>) -> JsonB {
    JsonB(serde_json::json!({"result": "not supported"}))
}
