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

use crate::gucs;

use crate::aggregate::{execute_aggregate, AggregateRequest};
use crate::api::HashMap;
use crate::customscan::aggregatescan::build::{
    AggregationKey, DocCountKey, FilterSentinelKey, GroupedKey,
};
use crate::postgres::customscan::aggregatescan::AggregateScan;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::types::TantivyValue;

use tantivy::aggregation::agg_result::{
    AggregationResult, AggregationResults as TantivyAggregationResults, BucketResult,
    MetricResult as TantivyMetricResult,
};
use tantivy::aggregation::metric::SingleMetricResult as TantivySingleMetricResult;
use tantivy::aggregation::{Key, DEFAULT_BUCKET_LIMIT};
use tantivy::schema::OwnedValue;

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

    let result: AggregationResults = execute_aggregate(
        state.custom_state().indexrel(),
        query,
        AggregateRequest::Sql(aggregate_clause),
        true,                                              // solve_mvcc
        gucs::adjust_work_mem().get().try_into().unwrap(), // memory_limit
        DEFAULT_BUCKET_LIMIT,                              // bucket_limit
        expr_context,
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
struct AggregationResults(HashMap<String, AggregationResult>);
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
            AggregationStyle::Ungrouped => self.flatten_ungrouped(&mut result),
            AggregationStyle::Grouped => self.flatten_grouped(&mut result),
            AggregationStyle::GroupedWithFilter => self.flatten_grouped_with_filter(&mut result),
        }

        result.into_iter()
    }
}

#[derive(Debug, Default)]
pub struct AggregationResultsRow {
    pub group_keys: Vec<TantivyValue>,
    pub aggregates: Vec<Option<TantivySingleMetricResult>>,
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
impl Into<TantivySingleMetricResult> for MetricResult {
    fn into(self) -> TantivySingleMetricResult {
        match self.0 {
            TantivyMetricResult::Average(r)
            | TantivyMetricResult::Count(r)
            | TantivyMetricResult::Sum(r)
            | TantivyMetricResult::Min(r)
            | TantivyMetricResult::Max(r) => r,
            unsupported => {
                panic!("unsupported metric type: {:?}", unsupported);
            }
        }
    }
}

impl AggregationResults {
    fn is_empty(&self) -> bool {
        if self.0.is_empty() {
            return true;
        }

        let _doc_count = self
            .0
            .get(DocCountKey::NAME)
            .and_then(|result| match result {
                AggregationResult::MetricResult(TantivyMetricResult::Count(
                    TantivySingleMetricResult { value, .. },
                )) => value.clone(),
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
        if let Some(AggregationResult::BucketResult(BucketResult::Terms { buckets, .. })) =
            self.0.get(GroupedKey::NAME)
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
                    Some(AggregationResult::BucketResult(BucketResult::Terms {
                        buckets, ..
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

        // 1. collect all group key paths first
        self.collect_group_keys(Vec::new(), out);

        // 2. for each row, chase down aggregate values matching its group keys
        for row in out.iter_mut() {
            let mut current = self.0.clone();

            // traverse down into nested "grouped" buckets following group_keys
            for key in &row.group_keys {
                if let Some(AggregationResult::BucketResult(BucketResult::Terms {
                    buckets, ..
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

            // 3. collect any metric results at this nested level
            let mut entries: Vec<_> = current.into_iter().collect();
            entries.sort_by_key(|(k, _)| k.parse::<usize>().unwrap_or(usize::MAX));

            for (_name, result) in entries {
                if let AggregationResult::MetricResult(metric) = result {
                    let single = MetricResult(metric).into();
                    row.aggregates.push(Some(single));
                }
            }
        }
    }

    fn flatten_ungrouped(self, out: &mut Vec<AggregationResultsRow>) {
        if self.is_empty() {
            return;
        }

        let mut aggregates = Vec::new();
        let mut entries: Vec<_> = self.0.into_iter().collect();
        entries.sort_by_key(|(k, _)| k.parse::<usize>().unwrap_or(usize::MAX));

        for (_name, result) in entries {
            match result {
                AggregationResult::MetricResult(metric) => {
                    let single = MetricResult(metric).into();
                    aggregates.push(Some(single));
                }
                AggregationResult::BucketResult(BucketResult::Filter(filter_bucket)) => {
                    let mut sub_rows = Vec::new();
                    let sub = AggregationResults(filter_bucket.sub_aggregations.0);
                    sub.flatten_ungrouped(&mut sub_rows);
                    for sub_row in sub_rows {
                        aggregates.extend(sub_row.aggregates);
                    }
                }
                unsupported => todo!("unsupported bucket type: {:?}", unsupported),
            }
        }

        out.push(AggregationResultsRow {
            group_keys: Vec::new(),
            aggregates,
            doc_count: None,
        });
    }

    fn flatten_grouped_with_filter(self, out: &mut Vec<AggregationResultsRow>) {
        if self.is_empty() {
            return;
        }

        // Ensure stable sorting of results
        let mut filter_entries: Vec<_> = self
            .0
            .iter()
            .filter(|(k, _)| *k != FilterSentinelKey::NAME)
            .collect();
        filter_entries.sort_by_key(|(k, _)| k.parse::<usize>().unwrap_or(usize::MAX));

        // Extract the sentinel filter bucket, used to get all the group keys
        let sentinel = match self.0.get(FilterSentinelKey::NAME) {
            Some(AggregationResult::BucketResult(BucketResult::Filter(filter_bucket))) => {
                filter_bucket
            }
            _ => {
                panic!("missing filter_sentinel in flatten_grouped_with_filter");
            }
        };

        // Collect all group keys from the sentinel
        let sentinel_sub = AggregationResults(sentinel.sub_aggregations.0.clone());
        let mut rows = Vec::new();
        sentinel_sub.collect_group_keys(Vec::new(), &mut rows);

        // For each row of group keys, collect aggregates
        let num_filters = filter_entries.len();
        for row in &mut rows {
            let mut aggregates = Vec::new();

            for (_filter_name, filter_result) in &filter_entries {
                let mut found_metrics: Vec<Option<TantivySingleMetricResult>> = Vec::new();

                if let AggregationResult::BucketResult(BucketResult::Filter(filter_bucket)) =
                    filter_result
                {
                    let sub = AggregationResults(filter_bucket.sub_aggregations.0.clone());
                    let mut current = sub.0;

                    for key in &row.group_keys {
                        if let Some(AggregationResult::BucketResult(BucketResult::Terms {
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
                        if let AggregationResult::MetricResult(metric) = res {
                            let single = MetricResult(metric).into();
                            found_metrics.push(Some(single));
                        }
                    }
                }

                // pad: if no metrics found for this filter, insert an empty placeholder
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
