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
use crate::gucs::WorkMem;

use crate::aggregate::{AggregateRequest, execute_aggregate, scrub_missing_sentinel_value};
use crate::api::HashMap;
use crate::api::version::VersionInfo;
use crate::customscan::aggregatescan::build::{
    AggregationKey, DocCountKey, FilterSentinelKey, GroupedKey,
};
use crate::postgres::customscan::aggregatescan::explain::AggregateParallelism;
use crate::postgres::customscan::aggregatescan::json_rewrite::rewrite_aggregate_result_json_timestamps;
use crate::postgres::customscan::aggregatescan::{AggIndexInfo, AggregateScan, AggregateType};
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::datetime::PostgresDateTime;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::types::{TantivyValue, is_datetime_type};
use pgrx::{IntoDatum, JsonB, check_for_interrupts, pg_sys};

use tantivy::aggregation::Key;
use tantivy::aggregation::agg_result::{
    AggregationResult as TantivyAggregationResult, AggregationResults as TantivyAggregationResults,
    BucketEntry, BucketResult, MetricResult as TantivyMetricResult,
};
use tantivy::aggregation::metric::SingleMetricResult as TantivySingleMetricResult;

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
    let expr_context = state.csstate.ss.ps.ps_ExprContext;
    let runtime_context = state.custom_state().runtime_context;

    state
        .custom_state_mut()
        .prepare_query_for_execution(planstate, runtime_context);

    let aggregate_clause = state.custom_state().aggregate_clause.clone();
    let query = aggregate_clause.query().clone();

    // Use the GUC for term aggregation bucket limits (single source of truth).
    let bucket_limit: u32 = gucs::max_term_agg_buckets() as u32;
    let visibility = aggregate_clause.visibility();
    let grouping_fields: Vec<String> = aggregate_clause
        .grouping_columns()
        .into_iter()
        .map(|column| column.field_name)
        .collect();
    // Routing (see create_custom_path) sends high-cardinality GROUP BYs to
    // DataFusion using an estimate, but a stale/wrong estimate can still land an
    // over-cap query here. Truncation at the cap is only recoverable for a single
    // grouping column bounded by a static LIMIT+OFFSET within the cap: the ordered
    // prefix Tantivy returns covers the requested window. Otherwise a dropped
    // group is silent data loss, so we error instead.
    let truncation_recoverable = grouping_fields.len() == 1
        && aggregate_clause
            .static_fetch()
            .is_some_and(|fetch| fetch as u64 <= bucket_limit as u64);

    let mut bitmap_exec = state.custom_state_mut().bitmap_exec.take();
    let mut segment_pruning = std::mem::take(&mut state.custom_state_mut().segment_pruning);
    let mut visibility_stats = std::mem::take(&mut state.custom_state_mut().visibility_stats);
    let mut parallelism = AggregateParallelism::default();
    let collect_visibility_stats =
        unsafe { !planstate.is_null() && !(*planstate).instrument.is_null() };
    let result: AggregationResults = execute_aggregate(
        state.custom_state().indexrel(),
        query,
        AggregateRequest::Sql(aggregate_clause),
        visibility,
        WorkMem::Tantivy.bytes().try_into().unwrap(),
        bucket_limit,
        expr_context,
        planstate,
        bitmap_exec.as_mut(),
        collect_visibility_stats.then_some(&mut segment_pruning),
        collect_visibility_stats.then_some(&mut visibility_stats),
        collect_visibility_stats.then_some(&mut parallelism),
    )
    .unwrap_or_else(|e| pgrx::error!("Failed to execute filter aggregation: {}", e))
    .into();
    state.custom_state_mut().bitmap_exec = bitmap_exec;
    state.custom_state_mut().segment_pruning = segment_pruning;
    state.custom_state_mut().visibility_stats = visibility_stats;
    state.custom_state_mut().parallelism = collect_visibility_stats.then_some(parallelism);

    // Tantivy caps a terms aggregation at `size` and folds the dropped groups into
    // `sum_other_doc_count` rather than erroring, which would silently return an
    // incomplete GROUP BY. Fail loudly instead. This is an interim guard until we
    // route on cost between Tantivy and DataFusion; see
    // https://github.com/paradedb/paradedb/issues/5565.
    if !truncation_recoverable && result.any_terms_truncated(bucket_limit as u64) {
        let on_fields = if grouping_fields.is_empty() {
            String::new()
        } else {
            let names = grouping_fields
                .iter()
                .map(|field| format!("\"{field}\""))
                .collect::<Vec<_>>()
                .join(", ");
            format!(" on {names}")
        };
        pgrx::error!(
            "GROUP BY{on_fields} produced more than paradedb.max_term_agg_buckets ({bucket_limit}) \
             groups; returning the result would silently drop groups. Raise \
             paradedb.max_term_agg_buckets to cover the group cardinality, or add a LIMIT to bound \
             the result."
        );
    }

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
            Some(doc_count) => TantivyValue(PdbOwnedValue::U64(doc_count)),
            None => TantivyValue(PdbOwnedValue::Null),
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

