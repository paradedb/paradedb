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

/// Represents an item in the SELECT list
#[derive(Clone, Debug, PartialEq)]
pub enum SelectItem {
    Column(String),
    Aggregate(String),
}

/// Represents a GROUP BY expression with an explicit target list
#[derive(Clone, Debug)]
pub struct GroupByExpr {
    pub group_by_columns: Vec<String>,
    pub target_list: Vec<SelectItem>,
}

impl GroupByExpr {
    pub fn to_sql(&self) -> String {
        if self.group_by_columns.is_empty() {
            String::new()
        } else {
            format!("GROUP BY {}", self.group_by_columns.join(", "))
        }
    }

    pub fn to_select_list(&self) -> String {
        self.target_list
            .iter()
            .map(|item| match item {
                SelectItem::Column(col) => col.clone(),
                SelectItem::Aggregate(agg) => agg.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Generate arbitrary GROUP BY expressions with random target list ordering
pub fn arb_group_by(
    columns: Vec<impl AsRef<str>>,
    aggregates: Vec<&'static str>,
) -> impl Strategy<Value = GroupByExpr> {
    let columns = columns
        .into_iter()
        .map(|c| c.as_ref().to_string())
        .collect::<Vec<_>>();

    // Generate 0-3 grouping columns from the available columns
    (0..=3.min(columns.len())).prop_flat_map(move |group_size| {
        if group_size == 0 {
            // No GROUP BY - just aggregates
            let target_list = aggregates
                .iter()
                .map(|&agg| SelectItem::Aggregate(agg.to_string()))
                .collect();

            Just(GroupByExpr {
                group_by_columns: vec![],
                target_list,
            })
            .boxed()
        } else {
            // Choose a subset of columns for grouping
            let aggregates_clone = aggregates.clone();
            proptest::sample::subsequence(columns.clone(), group_size)
                .prop_flat_map(move |selected_columns| {
                    // Create select items for columns and aggregates
                    let mut select_items = Vec::new();

                    // Add all selected columns as SelectItem::Column
                    for col in &selected_columns {
                        select_items.push(SelectItem::Column(col.clone()));
                    }

                    // Add all aggregates as SelectItem::Aggregate
                    for &agg in &aggregates_clone {
                        select_items.push(SelectItem::Aggregate(agg.to_string()));
                    }

                    // Generate a random permutation of the target list
                    let n_items = select_items.len();
                    let selected_columns_clone = selected_columns.clone();
                    proptest::collection::vec(0..n_items, n_items).prop_filter_map(
                        "unique permutation",
                        move |indices| {
                            // Check if all indices are unique (valid permutation)
                            let mut sorted = indices.clone();
                            sorted.sort();
                            sorted.dedup();
                            if sorted.len() == n_items {
                                // Use indices to reorder select_items
                                let shuffled: Vec<_> = indices
                                    .into_iter()
                                    .map(|i| select_items[i].clone())
                                    .collect();
                                Some(GroupByExpr {
                                    group_by_columns: selected_columns_clone.clone(),
                                    target_list: shuffled,
                                })
                            } else {
                                None
                            }
                        },
                    )
                })
                .boxed()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_by_expr_empty() {
        let expr = GroupByExpr {
            group_by_columns: vec![],
            target_list: vec![SelectItem::Aggregate("COUNT(*)".to_string())],
        };
        assert_eq!(expr.to_sql(), "");
        assert_eq!(expr.to_select_list(), "COUNT(*)");
    }

    #[test]
    fn test_group_by_expr_single_column_first() {
        let expr = GroupByExpr {
            group_by_columns: vec!["name".to_string()],
            target_list: vec![
                SelectItem::Column("name".to_string()),
                SelectItem::Aggregate("COUNT(*)".to_string()),
            ],
        };
        assert_eq!(expr.to_sql(), "GROUP BY name");
        assert_eq!(expr.to_select_list(), "name, COUNT(*)");
    }

    #[test]
    fn test_group_by_expr_single_aggregate_first() {
        let expr = GroupByExpr {
            group_by_columns: vec!["name".to_string()],
            target_list: vec![
                SelectItem::Aggregate("COUNT(*)".to_string()),
                SelectItem::Column("name".to_string()),
            ],
        };
        assert_eq!(expr.to_sql(), "GROUP BY name");
        assert_eq!(expr.to_select_list(), "COUNT(*), name");
    }

    #[test]
    fn test_group_by_expr_multiple_mixed_order() {
        let expr = GroupByExpr {
            group_by_columns: vec!["name".to_string(), "color".to_string()],
            target_list: vec![
                SelectItem::Aggregate("COUNT(*)".to_string()),
                SelectItem::Column("name".to_string()),
                SelectItem::Column("color".to_string()),
            ],
        };
        assert_eq!(expr.to_sql(), "GROUP BY name, color");
        assert_eq!(expr.to_select_list(), "COUNT(*), name, color");
    }
}
