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

use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};

use proptest::prelude::*;
use proptest::sample;
use proptest_derive::Arbitrary;

use super::Column;

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
pub struct LateralUnnestStep {
    pub is_left: bool,
    pub table: String,
    pub array_col: String,
    pub alias: String,
}

pub const NON_EQUI_OPS: &[&str] = &["<", "<=", ">", ">=", "<>"];

#[derive(Clone, Copy, Debug)]
pub enum ConditionKind {
    Equi,
    NonEqui,
    Mixed,
}

#[derive(Clone, Debug)]
pub enum OnCondition {
    Equi {
        col: String,
    },
    NonEqui {
        col: String,
        op: &'static str,
    },
    Mixed {
        equi_col: String,
        non_equi_col: String,
        op: &'static str,
    },
}

#[derive(Clone, Debug)]
struct JoinStep {
    join_type: JoinType,
    table: String,
    on_left_table: Option<String>,
    on_condition: Option<OnCondition>,
}

#[derive(Clone)]
pub struct JoinExpr {
    initial_table: String,
    steps: Vec<JoinStep>,
    unnests: Vec<LateralUnnestStep>,
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

    pub fn unnest_aliases(&self) -> Vec<&str> {
        self.unnests.iter().map(|u| u.alias.as_str()).collect()
    }

    pub fn has_only_inner(&self) -> bool {
        self.steps
            .iter()
            .all(|s| matches!(s.join_type, JoinType::Inner))
            && self.unnests.iter().all(|u| !u.is_left)
    }