/// A metric or filter count as its value; anything else serialized to JSON.
fn to_aggregate_result(result: &TantivyAggregationResult) -> AggregateResult {
    match result {
        TantivyAggregationResult::MetricResult(metric) => MetricResult(metric.clone()).into(),
        TantivyAggregationResult::BucketResult(BucketResult::Filter(filter_bucket))
            if filter_bucket.sub_aggregations.0.is_empty() =>
        {
            AggregateResult::Metric(TantivySingleMetricResult {
                value: Some(filter_bucket.doc_count as f64),
            })
        }
        other => {
            let json_value = serde_json::to_value(other)
                .unwrap_or_else(|e| pgrx::error!("Failed to serialize aggregate: {}", e));
            AggregateResult::Json(json_value)
        }
    }
}

/// Convert an AggregateResult to a PostgreSQL Datum
/// This is the shared logic for converting both Custom (JSON) and Metric aggregates
///
/// # Arguments
/// * `agg_result` - The aggregate result to convert (Metric or Json)
/// * `agg_type` - The aggregate type (for nullish fallback)
/// * `expected_typoid` - The expected PostgreSQL type OID from the tuple descriptor
/// * `index_info` - The index metadata used to decide whether to rewrite datetime values
pub fn aggregate_result_to_datum(
    agg_result: Option<AggregateResult>,
    agg_type: &AggregateType,
    expected_typoid: pg_sys::Oid,
    index_info: &AggIndexInfo,
) -> Option<pg_sys::Datum> {
    match agg_result {
        Some(AggregateResult::Json(mut json_value)) => {
            // Custom aggregate - return as JSONB
            scrub_missing_sentinel_value(&mut json_value);
            if index_info.created_by_version.stores_datetimes_in_i64() {
                let agg_json = agg_type.custom_agg_json().expect(
                    "AggregateResult::Json should always be paired with AggregateType::Custom",
                );
                rewrite_aggregate_result_json_timestamps(
                    &mut json_value,
                    agg_json,
                    &index_info.schema,
                );
            }
            JsonB(json_value).into_datum()
        }
        Some(AggregateResult::Metric(metric)) => {
            // Standard metric - convert f64 to appropriate type
            // If expected type is JSONB (for pdb.agg() custom aggregates),
            // serialize the entire metric result to match Tantivy's JSON format
            if expected_typoid == pg_sys::JSONBOID {
                // Serialize the SingleMetricResult to JSON
                let mut json_value = serde_json::to_value(&metric).unwrap_or_else(|e| {
                    pgrx::error!("Failed to serialize metric result to JSON: {}", e)
                });
                // For v2 indexes, attach `key_as_string` for single-value metrics on datetime
                // fields so consumers don't have to interpret raw i64 micros.
                if index_info.created_by_version.stores_datetimes_in_i64()
                    && let Some(agg_json) = agg_type.custom_agg_json()
                {
                    rewrite_aggregate_result_json_timestamps(
                        &mut json_value,
                        agg_json,
                        &index_info.schema,
                    );
                }
                JsonB(json_value).into_datum()
            } else if is_datetime_type(expected_typoid) {
                if index_info.created_by_version.stores_datetimes_in_i64() {
                    // v2 (>= DATETIME_I64_STORAGE_VERSION): datetimes are stored as i64
                    // microseconds from the postgres epoch.
                    metric.value.and_then(|value| unsafe {
                        let pgdt = PostgresDateTime::try_from_raw(value as i64).expect(
                            "We should never see an invalid timestamp converting back from tantivy",
                        );
                        TantivyValue(PdbOwnedValue::Date(pgdt))
                            .try_into_datum(expected_typoid.into())
                            .unwrap()
                    })
                } else {
                    // v1 legacy: tantivy stores DateTime values in fast fields as nanoseconds
                    // since UNIX epoch. The f64 from MIN/MAX represents that nanosecond timestamp.
                    metric.value.and_then(|value| unsafe {
                        let datetime = tantivy::DateTime::from_timestamp_nanos(value as i64);
                        let pgdt = PostgresDateTime::try_from(datetime).expect(
                            "We should never see an invalid timestamp coming back from tantivy",
                        );
                        TantivyValue(PdbOwnedValue::Date(pgdt))
                            .try_into_datum(expected_typoid.into())
                            .unwrap()
                    })
                }
            } else {
                metric.value.and_then(|value| unsafe {
                    TantivyValue(PdbOwnedValue::F64(value))
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
                    TantivyValue(PdbOwnedValue::F64(value))
                        .try_into_datum(expected_typoid.into())
                        .unwrap()
                })
            }
        }
    }
}

