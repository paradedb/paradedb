// Copyright (c) 2023-2024 Retake, Inc.
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

use crate::postgres::types::TantivyValue;
use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

/// A custom score struct for ordering Tantivy results.
/// For use with the `stable` sorting feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIndexScore {
    pub bm25: f32,
    pub key: TantivyValue,
    pub ctid: u64,
}

// We do these custom trait impls, because we want these to be sortable so:
// - they're ordered descending by bm25 score.
// - in case of a tie, they're ordered by ascending key.

impl PartialEq for SearchIndexScore {
    fn eq(&self, other: &Self) -> bool {
        self.bm25 == other.bm25 && self.key == other.key
    }
}

impl PartialOrd for SearchIndexScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.bm25 == other.bm25 {
            other.key.partial_cmp(&self.key)
        } else {
            self.bm25.partial_cmp(&other.bm25)
        }
    }
}

// Same as SearchIndexScore but used for queries using order_by_field
// Order ascending by score and in case of a tie also by key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderByScore {
    pub score: u64,
    pub key: TantivyValue,
    pub ctid: u64,
}

impl PartialEq for OrderByScore {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score && self.key == other.key
    }
}

impl PartialOrd for OrderByScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.score == other.score {
            self.key.partial_cmp(&other.key)
        } else {
            self.score.partial_cmp(&other.score)
        }
    }
}
