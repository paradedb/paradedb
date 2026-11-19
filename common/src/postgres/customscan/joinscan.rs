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

use pgrx::pg_sys;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub struct RelationAlias<'a> {
    name: Option<&'a str>,
}

impl<'a> RelationAlias<'a> {
    pub fn new(name: Option<&'a str>) -> Self {
        Self { name }
    }

    pub fn display(&self, index: usize) -> String {
        self.name
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("source_{}", index))
    }

    pub fn execution(&self, index: usize) -> String {
        match self.name {
            Some(alias) => {
                let sanitized: String = alias
                    .chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() || c == '_' {
                            c.to_ascii_lowercase()
                        } else {
                            '_'
                        }
                    })
                    .collect();
                let prefixed = if sanitized.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                    format!("_{sanitized}")
                } else {
                    sanitized
                };
                format!("{}_{}", prefixed, index)
            }
            None => format!("source_{}", index),
        }
    }

    pub fn warning_context(&self, heaprelid: pg_sys::Oid) -> String {
        self.name
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("relid_{}", heaprelid))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CtidColumn {
    plan_position: usize,
}

impl CtidColumn {
    const PREFIX: &'static str = "ctid_";

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

#[derive(Debug, Clone)]
pub struct ScoreColumn(String);

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

#[derive(Debug, Clone)]
pub struct VectorScoreColumn(String);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlannerRootId(usize);

impl From<usize> for PlannerRootId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<*mut pg_sys::PlannerInfo> for PlannerRootId {
    fn from(value: *mut pg_sys::PlannerInfo) -> Self {
        Self(value as usize)
    }
}
