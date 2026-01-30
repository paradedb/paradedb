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

use crate::gucs;

use crate::aggregate::{execute_aggregate, AggregateRequest};
use crate::api::HashMap;
use crate::customscan::aggregatescan::build::{
    AggregationKey, DocCountKey, FilterSentinelKey, GroupedKey,
};
use crate::postgres::customscan::aggregatescan::{AggregateScan, AggregateType};
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::types::{is_datetime_type, TantivyValue};
use pgrx::{pg_sys, IntoDatum, JsonB};

use tantivy::aggregation::agg_result::{
    AggregationResult as TantivyAggregationResult, AggregationResults as TantivyAggregationResults,
    BucketResult, MetricResult as TantivyMetricResult,
};
use tantivy::aggregation::metric::SingleMetricResult as TantivySingleMetricResult;
use tantivy::aggregation::Key;
use tantivy::schema::OwnedValue;

/// Unified result type for aggregates
/// Can hold either a standard metric (f64) or a custom aggregate (JSON)
#[derive(Debug, Clone)]
pub enum AggregateResult {
    /// Standard metric aggregates (COUNT, SUM, AVG, MIN, MAX)
    Metric(TantivySingleMetricResult),
    /// Custom aggregates (full JSON result)
    Json(serde_json::Value),
}

pub fn aggregation_results_iter(
    state: &mut CustomScanStateWrapper<AggregateScan>,
) -> std::vec::IntoIter<AggregationResultsRow> {
    state
        .custom_state_mut()
        .aggregate_clause
        .set_is_execution_time();

    let planstate = state.planstate();
    let expr_context = state.runtime_context;

    state
        .custom_state_mut()
        .prepare_query_for_execution(planstate, expr_context);

    let aggregate_clause = state.custom_state().aggregate_clause.clone();
    let query = aggregate_clause.query().clone();

    // Use the GUC for term aggregation bucket limits (single source of truth).
    let bucket_limit: u32 = gucs::max_term_agg_buckets() as u32;

    let result: AggregationResults = execute_aggregate(
        state.custom_state().indexrel(),
        query,
        AggregateRequest::Sql(aggregate_clause),
        true,
        gucs::adjust_work_mem().get().try_into().unwrap(),
        bucket_limit,
        expr_context,
        planstate,
    )
    .unwrap_or_else(|e| pgrx::error!("Failed to execute filter aggregation: {}", e))
    .into();

    if result.is_empty() {
        if state.custom_state().aggregate_clause.has_groupby() {
            vec![].into_iter()
        } else {
            vec![AggregationResultsRow::default()].into_iter()
        }
    } else {
        result.into_iter()
    }
}

#[derive(Debug)]
pub struct AggregationResults(HashMap<String, TantivyAggregationResult>);
impl From<TantivyAggregationResults> for AggregationResults {
    fn from(results: TantivyAggregationResults) -> Self {
        Self(results.0)
    }
}

impl IntoIterator for AggregationResults {
    type Item = AggregationResultsRow;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let mut result = Vec::new();
        match self.style() {
            AggregationStyle::Ungrouped => self.flatten_ungrouped(&mut result, None),
            AggregationStyle::Grouped => self.flatten_grouped(&mut result),
            AggregationStyle::GroupedWithFilter => self.flatten_grouped_with_filter(&mut result),
        }

        result.into_iter()
    }
}

#[derive(Debug, Default)]
pub struct AggregationResultsRow {
    pub group_keys: Vec<TantivyValue>,
    pub aggregates: Vec<Option<AggregateResult>>,
    doc_count: Option<u64>,
}

