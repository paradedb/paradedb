// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::query::SearchQueryInput;
use serde::{Deserialize, Serialize};

/// Query tree with recursive cost estimates.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryWithEstimates {
    pub query: SearchQueryInput,
    pub query_type: String,
    pub estimated_docs: Option<usize>,
    pub children: Vec<QueryWithEstimates>,
}

impl QueryWithEstimates {
    /// Create a new node without children.
    pub fn new(query: SearchQueryInput, query_type: impl Into<String>) -> Self {
        Self {
            query,
            query_type: query_type.into(),
            estimated_docs: None,
            children: Vec::new(),
        }
    }

    /// Create a new node with children.
    pub fn with_children(
        query: SearchQueryInput,
        query_type: impl Into<String>,
        children: Vec<QueryWithEstimates>,
    ) -> Self {
        Self {
            query,
            query_type: query_type.into(),
            estimated_docs: None,
            children,
        }
    }

    /// Set the estimated document count.
    pub fn set_estimate(&mut self, estimated_docs: usize) {
        self.estimated_docs = Some(estimated_docs);
    }

    pub fn children(&self) -> &[QueryWithEstimates] {
        &self.children
    }

    pub fn children_mut(&mut self) -> &mut Vec<QueryWithEstimates> {
        &mut self.children
    }

    /// Recursively traverse and apply function to each node.
    pub fn traverse_mut<F>(&mut self, depth: usize, f: &mut F)
    where
        F: FnMut(&mut Self, usize),
    {
        f(self, depth);
        for child in &mut self.children {
            child.traverse_mut(depth + 1, f);
        }
    }
}
