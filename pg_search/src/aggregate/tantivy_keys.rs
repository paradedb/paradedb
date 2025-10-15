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

//! Constants for Tantivy aggregation DSL keys
//!
//! This module centralizes all string literals used as keys in Tantivy's aggregation JSON DSL.
//! Using constants instead of string literals makes the code more maintainable and less error-prone.

/// Key for the sentinel filter aggregation (ensures all groups are generated)
pub const FILTER_SENTINEL: &str = "filter_sentinel";

/// Prefix for numbered filter aggregations (e.g., "filter_0", "filter_1")
pub const FILTER_PREFIX: &str = "filter_";

/// Key for grouped/nested terms aggregations
pub const GROUPED: &str = "grouped";

/// Key for the filtered aggregate result within a filter aggregation
pub const FILTERED_AGG: &str = "filtered_agg";

/// Key for document count in aggregation results
pub const DOC_COUNT: &str = "doc_count";

/// Key for the hidden document count aggregation (used for NULL handling)
pub const HIDDEN_DOC_COUNT: &str = "_doc_count";

/// Key for sum_other_doc_count (indicates truncated results)
pub const SUM_OTHER_DOC_COUNT: &str = "sum_other_doc_count";

/// Prefix for numbered group aggregations (e.g., "group_0", "group_1")
pub const GROUP_PREFIX: &str = "group_";

/// Key for the aggregation value field
pub const VALUE: &str = "value";

/// Key for average aggregation results
pub const AVG: &str = "avg";

/// Key for sum aggregation results
pub const SUM: &str = "sum";

/// Key for min aggregation results
pub const MIN: &str = "min";

/// Key for max aggregation results
pub const MAX: &str = "max";

/// Key for the "_key" field in bucket results (the grouped value)
pub const KEY: &str = "_key";

/// Key for the bucket key in terms aggregation results
pub const BUCKETS: &str = "buckets";

/// Helper function to create a numbered filter key (e.g., "filter_0")
#[inline]
pub fn filter_key(index: usize) -> String {
    format!("{}{}", FILTER_PREFIX, index)
}

/// Helper function to create a numbered group key (e.g., "group_0")
#[inline]
pub fn group_key(index: usize) -> String {
    format!("{}{}", GROUP_PREFIX, index)
}
