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

pub mod score;
pub mod snippet;
pub mod window_agg;

fn missing_scan_error(function: &str) -> ! {
    pgrx::pg_sys::panic::ErrorReport::new(
        pgrx::PgSqlErrorCode::ERRCODE_FEATURE_NOT_SUPPORTED,
        format!("`{function}` must be evaluated by a ParadeDB scan"),
        pgrx::function_name!(),
    )
    .set_detail(
        "A search predicate must remain after query optimization. PostgreSQL can remove redundant search predicates.",
    )
    .set_hint(format!(
        "Use `EXPLAIN` to check whether a ParadeDB scan evaluates `{function}`, or remove it from the query.",
    ))
    .report(pgrx::PgLogLevel::ERROR);
    unreachable!()
}
