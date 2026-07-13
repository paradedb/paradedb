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

//! Helpers for emitting a WARNING at most once per SQL statement.
//!
//! Unlike [`crate::postgres::planner_warnings`], which batches planner diagnostics and flushes them
//! at the end of planning, these fire during execution and dedup against the currently-executing
//! statement so a warning emitted from many `@@@` predicate nodes surfaces just once.

use std::cell::Cell;
use std::thread::LocalKey;

use pgrx::pg_sys;

/// Uniquely identifies the currently-executing statement (its start timestamp).
pub type StatementId = pg_sys::TimestampTz;

/// Sentinel [`StatementId`] that never matches a real statement, for initializing trackers.
pub const NEVER: StatementId = pg_sys::TimestampTz::MIN;

/// Identifies the currently-executing statement. Distinct statements -- including each `EXECUTE` of
/// a prepared statement -- get distinct values, letting us dedup a warning to once per statement.
pub fn current_statement_id() -> StatementId {
    unsafe { pg_sys::GetCurrentStatementStartTimestamp() }
}

/// Emit `message` as a WARNING at most once per statement, tracked by `warned_at`.
pub fn warn_once_per_statement(warned_at: &'static LocalKey<Cell<StatementId>>, message: &str) {
    let stmt_id = current_statement_id();
    if warned_at.with(|last| last.replace(stmt_id)) != stmt_id {
        pgrx::warning!("{message}");
    }
}
