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
use crate::postgres::customscan::parallel::{ParallelCost, WorkerDecisionReason};

#[derive(Default)]
pub struct AggregateParallelism {
    pub workers_requested: usize,
    pub workers_launched: usize,
    pub worker_selection_reason: Option<WorkerDecisionReason>,
    pub cost: Option<ParallelCost>,
}

impl AggregateParallelism {
    pub fn explain(&self, explainer: &mut Explainer) {
        explainer.add_unsigned_integer("Workers Requested", self.workers_requested as u64, None);
        explainer.add_unsigned_integer("Workers Launched", self.workers_launched as u64, None);
        if let Some(reason) = self.worker_selection_reason {
            explainer.add_text("Worker Selection", reason.to_string());
        }
        if explainer.is_costs()
            && let Some(cost) = &self.cost
        {
            explainer.add_float("Estimated Query Work", cost.estimated_work, 2, None);
            if cost.parallel_threshold.is_finite() {
                explainer.add_float("Parallel Threshold", cost.parallel_threshold, 2, None);
            }
        }
    }
}
