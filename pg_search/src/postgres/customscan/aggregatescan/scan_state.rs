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
    // Filter groups for optimization (filter_expr, aggregate_indices)
    pub filter_groups: Vec<super::FilterGroup>,
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

            if has_sum {
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

    /// Unified result processing function - handles all aggregation result types
    /// This replaces the multiple specialized processing functions with a single unified approach
    pub fn process_aggregation_results(
        &self,
        result: serde_json::Value,
    ) -> Vec<GroupedAggregateRow> {
        if self.grouping_columns.is_empty() {
            // No GROUP BY - simple aggregation results
            self.process_simple_filter_aggregation_results(result)
        } else {
            // GROUP BY - process nested results (handles both filtered and non-filtered)
            self.process_grouped_filter_aggregation_results(result)
        }
    }

    /// Process simple filter aggregation results (no GROUP BY)
    fn process_simple_filter_aggregation_results(
        &self,
        result: serde_json::Value,
    ) -> Vec<GroupedAggregateRow> {
        let result_map = match result.as_object() {
            Some(obj) => obj,
            None => {
                // Handle null or empty results
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

        let mut aggregate_values = vec![AggregateValue::Null; self.aggregate_types.len()];

        // Process each result in the map
        for (key, value) in result_map {
            if key.starts_with("filter_") {
                // This is a filter aggregation result
                if let Some(filter_aggs) = value.get("aggs").and_then(|v| v.as_object()) {
                    for (agg_key, agg_result) in filter_aggs {
                        if let Ok(idx) = agg_key.parse::<usize>() {
                            if idx < self.aggregate_types.len() {
                                let aggregate = &self.aggregate_types[idx];
                                let agg_value = self.extract_aggregate_value_from_result(
                                    aggregate, agg_result, None,
                                );
                                aggregate_values[idx] = agg_value;
                            }
                        }
                    }
                }
            } else if let Ok(idx) = key.parse::<usize>() {
                // This is a direct (unfiltered) aggregate result
                if idx < self.aggregate_types.len() {
                    let aggregate = &self.aggregate_types[idx];

                    // Extract doc_count if available for empty result set detection
                    let doc_count = if let Some(agg_obj) = value.as_object() {
                        agg_obj.get("doc_count").and_then(|v| v.as_i64())
                    } else {
                        None
                    };

                    // Extract the aggregate result
                    let agg_result = Self::extract_aggregate_value_from_json(value);

                    // Use result_from_aggregate_with_doc_count for proper empty result handling
                    let agg_value =
                        aggregate.result_from_aggregate_with_doc_count(agg_result, doc_count);
                    aggregate_values[idx] = agg_value;
                }
            }
        }

        vec![GroupedAggregateRow {
            group_keys: vec![],
            aggregate_values: aggregate_values.into_iter().collect(),
        }]
    }

    /// Process grouped filter aggregation results (with GROUP BY)
    /// This now uses the unified approach that handles both filtered and non-filtered GROUP BY
    fn process_grouped_filter_aggregation_results(
        &self,
        result: serde_json::Value,
    ) -> Vec<GroupedAggregateRow> {
        // Use the existing comprehensive GROUP BY processing logic
        // This handles all GROUP BY scenarios: filtered, non-filtered, and mixed
        self.json_to_aggregate_results(result)
    }

    /// Extract aggregate value from a result (helper method)
    fn extract_aggregate_value_from_result(
        &self,
        aggregate: &AggregateType,
        result: &serde_json::Value,
        doc_count: Option<i64>,
    ) -> AggregateValue {
        let agg_result = Self::extract_aggregate_value_from_json(result);
        aggregate.result_from_aggregate_with_doc_count(agg_result, doc_count)
    }

    /// Transform FilterAggregation structure to regular group structure
    /// Converts from: {"filter_1": {"grouped": {"buckets": [...]}}}
    /// To: {"group_0": {"buckets": [...]}}
    fn transform_filter_aggregation_to_group_structure(
        &self,
        result_obj: &serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Value {
        // For optimized FilterAggregation queries, we expect only one filter key
        // since all FILTER clauses were identical and moved to the base query
        if let Some((_, filter_value)) = result_obj.iter().find(|(k, _)| k.starts_with("filter_")) {
            if let Some(grouped) = filter_value.get("grouped") {
                // Transform the nested "grouped" structure to "group_0", "group_1", etc.
                return Self::transform_nested_grouped_to_group_structure(grouped, 0);
            }
        }

        // Fallback: return empty structure
        serde_json::json!({})
    }

    /// Recursively transform nested "grouped" structures to "group_X" structures
    fn transform_nested_grouped_to_group_structure(
        grouped: &serde_json::Value,
        depth: usize,
    ) -> serde_json::Value {
        if let Some(buckets) = grouped.get("buckets").and_then(|b| b.as_array()) {
            let mut transformed_buckets = Vec::new();

            for bucket in buckets {
                if let Some(bucket_obj) = bucket.as_object() {
                    let mut new_bucket = bucket_obj.clone();

                    // If this bucket has a nested "grouped", transform it recursively
                    if let Some(nested_grouped) = bucket_obj.get("grouped") {
                        let transformed_nested = Self::transform_nested_grouped_to_group_structure(
                            nested_grouped,
                            depth + 1,
                        );
                        new_bucket.insert(format!("group_{}", depth + 1), transformed_nested);
                        new_bucket.remove("grouped");
                    }

                    transformed_buckets.push(serde_json::Value::Object(new_bucket));
                }
            }

            let mut result = serde_json::Map::new();
            result.insert(
                "buckets".to_string(),
                serde_json::Value::Array(transformed_buckets),
            );

            // Copy other properties from the original grouped object
            if let Some(grouped_obj) = grouped.as_object() {
                for (key, value) in grouped_obj {
                    if key != "buckets" {
                        result.insert(key.clone(), value.clone());
                    }
                }
            }

            return serde_json::json!({
                format!("group_{}", depth): serde_json::Value::Object(result)
            });
        }

        serde_json::json!({})
    }

    /// Extract bucket results from transformed FilterAggregation structure
    fn extract_bucket_results_from_transformed(
        &self,
        transformed_result: &serde_json::Value,
    ) -> Vec<GroupedAggregateRow> {
        let mut rows = Vec::new();
        self.extract_bucket_results(transformed_result, 0, &mut Vec::new(), &mut rows, None);
        rows
    }

    /// Recursively extract filtered aggregation buckets from nested grouped structures
    /// This handles multi-column GROUP BY where filtered aggregations have nested "grouped" objects
    fn extract_filtered_aggregation_buckets(
        grouped: &serde_json::Value,
        agg_idx: usize,
        current_keys: &mut Vec<String>,
        group_buckets: &mut std::collections::HashMap<
            String,
            (Vec<tantivy::schema::OwnedValue>, AggregateRow),
        >,
    ) {
        if let Some(buckets) = grouped.get("buckets").and_then(|b| b.as_array()) {
            for bucket in buckets {
                if let Some(bucket_obj) = bucket.as_object() {
                    // Extract the group key for this level
                    if let Some(key_value) = bucket_obj.get("key") {
                        let key_str = match key_value {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            _ => format!("{key_value:?}"),
                        };

                        current_keys.push(key_str);

                        // Check if this bucket has a nested "grouped" structure
                        if let Some(nested_grouped) = bucket_obj.get("grouped") {
                            // Recurse to the next level
                            Self::extract_filtered_aggregation_buckets(
                                nested_grouped,
                                agg_idx,
                                current_keys,
                                group_buckets,
                            );
                        } else {
                            // This is a leaf level - extract the aggregate value
                            let group_key_str = current_keys.join("|");

                            // Extract the aggregate value at the specific index
                            let agg_key = agg_idx.to_string();
                            let aggregate_value = if let Some(agg_obj) = bucket_obj.get(&agg_key) {
                                if let Some(value_obj) = agg_obj.as_object() {
                                    if let Some(value) = value_obj.get("value") {
                                        match value {
                                            serde_json::Value::Number(n) => {
                                                if let Some(i) = n.as_i64() {
                                                    AggregateValue::Int(i)
                                                } else if let Some(f) = n.as_f64() {
                                                    AggregateValue::Float(f)
                                                } else {
                                                    AggregateValue::Null
                                                }
                                            }
                                            _ => AggregateValue::Null,
                                        }
                                    } else {
                                        AggregateValue::Null
                                    }
                                } else {
                                    AggregateValue::Null
                                }
                            } else {
                                AggregateValue::Null
                            };

                            // Update the specific aggregate value in the existing group bucket
                            if let Some((_, aggregate_values)) =
                                group_buckets.get_mut(&group_key_str)
                            {
                                if agg_idx < aggregate_values.len() {
                                    aggregate_values[agg_idx] = aggregate_value;
                                }
                            }
                        }

                        current_keys.pop();
                    }
                }
            }
        }
    }

    /// Merge results from mixed FilterAggregation structure (both filter_X and group_0)
    fn merge_mixed_filter_aggregation_results(
        &self,
        result_obj: &serde_json::Map<String, serde_json::Value>,
    ) -> Vec<GroupedAggregateRow> {
        use std::collections::HashMap;

        // First, extract all group buckets from group_0 (non-filtered aggregations)
        let mut group_buckets: HashMap<String, (Vec<OwnedValue>, AggregateRow)> = HashMap::new();

        // Identify which aggregates are non-filtered (should be in group_0)
        let non_filtered_indices: Vec<usize> = self
            .aggregate_types
            .iter()
            .enumerate()
            .filter_map(|(idx, agg)| {
                if agg.filter_expr().is_none() {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();

        if let Some(group_0) = result_obj.get("group_0") {
            // Create a wrapper object with the expected structure for extract_bucket_results
            let wrapper = serde_json::json!({
                "group_0": group_0
            });
            let mut temp_rows = Vec::new();
            // Only extract non-filtered aggregates from group_0
            self.extract_bucket_results(
                &wrapper,
                0,
                &mut Vec::new(),
                &mut temp_rows,
                Some(&non_filtered_indices),
            );

            for (i, row) in temp_rows.iter().enumerate() {
                let group_key = row
                    .group_keys
                    .iter()
                    .map(|k| match k {
                        tantivy::schema::OwnedValue::Str(s) => s.clone(),
                        tantivy::schema::OwnedValue::I64(i) => i.to_string(),
                        tantivy::schema::OwnedValue::F64(f) => f.to_string(),
                        tantivy::schema::OwnedValue::Bool(b) => b.to_string(),
                        _ => format!("{k:?}"),
                    })
                    .collect::<Vec<_>>()
                    .join("|");

                // Create a full aggregate row with all positions initialized to Null
                let mut full_aggregate_values = AggregateRow::default();
                full_aggregate_values.resize(self.aggregate_types.len(), AggregateValue::Null);

                // Copy the extracted values to their correct positions
                for (extracted_idx, &aggregate_idx) in non_filtered_indices.iter().enumerate() {
                    if let Some(value) = row.aggregate_values.get(extracted_idx) {
                        if aggregate_idx < full_aggregate_values.len() {
                            full_aggregate_values[aggregate_idx] = value.clone();
                        }
                    }
                }

                group_buckets.insert(group_key, (row.group_keys.clone(), full_aggregate_values));
            }
        }

        // Then, process filtered aggregations and merge them with the base groups
        for (key, value) in result_obj {
            if key.starts_with("filter_") {
                // Extract the aggregate index from the key
                if let Ok(agg_idx) = key.strip_prefix("filter_").unwrap_or("").parse::<usize>() {
                    if agg_idx < self.aggregate_types.len() {
                        // Extract buckets from the filtered aggregation using recursive processing
                        if let Some(grouped) = value.get("grouped") {
                            Self::extract_filtered_aggregation_buckets(
                                grouped,
                                agg_idx,
                                &mut Vec::new(),
                                &mut group_buckets,
                            );
                        }
                    }
                }
            }
        }

        // Convert the merged results back to GroupedAggregateRow
        let final_results: Vec<GroupedAggregateRow> = group_buckets
            .into_iter()
            .map(|(_, (group_keys, aggregate_values))| GroupedAggregateRow {
                group_keys,
                aggregate_values,
            })
            .collect();

        final_results
    }

    /// Process results when only filtered aggregations are present (no group_0)
    fn process_filter_only_group_results(
        &self,
        result_obj: &serde_json::Map<String, serde_json::Value>,
    ) -> Vec<GroupedAggregateRow> {
        use std::collections::HashMap;

        let mut group_buckets: HashMap<String, (Vec<OwnedValue>, AggregateRow)> = HashMap::new();

        // Process each filtered aggregation
        for (key, value) in result_obj {
            if key.starts_with("filter_") {
                // Extract the aggregate index from the key
                if let Ok(agg_idx) = key.strip_prefix("filter_").unwrap_or("").parse::<usize>() {
                    if agg_idx < self.aggregate_types.len() {
                        // Extract buckets from the filtered aggregation
                        if let Some(grouped) = value.get("grouped") {
                            if let Some(buckets) = grouped.get("buckets").and_then(|b| b.as_array())
                            {
                                for bucket in buckets {
                                    if let Some(bucket_obj) = bucket.as_object() {
                                        // Extract the group key
                                        if let Some(key_value) = bucket_obj.get("key") {
                                            let group_key_str = format!("{key_value:?}");

                                            // Convert the key to OwnedValue for group_keys
                                            let group_key_owned = match key_value {
                                                serde_json::Value::String(s) => {
                                                    OwnedValue::Str(s.clone())
                                                }
                                                serde_json::Value::Number(n) => {
                                                    if let Some(i) = n.as_i64() {
                                                        OwnedValue::I64(i)
                                                    } else if let Some(f) = n.as_f64() {
                                                        OwnedValue::F64(f)
                                                    } else {
                                                        OwnedValue::Str(n.to_string())
                                                    }
                                                }
                                                _ => OwnedValue::Str(key_value.to_string()),
                                            };

                                            // Extract the aggregate value at the specific index
                                            let agg_key = agg_idx.to_string();
                                            let aggregate_value = if let Some(agg_obj) =
                                                bucket_obj.get(&agg_key)
                                            {
                                                if let Some(value_obj) = agg_obj.as_object() {
                                                    if let Some(value) = value_obj.get("value") {
                                                        match value {
                                                            serde_json::Value::Number(n) => {
                                                                if let Some(i) = n.as_i64() {
                                                                    AggregateValue::Int(i)
                                                                } else if let Some(f) = n.as_f64() {
                                                                    AggregateValue::Float(f)
                                                                } else {
                                                                    AggregateValue::Null
                                                                }
                                                            }
                                                            _ => AggregateValue::Null,
                                                        }
                                                    } else {
                                                        AggregateValue::Null
                                                    }
                                                } else {
                                                    AggregateValue::Null
                                                }
                                            } else {
                                                AggregateValue::Null
                                            };

                                            // Get or create the group bucket
                                            let (group_keys, aggregate_values) = group_buckets
                                                .entry(group_key_str.clone())
                                                .or_insert_with(|| {
                                                    // Create new group with NULL values for all aggregates
                                                    let mut agg_values = AggregateRow::default();
                                                    agg_values.resize(
                                                        self.aggregate_types.len(),
                                                        AggregateValue::Null,
                                                    );
                                                    (vec![group_key_owned.clone()], agg_values)
                                                });

                                            // Update the specific aggregate value
                                            if agg_idx < aggregate_values.len() {
                                                aggregate_values[agg_idx] = aggregate_value;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Convert the results to GroupedAggregateRow
        group_buckets
            .into_iter()
            .map(|(_, (group_keys, aggregate_values))| GroupedAggregateRow {
                group_keys,
                aggregate_values,
            })
            .collect()
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

        // Check if this is a FilterAggregation structure
        if let Some(result_obj) = result.as_object() {
            let has_filter_keys = result_obj.keys().any(|k| k.starts_with("filter_"));
            let has_group_key = result_obj.contains_key("group_0");

            // Check if this is a simple FilterAggregation (numeric keys with doc_count)
            let has_simple_filter_agg = !has_filter_keys
                && !has_group_key
                && self.grouping_columns.is_empty()
                && result_obj.keys().all(|k| k.parse::<usize>().is_ok())
                && result_obj.values().any(|v| {
                    v.as_object()
                        .is_some_and(|obj| obj.contains_key("doc_count"))
                });

            if has_filter_keys && has_group_key {
                // This is a mixed FilterAggregation structure - merge the results
                return self.merge_mixed_filter_aggregation_results(result_obj);
            } else if has_filter_keys && !has_group_key {
                // Check if this is a multi-column GROUP BY FilterAggregation
                // (has nested "grouped" structures)
                let has_nested_grouped = result_obj.values().any(|v| {
                    v.get("grouped")
                        .and_then(|g| g.get("buckets"))
                        .and_then(|b| b.as_array())
                        .is_some_and(|buckets| {
                            buckets.iter().any(|bucket| {
                                bucket
                                    .as_object()
                                    .is_some_and(|obj| obj.contains_key("grouped"))
                            })
                        })
                });

                if has_nested_grouped && self.grouping_columns.len() > 1 {
                    // Multi-column GROUP BY FilterAggregation - transform and use regular processing
                    let transformed_result =
                        self.transform_filter_aggregation_to_group_structure(result_obj);
                    return self.extract_bucket_results_from_transformed(&transformed_result);
                } else {
                    // Single-column GROUP BY FilterAggregation - use filter-only processing
                    return self.process_filter_only_group_results(result_obj);
                }
            } else if has_simple_filter_agg {
                // Simple FilterAggregation (no GROUP BY) with numeric keys
                return self.process_simple_filter_aggregation_results(result.clone());
            } else if !has_filter_keys && has_group_key {
                // Only non-filtered GROUP BY - use regular processing
                // Fall through to regular extract_bucket_results
            }
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
                } else if let Some(agg_obj) = bucket_obj.get(&agg_idx.to_string()) {
                    // For optimized queries, aggregates might use numeric keys ("0", "1", etc.)
                    // instead of "agg_X" format
                    Self::extract_aggregate_value_from_json(agg_obj)
                } else {
                    // For optimized queries, COUNT aggregates might not have agg_X entries
                    // because they use doc_count instead. Check if this is a COUNT aggregate.
                    match aggregate {
                        AggregateType::CountAny { .. } => {
                            // Use doc_count for COUNT aggregates in optimized queries
                            if let Some(doc_count_obj) = bucket_obj.get("doc_count") {
                                Self::extract_aggregate_value_from_json(doc_count_obj)
                            } else {
                                AggregateResult::Null
                            }
                        }
                        _ => AggregateResult::Null,
                    }
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
                                        // Non-count aggregate - look for both agg_X and numeric keys
                                        let agg_key = format!("agg_{group_agg_idx}");
                                        let numeric_key = original_agg_idx.to_string();

                                        if let Some(agg_obj) = bucket_obj.get(&agg_key) {
                                            let agg_result =
                                                Self::extract_aggregate_value_from_json(agg_obj);
                                            aggregate.result_from_aggregate_with_doc_count(
                                                agg_result,
                                                Some(doc_count),
                                            )
                                        } else if let Some(agg_obj) = bucket_obj.get(&numeric_key) {
                                            // For optimized queries, aggregates might use numeric keys
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