    pub fn has_no_cross(&self) -> bool {
        self.steps
            .iter()
            .all(|s| !matches!(s.join_type, JoinType::Cross))
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
                match step.on_condition.as_ref().unwrap() {
                    OnCondition::Equi { col } => {
                        join_clause.push_str(&format!(" ON {lt}.{col} = {}.{col}", step.table));
                    }
                    OnCondition::NonEqui { col, op } => {
                        join_clause.push_str(&format!(" ON {lt}.{col} {op} {}.{col}", step.table));
                    }
                    OnCondition::Mixed {
                        equi_col,
                        non_equi_col,
                        op,
                    } => {
                        join_clause.push_str(&format!(
                            " ON {lt}.{equi_col} = {}.{equi_col} AND {lt}.{non_equi_col} {op} {}.{non_equi_col}",
                            step.table, step.table
                        ));
                    }
                }
            }
        }

        for unnest in &self.unnests {
            join_clause.push(' ');
            if unnest.is_left {
                join_clause.push_str(&format!(
                    "LEFT JOIN LATERAL unnest({}.{}) AS {} ON true",
                    unnest.table, unnest.array_col, unnest.alias
                ));
            } else {
                join_clause.push_str(&format!(
                    "CROSS JOIN LATERAL unnest({}.{}) AS {}",
                    unnest.table, unnest.array_col, unnest.alias
                ));
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

#[derive(Clone, Debug)]
pub struct SemiJoinExpr {
    outer_table: String,
    inner_table: String,
    join_column: String,
}

impl SemiJoinExpr {
    pub fn outer_table(&self) -> &str {
        &self.outer_table
    }

    pub fn inner_table(&self) -> &str {
        &self.inner_table
    }

    pub fn join_column(&self) -> &str {
        &self.join_column
    }
}

///
/// Generate an arbitrary join expression chaining the given tables in sequence.
///
/// Consecutively joins each table using join types drawn from `join_types` and
/// join conditions (equi, non-equi, or mixed) drawn from non-array columns in `columns`.
/// For each table in the join, an optional lateral unnest step (`CROSS JOIN LATERAL` or
/// `LEFT JOIN LATERAL`) may be generated if any array columns are present in `columns`.
///
pub fn arb_joins<J, S>(
    join_types: J,
    tables_to_join: Vec<S>,
    columns: &[Column],
) -> impl Strategy<Value = JoinExpr> + use<J, S>
where
    J: Strategy<Value = JoinType>,
    S: AsRef<str>,
{
    let tables_to_join = tables_to_join
        .into_iter()
        .map(|tn| tn.as_ref().to_string())
        .collect::<Vec<_>>();
    let mut table_cols = columns
        .iter()
        .filter(|c| !c.is_array())
        .map(|c| c.name.to_string())
        .collect::<Vec<_>>();
    if table_cols.is_empty() {
        table_cols = columns.iter().map(|c| c.name.to_string()).collect();
    }

    let array_cols = columns
        .iter()
        .filter(|c| c.is_array())
        .map(|c| c.name.to_string())
        .collect::<Vec<_>>();

    let join_count = tables_to_join.len() - 1;
    let tables_len = tables_to_join.len();

    let unnest_strategy = if array_cols.is_empty() {
        proptest::strategy::Just(vec![None; tables_len]).boxed()
    } else {
        proptest::collection::vec(
            proptest::option::of((proptest::bool::ANY, proptest::sample::select(array_cols))),
            tables_len,
        )
        .boxed()
    };

    let orderable_cols: Vec<String> = columns
        .iter()
        .filter(|c| c.is_orderable())
        .map(|c| c.name.to_string())
        .collect();
    let orderable_cols = if orderable_cols.is_empty() {
        table_cols.clone()
    } else {
        orderable_cols
    };

    let step_strategy = (
        join_types,
        prop_oneof![
            2 => Just(ConditionKind::Equi),
            1 => Just(ConditionKind::NonEqui),
            1 => Just(ConditionKind::Mixed),
        ],
        0..table_cols.len(),
        0..orderable_cols.len(),
        0..NON_EQUI_OPS.len(),
    );

    (
        proptest::collection::vec(step_strategy, join_count),
        unnest_strategy,
    )
        .prop_map(move |(step_choices, unnest_choices)| {
            // Construct a JoinExpr for the tables and joins.
            let mut tables_iter = tables_to_join.clone().into_iter();
            let initial_table = tables_iter.next().expect("At least one table in a join.");

            let mut previous_table = initial_table.clone();
            let mut steps = Vec::with_capacity(step_choices.len());
            for ((join_type, kind, equi_col_idx, non_equi_col_idx, op_idx), table_to_join) in
                step_choices.into_iter().zip(tables_iter)
            {
                match join_type {
                    JoinType::Cross => {
                        steps.push(JoinStep {
                            join_type,
                            table: table_to_join.clone(),
                            on_left_table: None,
                            on_condition: None,
                        });
                    }
                    _ => {
                        let condition = match (join_type, kind) {
                            (JoinType::Full, ConditionKind::NonEqui)
                            | (_, ConditionKind::Mixed) => {
                                let equi_col = table_cols[equi_col_idx].clone();
                                let non_equi_col = if orderable_cols.len() > 1
                                    && orderable_cols[non_equi_col_idx] == equi_col
                                {
                                    orderable_cols[(non_equi_col_idx + 1) % orderable_cols.len()]
                                        .clone()
                                } else {
                                    orderable_cols[non_equi_col_idx].clone()
                                };
                                OnCondition::Mixed {
                                    equi_col,
                                    non_equi_col,
                                    op: NON_EQUI_OPS[op_idx],
                                }
                            }
                            (_, ConditionKind::NonEqui) => OnCondition::NonEqui {
                                col: orderable_cols[non_equi_col_idx].clone(),
                                op: NON_EQUI_OPS[op_idx],
                            },
                            _ => OnCondition::Equi {
                                col: table_cols[equi_col_idx].clone(),
                            },
                        };
                        steps.push(JoinStep {
                            join_type,
                            table: table_to_join.clone(),
                            on_left_table: Some(previous_table.clone()),
                            on_condition: Some(condition),
                        });
                    }
                }
                previous_table = table_to_join;
            }

            let mut unnests = Vec::new();
            for (i, table) in tables_to_join.iter().enumerate() {
                if let Some(Some((is_left, array_col))) = unnest_choices.get(i) {
                    unnests.push(LateralUnnestStep {
                        is_left: *is_left,
                        table: table.clone(),
                        array_col: array_col.clone(),
                        alias: format!("{table}_{array_col}"),
                    });
                }
            }

            JoinExpr {
                initial_table,
                steps,
                unnests,
            }
        })
}

///
/// Generate EXISTS-based semi joins using two distinct tables and one join column.
///
pub fn arb_semi_joins(
    tables_to_join: Vec<impl AsRef<str>>,
    columns: Vec<impl AsRef<str>>,
) -> impl Strategy<Value = SemiJoinExpr> {
    let tables_to_join = tables_to_join
        .into_iter()
        .map(|tn| tn.as_ref().to_string())
        .collect::<Vec<_>>();
    let join_columns = columns
        .into_iter()
        .map(|cn| cn.as_ref().to_string())
        .collect::<Vec<_>>();

    (
        sample::subsequence(tables_to_join, 2),
        sample::select(join_columns),
    )
        .prop_map(|(tables, join_column)| SemiJoinExpr {
            outer_table: tables[0].clone(),
            inner_table: tables[1].clone(),
            join_column,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    #[test]
    fn test_arb_joins_generation() {
        let mut runner = TestRunner::default();
        let tables = vec!["t1", "t2", "t3"];
        let columns = vec![
            Column::new("id", "BIGINT", "1"),
            Column::new("age", "INTEGER", "20"),
            Column::new("price", "NUMERIC(10,2)", "9.99"),
            Column::new("name", "TEXT", "alice"),
            Column::new("tags", "TEXT[]", "ARRAY['alpha', 'beta']::text[]"),
        ];

        let strategy = arb_joins(
            prop_oneof![
                Just(JoinType::Inner),
                Just(JoinType::Left),
                Just(JoinType::Right),
                Just(JoinType::Full),
            ],
            tables,
            &columns,
        );

        let mut saw_equi = false;
        let mut saw_non_equi = false;
        let mut saw_mixed = false;
        let mut saw_unnest = false;

        for _ in 0..100 {
            let join_expr = strategy.new_tree(&mut runner).unwrap().current();
            let sql = join_expr.to_sql();
            assert!(sql.starts_with("FROM t1"));

            if !join_expr.unnests.is_empty() {
                saw_unnest = true;
            }

            for step in &join_expr.steps {
                if let Some(cond) = &step.on_condition {
                    match cond {
                        OnCondition::Equi { .. } => saw_equi = true,
                        OnCondition::NonEqui { .. } => {
                            saw_non_equi = true;
                            // FULL JOIN must never generate a keyless non-equi condition.
                            assert!(!matches!(step.join_type, JoinType::Full));
                        }
                        OnCondition::Mixed { .. } => saw_mixed = true,
                    }
                }
            }
        }

        assert!(saw_equi, "Should have generated at least one equi join");
        assert!(
            saw_non_equi,
            "Should have generated at least one non-equi join"
        );
        assert!(saw_mixed, "Should have generated at least one mixed join");
        assert!(saw_unnest, "Should have generated at least one unnest step");
    }
}
