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

use datafusion::config::ConfigOptions;
use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::{TaskEstimation, TaskEstimator};

/// `PgSearchScanTaskEstimator` intercepts `PgSearchScanPlan` during distributed planning
/// and requests a number of tasks equal to `partition_count = min(segment_count, target_partitions)`.
///
/// This correctly maps PostgreSQL parallel workers to tasks, ensuring that tables with 1 segment
/// do not force MPP planning and fall back to local serial execution, whereas large tables
/// scale out efficiently across all available Postgres parallel workers.
#[derive(Debug)]
pub(crate) struct PgSearchScanTaskEstimator;

impl TaskEstimator for PgSearchScanTaskEstimator {
    fn task_estimation(
        &self,
        plan: &Arc<dyn ExecutionPlan>,
        _cfg: &ConfigOptions,
    ) -> Option<TaskEstimation> {
        if plan.name() != "PgSearchScan" {
            return None;
        }

        let partition_count = plan.properties().output_partitioning().partition_count();

        Some(TaskEstimation::desired(partition_count))
    }

    fn scale_up_leaf_node(
        &self,
        plan: &Arc<dyn ExecutionPlan>,
        task_count: usize,
        _cfg: &ConfigOptions,
    ) -> datafusion::error::Result<Option<Arc<dyn ExecutionPlan>>> {
        if plan.name() != "PgSearchScan" {
            return Ok(None);
        }

        // `Arc::clone` works perfectly here. Each worker decodes its own copy
        // anyway, and `execute()` dynamically claims segments from the
        // parallel state so there is no conflict.
        let variants = (0..task_count)
            .map(|_| Arc::clone(plan))
            .collect::<Vec<_>>();

        Ok(Some(Arc::new(
            datafusion_distributed::DistributedLeafExec::try_new(Arc::clone(plan), variants)?,
        )))
    }
}
