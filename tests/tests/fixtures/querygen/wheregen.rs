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

use std::fmt::{Debug, Display};

use proptest::prelude::*;

use crate::fixtures::querygen::Column;

#[derive(Clone, Debug)]
pub enum Expr {
    Atom {
        name: String,
        value: String,
        is_indexed: bool,
    },
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

impl Expr {
    pub fn to_sql(&self, indexed_op: &str) -> String {
        match self {
            Expr::Atom {
                name,
                value,
                is_indexed,
            } => {
                let op = if *is_indexed { indexed_op } else { " = " };
                format!("{name} {op} {value}")
            }
            Expr::Not(e) => {
                format!("NOT ({})", e.to_sql(indexed_op))
            }
            Expr::And(l, r) => {
                format!("({}) AND ({})", l.to_sql(indexed_op), r.to_sql(indexed_op))
            }
            Expr::Or(l, r) => {
                format!("({}) OR ({})", l.to_sql(indexed_op), r.to_sql(indexed_op))
            }
        }
    }
}

pub fn arb_wheres(tables: Vec<impl AsRef<str>>, columns: &[Column]) -> impl Strategy<Value = Expr> {
    let tables = tables
        .into_iter()
        .map(|t| t.as_ref().to_owned())
        .collect::<Vec<_>>();
    let columns = columns
        .iter()
        .map(|c| (c.name.to_owned(), c.sample_value.to_owned(), c.is_indexed))
        .collect::<Vec<_>>();

    // leaves: the atomic predicate. select a table, and a column.
    let atom = proptest::sample::select(tables).prop_flat_map(move |table| {
        proptest::sample::select::<Expr>(
            columns
                .iter()
                .map(|(col, val, is_indexed)| Expr::Atom {
                    name: format!("{table}.{col}"),
                    value: val.clone(),
                    is_indexed: *is_indexed,
                })
                .collect::<Vec<_>>(),
        )
    });

    // inner nodes
    atom.prop_recursive(
        5, // target depth
        8, // target total size
        3, // expected size of each node
        |child| {
            prop_oneof![
                child.clone().prop_map(|c| Expr::Not(Box::new(c.clone()))),
                (child.clone(), child.clone())
                    .prop_map(|(l, r)| Expr::And(Box::new(l), Box::new(r))),
                (child.clone(), child.clone())
                    .prop_map(|(l, r)| Expr::Or(Box::new(l), Box::new(r))),
            ]
        },
    )
}
