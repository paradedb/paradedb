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

    /// Generate filter aggregation JSON using Tantivy's Filter Aggregation feature
    /// This replaces the complex multi-query approach with a single query containing filter aggregations
    pub fn aggregates_to_filter_aggregation_json(&self) -> serde_json::Value {
        // Group aggregates by their filter expressions
        let mut filter_groups: HashMap<String, Vec<(usize, &AggregateType)>> = HashMap::default();

        for (idx, aggregate) in self.aggregate_types.iter().enumerate() {
            let (filter_key, _, _) = aggregate.to_filter_aggregation_json(idx);
            filter_groups
                .entry(filter_key)
                .or_default()
                .push((idx, aggregate));
        }

        if self.grouping_columns.is_empty() {
            // No GROUP BY - simple filter aggregations
            self.build_simple_filter_aggregations(&filter_groups)
        } else {
            // GROUP BY with filter aggregations
            self.build_grouped_filter_aggregations(&filter_groups)
        }
    }

    /// Build simple filter aggregations (no GROUP BY)
    fn build_simple_filter_aggregations(
        &self,
        filter_groups: &HashMap<String, Vec<(usize, &AggregateType)>>,
    ) -> serde_json::Value {
        let mut agg_map = serde_json::Map::new();

        for (filter_key, aggregates) in filter_groups {
            if filter_key == "no_filter" {
                // Unfiltered aggregates - add directly to root
                for (idx, aggregate) in aggregates {
                    agg_map.insert(
                        idx.to_string(),
                        aggregate
                            .convert_filtered_aggregate_to_unfiltered()
                            .to_json(),
                    );
                }
            } else {
                // Filtered aggregates - wrap in filter aggregation
                let mut filter_aggs = serde_json::Map::new();
                for (idx, aggregate) in aggregates {
                    if let Some(filter_expr) = aggregate.filter_expr() {
                        filter_aggs.insert(
                            idx.to_string(),
                            aggregate
                                .convert_filtered_aggregate_to_unfiltered()
                                .to_json(),
                        );
                    }
                }

                if !filter_aggs.is_empty() {
                    // Get the filter query from the first aggregate in this group
                    if let Some((_, first_agg)) = aggregates.first() {
                        if let Some(filter_expr) = first_agg.filter_expr() {
                            // Extract the actual query string for Tantivy Filter Aggregation
                            let filter_query_string =
                                self.extract_query_string_from_search_input(&filter_expr);
                            agg_map.insert(
                                filter_key.clone(),
                                serde_json::json!({
                                    "filter": filter_query_string,
                                    "aggs": filter_aggs
                                }),
                            );
                        }
                    }
                }
            }
        }

        // Add document count if needed for SUM aggregates
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

        serde_json::Value::Object(agg_map)
    }

    /// Build grouped filter aggregations (with GROUP BY)
    fn build_grouped_filter_aggregations(
        &self,
        filter_groups: &HashMap<String, Vec<(usize, &AggregateType)>>,
    ) -> serde_json::Value {
        // Build the nested GROUP BY structure, but with filter aggregations at the leaf level
        let mut current_aggs: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
        let max_term_agg_buckets = gucs::max_term_agg_buckets() as u32;

        for (i, group_col) in self.grouping_columns.iter().enumerate().rev() {
            let mut terms = serde_json::Map::new();
            terms.insert(
                "field".to_string(),
                serde_json::Value::String(group_col.field_name.clone()),
            );

            // Add ordering and size configuration (same as original)
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
                // Deepest level – attach filter aggregations instead of simple aggregations
                if !self.aggregate_types.is_empty() {
                    let filter_aggs = self.build_simple_filter_aggregations(filter_groups);
                    if let serde_json::Value::Object(filter_aggs_map) = filter_aggs {
                        if !filter_aggs_map.is_empty() {
                            terms_agg.insert(
                                "aggs".to_string(),
                                serde_json::Value::Object(filter_aggs_map),
                            );
                        }
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

    /// Extract the query string from a SearchQueryInput for use in Tantivy Filter Aggregation
    /// This method recursively extracts the actual query string from nested SearchQueryInput structures
    fn extract_query_string_from_search_input(&self, query: &SearchQueryInput) -> String {
        use crate::query::SearchQueryInput;

        match query {
            SearchQueryInput::Parse { query_string, .. } => query_string.clone(),
            SearchQueryInput::WithIndex { query, .. } => {
                self.extract_query_string_from_search_input(query)
            }
            SearchQueryInput::FieldedQuery {
                field,
                query: pdb_query,
            } => {
                // For fielded queries, we need to construct a field:query string
                // Extract the actual query string from the pdb::Query
                match pdb_query {
                    crate::query::pdb_query::pdb::Query::UnclassifiedString { string, .. } => {
                        format!("{}:{}", field.root(), string)
                    }
                    crate::query::pdb_query::pdb::Query::ParseWithField {
                        query_string, ..
                    } => {
                        format!("{}:{}", field.root(), query_string)
                    }
                    crate::query::pdb_query::pdb::Query::Term { value, .. } => {
                        format!("{}:{:?}", field.root(), value)
                    }
                    crate::query::pdb_query::pdb::Query::Phrase { phrases, .. } => {
                        format!("{}:\"{}\"", field.root(), phrases.join(" "))
                    }
                    crate::query::pdb_query::pdb::Query::FuzzyTerm { value, .. } => {
                        format!("{}:{}~", field.root(), value)
                    }
                    crate::query::pdb_query::pdb::Query::Regex { pattern } => {
                        format!("{}:/{}/", field.root(), pattern)
                    }
                    _ => {
                        // For other complex pdb::Query types, use a simple fallback
                        // This should cover most common cases
                        format!("{}:*", field.root())
                    }
                }
            }
            SearchQueryInput::Boolean {
                must,
                should,
                must_not,
            } => {
                // For boolean queries, construct a boolean query string
                let mut parts = Vec::new();

                for must_query in must {
                    parts.push(format!(
                        "+{}",
                        self.extract_query_string_from_search_input(must_query)
                    ));
                }

                for should_query in should {
                    parts.push(self.extract_query_string_from_search_input(should_query));
                }

                for must_not_query in must_not {
                    parts.push(format!(
                        "-{}",
                        self.extract_query_string_from_search_input(must_not_query)
                    ));
                }

                parts.join(" ")
            }
            SearchQueryInput::All => "*".to_string(),
            SearchQueryInput::Empty => "".to_string(),
            _ => {
                // For other query types, fall back to serialization
                // This might not work perfectly but provides a fallback
                let serialized = query.serialize_and_clean_query();
                if serialized.is_empty() || serialized == "Error" {
                    "*".to_string()
                } else {
                    serialized
                }
            }
        }
    }

    /// Process results from filter aggregation queries
    /// This method handles the new filter aggregation result format
    pub fn json_to_filter_aggregation_results(
        &self,
        result: serde_json::Value,
    ) -> Vec<GroupedAggregateRow> {
        if self.grouping_columns.is_empty() {
            // No GROUP BY - simple filter aggregation results
            self.process_simple_filter_aggregation_results(result)
        } else {
            // GROUP BY with filter aggregations - process nested results
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
                    pgrx::warning!("FilterAggregation processing aggregate {}: {:?} with doc_count: {:?}", idx, aggregate, doc_count);
                    let agg_value = aggregate.result_from_aggregate_with_doc_count(agg_result, doc_count);
                    pgrx::warning!("FilterAggregation result for aggregate {}: {:?}", idx, agg_value);
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
    fn process_grouped_filter_aggregation_results(
        &self,
        result: serde_json::Value,
    ) -> Vec<GroupedAggregateRow> {
        let mut rows = Vec::new();

        // Handle empty results for GROUP BY queries
        if result.is_null() || (result.is_object() && result.as_object().unwrap().is_empty()) {
            return rows;
        }

        // Extract bucket results recursively, but handle filter aggregations at leaf level
        self.extract_filter_aggregation_bucket_results(
            &result,
            0,
            &mut Vec::new(),
            &mut rows,
            None,
        );

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

    /// Extract bucket results for filter aggregations (similar to extract_bucket_results but handles filter aggs)
    fn extract_filter_aggregation_bucket_results(
        &self,
        result: &serde_json::Value,
        depth: usize,
        current_keys: &mut Vec<OwnedValue>,
        rows: &mut Vec<GroupedAggregateRow>,
        doc_count: Option<i64>,
    ) {
        if depth >= self.grouping_columns.len() {
            // We've reached the leaf level - process filter aggregations
            let mut aggregate_values = vec![AggregateValue::Null; self.aggregate_types.len()];

            if let Some(result_obj) = result.as_object() {
                // Process filter aggregations and direct aggregations
                for (key, value) in result_obj {
                    if key.starts_with("filter_") {
                        // This is a filter aggregation result
                        if let Some(filter_aggs) = value.get("aggs").and_then(|v| v.as_object()) {
                            for (agg_key, agg_result) in filter_aggs {
                                if let Ok(idx) = agg_key.parse::<usize>() {
                                    if idx < self.aggregate_types.len() {
                                        let aggregate = &self.aggregate_types[idx];
                                        let agg_value = self.extract_aggregate_value_from_result(
                                            aggregate, agg_result, doc_count,
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
                            let agg_value = self
                                .extract_aggregate_value_from_result(aggregate, value, doc_count);
                            aggregate_values[idx] = agg_value;
                        }
                    }
                }
            }

            rows.push(GroupedAggregateRow {
                group_keys: current_keys.clone(),
                aggregate_values: aggregate_values.into_iter().collect(),
            });
            return;
        }

        // Navigate to the next grouping level
        let group_key = format!("group_{depth}");
        if let Some(group_obj) = result.get(&group_key).and_then(|v| v.as_object()) {
            if let Some(buckets) = group_obj.get("buckets").and_then(|v| v.as_array()) {
                for bucket in buckets {
                    if let Some(bucket_obj) = bucket.as_object() {
                        // Extract the key for this bucket
                        if let Some(key_value) = bucket_obj.get("key") {
                            let field_name = &self.grouping_columns[depth].field_name;
                            let owned_key = self.json_value_to_owned_value(key_value, field_name);
                            current_keys.push(owned_key);

                            let bucket_doc_count =
                                bucket_obj.get("doc_count").and_then(|v| v.as_i64());

                            // Recurse to the next level
                            self.extract_filter_aggregation_bucket_results(
                                bucket,
                                depth + 1,
                                current_keys,
                                rows,
                                bucket_doc_count,
                            );

                            current_keys.pop();
                        }
                    }
                }
            }
        }
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

    /// Merge results from mixed FilterAggregation structure (both filter_X and group_0)
    fn merge_mixed_filter_aggregation_results(
        &self,
        result_obj: &serde_json::Map<String, serde_json::Value>,
    ) -> Vec<GroupedAggregateRow> {
        use std::collections::HashMap;

        // First, extract all group buckets from group_0 (non-filtered aggregations)
        let mut group_buckets: HashMap<String, (Vec<OwnedValue>, AggregateRow)> = HashMap::new();

        // Identify which aggregates are non-filtered (should be in group_0)
        let non_filtered_indices: Vec<usize> = self.aggregate_types
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

        pgrx::warning!("=== Processing group_0 ===");
        pgrx::warning!("Non-filtered aggregate indices: {:?}", non_filtered_indices);
        if let Some(group_0) = result_obj.get("group_0") {
            pgrx::warning!("Raw group_0 JSON: {}", serde_json::to_string_pretty(group_0).unwrap_or_else(|e| format!("Failed to serialize: {}", e)));
            // Create a wrapper object with the expected structure for extract_bucket_results
            let wrapper = serde_json::json!({
                "group_0": group_0
            });
            let mut temp_rows = Vec::new();
            // Only extract non-filtered aggregates from group_0
            self.extract_bucket_results(&wrapper, 0, &mut Vec::new(), &mut temp_rows, Some(&non_filtered_indices));
            
            pgrx::warning!("Extracted {} rows from group_0", temp_rows.len());
            for (i, row) in temp_rows.iter().enumerate() {
                pgrx::warning!("Row {}: {:?}", i, row);
                let group_key = row.group_keys.iter()
                    .map(|k| match k {
                        tantivy::schema::OwnedValue::Str(s) => s.clone(),
                        tantivy::schema::OwnedValue::I64(i) => i.to_string(),
                        tantivy::schema::OwnedValue::F64(f) => f.to_string(),
                        tantivy::schema::OwnedValue::Bool(b) => b.to_string(),
                        _ => format!("{:?}", k),
                    })
                    .collect::<Vec<_>>()
                    .join("|");
                
                // Create a full aggregate row with all positions initialized to Null
                let mut full_aggregate_values = AggregateRow::default();
                full_aggregate_values.resize(self.aggregate_types.len(), AggregateValue::Null);
                
                // Copy the extracted values to their correct positions
                pgrx::warning!("Mapping extracted values for group key: {}", group_key);
                pgrx::warning!("Non-filtered indices: {:?}", non_filtered_indices);
                pgrx::warning!("Extracted aggregate values: {:?}", row.aggregate_values);
                for (extracted_idx, &aggregate_idx) in non_filtered_indices.iter().enumerate() {
                    if let Some(value) = row.aggregate_values.get(extracted_idx) {
                        pgrx::warning!("Mapping extracted_idx {} (value: {:?}) to aggregate_idx {}", extracted_idx, value, aggregate_idx);
                        if aggregate_idx < full_aggregate_values.len() {
                            full_aggregate_values[aggregate_idx] = value.clone();
                        }
                    } else {
                        pgrx::warning!("No value found at extracted_idx {}", extracted_idx);
                    }
                }
                pgrx::warning!("Final aggregate values: {:?}", full_aggregate_values);
                
                group_buckets.insert(group_key, (row.group_keys.clone(), full_aggregate_values));
            }
        }

        // Then, process filtered aggregations and merge them with the base groups
        pgrx::warning!("=== Processing filter aggregations ===");
        for (key, value) in result_obj {
            if key.starts_with("filter_") {
                pgrx::warning!("Processing filter key: {}", key);
                // Extract the aggregate index from the key
                if let Ok(agg_idx) = key.strip_prefix("filter_").unwrap_or("").parse::<usize>() {
                    pgrx::warning!("Parsed aggregate index: {}", agg_idx);
                    if agg_idx < self.aggregate_types.len() {
                        // Extract buckets from the filtered aggregation
                        if let Some(grouped) = value.get("grouped") {
                            pgrx::warning!("Found grouped data for {}", key);
                            if let Some(buckets) = grouped.get("buckets").and_then(|b| b.as_array()) {
                                pgrx::warning!("Found {} buckets in {}", buckets.len(), key);
                                for bucket in buckets {
                                    if let Some(bucket_obj) = bucket.as_object() {
                                        // Extract the group key
                                        if let Some(key_value) = bucket_obj.get("key") {
                                            let group_key_str = match key_value {
                                                serde_json::Value::String(s) => s.clone(),
                                                serde_json::Value::Number(n) => n.to_string(),
                                                serde_json::Value::Bool(b) => b.to_string(),
                                                _ => format!("{:?}", key_value),
                                            };
                                            
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
                                                            },
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
                                            
                                            pgrx::warning!("Extracted from {}: group_key={}, agg_value={:?}", key, group_key_str, aggregate_value);
                                            
                                            // Update the specific aggregate value in the existing group bucket
                                            if let Some((_, aggregate_values)) = group_buckets.get_mut(&group_key_str) {
                                                if agg_idx < aggregate_values.len() {
                                                    pgrx::warning!("Updated aggregate {} for group {} with value: {:?}", agg_idx, group_key_str, aggregate_value);
                                                    aggregate_values[agg_idx] = aggregate_value;
                                                }
                                            } else {
                                                pgrx::warning!("Group key {} not found in group_buckets for filter {}", group_key_str, key);
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

        // Convert the merged results back to GroupedAggregateRow
        let final_results: Vec<GroupedAggregateRow> = group_buckets.into_iter()
            .map(|(_, (group_keys, aggregate_values))| GroupedAggregateRow {
                group_keys,
                aggregate_values,
            })
            .collect();
        
        pgrx::warning!("=== Final merged results ===");
        pgrx::warning!("Number of final results: {}", final_results.len());
        for (i, row) in final_results.iter().enumerate() {
            pgrx::warning!("Final row {}: {:?}", i, row);
        }
        
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
                pgrx::warning!("Processing filter key: {}", key);
                // Extract the aggregate index from the key
                if let Ok(agg_idx) = key.strip_prefix("filter_").unwrap_or("").parse::<usize>() {
                    pgrx::warning!("Parsed aggregate index: {}", agg_idx);
                    if agg_idx < self.aggregate_types.len() {
                        // Extract buckets from the filtered aggregation
                        pgrx::warning!("=== NEW DIRECT JSON EXTRACTION for {} ===", key);
                        if let Some(grouped) = value.get("grouped") {
                            if let Some(buckets) = grouped.get("buckets").and_then(|b| b.as_array()) {
                                pgrx::warning!("Found {} buckets in {}", buckets.len(), key);
                                for bucket in buckets {
                                    if let Some(bucket_obj) = bucket.as_object() {
                                        // Extract the group key
                                        if let Some(key_value) = bucket_obj.get("key") {
                                            let group_key_str = format!("{:?}", key_value);
                                            
                                            // Convert the key to OwnedValue for group_keys
                                            let group_key_owned = match key_value {
                                                serde_json::Value::String(s) => OwnedValue::Str(s.clone()),
                                                serde_json::Value::Number(n) => {
                                                    if let Some(i) = n.as_i64() {
                                                        OwnedValue::I64(i)
                                                    } else if let Some(f) = n.as_f64() {
                                                        OwnedValue::F64(f)
                                                    } else {
                                                        OwnedValue::Str(n.to_string())
                                                    }
                                                },
                                                _ => OwnedValue::Str(key_value.to_string()),
                                            };
                                            
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
                                                            },
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
                                            
                                            pgrx::warning!("Extracted from {}: group_key={}, agg_value={:?}", key, group_key_str, aggregate_value);
                                            
                                            // Get or create the group bucket
                                            let (group_keys, aggregate_values) = group_buckets.entry(group_key_str.clone())
                                                .or_insert_with(|| {
                                                    // Create new group with NULL values for all aggregates
                                                    let mut agg_values = AggregateRow::default();
                                                    agg_values.resize(self.aggregate_types.len(), AggregateValue::Null);
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
        group_buckets.into_iter()
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
            let has_simple_filter_agg = !has_filter_keys && !has_group_key && 
                self.grouping_columns.is_empty() && 
                result_obj.keys().all(|k| k.parse::<usize>().is_ok()) &&
                result_obj.values().any(|v| v.as_object().map_or(false, |obj| obj.contains_key("doc_count")));
            
            pgrx::warning!("=== GROUP BY Result Structure Analysis ===");
            pgrx::warning!("Has filter keys: {}, Has group key: {}, Has simple filter agg: {}", has_filter_keys, has_group_key, has_simple_filter_agg);
            pgrx::warning!("Keys: {:?}", result_obj.keys().collect::<Vec<_>>());
            
            if has_filter_keys && has_group_key {
                // This is a mixed FilterAggregation structure - merge the results
                pgrx::warning!("Using mixed FilterAggregation processing");
                return self.merge_mixed_filter_aggregation_results(result_obj);
            } else if has_filter_keys && !has_group_key {
                // Only filtered aggregations - process them
                pgrx::warning!("Using filter-only processing");
                return self.process_filter_only_group_results(result_obj);
            } else if has_simple_filter_agg {
                // Simple FilterAggregation (no GROUP BY) with numeric keys
                pgrx::warning!("Using simple FilterAggregation processing");
                return self.process_simple_filter_aggregation_results(result.clone());
            } else if !has_filter_keys && has_group_key {
                // Only non-filtered GROUP BY - use regular processing
                pgrx::warning!("Using regular GROUP BY processing (no filters)");
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

    /// Merge results from multi-group queries where aggregates are grouped by filter
    pub fn merge_multi_group_results(
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
            all_groups
                .into_iter()
                .map(|(_, group_keys, aggregate_values)| GroupedAggregateRow {
                    group_keys,
                    aggregate_values: aggregate_values.into_iter().collect(),
                })
                .collect()
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
