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

use crate::api::OrderByInfo;
use crate::customscan::aggregatescan::{AggregateCSClause, AggregateScan, CustomScanClause};
use crate::postgres::customscan::aggregatescan::groupby::GroupingColumn;
use crate::postgres::customscan::aggregatescan::privdat::{
    AggregateResult, AggregateType, AggregateValue, TargetListEntry,
};
use crate::postgres::customscan::CustomScanState;
use crate::postgres::types::TantivyValue;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;
use tantivy::schema::OwnedValue;
use tantivy::aggregation::agg_result::AggregationResult;

use pgrx::pg_sys;
use tinyvec::TinyVec;

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
    Emitting(std::vec::IntoIter<Vec<AggregationResult>>),
    Completed,
}

#[derive(Default)]
pub struct AggregateScanState {
    pub state: ExecutionState,
    pub target_list_mapping: Vec<TargetListEntry>,
    pub indexrelid: pg_sys::Oid,
    pub indexrel: Option<(pg_sys::LOCKMODE, PgSearchRelation)>,
    pub execution_rti: pg_sys::Index,
    pub aggregate_clause: AggregateCSClause,
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

    pub fn process_aggregation_results(
        &self,
        result: serde_json::Value,
    ) -> Vec<GroupedAggregateRow> {
        if self.aggregate_clause.grouping_columns().is_empty() {
            // No GROUP BY - simple aggregation results
            self.process_simple_filter_aggregation_results(result)
        } else {
            // GROUP BY - process nested results (handles both filtered and non-filtered)
            self.process_grouped_aggregation_results(result)
        }
    }

    /// Process simple filter aggregation results (no GROUP BY)
    fn process_simple_filter_aggregation_results(
        &self,
        result: serde_json::Value,
    ) -> Vec<GroupedAggregateRow> {
        let result_obj = match result.as_object() {
            Some(obj) => obj,
            None => {
                // Handle null or empty results - return appropriate empty values
                let row = self
                    .aggregate_clause
                    .aggregates()
                    .iter()
                    .map(|aggregate| aggregate.empty_value())
                    .collect::<AggregateRow>();
                return vec![GroupedAggregateRow {
                    group_keys: vec![],
                    aggregate_values: row,
                }];
            }
        };

        // Check if this is direct format (numeric keys) or filter format (filter_* keys)
        if !self.has_filters() {
            // Fast path: Direct aggregation format (no FilterAggregation wrapper)
            // Format: {"0": {...}, "1": {...}, "_doc_count": {"value": 0}}

            // Extract _doc_count for NULL handling
            // _doc_count is a value_count aggregation with format {"value": N}
            let doc_count = result_obj
                .get("_doc_count")
                .and_then(|v| v.get("value"))
                .and_then(|v| v.as_f64())
                .map(|f| f as i64);

            let aggregate_values = self
                .aggregate_clause
                .aggregates()
                .iter()
                .enumerate()
                .map(|(idx, aggregate)| {
                    if let Some(value) = result_obj.get(&idx.to_string()) {
                        let agg_result = Self::extract_aggregate_value_from_json(value);
                        aggregate.result_from_aggregate_with_doc_count(agg_result, doc_count)
                    } else {
                        aggregate.empty_value()
                    }
                })
                .collect();

            return vec![GroupedAggregateRow {
                group_keys: vec![],
                aggregate_values,
            }];
        }

        // Slow path: FilterAggregation format (with filter_* keys)
        let filter_results: Vec<(usize, &serde_json::Value)> = result_obj
            .iter()
            .filter_map(|(key, value)| {
                key.strip_prefix("filter_")
                    .and_then(|idx_str| idx_str.parse::<usize>().ok())
                    .map(|idx| (idx, value))
            })
            .collect();

        // Extract aggregate values
        let aggregate_values = self
            .aggregate_clause
            .aggregates()
            .iter()
            .enumerate()
            .map(|(idx, aggregate)| {
                // Look up the filter result for this aggregate
                if let Some((_, filter_value)) = filter_results.iter().find(|(i, _)| *i == idx) {
                    self.extract_simple_aggregate_value(filter_value, aggregate)
                } else {
                    // No filter result for this aggregate (shouldn't happen)
                    aggregate.empty_value()
                }
            })
            .collect();

        vec![GroupedAggregateRow {
            group_keys: vec![],
            aggregate_values,
        }]
    }

