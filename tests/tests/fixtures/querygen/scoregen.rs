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

//! Generator for score-based expressions in queries.
//!
//! Generates ORDER BY clauses using `paradedb.score()` function
//! for BM25 relevance-based ordering.

use proptest::prelude::*;
use proptest_derive::Arbitrary;
use std::fmt::{self, Display};

/// Sort direction for ORDER BY clauses.
#[derive(Arbitrary, Copy, Clone, Debug)]
pub enum SortDir {
    Asc,
    Desc,
}

impl Display for SortDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SortDir::Asc => f.write_str("ASC"),
            SortDir::Desc => f.write_str("DESC"),
        }
    }
}

/// Generate an arbitrary score-based ORDER BY expression.
///
/// Creates expressions like `paradedb.score(table.key_field) DESC`
/// for relevance-based ordering in BM25 queries.
///
/// # Arguments
/// * `table` - The table name containing the BM25 index
/// * `key_field` - The key field of the BM25 index (typically "id")
pub fn arb_score_order(
    table: impl AsRef<str>,
    key_field: impl AsRef<str>,
) -> impl Strategy<Value = String> {
    let table = table.as_ref().to_string();
    let key_field = key_field.as_ref().to_string();

    any::<SortDir>().prop_map(move |dir| format!("paradedb.score({}.{}) {}", table, key_field, dir))
}

/// Generate a score ORDER BY that always uses DESC (most relevant first).
///
/// This is the most common use case for score ordering.
pub fn score_order_desc(table: impl AsRef<str>, key_field: impl AsRef<str>) -> String {
    format!(
        "paradedb.score({}.{}) DESC",
        table.as_ref(),
        key_field.as_ref()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_dir_display() {
        assert_eq!(format!("{}", SortDir::Asc), "ASC");
        assert_eq!(format!("{}", SortDir::Desc), "DESC");
    }

    #[test]
    fn test_score_order_desc() {
        assert_eq!(
            score_order_desc("users", "id"),
            "paradedb.score(users.id) DESC"
        );
    }
}
