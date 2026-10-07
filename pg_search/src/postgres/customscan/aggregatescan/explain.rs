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

use crate::postgres::customscan::Explainer;

#[derive(Default)]
pub struct AggregateParallelism {
    pub executed: bool,
    pub workers_requested: usize,
    pub workers_used: usize,
    pub estimated_work: Option<f64>,
    pub parallel_threshold: Option<f64>,
}

impl AggregateParallelism {
    pub fn explain(&self, explainer: &mut Explainer) {
        explainer.add_unsigned_integer("Workers Requested", self.workers_requested as u64, None);
        explainer.add_unsigned_integer("Workers Used", self.workers_used as u64, None);
        if explainer.is_costs() {
            if let Some(work) = self.estimated_work {
                explainer.add_float("Estimated Query Work", work, 2, None);
            }
            if let Some(threshold) = self.parallel_threshold {
                if threshold.is_finite() {
                    explainer.add_float("Parallel Threshold", threshold, 2, None);
                } else {
                    explainer.add_text("Parallel Threshold", "infinite");
                }
            }
        }
    }
}
