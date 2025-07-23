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

use proptest::prelude::*;
use std::fmt::Debug;

/// Represents a GROUP BY expression
#[derive(Clone, Debug)]
pub struct GroupByExpr {
    pub columns: Vec<String>,
}

impl GroupByExpr {
    pub fn to_sql(&self) -> String {
        if self.columns.is_empty() {
            String::new()
        } else {
            format!("GROUP BY {}", self.columns.join(", "))
        }
    }

    pub fn to_select_list(&self, aggregates: &[&str]) -> String {
        let mut select_items = Vec::new();

        // Add grouping columns
        select_items.extend(self.columns.iter().cloned());

        // Add aggregates
        select_items.extend(aggregates.iter().map(|&s| s.to_string()));

        select_items.join(", ")
    }
}

/// Generate arbitrary GROUP BY expressions for the given columns
pub fn arb_group_by(columns: Vec<String>) -> impl Strategy<Value = GroupByExpr> {
    // Generate 0-3 grouping columns from the available columns
    (0..=3.min(columns.len())).prop_flat_map(move |group_size| {
        if group_size == 0 {
            // No GROUP BY
            Just(GroupByExpr { columns: vec![] }).boxed()
        } else {
            // Choose a subset of columns for grouping
            proptest::sample::subsequence(columns.clone(), group_size)
                .prop_map(|selected_columns| GroupByExpr {
                    columns: selected_columns,
                })
                .boxed()
        }
    })
}

/// Generate arbitrary aggregate functions
pub fn arb_aggregates() -> impl Strategy<Value = Vec<&'static str>> {
    // For now, only support COUNT(*) since that's what's implemented
    Just(vec!["COUNT(*)"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_by_expr_empty() {
        let expr = GroupByExpr { columns: vec![] };
        assert_eq!(expr.to_sql(), "");
        assert_eq!(expr.to_select_list(&["COUNT(*)"]), "COUNT(*)");
    }

    #[test]
    fn test_group_by_expr_single() {
        let expr = GroupByExpr {
            columns: vec!["name".to_string()],
        };
        assert_eq!(expr.to_sql(), "GROUP BY name");
        assert_eq!(expr.to_select_list(&["COUNT(*)"]), "name, COUNT(*)");
    }

    #[test]
    fn test_group_by_expr_multiple() {
        let expr = GroupByExpr {
            columns: vec!["name".to_string(), "color".to_string()],
        };
        assert_eq!(expr.to_sql(), "GROUP BY name, color");
        assert_eq!(expr.to_select_list(&["COUNT(*)"]), "name, color, COUNT(*)");
    }
}
