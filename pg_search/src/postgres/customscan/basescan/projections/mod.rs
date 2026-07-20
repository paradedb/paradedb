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

/// Emit a clean, user-actionable `ERROR` when a ParadeDB projection placeholder
/// function (e.g. `pdb.score()`, `pdb.snippet()`) is invoked at execution time.
///
/// These `#[pg_extern]` shells only run when the planner did NOT choose a
/// ParadeDB `CustomScan` for the referenced relation — typically because the
/// query has no ParadeDB search predicate (`@@@`, `|||`, ...) on that relation.
/// Historically the shells `panic!()`d with a generic "Unsupported query
/// shape. Please report at github" message, which reads like an internal bug
/// and gives the user nothing actionable. This helper emits a proper Postgres
/// `ERROR` with `ERRCODE_FEATURE_NOT_SUPPORTED`, a detail explaining why the
/// shell was reached, and a hint pointing at the required search predicate.
///
/// See paradedb/paradedb#5052.
pub fn unsupported_projection_error(func_name: &'static str) -> ! {
    use pgrx::pg_sys::panic::ErrorReport;
    use pgrx::{PgLogLevel, PgSqlErrorCode};

    ErrorReport::new(
        PgSqlErrorCode::ERRCODE_FEATURE_NOT_SUPPORTED,
        format!("`{func_name}` requires a ParadeDB search predicate on its referenced relation"),
        func_name,
    )
    .set_detail(
        "This function is a placeholder that is only evaluated inside a ParadeDB \
         `CustomScan`. It was reached at execution time because the planner did \
         not choose a `CustomScan` for the referenced relation.",
    )
    .set_hint(
        "Add a ParadeDB search predicate (e.g. `@@@` or `|||`) on the relation \
         referenced by this function, or remove the function from the query.",
    )
    .report(PgLogLevel::ERROR);

    // ereport(ERROR, ...) longjmps and never returns, but pgrx types `.report()`
    // as `()`. `unreachable!()` closes the `!` return type without adding a
    // panic path.
    unreachable!("ereport(ERROR) does not return")
}
