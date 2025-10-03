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
        let mut rows = if self.grouping_columns.is_empty() {
            // No GROUP BY - simple aggregation results
            self.process_simple_filter_aggregation_results(result)
        } else {
            // GROUP BY - process nested results (handles both filtered and non-filtered)
            self.process_grouped_aggregation_results(result)
        };

        // Apply ORDER BY sorting if we have grouping columns and ordering info
        if !self.grouping_columns.is_empty() && !self.orderby_info.is_empty() {
            self.sort_grouped_results(&mut rows);
        }

        rows
    }

    /// Sort grouped results according to ORDER BY clauses
    /// This handles ordering at the result processing level since our unified FilterAggregation
    /// approach doesn't have access to ordering info at the Tantivy aggregation building level
    fn sort_grouped_results(&self, rows: &mut [GroupedAggregateRow]) {
        use crate::api::{OrderByFeature, SortDirection};

        // Pre-compute sort key indices to avoid repeated lookups during comparison
        let sort_key_indices: Vec<(usize, SortDirection)> = self
            .orderby_info
            .iter()
            .filter_map(|orderby| {
                if let OrderByFeature::Field(field_name) = &orderby.feature {
                    self.grouping_columns
                        .iter()
                        .position(|col| {
                            crate::api::FieldName::from(col.field_name.clone()) == *field_name
                        })
                        .map(|col_idx| (col_idx, orderby.direction))
                } else {
                    None
                }
            })
            .collect();

        // Use optimized comparison with pre-computed indices
        rows.sort_by(|a, b| {
            for &(col_idx, direction) in &sort_key_indices {
                if col_idx < a.group_keys.len() && col_idx < b.group_keys.len() {
                    let cmp = TantivyValue::partial_cmp_extended(
                        &a.group_keys[col_idx],
                        &b.group_keys[col_idx],
                    )
                    .expect("group keys should be comparable");

                    let final_cmp = match direction {
                        SortDirection::Asc => cmp,
                        SortDirection::Desc => cmp.reverse(),
                    };

                    if final_cmp != std::cmp::Ordering::Equal {
                        return final_cmp;
                    }
                }
            }
            std::cmp::Ordering::Equal
        });
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
                    .aggregate_types
                    .iter()
                    .map(|aggregate| aggregate.empty_value())
                    .collect::<AggregateRow>();
                return vec![GroupedAggregateRow {
                    group_keys: vec![],
                    aggregate_values: row,
                }];
            }
        };

        // Collect filter results for lookup
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
            .aggregate_types
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
    /// All GROUP BY queries use: filter_sentinel/filter_X -> grouped -> buckets
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

        // Extract sentinel's grouped structure (has all groups)
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
        let mut rows = Vec::new();
        self.walk_grouped_buckets(
            sentinel_grouped,
            &filter_results,
            0,
            &mut Vec::new(),
            &mut rows,
        );

        // Check for truncation
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

    /// Walk grouped buckets recursively, collecting group keys and looking up aggregate values
    fn walk_grouped_buckets(
        &self,
        grouped: &serde_json::Value,
        filter_results: &[(usize, &serde_json::Value)],
        depth: usize,
        group_keys: &mut Vec<OwnedValue>,
        output_rows: &mut Vec<GroupedAggregateRow>,
    ) {
        let buckets = match grouped.get("buckets").and_then(|b| b.as_array()) {
            Some(b) => b,
            None => return,
        };

        if depth >= self.grouping_columns.len() {
            return;
        }

        let grouping_column = &self.grouping_columns[depth];

        for bucket in buckets {
            let bucket_obj = bucket.as_object().expect("bucket should be object");

            // Extract and store group key
            let key_json = bucket_obj.get("key").expect("bucket should have key");
            let key_owned = self.json_value_to_owned_value(key_json, &grouping_column.field_name);
            group_keys.push(key_owned.clone());

            if depth + 1 == self.grouping_columns.len() {
                // Leaf level - extract aggregate values
                let aggregate_values =
                    self.extract_aggregates_for_group(filter_results, group_keys);

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
        if self.aggregate_types.is_empty() {
            return AggregateRow::default();
        }

        self.aggregate_types
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
            let grouping_column = &self.grouping_columns[level];

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
        matches!(
            TantivyValue::partial_cmp_extended(&bucket_key, target_key),
            Some(std::cmp::Ordering::Equal)
        )
    }

    /// Extract aggregate value from a bucket at the leaf level
    #[inline]
    fn extract_aggregate_from_bucket(
        &self,
        bucket: &serde_json::Map<String, serde_json::Value>,
        aggregate: &AggregateType,
    ) -> Option<AggregateValue> {
        let doc_count = bucket.get("doc_count").and_then(|d| d.as_i64());

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
}

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        // TODO: Unused currently. See the comment on `trait CustomScanState` regarding making this
        // more useful.
    }
}
