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
use serde_json::{json, Value};
use sqlx::postgres::PgRow;
use sqlx::Row;

use crate::fixtures::querygen::joingen::{arb_joins, JoinExpr, JoinType};
use crate::fixtures::querygen::Column;

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

/// One `pdb.agg()` call and the shape of its result.
#[derive(Clone, Debug)]
pub struct PdbAggExpr {
    /// A SQL `GROUP BY` column beside the call, which becomes the root grouping set.
    pub outer_group: Option<String>,
    /// `terms` levels, outermost first. Empty for a spec that is one metric.
    pub terms: Vec<String>,
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
        for (level, field) in self.terms.iter().enumerate().rev() {
            let terms = match self.size {
                // Every segment keeps every term, so the cut is exact and its
                // error bound zero, the same as a backend that has every group.
                Some((cut_level, size)) if level == cut_level => {
                    json!({ "field": field, "size": size, "segment_size": NO_CUT })
                }
                _ => json!({ "field": field, "size": NO_CUT, "order": { "_key": "asc" } }),
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
        match &self.outer_group {
            Some(group) => {
                format!(
                    "SELECT {group}, {call} {from_clause} WHERE {where_clause} GROUP BY {group}"
                )
            }
            None => format!("SELECT {call} {from_clause} WHERE {where_clause}"),
        }
    }

    /// The `GROUP BY` that yields one row per innermost bucket, in the column order
    /// [`Self::rows`] flattens the JSON into. Only a cut on a lone top level has a
    /// SQL form.
    pub fn pg_query(&self, from_clause: &str, where_clause: &str) -> String {
        let keys: Vec<&str> = self
            .outer_group
            .iter()
            .chain(self.terms.iter())
            .map(String::as_str)
            .collect();
        let mut select: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
        if !self.terms.is_empty() {
            select.push("COUNT(*)".to_string());
        }
        select.extend(self.metrics.iter().map(|m| m.kind.sql(&m.field)));
        let mut sql = format!(
            "SELECT {} {from_clause} WHERE {where_clause}",
            select.join(", ")
        );
        if !keys.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", keys.join(", ")));
        }
        if let Some((cut_level, size)) = self.size {
            assert_eq!(cut_level, 0, "SQL can only mirror a cut on the top level");
            // Tantivy breaks count ties on the key and puts the NULL bucket last.
            sql.push_str(&format!(
                " ORDER BY COUNT(*) DESC, {} ASC NULLS LAST LIMIT {size}",
                self.terms[0]
            ));
        }
        sql
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
    /// NUMERIC columns may be metric fields.
    numeric_metrics: bool,
    /// A `size` may cut any level, not only a lone top level under no group.
    size_anywhere: bool,
}

/// A join over a prefix of `tables` and a `pdb.agg()` whose fields belong to
/// those tables. `key_columns` are the join keys; `terms` keys come from the
/// text and integer columns, metrics from the integer and NUMERIC ones.
pub fn arb_pdb_agg_join(
    tables: Vec<String>,
    key_columns: &[Column],
) -> impl Strategy<Value = (JoinExpr, PdbAggExpr)> {
    let shape = SpecShape {
        grouped: false,
        numeric_metrics: true,
        size_anywhere: false,
    };
    let key_columns = key_columns.to_vec();
    (2..=tables.len()).prop_flat_map(move |num_tables| {
        let joined: Vec<String> = tables[..num_tables].to_vec();
        // The planner cannot see the fields inside a spec, so it removes an outer
        // join whose table nothing else reads, and the spec then names a table
        // that is gone. Inner joins are never removed.
        let join = arb_joins(Just(JoinType::Inner), joined.clone(), &key_columns);
        (join, arb_pdb_agg(joined, shape))
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
            numeric_metrics: false,
            size_anywhere: true,
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
        1 => (Just(MetricKind::Avg), proptest::sample::select(int_fields)),
    ];
    let outer_group = if shape.grouped {
        proptest::sample::select(key_fields.clone())
            .prop_map(Some)
            .boxed()
    } else {
        proptest::option::weighted(0.3, proptest::sample::select(key_fields.clone())).boxed()
    };

    (
        outer_group,
        proptest::sample::subsequence(key_fields, 0..=2),
        proptest::option::weighted(0.4, (0..2usize, 1..4u32)),
        proptest::collection::vec(metric, 0..=3),
    )
        .prop_map(move |(outer_group, terms, size, metrics)| {
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
            let size = size.filter(|&(level, _)| {
                level < terms.len()
                    && (shape.size_anywhere
                        || (level == 0 && terms.len() == 1 && outer_group.is_none()))
            });
            PdbAggExpr {
                outer_group,
                terms,
                size,
                metrics,
            }
        })
}
