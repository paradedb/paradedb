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

use serde::{Deserialize, Serialize};

/// Defines the logical boundary split points for scanning the index. When provided,
/// the DataFusion execution plan uses these points to statically partition the scan
/// into sequential chunks, rather than relying on dynamic segment checkout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangePartitioning {
    /// The index field used to define the boundaries.
    pub partition_by: crate::api::FieldName,
    /// The values that split the data space into separate partitions. A length of N
    /// produces N+1 partitions.
    pub split_points: Vec<crate::postgres::pdb_owned_value::PdbOwnedValue>,
}

impl RangePartitioning {
    /// Returns the static boundary constraint for the given partition as a single RangeQuery.
    pub fn partition_bounds(&self, partition: usize) -> crate::query::SearchQueryInput {
        let lower = if partition > 0 {
            std::ops::Bound::Included(self.split_points[partition - 1].clone())
        } else {
            std::ops::Bound::Unbounded
        };
        let upper = if partition < self.split_points.len() {
            std::ops::Bound::Excluded(self.split_points[partition].clone())
        } else {
            std::ops::Bound::Unbounded
        };

        crate::query::SearchQueryInput::FieldedQuery {
            field: self.partition_by.clone(),
            query: crate::query::pdb_query::pdb::Query::Range {
                lower_bound: lower,
                upper_bound: upper,
            },
        }
    }
}

/// A representation of the raw partition split points for range partitioning.
/// Rather than directly instantiating a static `RangePartitioning` with these
/// points, this sample is kept so that `TaskEstimator`s and distributed execution
/// engines can ask for exactly their preferred number of partitions at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangePartitioningSample {
    /// The index field used to define the boundaries.
    pub partition_by: crate::api::FieldName,
    /// A sample of values from the data space. This sample is typically much larger
    /// than the target number of partitions, allowing us to safely down-sample to compute
    /// relatively uniform distribution boundaries.
    pub sample_points: Vec<crate::postgres::pdb_owned_value::PdbOwnedValue>,
}

impl RangePartitioningSample {
    /// Generates a concrete `RangePartitioning` bounding exactly `target_partitions`.
    ///
    /// If `target_partitions` is smaller than the sample's inherent size, the
    /// sample points are evenly down-sampled (effectively merging contiguous partitions).
    /// If it is larger, the last sample point is duplicated to pad the set with empty ranges.
    pub fn build(&self, target_partitions: usize) -> RangePartitioning {
        let num_samples = self.sample_points.len();
        if target_partitions == 0 || target_partitions == 1 {
            return RangePartitioning {
                partition_by: self.partition_by.clone(),
                split_points: vec![],
            };
        }

        let mut new_split_points = Vec::with_capacity(target_partitions - 1);

        // If we want more partitions than we have splits, just keep all and pad.
        if target_partitions > num_samples + 1 {
            new_split_points.extend_from_slice(&self.sample_points);
            if let Some(pad_val) = self.sample_points.last() {
                while new_split_points.len() < target_partitions - 1 {
                    new_split_points.push(pad_val.clone());
                }
            }
        } else {
            // Down-sample evenly. We want `target_partitions - 1` split points.
            for i in 1..target_partitions {
                let split_idx = (i * num_samples) / target_partitions;
                new_split_points.push(self.sample_points[split_idx].clone());
            }
        }

        RangePartitioning {
            partition_by: self.partition_by.clone(),
            split_points: new_split_points,
        }
    }
}
