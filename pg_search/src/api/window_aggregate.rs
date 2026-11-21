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

//! Internal window function placeholder.

pub use common::api::window_aggregate::{make_window_agg_placeholder, window_agg_oid};

#[pgrx::pg_schema]
mod pdb {
    use pgrx::prelude::*;

    /// It is STABLE because the aggregate it stands for holds for the whole statement. Postgres
    /// computes a volatile target after the sort, above the scan, where no scan can replace it:
    /// over a partitioned table that is a Result above the Merge Append (#6783).
    #[pg_extern(stable, parallel_safe, name = "window_agg")]
    pub fn window_agg_placeholder(window_aggregate_json: &str) -> i64 {
        pgrx::error!(
            "window_agg placeholder should not be executed - custom scan should have intercepted this. JSON: {}",
            window_aggregate_json
        )
    }
}
