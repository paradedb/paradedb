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

        // Process each filter aggregation result (ALL aggregates now use filter_X keys)
        for (key, value) in result_map {
            if key.starts_with("filter_") {
                // Extract the aggregate index from the filter key
                if let Ok(idx) = key.strip_prefix("filter_").unwrap_or("").parse::<usize>() {
                    if idx < self.aggregate_types.len() {
                        let aggregate = &self.aggregate_types[idx];

                        // Extract doc_count from the FilterAggregation result
                        let doc_count = value.get("doc_count").and_then(|v| v.as_i64());

                        // Extract the aggregate result from the filtered_agg sub-aggregation
                        let agg_result = if let Some(filtered_agg) = value.get("filtered_agg") {
                            Self::extract_aggregate_value_from_json(filtered_agg)
                        } else {
                            // Fallback: try to extract directly from the filter result
                            Self::extract_aggregate_value_from_json(value)
                        };

                        // Use result_from_aggregate_with_doc_count for proper empty result handling
                        let agg_value =
                            aggregate.result_from_aggregate_with_doc_count(agg_result, doc_count);
                        aggregate_values[idx] = agg_value;
                    }
                }
            }
        }

        vec![GroupedAggregateRow {
            group_keys: vec![],
            aggregate_values: aggregate_values.into_iter().collect(),
        }]
    }

    // ====================================================================================
    // TRANSFORMATION FUNCTIONS FOR GROUP BY WITH FILTER AGGREGATIONS
    // These handle the filter_X -> grouped -> buckets structure generated by GROUP BY queries
    // ====================================================================================

    /// Transform FilterAggregation structure to regular group structure
    /// Converts from: {"filter_0": {"grouped": {...}}, "filter_1": {"grouped": {...}}}
    /// To: {"group_0": {"buckets": [...]}} with merged aggregate values
    fn transform_filter_aggregation_to_group_structure(
        &self,
        result_obj: &serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Value {
        // For multi-column GROUP BY FilterAggregation, we need to merge results from all filter keys
        // The structure is: filter_X -> grouped -> buckets -> [category buckets] -> grouped -> buckets -> [brand buckets] -> aggregate values

        // Find the filter with the most comprehensive bucket set as the base structure
        // Prefer the sentinel filter if present, as it has ALL groups
        let mut best_filter = None;
        let mut max_bucket_count = 0;

        // First, check for the sentinel filter (has all groups)
        if let Some(sentinel_value) = result_obj.get("filter_sentinel") {
            if let Some(grouped) = sentinel_value.get("grouped") {
                best_filter = Some(("filter_sentinel", sentinel_value));
                max_bucket_count = Self::count_total_buckets(grouped);
            }
        }

        // If no sentinel, find the filter with the most buckets
        if best_filter.is_none() {
            for (key, value) in result_obj.iter() {
                if key.starts_with("filter_") {
                    if let Some(grouped) = value.get("grouped") {
                        let bucket_count = Self::count_total_buckets(grouped);
                        if bucket_count > max_bucket_count {
                            max_bucket_count = bucket_count;
                            best_filter = Some((key, value));
                        }
                    }
                }
            }
        }

        if let Some((_, best_value)) = best_filter {
            if let Some(grouped) = best_value.get("grouped") {
                // Clone the most comprehensive structure as base
                let mut base_structure = grouped.clone();

                // Merge aggregate values from all filter keys (including the base one)
                self.merge_filter_aggregates_into_structure(&mut base_structure, result_obj);

                // Transform to the expected group_0 structure
                return Self::transform_nested_grouped_to_group_structure(&base_structure, 0);
            }
        }

        // Fallback: return empty structure
        serde_json::json!({})
    }

    /// Count total number of buckets in a nested grouped structure (iteratively)
    /// This helps identify which filter has the most comprehensive bucket set
    fn count_total_buckets(grouped: &serde_json::Value) -> usize {
        let mut stack = vec![grouped];
        let mut count = 0;

        while let Some(current) = stack.pop() {
            if let Some(buckets) = current.get("buckets").and_then(|b| b.as_array()) {
                count += buckets.len();
                // Push nested grouped structures onto stack for processing
                for bucket in buckets {
                    if let Some(nested) = bucket.get("grouped") {
                        stack.push(nested);
                    }
                }
            }
        }

        count
    }

    /// Merge aggregate values from all filter keys into the base structure
    fn merge_filter_aggregates_into_structure(
        &self,
        base_structure: &mut serde_json::Value,
        result_obj: &serde_json::Map<String, serde_json::Value>,
    ) {
        // Process each filter aggregation and merge its values
        for (key, filter_value) in result_obj.iter() {
            if key.starts_with("filter_") {
                if let Ok(agg_idx) = key.strip_prefix("filter_").unwrap_or("").parse::<usize>() {
                    if let Some(filter_grouped) = filter_value.get("grouped") {
                        Self::merge_aggregates_recursively(
                            base_structure,
                            filter_grouped,
                            agg_idx,
                            0,
                        );
                    }
                }
            }
        }
    }

    /// Recursively merge aggregate values at each level of the nested structure
    fn merge_aggregates_recursively(
        base_buckets: &mut serde_json::Value,
        filter_buckets: &serde_json::Value,
        agg_idx: usize,
        _depth: usize,
    ) {
        if let (Some(base_array), Some(filter_array)) = (
            base_buckets
                .get_mut("buckets")
                .and_then(|b| b.as_array_mut()),
            filter_buckets.get("buckets").and_then(|b| b.as_array()),
        ) {
            // Match buckets by their "key" field and merge aggregate values
            for base_bucket in base_array.iter_mut() {
                if let Some(base_key) = base_bucket.get("key") {
                    // Find the corresponding bucket in the filter aggregation
                    for filter_bucket in filter_array {
                        if let Some(filter_key) = filter_bucket.get("key") {
                            if base_key == filter_key {
                                // Found matching bucket - merge the aggregate value
                                if let Some(agg_value) = filter_bucket.get(agg_idx.to_string()) {
                                    if let Some(base_obj) = base_bucket.as_object_mut() {
                                        base_obj.insert(agg_idx.to_string(), agg_value.clone());
                                    }
                                }

                                // If there are nested grouped structures, recurse
                                if let (Some(base_nested), Some(filter_nested)) =
                                    (base_bucket.get_mut("grouped"), filter_bucket.get("grouped"))
                                {
                                    Self::merge_aggregates_recursively(
                                        base_nested,
                                        filter_nested,
                                        agg_idx,
                                        _depth + 1,
                                    );
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
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
                        // The transformed_nested already contains the group_X wrapper, so extract the inner content
                        if let Some(inner_content) =
                            transformed_nested.get(format!("group_{}", depth + 1))
                        {
                            new_bucket
                                .insert(format!("group_{}", depth + 1), inner_content.clone());
                        }
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

        // All GROUP BY queries use FilterAggregation structure
        let result_obj = result
            .as_object()
            .expect("GROUP BY results should be an object");

        // Transform filter_X -> grouped structure to group_0 -> buckets for processing
        let transformed = self.transform_filter_aggregation_to_group_structure(result_obj);
        let rows = self.extract_bucket_results_from_transformed(&transformed);

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

    /// Extract aggregate value from a bucket (handles both filter_X and numeric keys)
    /// This handles:
    /// 1. Untransformed structure: filter_X keys with FilterAggregation wrapper
    /// 2. Transformed structure: numeric keys (0, 1, 2, ...) after transformation
    fn extract_filter_aggregate_from_bucket(
        &self,
        bucket_obj: &serde_json::Map<String, serde_json::Value>,
        idx: usize,
        aggregate: &AggregateType,
    ) -> AggregateValue {
        let filter_key = format!("filter_{idx}");
        let numeric_key = idx.to_string();

        // Try filter_X key first (untransformed structure)
        if let Some(filter_obj) = bucket_obj.get(&filter_key) {
            // Extract doc_count from FilterAggregation result
            let doc_count = filter_obj.get("doc_count").and_then(|d| d.as_i64());

            // Extract the aggregate result from filtered_agg sub-aggregation
            let agg_result = filter_obj
                .get("filtered_agg")
                .map(Self::extract_aggregate_value_from_json)
                .unwrap_or(AggregateResult::Null);

            return aggregate.result_from_aggregate_with_doc_count(agg_result, doc_count);
        }

        // Try numeric key (transformed structure)
        if let Some(agg_obj) = bucket_obj.get(&numeric_key) {
            // Extract doc_count from bucket
            let doc_count = bucket_obj.get("doc_count").and_then(|d| d.as_i64());

            // Extract the aggregate result directly
            let agg_result = Self::extract_aggregate_value_from_json(agg_obj);

            return aggregate.result_from_aggregate_with_doc_count(agg_result, doc_count);
        }

        // No aggregate found - this means the filter didn't match for this bucket
        // Return appropriate empty value based on aggregate type
        if matches!(
            aggregate,
            AggregateType::CountAny { .. } | AggregateType::Count { .. }
        ) {
            // COUNT(*) and COUNT(field) with no matches return 0
            AggregateValue::Int(0)
        } else {
            // All other aggregates (SUM, AVG, MIN, MAX) return NULL
            AggregateValue::Null
        }
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
                            // Extract aggregates from filter_X keys at leaf level
                            self.aggregate_types
                                .iter()
                                .enumerate()
                                .map(|(idx, aggregate)| {
                                    self.extract_filter_aggregate_from_bucket(
                                        bucket_obj, idx, aggregate,
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