impl AggregationResults {
    /// True when a terms (GROUP BY) aggregation was truncated at the
    /// `max_term_agg_buckets` cap: a bucket list filled to the cap with a
    /// non-zero `sum_other_doc_count` (dropped groups). A smaller user LIMIT
    /// leaves `buckets.len() < max_buckets`, so it does not count. Tantivy folds
    /// the dropped groups into `sum_other_doc_count` rather than erroring, so the
    /// caller must reject the result to avoid silently returning an incomplete
    /// GROUP BY.
    fn any_terms_truncated(&self, max_buckets: u64) -> bool {
        fn walk<'a>(
            mut results: impl Iterator<Item = &'a TantivyAggregationResult>,
            max_buckets: u64,
        ) -> bool {
            results.any(|result| match result {
                TantivyAggregationResult::BucketResult(BucketResult::Terms {
                    buckets,
                    sum_other_doc_count,
                    ..
                }) => {
                    (*sum_other_doc_count > 0 && buckets.len() as u64 >= max_buckets)
                        || buckets
                            .iter()
                            .any(|bucket| walk(bucket.sub_aggregation.0.values(), max_buckets))
                }
                TantivyAggregationResult::BucketResult(BucketResult::Filter(filter)) => {
                    walk(filter.sub_aggregations.0.values(), max_buckets)
                }
                _ => false,
            })
        }

