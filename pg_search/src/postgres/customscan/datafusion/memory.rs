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

use datafusion::common::tree_node::{TreeNode, TreeNodeRecursion};
use datafusion::common::DataFusionError;
use datafusion::execution::disk_manager::{DiskManagerBuilder, DiskManagerMode};
use datafusion::execution::memory_pool::{GreedyMemoryPool, MemoryPool, MemoryReservation};
use datafusion::execution::runtime_env::{RuntimeEnv, RuntimeEnvBuilder};
use datafusion::physical_plan::joins::{HashJoinExec, SortMergeJoinExec};
use datafusion::physical_plan::sorts::sort::SortExec;
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties};
use std::sync::Arc;

/// Caps DataFusion allocations at PostgreSQL's `work_mem` and reports an overflow as a
/// query error. A panic here used to abort the backend: on the MPP path the pool runs in
/// a parallel worker, so the panic took down the whole server instead of failing the one
/// query. The caller pairs this with a disabled disk manager, so a `try_grow` past the
/// limit aborts the query rather than spilling to untracked temp files.
///
// TODO: spill through PostgreSQL's temp-file management (BufFile/VFD) so large
// aggregates and sorts complete instead of erroring once `work_mem` is exceeded.
#[derive(Debug)]
struct WorkMemMemoryPool {
    pool: GreedyMemoryPool,
    limit: usize,
}

impl WorkMemMemoryPool {
    fn new(limit: usize) -> Self {
        Self {
            pool: GreedyMemoryPool::new(limit),
            limit,
        }
    }
}

impl std::fmt::Display for WorkMemMemoryPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WorkMemMemoryPool")
    }
}

impl MemoryPool for WorkMemMemoryPool {
    fn name(&self) -> &str {
        "WorkMemMemoryPool"
    }

    fn grow(&self, reservation: &MemoryReservation, additional: usize) {
        self.pool.grow(reservation, additional);
    }

    fn shrink(&self, reservation: &MemoryReservation, returned: usize) {
        self.pool.shrink(reservation, returned);
    }

    fn try_grow(
        &self,
        reservation: &MemoryReservation,
        additional: usize,
    ) -> Result<(), DataFusionError> {
        self.pool.try_grow(reservation, additional).map_err(|_| {
            DataFusionError::ResourcesExhausted(format!(
                "query exceeded the work_mem limit of {} bytes; raise work_mem to run it",
                self.limit
            ))
        })
    }

    fn reserved(&self) -> usize {
        self.pool.reserved()
    }
}

/// Returns a memory pool that fails the query with a `ResourcesExhausted` error when the
/// `work_mem` budget is exceeded, rather than crashing the backend.
///
/// Sized for `JoinScan` and `AggregateScan`. The estimate is per-operator: `HashJoinExec`
/// reserves `work_mem * hash_mem_multiplier`, sorts reserve `work_mem`, each times their
/// partition count.
pub fn create_memory_pool(
    plan: &Arc<dyn ExecutionPlan>,
    work_mem: usize,
    hash_mem_multiplier: f64,
) -> Arc<dyn MemoryPool> {
    // estimate memory by walking through the plan
    let mut total_memory = 0usize;
    plan.apply(|node| {
        let partitions = node.output_partitioning().partition_count();
        if node.is::<HashJoinExec>() {
            total_memory += (work_mem as f64 * hash_mem_multiplier) as usize * partitions;
        } else if node.is::<SortExec>() || node.is::<SortMergeJoinExec>() {
            total_memory += work_mem * partitions;
        }
        Ok(TreeNodeRecursion::Continue)
    })
    .expect("Failed to traverse plan for estimating memory");

    Arc::new(WorkMemMemoryPool::new(total_memory.max(work_mem)))
}

/// Build the DataFusion `RuntimeEnv` for JoinScan and AggregateScan: the `work_mem` pool plus a
/// disabled disk manager, so a `try_grow` past the budget errors instead of writing untracked
/// temp files. Spilling isn't wired to PG's temp-file management yet.
pub fn build_runtime_env(memory_pool: Arc<dyn MemoryPool>) -> Arc<RuntimeEnv> {
    Arc::new(
        RuntimeEnvBuilder::new()
            .with_memory_pool(memory_pool)
            .with_disk_manager_builder(
                DiskManagerBuilder::default().with_mode(DiskManagerMode::Disabled),
            )
            .build()
            .expect("Failed to create RuntimeEnv"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::execution::memory_pool::MemoryConsumer;

    #[test]
    fn try_grow_past_work_mem_errors_instead_of_panicking() {
        let pool: Arc<dyn MemoryPool> = Arc::new(WorkMemMemoryPool::new(1024));
        let res = MemoryConsumer::new("test").register(&pool);
        res.try_grow(512).expect("within budget should succeed");
        let err = res
            .try_grow(4096)
            .expect_err("over budget must return an error, not panic");
        assert!(matches!(err, DataFusionError::ResourcesExhausted(_)));
        assert!(err.to_string().contains("work_mem"));
    }

    #[test]
    fn grow_is_infallible_past_work_mem() {
        // The infallible path must not panic even past the limit; only `try_grow` enforces.
        let pool: Arc<dyn MemoryPool> = Arc::new(WorkMemMemoryPool::new(1024));
        let res = MemoryConsumer::new("test").register(&pool);
        res.grow(8192);
        assert_eq!(pool.reserved(), 8192);
    }
}
