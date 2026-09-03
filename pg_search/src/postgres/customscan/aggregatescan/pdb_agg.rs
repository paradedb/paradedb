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
//! no collectors, so the spec is lowered here: every `terms` level becomes one grouping
//! set of a single `Aggregate`, and each metric becomes the aggregate expressions that
//! feed Tantivy's intermediate result for it. After execution the flat grouped rows are
//! folded back into Tantivy's intermediate results, and Tantivy finalizes them: bucket
//! order, `size`, `min_doc_count`, `sum_other_doc_count`, and the result shape are its
//! own, so a query reads the same on either backend.

use crate::aggregate::{NULL_SENTINEL_MAX, scrub_missing_sentinel_value};
use crate::api::{HashMap, HashSet, MvccVisibility};
use crate::postgres::customscan::aggregatescan::json_rewrite::rewrite_aggregate_result_json_timestamps_with;
use crate::postgres::customscan::datafusion::cardinality_agg::decode_sketch;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::types::is_pgoid_datetime_type;
use crate::schema::SearchFieldType;
use arrow_array::cast::AsArray;
use arrow_array::{Array, ArrayRef, Int64Array, RecordBatch, UInt64Array, new_null_array};
use arrow_schema::{DataType, Schema, SchemaRef};
use datafusion::common::{DataFusionError, Result};
use decimal_bytes::Decimal;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tantivy::aggregation::AggregationLimitsGuard;
use tantivy::aggregation::Key;
use tantivy::aggregation::agg_req::{Aggregation, AggregationVariants, Aggregations};
use tantivy::aggregation::bucket::{CustomOrder, OrderTarget};
use tantivy::aggregation::intermediate_agg_result::{
    IntermediateAggregationResult, IntermediateAggregationResults, IntermediateBucketResult,
    IntermediateKey, IntermediateMetricResult, IntermediateTermBucketEntry,
    IntermediateTermBucketResult,
};
use tantivy::aggregation::metric::{
    CardinalityCollector, IntermediateAverage, IntermediateCount, IntermediateMax, IntermediateMin,
    IntermediateStats, IntermediateSum,
};
use tantivy::columnar::ColumnType;

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
        is_pgoid_datetime_type(self.field_type.typeoid())
    }

    /// The Tantivy column type of the field's fast field, which salts the
    /// cardinality sketch and picks the sentinel for a NULL bucket. A JSON
    /// sub-field only reaches this backend when it holds text.
    pub fn column_type(&self) -> ColumnType {
        match self.field_type {
            SearchFieldType::I64(_) | SearchFieldType::Date(_) | SearchFieldType::Numeric64(..) => {
                ColumnType::I64
            }
            SearchFieldType::U64(_) => ColumnType::U64,
            SearchFieldType::F64(_) => ColumnType::F64,
            SearchFieldType::Bool(_) => ColumnType::Bool,
            SearchFieldType::NumericBytes(..) => ColumnType::Bytes,
            _ => ColumnType::Str,
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
/// the sources.
pub trait PdbAggFieldResolver {
    fn resolve(&self, field: &str, usage: PdbAggFieldUsage) -> Result<PdbAggFieldRef, String>;
}

/// Type gate for a resolved field: the types DataFusion's grouping or metric
/// accumulators cannot take. NUMERIC passes; its storage is grouped as-is and
/// summed through the decimal accumulators the SQL aggregates use.
pub fn check_field_usage(
    field: &str,
    field_type: &SearchFieldType,
    usage: PdbAggFieldUsage,
) -> Result<(), String> {
    if matches!(field_type, SearchFieldType::Vector(..)) {
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
                | SearchFieldType::Numeric64(..)
                | SearchFieldType::NumericBytes(..)
        )
    {
        return Err(format!(
            "field '{field}' must be a numeric or date field for this metric aggregation"
        ));
    }
    Ok(())
}

/// The metric aggregations this backend runs, by the Tantivy intermediate each
/// one finalizes through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PdbMetricKind {
    Sum,
    Avg,
    Min,
    Max,
    ValueCount,
    Cardinality,
}

/// A DataFusion aggregate that feeds one part of a metric's intermediate: the
/// value count and one of the stats, or the cardinality sketch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PdbStat {
    Count,
    Sum,
    Min,
    Max,
    Cardinality,
}