impl AggregationResultsRow {
    pub fn doc_count(&self) -> TantivyValue {
        match self.doc_count {
            Some(doc_count) => TantivyValue(OwnedValue::U64(doc_count)),
            None => TantivyValue(OwnedValue::Null),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.group_keys.is_empty() && self.aggregates.is_empty() && self.doc_count.is_none()
    }
}

enum AggregationStyle {
    Ungrouped,
    Grouped,
    GroupedWithFilter,
}

struct MetricResult(TantivyMetricResult);
/// Convert a Tantivy MetricResult to an AggregateResult
///
/// Simple metrics (Average, Count, Sum, Min, Max) are converted to single metric results.
/// Complex metrics (Stats, Percentiles, etc.) are serialized to JSON.
impl From<MetricResult> for AggregateResult {
    fn from(val: MetricResult) -> Self {
        match val.0 {
            TantivyMetricResult::Average(r)
            | TantivyMetricResult::Count(r)
            | TantivyMetricResult::Sum(r)
            | TantivyMetricResult::Min(r)
            | TantivyMetricResult::Max(r) => AggregateResult::Metric(r),
            // Complex metrics (Stats, Percentiles, etc.) - serialize to JSON
            other => {
                let json_value = serde_json::to_value(&other)
                    .unwrap_or_else(|e| pgrx::error!("Failed to serialize metric: {}", e));
                AggregateResult::Json(json_value)
            }
        }
    }
}

/// Convert an AggregateResult to a PostgreSQL Datum
/// This is the shared logic for converting both Custom (JSON) and Metric aggregates
///
/// # Arguments
/// * `agg_result` - The aggregate result to convert (Metric or Json)
/// * `agg_type` - The aggregate type (for nullish fallback and numeric_scale)
/// * `expected_typoid` - The expected PostgreSQL type OID from the tuple descriptor
pub fn aggregate_result_to_datum(
    agg_result: Option<AggregateResult>,
    agg_type: &AggregateType,
    expected_typoid: pg_sys::Oid,
) -> Option<pg_sys::Datum> {
    match agg_result {
        Some(AggregateResult::Json(json_value)) => JsonB(json_value).into_datum(),
        Some(AggregateResult::Metric(metric)) => {
            if expected_typoid == pg_sys::JSONBOID {
                // JSONB output - serialize metric to JSON
                let json_value = serde_json::to_value(&metric).unwrap_or_else(|e| {
                    pgrx::error!("Failed to serialize metric result to JSON: {}", e)
                });
                JsonB(json_value).into_datum()
            } else if is_datetime_type(expected_typoid) {
                // For date/time types, Tantivy stores DateTime values in fast fields as nanoseconds
                // since UNIX epoch. The f64 value from MIN/MAX aggregates represents this nanosecond
                // timestamp. We need to convert it back to a DateTime before converting to the
                // expected PostgreSQL type.
                metric.value.and_then(|value| unsafe {
                    let datetime = tantivy::DateTime::from_timestamp_nanos(value as i64);
                    TantivyValue(OwnedValue::Date(datetime))
                        .try_into_datum(expected_typoid.into())
                        .unwrap()
                })
            } else {
                // Scalar output - convert to expected type
                metric.value.and_then(|value| unsafe {
                    TantivyValue(OwnedValue::F64(value))
                        .try_into_datum(expected_typoid.into())
                        .unwrap()
                })
            }
        }
        None => {
            // No result - use nullish value
            // If expected type is JSONB, return JSON null
            if expected_typoid == pg_sys::JSONBOID {
                JsonB(serde_json::Value::Null).into_datum()
            } else {
                agg_type.nullish().value.and_then(|value| unsafe {
                    TantivyValue(OwnedValue::F64(value))
                        .try_into_datum(expected_typoid.into())
                        .unwrap()
                })
            }
        }
    }
}

impl AggregationResults {
    pub fn is_empty(&self) -> bool {
        // we should return an empty result set if either the Aggregations JSON is empty,
        // or if the top-level `_doc_count` is `0.0` (i.e. zero documents were matched)
        if self.0.is_empty() {
            return true;
        }

        let _doc_count = self
            .0
            .get(DocCountKey::NAME)
            .and_then(|result| match result {
                TantivyAggregationResult::MetricResult(TantivyMetricResult::Count(
                    TantivySingleMetricResult { value, .. },
                )) => *value,
                _ => None,
            });

        if let Some(_doc_count) = _doc_count {
            if _doc_count == 0.0 {
                return true;
            }
        }

        false
    }

    fn style(&self) -> AggregationStyle {
        if self.0.contains_key(FilterSentinelKey::NAME) {
            AggregationStyle::GroupedWithFilter
        } else if self.0.contains_key(GroupedKey::NAME) {
            AggregationStyle::Grouped
        } else {
            AggregationStyle::Ungrouped
        }
    }

