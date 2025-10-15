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

//! Aggregation result processing and format detection
//!
//! This module handles the conversion of Tantivy's JSON aggregation results into PostgreSQL
//! tuples. It supports two distinct JSON formats that Tantivy can produce:
//!
//! ## Formats
//!
//! ### Direct Format
//! Used for queries without FILTER clauses:
//! ```json
//! {
//!   "0": {"value": 42},           // First aggregate
//!   "1": {"value": 3.14},         // Second aggregate
//!   "grouped": {                  // GROUP BY buckets
//!     "buckets": [
//!       {"key": "category1", "doc_count": 10, "0": {...}, "1": {...}},
//!       {"key": "category2", "doc_count": 20, "0": {...}, "1": {...}}
//!     ]
//!   }
//! }
//! ```
//!
//! ### Filter Format
//! Used for queries with FILTER clauses (uses Tantivy's FilterAggregation):
//! ```json
//! {
//!   "filter_sentinel": {          // Base query results (all groups)
//!     "filtered_agg": {
//!       "grouped": {
//!         "buckets": [...]
//!       }
//!     }
//!   },
//!   "filter_0": {                  // First FILTER clause results
//!     "filtered_agg": {...}
//!   },
//!   "filter_1": {                  // Second FILTER clause results
//!     "filtered_agg": {...}
//!   }
//! }
//! ```

use super::tantivy_keys::{
    BUCKETS, DOC_COUNT, FILTERED_AGG, FILTER_PREFIX, FILTER_SENTINEL, GROUPED, HIDDEN_DOC_COUNT,
    KEY, SUM_OTHER_DOC_COUNT,
};
use crate::aggregate::agg_spec::AggregationSpec;
use crate::gucs;
use crate::postgres::customscan::aggregatescan::privdat::{
    AggregateResult, AggregateType, AggregateValue,
};
use crate::postgres::customscan::aggregatescan::scan_state::{
    AggregateRow, AggregateScanState, GroupedAggregateRow,
};
use crate::postgres::types::TantivyValue;
use crate::schema::SearchIndexSchema;
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{function_name, PgLogLevel, PgSqlErrorCode};
use tantivy::schema::OwnedValue;

/// Represents the format of aggregation results from Tantivy
#[derive(Debug, Clone, Copy)]
pub enum AggResult {
    /// Direct aggregation: {"0": {value}, "grouped": {...}}
    Direct,
    /// FilterAggregation: {"filter_sentinel": {...}, "filter_0": {...}}
    Filter,
}

impl AggResult {
    /// Process aggregation results according to this format
    ///
    /// Main entry point for converting Tantivy JSON results to PostgreSQL tuples
    pub fn process_results(
        state: &AggregateScanState,
        result: serde_json::Value,
    ) -> Vec<GroupedAggregateRow> {
        let format = AggResult::detect(&result, state.grouping_columns().is_empty());
        if state.grouping_columns().is_empty() {
            format.process_simple(state, result)
        } else {
            format.process_grouped(state, result)
        }
    }

    /// Detect which format Tantivy returned (Direct or Filter)
    fn detect(result: &serde_json::Value, is_simple: bool) -> Self {
        let obj = match result.as_object() {
            Some(obj) => obj,
            None => return Self::Direct, // Default for empty results
        };

        if is_simple {
            // Simple aggregation: check for "filter_" prefix
            if obj.keys().any(|k| k.starts_with(FILTER_PREFIX)) {
                Self::Filter
            } else {
                Self::Direct
            }
        } else {
            // Grouped aggregation: check for "filter_sentinel"
            if obj.contains_key(FILTER_SENTINEL) {
                Self::Filter
            } else {
                Self::Direct
            }
        }
    }

