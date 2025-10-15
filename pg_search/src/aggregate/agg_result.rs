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
use crate::postgres::customscan::aggregatescan::privdat::{
    AggregateResult, AggregateType, AggregateValue,
};
use crate::postgres::customscan::aggregatescan::scan_state::{
    AggregateRow, AggregateScanState, GroupedAggregateRow,
};
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{function_name, PgLogLevel, PgSqlErrorCode};

/// Represents the format of aggregation results from Tantivy
#[derive(Debug, Clone, Copy)]
pub enum AggResult {
    /// Direct aggregation: {"0": {value}, "grouped": {...}}
    Direct,
    /// FilterAggregation: {"filter_sentinel": {...}, "filter_0": {...}}
    Filter,
}

impl AggResult {
    pub fn detect(result: &serde_json::Value, is_simple: bool) -> Self {
        let obj = match result.as_object() {
            Some(obj) => obj,
            None => return Self::Direct, // Default for empty results
        };

        if is_simple {
            // Simple aggregation: check for "filter_" prefix
            if obj.keys().any(|k| k.starts_with("filter_")) {
                Self::Filter
            } else {
                Self::Direct
            }
        } else {
            // Grouped aggregation: check for "filter_sentinel"
            if obj.contains_key("filter_sentinel") {
                Self::Filter
            } else {
                Self::Direct
            }
        }
    }

    /// Process aggregation results according to this format
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
                        key.strip_prefix("filter_")
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

        match self {
            Self::Direct => {
                // Direct format: {"grouped": {...}}
                let grouped = result_obj
                    .get("grouped")
                    .expect("Direct GROUP BY results should have grouped structure");

                state.walk_grouped_buckets(grouped, None, 0, &mut Vec::new(), &mut rows);
            }
            Self::Filter => {
                // FilterAggregation format: {"filter_sentinel": {...}, "filter_0": {...}}
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

                state.walk_grouped_buckets(
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

    /// Extract _doc_count from result object for NULL handling
    fn extract_doc_count(result_obj: &serde_json::Map<String, serde_json::Value>) -> Option<i64> {
        result_obj.get("_doc_count").and_then(|v| {
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

    /// Extract aggregate value from JSON using serde deserialization
    /// This handles different structures: direct values, objects with "value" field, or raw objects for COUNT
    pub fn extract_aggregate_value_from_json(agg_obj: &serde_json::Value) -> AggregateResult {
        // Deserialize using our structured type
        match serde_json::from_value::<AggregateResult>(agg_obj.clone()) {
            Ok(result) => result,
            Err(e) => {
                panic!("Failed to deserialize aggregate result: {e}, value: {agg_obj:?}");
            }
        }
    }

    fn was_truncated(result: &serde_json::Value) -> bool {
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