impl PdbMetricKind {
    /// The stat beside the value count, when the metric needs one.
    fn value_stat(self) -> Option<PdbStat> {
        match self {
            PdbMetricKind::Sum | PdbMetricKind::Avg => Some(PdbStat::Sum),
            PdbMetricKind::Min => Some(PdbStat::Min),
            PdbMetricKind::Max => Some(PdbStat::Max),
            PdbMetricKind::ValueCount => None,
            PdbMetricKind::Cardinality => Some(PdbStat::Cardinality),
        }
    }

    fn counts_values(self) -> bool {
        self != PdbMetricKind::Cardinality
    }
}

/// A metric aggregation of the spec: its kind, field, and `missing` literal.
/// `None` for a bucket aggregation or one this backend does not run.
fn metric_of(agg: &AggregationVariants) -> Option<(PdbMetricKind, &str, Option<Key>)> {
    Some(match agg {
        AggregationVariants::Sum(m) => (
            PdbMetricKind::Sum,
            m.field.as_str(),
            m.missing.map(Key::F64),
        ),
        AggregationVariants::Average(m) => (
            PdbMetricKind::Avg,
            m.field.as_str(),
            m.missing.map(Key::F64),
        ),
        AggregationVariants::Min(m) => (
            PdbMetricKind::Min,
            m.field.as_str(),
            m.missing.map(Key::F64),
        ),
        AggregationVariants::Max(m) => (
            PdbMetricKind::Max,
            m.field.as_str(),
            m.missing.map(Key::F64),
        ),
        AggregationVariants::Count(m) => (
            PdbMetricKind::ValueCount,
            m.field.as_str(),
            m.missing.map(Key::F64),
        ),
        AggregationVariants::Cardinality(m) => (
            PdbMetricKind::Cardinality,
            m.field.as_str(),
            m.missing.clone(),
        ),
        _ => return None,
    })
}

