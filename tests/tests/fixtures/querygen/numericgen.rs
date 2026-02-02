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

//! Generators for numeric comparison expressions in property tests.
//!
//! This module provides strategies for generating numeric WHERE clause expressions
//! that test equality and range pushdown for NUMERIC columns.

use proptest::prelude::*;
use proptest_derive::Arbitrary;

use crate::fixtures::querygen::Column;

/// Comparison operators for numeric expressions.
#[derive(Debug, Clone, Copy, Arbitrary)]
pub enum ComparisonOp {
    Eq, // =
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=
}

impl ComparisonOp {
    pub fn convert_to_sql(&self) -> &'static str {
        match self {
            ComparisonOp::Eq => "=",
            ComparisonOp::Lt => "<",
            ComparisonOp::Le => "<=",
            ComparisonOp::Gt => ">",
            ComparisonOp::Ge => ">=",
        }
    }
}

/// A numeric comparison expression for WHERE clauses.
#[derive(Debug, Clone)]
pub enum NumericExpr {
    /// Simple comparison: column op value (e.g., price > 100.50)
    Comparison {
        column: String,
        op: ComparisonOp,
        value: String,
    },
    /// BETWEEN expression: column BETWEEN low AND high
    Between {
        column: String,
        low: String,
        high: String,
    },
    /// Combined AND of two numeric expressions
    And(Box<NumericExpr>, Box<NumericExpr>),
    /// Combined OR of two numeric expressions
    Or(Box<NumericExpr>, Box<NumericExpr>),
}

impl NumericExpr {
    /// Generate SQL for both PostgreSQL and BM25 queries.
    /// For numeric comparisons, both use the same SQL syntax since
    /// BM25 pushdown handles the conversion internally.
    pub fn to_sql(&self) -> String {
        match self {
            NumericExpr::Comparison { column, op, value } => {
                format!("{} {} {}", column, op.convert_to_sql(), value)
            }
            NumericExpr::Between { column, low, high } => {
                format!("{} BETWEEN {} AND {}", column, low, high)
            }
            NumericExpr::And(left, right) => {
                format!("({}) AND ({})", left.to_sql(), right.to_sql())
            }
            NumericExpr::Or(left, right) => {
                format!("({}) OR ({})", left.to_sql(), right.to_sql())
            }
        }
    }
}

/// Information about a numeric column for value generation.
#[derive(Debug, Clone)]
pub struct NumericColumnInfo {
    pub name: String,
    pub table: String,
    /// Sample values appropriate for this column's precision/scale
    pub sample_values: Vec<String>,
}

impl NumericColumnInfo {
    pub fn full_name(&self) -> String {
        format!("{}.{}", self.table, self.name)
    }
}

/// Creates NumericColumnInfo from Column definitions, filtering to numeric types.
pub fn numeric_column_infos(table: &str, columns: &[Column]) -> Vec<NumericColumnInfo> {
    columns
        .iter()
        .filter(|c| c.is_whereable && c.is_indexed)
        .filter(|c| {
            c.sql_type.to_uppercase().starts_with("NUMERIC")
                || c.sql_type.to_uppercase() == "INTEGER"
                || c.sql_type.to_uppercase() == "INT"
                || c.sql_type.to_uppercase() == "BIGINT"
                || c.sql_type.to_uppercase() == "SMALLINT"
                || c.sql_type.to_uppercase() == "REAL"
                || c.sql_type.to_uppercase() == "FLOAT"
                || c.sql_type.to_uppercase() == "DOUBLE PRECISION"
        })
        .map(|c| {
            // Generate appropriate sample values based on column type
            let sample_values = generate_sample_values(c);
            NumericColumnInfo {
                name: c.name.to_string(),
                table: table.to_string(),
                sample_values,
            }
        })
        .collect()
}

