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

use crate::postgres::types::TantivyValue;

/// Compute split points to divide the sample space into `num_partitions` roughly equal ranges.
pub fn compute_range_split_points(
    samples: &[TantivyValue],
    num_partitions: usize,
) -> Vec<TantivyValue> {
    if num_partitions <= 1 || samples.is_empty() {
        return vec![];
    }

    let mut boundaries = Vec::with_capacity(num_partitions - 1);
    let total_samples = samples.len();

    // We want to divide the samples into `num_partitions` roughly equal groups.
    // The split points should be at indices: k * (N / P) for k = 1 to P-1
    // where N is total_samples and P is num_partitions.

    for i in 1..num_partitions {
        let idx = (i * total_samples) / num_partitions;
        if idx < total_samples {
            boundaries.push(samples[idx].clone());
        }
    }

    boundaries
}
