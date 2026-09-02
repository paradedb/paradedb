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

//! `pdb.agg()` on the DataFusion aggregate backend.
//!
//! The Tantivy backend hands the JSON spec to Tantivy's collectors as-is. DataFusion has
//! no equivalent, so the spec is lowered here: every `terms` level becomes one grouping
//! set of a single `Aggregate`, metrics become aggregate expressions, and the nested
//! result JSON is assembled from the flat grouped rows after execution. The output
//! mirrors Tantivy's result shape so a query reads the same on either backend.

use crate::api::{HashMap, MvccVisibility};
use crate::postgres::datetime::PostgresDateTime;
use crate::postgres::types::is_datetime_type;
use crate::schema::SearchFieldType;
use arrow_array::cast::AsArray;
use arrow_array::types::{
    Float32Type, Float64Type, Int8Type, Int16Type, Int32Type, Int64Type, TimestampMicrosecondType,
    UInt8Type, UInt16Type, UInt32Type, UInt64Type,
};
use arrow_array::{Array, ArrayRef, Int64Array, RecordBatch, UInt64Array, new_null_array};
use arrow_schema::{DataType, Schema, SchemaRef, TimeUnit};
use datafusion::common::Result;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tantivy::aggregation::Key;
use tantivy::aggregation::agg_req::{Aggregation, AggregationVariants, Aggregations};
use tantivy::aggregation::bucket::{CustomOrder, Order, OrderTarget};

/// A fast field referenced by a `pdb.agg()` spec, resolved to its join source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbAggFieldRef {
    pub plan_position: usize,
    /// Heap attribute of the column. A JSON sub-field keeps its parent column's
    /// attno and is told apart by `field_name`.
    pub attno: pg_sys::AttrNumber,
    /// Index field name, dotted for a JSON sub-field.
    pub field_name: String,
    pub field_type: SearchFieldType,
}

impl PdbAggFieldRef {
    /// Datetime columns reach DataFusion as PG-epoch microseconds and are rendered
    /// as timestamps, the way the Tantivy backend rewrites them.
    pub fn is_datetime(&self) -> bool {
        match self.field_type {
            SearchFieldType::Date(_) => true,
            SearchFieldType::I64(oid) => is_datetime_type(oid),
            _ => false,
        }
    }
}

/// How a spec uses a field. Drives the type gate in the resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdbAggFieldUsage {
    TermsKey,
    NumericMetric,
    AnyMetric,
}

/// Maps a spec field name to a join source. Implemented by the planner, which owns
/// the sources; a permissive stand-in is used for the shape check.
pub trait PdbAggFieldResolver {
    fn resolve(&self, field: &str, usage: PdbAggFieldUsage) -> Result<PdbAggFieldRef, String>;
}

/// Type gate shared by every resolver. NUMERIC declines on both backends. The other
/// rejects are types DataFusion's grouping or metric accumulators cannot take.
pub fn check_field_usage(
    field: &str,
    field_type: &SearchFieldType,
    usage: PdbAggFieldUsage,
) -> Result<(), String> {
    if field_type.is_numeric() {
        return Err(format!(
            "Aggregation references NUMERIC field '{field}' which cannot be aggregated. \
             NUMERIC columns do not support aggregate pushdown."
        ));
    }
    if matches!(
        field_type,
        SearchFieldType::Range(_) | SearchFieldType::Vector(..)
    ) {
        return Err(format!(
            "field '{field}' has a type that cannot be aggregated"
        ));
    }
    if usage == PdbAggFieldUsage::NumericMetric
        && !matches!(
            field_type,
            SearchFieldType::I64(_)
                | SearchFieldType::U64(_)
                | SearchFieldType::F64(_)
                | SearchFieldType::Date(_)
        )
    {
        return Err(format!(
            "field '{field}' must be a numeric or date field for this metric aggregation"
        ));
    }
    Ok(())
}

/// A literal from the spec, such as a `missing` value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PdbKey {
    Str(String),
    I64(i64),
    U64(u64),
    F64(f64),
}

