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
use std::fmt::Display;

pub trait SqlValue {
    fn to_sql_literal(&self) -> String;
}

impl<D: Display> SqlValue for D {
    fn to_sql_literal(&self) -> String {
        format!("'{}'", self.to_string().replace('\'', "''"))
    }
}

#[derive(Clone)]
pub enum Expr<V: Clone + Eq> {
    Atom(usize, V), // column index, literal
    Not(Box<Expr<V>>),
    And(Box<Expr<V>>, Box<Expr<V>>),
    Or(Box<Expr<V>>, Box<Expr<V>>),
}

impl<V: Clone + Eq + SqlValue> Expr<V> {
    fn eval(&self, row: &[V]) -> bool {
        match self {
            Expr::Atom(i, v) => &row[*i] == v,
            Expr::Not(e) => !e.eval(row),
            Expr::And(l, r) => l.eval(row) && r.eval(row),
            Expr::Or(l, r) => l.eval(row) || r.eval(row),
        }
    }

    fn to_sql(&self, op: &str, cols: &[String]) -> String {
        match self {
            Expr::Atom(i, v) => {
                format!("{} {op} {}", cols[*i], v.to_sql_literal())
            }
            Expr::Not(e) => {
                format!("NOT ({})", e.to_sql(op, cols))
            }
            Expr::And(l, r) => {
                format!("({}) AND ({})", l.to_sql(op, cols), r.to_sql(op, cols))
            }
            Expr::Or(l, r) => {
                format!("({}) OR ({})", l.to_sql(op, cols), r.to_sql(op, cols))
            }
        }
    }
}

pub struct WhereGenerator<V: Clone + Eq + SqlValue> {
    cols: Vec<String>,
    row: Vec<V>,
    op: String,
    size_to_exprs: Vec<Vec<Expr<V>>>,
    current_size: usize,
    current_index: usize,
}

impl<V: Clone + Eq + SqlValue> WhereGenerator<V> {
    pub fn new(operator: &str, data: Vec<(impl AsRef<str>, V)>) -> Self {
        // size 1 = the atomic predicates for each column, on that row
        let atoms = (0..data.len())
            .map(|i| Expr::Atom(i, data[i].1.clone()))
            .collect();

        let mut cols = Vec::with_capacity(data.len());
        let mut row = Vec::with_capacity(data.len());
        for (col, value) in data {
            cols.push(col.as_ref().to_string());
            row.push(value);
        }

        let mut size_to_exprs = Vec::new();
        size_to_exprs.push(Vec::new()); // placeholder for size 0
        size_to_exprs.push(atoms); // size 1

        WhereGenerator {
            cols,
            row,
            op: operator.to_string(),
            size_to_exprs,
            current_size: 1,
            current_index: 0,
        }
    }

    /// Produce _all_ Expr of the given `size`, by
    /// - unary: `NOT` of every expr of size size−1  
    /// - binary: for every split `i + j + 1 == size`, combine size-i and size-j with `AND` and `OR`
    fn build_size(&self, size: usize) -> Vec<Expr<V>> {
        let mut out = Vec::new();
        // unary NOT
        for e in &self.size_to_exprs[size - 1] {
            out.push(Expr::Not(Box::new(e.clone())));
        }
        // binary AND/OR
        for i in 1..size - 1 {
            let j = size - 1 - i;
            for left in &self.size_to_exprs[i] {
                for right in &self.size_to_exprs[j] {
                    out.push(Expr::And(Box::new(left.clone()), Box::new(right.clone())));
                    out.push(Expr::Or(Box::new(left.clone()), Box::new(right.clone())));
                }
            }
        }
        out
    }
}

impl<V: Clone + Eq + SqlValue> Iterator for WhereGenerator<V> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // ensure size bucket exists
            if self.current_size >= self.size_to_exprs.len() {
                let exprs = self.build_size(self.current_size);
                self.size_to_exprs.push(exprs);
            }

            let bucket = &self.size_to_exprs[self.current_size];
            if self.current_index < bucket.len() {
                let expr = &bucket[self.current_index];
                self.current_index += 1;

                // only yield it if it actually matches our target row
                if expr.eval(&self.row) {
                    return Some(expr.to_sql(&self.op, &self.cols));
                }
                // otherwise skip it
            } else {
                // advance to next size
                self.current_size += 1;
                self.current_index = 0;
            }
        }
    }
}
