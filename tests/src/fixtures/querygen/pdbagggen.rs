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

//! Generates `pdb.agg()` specs over joins together with the SQL `GROUP BY` that
//! computes the same buckets. `pdb.agg()` has no Postgres fallback, so the SQL
//! query is the oracle: its rows, and the JSON buckets flattened the same way,
//! must be equal.

use proptest::prelude::*;
use serde_json::{Value, json};
use sqlx::Row;
use sqlx::postgres::PgRow;

use crate::fixtures::querygen::joingen::{JoinExpr, JoinType, arb_joins};

/// Size that no generated bucket count reaches, so a level is never cut.
const NO_CUT: u32 = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricKind {
    Sum,
    Min,
    Max,
    ValueCount,
    Cardinality,
}

impl MetricKind {
    fn spec_name(self) -> &'static str {
        match self {
            MetricKind::Sum => "sum",
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
    /// A `size` cut under the default `_count desc` order. Only on a single level
    /// without an outer group, where SQL can express the same top-N.
    pub size: Option<u32>,
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
                Some(size) if level == 0 => json!({ "field": field, "size": size }),
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

    pub fn pdb_query(&self, join_clause: &str, where_clause: &str) -> String {
        let spec = self.spec().to_string().replace('\'', "''");
        match &self.outer_group {
            Some(group) => format!(
                "SELECT {group}, pdb.agg('{spec}') {join_clause} WHERE {where_clause} GROUP BY {group}"
            ),
            None => format!("SELECT pdb.agg('{spec}') {join_clause} WHERE {where_clause}"),
        }
    }

    /// The `GROUP BY` that yields one row per innermost bucket, in the column order
    /// [`Self::rows`] flattens the JSON into.
    pub fn pg_query(&self, join_clause: &str, where_clause: &str) -> String {
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
            "SELECT {} {join_clause} WHERE {where_clause}",
            select.join(", ")
        );
        if !keys.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", keys.join(", ")));
        }
        if let Some(size) = self.size {
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

/// A join over a prefix of `tables` and a `pdb.agg()` whose fields belong to
/// those tables. `key_columns` are the join keys; `terms` keys come from the
/// text and integer columns, metrics from the integer and NUMERIC ones.
pub fn arb_pdb_agg_join(
    tables: Vec<String>,
    key_columns: Vec<String>,
) -> impl Strategy<Value = (JoinExpr, PdbAggExpr)> {
    (2..=tables.len()).prop_flat_map(move |num_tables| {
        let joined: Vec<String> = tables[..num_tables].to_vec();
        // The planner cannot see the fields inside a spec, so it removes an outer
        // join whose table nothing else reads, and the spec then names a table
        // that is gone. Inner joins are never removed.
        let join = arb_joins(Just(JoinType::Inner), joined.clone(), key_columns.clone());
        (join, arb_pdb_agg(joined))
    })
}

fn arb_pdb_agg(tables: Vec<String>) -> impl Strategy<Value = PdbAggExpr> {
    let first_table = tables[0].clone();
    let qualify = |columns: &[&str]| -> Vec<String> {
        tables
            .iter()
            .flat_map(|t| columns.iter().map(move |c| format!("{t}.{c}")))
            .collect()
    };
    // `color` and `quantity` carry NULLs, which become a bucket of their own.
    let key_fields = qualify(&["color", "age", "quantity"]);
    let metric_fields = qualify(&["age", "quantity", "price"]);
    let kinds = [
        MetricKind::Sum,
        MetricKind::Min,
        MetricKind::Max,
        MetricKind::ValueCount,
        MetricKind::Cardinality,
    ];
    let metric = (
        proptest::sample::select(kinds.to_vec()),
        proptest::sample::select(metric_fields),
    );

    (
        proptest::option::weighted(0.3, proptest::sample::select(key_fields.clone())),
        proptest::sample::subsequence(key_fields, 0..=2),
        proptest::option::weighted(0.4, 1..4u32),
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
                        field: format!("{first_table}.age"),
                    });
                }
            }
            let size = size.filter(|_| terms.len() == 1 && outer_group.is_none());
            PdbAggExpr {
                outer_group,
                terms,
                size,
                metrics,
            }
        })
}
