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

//! One execution-visible view of segment statistics.
//!
//! Between planning and execution the visible segment set changes: inserts add segments and
//! merges replace them. The snapshot is therefore taken from the execution Searcher, never from
//! planner-visible segment identities.
//!
//! Statistics are accelerators, never substitutes for the query predicate. Missing statistics
//! leave segments eligible for execution; errors opening or decoding existing statistics abort
//! the query.

mod snapshot;

pub(crate) use snapshot::SegmentStatsSnapshot;
#[cfg(any(test, feature = "pg_test"))]
pub(crate) use snapshot::test_support::{InjectedStatsFailure, STATS_OPENS, inject_stats_failure};
