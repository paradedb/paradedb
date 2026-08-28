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

use std::ops::Bound;
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

/// The rows one partition holds: a half-open value range, and whether the NULLs are among them.
#[derive(Debug, Clone, PartialEq)]
pub struct PartitionRange {
    pub lower: Bound<PdbOwnedValue>,
    pub upper: Bound<PdbOwnedValue>,
    pub includes_nulls: bool,
}

impl RangePartitioning {
    /// The partition a row with a NULL partition field lands in. The kd-tree of a partitioned
    /// build sends NULLs below every split, so its lowest partition is this one, and so is the
    /// first partition of DataFusion's `NULLS FIRST` range ordering. Every consumer of the NULL
    /// rule reads it from here.
    pub const NULL_PARTITION: usize = 0;

    /// Returns the static boundary constraint for the given partition as a single RangeQuery.
    ///
    /// **Consumer Caveats**:
    /// - A row whose partition field is NULL will be deterministically routed to
    ///   [`Self::NULL_PARTITION`].
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

        if partition == Self::NULL_PARTITION {
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

    /// The rows of `partition`, or `None` when the partition is not a range: no split points
    /// at all (every row), or a NULL upper split (no row). The same rows
    /// [`Self::partition_bounds`] selects.
    pub fn partition_range(&self, partition: usize) -> Option<PartitionRange> {
        if self.split_points.is_empty() {
            return None;
        }
        let lower = match partition
            .checked_sub(1)
            .and_then(|i| self.split_points.get(i))
        {
            None | Some(PdbOwnedValue::Null) => Bound::Unbounded,
            Some(val) => Bound::Included(val.clone()),
        };
        let upper = match self.split_points.get(partition) {
            Some(PdbOwnedValue::Null) => return None,
            Some(val) => Bound::Excluded(val.clone()),
            None => Bound::Unbounded,
        };
        Some(PartitionRange {
            lower,
            upper,
            includes_nulls: partition == Self::NULL_PARTITION,
        })
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

        // `partition_bounds` routes NULLs to `NULL_PARTITION`, the first one, and uses
        // lower-inclusive, upper-exclusive interior ranges, which is exactly DataFusion's
        // split-point convention under an ascending NULLS FIRST ordering.
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

/// The grid a partitioned build stamped on an index's segments, kept as raw points rather
/// than a static `RangePartitioning`, so that `TaskEstimator`s and distributed execution
/// engines can ask for exactly their preferred number of partitions at runtime.
///
/// **Precondition**: `points` must be sorted ascending, otherwise the generated
/// boundaries will produce overlapping or gapped ranges that silently drop or duplicate rows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RangePartitioningGrid {
    /// The index field used to define the boundaries.
    pub partition_by: FieldName,
    /// The points to cut on, sorted ascending: the edges of every box the build stamped, so a
    /// partition cut on them lines up with the segments. Typically there are more than the
    /// target number of partitions, so `build` can down-sample them evenly.
    pub points: Vec<PdbOwnedValue>,
}

impl RangePartitioningGrid {
    /// Generates a concrete `RangePartitioning` bounding exactly `target_partitions`.
    ///
    /// If `target_partitions` is smaller than the grid's inherent size, the
    /// points are evenly down-sampled (effectively merging contiguous partitions).
    /// To avoid scheduling unnecessary tasks scanning empty ranges, `target_partitions`
    /// is capped at `points.len() + 1`.
    pub fn build(&self, target_partitions: usize) -> RangePartitioning {
        let points = &self.points;
        debug_assert!(
            points
                .windows(2)
                .all(|w| w[0].total_cmp(&w[1]) != std::cmp::Ordering::Greater),
            "RangePartitioningGrid requires its points to be sorted ascending"
        );

        let num_points = points.len();

        // Cap target_partitions to avoid scheduling unnecessary empty ranges
        let actual_partitions = if target_partitions > num_points + 1 {
            num_points + 1
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
            let split_idx = (i * num_points) / actual_partitions;
            new_split_points.push(points[split_idx].clone());
        }

        RangePartitioning {
            partition_by: self.partition_by.clone(),
            split_points: new_split_points,
        }
    }
}
