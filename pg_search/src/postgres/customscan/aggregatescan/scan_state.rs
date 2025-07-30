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
    AggregateType, GroupingColumn, TargetListEntry,
};
use crate::postgres::customscan::builders::custom_path::OrderByInfo;
use crate::postgres::customscan::CustomScanState;
use crate::postgres::types::TantivyValue;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;
use tantivy::schema::OwnedValue;

use pgrx::pg_sys;
use tinyvec::TinyVec;

// TODO: This should match the output types of the extracted aggregate functions. For now we only
// support COUNT.
pub type AggregateRow = TinyVec<[i64; 4]>;

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
            return serde_json::Value::Object(
                self.aggregate_types
                    .iter()
                    .enumerate()
                    .map(|(idx, aggregate)| (idx.to_string(), aggregate.to_json()))
                    .collect(),
            );
        }
        // GROUP BY - bucket aggregation
        let mut root = serde_json::Map::new();

        // Build nested bucket aggregations for each grouping column
        let current_level = &mut root;
        let _ = current_level; // Mark as used

        for (i, group_col) in self.grouping_columns.iter().enumerate() {
            let bucket_name = format!("group_{i}");
            let mut bucket_agg = serde_json::Map::new();

            // Terms aggregation for grouping
            let mut terms = serde_json::Map::new();
            terms.insert(
                "field".to_string(),
                serde_json::Value::String(group_col.field_name.clone()),
            );

            let mut terms_agg = serde_json::Map::new();
            terms_agg.insert("terms".to_string(), serde_json::Value::Object(terms));

            // If this is the last grouping column, add the metric aggregations
            if i == self.grouping_columns.len() - 1 {
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

            bucket_agg.insert(bucket_name.clone(), serde_json::Value::Object(terms_agg));
            current_level.insert("aggs".to_string(), serde_json::Value::Object(bucket_agg));

            // For nested buckets, we'd need to traverse deeper, but for now we'll handle single-level
            if i < self.grouping_columns.len() - 1 {
                // This should never happen since we reject multiple grouping columns at planning time
                unreachable!("Multiple grouping columns should have been rejected during planning");
            }
        }

        serde_json::Value::Object(root.get("aggs").unwrap().as_object().unwrap().clone())
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

            let row = self
                .aggregate_types
                .iter()
                .enumerate()
                .map(move |(idx, aggregate)| {
                    let aggregate_val = result_map
                        .get(&idx.to_string())
                        .expect("missing aggregate result")
                        .as_object()
                        .expect("unexpected aggregate structure")
                        .get("value")
                        .expect("missing aggregate result value")
                        .as_number()
                        .expect("unexpected aggregate result type");

                    aggregate.result_from_json(aggregate_val)
                })
                .collect::<AggregateRow>();

            // No sorting needed for single aggregate result
            return vec![GroupedAggregateRow {
                group_keys: vec![],
                aggregate_values: row,
            }];
        }
        // GROUP BY - extract bucket results
        let mut rows = Vec::new();

        // Navigate to the bucket results
        let bucket_name = "group_0"; // For now, we only support single grouping column
        let bucket_results = result
            .get(bucket_name)
            .and_then(|v| v.get("buckets"))
            .and_then(|v| v.as_array())
            .expect("missing bucket results");

        for bucket in bucket_results {
            let bucket_obj = bucket.as_object().expect("bucket should be object");

            // Extract the group key - can be either string or number
            let grouping_column = &self.grouping_columns[0]; // We only support single grouping column for now
            let key = bucket_obj
                .get("key")
                .map(|k| {
                    // Create OwnedValue from JSON value based on the field type
                    self.json_value_to_owned_value(k, &grouping_column.field_name)
                })
                .expect("missing bucket key");

            // Extract aggregate values
            let aggregate_values = self
                .aggregate_types
                .iter()
                .enumerate()
                .map(|(idx, aggregate)| {
                    let agg_result = match aggregate {
                        // Count is handled by the 'terms' bucket
                        AggregateType::Count => bucket_obj
                            .get("doc_count")
                            .and_then(|v| v.as_number())
                            .expect("missing doc_count"),
                        _ => {
                            let agg_name = format!("agg_{idx}");
                            bucket_obj
                                .get(&agg_name)
                                .and_then(|v| v.as_object())
                                .and_then(|v| v.get("value"))
                                .and_then(|v| v.as_number())
                                .expect("missing aggregate result")
                        }
                    };

                    aggregate.result_from_json(agg_result)
                })
                .collect::<AggregateRow>();

            rows.push(GroupedAggregateRow {
                group_keys: vec![key],
                aggregate_values,
            });
        }

        // Sort the rows according to ORDER BY specification
        self.sort_rows(&mut rows);
        rows
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
