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

//! JoinScan executor stub.
//!
//! This module provides type definitions for JoinScan execution.
//! Actual execution implementation is in a separate commit.

#![allow(dead_code)]

use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;
use pgrx::pg_sys;

/// Result from the executor's next() call.
pub enum JoinExecResult {
    /// A visible tuple with its ctid and score.
    Visible { ctid: u64, score: f32 },
    /// No more results available.
    Eof,
}

/// Executor for iterating over search results on one side of a join.
///
/// Provides streaming iteration allowing JoinScan to fetch results incrementally.
pub struct JoinSideExecutor {
    _private: (),
}

impl JoinSideExecutor {
    /// Create a new FastField executor for batched ctid lookups.
    #[allow(unused_variables)]
    pub fn new_fast_field(
        heaprel: &PgSearchRelation,
        indexrelid: pg_sys::Oid,
        query: SearchQueryInput,
        snapshot: pg_sys::Snapshot,
        need_scores: bool,
    ) -> Self {
        unimplemented!("JoinScan execution not implemented - see next commit")
    }

    /// Get next visible tuple from the search results.
    pub fn next_visible(&mut self) -> JoinExecResult {
        unimplemented!("JoinScan execution not implemented - see next commit")
    }

    /// Reset the executor for rescanning.
    pub fn reset(&mut self) {
        unimplemented!("JoinScan execution not implemented - see next commit")
    }
}