        max_buckets != 0 && walk(self.0.values(), max_buckets)
    }

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

        if let Some(_doc_count) = _doc_count
            && _doc_count == 0.0
        {
            return true;
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

    /// Walks the "grouped" terms buckets depth first and calls `visit` once per leaf with its key
    /// path, its bucket, and each filter's subtree at the same group path (`None` once a filter has
    /// no bucket for that path).
    fn visit_groups<'a>(
        buckets: &'a [BucketEntry],
        filters: &[Option<&'a HashMap<String, TantivyAggregationResult>>],
        key_path: &mut Vec<TantivyValue>,
        visit: &mut impl FnMut(
            Vec<TantivyValue>,
            &'a BucketEntry,
            &[Option<&'a HashMap<String, TantivyAggregationResult>>],
        ),
    ) {
        // A filter without a "grouped" level here is passed down unchanged. The planner nests
        // every filter like the sentinel, so this only guards against an unexpected shape.
        let lookups: Vec<_> = filters
            .iter()
            .map(|filter| filter.and_then(grouped_buckets).map(BucketLookup::new))
            .collect();

        let mut filter_subtrees = Vec::with_capacity(filters.len());
        for bucket_entry in buckets {
            check_for_interrupts!();
            filter_subtrees.clear();
            filter_subtrees.extend(filters.iter().zip(&lookups).map(|(filter, lookup)| {
                match lookup {
                    Some(lookup) => lookup
                        .find(&bucket_entry.key)
                        .map(|bucket| &bucket.sub_aggregation.0),
                    None => *filter,
                }
            }));

            key_path.push(key_to_value(&bucket_entry.key));
            match grouped_buckets(&bucket_entry.sub_aggregation.0) {
                // not a leaf yet; keep descending
                Some(children) if !children.is_empty() => {
                    Self::visit_groups(children, &filter_subtrees, key_path, visit)
                }
                _ => visit(key_path.clone(), bucket_entry, &filter_subtrees),
            }
            key_path.pop();
        }
    }

    fn flatten_grouped(self, out: &mut Vec<AggregationResultsRow>) {
        if self.is_empty() {
            return;
        }

        let buckets = grouped_buckets(&self.0).unwrap_or_default();
        Self::visit_groups(
            buckets,
            &[],
            &mut Vec::new(),
            &mut |group_keys, bucket, _| {
                // collect any metric results at this nested level
                let mut entries: Vec<_> = bucket.sub_aggregation.0.iter().collect();
                entries.sort_by_key(|(k, _)| k.parse::<usize>().unwrap_or(usize::MAX));
                out.push(AggregationResultsRow {
                    group_keys,
                    aggregates: entries
                        .into_iter()
                        .map(|(_, result)| Some(to_aggregate_result(result)))
                        .collect(),
                    doc_count: Some(bucket.doc_count),
                });
            },
        );
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
                    if filter_bucket.sub_aggregations.0.is_empty() {
                        aggregates.push(Some(AggregateResult::Metric(TantivySingleMetricResult {
                            value: Some(filter_bucket.doc_count as f64),
                        })));
                    } else {
                        // Standard filter aggregate (not custom)
                        let mut sub_rows = Vec::new();
                        let sub = AggregationResults(filter_bucket.sub_aggregations.0);
                        sub.flatten_ungrouped(&mut sub_rows, agg_types);
                        for sub_row in sub_rows {
                            aggregates.extend(sub_row.aggregates);
                        }
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
        index_info: &AggIndexInfo,
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
                agg_types.iter().zip(row.aggregates).enumerate()
            {
                let expected_typoid = agg_type.result_type_oid();
                results[agg_idx] =
                    aggregate_result_to_datum(agg_result, agg_type, expected_typoid, index_info);
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

        // each filter's subtree, in filter order
        let filters: Vec<_> = filter_entries
            .iter()
            .map(|(_, filter_result)| match filter_result {
                TantivyAggregationResult::BucketResult(BucketResult::Filter(filter_bucket)) => {
                    Some(&filter_bucket.sub_aggregations.0)
                }
                _ => None,
            })
            .collect();

        // Walk the sentinel's groups, carrying each filter's matching subtree along.
        let buckets = grouped_buckets(&sentinel.sub_aggregations.0).unwrap_or_default();
        Self::visit_groups(
            buckets,
            &filters,
            &mut Vec::new(),
            &mut |group_keys, bucket, filter_subtrees| {
                let mut aggregates = Vec::with_capacity(filter_subtrees.len());
                for subtree in filter_subtrees {
                    match subtree {
                        Some(subtree) if !subtree.is_empty() => aggregates
                            .extend(subtree.values().map(|res| Some(to_aggregate_result(res)))),
                        // a filter with no metrics for this group means the aggregate is NULL
                        _ => aggregates.push(None),
                    }
                }
                out.push(AggregationResultsRow {
                    group_keys,
                    aggregates,
                    doc_count: Some(bucket.doc_count),
                });
            },
        );
    }
}

/// The buckets of the "grouped" terms aggregation at this level, if there is one.
fn grouped_buckets(map: &HashMap<String, TantivyAggregationResult>) -> Option<&[BucketEntry]> {
    match map.get(GroupedKey::NAME) {
        Some(TantivyAggregationResult::BucketResult(BucketResult::Terms { buckets, .. })) => {
            Some(buckets.as_slice())
        }
        _ => None,
    }
}

fn key_to_value(key: &Key) -> TantivyValue {
    match key {
        Key::Str(s) => TantivyValue(PdbOwnedValue::Str(s.clone())),
        Key::I64(i) => TantivyValue(PdbOwnedValue::I64(*i)),
        Key::U64(u) => TantivyValue(PdbOwnedValue::U64(*u)),
        Key::F64(f) => TantivyValue(PdbOwnedValue::F64(*f)),
    }
}

/// Finds the bucket of one terms level whose key equals a given key, matching keys of the same
/// variant with `==` (so NaN matches nothing and -0.0 matches 0.0) and returning the first match.
/// Short bucket lists are scanned; longer ones are indexed once so each lookup is O(1).
enum BucketLookup<'a> {
    Scan(&'a [BucketEntry]),
    Index(HashMap<LookupKey<'a>, &'a BucketEntry>),
}

#[derive(PartialEq, Eq, Hash)]
enum LookupKey<'a> {
    Str(&'a str),
    I64(i64),
    U64(u64),
    F64(u64),
}

impl<'a> LookupKey<'a> {
    fn new(key: &'a Key) -> Option<Self> {
        Some(match key {
            Key::Str(s) => LookupKey::Str(s),
            Key::I64(i) => LookupKey::I64(*i),
            Key::U64(u) => LookupKey::U64(*u),
            Key::F64(f) if f.is_nan() => return None,
            // -0.0 == 0.0, so both share one entry
            Key::F64(f) => LookupKey::F64(if *f == 0.0 { 0 } else { f.to_bits() }),
        })
    }
}

impl<'a> BucketLookup<'a> {
    const SCAN_LIMIT: usize = 32;

    fn new(buckets: &'a [BucketEntry]) -> Self {
        if buckets.len() <= Self::SCAN_LIMIT {
            return BucketLookup::Scan(buckets);
        }
        let mut index = HashMap::with_capacity_and_hasher(buckets.len(), rustc_hash::FxBuildHasher);
        for bucket in buckets {
            if let Some(key) = LookupKey::new(&bucket.key) {
                index.entry(key).or_insert(bucket);
            }
        }
        BucketLookup::Index(index)
    }

    fn find(&self, key: &Key) -> Option<&'a BucketEntry> {
        let key = LookupKey::new(key)?;
        match self {
            BucketLookup::Scan(buckets) => buckets
                .iter()
                .find(|b| LookupKey::new(&b.key).as_ref() == Some(&key)),
            BucketLookup::Index(index) => index.get(&key).copied(),
        }
    }
}
