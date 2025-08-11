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

use crate::postgres::customscan::aggregatescan::privdat::{
    AggregateResult, AggregateType, AggregateValue, GroupingColumn, TargetListEntry,
};
use crate::postgres::customscan::builders::custom_path::OrderByInfo;
use crate::postgres::customscan::CustomScanState;
use crate::postgres::types::TantivyValue;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;
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
    pub order_by_info: Vec<OrderByInfo>,
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

            // Add a document count aggregation only if we have SUM aggregates (to detect empty result sets)
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

        for (i, group_col) in self.grouping_columns.iter().enumerate().rev() {
            let mut terms = serde_json::Map::new();
            terms.insert(
                "field".to_string(),
                serde_json::Value::String(group_col.field_name.clone()),
            );
            // if we remove this, we'd get the default size of 10, which means we receive 10 groups max from tantivy
            terms.insert("size".to_string(), serde_json::Value::Number(10000.into())); // TODO: make configurable

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
            let result_map = result
                .as_object()
                .expect("unexpected aggregate result collection type");

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
                    self.process_aggregate_result(
                        aggregate,
                        idx,
                        AggregateResultSource::ResultMap(result_map),
                        doc_count,
                    )
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

        self.extract_bucket_results(&result, 0, &mut Vec::new(), &mut rows);

        // Sort according to ORDER BY
        self.sort_rows(&mut rows);
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

    /// Process a single aggregate result, handling doc_count for empty result sets
    /// This consolidates the logic used in both simple and grouped aggregations
    fn process_aggregate_result(
        &self,
        aggregate: &AggregateType,
        agg_idx: usize,
        result_source: AggregateResultSource<'_>,
        doc_count: Option<i64>,
    ) -> AggregateValue {
        let agg_result = match (aggregate, result_source) {
            (AggregateType::Count, AggregateResultSource::ResultMap(result_map)) => {
                let raw_result = result_map
                    .get(&agg_idx.to_string())
                    .expect("missing aggregate result");
                Self::extract_aggregate_value_from_json(raw_result)
            }
            (AggregateType::Count, AggregateResultSource::BucketObj(bucket_obj)) => {
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
                let agg_obj = bucket_obj
                    .get(&agg_name)
                    .unwrap_or_else(|| panic!("missing aggregate result for '{agg_name}'"));
                Self::extract_aggregate_value_from_json(agg_obj)
            }
        };

        // Apply appropriate doc_count handling based on aggregate type
        let doc_count_for_aggregate = match aggregate {
            AggregateType::Sum { .. } => doc_count,
            _ => None,
        };

        aggregate.result_from_aggregate_with_doc_count(agg_result, doc_count_for_aggregate)
    }

    #[allow(unreachable_patterns)]
    fn extract_bucket_results(
        &self,
        json: &serde_json::Value,
        depth: usize,
        prefix_keys: &mut Vec<OwnedValue>,
        rows: &mut Vec<GroupedAggregateRow>,
    ) {
        let bucket_name = format!("group_{depth}");
        let buckets = json
            .get(&bucket_name)
            .and_then(|v| v.get("buckets"))
            .and_then(|v| v.as_array())
            .expect("missing bucket results");

        for bucket in buckets {
            let bucket_obj = bucket.as_object().expect("bucket should be object");

            // Current grouping key
            let grouping_column = &self.grouping_columns[depth];
            let key_json = bucket_obj.get("key").expect("missing bucket key");
            let key_owned = self.json_value_to_owned_value(key_json, &grouping_column.field_name);
            prefix_keys.push(key_owned);

            if depth + 1 == self.grouping_columns.len() {
                // Deepest level – collect aggregates (may be empty)
                let aggregate_values: AggregateRow = if self.aggregate_types.is_empty() {
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
                };
                rows.push(GroupedAggregateRow {
                    group_keys: prefix_keys.clone(),
                    aggregate_values,
                });
            } else {
                // Recurse into next level
                self.extract_bucket_results(bucket, depth + 1, prefix_keys, rows);
            }

            prefix_keys.pop();
        }
    }

    fn sort_rows(&self, rows: &mut [GroupedAggregateRow]) {
        if self.order_by_info.is_empty() {
            return;
        }

        rows.sort_by(|a, b| {
            for order_info in &self.order_by_info {
                // Find the index of this grouping column
                let col_index = self
                    .grouping_columns
                    .iter()
                    .position(|gc| gc.field_name == order_info.field_name);

                let cmp = if let Some(idx) = col_index {
                    let val_a = a.group_keys.get(idx);
                    let val_b = b.group_keys.get(idx);

                    // Wrap in TantivyValue for comparison since OwnedValue doesn't implement Ord
                    let tantivy_a = val_a.map(|v| TantivyValue(v.clone()));
                    let tantivy_b = val_b.map(|v| TantivyValue(v.clone()));
                    let base_cmp = tantivy_a.partial_cmp(&tantivy_b);

                    if let Some(base_cmp) = base_cmp {
                        if order_info.is_desc {
                            base_cmp.reverse()
                        } else {
                            base_cmp
                        }
                    } else {
                        panic!(
                            "Cannot ORDER BY {order_info:?} for {tantivy_a:?} and {tantivy_b:?}."
                        );
                    }
                } else {
                    std::cmp::Ordering::Equal
                };

                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
            }
            std::cmp::Ordering::Equal
        });
    }
}

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        // TODO: Unused currently. See the comment on `trait CustomScanState` regarding making this
        // more useful.
    }
}
