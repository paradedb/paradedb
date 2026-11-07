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

pub use index_read::scan::range_partitioning::*;

use std::sync::Arc;

use arrow_schema::{SchemaRef, SortOptions};
use datafusion::common::SplitPoint;
use datafusion::physical_expr::{
    LexOrdering, PhysicalSortExpr, RangePartitioning as DataFusionRangePartitioning,
};
use datafusion::physical_plan::Partitioning;
use datafusion::physical_plan::expressions::Column;

use crate::postgres::pdb_owned_value::PdbOwnedValueScalarExt;

/// DataFusion [`Partitioning`] translation for [`RangePartitioning`].
pub trait RangePartitioningDataFusionExt {
    /// Translates these boundaries into a DataFusion [`Partitioning::Range`] declaration
    /// over `schema`, so the planner can co-partition operators (e.g. joins) without a
    /// repartition or broadcast.
    ///
    /// Returns `None` when the declaration would not be faithful to the execution
    /// semantics of [`RangePartitioning::partition_bounds`]:
    /// - the `partition_by` column is missing from the schema, or
    /// - a split point is NULL (`partition_bounds` gives NULL split points bespoke
    ///   empty-range semantics that DataFusion's model does not express), or
    /// - a split point cannot be represented as a `ScalarValue` of the column's type.
    fn to_datafusion(&self, schema: &SchemaRef) -> Option<Partitioning>;
}

impl RangePartitioningDataFusionExt for RangePartitioning {
    fn to_datafusion(&self, schema: &SchemaRef) -> Option<Partitioning> {
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

        // `new` rather than `try_new`: a repeated split point produces an empty partition in
        // both our execution model and DataFusion's, but fails `try_new`'s strict-ordering
        // validation. The build never repeats a split point; the tolerance costs nothing.
        Some(Partitioning::Range(DataFusionRangePartitioning::new(
            ordering,
            split_points,
        )))
    }
}

/// DataFusion [`Partitioning`] translation for [`RangeSplitPoints`].
pub trait RangeSplitPointsDataFusionExt {
    /// Converts these points into a DataFusion [`Partitioning::Range`] bounding `partition_count`
    /// partitions, using all `self.points` as samples so DataFusion stage scaling (`scale`)
    /// and anchor execution down-sample from the identical base.
    ///
    /// Returns `None` if:
    /// - `partition_count <= 1` or `partition_count > self.points.len() + 1`,
    /// - `self.partition_by` is not present in `schema`,
    /// - any split point cannot be represented as a `ScalarValue` of the column's type, or
    /// - DataFusion validation fails (e.g. non-strictly ordered points).
    fn to_datafusion(&self, schema: &SchemaRef, partition_count: usize) -> Option<Partitioning>;
}

impl RangeSplitPointsDataFusionExt for RangeSplitPoints {
    fn to_datafusion(&self, schema: &SchemaRef, partition_count: usize) -> Option<Partitioning> {
        if partition_count <= 1 || partition_count > self.points.len() + 1 {
            return None;
        }

        let (col_idx, field) = schema.column_with_name(self.partition_by.as_ref())?;

        let samples = self
            .points
            .iter()
            .map(|value| {
                value
                    .to_scalar(field.data_type())
                    .map(|sv| SplitPoint::new(vec![sv]))
            })
            .collect::<Option<Vec<_>>>()?;

        let sort_expr = PhysicalSortExpr {
            expr: Arc::new(Column::new(self.partition_by.as_ref(), col_idx)),
            options: SortOptions {
                descending: false,
                nulls_first: true,
            },
        };
        let ordering = LexOrdering::new([sort_expr])?;

        DataFusionRangePartitioning::try_new_with_samples(ordering, samples, partition_count)
            .ok()
            .map(Partitioning::Range)
    }
}