impl From<&Key> for PdbKey {
    fn from(key: &Key) -> Self {
        match key {
            Key::Str(s) => PdbKey::Str(s.clone()),
            Key::I64(v) => PdbKey::I64(*v),
            Key::U64(v) => PdbKey::U64(*v),
            Key::F64(v) => PdbKey::F64(*v),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PdbMetricKind {
    Sum,
    Avg,
    Min,
    Max,
    ValueCount,
    Cardinality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PdbOrderTarget {
    Count,
    Key,
    SubAggregation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbTermsOrder {
    pub target: PdbOrderTarget,
    pub asc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbTermsAgg {
    pub field: PdbAggFieldRef,
    pub size: u32,
    pub min_doc_count: u64,
    pub order: PdbTermsOrder,
    pub missing: Option<PdbKey>,
    pub show_doc_count_error: bool,
    pub sub_aggs: Vec<(String, PdbAggNode)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbMetricAgg {
    pub kind: PdbMetricKind,
    pub field: PdbAggFieldRef,
    pub missing: Option<PdbKey>,
    /// `sum` reports `null` rather than `0` on an empty input when the spec asks for it.
    pub none_if_no_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PdbAggNode {
    Terms(PdbTermsAgg),
    Metric(PdbMetricAgg),
}

impl PdbAggNode {
    fn has_terms(&self) -> bool {
        match self {
            PdbAggNode::Terms(_) => true,
            PdbAggNode::Metric(_) => false,
        }
    }

    fn for_each_field(&self, f: &mut impl FnMut(&PdbAggFieldRef)) {
        match self {
            PdbAggNode::Terms(terms) => {
                f(&terms.field);
                for (_, sub) in &terms.sub_aggs {
                    sub.for_each_field(f);
                }
            }
            PdbAggNode::Metric(metric) => f(&metric.field),
        }
    }
}

/// One `pdb.agg()` call, lowered for DataFusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbAggRequest {
    pub root: PdbAggNode,
    pub visibility: MvccVisibility,
    /// The spec as written, for EXPLAIN.
    pub agg_json: serde_json::Value,
}

impl PdbAggRequest {
    pub fn lower(
        agg_json: serde_json::Value,
        visibility: MvccVisibility,
        resolver: &dyn PdbAggFieldResolver,
    ) -> Result<Self, String> {
        let agg: Aggregation = serde_json::from_value(agg_json.clone())
            .map_err(|e| format!("invalid pdb.agg specification: {e}"))?;
        let root = lower_node(&agg, resolver)?;
        Ok(Self {
            root,
            visibility,
            agg_json,
        })
    }

    /// Whether the DataFusion backend can run this spec, judged from the JSON alone.
    /// Field resolution is left to [`Self::lower`]; a bad field fails the same way on
    /// the Tantivy backend, so it is no reason to prefer one over the other.
    pub fn check_shape(agg_json: &serde_json::Value) -> Result<(), String> {
        struct AcceptAll;
        impl PdbAggFieldResolver for AcceptAll {
            fn resolve(
                &self,
                field: &str,
                _usage: PdbAggFieldUsage,
            ) -> Result<PdbAggFieldRef, String> {
                // Text takes any `missing` literal, so no spec is turned down for a
                // type this stand-in cannot know.
                Ok(PdbAggFieldRef {
                    plan_position: 0,
                    attno: 0,
                    field_name: field.to_string(),
                    field_type: SearchFieldType::Text(pg_sys::TEXTOID),
                })
            }
        }
        let agg: Aggregation = serde_json::from_value(agg_json.clone())
            .map_err(|e| format!("invalid pdb.agg specification: {e}"))?;
        lower_node(&agg, &AcceptAll).map(|_| ())
    }

    /// True when the spec groups, which turns the plan into grouping sets.
    pub fn has_terms(&self) -> bool {
        self.root.has_terms()
    }

    pub fn for_each_field(&self, mut f: impl FnMut(&PdbAggFieldRef)) {
        self.root.for_each_field(&mut f);
    }
}

fn variant_name(agg: &AggregationVariants) -> &'static str {
    match agg {
        AggregationVariants::Range(_) => "range",
        AggregationVariants::Histogram(_) => "histogram",
        AggregationVariants::DateHistogram(_) => "date_histogram",
        AggregationVariants::Terms(_) => "terms",
        AggregationVariants::Filter(_) => "filter",
        AggregationVariants::Composite(_) => "composite",
        AggregationVariants::MultiTerms(_) => "multi_terms",
        AggregationVariants::Average(_) => "avg",
        AggregationVariants::Count(_) => "value_count",
        AggregationVariants::Max(_) => "max",
        AggregationVariants::Min(_) => "min",
        AggregationVariants::Stats(_) => "stats",
        AggregationVariants::ExtendedStats(_) => "extended_stats",
        AggregationVariants::Sum(_) => "sum",
        AggregationVariants::Percentiles(_) => "percentiles",
        AggregationVariants::TopHits(_) => "top_hits",
        AggregationVariants::Cardinality(_) => "cardinality",
    }
}

/// `missing` stands in for the field's own values, so it has to be one of them.
/// A number on a text field renders as its text. A string on a numeric field has
/// no value to take, and Tantivy's request has no boolean literal.
fn check_missing(
    field: &PdbAggFieldRef,
    missing: Option<PdbKey>,
) -> Result<Option<PdbKey>, String> {
    let Some(missing) = missing else {
        return Ok(None);
    };
    let name = &field.field_name;
    match (&field.field_type, missing) {
        (SearchFieldType::Bool(_), _) => Err(format!(
            "`missing` is not supported for boolean field '{name}'"
        )),
        (
            SearchFieldType::I64(_)
            | SearchFieldType::U64(_)
            | SearchFieldType::F64(_)
            | SearchFieldType::Date(_),
            PdbKey::Str(_),
        ) => Err(format!(
            "`missing` for numeric field '{name}' must be a number"
        )),
        // A timestamp column takes its literal as whole microseconds.
        (_, PdbKey::F64(v)) if field.is_datetime() => Ok(Some(PdbKey::I64(v as i64))),
        (_, missing) => Ok(Some(missing)),
    }
}

fn lower_node(agg: &Aggregation, resolver: &dyn PdbAggFieldResolver) -> Result<PdbAggNode, String> {
    let metric = |kind: PdbMetricKind,
                  field: &str,
                  missing: Option<PdbKey>,
                  none_if_no_match: bool|
     -> Result<PdbAggNode, String> {
        if !agg.sub_aggregation.is_empty() {
            return Err(format!(
                "`{}` is a metric aggregation and cannot have sub-aggregations",
                variant_name(&agg.agg)
            ));
        }
        let usage = match kind {
            PdbMetricKind::Sum | PdbMetricKind::Avg | PdbMetricKind::Min | PdbMetricKind::Max => {
                PdbAggFieldUsage::NumericMetric
            }
            PdbMetricKind::ValueCount | PdbMetricKind::Cardinality => PdbAggFieldUsage::AnyMetric,
        };
        let field = resolver.resolve(field, usage)?;
        let missing = check_missing(&field, missing)?;
        Ok(PdbAggNode::Metric(PdbMetricAgg {
            kind,
            field,
            missing,
            none_if_no_match,
        }))
    };

    match &agg.agg {
        AggregationVariants::Terms(terms) => {
            if terms.include.is_some() || terms.exclude.is_some() {
                return Err(
                    "terms `include` and `exclude` are not supported on the DataFusion backend"
                        .into(),
                );
            }
            // Tantivy answers `min_doc_count: 0` with every term of the column
            // dictionary. A grouped scan only sees the terms of matched rows.
            if terms.min_doc_count == Some(0) {
                return Err(
                    "terms `min_doc_count: 0` is not supported on the DataFusion backend".into(),
                );
            }
            let field = resolver.resolve(&terms.field, PdbAggFieldUsage::TermsKey)?;
            let missing = check_missing(&field, terms.missing.as_ref().map(PdbKey::from))?;
            let order = terms.order.clone().unwrap_or_default();
            // Tantivy only reports the error bound under the default ordering.
            let show_doc_count_error = terms
                .show_term_doc_count_error
                .unwrap_or(order == CustomOrder::default());
            let sub_aggs = lower_sub_aggs(&agg.sub_aggregation, resolver)?;
            let target = match &order.target {
                OrderTarget::Count => PdbOrderTarget::Count,
                OrderTarget::Key => PdbOrderTarget::Key,
                OrderTarget::SubAggregation(name) => {
                    // Tantivy accepts `name.property` for multi-value metrics; only
                    // single-value metrics are supported here, so the property is
                    // irrelevant.
                    let agg_name = name.split_once('.').map(|(n, _)| n).unwrap_or(name);
                    match sub_aggs.iter().find(|(n, _)| n == agg_name) {
                        Some((_, PdbAggNode::Metric(_))) => {}
                        Some(_) => {
                            return Err(format!(
                                "terms order target '{agg_name}' must be a metric sub-aggregation"
                            ));
                        }
                        None => {
                            return Err(format!(
                                "terms order references unknown sub-aggregation '{agg_name}'"
                            ));
                        }
                    }
                    PdbOrderTarget::SubAggregation(agg_name.to_string())
                }
            };
            Ok(PdbAggNode::Terms(PdbTermsAgg {
                field,
                size: terms.size.unwrap_or(10),
                min_doc_count: terms.min_doc_count.unwrap_or(1),
                order: PdbTermsOrder {
                    target,
                    asc: order.order == Order::Asc,
                },
                missing,
                show_doc_count_error,
                sub_aggs,
            }))
        }
        AggregationVariants::Sum(m) => metric(
            PdbMetricKind::Sum,
            &m.field,
            m.missing.map(PdbKey::F64),
            m.none_if_no_match.unwrap_or(false),
        ),
        AggregationVariants::Average(m) => metric(
            PdbMetricKind::Avg,
            &m.field,
            m.missing.map(PdbKey::F64),
            false,
        ),
        AggregationVariants::Min(m) => metric(
            PdbMetricKind::Min,
            &m.field,
            m.missing.map(PdbKey::F64),
            false,
        ),
        AggregationVariants::Max(m) => metric(
            PdbMetricKind::Max,
            &m.field,
            m.missing.map(PdbKey::F64),
            false,
        ),
        AggregationVariants::Count(m) => metric(
            PdbMetricKind::ValueCount,
            &m.field,
            m.missing.map(PdbKey::F64),
            false,
        ),
        AggregationVariants::Cardinality(m) => metric(
            PdbMetricKind::Cardinality,
            &m.field,
            m.missing.as_ref().map(PdbKey::from),
            false,
        ),
        other => Err(format!(
            "`{}` aggregations are not supported on the DataFusion backend",
            variant_name(other)
        )),
    }
}

fn lower_sub_aggs(
    aggs: &Aggregations,
    resolver: &dyn PdbAggFieldResolver,
) -> Result<Vec<(String, PdbAggNode)>, String> {
    // The request map is unordered; sort so column naming is stable across plans.
    let mut names: Vec<&String> = aggs.keys().collect();
    names.sort();
    names
        .into_iter()
        .map(|name| Ok((name.clone(), lower_node(&aggs[name], resolver)?)))
        .collect()
}

/// A grouping key interned across every `terms` node of a query.
#[derive(Debug, Clone)]
pub struct PdbKeySpec {
    pub field: PdbAggFieldRef,
    pub missing: Option<PdbKey>,
}

/// An aggregate expression interned across every metric node of a query. `kind`
/// is `None` for the bucket doc count.
#[derive(Debug, Clone)]
pub struct PdbMetricSpec {
    pub kind: Option<PdbMetricKind>,
    pub field: Option<PdbAggFieldRef>,
    pub missing: Option<PdbKey>,
    /// Target-list index of the `pdb.agg()` entry whose `FILTER` applies, if any.
    pub entry_filter: Option<usize>,
}

/// Column bindings of one lowered node into the DataFusion output.
#[derive(Debug, Clone)]
pub enum PdbAggNodeLayout {
    Terms {
        node_id: usize,
        level: usize,
        /// Key columns from the SQL group keys through this node's own key, in that
        /// order. The prefix identifies the parent bucket, the last one the bucket.
        key_cols: Vec<usize>,
        doc_count_col: usize,
        field: PdbAggFieldRef,
        size: u32,
        min_doc_count: u64,
        order: PdbTermsOrder,
        show_doc_count_error: bool,
        sub: Vec<(String, PdbAggNodeLayout)>,
    },
    Metric {
        value_col: usize,
        kind: PdbMetricKind,
        field: PdbAggFieldRef,
        none_if_no_match: bool,
    },
}

/// The grouping-set plan for every `pdb.agg()` in a query, plus where each piece
/// lands in the final DataFusion output. The output column order is:
/// SQL group keys, standard aggregates, `__grouping_id` (when any spec groups),
/// interned terms keys, interned metrics.
#[derive(Debug, Clone)]
pub struct PdbAggPlan {
    pub keys: Vec<PdbKeySpec>,
    pub metrics: Vec<PdbMetricSpec>,
    /// Per level, positions into `[SQL group keys ++ keys]`. Level 0 is the SQL level.
    pub levels: Vec<Vec<usize>>,
    /// Per `pdb.agg()` entry, in target-list order.
    pub entries: Vec<PdbAggNodeLayout>,
    num_outer_group_cols: usize,
    num_std_aggs: usize,
    num_terms_nodes: usize,
}

/// Identity of an interned metric: kind, `(plan_position, field_name)`, the
/// `missing` literal as JSON, and the entry whose `FILTER` applies.
type MetricId = (
    Option<PdbMetricKind>,
    Option<(usize, String)>,
    String,
    Option<usize>,
);

struct PlanBuilder {
    plan: PdbAggPlan,
    key_ids: HashMap<(usize, String, String), usize>,
    metric_ids: HashMap<MetricId, usize>,
    level_ids: HashMap<Vec<usize>, usize>,
}

impl PlanBuilder {
    fn intern_key(&mut self, field: &PdbAggFieldRef, missing: &Option<PdbKey>) -> usize {
        let id = (
            field.plan_position,
            field.field_name.clone(),
            serde_json::to_string(missing).unwrap_or_default(),
        );
        if let Some(&idx) = self.key_ids.get(&id) {
            return idx;
        }
        let idx = self.plan.keys.len();
        self.plan.keys.push(PdbKeySpec {
            field: field.clone(),
            missing: missing.clone(),
        });
        self.key_ids.insert(id, idx);
        idx
    }

    fn intern_metric(
        &mut self,
        kind: Option<PdbMetricKind>,
        field: Option<&PdbAggFieldRef>,
        missing: &Option<PdbKey>,
        entry_filter: Option<usize>,
    ) -> usize {
        let id = (
            kind,
            field.map(|f| (f.plan_position, f.field_name.clone())),
            serde_json::to_string(missing).unwrap_or_default(),
            entry_filter,
        );
        if let Some(&idx) = self.metric_ids.get(&id) {
            return idx;
        }
        let idx = self.plan.metrics.len();
        self.plan.metrics.push(PdbMetricSpec {
            kind,
            field: field.cloned(),
            missing: missing.clone(),
            entry_filter,
        });
        self.metric_ids.insert(id, idx);
        idx
    }

    fn intern_level(&mut self, positions: &[usize]) -> usize {
        let mut set = positions.to_vec();
        set.sort_unstable();
        if let Some(&idx) = self.level_ids.get(&set) {
            return idx;
        }
        let idx = self.plan.levels.len();
        self.plan.levels.push(set.clone());
        self.level_ids.insert(set, idx);
        idx
    }

    fn lower(
        &mut self,
        node: &PdbAggNode,
        parent_positions: &[usize],
        entry_filter: Option<usize>,
    ) -> PdbAggNodeLayout {
        match node {
            PdbAggNode::Terms(terms) => {
                let key_idx = self.intern_key(&terms.field, &terms.missing);
                let mut positions = parent_positions.to_vec();
                positions.push(self.plan.num_outer_group_cols + key_idx);
                let level = self.intern_level(&positions);
                let doc_count_idx = self.intern_metric(None, None, &None, entry_filter);
                let node_id = self.plan.num_terms_nodes;
                self.plan.num_terms_nodes += 1;
                let sub = terms
                    .sub_aggs
                    .iter()
                    .map(|(name, sub)| (name.clone(), self.lower(sub, &positions, entry_filter)))
                    .collect();
                PdbAggNodeLayout::Terms {
                    node_id,
                    level,
                    key_cols: positions,
                    doc_count_col: doc_count_idx,
                    field: terms.field.clone(),
                    size: terms.size,
                    min_doc_count: terms.min_doc_count,
                    order: terms.order.clone(),
                    show_doc_count_error: terms.show_doc_count_error,
                    sub,
                }
            }
            PdbAggNode::Metric(metric) => {
                let value_idx = self.intern_metric(
                    Some(metric.kind),
                    Some(&metric.field),
                    &metric.missing,
                    entry_filter,
                );
                PdbAggNodeLayout::Metric {
                    value_col: value_idx,
                    kind: metric.kind,
                    field: metric.field.clone(),
                    none_if_no_match: metric.none_if_no_match,
                }
            }
        }
    }
}

impl PdbAggPlan {
    /// `entries` are the `pdb.agg()` target-list entries in order: the target-list
    /// index, the lowered request, and whether the entry carries a `FILTER`.
    pub fn build(
        entries: &[(usize, &PdbAggRequest, bool)],
        num_outer_group_cols: usize,
        num_std_aggs: usize,
    ) -> Self {
        let mut builder = PlanBuilder {
            plan: PdbAggPlan {
                keys: Vec::new(),
                metrics: Vec::new(),
                levels: Vec::new(),
                entries: Vec::new(),
                num_outer_group_cols,
                num_std_aggs,
                num_terms_nodes: 0,
            },
            key_ids: HashMap::default(),
            metric_ids: HashMap::default(),
            level_ids: HashMap::default(),
        };
        let root_positions: Vec<usize> = (0..num_outer_group_cols).collect();
        builder.intern_level(&root_positions);
        for &(agg_idx, request, has_filter) in entries {
            let layout = builder.lower(
                &request.root,
                &root_positions,
                has_filter.then_some(agg_idx),
            );
            builder.plan.entries.push(layout);
        }
        // Key and metric indices were interned relative to their own lists; rebase
        // them onto the final output columns now that every list is complete.
        let mut plan = builder.plan;
        let entries = std::mem::take(&mut plan.entries);
        plan.entries = entries
            .into_iter()
            .map(|layout| plan.rebase(layout))
            .collect();
        plan
    }

    fn rebase(&self, layout: PdbAggNodeLayout) -> PdbAggNodeLayout {
        match layout {
            PdbAggNodeLayout::Terms {
                node_id,
                level,
                key_cols,
                doc_count_col,
                field,
                size,
                min_doc_count,
                order,
                show_doc_count_error,
                sub,
            } => PdbAggNodeLayout::Terms {
                node_id,
                level,
                key_cols: key_cols.into_iter().map(|p| self.group_col(p)).collect(),
                doc_count_col: self.metric_col(doc_count_col),
                field,
                size,
                min_doc_count,
                order,
                show_doc_count_error,
                sub: sub
                    .into_iter()
                    .map(|(name, sub)| (name, self.rebase(sub)))
                    .collect(),
            },
            PdbAggNodeLayout::Metric {
                value_col,
                kind,
                field,
                none_if_no_match,
            } => PdbAggNodeLayout::Metric {
                value_col: self.metric_col(value_col),
                kind,
                field,
                none_if_no_match,
            },
        }
    }

    /// True when any spec has a `terms` level, which makes the plan use grouping sets.
    pub fn has_grouping_sets(&self) -> bool {
        !self.keys.is_empty()
    }

    fn num_group_exprs(&self) -> usize {
        self.num_outer_group_cols + self.keys.len()
    }

    /// Output column of a grouping expression by its position in
    /// `[SQL group keys ++ keys]`.
    fn group_col(&self, position: usize) -> usize {
        if position < self.num_outer_group_cols {
            position
        } else {
            self.key_col(position - self.num_outer_group_cols)
        }
    }

    pub fn grouping_id_col(&self) -> Option<usize> {
        self.has_grouping_sets()
            .then_some(self.num_outer_group_cols + self.num_std_aggs)
    }

    pub fn key_col(&self, key_idx: usize) -> usize {
        self.num_outer_group_cols + self.num_std_aggs + 1 + key_idx
    }

    pub fn metric_col(&self, metric_idx: usize) -> usize {
        let grouping_id = usize::from(self.has_grouping_sets());
        self.num_outer_group_cols + self.num_std_aggs + grouping_id + self.keys.len() + metric_idx
    }

    /// The `__grouping_id` DataFusion assigns to a level: one bit per grouping
    /// expression, most significant first, set when the expression is absent.
    pub fn grouping_id_for_level(&self, level: usize) -> u64 {
        let n = self.num_group_exprs();
        (0..n)
            .filter(|position| !self.levels[level].contains(position))
            .fold(0u64, |acc, position| acc | (1u64 << (n - 1 - position)))
    }

    pub fn root_grouping_id(&self) -> u64 {
        self.grouping_id_for_level(0)
    }
}

/// A bucket key or SQL group value read back from Arrow. `F64` holds the bit
/// pattern so the value can be hashed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum KeyValue {
    Null,
    Str(String),
    I64(i64),
    U64(u64),
    F64(u64),
    Bool(bool),
}

impl KeyValue {
    fn read(col: &ArrayRef, row: usize) -> KeyValue {
        if col.is_null(row) {
            return KeyValue::Null;
        }
        match col.data_type() {
            DataType::Utf8 => KeyValue::Str(col.as_string::<i32>().value(row).to_string()),
            DataType::LargeUtf8 => KeyValue::Str(col.as_string::<i64>().value(row).to_string()),
            DataType::Utf8View => KeyValue::Str(col.as_string_view().value(row).to_string()),
            DataType::Int8 => KeyValue::I64(col.as_primitive::<Int8Type>().value(row) as i64),
            DataType::Int16 => KeyValue::I64(col.as_primitive::<Int16Type>().value(row) as i64),
            DataType::Int32 => KeyValue::I64(col.as_primitive::<Int32Type>().value(row) as i64),
            DataType::Int64 => KeyValue::I64(col.as_primitive::<Int64Type>().value(row)),
            DataType::UInt8 => KeyValue::U64(col.as_primitive::<UInt8Type>().value(row) as u64),
            DataType::UInt16 => KeyValue::U64(col.as_primitive::<UInt16Type>().value(row) as u64),
            DataType::UInt32 => KeyValue::U64(col.as_primitive::<UInt32Type>().value(row) as u64),
            DataType::UInt64 => KeyValue::U64(col.as_primitive::<UInt64Type>().value(row)),
            DataType::Float32 => {
                KeyValue::F64((col.as_primitive::<Float32Type>().value(row) as f64).to_bits())
            }
            DataType::Float64 => {
                KeyValue::F64(col.as_primitive::<Float64Type>().value(row).to_bits())
            }
            DataType::Boolean => KeyValue::Bool(col.as_boolean().value(row)),
            DataType::Timestamp(TimeUnit::Microsecond, _) => {
                KeyValue::I64(col.as_primitive::<TimestampMicrosecondType>().value(row))
            }
            other => panic!("BUG: unsupported pdb.agg group key type {other}"),
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            KeyValue::Null => None,
            KeyValue::Str(_) => None,
            KeyValue::I64(v) => Some(*v as f64),
            KeyValue::U64(v) => Some(*v as f64),
            KeyValue::F64(bits) => Some(f64::from_bits(*bits)),
            KeyValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        }
    }

    /// NULL sorts after every value, where the Tantivy backend's default
    /// `missing` sentinel lands.
    fn compare(&self, other: &KeyValue) -> std::cmp::Ordering {
        match (self, other) {
            (KeyValue::Null, KeyValue::Null) => std::cmp::Ordering::Equal,
            (KeyValue::Null, _) => std::cmp::Ordering::Greater,
            (_, KeyValue::Null) => std::cmp::Ordering::Less,
            (KeyValue::Str(a), KeyValue::Str(b)) => a.cmp(b),
            (a, b) => a
                .as_f64()
                .unwrap_or(f64::MIN)
                .total_cmp(&b.as_f64().unwrap_or(f64::MIN)),
        }
    }

    /// The JSON `key` of a bucket, plus `key_as_string` when Tantivy emits one.
    fn to_bucket_key(&self, field: &PdbAggFieldRef) -> (serde_json::Value, Option<String>) {
        match self {
            KeyValue::Null => (serde_json::Value::Null, None),
            KeyValue::Str(s) => (serde_json::Value::String(s.clone()), None),
            KeyValue::I64(v) if field.is_datetime() => (
                pg_micros_to_string(*v)
                    .map(serde_json::Value::String)
                    .unwrap_or_else(|| serde_json::Value::from(*v)),
                None,
            ),
            KeyValue::I64(v) => (serde_json::Value::from(*v), None),
            KeyValue::U64(v) => (serde_json::Value::from(*v), None),
            KeyValue::F64(bits) => (
                serde_json::Number::from_f64(f64::from_bits(*bits))
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null),
                None,
            ),
            KeyValue::Bool(b) => (
                serde_json::Value::from(u64::from(*b)),
                Some(if *b { "true" } else { "false" }.to_string()),
            ),
        }
    }
}

/// `None` outside the timestamp range, which a `sum` over a datetime column
/// reaches quickly.
fn pg_micros_to_string(micros: i64) -> Option<String> {
    PostgresDateTime::try_from_raw(micros)
        .ok()
        .map(|timestamp| timestamp.to_string())
}

fn f64_at(col: &ArrayRef, row: usize) -> Option<f64> {
    KeyValue::read(col, row).as_f64()
}

fn u64_at(col: &ArrayRef, row: usize) -> u64 {
    match KeyValue::read(col, row) {
        KeyValue::I64(v) => v.max(0) as u64,
        KeyValue::U64(v) => v,
        KeyValue::F64(bits) => f64::from_bits(bits) as u64,
        _ => 0,
    }
}

/// Root rows of a grouped result together with the assembled `pdb.agg()` JSON.
pub struct AssembledPdbAggRows {
    /// The SQL-level rows, projected to the SQL group keys and standard aggregates.
    pub root_batch: RecordBatch,
    /// Per root row, one JSON document per `pdb.agg()` entry in target-list order.
    pub json: Vec<Vec<serde_json::Value>>,
}

struct Bucket {
    key: KeyValue,
    doc_count: u64,
    subs: Vec<(String, serde_json::Value)>,
}

/// Rows of one terms node grouped by their parent bucket.
type NodeIndex = HashMap<Vec<KeyValue>, Vec<(KeyValue, usize)>>;

struct Assembler<'a> {
    batch: &'a RecordBatch,
    indexes: Vec<NodeIndex>,
}

impl Assembler<'_> {
    fn emit(
        &self,
        node: &PdbAggNodeLayout,
        prefix: &[KeyValue],
        row: Option<usize>,
    ) -> serde_json::Value {
        match node {
            PdbAggNodeLayout::Metric {
                value_col,
                kind,
                field,
                none_if_no_match,
            } => {
                let value = row.and_then(|row| f64_at(self.batch.column(*value_col), row));
                metric_json(*kind, field, value, *none_if_no_match)
            }
            PdbAggNodeLayout::Terms {
                node_id,
                key_cols,
                doc_count_col,
                field,
                size,
                min_doc_count,
                order,
                show_doc_count_error,
                sub,
                ..
            } => {
                // Rows without the field form a `null` bucket. The Tantivy backend
                // gives them one too, through a `missing` sentinel it adds to every
                // `terms`, so the two backends group the same way. Only the key
                // differs: the sentinel is rendered as `null` for text and leaks
                // as an extreme value for numbers.
                let children = self.indexes[*node_id].get(prefix);
                let mut buckets: Vec<Bucket> = children
                    .into_iter()
                    .flatten()
                    .map(|(key, child_row)| {
                        let doc_count = u64_at(self.batch.column(*doc_count_col), *child_row);
                        (key, *child_row, doc_count)
                    })
                    // Judged before the sub-aggregations are built, since a dropped
                    // bucket's subtree is never read.
                    .filter(|(_, _, doc_count)| *doc_count >= *min_doc_count)
                    .map(|(key, child_row, doc_count)| {
                        let mut child_prefix = Vec::with_capacity(key_cols.len());
                        child_prefix.extend_from_slice(prefix);
                        child_prefix.push(key.clone());
                        let subs = sub
                            .iter()
                            .map(|(name, node)| {
                                (
                                    name.clone(),
                                    self.emit(node, &child_prefix, Some(child_row)),
                                )
                            })
                            .collect();
                        Bucket {
                            key: key.clone(),
                            doc_count,
                            subs,
                        }
                    })
                    .collect();

                sort_buckets(&mut buckets, order);

                let size = *size as usize;
                let sum_other_doc_count: u64 = buckets
                    .get(size..)
                    .map(|dropped| dropped.iter().map(|b| b.doc_count).sum())
                    .unwrap_or(0);
                buckets.truncate(size);

                let buckets: Vec<serde_json::Value> = buckets
                    .into_iter()
                    .map(|bucket| {
                        let mut obj = serde_json::Map::new();
                        let (key, key_as_string) = bucket.key.to_bucket_key(field);
                        obj.insert("key".into(), key);
                        if let Some(key_as_string) = key_as_string {
                            obj.insert("key_as_string".into(), key_as_string.into());
                        }
                        obj.insert("doc_count".into(), bucket.doc_count.into());
                        for (name, value) in bucket.subs {
                            obj.insert(name, value);
                        }
                        serde_json::Value::Object(obj)
                    })
                    .collect();

                let mut obj = serde_json::Map::new();
                obj.insert("buckets".into(), buckets.into());
                obj.insert("sum_other_doc_count".into(), sum_other_doc_count.into());
                if *show_doc_count_error {
                    // Every bucket is exact, so the bound is always zero.
                    obj.insert("doc_count_error_upper_bound".into(), 0u64.into());
                }
                serde_json::Value::Object(obj)
            }
        }
    }
}

/// What a metric reports over no input: counts and sums read 0, the rest `null`,
/// like Tantivy. `sum` alone can opt into `null`.
fn metric_value(kind: PdbMetricKind, value: Option<f64>, none_if_no_match: bool) -> Option<f64> {
    match (kind, value) {
        (PdbMetricKind::Sum, None) if !none_if_no_match => Some(0.0),
        (PdbMetricKind::ValueCount | PdbMetricKind::Cardinality, None) => Some(0.0),
        (_, value) => value,
    }
}

fn metric_json(
    kind: PdbMetricKind,
    field: &PdbAggFieldRef,
    value: Option<f64>,
    none_if_no_match: bool,
) -> serde_json::Value {
    let value = metric_value(kind, value, none_if_no_match);
    let mut obj = serde_json::Map::new();
    let json_value = value
        .and_then(serde_json::Number::from_f64)
        .map(serde_json::Value::Number)
        .unwrap_or(serde_json::Value::Null);
    obj.insert("value".into(), json_value);
    if field.is_datetime()
        && matches!(
            kind,
            PdbMetricKind::Sum | PdbMetricKind::Avg | PdbMetricKind::Min | PdbMetricKind::Max
        )
        && let Some(key_as_string) = value.and_then(|value| pg_micros_to_string(value as i64))
    {
        obj.insert("key_as_string".into(), key_as_string.into());
    }
    serde_json::Value::Object(obj)
}

/// Tantivy orders by the requested target only; ties are left in collection
/// order. Ties break on the key here so the output is stable.
fn sort_buckets(buckets: &mut [Bucket], order: &PdbTermsOrder) {
    let by_key = |a: &Bucket, b: &Bucket| a.key.compare(&b.key);
    match &order.target {
        PdbOrderTarget::Key => {
            buckets.sort_by(|a, b| {
                let ord = by_key(a, b);
                if order.asc { ord } else { ord.reverse() }
            });
        }
        PdbOrderTarget::Count => {
            buckets.sort_by(|a, b| {
                let ord = a.doc_count.cmp(&b.doc_count);
                let ord = if order.asc { ord } else { ord.reverse() };
                ord.then_with(|| by_key(a, b))
            });
        }
        PdbOrderTarget::SubAggregation(name) => {
            let value_of = |bucket: &Bucket| -> f64 {
                bucket
                    .subs
                    .iter()
                    .find(|(n, _)| n == name)
                    .and_then(|(_, v)| v.get("value"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(f64::MIN)
            };
            buckets.sort_by(|a, b| {
                let ord = value_of(a).total_cmp(&value_of(b));
                let ord = if order.asc { ord } else { ord.reverse() };
                ord.then_with(|| by_key(a, b))
            });
        }
    }
}

fn grouping_ids(col: &ArrayRef) -> Vec<u64> {
    (0..col.len())
        .map(|row| match KeyValue::read(col, row) {
            KeyValue::U64(v) => v,
            KeyValue::I64(v) => v as u64,
            other => panic!("BUG: unexpected __grouping_id value {other:?}"),
        })
        .collect()
}

fn collect_terms_nodes<'a>(node: &'a PdbAggNodeLayout, out: &mut Vec<&'a PdbAggNodeLayout>) {
    if let PdbAggNodeLayout::Terms { sub, .. } = node {
        out.push(node);
        for (_, sub) in sub {
            collect_terms_nodes(sub, out);
        }
    }
}

/// Fold the grouped rows into one row per SQL group, each carrying its assembled
/// `pdb.agg()` documents.
///
/// `num_root_cols` is the width of the SQL-level projection (group keys and
/// standard aggregates). `synthesize_empty_root` asks for one root row when the
/// input produced none, which is what a scalar aggregate over no rows returns:
/// NULL everywhere except the `zero_on_empty` columns, the counts, which are 0.
pub fn assemble_pdb_agg_rows(
    schema: SchemaRef,
    batches: &[RecordBatch],
    plan: &PdbAggPlan,
    num_root_cols: usize,
    synthesize_empty_root: Option<&[usize]>,
) -> Result<AssembledPdbAggRows> {
    let batch = arrow_select::concat::concat_batches(&schema, batches)?;

    let level_of_row: Vec<usize> = match plan.grouping_id_col() {
        Some(col) => {
            let gid_to_level: HashMap<u64, usize> = (0..plan.levels.len())
                .map(|level| (plan.grouping_id_for_level(level), level))
                .collect();
            grouping_ids(batch.column(col))
                .into_iter()
                .map(|gid| {
                    *gid_to_level
                        .get(&gid)
                        .unwrap_or_else(|| panic!("BUG: unplanned grouping id {gid}"))
                })
                .collect()
        }
        None => vec![0; batch.num_rows()],
    };

    let mut terms_nodes = Vec::new();
    for entry in &plan.entries {
        collect_terms_nodes(entry, &mut terms_nodes);
    }
    let mut indexes: Vec<NodeIndex> = vec![HashMap::default(); plan.num_terms_nodes];
    for node in terms_nodes {
        let PdbAggNodeLayout::Terms {
            node_id,
            level,
            key_cols,
            ..
        } = node
        else {
            unreachable!("collect_terms_nodes only yields terms nodes");
        };
        let (prefix_cols, own_col) = key_cols.split_at(key_cols.len() - 1);
        for (row, row_level) in level_of_row.iter().enumerate() {
            if row_level != level {
                continue;
            }
            let prefix: Vec<KeyValue> = prefix_cols
                .iter()
                .map(|&col| KeyValue::read(batch.column(col), row))
                .collect();
            let key = KeyValue::read(batch.column(own_col[0]), row);
            indexes[*node_id]
                .entry(prefix)
                .or_default()
                .push((key, row));
        }
    }

    let assembler = Assembler {
        batch: &batch,
        indexes,
    };
    let root_rows: Vec<usize> = level_of_row
        .iter()
        .enumerate()
        .filter(|(_, level)| **level == 0)
        .map(|(row, _)| row)
        .collect();

    let root_cols: Vec<usize> = (0..num_root_cols).collect();
    if root_rows.is_empty()
        && let Some(zero_on_empty) = synthesize_empty_root
    {
        let json = vec![
            plan.entries
                .iter()
                .map(|entry| assembler.emit(entry, &[], None))
                .collect(),
        ];
        let fields: Vec<_> = schema
            .fields()
            .iter()
            .take(num_root_cols)
            .map(|f| f.as_ref().clone().with_nullable(true))
            .collect();
        let null_schema = Arc::new(Schema::new(fields));
        let columns = null_schema
            .fields()
            .iter()
            .enumerate()
            .map(|(col, f)| {
                if zero_on_empty.contains(&col) && *f.data_type() == DataType::Int64 {
                    Arc::new(Int64Array::from(vec![0i64])) as ArrayRef
                } else {
                    new_null_array(f.data_type(), 1)
                }
            })
            .collect();
        let root_batch = RecordBatch::try_new(null_schema, columns)?;
        return Ok(AssembledPdbAggRows { root_batch, json });
    }

    let json = root_rows
        .iter()
        .map(|&row| {
            let prefix: Vec<KeyValue> = (0..plan.num_outer_group_cols)
                .map(|col| KeyValue::read(batch.column(col), row))
                .collect();
            plan.entries
                .iter()
                .map(|entry| assembler.emit(entry, &prefix, Some(row)))
                .collect()
        })
        .collect();

    let indices = UInt64Array::from(root_rows.iter().map(|&r| r as u64).collect::<Vec<_>>());
    let root_batch =
        arrow_select::take::take_record_batch(&batch, &indices)?.project(&root_cols)?;
    Ok(AssembledPdbAggRows { root_batch, json })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(plan_position: usize, name: &str) -> PdbAggFieldRef {
        PdbAggFieldRef {
            plan_position,
            attno: 1,
            field_name: name.to_string(),
            field_type: SearchFieldType::I64(pg_sys::INT8OID),
        }
    }

    fn terms(name: &str, sub_aggs: Vec<(String, PdbAggNode)>) -> PdbAggNode {
        PdbAggNode::Terms(PdbTermsAgg {
            field: field(0, name),
            size: 10,
            min_doc_count: 1,
            order: PdbTermsOrder {
                target: PdbOrderTarget::Count,
                asc: false,
            },
            missing: None,
            show_doc_count_error: true,
            sub_aggs,
        })
    }

    fn sum(name: &str) -> PdbAggNode {
        PdbAggNode::Metric(PdbMetricAgg {
            kind: PdbMetricKind::Sum,
            field: field(0, name),
            missing: None,
            none_if_no_match: false,
        })
    }

    fn request(root: PdbAggNode) -> PdbAggRequest {
        PdbAggRequest {
            root,
            visibility: MvccVisibility::default(),
            agg_json: serde_json::Value::Null,
        }
    }

    /// One SQL group key, one standard aggregate, and a two-level terms tree:
    /// group exprs are `[g, k0 (a), k1 (b)]`, levels are `{g}`, `{g, a}`,
    /// `{g, a, b}`.
    fn nested_plan() -> PdbAggPlan {
        let spec = request(terms(
            "a",
            vec![
                ("inner".into(), terms("b", vec![("s".into(), sum("v"))])),
                ("total".into(), sum("v")),
            ],
        ));
        PdbAggPlan::build(&[(1, &spec, false)], 1, 1)
    }

    #[test]
    fn grouping_ids_follow_datafusion_bit_order() {
        let plan = nested_plan();
        assert_eq!(plan.keys.len(), 2);
        assert_eq!(plan.levels, vec![vec![0], vec![0, 1], vec![0, 1, 2]]);
        // The leftmost expression owns the most significant bit and a set bit
        // marks an absent expression.
        assert_eq!(plan.root_grouping_id(), 0b011);
        assert_eq!(plan.grouping_id_for_level(1), 0b001);
        assert_eq!(plan.grouping_id_for_level(2), 0b000);
    }

    #[test]
    fn output_columns_follow_the_documented_order() {
        let plan = nested_plan();
        // [g, agg_0, __grouping_id, k0, k1, m...]
        assert_eq!(plan.grouping_id_col(), Some(2));
        assert_eq!(plan.key_col(0), 3);
        assert_eq!(plan.key_col(1), 4);
        assert_eq!(plan.metric_col(0), 5);

        let PdbAggNodeLayout::Terms { key_cols, sub, .. } = &plan.entries[0] else {
            panic!("root is a terms node");
        };
        assert_eq!(key_cols, &[0, 3]);
        let PdbAggNodeLayout::Terms { key_cols, .. } = &sub[0].1 else {
            panic!("inner is a terms node");
        };
        assert_eq!(key_cols, &[0, 3, 4]);
    }

    #[test]
    fn metrics_are_shared_across_nodes_but_not_across_filters() {
        let spec = request(terms("a", vec![("s".into(), sum("v"))]));
        let filtered = request(sum("v"));
        let plan = PdbAggPlan::build(&[(0, &spec, false), (1, &filtered, true)], 0, 0);
        // doc count, unfiltered sum(v), filtered sum(v)
        assert_eq!(plan.metrics.len(), 3);
        assert_eq!(plan.metrics[1].entry_filter, None);
        assert_eq!(plan.metrics[2].entry_filter, Some(1));
    }

    #[test]
    fn metrics_without_terms_need_no_grouping_sets() {
        let spec = request(sum("v"));
        let plan = PdbAggPlan::build(&[(0, &spec, false)], 1, 0);
        assert!(!plan.has_grouping_sets());
        assert_eq!(plan.grouping_id_col(), None);
        assert_eq!(plan.metric_col(0), 1);
    }

    // `metric_json` renders datetimes through Postgres, which the lib test binary
    // cannot link on Linux, so the value rule is tested on its own.
    #[test]
    fn empty_metrics_render_like_tantivy() {
        let value = |kind, none_if_no_match| metric_value(kind, None, none_if_no_match);
        assert_eq!(value(PdbMetricKind::Sum, false), Some(0.0));
        assert_eq!(value(PdbMetricKind::Sum, true), None);
        assert_eq!(value(PdbMetricKind::ValueCount, false), Some(0.0));
        assert_eq!(value(PdbMetricKind::Cardinality, false), Some(0.0));
        assert_eq!(value(PdbMetricKind::Avg, false), None);
        assert_eq!(
            metric_value(PdbMetricKind::Min, Some(3.0), false),
            Some(3.0)
        );
    }

    #[test]
    fn null_keys_sort_last() {
        let bucket = |key: KeyValue| Bucket {
            key,
            doc_count: 1,
            subs: Vec::new(),
        };
        let mut buckets = vec![
            bucket(KeyValue::Null),
            bucket(KeyValue::I64(2)),
            bucket(KeyValue::I64(1)),
        ];
        sort_buckets(
            &mut buckets,
            &PdbTermsOrder {
                target: PdbOrderTarget::Key,
                asc: true,
            },
        );
        let keys: Vec<_> = buckets.iter().map(|b| b.key.clone()).collect();
        assert_eq!(
            keys,
            vec![KeyValue::I64(1), KeyValue::I64(2), KeyValue::Null]
        );
    }

    #[test]
    fn bucket_ties_break_on_the_key() {
        let bucket = |key: &str, doc_count| Bucket {
            key: KeyValue::Str(key.into()),
            doc_count,
            subs: Vec::new(),
        };
        let mut buckets = vec![bucket("b", 2), bucket("c", 1), bucket("a", 2)];
        sort_buckets(
            &mut buckets,
            &PdbTermsOrder {
                target: PdbOrderTarget::Count,
                asc: false,
            },
        );
        let keys: Vec<_> = buckets.iter().map(|b| b.key.clone()).collect();
        assert_eq!(
            keys,
            vec![
                KeyValue::Str("a".into()),
                KeyValue::Str("b".into()),
                KeyValue::Str("c".into())
            ]
        );
    }
}
