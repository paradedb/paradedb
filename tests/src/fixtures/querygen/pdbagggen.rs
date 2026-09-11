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

//! Generates `pdb.agg()` specs together with the SQL `GROUP BY` that computes the
//! same buckets. `pdb.agg()` has no Postgres fallback, so over a join the SQL
//! query is the oracle: its rows, and the JSON buckets flattened the same way,
//! must be equal. On a single table the other backend is the oracle instead, and
//! the documents themselves are compared. `missing`, `min_doc_count`, and `order`
//! by a metric have no SQL translation here and are left to the regression tests.

use proptest::prelude::*;
use serde_json::{Value, json};
use sqlx::Row;
use sqlx::postgres::PgRow;

use crate::fixtures::querygen::Column;
use crate::fixtures::querygen::joingen::{JoinExpr, JoinType, arb_joins};
use crate::fixtures::querygen::wheregen::{Expr, arb_wheres};

/// Size that no generated bucket count reaches, so a level is never cut.
const NO_CUT: u32 = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricKind {
    Sum,
    Avg,
    Min,
    Max,
    ValueCount,
    Cardinality,
}

impl MetricKind {
    fn spec_name(self) -> &'static str {
        match self {
            MetricKind::Sum => "sum",
            MetricKind::Avg => "avg",
            MetricKind::Min => "min",
            MetricKind::Max => "max",
            MetricKind::ValueCount => "value_count",
            MetricKind::Cardinality => "cardinality",
        }
    }

    /// The SQL aggregate with Tantivy's empty-input value and a float result, so
    /// both sides format the same way.
    fn sql(self, field: &str) -> String {
        match self {
            MetricKind::Sum => format!("COALESCE(SUM({field}), 0)::float8"),
            MetricKind::Avg => format!("AVG({field})::float8"),
            MetricKind::Min => format!("MIN({field})::float8"),
            MetricKind::Max => format!("MAX({field})::float8"),
            MetricKind::ValueCount => format!("COUNT({field})::float8"),
            MetricKind::Cardinality => format!("COUNT(DISTINCT {field})::float8"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Metric {
    pub name: String,
    pub kind: MetricKind,
    pub field: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdbTerm {
    pub field: String,
    pub is_array: bool,
}

impl PdbTerm {
    pub fn sql_key(&self) -> String {
        if self.is_array {
            format!("_{}", self.field.replace('.', "_"))
        } else {
            self.field.clone()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OuterAggKind {
    CountStar,
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OuterAgg {
    pub kind: OuterAggKind,
    pub field: Option<String>,
}

impl OuterAgg {
    pub fn sql(&self) -> String {
        match self.kind {
            OuterAggKind::CountStar => "COUNT(*)".to_string(),
            OuterAggKind::Count => format!("COUNT({})", self.field.as_ref().unwrap()),
            OuterAggKind::Sum => format!("SUM({})", self.field.as_ref().unwrap()),
            OuterAggKind::Avg => format!("AVG({})", self.field.as_ref().unwrap()),
            OuterAggKind::Min => format!("MIN({})", self.field.as_ref().unwrap()),
            OuterAggKind::Max => format!("MAX({})", self.field.as_ref().unwrap()),
        }
    }
}

/// One `pdb.agg()` call and the shape of its result.
#[derive(Clone, Debug)]
pub struct PdbAggExpr {
    /// A SQL `GROUP BY` column beside the call, which becomes the root grouping set.
    pub outer_group: Option<String>,
    /// SQL aggregates beside the call.
    pub outer_aggs: Vec<OuterAgg>,
    /// `terms` levels, outermost first. Empty for a spec that is one metric.
    pub terms: Vec<PdbTerm>,
    /// A `size` cut on one level, under the default `_count desc` order.
    pub size: Option<(usize, u32)>,
    /// Metrics under the innermost level; the whole spec when there are no levels.
    pub metrics: Vec<Metric>,
}

impl PdbAggExpr {
    fn metric_aggs(&self) -> serde_json::Map<String, Value> {
        self.metrics
            .iter()
            .map(|m| {
                (
                    m.name.clone(),
                    json!({ m.kind.spec_name(): { "field": m.field } }),
                )
            })
            .collect()
    }

    fn spec(&self) -> Value {
        if self.terms.is_empty() {
            let metric = &self.metrics[0];
            return json!({ metric.kind.spec_name(): { "field": metric.field } });
        }
        let mut node = Value::Null;
        for (level, term) in self.terms.iter().enumerate().rev() {
            let terms = match self.size {
                // Every segment keeps every term, so the cut is exact and its
                // error bound zero, the same as a backend that has every group.
                Some((cut_level, size)) if level == cut_level => {
                    json!({ "field": &term.field, "size": size, "segment_size": NO_CUT })
                }
                _ => json!({ "field": &term.field, "size": NO_CUT, "order": { "_key": "asc" } }),
            };
            let aggs = if node.is_null() {
                Value::Object(self.metric_aggs())
            } else {
                json!({ Self::level_name(level + 1): node })
            };
            node = json!({ "terms": terms, "aggs": aggs });
        }
        node
    }

    fn level_name(level: usize) -> String {
        format!("level{level}")
    }

    /// The `pdb.agg()` call with the spec as its literal.
    pub fn call(&self) -> String {
        format!("pdb.agg('{}')", self.spec().to_string().replace('\'', "''"))
    }

    pub fn pdb_query(&self, from_clause: &str, where_clause: &str) -> String {
        let call = self.call();
        let mut select = Vec::new();
        if let Some(group) = &self.outer_group {
            select.push(group.clone());
        }
        for agg in &self.outer_aggs {
            select.push(agg.sql());
        }
        select.push(call);
        let select_clause = select.join(", ");
        match &self.outer_group {
            Some(group) => {
                format!(
                    "SELECT {select_clause} {from_clause} WHERE {where_clause} GROUP BY {group}"
                )
            }
            None => format!("SELECT {select_clause} {from_clause} WHERE {where_clause}"),
        }
    }

    /// The outer group and outer aggregates without unnesting.
    pub fn pg_outer_query(&self, from_clause: &str, where_clause: &str) -> String {
        let mut select = Vec::new();
        if let Some(group) = &self.outer_group {
            select.push(group.clone());
        }
        for agg in &self.outer_aggs {
            select.push(agg.sql());
        }
        let select_clause = select.join(", ");
        match &self.outer_group {
            Some(group) => {
                format!(
                    "SELECT {select_clause} {from_clause} WHERE {where_clause} GROUP BY {group}"
                )
            }
            None => format!("SELECT {select_clause} {from_clause} WHERE {where_clause}"),
        }
    }

    /// The `GROUP BY` that yields one row per innermost bucket, in the column order
    /// [`Self::rows`] flattens the JSON into. Only a cut on a lone top level has a
    /// SQL form.
    pub fn pg_query(&self, from_clause: &str, where_clause: &str) -> String {
        let mut from = from_clause.to_string();
        let mut keys: Vec<String> = self.outer_group.iter().cloned().collect();
        for term in &self.terms {
            if term.is_array {
                let alias = term.sql_key();
                // Tantivy and DataFusion (via PreserveAndExpandEmpty) preserve documents
                // with empty or NULL arrays, assigning them to the NULL/missing bucket
                // rather than discarding them from the query or outer groupings.
                // Using `LEFT JOIN LATERAL ... ON true` mimics this: an inner unnest
                // (e.g. `CROSS JOIN LATERAL`) would drop rows with empty/NULL arrays.
                from.push_str(&format!(
                    " LEFT JOIN LATERAL unnest({}) AS {alias} ON true",
                    term.field
                ));
                keys.push(alias);
            } else {
                keys.push(term.field.clone());
            }
        }
        let mut select: Vec<String> = keys.clone();
        if !self.terms.is_empty() {
            select.push("COUNT(*)".to_string());
        }
        select.extend(self.metrics.iter().map(|m| m.kind.sql(&m.field)));
        let mut sql = format!("SELECT {} {from} WHERE {where_clause}", select.join(", "));
        if !keys.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", keys.join(", ")));
        }
        if let Some((cut_level, size)) = self.size {
            assert_eq!(cut_level, 0, "SQL can only mirror a cut on the top level");
            // Tantivy breaks count ties on the key and puts the NULL bucket last.
            let order_key = self.terms[0].sql_key();
            sql.push_str(&format!(
                " ORDER BY COUNT(*) DESC, {order_key} ASC NULLS LAST LIMIT {size}",
            ));
        }
        sql
    }

    /// The outer group and outer aggregates from either `pg_outer_query` or `pdb_query`.
    /// When `is_pdb` is true, the trailing `pdb.agg()` JSON column is omitted.
    pub fn outer_rows(&self, rows: Vec<PgRow>, is_pdb: bool) -> Result<Vec<String>, sqlx::Error> {
        let mut out = Vec::new();
        for row in rows {
            let num_cols = if is_pdb { row.len() - 1 } else { row.len() };
            let cells: Vec<String> = (0..num_cols).map(|i| column_string(&row, i)).collect();
            out.push(cells.join("|"));
        }
        Ok(out)
    }

    /// The rows of either query as strings: the SQL result column by column, or
    /// the JSON documents flattened to the same columns.
    pub fn rows(&self, rows: Vec<PgRow>) -> Result<Vec<String>, sqlx::Error> {
        let mut out = Vec::new();
        for row in rows {
            let last = row.len() - 1;
            match row.try_get::<Value, _>(last) {
                Ok(document) => {
                    let outer = self.outer_group.is_some().then(|| column_string(&row, 0));
                    self.flatten(&document, 0, &mut outer.into_iter().collect(), &mut out);
                }
                Err(_) => {
                    let cells: Vec<String> =
                        (0..row.len()).map(|i| column_string(&row, i)).collect();
                    out.push(cells.join("|"));
                }
            }
        }
        Ok(out)
    }

    /// The SQL group value and the `pdb.agg()` document of each row of
    /// [`Self::pdb_query`], as they are.
    pub fn documents(&self, rows: Vec<PgRow>) -> Result<Vec<(String, Value)>, sqlx::Error> {
        rows.iter()
            .map(|row| {
                let group = if self.outer_group.is_some() {
                    column_string(row, 0)
                } else {
                    String::new()
                };
                Ok((group, row.try_get::<Value, _>(row.len() - 1)?))
            })
            .collect()
    }

    fn flatten(&self, node: &Value, level: usize, prefix: &mut Vec<String>, out: &mut Vec<String>) {
        if level == self.terms.len() {
            let mut cells = prefix.clone();
            if !self.terms.is_empty() {
                cells.push(json_string(&node["doc_count"]));
            }
            for metric in &self.metrics {
                let value = if self.terms.is_empty() {
                    &node["value"]
                } else {
                    &node[&metric.name]["value"]
                };
                cells.push(json_string(value));
            }
            out.push(cells.join("|"));
            return;
        }
        let Some(buckets) = node["buckets"].as_array() else {
            return;
        };
        for bucket in buckets {
            prefix.push(json_string(&bucket["key"]));
            let child = if level + 1 == self.terms.len() {
                bucket
            } else {
                &bucket[&Self::level_name(level + 1)]
            };
            self.flatten(child, level + 1, prefix, out);
            prefix.pop();
        }
    }
}

/// A SQL cell the way [`json_string`] renders the JSON value it corresponds to.
fn column_string(row: &PgRow, i: usize) -> String {
    if let Ok(v) = row.try_get::<i64, _>(i) {
        v.to_string()
    } else if let Ok(v) = row.try_get::<i32, _>(i) {
        v.to_string()
    } else if let Ok(v) = row.try_get::<f64, _>(i) {
        format!("{v:.6}")
    } else if let Ok(v) = row.try_get::<String, _>(i) {
        v
    } else {
        "NULL".to_string()
    }
}

fn json_string(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::String(s) => s.clone(),
        Value::Number(n) => match n.as_i64() {
            Some(v) => v.to_string(),
            None => format!("{:.6}", n.as_f64().unwrap_or(f64::NAN)),
        },
        other => other.to_string(),
    }
}

/// What the oracle of a test can express, which bounds the specs it gets.
#[derive(Clone, Copy)]
struct SpecShape {
    /// A SQL `GROUP BY` always sits beside the call.
    grouped: bool,
    /// Standard SQL aggregates may sit beside the call.
    allow_outer_aggs: bool,
    /// NUMERIC columns may be metric fields.
    numeric_metrics: bool,
    /// A `size` may cut any level, not only a lone top level under no group.
    size_anywhere: bool,
    /// Bucket keys a spec with a `cardinality` metric may have. Every bucket
    /// carries its own sketch, and the DataFusion aggregate holds them all in
    /// `work_mem` with no spill.
    sketch_keys: usize,
    /// Array columns may be used as terms fields.
    allow_arrays: bool,
}

#[derive(Clone, Debug)]
pub struct PdbAggJoinWheres {
    pub outer: Expr,
    pub inner: Expr,
}

impl PdbAggJoinWheres {
    pub fn pg_where(&self) -> String {
        format!(
            "({}) AND ({})",
            self.outer.to_sql(" = "),
            self.inner.to_sql(" = ")
        )
    }

    pub fn bm25_where(&self) -> String {
        format!(
            "({}) AND ({})",
            self.outer.to_sql("@@@"),
            self.inner.to_sql(" = ")
        )
    }
}

/// A join over a prefix of `tables` and a `pdb.agg()` whose fields belong to
/// those tables. `key_columns` are the join keys; `terms` keys come from the
/// text and integer columns, metrics from the integer and NUMERIC ones. A join
/// with a keyless step yields tens of thousands of buckets under several keys,
/// so `cardinality` keeps to one key there.
///
/// Generates `Inner` and `Left` joins. To prevent the PostgreSQL planner from
/// eliminating outer-joined tables that aren't referenced elsewhere, a WHERE
/// expression is generated that includes a predicate for each joined table.
pub fn arb_pdb_agg_join(
    tables: Vec<String>,
    key_columns: &[Column],
    where_columns: &[Column],
) -> impl Strategy<Value = (JoinExpr, PdbAggExpr, PdbAggJoinWheres)> {
    let key_columns = key_columns.to_vec();
    let where_columns = where_columns.to_vec();
    (2..=tables.len()).prop_flat_map(move |num_tables| {
        let joined: Vec<String> = tables[..num_tables].to_vec();
        // The planner cannot see the fields inside a spec, so it removes an outer
        // join whose table nothing else reads, and the spec then names a table
        // that is gone. We generate predicates for every joined table so outer joins
        // are retained.
        let join_types = prop_oneof![Just(JoinType::Inner), Just(JoinType::Left)];
        let where_cols = where_columns.clone();
        arb_joins(join_types, joined.clone(), &key_columns).prop_flat_map(move |join| {
            let shape = SpecShape {
                grouped: false,
                allow_outer_aggs: true,
                numeric_metrics: true,
                size_anywhere: false,
                sketch_keys: if join.has_keyless_step() {
                    1
                } else {
                    usize::MAX
                },
                allow_arrays: true,
            };
            let agg = arb_pdb_agg(joined.clone(), shape);
            let outer_strat = arb_wheres(vec![joined[0].clone()], &where_cols).boxed();
            let inner_strat = joined[1..]
                .iter()
                .map(|t| arb_wheres(vec![t.clone()], &where_cols).boxed())
                .reduce(|acc, s| {
                    (acc, s)
                        .prop_map(|(l, r)| Expr::And(Box::new(l), Box::new(r)))
                        .boxed()
                })
                .unwrap();
            let where_strat = (outer_strat, inner_strat)
                .prop_map(|(outer, inner)| PdbAggJoinWheres { outer, inner });
            (Just(join), agg, where_strat)
        })
    })
}

/// A `pdb.agg()` over one table, with bare field names, beside a SQL `GROUP BY`.
/// The group is what lets a query be routed to either backend, and with the
/// backends as each other's oracle a `size` may cut any level. NUMERIC fields
/// stay out, since only one backend reads them.
pub fn arb_pdb_agg_single_table() -> impl Strategy<Value = PdbAggExpr> {
    arb_pdb_agg(
        Vec::new(),
        SpecShape {
            grouped: true,
            allow_outer_aggs: false,
            numeric_metrics: false,
            size_anywhere: true,
            sketch_keys: usize::MAX,
            allow_arrays: false,
        },
    )
}

/// Fields are qualified by each of `tables`, or bare when there are none.
fn arb_pdb_agg(tables: Vec<String>, shape: SpecShape) -> impl Strategy<Value = PdbAggExpr> {
    let qualify = |columns: &[&str]| -> Vec<String> {
        if tables.is_empty() {
            return columns.iter().map(|c| c.to_string()).collect();
        }
        tables
            .iter()
            .flat_map(|t| columns.iter().map(move |c| format!("{t}.{c}")))
            .collect()
    };
    // `color` and `quantity` carry NULLs, which become a bucket of their own.
    let key_fields = qualify(&["color", "age", "quantity"]);
    let array_fields = qualify(&["tags"]);
    let int_fields = qualify(&["age", "quantity"]);
    let metric_fields = if shape.numeric_metrics {
        qualify(&["age", "quantity", "price"])
    } else {
        int_fields.clone()
    };
    let default_metric_field = int_fields[0].clone();
    let kinds = [
        MetricKind::Sum,
        MetricKind::Min,
        MetricKind::Max,
        MetricKind::ValueCount,
        MetricKind::Cardinality,
    ];
    // An average divides in floating point on both sides only for an integer
    // field; Postgres averages NUMERIC exactly, where the join path rounds twice.
    let metric = prop_oneof![
        3 => (
            proptest::sample::select(kinds.to_vec()),
            proptest::sample::select(metric_fields),
        ),
        1 => (Just(MetricKind::Avg), proptest::sample::select(int_fields.clone())),
    ];
    let outer_group = if shape.grouped {
        proptest::sample::select(key_fields.clone())
            .prop_map(Some)
            .boxed()
    } else {
        proptest::option::weighted(0.3, proptest::sample::select(key_fields.clone())).boxed()
    };

    let outer_aggs_strat = if shape.allow_outer_aggs {
        let outer_agg_fields = int_fields.clone();
        let outer_agg = prop_oneof![
            Just(OuterAgg {
                kind: OuterAggKind::CountStar,
                field: None,
            }),
            proptest::sample::select(outer_agg_fields.clone()).prop_map(|f| OuterAgg {
                kind: OuterAggKind::Sum,
                field: Some(f),
            }),
            proptest::sample::select(outer_agg_fields.clone()).prop_map(|f| OuterAgg {
                kind: OuterAggKind::Avg,
                field: Some(f),
            }),
            proptest::sample::select(outer_agg_fields.clone()).prop_map(|f| OuterAgg {
                kind: OuterAggKind::Min,
                field: Some(f),
            }),
            proptest::sample::select(outer_agg_fields).prop_map(|f| OuterAgg {
                kind: OuterAggKind::Max,
                field: Some(f),
            }),
            proptest::sample::select(key_fields.clone()).prop_map(|f| OuterAgg {
                kind: OuterAggKind::Count,
                field: Some(f),
            }),
        ];
        proptest::collection::vec(outer_agg, 0..=2).boxed()
    } else {
        Just(vec![]).boxed()
    };

    let mut all_terms: Vec<PdbTerm> = key_fields
        .iter()
        .map(|f| PdbTerm {
            field: f.clone(),
            is_array: false,
        })
        .collect();
    if shape.allow_arrays {
        all_terms.extend(array_fields.iter().map(|f| PdbTerm {
            field: f.clone(),
            is_array: true,
        }));
    }

    (
        outer_group,
        outer_aggs_strat,
        proptest::sample::subsequence(all_terms, 0..=2),
        proptest::option::weighted(0.4, (0..2usize, 1..4u32)),
        proptest::collection::vec(metric, 0..=3),
    )
        .prop_map(move |(outer_group, outer_aggs, terms, size, metrics)| {
            let mut metrics: Vec<Metric> = metrics
                .into_iter()
                .enumerate()
                .map(|(i, (kind, field))| Metric {
                    name: format!("m{i}"),
                    kind,
                    field,
                })
                .collect();
            if terms.is_empty() {
                // A spec without levels is exactly one metric.
                metrics.truncate(1);
                if metrics.is_empty() {
                    metrics.push(Metric {
                        name: "m0".to_string(),
                        kind: MetricKind::ValueCount,
                        field: default_metric_field.clone(),
                    });
                }
            }
            if usize::from(outer_group.is_some()) + terms.len() > shape.sketch_keys {
                for metric in &mut metrics {
                    if metric.kind == MetricKind::Cardinality {
                        metric.kind = MetricKind::ValueCount;
                    }
                }
            }
            let size = size.filter(|&(level, _)| {
                level < terms.len()
                    && (shape.size_anywhere
                        || (level == 0 && terms.len() == 1 && outer_group.is_none()))
            });
            PdbAggExpr {
                outer_group,
                outer_aggs,
                terms,
                size,
                metrics,
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::strategy::{Strategy, ValueTree};
    use proptest::test_runner::TestRunner;

    #[test]
    fn cardinality_keeps_to_one_key_over_a_keyless_join() {
        let mut runner = TestRunner::default();
        let tables = vec![
            "users".to_string(),
            "products".to_string(),
            "orders".to_string(),
        ];
        let key_columns = vec![
            Column::new("id", "BIGINT", "1"),
            Column::new("age", "INTEGER", "20"),
        ];
        let where_columns = vec![
            Column::new("name", "TEXT", "'bob'").whereable(true),
            Column::new("color", "VARCHAR", "'blue'").whereable(true),
        ];
        let strategy = arb_pdb_agg_join(tables, &key_columns, &where_columns);

        let mut saw_sketch_over_keyless = false;
        let mut saw_sketch_under_keys = false;
        for _ in 0..500 {
            let (join, agg, _wheres) = strategy.new_tree(&mut runner).unwrap().current();
            let keys = usize::from(agg.outer_group.is_some()) + agg.terms.len();
            let sketch = agg
                .metrics
                .iter()
                .any(|m| m.kind == MetricKind::Cardinality);
            if join.has_keyless_step() {
                assert!(!(sketch && keys > 1), "{join:?} {agg:?}");
                saw_sketch_over_keyless |= sketch;
            } else {
                saw_sketch_under_keys |= sketch && keys > 1;
            }
        }
        assert!(saw_sketch_over_keyless);
        assert!(saw_sketch_under_keys);
    }

    #[test]
    fn test_arb_pdb_agg_join_generates_left_joins_and_multi_table_wheres() {
        let mut runner = TestRunner::default();
        let tables = vec![
            "users".to_string(),
            "products".to_string(),
            "orders".to_string(),
        ];
        let key_columns = vec![
            Column::new("id", "INTEGER", "'1'").whereable(true),
            Column::new("age", "INTEGER", "'20'").whereable(true),
        ];
        let where_columns = vec![
            Column::new("name", "TEXT", "'bob'").whereable(true),
            Column::new("color", "VARCHAR", "'blue'").whereable(true),
        ];

        let strategy = arb_pdb_agg_join(tables, &key_columns, &where_columns);
        let mut generated_left = false;
        let mut generated_is_null = false;

        for _ in 0..100 {
            let (join, _agg, wheres) = strategy.new_tree(&mut runner).unwrap().current();
            let join_sql = join.to_sql();
            if join_sql.contains("LEFT JOIN") {
                generated_left = true;
            }
            let where_sql = wheres.pg_where();
            if where_sql.contains("IS NULL") || where_sql.contains("IS NOT NULL") {
                generated_is_null = true;
            }
            // Verify every joined table is referenced in WHERE
            for table in join.used_tables() {
                assert!(
                    where_sql.contains(table),
                    "Table {table} missing from WHERE: {where_sql}"
                );
            }
        }

        assert!(
            generated_left,
            "Expected at least one LEFT JOIN to be generated"
        );
        assert!(
            generated_is_null,
            "Expected at least one IS NULL/IS NOT NULL predicate to be generated"
        );
    }

    #[test]
    fn test_pg_query_unnests_array_terms() {
        let agg = PdbAggExpr {
            outer_group: Some("users.color".to_string()),
            outer_aggs: vec![],
            terms: vec![
                PdbTerm {
                    field: "users.tags".to_string(),
                    is_array: true,
                },
                PdbTerm {
                    field: "users.age".to_string(),
                    is_array: false,
                },
            ],
            size: Some((0, 10)),
            metrics: vec![Metric {
                name: "m0".to_string(),
                kind: MetricKind::Sum,
                field: "users.quantity".to_string(),
            }],
        };

        let pg_sql = agg.pg_query("FROM users JOIN products ON users.id = products.id", "TRUE");
        assert!(
            pg_sql.contains("LEFT JOIN LATERAL unnest(users.tags) AS _users_tags ON true"),
            "Expected LEFT JOIN LATERAL unnest for array term, got: {pg_sql}"
        );
        assert!(
            pg_sql.contains("GROUP BY users.color, _users_tags, users.age"),
            "Expected GROUP BY with unnested alias, got: {pg_sql}"
        );
        assert!(
            pg_sql.contains("ORDER BY COUNT(*) DESC, _users_tags ASC NULLS LAST LIMIT 10"),
            "Expected ORDER BY with unnested alias for size cut, got: {pg_sql}"
        );
    }
}