/// One `pdb.agg()` call, checked for DataFusion. The spec stays in Tantivy's own
/// request form, which is what finalizes the result; `fields` maps every field
/// name in it, as written, to its join source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbAggRequest {
    pub agg: Aggregation,
    pub fields: HashMap<String, PdbAggFieldRef>,
    pub visibility: MvccVisibility,
    /// The spec as written, for EXPLAIN and for the result rewrites keyed on it.
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
        let mut fields = HashMap::default();
        check_node(&agg, resolver, &mut fields)?;
        Ok(Self {
            agg,
            fields,
            visibility,
            agg_json,
        })
    }

    /// True when the spec groups, which turns the plan into grouping sets.
    pub fn has_terms(&self) -> bool {
        matches!(self.agg.agg, AggregationVariants::Terms(_))
    }

    pub fn for_each_field(&self, f: impl FnMut(&PdbAggFieldRef)) {
        self.fields.values().for_each(f);
    }

    fn field(&self, name: &str) -> &PdbAggFieldRef {
        self.fields
            .get(name)
            .unwrap_or_else(|| panic!("BUG: pdb.agg field '{name}' was not resolved"))
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
fn check_missing(field: &PdbAggFieldRef, missing: Option<&Key>) -> Result<(), String> {
    let Some(missing) = missing else {
        return Ok(());
    };
    let name = &field.field_name;
    match (&field.field_type, missing) {
        (SearchFieldType::Bool(_), _) => Err(format!(
            "`missing` is not supported for boolean field '{name}'"
        )),
        // The literal would have to be encoded in the column's decimal storage.
        (SearchFieldType::Numeric64(..) | SearchFieldType::NumericBytes(..), _) => Err(format!(
            "`missing` is not supported for NUMERIC field '{name}'"
        )),
        (
            SearchFieldType::I64(_)
            | SearchFieldType::U64(_)
            | SearchFieldType::F64(_)
            | SearchFieldType::Date(_),
            Key::Str(_),
        ) => Err(format!(
            "`missing` for numeric field '{name}' must be a number"
        )),
        _ => Ok(()),
    }
}

fn resolve_field(
    resolver: &dyn PdbAggFieldResolver,
    fields: &mut HashMap<String, PdbAggFieldRef>,
    name: &str,
    usage: PdbAggFieldUsage,
) -> Result<PdbAggFieldRef, String> {
    let field = resolver.resolve(name, usage)?;
    fields.insert(name.to_string(), field.clone());
    Ok(field)
}

/// Resolve every field of the spec and turn down what this backend cannot run.
fn check_node(
    agg: &Aggregation,
    resolver: &dyn PdbAggFieldResolver,
    fields: &mut HashMap<String, PdbAggFieldRef>,
) -> Result<(), String> {
    match &agg.agg {
        AggregationVariants::Terms(terms) => {
            if terms.include.is_some() || terms.exclude.is_some() {
                return Err("terms `include` and `exclude` are not supported over joins".into());
            }
            // Tantivy answers `min_doc_count: 0` with every term of the column
            // dictionary. A grouped scan only sees the terms of matched rows.
            if terms.min_doc_count == Some(0) {
                return Err("terms `min_doc_count: 0` is not supported over joins".into());
            }
            let field = resolve_field(resolver, fields, &terms.field, PdbAggFieldUsage::TermsKey)?;
            check_missing(&field, terms.missing.as_ref())?;
            for sub in agg.sub_aggregation.values() {
                check_node(sub, resolver, fields)?;
            }
            if let Some(CustomOrder {
                target: OrderTarget::SubAggregation(name),
                ..
            }) = &terms.order
            {
                // Tantivy accepts `name.property` for multi-value metrics; only
                // single-value metrics run here, so the property is irrelevant.
                let agg_name = name.split_once('.').map(|(n, _)| n).unwrap_or(name);
                match agg.sub_aggregation.get(agg_name) {
                    Some(sub) if metric_of(&sub.agg).is_some() => {}
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
            }
            Ok(())
        }
        other => {
            let Some((kind, name, missing)) = metric_of(other) else {
                return Err(format!(
                    "`{}` aggregations are not supported over joins",
                    variant_name(other)
                ));
            };
            if !agg.sub_aggregation.is_empty() {
                return Err(format!(
                    "`{}` is a metric aggregation and cannot have sub-aggregations",
                    variant_name(other)
                ));
            }
            let usage = match kind {
                PdbMetricKind::Sum
                | PdbMetricKind::Avg
                | PdbMetricKind::Min
                | PdbMetricKind::Max => PdbAggFieldUsage::NumericMetric,
                PdbMetricKind::ValueCount | PdbMetricKind::Cardinality => {
                    PdbAggFieldUsage::AnyMetric
                }
            };
            let field = resolve_field(resolver, fields, name, usage)?;
            check_missing(&field, missing.as_ref())
        }
    }
}

/// A grouping key interned across every `terms` node of a query.
#[derive(Debug, Clone)]
pub struct PdbKeySpec {
    pub field: PdbAggFieldRef,
    pub missing: Option<Key>,
}

/// An aggregate expression interned across every metric node of a query. `stat`
/// is `None` for the bucket doc count.
#[derive(Debug, Clone)]
pub struct PdbMetricSpec {
    pub stat: Option<PdbStat>,
    pub field: Option<PdbAggFieldRef>,
    pub missing: Option<Key>,
    /// Target-list index of the `pdb.agg()` entry whose `FILTER` applies, if any.
    pub entry_filter: Option<usize>,
}

/// Column bindings of one spec node into the DataFusion output.
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
        sub: Vec<(String, PdbAggNodeLayout)>,
    },
    Metric {
        kind: PdbMetricKind,
        field: PdbAggFieldRef,
        /// The value count; every metric but `cardinality` has one.
        count_col: Option<usize>,
        /// The metric's own stat: sum, min, max, or the cardinality sketch.
        value_col: Option<usize>,
    },
}

/// One lowered `pdb.agg()` entry: its layout plus the request and what the
/// result rewrites need.
#[derive(Debug, Clone)]
pub struct PdbAggEntryLayout {
    pub root: PdbAggNodeLayout,
    agg: Aggregation,
    agg_json: serde_json::Value,
    datetime_fields: HashSet<String>,
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
    pub entries: Vec<PdbAggEntryLayout>,
    num_outer_group_cols: usize,
    num_std_aggs: usize,
    num_terms_nodes: usize,
}

