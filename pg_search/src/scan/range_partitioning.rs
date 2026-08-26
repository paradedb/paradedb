// Copyright (c) 2023-2026 ParadeDB, Inc.
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

use std::sync::Arc;

use arrow_schema::{SchemaRef, SortOptions};
use datafusion::common::SplitPoint;
use datafusion::physical_expr::{
    LexOrdering, PhysicalSortExpr, RangePartitioning as DataFusionRangePartitioning,
};
use datafusion::physical_plan::Partitioning;
use datafusion::physical_plan::expressions::Column;
use serde::{Deserialize, Serialize};

use crate::api::FieldName;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::query::SearchQueryInput;
use crate::query::pdb_query::pdb::Query;

/// Defines the logical boundary split points for scanning the index. When provided,
/// the DataFusion execution plan uses these points to statically partition the scan
/// into sequential chunks, rather than relying on dynamic segment checkout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangePartitioning {
    /// The index field used to define the boundaries.
    pub partition_by: FieldName,
    /// The values that split the data space into separate partitions. A length of N
    /// produces N+1 partitions.
    pub split_points: Vec<PdbOwnedValue>,
}

impl RangePartitioning {
    /// Returns the static boundary constraint for the given partition as a single RangeQuery.
    ///
    /// **Consumer Caveats**:
    /// - A row whose partition field is NULL will be deterministically routed to partition 0.
    /// - A multi-valued field can fall into multiple partition ranges and duplicate the row. This is statically prevented during index configuration for `partition_by` columns.
    pub fn partition_bounds(&self, partition: usize) -> SearchQueryInput {
        if self.split_points.is_empty() {
            return SearchQueryInput::All;
        }

        let lower = if partition > 0 {
            let val = &self.split_points[partition - 1];
            if matches!(val, PdbOwnedValue::Null) {
                std::ops::Bound::Unbounded
            } else {
                std::ops::Bound::Included(val.clone())
            }
        } else {
            std::ops::Bound::Unbounded
        };

        let mut is_empty_range = false;
        let upper = if partition < self.split_points.len() {
            let val = &self.split_points[partition];
            if matches!(val, PdbOwnedValue::Null) {
                is_empty_range = true;
                std::ops::Bound::Unbounded
            } else {
                std::ops::Bound::Excluded(val.clone())
            }
        } else {
            std::ops::Bound::Unbounded
        };

        let range_query = if is_empty_range {
            SearchQueryInput::Empty
        } else {
            SearchQueryInput::FieldedQuery {
                field: self.partition_by.clone(),
                query: Query::Range {
                    lower_bound: lower,
                    upper_bound: upper,
                },
            }
        };

        // NULLs fall into partition 0
        if partition == 0 {
            let null_query = SearchQueryInput::Boolean {
                must: vec![],
                should: vec![],
                must_not: vec![SearchQueryInput::FieldedQuery {
                    field: self.partition_by.clone(),
                    query: Query::Exists,
                }],
                minimum_should_match: None,
            };

            if is_empty_range {
                null_query
            } else {
                SearchQueryInput::Boolean {
                    must: vec![],
                    should: vec![range_query, null_query],
                    must_not: vec![],
                    minimum_should_match: None,
                }
            }
        } else {
            range_query
        }
    }

    /// The value range of `partition` as bounds, or `None` when the partition is not a range:
    /// no split points at all (every row), or a NULL upper split (no row).
    pub fn partition_range(
        &self,
        partition: usize,
    ) -> Option<(
        std::ops::Bound<PdbOwnedValue>,
        std::ops::Bound<PdbOwnedValue>,
    )> {
        if self.split_points.is_empty() {
            return None;
        }
        let lower = match partition
            .checked_sub(1)
            .and_then(|i| self.split_points.get(i))
        {
            None | Some(PdbOwnedValue::Null) => std::ops::Bound::Unbounded,
            Some(val) => std::ops::Bound::Included(val.clone()),
        };
        let upper = match self.split_points.get(partition) {
            Some(PdbOwnedValue::Null) => return None,
            Some(val) => std::ops::Bound::Excluded(val.clone()),
            None => std::ops::Bound::Unbounded,
        };
        Some((lower, upper))
    }