/// Generate sample values appropriate for the column type.
fn generate_sample_values(column: &Column) -> Vec<String> {
    let sql_type = column.sql_type.to_uppercase();

    if sql_type.starts_with("NUMERIC") {
        // Parse precision and scale from NUMERIC(p,s) or NUMERIC(p) or NUMERIC
        if let Some((precision, scale)) = parse_numeric_type(&sql_type) {
            generate_numeric_samples(precision, scale)
        } else {
            // Unbounded NUMERIC - use general decimal values
            vec![
                "0".to_string(),
                "1".to_string(),
                "100".to_string(),
                "1000".to_string(),
                "0.5".to_string(),
                "10.25".to_string(),
                "999.99".to_string(),
                "-1".to_string(),
                "-100".to_string(),
            ]
        }
    } else if sql_type == "INTEGER" || sql_type == "INT" {
        vec![
            "0".to_string(),
            "1".to_string(),
            "10".to_string(),
            "50".to_string(),
            "100".to_string(),
            "-1".to_string(),
            "-50".to_string(),
        ]
    } else {
        // Default numeric samples
        vec![
            "0".to_string(),
            "1".to_string(),
            "10".to_string(),
            "100".to_string(),
            "0.5".to_string(),
        ]
    }
}

/// Parse NUMERIC(precision, scale) type string.
/// Returns (precision, scale) or None for unbounded NUMERIC.
fn parse_numeric_type(sql_type: &str) -> Option<(u32, i32)> {
    // Match NUMERIC(p,s) or NUMERIC(p)
    if let Some(start) = sql_type.find('(') {
        if let Some(end) = sql_type.find(')') {
            let params = &sql_type[start + 1..end];
            let parts: Vec<&str> = params.split(',').map(|s| s.trim()).collect();

            let precision: u32 = parts.first()?.parse().ok()?;
            let scale: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

            return Some((precision, scale));
        }
    }
    None
}

/// Generate sample values appropriate for NUMERIC(precision, scale).
fn generate_numeric_samples(precision: u32, scale: i32) -> Vec<String> {
    let mut samples = vec!["0".to_string()];

    if scale == 0 {
        // Integer-like NUMERIC
        samples.extend(vec![
            "1".to_string(),
            "10".to_string(),
            "100".to_string(),
            "1000".to_string(),
            "-1".to_string(),
            "-100".to_string(),
        ]);
    } else if scale > 0 {
        // Decimal NUMERIC - generate values with appropriate scale
        let scale_factor = format!("{:.1$}", 1.0, scale as usize);
        let _ = scale_factor; // Used for format reference

        samples.extend(vec![
            format!("{:.1$}", 0.01, scale as usize),
            format!("{:.1$}", 0.5, scale as usize),
            format!("{:.1$}", 1.0, scale as usize),
            format!("{:.1$}", 10.5, scale as usize),
            format!("{:.1$}", 100.25, scale as usize),
            format!("{:.1$}", 999.99, scale as usize),
            format!("{:.1$}", -1.0, scale as usize),
            format!("{:.1$}", -50.5, scale as usize),
        ]);

        // Add the sample value from the column definition
        samples.push("99.99".to_string());
    }

    // Limit samples based on precision to avoid overflow
    if precision <= 5 {
        samples.retain(|s| {
            s.replace(['-', '.'], "")
                .chars()
                .filter(|c| c.is_ascii_digit())
                .count()
                <= precision as usize
        });
    }

    samples
}

