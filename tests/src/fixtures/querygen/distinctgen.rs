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
use std::fmt::{self, Display};

use super::{Column, IndexExpression};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistinctExpr {
    /// Function call on a text column: `upper({table}.{column})`
    Upper { table: String, column: String },
    /// Null test on a column: `{table}.{column} IS NULL`
    IsNull { table: String, column: String },
    /// Arithmetic on a numeric column: `{table}.{column} * {factor}`
    Multiply {
        table: String,
        column: String,
        factor: i64,
    },
}

impl DistinctExpr {
    pub fn to_sql(&self) -> String {
        match self {
            DistinctExpr::Upper { table, column } => format!("upper({table}.{column})"),
            DistinctExpr::IsNull { table, column } => format!("{table}.{column} IS NULL"),
            DistinctExpr::Multiply {
                table,
                column,
                factor,
            } => format!("{table}.{column} * {factor}"),
        }
    }
}

impl Display for DistinctExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_sql())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistinctMode {
    /// No DISTINCT clause.
    None,
    /// Plain DISTINCT over base projected columns.
    Columns,
    /// DISTINCT with an expression.
    Expression(DistinctExpr),
}

impl DistinctMode {
    pub fn is_distinct(&self) -> bool {
        !matches!(self, DistinctMode::None)
    }

    pub fn expression(&self) -> Option<&DistinctExpr> {
        match self {
            DistinctMode::Expression(expr) => Some(expr),
            _ => None,
        }
    }
}

/// Strategy to dynamically generate a DISTINCT mode (None, Columns, or Expression)
/// derived from the available schema `columns` and `tables` without hardcoding any column names.
pub fn arb_distinct_mode<S: AsRef<str>>(
    tables: Vec<S>,
    columns: &[Column],
) -> BoxedStrategy<DistinctMode> {
    let tables: Vec<String> = tables.iter().map(|t| t.as_ref().to_string()).collect();
    if tables.is_empty() {
        return Just(DistinctMode::None).boxed();
    }

    let mut candidates = Vec::new();

    for table in &tables {
        for col in columns {
            if !col.is_groupable || !col.is_indexed {
                continue;
            }

            if let Some(expr) = col.index_expression {
                // If the column uses an index expression, only expressions matching
                // the index definition (e.g. `upper(category)`) are columnar fast fields.
                // Bare `col IS NULL` or other transforms cannot be evaluated without
                // the bare column being an indexed fast field.
                match expr {
                    IndexExpression::Upper => {
                        candidates.push(DistinctExpr::Upper {
                            table: table.clone(),
                            column: col.name.to_string(),
                        });
                    }
                    IndexExpression::LiteralNormalized => {}
                }
                continue;
            }

            // Null test is valid on any groupable indexed column
            candidates.push(DistinctExpr::IsNull {
                table: table.clone(),
                column: col.name.to_string(),
            });

            match col.sql_type {
                "TEXT" | "VARCHAR" => {
                    candidates.push(DistinctExpr::Upper {
                        table: table.clone(),
                        column: col.name.to_string(),
                    });
                }
                "INTEGER" | "BIGINT" | "SERIAL8" => {
                    candidates.push(DistinctExpr::Multiply {
                        table: table.clone(),
                        column: col.name.to_string(),
                        factor: 10,
                    });
                }
                _ => {}
            }
        }
    }

    if candidates.is_empty() {
        return prop_oneof![
            1 => Just(DistinctMode::None),
            1 => Just(DistinctMode::Columns),
        ]
        .boxed();
    }

    prop_oneof![
        2 => Just(DistinctMode::None),
        1 => Just(DistinctMode::Columns),
        1 => proptest::sample::select(candidates).prop_map(DistinctMode::Expression),
    ]
    .boxed()
}