    fn collect_group_keys(
        &self,
        key_accumulator: Vec<TantivyValue>,
        out: &mut Vec<AggregationResultsRow>,
    ) {
        // look only at the "grouped" terms bucket at this level
        if let Some(TantivyAggregationResult::BucketResult(BucketResult::Terms {
            buckets, ..
        })) = self.0.get(GroupedKey::NAME)
        {
            for bucket_entry in buckets {
                // extend the key path with this bucket's key
                let mut new_keys = key_accumulator.clone();
                let key_val = match &bucket_entry.key {
                    Key::Str(s) => TantivyValue(OwnedValue::Str(s.clone())),
                    Key::I64(i) => TantivyValue(OwnedValue::I64(*i)),
                    Key::U64(u) => TantivyValue(OwnedValue::U64(*u)),
                    Key::F64(f) => TantivyValue(OwnedValue::F64(*f)),
                };
                new_keys.push(key_val);

                // check if this bucket has a child "grouped" terms bucket
                let sub = AggregationResults(bucket_entry.sub_aggregation.0.clone());
                let has_child_grouped = match sub.0.get(GroupedKey::NAME) {
                    Some(TantivyAggregationResult::BucketResult(BucketResult::Terms {
                        buckets,
                        ..
                    })) => !buckets.is_empty(),
                    _ => false,
                };

                if has_child_grouped {
                    // not a leaf yet; keep descending
                    sub.collect_group_keys(new_keys, out);
                } else {
                    // leaf: emit ONLY the deepest group path
                    out.push(AggregationResultsRow {
                        group_keys: new_keys,
                        aggregates: Vec::new(),
                        doc_count: Some(bucket_entry.doc_count),
                    });
                }
            }
        }
    }

    fn flatten_grouped(self, out: &mut Vec<AggregationResultsRow>) {
        if self.is_empty() {
            return;
        }

        // collect all group key paths first
        self.collect_group_keys(Vec::new(), out);

        // for each row, chase down aggregate values matching its group keys
        for row in out.iter_mut() {
            let mut current = self.0.clone();

            // traverse down into nested "grouped" buckets following group_keys
            for key in &row.group_keys {
                if let Some(TantivyAggregationResult::BucketResult(BucketResult::Terms {
                    buckets,
                    ..
                })) = current.get(GroupedKey::NAME)
                {
                    // find the bucket whose key matches this level
                    let maybe_bucket = buckets.iter().find(|b| match (&b.key, &key.0) {
                        (Key::Str(s), OwnedValue::Str(v)) => s == v,
                        (Key::I64(i), OwnedValue::I64(v)) => i == v,
                        (Key::U64(i), OwnedValue::U64(v)) => i == v,
                        (Key::F64(i), OwnedValue::F64(v)) => i == v,
                        _ => false,
                    });

                    if let Some(bucket) = maybe_bucket {
                        // descend into this bucket’s sub-aggregations
                        current = bucket.sub_aggregation.0.clone();
                    } else {
                        // no matching bucket found — bail out early
                        current.clear();
                        break;
                    }
                }
            }

            // collect any metric results at this nested level
            let mut entries: Vec<_> = current.into_iter().collect();
            entries.sort_by_key(|(k, _)| k.parse::<usize>().unwrap_or(usize::MAX));

            for (_name, result) in entries {
                match result {
                    TantivyAggregationResult::MetricResult(metric) => {
                        row.aggregates.push(Some(MetricResult(metric).into()));
                    }
                    other => {
                        let json_value = serde_json::to_value(&other).unwrap_or_else(|e| {
                            pgrx::error!("Failed to serialize aggregate: {}", e)
                        });
                        row.aggregates.push(Some(AggregateResult::Json(json_value)));
                    }
                }
            }
        }
    }

    pub fn flatten_ungrouped(
        self,
        out: &mut Vec<AggregationResultsRow>,
        agg_types: Option<&[AggregateType]>,
    ) {
        if self.is_empty() {
            return;
        }

        let mut aggregates = Vec::new();
        let mut entries: Vec<_> = self.0.into_iter().collect();
        entries.sort_by_key(|(k, _)| k.parse::<usize>().unwrap_or(usize::MAX));

        for (idx, (_name, result)) in entries.into_iter().enumerate() {
            // Check if this is a Custom aggregate
            let is_custom = agg_types
                .and_then(|types| types.get(idx))
                .map(|agg| matches!(agg, AggregateType::Custom { .. }))
                .unwrap_or(false);

            match result {
                TantivyAggregationResult::MetricResult(metric) if !is_custom => {
                    // Standard metric aggregate
                    aggregates.push(Some(MetricResult(metric).into()));
                }
                TantivyAggregationResult::BucketResult(BucketResult::Filter(filter_bucket))
                    if !is_custom =>
                {
                    // Standard filter aggregate (not custom)
                    let mut sub_rows = Vec::new();
                    let sub = AggregationResults(filter_bucket.sub_aggregations.0);
                    sub.flatten_ungrouped(&mut sub_rows, agg_types);
                    for sub_row in sub_rows {
                        aggregates.extend(sub_row.aggregates);
                    }
                }
                // For all other results (custom aggregates and other bucket types), serialize as JSON
                // For custom aggregates (pdb.agg), this preserves all nested aggregations
                other => {
                    let json_value = serde_json::to_value(&other)
                        .unwrap_or_else(|e| pgrx::error!("Failed to serialize aggregate: {}", e));
                    aggregates.push(Some(AggregateResult::Json(json_value)));
                }
            }
        }

        out.push(AggregationResultsRow {
            group_keys: Vec::new(),
            aggregates,
            doc_count: None,
        });
    }

