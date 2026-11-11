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

pub use index_read::postgres::*;

pub mod build;
pub mod build_logging;
pub mod build_parallel;
pub mod build_partitioning;
pub mod delete;
#[cfg(not(any(feature = "pg17", feature = "pg18")))]
pub mod fake_aminsertcleanup;
pub mod insert;
pub mod merge;
pub mod mutable_sweep;
pub mod promotion_sweep;
pub mod vacuum;
