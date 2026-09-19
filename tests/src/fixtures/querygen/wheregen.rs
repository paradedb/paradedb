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
        sql_type: String,
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
            Expr::Atom { name, value, .. } => {
                if indexed_op == "@@@" && self.has_search_operator() {
                    format!("{name} === {value}")
                } else {
                    format!("{name} = {value}")
                }
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

    /// Check if a search operator is still in the qual after the Postgres planner simplifies it.
    ///
    /// `A OR (A AND B)` has one, but the planner reduces it to `A`. If `B` is the only search
    /// operator, a custom scan has nothing to plan on.
    pub fn planner_keeps_search_operator(&self) -> bool {
        self.to_qual(false).canonicalize().has_search_operator()
    }

    /// The qual after `eval_const_expressions`, which pushes each `NOT` down to a leaf.
    fn to_qual(&self, negated: bool) -> Qual {
        match self {
            Expr::Not(e) => e.to_qual(!negated),
            Expr::And(l, r) if !negated => Qual::and(vec![l.to_qual(false), r.to_qual(false)]),
            Expr::And(l, r) => Qual::or(vec![l.to_qual(true), r.to_qual(true)]),
            Expr::Or(l, r) if !negated => Qual::or(vec![l.to_qual(false), r.to_qual(false)]),
            Expr::Or(l, r) => Qual::and(vec![l.to_qual(true), r.to_qual(true)]),
            // The planner turns `NOT (x IS NULL)` into `x IS NOT NULL`, so the two must compare
            // equal when it looks for clauses that `OR` arms share.
            Expr::IsNull(name) => Qual::Leaf {
                sql: format!("{name} IS NULL"),
                negated,
                is_search: false,
            },
            Expr::IsNotNull(name) => Qual::Leaf {
                sql: format!("{name} IS NULL"),
                negated: !negated,
                is_search: false,
            },
            Expr::Atom { .. } | Expr::All { .. } => Qual::Leaf {
                sql: self.to_sql("@@@"),
                negated,
                is_search: self.has_search_operator(),
            },
        }
    }

    /// Check if this expression contains at least one ParadeDB search operator.
    pub fn has_search_operator(&self) -> bool {
        match self {
            Expr::Atom {
                is_indexed,
                sql_type,
                ..
            } => {
                *is_indexed
                    && matches!(
                        sql_type.as_str(),
                        "TEXT" | "VARCHAR" | "TEXT[]" | "VARCHAR[]"
                    )
            }
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

    /// Check if this expression contains a null-testing predicate (`IS NULL` or `IS NOT NULL`).
    pub fn has_null_predicate(&self) -> bool {
        match self {
            Expr::Atom { .. } | Expr::All { .. } => false,
            Expr::IsNull(_) | Expr::IsNotNull(_) => true,
            Expr::Not(e) => e.has_null_predicate(),
            Expr::And(l, r) | Expr::Or(l, r) => l.has_null_predicate() || r.has_null_predicate(),
        }
    }
}

/// A qual in the shape the Postgres planner keeps it: `NOT` only on leaves, and no `AND` directly
/// under an `AND` or `OR` directly under an `OR`.
#[derive(Clone, Debug, PartialEq)]
enum Qual {
    Leaf {
        sql: String,
        negated: bool,
        is_search: bool,
    },
    And(Vec<Qual>),
    Or(Vec<Qual>),
}

impl Qual {
    fn and(args: Vec<Qual>) -> Qual {
        let mut args = args
            .into_iter()
            .flat_map(Qual::into_conjuncts)
            .collect::<Vec<_>>();
        if args.len() == 1 {
            args.remove(0)
        } else {
            Qual::And(args)
        }
    }

    fn or(args: Vec<Qual>) -> Qual {
        let mut args = args
            .into_iter()
            .flat_map(Qual::into_disjuncts)
            .collect::<Vec<_>>();
        if args.len() == 1 {
            args.remove(0)
        } else {
            Qual::Or(args)
        }
    }

    fn into_conjuncts(self) -> Vec<Qual> {
        match self {
            Qual::And(args) => args,
            other => vec![other],
        }
    }

    fn into_disjuncts(self) -> Vec<Qual> {
        match self {
            Qual::Or(args) => args,
            other => vec![other],
        }
    }

    /// The clauses of an `AND`, or the qual itself as a one-clause `AND`.
    fn conjuncts(&self) -> &[Qual] {
        match self {
            Qual::And(args) => args,
            other => std::slice::from_ref(other),
        }
    }

    /// Follows `find_duplicate_ors` in the planner's `prepqual.c`.
    fn canonicalize(self) -> Qual {
        match self {
            Qual::And(args) => Qual::and(args.into_iter().map(Qual::canonicalize).collect()),
            Qual::Or(arms) => Qual::factor_or(
                arms.into_iter()
                    .map(Qual::canonicalize)
                    .flat_map(Qual::into_disjuncts)
                    .collect(),
            ),
            leaf => leaf,
        }
    }

    /// Follows `process_duplicate_ors` in `prepqual.c`: `(A AND B) OR (A AND C)` becomes
    /// `A AND (B OR C)`, and `A OR (A AND B)` becomes `A`, which drops `B`.
    fn factor_or(arms: Vec<Qual>) -> Qual {
        if arms.len() == 1 {
            return arms.into_iter().next().unwrap();
        }

        // The planner compares clauses in the order of the shortest arm, and the order decides
        // whether this result is equal to a clause of an outer `OR`.
        let reference = arms
            .iter()
            .map(Qual::conjuncts)
            .min_by_key(|conjuncts| conjuncts.len())
            .unwrap();
        let mut winners: Vec<Qual> = Vec::new();
        for clause in reference {
            if !winners.contains(clause) && arms.iter().all(|arm| arm.conjuncts().contains(clause))
            {
                winners.push(clause.clone());
            }
        }
        if winners.is_empty() {
            return Qual::Or(arms);
        }

        let mut rest_of_arms = Vec::new();
        for arm in &arms {
            let rest = arm
                .conjuncts()
                .iter()
                .filter(|clause| !winners.contains(clause))
                .cloned()
                .collect::<Vec<_>>();
            if rest.is_empty() {
                // This arm is true whenever the winners are, so the other arms add nothing.
                return Qual::and(winners);
            }
            rest_of_arms.push(Qual::and(rest));
        }
        winners.push(Qual::or(rest_of_arms));
        Qual::and(winners)
    }

    fn has_search_operator(&self) -> bool {
        match self {
            Qual::Leaf { is_search, .. } => *is_search,
            Qual::And(args) | Qual::Or(args) => args.iter().any(Qual::has_search_operator),
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
                c.sql_type.to_owned(),
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
            move |(table, (col, val, sql_type, is_indexed, is_primary_key), kind)| {
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
                        sql_type,
                        is_indexed,
                    },
                }
            },
        );

    // inner nodes, wrapped so that at least one search operator survives the planner. Without
    // one, a custom scan declines the query, and the plan checks would fail.
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
        if !expr.planner_keeps_search_operator() {
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

    #[test]
    fn test_has_null_predicate() {
        let atom = Expr::Atom {
            name: "users.name".to_string(),
            value: "'alice'".to_string(),
            sql_type: "TEXT".to_string(),
            is_indexed: false,
        };
        let all = Expr::All {
            table: "users".to_string(),
            key_col: "id".to_string(),
        };
        assert!(!atom.has_null_predicate());
        assert!(!all.has_null_predicate());

        let is_null = Expr::IsNull("products.color".to_string());
        let is_not_null = Expr::IsNotNull("orders.id".to_string());
        assert!(is_null.has_null_predicate());
        assert!(is_not_null.has_null_predicate());

        let not_is_not_null = Expr::Not(Box::new(is_not_null.clone()));
        assert!(not_is_not_null.has_null_predicate());

        let and_expr = Expr::And(Box::new(atom), Box::new(not_is_not_null));
        assert!(and_expr.has_null_predicate());
    }

    fn atom(name: &str, value: &str, sql_type: &str) -> Expr {
        Expr::Atom {
            name: name.to_string(),
            value: value.to_string(),
            sql_type: sql_type.to_string(),
            is_indexed: true,
        }
    }

    fn and(l: Expr, r: Expr) -> Expr {
        Expr::And(Box::new(l), Box::new(r))
    }

    fn or(l: Expr, r: Expr) -> Expr {
        Expr::Or(Box::new(l), Box::new(r))
    }

    fn not(e: Expr) -> Expr {
        Expr::Not(Box::new(e))
    }

    #[test]
    fn test_planner_drops_an_absorbed_search_operator() {
        let uuid = atom(
            "users.uuid",
            "'550e8400-e29b-41d4-a716-446655440000'",
            "UUID",
        );
        let id = atom("users.id", "'4'", "SERIAL8");
        let name = atom("users.name", "'bob'", "TEXT");

        // A OR (A AND B)
        let absorbed = or(
            uuid.clone(),
            and(uuid.clone(), or(id.clone(), name.clone())),
        );
        assert!(absorbed.has_search_operator());
        assert!(!absorbed.planner_keeps_search_operator());

        // (C AND B AND A) OR (A OR A): the nested OR is flattened into its parent first.
        let flattened = or(
            and(id.clone(), and(name.clone(), uuid.clone())),
            or(uuid.clone(), uuid.clone()),
        );
        assert!(!flattened.planner_keeps_search_operator());

        // NOT (NOT A AND NOT (A AND B)) is A OR (A AND B) once the planner pushes the NOTs down.
        let negated = not(and(not(uuid.clone()), not(and(uuid.clone(), name.clone()))));
        assert!(!negated.planner_keeps_search_operator());

        // NOT (x IS NOT NULL) is the same clause as x IS NULL.
        let color_is_null = Expr::IsNull("users.color".to_string());
        let color_is_not_null = Expr::IsNotNull("users.color".to_string());
        let null_tests = or(color_is_null, and(not(color_is_not_null), name));
        assert!(!null_tests.planner_keeps_search_operator());
    }

    #[test]
    fn test_planner_keeps_a_factored_search_operator() {
        let uuid = atom(
            "users.uuid",
            "'550e8400-e29b-41d4-a716-446655440000'",
            "UUID",
        );
        let id = atom("users.id", "'4'", "SERIAL8");
        let name = atom("users.name", "'bob'", "TEXT");

        // (A AND B) OR (A AND C) becomes A AND (B OR C).
        let factored = or(
            and(uuid.clone(), name.clone()),
            and(uuid.clone(), id.clone()),
        );
        assert!(factored.planner_keeps_search_operator());

        // A OR (B AND A) keeps nothing but A, which is itself a search operator here.
        let absorbed_into_search = or(name.clone(), and(uuid.clone(), name.clone()));
        assert!(absorbed_into_search.planner_keeps_search_operator());

        // x IS NULL and x IS NOT NULL are different clauses, so nothing is factored out.
        let opposite_null_tests = or(
            Expr::IsNull("users.color".to_string()),
            and(Expr::IsNotNull("users.color".to_string()), name.clone()),
        );
        assert!(opposite_null_tests.planner_keeps_search_operator());

        let no_shared_clause = or(uuid, name);
        assert!(no_shared_clause.planner_keeps_search_operator());
    }

    proptest! {
        #[test]
        fn test_arb_wheres_keeps_a_search_operator_through_the_planner(
            expr in arb_wheres(vec!["users", "products"], &[
                Column::new("name", "TEXT", "'bob'").whereable(true),
                Column::new("color", "VARCHAR", "'blue'").whereable(true),
                Column::new("uuid", "UUID", "'550e8400-e29b-41d4-a716-446655440000'")
                    .whereable(true),
            ])
        ) {
            prop_assert!(expr.planner_keeps_search_operator(), "{}", expr.to_sql("@@@"));
        }

        #[test]
        fn test_arb_wheres_generates_null_checks(
            expr in arb_wheres(vec!["users", "products"], &[
                Column::new("color", "VARCHAR", "'blue'").whereable(true),
                Column::new("quantity", "INTEGER", "4").whereable(true),
                Column::new("active", "BOOLEAN", "true").whereable(true),
            ])
        ) {
            let sql_pg = expr.to_sql(" = ");
            let sql_bm25 = expr.to_sql("@@@");
            assert!(!sql_pg.is_empty());
            assert!(!sql_bm25.is_empty());
            assert!(expr.has_search_operator());
            assert!(sql_bm25.contains("@@@") || sql_bm25.contains("==="));
            assert!(!sql_bm25.contains("pdb.term"));
        }
    }
}
