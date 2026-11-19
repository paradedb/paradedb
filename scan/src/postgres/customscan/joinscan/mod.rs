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

pub mod range_partitioning_rule;
pub mod visibility_filter;

use std::fmt;

/// DataFusion-facing synthetic CTID column name helper.
///
/// JoinScan exposes per-source CTID columns into DataFusion so rows can be
/// materialized back to PostgreSQL tuples after query execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CtidColumn {
    plan_position: usize,
}

impl CtidColumn {
    pub const PREFIX: &'static str = "ctid_";

    pub fn new(plan_position: usize) -> Self {
        Self { plan_position }
    }

    pub fn plan_position(self) -> usize {
        self.plan_position
    }
}

impl fmt::Display for CtidColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.plan_position)
    }
}

impl TryFrom<&str> for CtidColumn {
    type Error = ();

    fn try_from(col_name: &str) -> Result<Self, Self::Error> {
        let plan_position = col_name
            .strip_prefix(Self::PREFIX)
            .ok_or(())?
            .parse::<usize>()
            .map_err(|_| ())?;
        Ok(Self::new(plan_position))
    }
}

/// DataFusion-facing synthetic score column name helper.
///
/// JoinScan and AggregateScan format score column names as `pdb.score({table})`
/// so that EXPLAIN plans and physical plan columns clearly identify which relation
/// the score was computed from.
#[derive(Debug, Clone)]
pub struct ScoreColumn(pub String);

impl ScoreColumn {
    pub const PREFIX: &'static str = "pdb.score(";

    pub fn new(table: impl Into<String>) -> Self {
        Self(table.into())
    }

    pub fn is_score_column(name: &str) -> bool {
        name.starts_with(Self::PREFIX) && name.ends_with(')')
    }
}

impl fmt::Display for ScoreColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pdb.score({})", self.0)
    }
}

/// DataFusion-facing synthetic vector score column name helper.
///
/// JoinScan formats vector similarity score column names as `pdb.vector_score({table})`
/// to distinguish them from Tantivy BM25 `pdb.score({table})` columns.
#[derive(Debug, Clone)]
pub struct VectorScoreColumn(pub String);

impl VectorScoreColumn {
    pub const PREFIX: &'static str = "pdb.vector_score(";

    pub fn new(table: impl Into<String>) -> Self {
        Self(table.into())
    }

    pub fn is_vector_score_column(name: &str) -> bool {
        name.starts_with(Self::PREFIX) && name.ends_with(')')
    }
}

impl fmt::Display for VectorScoreColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pdb.vector_score({})", self.0)
    }
}