/// Strategy to generate a single numeric comparison expression.
pub fn arb_numeric_comparison(
    tables: Vec<impl AsRef<str>>,
    columns: &[Column],
) -> impl Strategy<Value = NumericExpr> {
    let tables: Vec<String> = tables.iter().map(|t| t.as_ref().to_string()).collect();

    // Collect numeric column info for all tables
    let all_column_infos: Vec<NumericColumnInfo> = tables
        .iter()
        .flat_map(|table| numeric_column_infos(table, columns))
        .collect();

    if all_column_infos.is_empty() {
        // Return a dummy strategy if no numeric columns
        return Just(NumericExpr::Comparison {
            column: "1".to_string(),
            op: ComparisonOp::Eq,
            value: "1".to_string(),
        })
        .boxed();
    }

    // Generate comparison expressions
    proptest::sample::select(all_column_infos)
        .prop_flat_map(|col_info| {
            let column = col_info.full_name();
            let column_for_between = column.clone();
            let values = col_info.sample_values.clone();
            let values_for_between = values.clone();

            // Choose between simple comparison and BETWEEN
            prop_oneof![
                // Simple comparison (80% weight)
                8 => (any::<ComparisonOp>(), proptest::sample::select(values))
                    .prop_map(move |(op, value)| NumericExpr::Comparison {
                        column: column.clone(),
                        op,
                        value,
                    }),
                // BETWEEN expression (20% weight)
                2 => proptest::sample::subsequence(values_for_between, 2)
                    .prop_filter_map("need two values for BETWEEN", move |mut vals| {
                        if vals.len() >= 2 {
                            // Sort to ensure low <= high
                            vals.sort_by(|a, b| {
                                a.parse::<f64>()
                                    .unwrap_or(0.0)
                                    .partial_cmp(&b.parse::<f64>().unwrap_or(0.0))
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            });
                            Some(NumericExpr::Between {
                                column: column_for_between.clone(),
                                low: vals[0].clone(),
                                high: vals[1].clone(),
                            })
                        } else {
                            None
                        }
                    }),
            ]
        })
        .boxed()
}

/// Strategy to generate combined numeric expressions with AND/OR.
pub fn arb_numeric_expr(
    tables: Vec<impl AsRef<str>>,
    columns: &[Column],
) -> impl Strategy<Value = NumericExpr> {
    let tables: Vec<String> = tables.iter().map(|t| t.as_ref().to_string()).collect();
    let columns = columns.to_vec();

    arb_numeric_comparison(tables.clone(), &columns).prop_recursive(
        3, // target depth
        6, // target total size
        2, // expected size of each node
        move |child| {
            let tables = tables.clone();
            let columns = columns.clone();

            prop_oneof![
                // Just the child expression (most common)
                6 => child.clone(),
                // AND of two expressions
                2 => (child.clone(), arb_numeric_comparison(tables.clone(), &columns))
                    .prop_map(|(l, r)| NumericExpr::And(Box::new(l), Box::new(r))),
                // OR of two expressions (less common for numeric)
                2 => (child.clone(), arb_numeric_comparison(tables.clone(), &columns))
                    .prop_map(|(l, r)| NumericExpr::Or(Box::new(l), Box::new(r))),
            ]
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comparison_op_sql() {
        assert_eq!(ComparisonOp::Eq.convert_to_sql(), "=");
        assert_eq!(ComparisonOp::Lt.convert_to_sql(), "<");
        assert_eq!(ComparisonOp::Le.convert_to_sql(), "<=");
        assert_eq!(ComparisonOp::Gt.convert_to_sql(), ">");
        assert_eq!(ComparisonOp::Ge.convert_to_sql(), ">=");
    }

    #[test]
    fn test_numeric_expr_to_sql() {
        let expr = NumericExpr::Comparison {
            column: "users.price".to_string(),
            op: ComparisonOp::Gt,
            value: "100.50".to_string(),
        };
        assert_eq!(expr.to_sql(), "users.price > 100.50");

        let expr = NumericExpr::Between {
            column: "users.price".to_string(),
            low: "50.00".to_string(),
            high: "150.00".to_string(),
        };
        assert_eq!(expr.to_sql(), "users.price BETWEEN 50.00 AND 150.00");
    }

    #[test]
    fn test_parse_numeric_type() {
        assert_eq!(parse_numeric_type("NUMERIC(10,2)"), Some((10, 2)));
        assert_eq!(parse_numeric_type("NUMERIC(5)"), Some((5, 0)));
        assert_eq!(parse_numeric_type("NUMERIC"), None);
        assert_eq!(parse_numeric_type("NUMERIC(18,6)"), Some((18, 6)));
    }
}
