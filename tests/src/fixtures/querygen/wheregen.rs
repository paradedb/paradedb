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
    All {
        table: String,
        key_col: String,
    },
    IsNull(String),
    IsNotNull(String),
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
            Expr::All { table, key_col } => {
                if indexed_op == "@@@" {
                    format!("{table}.{key_col} @@@ pdb.all()")
                } else {
                    format!("{table}.{key_col} IS NOT NULL")
                }
            }
            Expr::IsNull(name) => {
                format!("{name} IS NULL")
            }
            Expr::IsNotNull(name) => {
                format!("{name} IS NOT NULL")
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

    pub fn referenced_tables(&self) -> std::collections::BTreeSet<String> {
        let mut tables = std::collections::BTreeSet::new();
        self.collect_tables(&mut tables);
        tables
    }

    fn collect_tables(&self, tables: &mut std::collections::BTreeSet<String>) {
        match self {
            Expr::Atom { name, .. } | Expr::IsNull(name) | Expr::IsNotNull(name) => {
                if let Some((tbl, _)) = name.split_once('.') {
                    tables.insert(tbl.to_string());
                }
            }
            Expr::All { table, .. } => {
                tables.insert(table.clone());
            }
            Expr::Not(e) => e.collect_tables(tables),
            Expr::And(l, r) | Expr::Or(l, r) => {
                l.collect_tables(tables);
                r.collect_tables(tables);
            }
        }
    }

    /// Check if this expression contains at least one search operator (`@@@`).
    pub fn has_search_operator(&self) -> bool {
        match self {
            Expr::Atom { is_indexed, .. } => *is_indexed,
            Expr::All { .. } => true,
            Expr::IsNull(_) | Expr::IsNotNull(_) => false,
            Expr::Not(e) => e.has_search_operator(),
            Expr::And(l, r) | Expr::Or(l, r) => l.has_search_operator() || r.has_search_operator(),
        }
    }

    /// Check if this expression contains an OR where the operands reference different tables.
    pub fn has_cross_table_or(&self) -> bool {
        match self {
            Expr::Atom { .. } | Expr::All { .. } | Expr::IsNull(_) | Expr::IsNotNull(_) => false,
            Expr::Not(e) => e.has_cross_table_or(),
            Expr::And(l, r) => l.has_cross_table_or() || r.has_cross_table_or(),
            Expr::Or(l, r) => {
                let l_tables = l.referenced_tables();
                let r_tables = r.referenced_tables();
                l_tables != r_tables || l.has_cross_table_or() || r.has_cross_table_or()
            }
        }
    }
}

// `tables` is a named generic rather than `impl AsRef<str>` so the return type can use precise
// capturing: the strategy copies `columns` into owned data, and `use<S>` keeps it from capturing
// the `columns` lifetime, which edition 2024 would otherwise pull into the opaque type.
pub fn arb_wheres<S: AsRef<str>>(
    tables: Vec<S>,
    columns: &[Column],
) -> impl Strategy<Value = Expr> + use<S> {
    let tables = tables
        .into_iter()
        .map(|t| t.as_ref().to_owned())
        .collect::<Vec<_>>();
    let key_col = columns
        .iter()
        .find(|c| c.is_primary_key)
        .map(|c| c.name.to_owned())
        .unwrap_or_else(|| "id".to_string());
    let primary_table = tables[0].clone();
    let where_columns = columns
        .iter()
        .filter(|c| c.is_whereable)
        .map(|c| {
            (
                c.name.to_owned(),
                c.sample_value.to_owned(),
                c.is_indexed,
                c.is_primary_key,
            )
        })
        .collect::<Vec<_>>();

    // leaves: atomic predicate. select a table and column.
    let atom = (
        proptest::sample::select(tables),
        proptest::sample::select(where_columns),
        prop_oneof![
            4 => Just(0),
            1 => Just(1),
            1 => Just(2),
        ],
    )
        .prop_map(
            move |(table, (col, val, is_indexed, is_primary_key), kind)| {
                let name = format!("{table}.{col}");
                // Primary key columns are NOT NULL, so IS NULL is constant FALSE and IS NOT NULL is constant TRUE.
                // Inside an OR branch, `(search_op) OR (pk IS NOT NULL)` simplifies to TRUE, causing PostgreSQL
                // to strip the WHERE clause entirely during constant-folding. Only generate null tests for nullable columns.
                match kind {
                    1 if !is_primary_key => Expr::IsNull(name),
                    2 if !is_primary_key => Expr::IsNotNull(name),
                    _ => Expr::Atom {
                        name,
                        value: val,
                        is_indexed,
                    },
                }
            },
        );

    // inner nodes, wrapped so every expression is guaranteed to have at least one search operator
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
    .prop_map(move |expr| {
        if !expr.has_search_operator() {
            Expr::And(
                Box::new(expr),
                Box::new(Expr::All {
                    table: primary_table.clone(),
                    key_col: key_col.clone(),
                }),
            )
        } else {
            expr
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::querygen::Column;

    #[test]
    fn test_null_test_sql() {
        let is_null = Expr::IsNull("products.color".to_string());
        assert_eq!(is_null.to_sql(" = "), "products.color IS NULL");
        assert_eq!(is_null.to_sql("@@@"), "products.color IS NULL");

        let is_not_null = Expr::IsNotNull("orders.quantity".to_string());
        assert_eq!(is_not_null.to_sql(" = "), "orders.quantity IS NOT NULL");
        assert_eq!(is_not_null.to_sql("@@@"), "orders.quantity IS NOT NULL");

        let combined = Expr::And(Box::new(is_null), Box::new(is_not_null));
        assert_eq!(
            combined.to_sql(" = "),
            "(products.color IS NULL) AND (orders.quantity IS NOT NULL)"
        );

        let tables = combined.referenced_tables();
        assert!(tables.contains("products"));
        assert!(tables.contains("orders"));
    }

    #[test]
    fn test_all_sql() {
        let all = Expr::All {
            table: "users".to_string(),
            key_col: "id".to_string(),
        };
        assert_eq!(all.to_sql(" = "), "users.id IS NOT NULL");
        assert_eq!(all.to_sql("@@@"), "users.id @@@ pdb.all()");
        assert!(all.has_search_operator());

        let is_null = Expr::IsNull("products.color".to_string());
        assert!(!is_null.has_search_operator());

        let combined = Expr::And(Box::new(is_null), Box::new(all));
        assert!(combined.has_search_operator());
    }

    proptest! {
        #[test]
        fn test_arb_wheres_generates_null_checks(
            expr in arb_wheres(vec!["users", "products"], &[Column::new("color", "VARCHAR", "'blue'").whereable(true)])
        ) {
            let sql_pg = expr.to_sql(" = ");
            let sql_bm25 = expr.to_sql("@@@");
            assert!(!sql_pg.is_empty());
            assert!(!sql_bm25.is_empty());
            assert!(expr.has_search_operator());
            assert!(sql_bm25.contains("@@@"));
        }
    }
}
