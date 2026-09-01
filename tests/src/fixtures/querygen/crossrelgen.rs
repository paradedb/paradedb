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

//! Generator for cross-relation predicates.
//!
//! These are predicates that compare columns from different tables
//! (e.g., `a.age > b.age`), which cannot be pushed down to individual Tantivy
//! scan nodes and must be evaluated as post-join filters.

use proptest::prelude::*;
use proptest_derive::Arbitrary;
use std::fmt::{self, Debug, Display};

/// Comparison operators for cross-relation predicates.
#[derive(Arbitrary, Copy, Clone, Debug)]
pub enum CrossRelOp {
    Lt,
    Le,
    Gt,
    Ge,
}

impl Display for CrossRelOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            CrossRelOp::Lt => "<",
            CrossRelOp::Le => "<=",
            CrossRelOp::Gt => ">",
            CrossRelOp::Ge => ">=",
        };
        f.write_str(s)
    }
}

/// A cross-relation predicate expression comparing columns from two tables.
#[derive(Clone, Debug)]
pub struct CrossRelExpr {
    pub left_table: String,
    pub left_col: String,
    pub op: CrossRelOp,
    pub right_table: String,
    pub right_col: String,
}

impl CrossRelExpr {
    /// Convert to SQL fragment, e.g. "users.age > products.age"
    pub fn to_sql(&self) -> String {
        format!(
            "{}.{} {} {}.{}",
            self.left_table, self.left_col, self.op, self.right_table, self.right_col
        )
    }
}

/// Generate arbitrary cross-relation predicate expressions comparing columns between two tables
/// chosen from the given set of tables.
///
/// Creates predicates comparing numeric columns between two tables,
/// such as `table_a.col > table_b.col`.
///
/// # Arguments
/// * `tables` - List of table names to select pairs from (requires at least 2 tables)
/// * `numeric_columns` - List of numeric column names that can be compared
pub fn arb_cross_rel_expr<S: AsRef<str>, C: AsRef<str>>(
    tables: Vec<S>,
    numeric_columns: Vec<C>,
) -> impl Strategy<Value = CrossRelExpr> + use<S, C> {
    let tables: Vec<String> = tables.into_iter().map(|t| t.as_ref().to_string()).collect();
    let columns: Vec<String> = numeric_columns
        .into_iter()
        .map(|c| c.as_ref().to_string())
        .collect();

    assert!(
        tables.len() >= 2,
        "arb_cross_rel_expr requires at least 2 tables, got {}",
        tables.len()
    );

    (
        any::<CrossRelOp>(),
        proptest::sample::subsequence(tables, 2),
        proptest::bool::ANY,
        proptest::sample::select(columns.clone()),
        proptest::sample::select(columns),
    )
        .prop_map(move |(op, pair, swap, left_col, right_col)| {
            let (left_table, right_table) = if swap {
                (pair[1].clone(), pair[0].clone())
            } else {
                (pair[0].clone(), pair[1].clone())
            };
            CrossRelExpr {
                left_table,
                left_col,
                op,
                right_table,
                right_col,
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_rel_expr_to_sql() {
        let expr = CrossRelExpr {
            left_table: "users".to_string(),
            left_col: "age".to_string(),
            op: CrossRelOp::Gt,
            right_table: "products".to_string(),
            right_col: "price".to_string(),
        };
        assert_eq!(expr.to_sql(), "users.age > products.price");
    }

    #[test]
    fn test_cross_rel_op_display() {
        assert_eq!(format!("{}", CrossRelOp::Lt), "<");
        assert_eq!(format!("{}", CrossRelOp::Le), "<=");
        assert_eq!(format!("{}", CrossRelOp::Gt), ">");
        assert_eq!(format!("{}", CrossRelOp::Ge), ">=");
    }
}
