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

use crate::aggregate::agg_result::AggResult;
use crate::aggregate::agg_spec::AggregationSpec;
use crate::aggregate::tantivy_keys::{AVG, BUCKETS, DOC_COUNT, GROUPED, KEY, MAX, MIN, SUM, VALUE};
use crate::api::OrderByInfo;
use crate::postgres::customscan::aggregatescan::privdat::{
    AggregateType, AggregateValue, GroupingColumn, TargetListEntry,
};
use crate::postgres::customscan::CustomScanState;
use crate::postgres::types::TantivyValue;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;
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
    // The aggregation specification (agg_types, grouping_columns)
    pub agg_spec: AggregationSpec,
    // ORDER BY specification (query execution detail)
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

    /// Walk grouped buckets recursively, collecting group keys and aggregate values
    ///
    /// For Direct format: filter_results is None, aggregates are in the bucket
    /// For Filter format: filter_results contains lookup table for filtered aggregates
    pub(crate) fn walk_grouped_buckets(
        &self,
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

        if depth >= self.grouping_columns().len() {
            return;
        }

        let grouping_column = &self.grouping_columns()[depth];

        for bucket in buckets {
            let bucket_obj = bucket.as_object().expect("bucket should be object");

            // Extract and store group key
            let key_json = bucket_obj.get(KEY).expect("bucket should have key");
            let key_owned = self.json_value_to_owned_value(key_json, &grouping_column.field_name);
            group_keys.push(key_owned.clone());

            if depth + 1 == self.grouping_columns().len() {
                // Leaf level - extract aggregate values
                let aggregate_values =
                    self.extract_aggregates_for_group(bucket_obj, filter_results, group_keys);

                output_rows.push(GroupedAggregateRow {
                    group_keys: group_keys.clone(),
                    aggregate_values,
                });
            } else {
                // Recurse into nested grouped
                if let Some(nested_grouped) = bucket_obj.get(GROUPED) {
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

    /// Extract aggregate values for a specific group
    ///
    /// For Direct format: extract from bucket_obj directly
    /// For Filter format: look up in filter_results using group_keys
    fn extract_aggregates_for_group(
        &self,
        bucket_obj: &serde_json::Map<String, serde_json::Value>,
        filter_results: Option<&[(usize, &serde_json::Value)]>,
        group_keys: &[OwnedValue],
    ) -> AggregateRow {
        if self.aggregate_types().is_empty() {
            return AggregateRow::default();
        }

        match filter_results {
            None => {
                // Direct format: aggregates are in the bucket
                self.aggregate_types()
                    .iter()
                    .enumerate()
                    .map(|(idx, aggregate)| {
                        bucket_obj
                            .get(&idx.to_string())
                            .map(|v| {
                                let agg_result = AggResult::extract_aggregate_value_from_json(v);
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
                self.aggregate_types()
                    .iter()
                    .enumerate()
                    .map(|(agg_idx, aggregate)| {
                        filter_results
                            .iter()
                            .find(|(idx, _)| *idx == agg_idx)
                            .and_then(|(_, filter_grouped)| {
                                self.find_aggregate_value_in_filter(
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
        &self,
        mut grouped: &serde_json::Value,
        group_keys: &[OwnedValue],
        depth: usize,
        aggregate: &AggregateType,
    ) -> Option<AggregateValue> {
        // Iterate through each grouping level instead of recursing
        for level in depth..group_keys.len() {
            let buckets = grouped.get(BUCKETS)?.as_array()?;
            let target_key = &group_keys[level];
            let grouping_column = &self.grouping_columns()[level];

            // Find bucket matching this group key
            let matching_bucket = buckets.iter().find_map(|bucket| {
                let bucket_obj = bucket.as_object()?;
                let key_json = bucket_obj.get(KEY)?;

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
            grouped = matching_bucket.get(GROUPED)?;
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
        let doc_count = bucket.get(DOC_COUNT).and_then(|d| d.as_i64());

        // Look for the aggregate value - could be a numeric key or named sub-aggregation
        for (key, value) in bucket.iter() {
            if key.parse::<usize>().is_ok() || matches!(key.as_str(), VALUE | AVG | SUM | MIN | MAX)
            {
                let agg_result = AggResult::extract_aggregate_value_from_json(value);
                return Some(aggregate.result_from_aggregate_with_doc_count(agg_result, doc_count));
            }
        }

        // No aggregate found in this bucket
        None
    }

    // Convenience accessors for agg_spec fields
    #[inline(always)]
    pub fn aggregate_types(&self) -> &[AggregateType] {
        &self.agg_spec.aggs
    }

    #[inline(always)]
    pub fn aggregate_types_mut(&mut self) -> &mut [AggregateType] {
        &mut self.agg_spec.aggs
    }

    #[inline(always)]
    pub fn grouping_columns(&self) -> &[GroupingColumn] {
        &self.agg_spec.groupby
    }
}

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        // TODO: Unused currently. See the comment on `trait CustomScanState` regarding making this
        // more useful.
    }
}
