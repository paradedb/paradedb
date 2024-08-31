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
    pub key: Option<TantivyValue>,
    pub ctid: u64,

    /// if we have a specific order by requirement, use that first, instead of sorting by the bm25 score
    pub order_by: Option<TantivyValue>,
    pub sort_asc: bool,
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
        if let (Some(a_order_by), Some(b_order_by)) = (&self.order_by, &other.order_by) {
            let cmp = if self.sort_asc {
                a_order_by.partial_cmp(b_order_by)
            } else {
                a_order_by.partial_cmp(b_order_by).map(|o| o.reverse())
            };

            // NB:  we are called from tantivy via our use of its "TopN" collector, which sorts
            // results in descending order.  As such, for a straight-up order_by, we have to reverse
            // our understanding of ascending/descending so that the user gets back their results
            // in the order they asked for
            let cmp = cmp.map(|o| o.reverse());

            match cmp {
                // tie-break on the key
                Some(Ordering::Equal) => other.key.partial_cmp(&self.key),
                ne => ne,
            }
        } else {
            match self.bm25.partial_cmp(&other.bm25) {
                // tie-break on the key
                Some(Ordering::Equal) => other.key.partial_cmp(&self.key),
                ne => ne,
            }
        }
    }
}