    /// Process simple aggregation results (no GROUP BY)
    fn process_simple(
        &self,
        state: &AggregateScanState,
        result: serde_json::Value,
    ) -> Vec<GroupedAggregateRow> {
        let result_obj = match result.as_object() {
            Some(obj) => obj,
            None => {
                // Handle null or empty results - return appropriate empty values
                let row = state
                    .aggregate_types()
                    .iter()
                    .map(|aggregate| aggregate.empty_value())
                    .collect::<AggregateRow>();
                return vec![GroupedAggregateRow {
                    group_keys: vec![],
                    aggregate_values: row,
                }];
            }
        };

        let aggregate_values = match self {
            Self::Direct => {
                // Extract _doc_count for NULL handling
                let doc_count = Self::extract_doc_count(result_obj);

                state
                    .aggregate_types()
                    .iter()
                    .enumerate()
                    .map(|(idx, aggregate)| {
                        result_obj
                            .get(&idx.to_string())
                            .map(|v| {
                                let agg_result = Self::extract_aggregate_value_from_json(v);
                                aggregate
                                    .result_from_aggregate_with_doc_count(agg_result, doc_count)
                            })
                            .unwrap_or_else(|| aggregate.empty_value())
                    })
                    .collect()
            }
            Self::Filter => {
                // Collect filter results
                let filter_results: Vec<(usize, &serde_json::Value)> = result_obj
                    .iter()
                    .filter_map(|(key, value)| {
                        key.strip_prefix(FILTER_PREFIX)
                            .and_then(|idx_str| idx_str.parse::<usize>().ok())
                            .map(|idx| (idx, value))
                    })
                    .collect();

                state
                    .aggregate_types()
                    .iter()
                    .enumerate()
                    .map(|(idx, aggregate)| {
                        filter_results
                            .iter()
                            .find(|(i, _)| *i == idx)
                            .map(|(_, filter_value)| {
                                Self::extract_filter_aggregate_value(filter_value, aggregate)
                            })
                            .unwrap_or_else(|| aggregate.empty_value())
                    })
                    .collect()
            }
        };

        vec![GroupedAggregateRow {
            group_keys: vec![],
            aggregate_values,
        }]
    }

