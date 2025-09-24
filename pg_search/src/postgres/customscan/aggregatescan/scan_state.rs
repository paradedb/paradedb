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

use crate::api::HashMap;
use crate::api::{FieldName, OrderByFeature, OrderByInfo, ToTantivyJson};
use crate::gucs;
use crate::postgres::customscan::aggregatescan::privdat::{
    AggregateResult, AggregateType, AggregateValue, GroupingColumn, TargetListEntry,
};
use crate::postgres::customscan::CustomScanState;
use crate::postgres::types::TantivyValue;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{function_name, PgLogLevel, PgSqlErrorCode};
use tantivy::schema::OwnedValue;

use pgrx::pg_sys;
use tinyvec::TinyVec;

/// Source of aggregate result data - either from result map or bucket object
enum AggregateResultSource<'a> {
    /// Results from simple aggregation (no GROUP BY)
    ResultMap(&'a serde_json::Map<String, serde_json::Value>),
    /// Results from bucket aggregation (GROUP BY)
    BucketObj(&'a serde_json::Map<String, serde_json::Value>),
}

pub type AggregateRow = TinyVec<[AggregateValue; 4]>;

// For GROUP BY results, we need both the group keys and aggregate values
#[derive(Debug, Clone)]
pub struct GroupedAggregateRow {
    pub group_keys: Vec<OwnedValue>, // The values of the grouping columns
    pub aggregate_values: AggregateRow,
}

#[derive(Default)]
pub enum ExecutionState {
    #[default]
    NotStarted,
    Emitting(std::vec::IntoIter<GroupedAggregateRow>),
    Completed,
}

#[derive(Default)]
pub struct AggregateScanState {
    // The state of this scan.
    pub state: ExecutionState,
    // The aggregate types that we are executing for.
    pub aggregate_types: Vec<AggregateType>,
    // The grouping columns for GROUP BY
    pub grouping_columns: Vec<GroupingColumn>,
    // The ORDER BY information for sorting
    pub orderby_info: Vec<OrderByInfo>,
    // Maps target list position to data type
    pub target_list_mapping: Vec<TargetListEntry>,
    // The query that will be executed.
    pub query: SearchQueryInput,
    // The index that will be scanned.
    pub indexrelid: pg_sys::Oid,
    // The index relation. Opened during `begin_custom_scan`.
    pub indexrel: Option<(pg_sys::LOCKMODE, PgSearchRelation)>,
    // The execution time RTI (note: potentially different from the planning-time RTI).
    pub execution_rti: pg_sys::Index,
    // The LIMIT, if GROUP BY ... ORDER BY ... LIMIT is present
    pub limit: Option<u32>,
    // The OFFSET, if GROUP BY ... ORDER BY ... LIMIT is present
    pub offset: Option<u32>,
    // Whether a GROUP BY could be lossy (i.e. some buckets truncated)
    pub maybe_truncated: bool,
}

impl AggregateScanState {
    pub fn open_relations(&mut self, lockmode: pg_sys::LOCKMODE) {
        self.indexrel = Some((
            lockmode,
            PgSearchRelation::with_lock(self.indexrelid, lockmode),
        ));
    }

    #[inline(always)]
    pub fn indexrel(&self) -> &PgSearchRelation {
        self.indexrel
            .as_ref()
            .map(|(_, rel)| rel)
            .expect("PdbScanState: indexrel should be initialized")
    }

    pub fn aggregates_to_json(&self) -> serde_json::Value {
        if self.grouping_columns.is_empty() {
            // No GROUP BY - simple aggregation
            let mut agg_map: serde_json::Map<String, serde_json::Value> = self
                .aggregate_types
                .iter()
                .enumerate()
                .map(|(idx, aggregate)| (idx.to_string(), aggregate.to_json()))
                .collect();

            // Add a document count aggregation only if we have SUM aggregates but no COUNT aggregate
            // (to detect empty result sets for SUM)
            let has_sum = self
                .aggregate_types
                .iter()
                .any(|agg| matches!(agg, AggregateType::Sum { .. }));
            let has_count = self
                .aggregate_types
                .iter()
                .any(|agg| matches!(agg, AggregateType::CountAny { .. }));

            if has_sum && !has_count {
                agg_map.insert(
                    "_doc_count".to_string(),
                    serde_json::json!({
                        "value_count": {
                            "field": "ctid"
                        }
                    }),
                );
            }

            return serde_json::Value::Object(agg_map);
        }
        // GROUP BY - nested bucket aggregation (supports arbitrary number of grouping columns)
        // We build the JSON bottom-up so that each grouping column nests the next one.
        let mut current_aggs: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
        let max_term_agg_buckets = gucs::max_term_agg_buckets() as u32;

        for (i, group_col) in self.grouping_columns.iter().enumerate().rev() {
            let mut terms = serde_json::Map::new();
            terms.insert(
                "field".to_string(),
                serde_json::Value::String(group_col.field_name.clone()),
            );

            let orderby_info = self.orderby_info.iter().find(|info| {
                if let OrderByFeature::Field(field_name) = &info.feature {
                    *field_name == FieldName::from(group_col.field_name.clone())
                } else {
                    false
                }
            });

            if let Some(orderby_info) = orderby_info {
                terms.insert(
                    orderby_info.key(),
                    orderby_info
                        .json_value()
                        .expect("ordering by score is not supported"),
                );
            }

            if let Some(limit) = self.limit {
                let size = (limit + self.offset.unwrap_or(0)).min(max_term_agg_buckets);
                terms.insert("size".to_string(), serde_json::Value::Number(size.into()));
                // because we currently support ordering only by the grouping columns, the Top N
                // of all segments is guaranteed to contain the global Top N
                // once we support ordering by aggregates like COUNT, this is no longer guaranteed,
                // and we can no longer set segment_size (per segment top N) = size (global top N)
                terms.insert(
                    "segment_size".to_string(),
                    serde_json::Value::Number(size.into()),
                );
            } else {
                terms.insert(
                    "size".to_string(),
                    serde_json::Value::Number(max_term_agg_buckets.into()),
                );
                terms.insert(
                    "segment_size".to_string(),
                    serde_json::Value::Number(max_term_agg_buckets.into()),
                );
            }

            let mut terms_agg = serde_json::Map::new();
            terms_agg.insert("terms".to_string(), serde_json::Value::Object(terms));

            if i == self.grouping_columns.len() - 1 {
                // Deepest level – attach metric aggregations (may be empty)
                if !self.aggregate_types.is_empty() {
                    let mut sub_aggs = serde_json::Map::new();
                    for (j, aggregate) in self.aggregate_types.iter().enumerate() {
                        if let Some((name, agg)) = aggregate.to_json_for_group(j) {
                            sub_aggs.insert(name, agg);
                        }
                    }
                    if !sub_aggs.is_empty() {
                        terms_agg.insert("aggs".to_string(), serde_json::Value::Object(sub_aggs));
                    }
                }
            } else {
                // Not deepest – nest previously built aggs
                terms_agg.insert("aggs".to_string(), serde_json::Value::Object(current_aggs));
            }

            let mut bucket_container = serde_json::Map::new();
            bucket_container.insert(format!("group_{i}"), serde_json::Value::Object(terms_agg));
            current_aggs = bucket_container;
        }

        serde_json::Value::Object(current_aggs)
    }

    /// Convert a JSON value to an OwnedValue based on the field type from the schema
    fn json_value_to_owned_value(
        &self,
        json_value: &serde_json::Value,
        field_name: &str,
    ) -> OwnedValue {
        // Get the search field from the schema to determine the type
        let indexrel = self.indexrel();
        let schema = indexrel.schema().expect("indexrel should have a schema");
        let search_field = schema.search_field(field_name);
        TantivyValue::json_value_to_owned_value(&search_field, json_value)
    }

    #[allow(unreachable_patterns)]
    pub fn json_to_aggregate_results(&self, result: serde_json::Value) -> Vec<GroupedAggregateRow> {
        if self.grouping_columns.is_empty() {
            // No GROUP BY - single result row
            let result_map = match result.as_object() {
                Some(obj) => obj,
                None => {
                    // Handle null or empty results for empty tables
                    // Create empty aggregate results with proper default values
                    let row = self
                        .aggregate_types
                        .iter()
                        .map(|aggregate| {
                            // For empty tables, return appropriate empty values
                            let empty_result = AggregateResult::Null;
                            aggregate.result_from_aggregate_with_doc_count(empty_result, Some(0))
                        })
                        .collect::<AggregateRow>();

                    return vec![GroupedAggregateRow {
                        group_keys: vec![],
                        aggregate_values: row,
                    }];
                }
            };

            // Check document count for SUM empty result set detection
            let doc_count = result_map
                .get("_doc_count")
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("value"))
                .and_then(|v| {
                    // Try both integer and float values
                    v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))
                });

            let row = self
                .aggregate_types
                .iter()
                .enumerate()
                .map(|(idx, aggregate)| {
                    // Check if the aggregate result exists in the map
                    if let Some(agg_value) = result_map.get(&idx.to_string()) {
                        self.process_aggregate_result(
                            aggregate,
                            idx,
                            AggregateResultSource::ResultMap(result_map),
                            doc_count,
                        )
                    } else {
                        // Handle missing aggregate result (empty table case)
                        use crate::postgres::customscan::aggregatescan::privdat::AggregateResult;
                        let empty_result = AggregateResult::Null;
                        aggregate.result_from_aggregate_with_doc_count(empty_result, Some(0))
                    }
                })
                .collect::<AggregateRow>();

            // No sorting needed for single aggregate result
            return vec![GroupedAggregateRow {
                group_keys: vec![],
                aggregate_values: row,
            }];
        }
        // GROUP BY - extract nested bucket results recursively
        let mut rows = Vec::new();

        // Handle empty results for GROUP BY queries
        if result.is_null() || (result.is_object() && result.as_object().unwrap().is_empty()) {
            // Return empty result set for GROUP BY on empty tables
            return rows;
        }

        self.extract_bucket_results(&result, 0, &mut Vec::new(), &mut rows, None);

        if self.maybe_truncated && self.was_truncated(&result) {
            ErrorReport::new(
                PgSqlErrorCode::ERRCODE_PROGRAM_LIMIT_EXCEEDED,
                format!("query cancelled because result was truncated due to more than {} groups being returned", gucs::max_term_agg_buckets()),
                function_name!(),
            )
            .set_detail("any buckets/groups beyond the first `paradedb.max_term_agg_buckets` were truncated")
            .set_hint("consider lowering the query's `LIMIT` or `OFFSET`")
            .report(PgLogLevel::ERROR);
        }

        rows
    }

    /// Merge results from optimized multi-group queries where aggregates are grouped by filter
    pub fn merge_optimized_multi_group_results(
        &self,
        group_results: Vec<(serde_json::Value, Vec<usize>)>,
    ) -> Vec<GroupedAggregateRow> {
        if self.grouping_columns.is_empty() {
            // Simple aggregation without GROUP BY
            let mut aggregate_values = vec![AggregateValue::Null; self.aggregate_types.len()];

            for (result, aggregate_indices) in group_results {
                let result_map = match result.as_object() {
                    Some(obj) => obj,
                    None => {
                        // Handle non-object results (e.g., null or array results)
                        continue;
                    }
                };

                // Extract results for each aggregate in this group
                for (group_agg_idx, &original_agg_idx) in aggregate_indices.iter().enumerate() {
                    let agg_key = group_agg_idx.to_string();
                    let agg_result = if result_map.contains_key(&agg_key) {
                        Self::extract_aggregate_value_from_json(&result_map[&agg_key])
                    } else {
                        AggregateResult::Null
                    };

                    let aggregate = &self.aggregate_types[original_agg_idx];
                    let agg_value =
                        aggregate.result_from_aggregate_with_doc_count(agg_result, None);
                    aggregate_values[original_agg_idx] = agg_value;
                }
            }

            vec![GroupedAggregateRow {
                group_keys: vec![],
                aggregate_values: aggregate_values.into_iter().collect(),
            }]
        } else {
            // GROUP BY aggregation - merge buckets from multiple groups
            // Use Vec to preserve the order from the first query result (which has ORDER BY applied)
            let mut all_groups: Vec<(String, Vec<OwnedValue>, Vec<AggregateValue>)> = Vec::new();
            let mut group_lookup: HashMap<String, usize> = HashMap::default();

            for (result, aggregate_indices) in group_results {
                // Use the unified extract_bucket_results logic to handle multi-level grouping
                let mut temp_rows = Vec::new();
                self.extract_bucket_results(
                    &result,
                    0,
                    &mut Vec::new(),
                    &mut temp_rows,
                    Some(&aggregate_indices),
                );

                // Merge the extracted rows into all_groups
                for row in temp_rows {
                    let group_key_str = format!("{:?}", row.group_keys);

                    let group_index = if let Some(&index) = group_lookup.get(&group_key_str) {
                        index
                    } else {
                        // New group - add it to the Vec in the order it appears
                        let index = all_groups.len();
                        all_groups.push((
                            group_key_str.clone(),
                            row.group_keys.clone(),
                            vec![AggregateValue::Null; self.aggregate_types.len()],
                        ));
                        group_lookup.insert(group_key_str, index);
                        index
                    };

                    let (_, _, aggregate_values) = &mut all_groups[group_index];

                    // Update aggregate values for this group
                    for (group_agg_idx, &original_agg_idx) in aggregate_indices.iter().enumerate() {
                        if group_agg_idx < row.aggregate_values.len() {
                            aggregate_values[original_agg_idx] =
                                row.aggregate_values[group_agg_idx].clone();
                        }
                    }
                }
            }

            // Convert the Vec back to rows, preserving the order from Tantivy
            let rows: Vec<GroupedAggregateRow> = all_groups
                .into_iter()
                .map(|(_, group_keys, aggregate_values)| GroupedAggregateRow {
                    group_keys,
                    aggregate_values: aggregate_values.into_iter().collect(),
                })
                .collect();

            // Order is preserved from Tantivy's ORDER BY configuration
            rows
        }
    }

    /// Extract aggregate value from JSON using serde deserialization
    /// This handles different structures: direct values, objects with "value" field, or raw objects for COUNT
    fn extract_aggregate_value_from_json(agg_obj: &serde_json::Value) -> AggregateResult {
        // Deserialize using our structured type
        match serde_json::from_value::<AggregateResult>(agg_obj.clone()) {
            Ok(result) => result,
            Err(e) => {
                panic!("Failed to deserialize aggregate result: {e}, value: {agg_obj:?}");
            }
        }
    }

    /// Process a single aggregate result, extracting the raw value and delegating
    /// doc_count handling to the aggregate type's processing method.
    /// This consolidates the logic used in both simple and grouped aggregations.
    fn process_aggregate_result(
        &self,
        aggregate: &AggregateType,
        agg_idx: usize,
        result_source: AggregateResultSource<'_>,
        doc_count: Option<i64>,
    ) -> AggregateValue {
        let agg_result = match (aggregate, result_source) {
            (AggregateType::CountAny { .. }, AggregateResultSource::ResultMap(result_map)) => {
                let raw_result = result_map
                    .get(&agg_idx.to_string())
                    .expect("missing aggregate result");
                Self::extract_aggregate_value_from_json(raw_result)
            }
            (AggregateType::CountAny { .. }, AggregateResultSource::BucketObj(bucket_obj)) => {
                let raw_result = bucket_obj.get("doc_count").expect("missing doc_count");
                Self::extract_aggregate_value_from_json(raw_result)
            }
            (_, AggregateResultSource::ResultMap(result_map)) => {
                let agg_obj = result_map
                    .get(&agg_idx.to_string())
                    .expect("missing aggregate result");
                Self::extract_aggregate_value_from_json(agg_obj)
            }
            (_, AggregateResultSource::BucketObj(bucket_obj)) => {
                let agg_name = format!("agg_{agg_idx}");
                if let Some(agg_obj) = bucket_obj.get(&agg_name) {
                    Self::extract_aggregate_value_from_json(agg_obj)
                } else {
                    AggregateResult::Null
                }
            }
        };

        aggregate.result_from_aggregate_with_doc_count(agg_result, doc_count)
    }

    /// Extract bucket results from JSON, optionally filtering to specific aggregate indices
    /// If aggregate_indices is None, extracts all aggregates (normal case)
    /// If aggregate_indices is Some, extracts only the specified aggregates (merge case)
    fn extract_bucket_results(
        &self,
        json: &serde_json::Value,
        depth: usize,
        prefix_keys: &mut Vec<OwnedValue>,
        rows: &mut Vec<GroupedAggregateRow>,
        aggregate_indices: Option<&[usize]>,
    ) {
        let bucket_name = format!("group_{depth}");
        let buckets = json
            .get(&bucket_name)
            .and_then(|v| v.get("buckets"))
            .and_then(|v| v.as_array());

        // Handle missing bucket results (empty table case)
        let buckets = match buckets {
            Some(b) => b,
            None => {
                // No buckets found, which is expected for empty tables
                // Return early as there are no results to process
                return;
            }
        };

        for bucket in buckets {
            let bucket_obj = bucket.as_object().expect("bucket should be object");

            // Current grouping key - handle bounds checking for multi-column GROUP BY
            if depth >= self.grouping_columns.len() {
                // This shouldn't happen, but handle it gracefully
                return;
            }
            let grouping_column = &self.grouping_columns[depth];
            let key_json = bucket_obj.get("key").expect("missing bucket key");
            let key_owned = self.json_value_to_owned_value(key_json, &grouping_column.field_name);
            prefix_keys.push(key_owned);

            if depth + 1 == self.grouping_columns.len() {
                // Deepest level – collect aggregates
                let aggregate_values: AggregateRow = match aggregate_indices {
                    Some(indices) => {
                        // Merge case: extract only specified aggregates
                        if indices.is_empty() {
                            AggregateRow::default()
                        } else {
                            // Extract doc_count for empty result set handling
                            let doc_count = bucket_obj
                                .get("doc_count")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);

                            indices
                                .iter()
                                .enumerate()
                                .map(|(group_agg_idx, &original_agg_idx)| {
                                    let aggregate = &self.aggregate_types[original_agg_idx];

                                    if matches!(aggregate, AggregateType::CountAny { .. }) {
                                        // Count aggregate - use doc_count
                                        let count_result = AggregateResult::DirectValue(
                                            serde_json::Number::from(doc_count),
                                        );
                                        aggregate.result_from_aggregate_with_doc_count(
                                            count_result,
                                            Some(doc_count),
                                        )
                                    } else {
                                        // Non-count aggregate - look for agg_{group_agg_idx}
                                        let agg_key = format!("agg_{group_agg_idx}");
                                        if let Some(agg_obj) = bucket_obj.get(&agg_key) {
                                            let agg_result =
                                                Self::extract_aggregate_value_from_json(agg_obj);
                                            aggregate.result_from_aggregate_with_doc_count(
                                                agg_result,
                                                Some(doc_count),
                                            )
                                        } else {
                                            AggregateValue::Null
                                        }
                                    }
                                })
                                .collect()
                        }
                    }
                    None => {
                        // Normal case: extract all aggregates
                        if self.aggregate_types.is_empty() {
                            AggregateRow::default()
                        } else {
                            // Extract doc_count for empty result set handling
                            let doc_count = bucket_obj
                                .get("doc_count")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);

                            self.aggregate_types
                                .iter()
                                .enumerate()
                                .map(|(idx, aggregate)| {
                                    self.process_aggregate_result(
                                        aggregate,
                                        idx,
                                        AggregateResultSource::BucketObj(bucket_obj),
                                        Some(doc_count),
                                    )
                                })
                                .collect()
                        }
                    }
                };
                rows.push(GroupedAggregateRow {
                    group_keys: prefix_keys.clone(),
                    aggregate_values,
                });
            } else {
                // Recurse into next level
                self.extract_bucket_results(
                    bucket,
                    depth + 1,
                    prefix_keys,
                    rows,
                    aggregate_indices,
                );
            }

            prefix_keys.pop();
        }
    }

    fn was_truncated(&self, result: &serde_json::Value) -> bool {
        result
            .as_object()
            .map(|obj| {
                obj.iter()
                    .filter_map(|(key, value)| {
                        if key.starts_with("group_") {
                            value
                                .as_object()
                                .and_then(|group_obj| group_obj.get("sum_other_doc_count"))
                                .and_then(|v| v.as_i64())
                        } else {
                            None
                        }
                    })
                    .sum::<i64>()
            })
            .unwrap_or(0)
            > 0
    }
}

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        // TODO: Unused currently. See the comment on `trait CustomScanState` regarding making this
        // more useful.
    }
}
