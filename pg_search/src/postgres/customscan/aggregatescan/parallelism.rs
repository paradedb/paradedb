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
    pub executions: u64,
    pub segments: usize,
    pub workers_requested: usize,
    pub workers_launched: usize,
    pub leader_participated: bool,
    pub max_workers_per_gather: usize,
    pub max_parallel_workers: usize,
    pub max_worker_processes: usize,
    pub reason: &'static str,
}

impl AggregateParallelism {
    pub fn explain(&self, explainer: &mut Explainer) {
        explainer.add_unsigned_integer("Executions", self.executions, None);
        if self.executions == 0 {
            explainer.add_text("Status", "not executed");
            return;
        }
        if self.executions > 1 {
            explainer.add_text("Scope", "last execution");
        }
        explainer.add_unsigned_integer("Segments", self.segments as u64, None);
        explainer.add_unsigned_integer("Workers Requested", self.workers_requested as u64, None);
        explainer.add_unsigned_integer("Workers Launched", self.workers_launched as u64, None);
        explainer.add_bool("Leader Participated", self.leader_participated);
        explainer.add_unsigned_integer(
            "Max Workers Per Gather",
            self.max_workers_per_gather as u64,
            None,
        );
        explainer.add_unsigned_integer(
            "Max Parallel Workers",
            self.max_parallel_workers as u64,
            None,
        );
        explainer.add_unsigned_integer(
            "Max Worker Processes",
            self.max_worker_processes as u64,
            None,
        );
        explainer.add_text("Reason", self.reason);
    }
}