    /// Process grouped aggregation results (with GROUP BY)
    fn process_grouped(
        &self,
        state: &AggregateScanState,
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

        let schema = state
            .indexrel()
            .schema()
            .expect("indexrel should have a schema");
        match self {
            Self::Direct => {
                // Direct format: {"grouped": {...}}
                let grouped = result_obj
                    .get(GROUPED)
                    .expect("Direct GROUP BY results should have grouped structure");

                Self::walk_grouped_buckets(
                    &state.agg_spec,
                    &schema,
                    grouped,
                    None,
                    0,
                    &mut Vec::new(),
                    &mut rows,
                );
            }
            Self::Filter => {
                // FilterAggregation format: {"filter_sentinel": {...}, "filter_0": {...}}
                let sentinel_grouped = result_obj
                    .get(FILTER_SENTINEL)
                    .and_then(|f| f.get(GROUPED))
                    .expect("filter_sentinel should have grouped structure");

                // Collect filter aggregation results for value lookup
                let filter_results: Vec<(usize, &serde_json::Value)> = result_obj
                    .iter()
                    .filter_map(|(key, value)| {
                        key.strip_prefix(FILTER_PREFIX)
                            .and_then(|idx_str| idx_str.parse::<usize>().ok())
                            .and_then(|idx| value.get(GROUPED).map(|grouped| (idx, grouped)))
                    })
                    .collect();

                Self::walk_grouped_buckets(
                    &state.agg_spec,
                    &schema,
                    sentinel_grouped,
                    Some(&filter_results),
                    0,
                    &mut Vec::new(),
                    &mut rows,
                );
            }
        }

        // Check for truncation
        if state.maybe_truncated && Self::was_truncated(&result) {
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

    /// Walk grouped buckets recursively, collecting group keys and aggregate values
    ///
    /// For Direct format: filter_results is None, aggregates are in the bucket
    /// For Filter format: filter_results contains lookup table for filtered aggregates
    fn walk_grouped_buckets(
        agg_spec: &AggregationSpec,
        schema: &SearchIndexSchema,
        grouped: &serde_json::Value,
        filter_results: Option<&[(usize, &serde_json::Value)]>,
        depth: usize,
        group_keys: &mut Vec<OwnedValue>,
        output_rows: &mut Vec<GroupedAggregateRow>,
    ) {
        let buckets = match grouped.get(BUCKETS).and_then(|b| b.as_array()) {
            Some(b) => b,
            None => return,
        };

        if depth >= agg_spec.groupby.len() {
            return;
        }

        let grouping_column = &agg_spec.groupby[depth];

        for bucket in buckets {
            let bucket_obj = bucket.as_object().expect("bucket should be object");

            // Extract and store group key
            let key_json = bucket_obj.get(KEY).expect("bucket should have key");
            let key_owned =
                Self::json_value_to_owned_value(schema, key_json, &grouping_column.field_name);
            group_keys.push(key_owned.clone());

            if depth + 1 == agg_spec.groupby.len() {
                // Leaf level - extract aggregate values
                let aggregate_values = AggResult::extract_aggregates_for_group(
                    agg_spec,
                    schema,
                    bucket_obj,
                    filter_results,
                    group_keys,
                );

                output_rows.push(GroupedAggregateRow {
                    group_keys: group_keys.clone(),
                    aggregate_values,
                });
            } else {
                // Recurse into nested grouped
                if let Some(nested_grouped) = bucket_obj.get(GROUPED) {
                    Self::walk_grouped_buckets(
                        agg_spec,
                        schema,
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

    /// Extract _doc_count from result object for NULL handling
    fn extract_doc_count(result_obj: &serde_json::Map<String, serde_json::Value>) -> Option<i64> {
        result_obj.get(HIDDEN_DOC_COUNT).and_then(|v| {
            let agg_result = Self::extract_aggregate_value_from_json(v);
            agg_result.extract_number().and_then(|n| {
                // Tantivy returns counts as floats, convert to i64
                n.as_f64().map(|f| f as i64)
            })
        })
    }

    /// Extract aggregate value from a FilterAggregation result
    fn extract_filter_aggregate_value(
        filter_value: &serde_json::Value,
        aggregate: &AggregateType,
    ) -> AggregateValue {
        let doc_count = filter_value.get(DOC_COUNT).and_then(|v| v.as_i64());

        // Look for the aggregate result in filtered_agg or directly
        let agg_result = if let Some(filtered_agg) = filter_value.get(FILTERED_AGG) {
            Self::extract_aggregate_value_from_json(filtered_agg)
        } else {
            // Fallback: extract directly (shouldn't normally happen)
            Self::extract_aggregate_value_from_json(filter_value)
        };

        aggregate.result_from_aggregate_with_doc_count(agg_result, doc_count)
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

    /// Extract aggregate values for a specific group
    ///
    /// For Direct format: extract from bucket_obj directly
    /// For Filter format: look up in filter_results using group_keys
    fn extract_aggregates_for_group(
        agg_spec: &AggregationSpec,
        schema: &SearchIndexSchema,
        bucket_obj: &serde_json::Map<String, serde_json::Value>,
        filter_results: Option<&[(usize, &serde_json::Value)]>,
        group_keys: &[OwnedValue],
    ) -> AggregateRow {
        if agg_spec.aggs.is_empty() {
            return AggregateRow::default();
        }

        match filter_results {
            None => {
                // Direct format: aggregates are in the bucket
                agg_spec
                    .aggs
                    .iter()
                    .enumerate()
                    .map(|(idx, aggregate)| {
                        bucket_obj
                            .get(&idx.to_string())
                            .map(|v| {
                                let agg_result = Self::extract_aggregate_value_from_json(v);
                                let doc_count = bucket_obj.get(DOC_COUNT).and_then(|v| v.as_i64());
                                aggregate
                                    .result_from_aggregate_with_doc_count(agg_result, doc_count)
                            })
                            .unwrap_or_else(|| aggregate.empty_value())
                    })
                    .collect()
            }
            Some(filter_results) => {
                // Filter format: look up aggregates in filter_results
                agg_spec
                    .aggs
                    .iter()
                    .enumerate()
                    .map(|(agg_idx, aggregate)| {
                        filter_results
                            .iter()
                            .find(|(idx, _)| *idx == agg_idx)
                            .and_then(|(_, filter_grouped)| {
                                Self::find_aggregate_value_in_filter(
                                    agg_spec,
                                    schema,
                                    filter_grouped,
                                    group_keys,
                                    0,
                                    aggregate,
                                )
                            })
                            .unwrap_or_else(|| aggregate.empty_value())
                    })
                    .collect()
            }
        }
    }

    /// Find aggregate value in filter result by matching group keys (iterative)
    /// This iterates through grouping levels to avoid stack overflow with deep nesting
    fn find_aggregate_value_in_filter(
        agg_spec: &AggregationSpec,
        schema: &SearchIndexSchema,
        mut grouped: &serde_json::Value,
        group_keys: &[OwnedValue],
        depth: usize,
        aggregate: &AggregateType,
    ) -> Option<AggregateValue> {
        // Iterate through each grouping level instead of recursing
        for level in depth..group_keys.len() {
            let buckets = grouped.get(BUCKETS)?.as_array()?;
            let target_key = &group_keys[level];
            let grouping_column = &agg_spec.groupby[level];

            // Find bucket matching this group key
            let matching_bucket = buckets.iter().find_map(|bucket| {
                let bucket_obj = bucket.as_object()?;
                let key_json = bucket_obj.get(KEY)?;

                // Convert JSON value to OwnedValue using schema
                let search_field = schema.search_field(&grouping_column.field_name);
                let bucket_key = TantivyValue::json_value_to_owned_value(&search_field, key_json);

                if bucket_key == *target_key {
                    Some(bucket_obj)
                } else {
                    None
                }
            })?;

            // If this is the last grouping level, extract the aggregate value
            if level + 1 == group_keys.len() {
                return Self::extract_aggregate_from_bucket(matching_bucket, aggregate);
            }

            // Move to nested grouped for next iteration
            grouped = matching_bucket.get(GROUPED)?;
        }

        None
    }

    /// Extract aggregate value from a bucket at the leaf level
    fn extract_aggregate_from_bucket(
        bucket: &serde_json::Map<String, serde_json::Value>,
        aggregate: &AggregateType,
    ) -> Option<AggregateValue> {
        use crate::aggregate::tantivy_keys::{AVG, MAX, MIN, SUM, VALUE};

        let doc_count = bucket.get(DOC_COUNT).and_then(|d| d.as_i64());

        // Look for the aggregate value - could be a numeric key or named sub-aggregation
        for (key, value) in bucket.iter() {
            if key.parse::<usize>().is_ok() || matches!(key.as_str(), VALUE | AVG | SUM | MIN | MAX)
            {
                let agg_result = Self::extract_aggregate_value_from_json(value);
                return Some(aggregate.result_from_aggregate_with_doc_count(agg_result, doc_count));
            }
        }

        // No aggregate found in this bucket
        None
    }

    fn was_truncated(result: &serde_json::Value) -> bool {
        result
            .as_object()
            .map(|obj| {
                obj.iter()
                    .filter_map(|(key, value)| {
                        if key.starts_with(GROUPED) {
                            value
                                .as_object()
                                .and_then(|group_obj| group_obj.get(SUM_OTHER_DOC_COUNT))
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

    /// Convert a JSON value to an OwnedValue based on the field type from the schema
    fn json_value_to_owned_value(
        schema: &SearchIndexSchema,
        json_value: &serde_json::Value,
        field_name: &str,
    ) -> OwnedValue {
        // Get the search field from the schema to determine the type
        let search_field = schema.search_field(field_name);
        TantivyValue::json_value_to_owned_value(&search_field, json_value)
    }
}