    /// Extract aggregate value from a simple (non-grouped) filter result
    fn extract_simple_aggregate_value(
        &self,
        filter_value: &serde_json::Value,
        aggregate: &AggregateType,
    ) -> AggregateValue {
        let doc_count = filter_value.get("doc_count").and_then(|v| v.as_i64());

        // Look for the aggregate result in filtered_agg or directly
        let agg_result = if let Some(filtered_agg) = filter_value.get("filtered_agg") {
            Self::extract_aggregate_value_from_json(filtered_agg)
        } else {
            // Fallback: extract directly (shouldn't normally happen)
            Self::extract_aggregate_value_from_json(filter_value)
        };

        aggregate.result_from_aggregate_with_doc_count(agg_result, doc_count)
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

    /// Process grouped aggregation results (with GROUP BY)
    /// Handles both direct format (no filters) and filter format (with filters)
    fn process_grouped_aggregation_results(
        &self,
        result: serde_json::Value,
    ) -> Vec<GroupedAggregateRow> {
        // Handle empty results
        if result.is_null() || (result.is_object() && result.as_object().unwrap().is_empty()) {
            return Vec::new();
        }

        let result_obj = result
            .as_object()
            .expect("GROUP BY results should be an object");

        let mut rows = Vec::new();
        // Check if this is direct format (no filters) or filter format (with filters)
        if !self.has_filters() {
            // Fast path: Direct aggregation format (no FilterAggregation wrapper)
            // Format: {"grouped": {"buckets": [...], "aggs": {"0": {...}, "1": {...}}}}
            let grouped = result_obj.get("grouped").unwrap();
            self.walk_grouped_buckets(grouped, None, 0, &mut Vec::new(), &mut rows);
        } else {
            // Slow path: FilterAggregation format (with filter_sentinel and filter_* keys)
            let sentinel_grouped = result_obj
                .get("filter_sentinel")
                .and_then(|f| f.get("grouped"))
                .expect("filter_sentinel should have grouped structure");

            // Collect filter aggregation results for value lookup
            let filter_results: Vec<(usize, &serde_json::Value)> = result_obj
                .iter()
                .filter_map(|(key, value)| {
                    key.strip_prefix("filter_")
                        .and_then(|idx_str| idx_str.parse::<usize>().ok())
                        .and_then(|idx| value.get("grouped").map(|grouped| (idx, grouped)))
                })
                .collect();

            // Walk sentinel structure and extract rows
            self.walk_grouped_buckets(
                sentinel_grouped,
                Some(&filter_results),
                0,
                &mut Vec::new(),
                &mut rows,
            );
        }

        rows
    }

    /// Walk grouped buckets recursively, collecting group keys and aggregate values
    ///
    /// - If `filter_results` is None: Direct format - extract aggregates from buckets
    /// - If `filter_results` is Some: Filter format - look up aggregates in filter_results
    fn walk_grouped_buckets(
        &self,
        grouped: &serde_json::Value,
        filter_results: Option<&[(usize, &serde_json::Value)]>,
        depth: usize,
        group_keys: &mut Vec<OwnedValue>,
        output_rows: &mut Vec<GroupedAggregateRow>,
    ) {
        let buckets = match grouped.get("buckets").and_then(|b| b.as_array()) {
            Some(b) => b,
            None => return,
        };

        let grouping_columns = self.aggregate_clause.grouping_columns();

        if depth >= grouping_columns.len() {
            return;
        }

        let grouping_column = &grouping_columns[depth];

        for bucket in buckets {
            let bucket_obj = bucket.as_object().expect("bucket should be object");

            // Extract and store group key
            let key_json = bucket_obj.get("key").expect("bucket should have key");
            let key_owned = self.json_value_to_owned_value(key_json, &grouping_column.field_name);
            group_keys.push(key_owned.clone());

            if depth + 1 == grouping_columns.len() {
                // Leaf level - extract aggregate values
                let aggregate_values = match filter_results {
                    None => {
                        // Direct format: extract from bucket
                        let doc_count = bucket_obj.get("doc_count").and_then(|d| d.as_i64());

                        self.aggregate_clause
                            .aggregates()
                            .iter()
                            .enumerate()
                            .map(|(idx, aggregate)| {
                                // For COUNT(*), use doc_count directly
                                if matches!(aggregate, AggregateType::CountAny { .. }) {
                                    return doc_count
                                        .map(AggregateValue::Int)
                                        .unwrap_or_else(|| aggregate.empty_value());
                                }

                                // For other aggregates, look up by index
                                if let Some(value) = bucket_obj.get(&idx.to_string()) {
                                    let agg_result = Self::extract_aggregate_value_from_json(value);
                                    aggregate
                                        .result_from_aggregate_with_doc_count(agg_result, doc_count)
                                } else {
                                    aggregate.empty_value()
                                }
                            })
                            .collect()
                    }
                    Some(filter_results) => {
                        // Filter format: look up in filter_results
                        self.extract_aggregates_for_group(filter_results, group_keys)
                    }
                };

                output_rows.push(GroupedAggregateRow {
                    group_keys: group_keys.clone(),
                    aggregate_values,
                });
            } else {
                // Recurse into nested grouped
                if let Some(nested_grouped) = bucket_obj.get("grouped") {
                    self.walk_grouped_buckets(
                        nested_grouped,
                        filter_results,
                        depth + 1,
                        group_keys,
                        output_rows,
                    );
                }
            }

            group_keys.pop();
        }
    }

    /// Extract aggregate values for a specific group by looking them up in filter results
    fn extract_aggregates_for_group(
        &self,
        filter_results: &[(usize, &serde_json::Value)],
        group_keys: &[OwnedValue],
    ) -> AggregateRow {
        let aggregates = self.aggregate_clause.aggregates();
        if aggregates.is_empty() {
            return AggregateRow::default();
        }

        aggregates
            .iter()
            .enumerate()
            .map(|(agg_idx, aggregate)| {
                // Find the filter result for this aggregate
                if let Some((_, filter_grouped)) =
                    filter_results.iter().find(|(idx, _)| *idx == agg_idx)
                {
                    // Look up the aggregate value for this group in the filter result
                    if let Some(value) = self.find_aggregate_value_in_filter(
                        filter_grouped,
                        group_keys,
                        0,
                        aggregate,
                    ) {
                        value
                    } else {
                        // Filter didn't match this group
                        aggregate.empty_value()
                    }
                } else {
                    // No filter result for this aggregate (shouldn't happen)
                    aggregate.empty_value()
                }
            })
            .collect()
    }

    /// Find aggregate value in filter result by matching group keys (iterative)
    /// This iterates through grouping levels to avoid stack overflow with deep nesting
    fn find_aggregate_value_in_filter(
        &self,
        mut grouped: &serde_json::Value,
        group_keys: &[OwnedValue],
        depth: usize,
        aggregate: &AggregateType,
    ) -> Option<AggregateValue> {
        // Iterate through each grouping level instead of recursing
        for level in depth..group_keys.len() {
            let buckets = grouped.get("buckets")?.as_array()?;
            let target_key = &group_keys[level];
            let grouping_column = &self.aggregate_clause.grouping_columns()[level];

            // Find bucket matching this group key
            let matching_bucket = buckets.iter().find_map(|bucket| {
                let bucket_obj = bucket.as_object()?;
                let key_json = bucket_obj.get("key")?;

                if Self::keys_match(key_json, target_key, grouping_column, self) {
                    Some(bucket_obj)
                } else {
                    None
                }
            })?;

            // If this is the last grouping level, extract the aggregate value
            if level + 1 == group_keys.len() {
                return self.extract_aggregate_from_bucket(matching_bucket, aggregate);
            }

            // Move to nested grouped for next iteration
            grouped = matching_bucket.get("grouped")?;
        }

        None
    }

    /// Check if a JSON key matches a target OwnedValue key
    #[inline]
    fn keys_match(
        key_json: &serde_json::Value,
        target_key: &OwnedValue,
        grouping_column: &GroupingColumn,
        scan_state: &AggregateScanState,
    ) -> bool {
        let bucket_key =
            scan_state.json_value_to_owned_value(key_json, &grouping_column.field_name);
        bucket_key == *target_key
    }

    /// Extract aggregate value from a bucket at the leaf level
    #[inline]
    fn extract_aggregate_from_bucket(
        &self,
        bucket: &serde_json::Map<String, serde_json::Value>,
        aggregate: &AggregateType,
    ) -> Option<AggregateValue> {
        let doc_count = bucket.get("doc_count").and_then(|d| d.as_i64());

        // Performance optimization: For COUNT(*) in GROUP BY, use doc_count directly
        // No explicit aggregation is added for this case
        if matches!(aggregate, AggregateType::CountAny { .. }) {
            return doc_count.map(AggregateValue::Int);
        }

        // Look for the aggregate value - could be a numeric key or named sub-aggregation
        for (key, value) in bucket.iter() {
            if key.parse::<usize>().is_ok()
                || matches!(key.as_str(), "value" | "avg" | "sum" | "min" | "max")
            {
                let agg_result = Self::extract_aggregate_value_from_json(value);
                return Some(aggregate.result_from_aggregate_with_doc_count(agg_result, doc_count));
            }
        }

        // No aggregate found in this bucket
        None
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

    fn has_filters(&self) -> bool {
        self.aggregate_clause
            .aggregates()
            .iter()
            .any(|agg| agg.filter_expr().is_some())
    }
}

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        // TODO: Unused currently. See the comment on `trait CustomScanState` regarding making this
        // more useful.
    }
}
