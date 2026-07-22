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
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::joins::{
    CrossJoinExec, HashJoinExec, NestedLoopJoinExec, PartitionMode,
};
use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::{BroadcastExec, TaskEstimation, TaskEstimator};

use crate::scan::execution_plan::PgSearchScanPlan;

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
        let _ = plan.downcast_ref::<PgSearchScanPlan>()?;

        let partition_count = plan.properties().output_partitioning().partition_count();

        Some(TaskEstimation::desired(partition_count))
    }

    fn scale_up_leaf_node(
        &self,
        plan: &Arc<dyn ExecutionPlan>,
        task_count: usize,
        _cfg: &ConfigOptions,
    ) -> datafusion::error::Result<Option<Arc<dyn ExecutionPlan>>> {
        if plan.downcast_ref::<PgSearchScanPlan>().is_none() {
            return Ok(None);
        }

        // Each worker decodes its own copy, and `execute()` dynamically claims segments from the
        // parallel state. Variants in the same process share state: see `ExecutionState` for why
        // this is safe.
        let variants = (0..task_count)
            .map(|_| Arc::clone(plan))
            .collect::<Vec<_>>();

        Ok(Some(Arc::new(
            datafusion_distributed::DistributedLeafExec::try_new(Arc::clone(plan), variants)?,
        )))
    }
}

/// Caps at one task every collect-build join whose build child is not a
/// [`BroadcastExec`].
///
/// A CollectLeft [`HashJoinExec`] (and the build loop of
/// [`NestedLoopJoinExec`] / [`CrossJoinExec`]) needs the complete build input
/// in every task. Our leaf scans claim segments from per-source work-stealing
/// pools, so in a multi-task stage each task sees only the segments it
/// happened to claim. `insert_broadcast_execs` replicates the build side only
/// for join types where that cannot duplicate output rows; when it declines
/// (Left / Full / LeftSemi / LeftAnti build sides), the join must collapse to
/// a single task or per-task builds are silently incomplete.
///
/// The check targets the direct build child (through the
/// `CoalescePartitionsExec` wrapper), not the whole subtree: a broadcast
/// buried deeper inside the build input replicates only that inner input, not
/// the build side of this join.
#[derive(Debug)]
pub struct CollectBuildNoBroadcastOneTaskEstimator;

fn build_child_is_broadcast(join: &dyn ExecutionPlan) -> bool {
    let Some(mut node) = join.children().first().copied() else {
        return false;
    };
    while let Some(coalesce) = node.downcast_ref::<CoalescePartitionsExec>() {
        node = coalesce.input();
    }
    node.is::<BroadcastExec>()
}

impl TaskEstimator for CollectBuildNoBroadcastOneTaskEstimator {
    fn task_estimation(
        &self,
        plan: &Arc<dyn ExecutionPlan>,
        _: &ConfigOptions,
    ) -> Option<TaskEstimation> {
        let collects_build = if let Some(hj) = plan.downcast_ref::<HashJoinExec>() {
            hj.partition_mode() == &PartitionMode::CollectLeft
        } else {
            plan.is::<NestedLoopJoinExec>() || plan.is::<CrossJoinExec>()
        };
        if collects_build && !build_child_is_broadcast(plan.as_ref()) {
            Some(TaskEstimation::maximum(1))
        } else {
            None
        }
    }

    fn scale_up_leaf_node(
        &self,
        _: &Arc<dyn ExecutionPlan>,
        _: usize,
        _: &ConfigOptions,
    ) -> datafusion::error::Result<Option<Arc<dyn ExecutionPlan>>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::common::{JoinType, NullEquality};
    use datafusion::physical_expr::expressions::Column;
    use datafusion::physical_expr::PhysicalExpr;
    use datafusion::physical_plan::empty::EmptyExec;

    fn cfg() -> ConfigOptions {
        ConfigOptions::default()
    }

    fn empty_leaf() -> Arc<dyn ExecutionPlan> {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        Arc::new(EmptyExec::new(schema))
    }

    fn hash_join(
        left: Arc<dyn ExecutionPlan>,
        right: Arc<dyn ExecutionPlan>,
        mode: PartitionMode,
    ) -> Arc<dyn ExecutionPlan> {
        let on = vec![(
            Arc::new(Column::new("x", 0)) as Arc<dyn PhysicalExpr>,
            Arc::new(Column::new("x", 0)) as Arc<dyn PhysicalExpr>,
        )];
        Arc::new(
            HashJoinExec::try_new(
                left,
                right,
                on,
                None,
                &JoinType::Left,
                None,
                mode,
                NullEquality::NullEqualsNothing,
                false,
            )
            .unwrap(),
        )
    }

    #[test]
    fn collect_left_without_broadcast_is_capped_at_one() {
        let join = hash_join(empty_leaf(), empty_leaf(), PartitionMode::CollectLeft);
        let est = CollectBuildNoBroadcastOneTaskEstimator;
        let out = est.task_estimation(&join, &cfg()).expect("estimation");
        assert!(matches!(
            out.task_count,
            datafusion_distributed::TaskCountAnnotation::Maximum(1)
        ));
    }

    #[test]
    fn collect_left_with_broadcast_build_falls_through() {
        let build: Arc<dyn ExecutionPlan> = Arc::new(CoalescePartitionsExec::new(Arc::new(
            BroadcastExec::new(empty_leaf(), 1),
        )));
        let join = hash_join(build, empty_leaf(), PartitionMode::CollectLeft);
        let est = CollectBuildNoBroadcastOneTaskEstimator;
        assert!(est.task_estimation(&join, &cfg()).is_none());
    }

    #[test]
    fn partitioned_hash_join_falls_through() {
        let join = hash_join(empty_leaf(), empty_leaf(), PartitionMode::Partitioned);
        let est = CollectBuildNoBroadcastOneTaskEstimator;
        assert!(est.task_estimation(&join, &cfg()).is_none());
    }

    #[test]
    fn nested_loop_join_without_broadcast_is_capped_at_one() {
        let join: Arc<dyn ExecutionPlan> = Arc::new(
            NestedLoopJoinExec::try_new(empty_leaf(), empty_leaf(), None, &JoinType::Left, None)
                .unwrap(),
        );
        let est = CollectBuildNoBroadcastOneTaskEstimator;
        let out = est.task_estimation(&join, &cfg()).expect("estimation");
        assert!(matches!(
            out.task_count,
            datafusion_distributed::TaskCountAnnotation::Maximum(1)
        ));
    }
}
