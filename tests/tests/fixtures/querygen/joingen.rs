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
use std::fmt::{self, Display};

#[derive(Clone, Debug)]
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
    table_idx: usize,
    on_left_table: Option<usize>,
    on_left_col: Option<String>,
    on_right_col: Option<String>,
}

#[derive(Clone, Debug)]
struct JoinExpr {
    initial_table: usize,
    steps: Vec<JoinStep>,
}

impl JoinExpr {
    fn used_tables(&self) -> Vec<usize> {
        let mut v = Vec::with_capacity(1 + self.steps.len());
        v.push(self.initial_table);
        for s in &self.steps {
            v.push(s.table_idx);
        }
        v
    }

    /// Render as a SQL fragment, e.g.
    /// `FROM t0 JOIN t1 ON t0.a = t1.b LEFT JOIN t2 ON t1.x = t2.y ...`
    fn to_sql(&self, names: &[String]) -> (String, Vec<String>) {
        let mut used = Vec::new();
        let mut sql = format!("FROM {}", names[self.initial_table]);
        used.push(names[self.initial_table].clone());

        for step in &self.steps {
            sql.push_str(" ");
            sql.push_str(&step.join_type.to_string());
            sql.push(' ');
            sql.push_str(&names[step.table_idx]);
            used.push(names[step.table_idx].clone()); // rhs
            if let JoinType::Cross = step.join_type {
                // no ON clause
            } else {
                let lt = step.on_left_table.unwrap();
                let lc = step.on_left_col.as_ref().unwrap();
                let rc = step.on_right_col.as_ref().unwrap();
                sql.push_str(&format!(
                    " ON {}.{} = {}.{}",
                    names[lt], lc, names[step.table_idx], rc
                ));
                used.push(names[lt].clone()); // lhs
                used.push(names[step.table_idx].clone()); // rhs
            }
        }
        (sql, used)
    }
}

/// The generator itself.  On each `next()` it returns one more `JoinExpr.to_sql(...)`.
pub struct JoinGenerator {
    table_names: Vec<String>,
    table_cols: Vec<Vec<String>>,
    size_to_exprs: Vec<Vec<JoinExpr>>,
    current_size: usize,
    current_index: usize,
}

impl JoinGenerator {
    /// `tables` is a Vec of `(table_name, Vec<column_names>)`.
    pub fn new<T: AsRef<str>>(tables: Vec<(T, Vec<T>)>) -> Self {
        let mut names = Vec::with_capacity(tables.len());
        let mut cols = Vec::with_capacity(tables.len());

        for (tn, cs) in tables {
            names.push(tn.as_ref().to_string());
            cols.push(cs.into_iter().map(|c| c.as_ref().to_string()).collect());
        }

        // size 0: unused
        // size 1: “ FROM each_table ”
        let mut size_to_exprs = Vec::new();
        size_to_exprs.push(Vec::new());
        let one = (0..names.len())
            .map(|i| JoinExpr {
                initial_table: i,
                steps: Vec::new(),
            })
            .collect();
        size_to_exprs.push(one);

        // we’ll start yielding at size = 2 (i.e. real joins of 2+ tables)
        JoinGenerator {
            table_names: names,
            table_cols: cols,
            size_to_exprs,
            current_size: 2,
            current_index: 0,
        }
    }

    /// Build all partial joins that use exactly `size` tables,
    /// by extending each expr of size−1 with one new table.
    fn build_size(&self, size: usize) -> Vec<JoinExpr> {
        let mut out = Vec::new();
        let types = [
            JoinType::Inner,
            JoinType::Left,
            JoinType::Right,
            JoinType::Full,
            JoinType::Cross,
        ];

        for expr in &self.size_to_exprs[size - 1] {
            let used = expr.used_tables();
            for new_idx in 0..self.table_names.len() {
                if used.contains(&new_idx) {
                    continue;
                }

                for jt in &types {
                    match jt {
                        JoinType::Cross => {
                            let mut e = expr.clone();
                            e.steps.push(JoinStep {
                                join_type: jt.clone(),
                                table_idx: new_idx,
                                on_left_table: None,
                                on_left_col: None,
                                on_right_col: None,
                            });
                            out.push(e);
                        }
                        _ => {
                            // for non‐CROSS joins, try matching new table to every old one
                            for &left_idx in &used {
                                for lc in &self.table_cols[left_idx] {
                                    for rc in &self.table_cols[new_idx] {
                                        let mut e = expr.clone();
                                        e.steps.push(JoinStep {
                                            join_type: jt.clone(),
                                            table_idx: new_idx,
                                            on_left_table: Some(left_idx),
                                            on_left_col: Some(lc.clone()),
                                            on_right_col: Some(rc.clone()),
                                        });
                                        out.push(e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        out
    }
}

impl Iterator for JoinGenerator {
    type Item = (String, Vec<String>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // if we've already built all possible JOIN‐sizes, terminate
            if self.current_size > self.table_names.len() {
                return None;
            }
            // ensure bucket exists
            if self.current_size >= self.size_to_exprs.len() {
                let built = self.build_size(self.current_size);
                self.size_to_exprs.push(built);
            }

            let bucket = &self.size_to_exprs[self.current_size];
            if self.current_index < bucket.len() {
                let expr = &bucket[self.current_index];
                self.current_index += 1;
                return Some(expr.to_sql(&self.table_names));
            } else {
                // advance to the next size
                self.current_size += 1;
                self.current_index = 0;
            }
        }
    }
}
