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

use crate::postgres::customscan::aggregatescan::privdat::{AggregateType, GroupingColumn};
use crate::postgres::customscan::CustomScanState;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;

use pgrx::pg_sys;
use tinyvec::TinyVec;

// TODO: This should match the output types of the extracted aggregate functions. For now we only
// support COUNT.
pub type AggregateRow = TinyVec<[i64; 4]>;

// For GROUP BY results, we need both the group keys and aggregate values
#[derive(Debug, Clone)]
pub struct GroupedAggregateRow {
    pub group_keys: Vec<String>, // The values of the grouping columns
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
            serde_json::Value::Object(
                self.aggregate_types
                    .iter()
                    .enumerate()
                    .map(|(idx, aggregate)| (idx.to_string(), aggregate.to_json()))
                    .collect(),
            )
        } else {
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
                terms.insert("size".to_string(), serde_json::Value::Number(10000.into())); // TODO: make configurable

                let mut terms_agg = serde_json::Map::new();
                terms_agg.insert("terms".to_string(), serde_json::Value::Object(terms));

                // If this is the last grouping column, add the metric aggregations
                if i == self.grouping_columns.len() - 1 {
                    let mut sub_aggs = serde_json::Map::new();
                    for (j, aggregate) in self.aggregate_types.iter().enumerate() {
                        let (name, agg) = aggregate.to_json_for_group(j);
                        sub_aggs.insert(name, agg);
                    }
                    terms_agg.insert("aggs".to_string(), serde_json::Value::Object(sub_aggs));
                }

                bucket_agg.insert(bucket_name.clone(), serde_json::Value::Object(terms_agg));
                current_level.insert("aggs".to_string(), serde_json::Value::Object(bucket_agg));

                // For nested buckets, we'd need to traverse deeper, but for now we'll handle single-level
                if i < self.grouping_columns.len() - 1 {
                    // TODO: Support multiple grouping columns with nested bucket aggregations
                    todo!("Multiple grouping columns not yet supported");
                }
            }

            serde_json::Value::Object(root.get("aggs").unwrap().as_object().unwrap().clone())
        }
    }

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

            vec![GroupedAggregateRow {
                group_keys: vec![],
                aggregate_values: row,
            }]
        } else {
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
                let key = bucket_obj
                    .get("key")
                    .map(|k| {
                        // Handle both string and numeric keys
                        match k {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            _ => panic!("unexpected bucket key type: {k:?}"),
                        }
                    })
                    .expect("missing bucket key");

                // Extract aggregate values
                let aggregate_values = self
                    .aggregate_types
                    .iter()
                    .enumerate()
                    .map(|(idx, aggregate)| {
                        let agg_name = format!("agg_{idx}");
                        let agg_result = bucket_obj
                            .get(&agg_name)
                            .and_then(|v| v.as_object())
                            .and_then(|v| v.get("value"))
                            .and_then(|v| v.as_number())
                            .expect("missing aggregate result");

                        aggregate.result_from_json(agg_result)
                    })
                    .collect::<AggregateRow>();

                rows.push(GroupedAggregateRow {
                    group_keys: vec![key],
                    aggregate_values,
                });
            }

            rows
        }
    }
}

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        // TODO: Unused currently. See the comment on `trait CustomScanState` regarding making this
        // more useful.
    }
}
