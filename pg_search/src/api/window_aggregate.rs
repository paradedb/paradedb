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

    #[pg_extern(volatile, parallel_safe, name = "window_agg")]
    pub fn window_agg_placeholder(window_aggregate_json: &str) -> i64 {
        pgrx::error!(
            "window_agg placeholder should not be executed - custom scan should have intercepted this. JSON: {}",
            window_aggregate_json
        )
    }
}
