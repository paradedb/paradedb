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
/// query. With `paradedb.spill_to_disk` off (the default), the caller pairs this with a disabled
/// disk manager (see [`spill_disk_manager`]), so a `try_grow` past the limit aborts the query;
/// with it on, sorts and aggregates spill instead.
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

/// Build the DataFusion `RuntimeEnv` for JoinScan and AggregateScan: the `work_mem` pool plus the
/// spill disk manager. With `paradedb.spill_to_disk` off (the default) spilling is disabled, so a
/// `try_grow` past the budget errors instead of writing temp files; with it on, sorts and
/// aggregates spill.
pub fn build_runtime_env(memory_pool: Arc<dyn MemoryPool>) -> Arc<RuntimeEnv> {
    Arc::new(
        RuntimeEnvBuilder::new()
            .with_memory_pool(memory_pool)
            .with_disk_manager_builder(spill_disk_manager())
            .build()
            .expect("Failed to create RuntimeEnv"),
    )
}
/// Disk-manager config for the JoinScan and AggregateScan DataFusion runtimes (serial and MPP),
/// gated on `paradedb.spill_to_disk`.
///
/// Off (default): spilling is disabled, so a `work_mem` overflow surfaces as the
/// `ResourcesExhausted` error from [`WorkMemMemoryPool`]. On: DataFusion spills sorts and
/// aggregates to a per-backend subdirectory of PostgreSQL's temp directory and the query
/// completes. DataFusion gives each `RuntimeEnv` its own `datafusion-*` sub-subdirectory and
/// removes it when the runtime drops, so concurrent fragments in one backend don't collide.
///
/// These spill files use DataFusion's own OS temp-file backend, so they don't count against
/// `temp_file_limit` and aren't tied to PG's per-transaction cleanup. PG removes `base/pgsql_tmp`
/// on restart, which clears anything an aborted query leaked.
///
// TODO: spill through PostgreSQL's BufFile so spill files respect `temp_file_limit` and
// `temp_tablespaces` and are cleaned up with the transaction, once DataFusion's spill engine
// takes a pluggable temp-file backend.
#[cfg(not(test))]
pub fn spill_disk_manager() -> DiskManagerBuilder {
    if !crate::gucs::spill_to_disk() {
        return DiskManagerBuilder::default().with_mode(DiskManagerMode::Disabled);
    }
    let dir = pg_search_spill_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        pgrx::warning!(
            "paradedb.spill_to_disk: could not create spill directory {}: {e}; spilling stays off",
            dir.display()
        );
        return DiskManagerBuilder::default().with_mode(DiskManagerMode::Disabled);
    }
    DiskManagerBuilder::default().with_mode(DiskManagerMode::Directories(vec![dir]))
}

/// The lib-test binary doesn't link PG globals and never drives a real query, so spilling is
/// inert there.
#[cfg(test)]
pub fn spill_disk_manager() -> DiskManagerBuilder {
    DiskManagerBuilder::default().with_mode(DiskManagerMode::Disabled)
}

/// Per-backend spill directory under PG's temp-file directory (`<DataDir>/base/pgsql_tmp`), so
/// spill files land on the data volume and PG's startup cleanup removes any crash residue.
#[cfg(not(test))]
fn pg_search_spill_dir() -> std::path::PathBuf {
    // SAFETY: `DataDir` and `MyProcPid` are set once at startup and read-only thereafter; this
    // runs on the backend thread.
    let data_dir = unsafe { std::ffi::CStr::from_ptr(pgrx::pg_sys::DataDir) }
        .to_string_lossy()
        .into_owned();
    let pid = unsafe { pgrx::pg_sys::MyProcPid };
    std::path::Path::new(&data_dir)
        .join("base")
        .join("pgsql_tmp")
        .join(format!("pgsql_tmp_pg_search.{pid}"))
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
