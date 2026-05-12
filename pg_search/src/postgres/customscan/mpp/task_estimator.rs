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

//! Planner-level companion to the dispatcher's
//! [`FragmentRouting::Broadcast`] runtime guard.
//!
//! pg_search's natural-shape Aggregate plan canonical-replicates the
//! HashJoin build subtree across all workers via the `mpp build all-gather`
//! step. That step replaces the per-worker scan of the canonical source
//! with a single `DataSourceExec(MemorySourceConfig)` that returns the
//! fully replicated data. The DF-D fork's default task estimator would
//! still annotate this leaf with `Desired(n_workers)`, so the resulting
//! `NetworkBroadcastExec` would be built with `input_task_count =
//! n_workers` and each input task would emit the full canonical data —
//! the consumer's `select_all` then over-counts by `input_task_count`.
//!
//! [`BroadcastBuildSideOneTaskEstimator`] caps every all-gather memory
//! leaf at `TaskEstimation::maximum(1)`, which propagates up the build
//! subtree and produces a `NetworkBroadcastExec(input_task_count=1)`. The
//! single producer emits the canonical data once per consumer task, and
//! the consumer's `select_all` sees one real stream + empty placeholders
//! from the missing input tasks.
//!
//! Installed via [`datafusion_distributed::SessionStateBuilderExt::with_distributed_task_estimator`]
//! in front of the default per-leaf estimator, so non-canonical leaves
//! (`PgSearchScanPlan` and friends) fall through to the default. With
//! this estimator in place, the dispatcher's
//! [`FragmentRouting::Broadcast`] short-circuit becomes a defensive
//! `debug_assert!(fragment.task_idx == 0)` rather than a load-bearing
//! correctness patch — see the doc on
//! [`crate::postgres::customscan::mpp::worker_fragments::FragmentRouting::Broadcast`].

use std::sync::Arc;

use datafusion::config::ConfigOptions;
use datafusion::datasource::memory::DataSourceExec;
use datafusion::physical_plan::ExecutionPlan;
use datafusion_datasource::memory::MemorySourceConfig;
use datafusion_distributed::{TaskEstimation, TaskEstimator};

/// Caps every `DataSourceExec(MemorySourceConfig)` leaf at task_count=1.
///
/// In pg_search the only call site that produces this exact pair is
/// `memory_exec_for_cached` in `scan/table_provider.rs`, which materialises
/// the all-gathered canonical replica of an MPP build subtree. Capping the
/// leaf at 1 propagates up through `BroadcastExec` and tells
/// `_distribute_plan` to build a `NetworkBroadcastExec` with
/// `input_task_count = 1`.
#[derive(Debug)]
pub struct BroadcastBuildSideOneTaskEstimator;

impl TaskEstimator for BroadcastBuildSideOneTaskEstimator {
    fn task_estimation(
        &self,
        plan: &Arc<dyn ExecutionPlan>,
        _: &ConfigOptions,
    ) -> Option<TaskEstimation> {
        if !plan.children().is_empty() {
            return None;
        }
        let exec = plan.as_any().downcast_ref::<DataSourceExec>()?;
        if exec
            .data_source()
            .as_any()
            .downcast_ref::<MemorySourceConfig>()
            .is_some()
        {
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
    ) -> Option<Arc<dyn ExecutionPlan>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::array::Int32Array;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::arrow::record_batch::RecordBatch;
    use datafusion::physical_plan::empty::EmptyExec;

    fn cfg() -> ConfigOptions {
        ConfigOptions::default()
    }

    #[test]
    fn memory_leaf_is_capped_at_one() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        let batch = RecordBatch::try_new(
            Arc::clone(&schema),
            vec![Arc::new(Int32Array::from(vec![1, 2, 3]))],
        )
        .expect("build batch");
        let exec: Arc<dyn ExecutionPlan> =
            MemorySourceConfig::try_new_exec(&[vec![batch]], schema, None)
                .expect("build MemoryExec");
        let est = BroadcastBuildSideOneTaskEstimator;
        let out = est.task_estimation(&exec, &cfg()).expect("estimation");
        // `TaskEstimation::maximum(1)` is what propagates up to
        // `NetworkBroadcastExec::input_task_count = 1`.
        assert_eq!(out.task_count.as_usize(), 1);
        // `task_count` is `Maximum`, not `Desired` — confirm the variant
        // so an accidental refactor that promotes it to `Desired` (and
        // therefore loses the "hard cap" behaviour) breaks the test.
        assert!(matches!(
            out.task_count,
            datafusion_distributed::TaskCountAnnotation::Maximum(1)
        ));
    }

    #[test]
    fn non_memory_leaf_falls_through() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        let plan: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(schema));
        let est = BroadcastBuildSideOneTaskEstimator;
        assert!(est.task_estimation(&plan, &cfg()).is_none());
    }

    #[test]
    fn scale_up_is_a_no_op() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        let batch = RecordBatch::try_new(
            Arc::clone(&schema),
            vec![Arc::new(Int32Array::from(vec![1]))],
        )
        .expect("build batch");
        let exec: Arc<dyn ExecutionPlan> =
            MemorySourceConfig::try_new_exec(&[vec![batch]], schema, None).expect("build");
        let est = BroadcastBuildSideOneTaskEstimator;
        assert!(est.scale_up_leaf_node(&exec, 7, &cfg()).is_none());
    }
}
