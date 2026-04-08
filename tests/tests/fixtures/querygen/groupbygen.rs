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
    pub having: Option<String>,
}

impl GroupByExpr {
    pub fn to_sql(&self) -> String {
        if self.group_by_columns.is_empty() {
            if let Some(ref having) = self.having {
                format!("HAVING {}", having)
            } else {
                String::new()
            }
        } else {
            let mut result = format!("GROUP BY {}", self.group_by_columns.join(", "));
            if let Some(ref having) = self.having {
                result.push_str(&format!(" HAVING {}", having));
            }
            result
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

/// Generate an optional HAVING clause ~30% of the time when aggregates are available.
/// The HAVING clause references an aggregate already in the SELECT list.
fn arb_having(aggregates: Vec<String>) -> impl Strategy<Value = Option<String>> {
    if aggregates.is_empty() {
        Just(None).boxed()
    } else {
        // ~30% chance of generating a HAVING clause
        proptest::bool::weighted(0.3)
            .prop_flat_map(move |include_having| {
                if include_having {
                    let aggs = aggregates.clone();
                    proptest::sample::select(aggs)
                        .prop_map(|agg| {
                            // COUNT(*) always returns a non-negative integer
                            if agg == "COUNT(*)" {
                                Some(format!("{} > 0", agg))
                            } else {
                                // SUM, AVG, MIN, MAX can return NULL for empty groups,
                                // so use IS NOT NULL as a safe HAVING predicate
                                Some(format!("{} IS NOT NULL", agg))
                            }
                        })
                        .boxed()
                } else {
                    Just(None).boxed()
                }
            })
            .boxed()
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
    proptest::sample::subsequence(columns, 0..3).prop_flat_map(move |selected_columns| {
        // Generate 0-3 aggregates from the available aggregates
        // TODO: Support 3 aggregates as soon as issue #2963 is fixed
        let max_aggregates = std::cmp::min(aggregates.len(), 2);
        let agg_range = if selected_columns.is_empty() {
            // No GROUP BY - need at least one aggregate
            1..=max_aggregates
        } else {
            // GROUP BY - can have 0 to max_aggregates
            0..=max_aggregates
        };

        proptest::sample::subsequence(aggregates.clone(), agg_range).prop_flat_map(
            move |selected_aggregates| {
                if selected_columns.is_empty() {
                    // No GROUP BY - just aggregates
                    let target_list: Vec<SelectItem> = selected_aggregates
                        .iter()
                        .map(|&agg| SelectItem::Aggregate(agg.to_string()))
                        .collect();

                    // Optionally generate HAVING ~30% of the time when aggregates exist
                    let having_aggs: Vec<String> = target_list
                        .iter()
                        .filter_map(|item| match item {
                            SelectItem::Aggregate(agg) => Some(agg.clone()),
                            _ => None,
                        })
                        .collect();
                    let target_list_clone = target_list.clone();

                    arb_having(having_aggs)
                        .prop_map(move |having| GroupByExpr {
                            group_by_columns: vec![],
                            target_list: target_list_clone.clone(),
                            having,
                        })
                        .boxed()
                } else {
                    // GROUP BY - aggregates and columns.
                    // Choose a subset of columns for grouping
                    let aggregates_clone = selected_aggregates.clone();
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

                    let selected_columns_clone = selected_columns.clone();

                    // Collect aggregate strings for HAVING generation
                    let having_aggs: Vec<String> = select_items
                        .iter()
                        .filter_map(|item| match item {
                            SelectItem::Aggregate(agg) => Some(agg.clone()),
                            _ => None,
                        })
                        .collect();

                    // Generate a random permutation of the target list
                    (Just(select_items).prop_shuffle(), arb_having(having_aggs))
                        .prop_map(move |(permuted_target_list, having)| GroupByExpr {
                            group_by_columns: selected_columns_clone.clone(),
                            target_list: permuted_target_list,
                            having,
                        })
                        .boxed()
                }
            },
        )
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
            having: None,
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
            having: None,
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
            having: None,
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
            having: None,
        };
        assert_eq!(expr.to_sql(), "GROUP BY name, color");
        assert_eq!(expr.to_select_list(), "COUNT(*), name, color");
    }

    #[test]
    fn test_group_by_expr_with_having() {
        let expr = GroupByExpr {
            group_by_columns: vec!["name".to_string()],
            target_list: vec![
                SelectItem::Column("name".to_string()),
                SelectItem::Aggregate("COUNT(*)".to_string()),
            ],
            having: Some("COUNT(*) > 0".to_string()),
        };
        assert_eq!(expr.to_sql(), "GROUP BY name HAVING COUNT(*) > 0");
        assert_eq!(expr.to_select_list(), "name, COUNT(*)");
    }

    #[test]
    fn test_group_by_expr_having_without_group_by() {
        let expr = GroupByExpr {
            group_by_columns: vec![],
            target_list: vec![SelectItem::Aggregate("COUNT(*)".to_string())],
            having: Some("COUNT(*) > 0".to_string()),
        };
        assert_eq!(expr.to_sql(), "HAVING COUNT(*) > 0");
        assert_eq!(expr.to_select_list(), "COUNT(*)");
    }
}