/// Identity of an interned metric: stat, `(plan_position, field_name)`, the
/// `missing` literal, and the entry whose `FILTER` applies.
type MetricId = (
    Option<PdbStat>,
    Option<(usize, String)>,
    Option<Key>,
    Option<usize>,
);

struct PlanBuilder {
    plan: PdbAggPlan,
    key_ids: HashMap<(usize, String, Option<Key>), usize>,
    metric_ids: HashMap<MetricId, usize>,
    level_ids: HashMap<Vec<usize>, usize>,
}

impl PlanBuilder {
    fn intern_key(&mut self, field: &PdbAggFieldRef, missing: &Option<Key>) -> usize {
        let id = (
            field.plan_position,
            field.field_name.clone(),
            missing.clone(),
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
        stat: Option<PdbStat>,
        field: Option<&PdbAggFieldRef>,
        missing: &Option<Key>,
        entry_filter: Option<usize>,
    ) -> usize {
        let id = (
            stat,
            field.map(|f| (f.plan_position, f.field_name.clone())),
            missing.clone(),
            entry_filter,
        );
        if let Some(&idx) = self.metric_ids.get(&id) {
            return idx;
        }
        let idx = self.plan.metrics.len();
        self.plan.metrics.push(PdbMetricSpec {
            stat,
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
        agg: &Aggregation,
        request: &PdbAggRequest,
        parent_positions: &[usize],
        entry_filter: Option<usize>,
    ) -> PdbAggNodeLayout {
        match &agg.agg {
            AggregationVariants::Terms(terms) => {
                let field = request.field(&terms.field);
                let key_idx = self.intern_key(field, &terms.missing);
                let mut positions = parent_positions.to_vec();
                positions.push(self.plan.num_outer_group_cols + key_idx);
                let level = self.intern_level(&positions);
                let doc_count_idx = self.intern_metric(None, None, &None, entry_filter);
                let node_id = self.plan.num_terms_nodes;
                self.plan.num_terms_nodes += 1;
                // The request map is unordered; sort so column naming is stable
                // across plans.
                let mut names: Vec<&String> = agg.sub_aggregation.keys().collect();
                names.sort();
                let sub = names
                    .into_iter()
                    .map(|name| {
                        let sub = &agg.sub_aggregation[name];
                        (
                            name.clone(),
                            self.lower(sub, request, &positions, entry_filter),
                        )
                    })
                    .collect();
                PdbAggNodeLayout::Terms {
                    node_id,
                    level,
                    key_cols: positions,
                    doc_count_col: doc_count_idx,
                    field: field.clone(),
                    sub,
                }
            }
            other => {
                let (kind, name, missing) = metric_of(other).expect("checked by lowering");
                let field = request.field(name);
                let count_col = kind.counts_values().then(|| {
                    self.intern_metric(Some(PdbStat::Count), Some(field), &missing, entry_filter)
                });
                let value_col = kind.value_stat().map(|stat| {
                    self.intern_metric(Some(stat), Some(field), &missing, entry_filter)
                });
                PdbAggNodeLayout::Metric {
                    kind,
                    field: field.clone(),
                    count_col,
                    value_col,
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
            let root = builder.lower(
                &request.agg,
                request,
                &root_positions,
                has_filter.then_some(agg_idx),
            );
            builder.plan.entries.push(PdbAggEntryLayout {
                root,
                agg: request.agg.clone(),
                agg_json: request.agg_json.clone(),
                datetime_fields: request
                    .fields
                    .iter()
                    .filter(|(_, field)| field.is_datetime())
                    .map(|(name, _)| name.clone())
                    .collect(),
            });
        }
        // Key and metric indices were interned relative to their own lists; rebase
        // them onto the final output columns now that every list is complete.
        let mut plan = builder.plan;
        let entries = std::mem::take(&mut plan.entries);
        plan.entries = entries
            .into_iter()
            .map(|entry| PdbAggEntryLayout {
                root: plan.rebase(entry.root),
                ..entry
            })
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
                sub,
            } => PdbAggNodeLayout::Terms {
                node_id,
                level,
                key_cols: key_cols.into_iter().map(|p| self.group_col(p)).collect(),
                doc_count_col: self.metric_col(doc_count_col),
                field,
                sub: sub
                    .into_iter()
                    .map(|(name, sub)| (name, self.rebase(sub)))
                    .collect(),
            },
            PdbAggNodeLayout::Metric {
                kind,
                field,
                count_col,
                value_col,
            } => PdbAggNodeLayout::Metric {
                kind,
                field,
                count_col: count_col.map(|col| self.metric_col(col)),
                value_col: value_col.map(|col| self.metric_col(col)),
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

/// A bucket key, SQL group value, or metric read back from Arrow.
fn value_at(col: &ArrayRef, row: usize) -> PdbOwnedValue {
    PdbOwnedValue::from_arrow(col.as_ref(), row)
        .unwrap_or_else(|| panic!("BUG: unsupported pdb.agg value type {}", col.data_type()))
}

/// The value a NUMERIC key or metric stands for, as a number. Other values pass
/// through.
fn decode_numeric(value: PdbOwnedValue, field: &PdbAggFieldRef) -> PdbOwnedValue {
    match (&field.field_type, &value) {
        (SearchFieldType::Numeric64(_, scale), PdbOwnedValue::I64(v)) => {
            PdbOwnedValue::F64(*v as f64 / 10f64.powi(*scale as i32))
        }
        (SearchFieldType::NumericBytes(..), PdbOwnedValue::Bytes(bytes)) => {
            decimal_bytes_to_f64(bytes)
                .map(PdbOwnedValue::F64)
                .unwrap_or(PdbOwnedValue::Null)
        }
        _ => value,
    }
}

/// A value as the number Tantivy collects for it: a timestamp as its
/// microseconds, a boolean as 0 or 1.
fn value_f64(value: &PdbOwnedValue) -> Option<f64> {
    match value {
        PdbOwnedValue::I64(v) => Some(*v as f64),
        PdbOwnedValue::U64(v) => Some(*v as f64),
        PdbOwnedValue::F64(v) => Some(*v),
        PdbOwnedValue::Bool(b) => Some(f64::from(u8::from(*b))),
        PdbOwnedValue::Date(d) => Some(d.into_inner() as f64),
        _ => None,
    }
}

/// Tantivy's key for a bucket. A NULL bucket takes the sentinel the Tantivy
/// backend puts on `missing` for the column type, scrubbed to `null` after
/// serialization. The sentinel has to share the variant of the real keys, since
/// Tantivy orders keys of different variants by variant; a NUMERIC key decodes
/// to `F64`. A datetime keeps its microseconds for the timestamp rewrite.
fn intermediate_key(key: PdbOwnedValue, field: &PdbAggFieldRef) -> IntermediateKey {
    match decode_numeric(key, field) {
        PdbOwnedValue::Str(s) => IntermediateKey::Str(s),
        PdbOwnedValue::I64(v) => IntermediateKey::I64(v),
        PdbOwnedValue::U64(v) => IntermediateKey::U64(v),
        PdbOwnedValue::F64(v) => IntermediateKey::F64(v),
        PdbOwnedValue::Bool(b) => IntermediateKey::Bool(b),
        PdbOwnedValue::Date(d) => IntermediateKey::I64(d.into_inner()),
        _ if field.field_type.is_numeric() => IntermediateKey::F64(f64::MAX),
        _ => match field.column_type() {
            ColumnType::I64 => IntermediateKey::I64(i64::MAX),
            ColumnType::U64 => IntermediateKey::U64(u64::MAX),
            ColumnType::F64 => IntermediateKey::F64(f64::MAX),
            _ => IntermediateKey::Str(NULL_SENTINEL_MAX.to_string()),
        },
    }
}

fn bytes_at(col: &ArrayRef, row: usize) -> &[u8] {
    match col.data_type() {
        DataType::Binary => col.as_binary::<i32>().value(row),
        DataType::LargeBinary => col.as_binary::<i64>().value(row),
        DataType::BinaryView => col.as_binary_view().value(row),
        other => panic!("BUG: expected a binary column, found {other}"),
    }
}

fn decimal_bytes_to_f64(bytes: &[u8]) -> Option<f64> {
    Decimal::from_bytes(bytes).ok()?.to_string().parse().ok()
}

/// A stat value as the number Tantivy collects. A NUMERIC sum arrives as a
/// decimal-bytes blob from the decimal accumulator, a NUMERIC minimum or
/// maximum as the column's own storage.
fn stat_f64(col: &ArrayRef, row: usize, field: &PdbAggFieldRef) -> Option<f64> {
    if col.is_null(row) {
        return None;
    }
    if field.field_type.is_numeric() && matches!(col.data_type(), DataType::Binary) {
        return decimal_bytes_to_f64(bytes_at(col, row));
    }
    value_f64(&decode_numeric(value_at(col, row), field))
}

fn u64_at(col: &ArrayRef, row: usize) -> u64 {
    match value_at(col, row) {
        PdbOwnedValue::I64(v) => v.max(0) as u64,
        PdbOwnedValue::U64(v) => v,
        PdbOwnedValue::F64(v) => v as u64,
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

/// Rows of one terms node grouped by their parent bucket.
type NodeIndex = HashMap<Vec<PdbOwnedValue>, Vec<(PdbOwnedValue, usize)>>;

struct Assembler<'a> {
    batch: &'a RecordBatch,
    indexes: Vec<NodeIndex>,
}

/// The name the single aggregation of an entry gets in Tantivy's request and
/// result maps.
const ENTRY_NAME: &str = "agg";

impl Assembler<'_> {
    /// Fold the rows under `prefix` into Tantivy's intermediate result for `node`.
    /// `row` is the row of the parent bucket, or of the SQL group for a root
    /// node; `None` for a synthesized empty root.
    fn intermediate(
        &self,
        node: &PdbAggNodeLayout,
        prefix: &[PdbOwnedValue],
        row: Option<usize>,
    ) -> Result<IntermediateAggregationResult> {
        match node {
            PdbAggNodeLayout::Metric {
                kind,
                field,
                count_col,
                value_col,
            } => {
                let count = match (count_col, row) {
                    (Some(col), Some(row)) => u64_at(self.batch.column(*col), row),
                    _ => 0,
                };
                let value = match (value_col, row) {
                    (Some(col), Some(row)) => stat_f64(self.batch.column(*col), row, field),
                    _ => None,
                };
                let stats = |sum: f64, min: f64, max: f64| {
                    IntermediateStats::from_parts(count, sum, min, max)
                };
                let metric =
                    match kind {
                        PdbMetricKind::Sum => {
                            IntermediateMetricResult::Sum(IntermediateSum::from_stats(stats(
                                value.unwrap_or(0.0),
                                f64::MAX,
                                f64::MIN,
                            )))
                        }
                        PdbMetricKind::Avg => {
                            IntermediateMetricResult::Average(IntermediateAverage::from_stats(
                                stats(value.unwrap_or(0.0), f64::MAX, f64::MIN),
                            ))
                        }
                        PdbMetricKind::Min => {
                            IntermediateMetricResult::Min(IntermediateMin::from_stats(stats(
                                0.0,
                                value.unwrap_or(f64::MAX),
                                f64::MIN,
                            )))
                        }
                        PdbMetricKind::Max => {
                            IntermediateMetricResult::Max(IntermediateMax::from_stats(stats(
                                0.0,
                                f64::MAX,
                                value.unwrap_or(f64::MIN),
                            )))
                        }
                        PdbMetricKind::ValueCount => IntermediateMetricResult::Count(
                            IntermediateCount::from_stats(stats(0.0, f64::MAX, f64::MIN)),
                        ),
                        PdbMetricKind::Cardinality => {
                            let collector = match (value_col, row) {
                                (Some(col), Some(row)) if !self.batch.column(*col).is_null(row) => {
                                    decode_sketch(bytes_at(self.batch.column(*col), row))?
                                }
                                _ => CardinalityCollector::default(),
                            };
                            IntermediateMetricResult::Cardinality(collector)
                        }
                    };
                Ok(IntermediateAggregationResult::Metric(metric))
            }
            PdbAggNodeLayout::Terms {
                node_id,
                key_cols,
                doc_count_col,
                field,
                sub,
                ..
            } => {
                // Rows without the field form a NULL bucket, the way the Tantivy
                // backend's `missing` sentinel gives them one.
                let children = self.indexes[*node_id].get(prefix);
                let mut entries = HashMap::default();
                for (key, child_row) in children.into_iter().flatten() {
                    // Children are keyed on the stored value; the bucket carries
                    // the value it stands for.
                    let mut child_prefix = Vec::with_capacity(key_cols.len());
                    child_prefix.extend_from_slice(prefix);
                    child_prefix.push(key.clone());
                    let mut sub_aggregation = IntermediateAggregationResults::default();
                    for (name, node) in sub {
                        sub_aggregation
                            .push(
                                name.clone(),
                                self.intermediate(node, &child_prefix, Some(*child_row))?,
                            )
                            .map_err(|e| DataFusionError::External(Box::new(e)))?;
                    }
                    entries.insert(
                        intermediate_key(key.clone(), field),
                        IntermediateTermBucketEntry {
                            doc_count: u64_at(self.batch.column(*doc_count_col), *child_row),
                            sub_aggregation,
                        },
                    );
                }
                // Every bucket is exact, so the error bound is zero.
                Ok(IntermediateAggregationResult::Bucket(
                    IntermediateBucketResult::Terms {
                        buckets: IntermediateTermBucketResult::new(entries, 0, 0),
                    },
                ))
            }
        }
    }

    /// The `pdb.agg()` document of one entry for one SQL group: Tantivy finalizes
    /// the intermediate result, then the rewrites the Tantivy backend applies to
    /// its own output run over the JSON.
    fn render(
        &self,
        entry: &PdbAggEntryLayout,
        prefix: &[PdbOwnedValue],
        row: Option<usize>,
    ) -> Result<serde_json::Value> {
        let mut intermediate = IntermediateAggregationResults::default();
        intermediate
            .push(
                ENTRY_NAME.to_string(),
                self.intermediate(&entry.root, prefix, row)?,
            )
            .map_err(|e| DataFusionError::External(Box::new(e)))?;
        let request: Aggregations = [(ENTRY_NAME.to_string(), entry.agg.clone())]
            .into_iter()
            .collect();
        // This backend has no bucket cap; the leader materialized every group.
        let limits = AggregationLimitsGuard::new(None, Some(u32::MAX));
        let results = intermediate
            .into_final_result(request, limits)
            .map_err(|e| DataFusionError::External(Box::new(e)))?;
        let mut json =
            serde_json::to_value(results).map_err(|e| DataFusionError::External(Box::new(e)))?;
        let mut json = json
            .get_mut(ENTRY_NAME)
            .map(serde_json::Value::take)
            .unwrap_or(serde_json::Value::Null);
        // The datetime sentinel is a valid timestamp, so it has to go before the
        // rewrite would turn it into one.
        scrub_missing_sentinel_value(&mut json);
        rewrite_aggregate_result_json_timestamps_with(&mut json, &entry.agg_json, &|name| {
            entry.datetime_fields.contains(name)
        });
        Ok(json)
    }
}

fn grouping_ids(col: &ArrayRef) -> Vec<u64> {
    (0..col.len())
        .map(|row| match value_at(col, row) {
            PdbOwnedValue::U64(v) => v,
            PdbOwnedValue::I64(v) => v as u64,
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
        collect_terms_nodes(&entry.root, &mut terms_nodes);
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
            let prefix: Vec<PdbOwnedValue> = prefix_cols
                .iter()
                .map(|&col| value_at(batch.column(col), row))
                .collect();
            let key = value_at(batch.column(own_col[0]), row);
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
                .map(|entry| assembler.render(entry, &[], None))
                .collect::<Result<Vec<_>>>()?,
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
            let prefix: Vec<PdbOwnedValue> = (0..plan.num_outer_group_cols)
                .map(|col| value_at(batch.column(col), row))
                .collect();
            plan.entries
                .iter()
                .map(|entry| assembler.render(entry, &prefix, Some(row)))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;

    let indices = UInt64Array::from(root_rows.iter().map(|&r| r as u64).collect::<Vec<_>>());
    let root_batch =
        arrow_select::take::take_record_batch(&batch, &indices)?.project(&root_cols)?;
    Ok(AssembledPdbAggRows { root_batch, json })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn field(name: &str) -> PdbAggFieldRef {
        PdbAggFieldRef {
            plan_position: 0,
            attno: 1,
            field_name: name.to_string(),
            field_type: SearchFieldType::I64(pg_sys::INT8OID),
        }
    }

    fn request(spec: serde_json::Value, fields: &[&str]) -> PdbAggRequest {
        PdbAggRequest {
            agg: serde_json::from_value(spec.clone()).expect("a valid spec"),
            fields: fields
                .iter()
                .map(|name| (name.to_string(), field(name)))
                .collect(),
            visibility: MvccVisibility::default(),
            agg_json: spec,
        }
    }

    /// One SQL group key, one standard aggregate, and a two-level terms tree:
    /// group exprs are `[g, k0 (a), k1 (b)]`, levels are `{g}`, `{g, a}`,
    /// `{g, a, b}`.
    fn nested_plan() -> PdbAggPlan {
        let spec = request(
            json!({
                "terms": {"field": "a"},
                "aggs": {
                    "inner": {"terms": {"field": "b"}, "aggs": {"s": {"sum": {"field": "v"}}}},
                    "total": {"sum": {"field": "v"}}
                }
            }),
            &["a", "b", "v"],
        );
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

        let PdbAggNodeLayout::Terms { key_cols, sub, .. } = &plan.entries[0].root else {
            panic!("root is a terms node");
        };
        assert_eq!(key_cols, &[0, 3]);
        let PdbAggNodeLayout::Terms { key_cols, .. } = &sub[0].1 else {
            panic!("inner is a terms node");
        };
        assert_eq!(key_cols, &[0, 3, 4]);
    }

    #[test]
    fn stats_are_shared_across_nodes_but_not_across_filters() {
        let spec = request(
            json!({
                "terms": {"field": "a"},
                "aggs": {"s": {"sum": {"field": "v"}}, "m": {"avg": {"field": "v"}}}
            }),
            &["a", "v"],
        );
        let filtered = request(json!({"sum": {"field": "v"}}), &["v"]);
        let plan = PdbAggPlan::build(&[(0, &spec, false), (1, &filtered, true)], 0, 0);
        // The doc count, then count(v) and sum(v) shared by sum and avg, then the
        // filtered count(v) and sum(v).
        let stats: Vec<_> = plan
            .metrics
            .iter()
            .map(|m| (m.stat, m.entry_filter))
            .collect();
        assert_eq!(
            stats,
            vec![
                (None, None),
                (Some(PdbStat::Count), None),
                (Some(PdbStat::Sum), None),
                (Some(PdbStat::Count), Some(1)),
                (Some(PdbStat::Sum), Some(1)),
            ]
        );
    }

    #[test]
    fn metrics_without_terms_need_no_grouping_sets() {
        let spec = request(json!({"sum": {"field": "v"}}), &["v"]);
        let plan = PdbAggPlan::build(&[(0, &spec, false)], 1, 0);
        assert!(!plan.has_grouping_sets());
        assert_eq!(plan.grouping_id_col(), None);
        assert_eq!(plan.metric_col(0), 1);
    }

    #[test]
    fn null_keys_take_the_column_sentinel() {
        let text = PdbAggFieldRef {
            field_type: SearchFieldType::Text(pg_sys::TEXTOID),
            ..field("t")
        };
        assert_eq!(
            intermediate_key(PdbOwnedValue::Null, &text),
            IntermediateKey::Str(NULL_SENTINEL_MAX.to_string())
        );
        assert_eq!(
            intermediate_key(PdbOwnedValue::Null, &field("n")),
            IntermediateKey::I64(i64::MAX)
        );
        let numeric = PdbAggFieldRef {
            field_type: SearchFieldType::Numeric64(pg_sys::NUMERICOID, 2),
            ..field("p")
        };
        assert_eq!(
            intermediate_key(PdbOwnedValue::Null, &numeric),
            IntermediateKey::F64(f64::MAX)
        );
        assert_eq!(
            intermediate_key(PdbOwnedValue::I64(149999), &numeric),
            IntermediateKey::F64(1499.99)
        );
        assert_eq!(
            intermediate_key(PdbOwnedValue::Bool(true), &field("b")),
            IntermediateKey::Bool(true)
        );
    }
}
