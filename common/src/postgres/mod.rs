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

pub mod buffile;
pub mod catalog;
pub mod composite;
pub mod condition_variable;
pub mod customscan;
pub mod datetime;
pub mod jsonb_support;
pub mod locks;
pub mod node;
pub mod pdb_owned_value;
pub mod planner_warnings;
pub mod ps_status;
pub mod range;
pub mod sequentialscan {
    pub mod keyset;
    pub use keyset::KeySet;
}
pub mod tuplesort;
pub mod types;
pub mod types_arrow;
pub mod utils;
pub mod validate;

pub mod options {
    pub use crate::gucs::{MAX_MUTABLE_SEGMENT_ROWS, MAX_TARGET_SEGMENT_COUNT};
}

unsafe extern "C" {
    pub fn IsLogicalWorker() -> bool;
}
