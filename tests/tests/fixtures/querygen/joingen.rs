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

use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};

use proptest::prelude::*;
use proptest::sample;
use proptest_derive::Arbitrary;

#[derive(Arbitrary, Copy, Clone, Debug)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

impl Display for JoinType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            JoinType::Inner => "JOIN",
            JoinType::Left => "LEFT JOIN",
            JoinType::Right => "RIGHT JOIN",
            JoinType::Full => "FULL JOIN",
            JoinType::Cross => "CROSS JOIN",
        };
        f.write_str(s)
    }
}

#[derive(Clone, Debug)]
struct JoinStep {
    join_type: JoinType,
    table: String,
    on_left_table: Option<String>,
    on_left_col: Option<String>,
    on_right_col: Option<String>,
}

#[derive(Clone)]
pub struct JoinExpr {
    initial_table: String,
    steps: Vec<JoinStep>,
}

impl JoinExpr {
    pub fn used_tables(&self) -> Vec<&str> {
        let mut v = Vec::with_capacity(1 + self.steps.len());
        v.push(self.initial_table.as_str());
        for s in &self.steps {
            v.push(s.table.as_str());
        }
        v
    }

    /// Render as a SQL fragment, e.g.
    /// `FROM t0 JOIN t1 ON t0.a = t1.b LEFT JOIN t2 ON t1.x = t2.y ...`
    pub fn to_sql(&self) -> String {
        let mut join_clause = format!("FROM {}", self.initial_table);

        for step in &self.steps {
            join_clause.push(' ');
            join_clause.push_str(&step.join_type.to_string());
            join_clause.push(' ');
            join_clause.push_str(&step.table);
            if let JoinType::Cross = step.join_type {
                // no ON clause
            } else {
                let lt = step.on_left_table.as_ref().unwrap();
                let lc = step.on_left_col.as_ref().unwrap();
                let rc = step.on_right_col.as_ref().unwrap();
                join_clause.push_str(&format!(" ON {}.{} = {}.{}", lt, lc, step.table, rc));
            }
        }

        join_clause
    }
}

impl Debug for JoinExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JoinExpr")
            .field("sql", &self.to_sql())
            .finish_non_exhaustive()
    }
}

///
/// Generate all possible joins involving exactly the given tables.
///
pub fn arb_joins(
    join_types: impl Strategy<Value = JoinType>,
    tables_to_join: Vec<impl AsRef<str>>,
    columns: Vec<impl AsRef<str>>,
) -> impl Strategy<Value = JoinExpr> {
    let tables_to_join = tables_to_join
        .into_iter()
        .map(|tn| tn.as_ref().to_string())
        .collect::<Vec<_>>();
    let table_cols = columns
        .into_iter()
        .map(|cn| cn.as_ref().to_string())
        .collect::<Vec<_>>();

    // Choose joins and join columns.
    let join_count = tables_to_join.len() - 1;
    (
        proptest::collection::vec(join_types, join_count),
        proptest::sample::subsequence(table_cols, join_count),
    )
        .prop_map(move |(join_types, join_columns)| {
            // Construct a JoinExpr for the tables and joins.
            let mut tables_to_join = tables_to_join.clone().into_iter();
            let initial_table = tables_to_join
                .next()
                .expect("At least one table in a join.");

            let mut previous_table = initial_table.clone();
            let mut steps = Vec::with_capacity(join_types.len());
            for ((join_type, join_column), table_to_join) in
                join_types.into_iter().zip(join_columns).zip(tables_to_join)
            {
                match join_type {
                    JoinType::Cross => {
                        steps.push(JoinStep {
                            join_type,
                            table: table_to_join.clone(),
                            on_left_table: None,
                            on_left_col: None,
                            on_right_col: None,
                        });
                    }
                    _ => {
                        steps.push(JoinStep {
                            join_type,
                            table: table_to_join.clone(),
                            on_left_table: Some(previous_table.to_owned()),
                            on_left_col: Some(join_column.clone()),
                            on_right_col: Some(join_column),
                        });
                    }
                }
                previous_table = table_to_join;
            }

            JoinExpr {
                initial_table,
                steps,
            }
        })
}