    /// Flatten ungrouped results and convert directly to Datums
    /// This is useful for window aggregates where we need Datums immediately
    /// Returns a Vec where the index corresponds to the aggregate index
    pub fn flatten_ungrouped_to_datums(
        self,
        agg_types: &[AggregateType],
    ) -> Vec<Option<pg_sys::Datum>> {
        let mut results = vec![None; agg_types.len()];

        if self.is_empty() {
            return results;
        }

        // Flatten all aggregates (now handles both Custom and Metric)
        let mut rows = Vec::new();
        self.flatten_ungrouped(&mut rows, Some(agg_types));

        if let Some(row) = rows.into_iter().next() {
            for (agg_idx, (agg_type, agg_result)) in
                agg_types.iter().zip(row.aggregates.into_iter()).enumerate()
            {
                let expected_typoid = agg_type.result_type_oid();
                results[agg_idx] = aggregate_result_to_datum(agg_result, agg_type, expected_typoid);
            }
        }

        results
    }

    fn flatten_grouped_with_filter(self, out: &mut Vec<AggregationResultsRow>) {
        if self.is_empty() {
            return;
        }

        // ensure stable sorting of results
        let mut filter_entries: Vec<_> = self
            .0
            .iter()
            .filter(|(k, _)| *k != FilterSentinelKey::NAME)
            .collect();
        filter_entries.sort_by_key(|(k, _)| k.parse::<usize>().unwrap_or(usize::MAX));

        // extract the sentinel filter bucket, used to get all the group keys
        let sentinel = match self.0.get(FilterSentinelKey::NAME) {
            Some(TantivyAggregationResult::BucketResult(BucketResult::Filter(filter_bucket))) => {
                filter_bucket
            }
            _ => {
                panic!("missing filter_sentinel in flatten_grouped_with_filter");
            }
        };

        // collect all group keys from the sentinel
        let sentinel_sub = AggregationResults(sentinel.sub_aggregations.0.clone());
        let mut rows = Vec::new();
        sentinel_sub.collect_group_keys(Vec::new(), &mut rows);

        // for each row of group keys, collect aggregates
        let num_filters = filter_entries.len();
        for row in &mut rows {
            let mut aggregates = Vec::new();

            for (_filter_name, filter_result) in &filter_entries {
                let mut found_metrics: Vec<Option<AggregateResult>> = Vec::new();

                if let TantivyAggregationResult::BucketResult(BucketResult::Filter(filter_bucket)) =
                    filter_result
                {
                    let sub = AggregationResults(filter_bucket.sub_aggregations.0.clone());
                    let mut current = sub.0;

                    for key in &row.group_keys {
                        if let Some(TantivyAggregationResult::BucketResult(BucketResult::Terms {
                            buckets,
                            ..
                        })) = current.get(GroupedKey::NAME)
                        {
                            if let Some(bucket) = buckets.iter().find(|b| match (&b.key, &key.0) {
                                (Key::Str(s), OwnedValue::Str(v)) => s == v,
                                (Key::I64(i), OwnedValue::I64(v)) => i == v,
                                (Key::U64(i), OwnedValue::U64(v)) => i == v,
                                (Key::F64(i), OwnedValue::F64(v)) => i == v,
                                _ => false,
                            }) {
                                current = bucket.sub_aggregation.0.clone();
                            } else {
                                current.clear();
                                break;
                            }
                        }
                    }

                    for (_n, res) in current {
                        match res {
                            TantivyAggregationResult::MetricResult(metric) => {
                                found_metrics.push(Some(MetricResult(metric).into()));
                            }
                            other => {
                                let json_value = serde_json::to_value(&other).unwrap_or_else(|e| {
                                    pgrx::error!("Failed to serialize aggregate: {}", e)
                                });
                                found_metrics.push(Some(AggregateResult::Json(json_value)));
                            }
                        }
                    }
                }

                // if a bucket has multiple sub aggs but no agg was found, that means the agg is null
                // and we should insert an empty placeholder
                if found_metrics.is_empty() {
                    aggregates.push(None);
                } else {
                    aggregates.extend(found_metrics);
                }
            }

            row.aggregates = aggregates;
        }

        out.extend(rows);
    }
}
