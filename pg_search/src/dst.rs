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

//! Deterministic-simulation-testing (DST) hooks.
//!
//! The only module that talks to the DST vendor SDK (Antithesis), pulled in as
//! the optional `antithesis_sdk` dependency.
//!
//! Built only under `--features dst` (the instrumented build); a no-op everywhere else,
//! so production `pg_search` never links the SDK.
//!
//! [`report_merge_crash!`] is a forwarding macro, not a function, so the SDK captures the
//! assertion's location at the call site (`background_merge`) rather than here. The
//! bug-class classification stays in the `merge_crash_details` function.

#[cfg(feature = "dst")]
use pgrx::pg_sys::panic::CaughtError;

/// The DST details payload for a background-merge worker crash, or `None` when `caught` is
/// not a bug.
///
/// Returns `Some` only for bug-class errors — internal-error / corruption SQLSTATEs and
/// Rust panics. An interrupt-driven cancellation is not a bug and never reaches here:
/// `merge_index` downgrades it to a `warning!`, so the faults we deliberately inject do
/// not trip the assertion.
#[cfg(feature = "dst")]
pub(crate) fn merge_crash_details(caught: &CaughtError) -> Option<serde_json::Value> {
    use pgrx::PgSqlErrorCode::*;

    let report = match caught {
        // A Rust panic (a failed `expect`, or the `panic!("failed to merge…")` in
        // `merge_index`) is always a bug.
        CaughtError::RustPanic { ereport, .. } => ereport,
        // A Postgres/ereport error is a bug only when it is an internal error or
        // corruption; cancellations, shutdowns and connection faults are the chaos we
        // are injecting, not defects.
        CaughtError::PostgresError(report) | CaughtError::ErrorReport(report) => {
            if !matches!(
                report.sql_error_code(),
                ERRCODE_INTERNAL_ERROR
                    | ERRCODE_DATA_CORRUPTED
                    | ERRCODE_INDEX_CORRUPTED
                    | ERRCODE_ASSERT_FAILURE
            ) {
                return None;
            }
            report
        }
    };

    Some(serde_json::json!({
        "sqlstate": format!("{:?}", report.sql_error_code()),
        "message": report.message(),
    }))
}

/// Unreachable: surface a background-merge worker crash as an invariant violation so the
/// run fails instead of silently passing on a crash that only ever reached the container's
/// stdout.
#[cfg(feature = "dst")]
macro_rules! report_merge_crash {
    ($caught:expr) => {
        if let Some(details) = $crate::dst::merge_crash_details($caught) {
            ::antithesis_sdk::assert_unreachable!("pg_search background merge crashed", &details);
        }
    };
}

#[cfg(not(feature = "dst"))]
macro_rules! report_merge_crash {
    ($caught:expr) => {{
        let _ = $caught;
    }};
}

pub(crate) use report_merge_crash;
