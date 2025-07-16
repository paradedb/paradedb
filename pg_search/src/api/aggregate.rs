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

use pgrx::{default, pg_extern, Json, JsonB, PgRelation};

use crate::aggregate::execute_aggregate;
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
    Ok(JsonB(execute_aggregate(
        &relation,
        query,
        agg.0,
        solve_mvcc,
        memory_limit.try_into()?,
        bucket_limit.try_into()?,
    )?))
}
