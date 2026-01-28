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

use crate::api::AsCStr;
use crate::postgres::customscan::joinscan::build::JoinCSClause;
use pgrx::pg_sys;
use pgrx::pg_sys::AsPgCStr;
use pgrx::PgList;
use serde::{Deserialize, Serialize};

pub const OUTER_SCORE_ALIAS: &str = "outer_score";
pub const INNER_SCORE_ALIAS: &str = "inner_score";

/// Describes which relation a column comes from and its original attribute number.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputColumnInfo {
    /// True if the column comes from the outer relation, false for inner.
    pub is_outer: bool,
    /// The original attribute number in the source relation (1-indexed).
    /// Set to 0 for non-Var expressions (like functions).
    pub original_attno: i16,
    /// True if this column is a paradedb.score() function call.
    /// The score will be taken from current_driving_score during execution.
    #[serde(default)]
    pub is_score: bool,
}

/// Private data stored in the CustomPath/CustomScan for join operations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrivateData {
    /// The join clause containing all information about both sides and the join itself.
    pub join_clause: JoinCSClause,
    /// Mapping of output column positions to their source (outer/inner) and original attribute numbers.
    /// This is populated during planning (before setrefs) and used during execution.
    pub output_columns: Vec<OutputColumnInfo>,
}

impl PrivateData {
    pub fn new(join_clause: JoinCSClause) -> Self {
        Self {
            join_clause,
            output_columns: Vec::new(),
        }
    }

    /// Returns a reference to the join clause.
    pub fn join_clause(&self) -> &JoinCSClause {
        &self.join_clause
    }

    /// Returns a mutable reference to the join clause.
    pub fn join_clause_mut(&mut self) -> &mut JoinCSClause {
        &mut self.join_clause
    }
}

impl From<*mut pg_sys::List> for PrivateData {
    fn from(list: *mut pg_sys::List) -> Self {
        unsafe {
            let list = PgList::<pg_sys::Node>::from_pg(list);
            let node = list.get_ptr(0).unwrap();
            let content = node
                .as_c_str()
                .unwrap()
                .to_str()
                .expect("string node should be valid utf8");
            serde_json::from_str(content).unwrap()
        }
    }
}

impl From<PrivateData> for *mut pg_sys::List {
    fn from(value: PrivateData) -> Self {
        let content = serde_json::to_string(&value).unwrap();
        unsafe {
            let mut ser = PgList::new();
            ser.push(pg_sys::makeString(content.as_pg_cstr()).cast::<pg_sys::Node>());
            ser.into_pg()
        }
    }
}