    /// Translates these boundaries into a DataFusion [`Partitioning::Range`] declaration
    /// over `schema`, so the planner can co-partition operators (e.g. joins) without a
    /// repartition or broadcast.
    ///
    /// Returns `None` when the declaration would not be faithful to the execution
    /// semantics of [`Self::partition_bounds`]:
    /// - the `partition_by` column is missing from the schema, or
    /// - a split point is NULL (`partition_bounds` gives NULL split points bespoke
    ///   empty-range semantics that DataFusion's model does not express), or
    /// - a split point cannot be represented as a `ScalarValue` of the column's type.
    pub fn to_datafusion(&self, schema: &SchemaRef) -> Option<Partitioning> {
        let (col_idx, field) = schema.column_with_name(self.partition_by.as_ref())?;

        let split_points = self
            .split_points
            .iter()
            .map(|value| {
                value
                    .to_scalar(field.data_type())
                    .map(|sv| SplitPoint::new(vec![sv]))
            })
            .collect::<Option<Vec<_>>>()?;

        // `partition_bounds` routes NULLs to partition 0 and uses lower-inclusive,
        // upper-exclusive interior ranges, which is exactly DataFusion's split-point
        // convention under an ascending NULLS FIRST ordering.
        let sort_expr = PhysicalSortExpr {
            expr: Arc::new(Column::new(self.partition_by.as_ref(), col_idx)),
            options: SortOptions {
                descending: false,
                nulls_first: true,
            },
        };
        let ordering = LexOrdering::new([sort_expr])?;

        // `new` rather than `try_new`: a down-sampled distribution can legally repeat a
        // split point, which produces an empty partition in both our execution model and
        // DataFusion's, but fails `try_new`'s strict-ordering validation.
        Some(Partitioning::Range(DataFusionRangePartitioning::new(
            ordering,
            split_points,
        )))
    }
}

/// A representation of the raw partition split points for range partitioning.
/// Rather than directly instantiating a static `RangePartitioning` with these
/// points, this sample is kept so that `TaskEstimator`s and distributed execution
/// engines can ask for exactly their preferred number of partitions at runtime.
///
/// **Precondition**: `sample_points` must be sorted ascending, otherwise the generated
/// boundaries will produce overlapping or gapped ranges that silently drop or duplicate rows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RangePartitioningSample {
    /// The index field used to define the boundaries.
    pub partition_by: FieldName,
    /// A sample of values from the data space. This sample is typically much larger
    /// than the target number of partitions, allowing us to safely down-sample to compute
    /// relatively uniform distribution boundaries.
    pub sample_points: Vec<PdbOwnedValue>,
    /// The split grid a partitioned build stamped on its segments, sorted ascending, or empty.
    /// Partitions cut on it line up with the segments, so each one searches only its cell.
    pub persisted_points: Vec<PdbOwnedValue>,
}

impl RangePartitioningSample {
    /// Generates a concrete `RangePartitioning` bounding exactly `target_partitions`.
    ///
    /// If `target_partitions` is smaller than the sample's inherent size, the
    /// sample points are evenly down-sampled (effectively merging contiguous partitions).
    /// To avoid scheduling unnecessary tasks scanning empty ranges, `target_partitions`
    /// is capped at `sample_points.len() + 1`.
    pub fn build(&self, target_partitions: usize) -> RangePartitioning {
        // The grid is exact but coarse. Below the requested partition count it would cost
        // parallelism, and the sample still divides the space evenly.
        let points = if !self.persisted_points.is_empty()
            && self.persisted_points.len() + 1 >= target_partitions
        {
            &self.persisted_points
        } else {
            &self.sample_points
        };
        debug_assert!(
            points
                .windows(2)
                .all(|w| w[0].total_cmp(&w[1]) != std::cmp::Ordering::Greater),
            "RangePartitioningSample requires its points to be sorted ascending"
        );

        let num_samples = points.len();

        // Cap target_partitions to avoid scheduling unnecessary empty ranges
        let actual_partitions = if target_partitions > num_samples + 1 {
            num_samples + 1
        } else {
            target_partitions
        };

        if actual_partitions <= 1 {
            return RangePartitioning {
                partition_by: self.partition_by.clone(),
                split_points: vec![],
            };
        }

        let mut new_split_points = Vec::with_capacity(actual_partitions - 1);

        // Down-sample evenly. We want `actual_partitions - 1` split points.
        for i in 1..actual_partitions {
            let split_idx = (i * num_samples) / actual_partitions;
            new_split_points.push(points[split_idx].clone());
        }

        RangePartitioning {
            partition_by: self.partition_by.clone(),
            split_points: new_split_points,
        }
    }
}
