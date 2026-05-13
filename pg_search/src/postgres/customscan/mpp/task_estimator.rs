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
//! in front of the default per-leaf estimator. The DF-D fork's
//! `CombinedTaskEstimator` iterates registered estimators and returns
//! the FIRST `Some(_)` — registration order, not a "biggest task_count
//! wins" tiebreak. (That tiebreak runs at the per-stage layer across
//! distinct leaves, not within one leaf's estimator chain.) So this
//! estimator returns `Some(Maximum(1))` for canonical-replica memory
//! leaves and `None` everywhere else; the default `Desired(n_workers)`
//! estimator handles the fallthrough.
//!
//! With this estimator in place, the dispatcher's
//! [`FragmentRouting::Broadcast`] short-circuit becomes a defensive
//! `debug_assert!(fragment.task_idx == 0)` rather than a load-bearing
//! correctness patch — see the doc on
//! [`crate::postgres::customscan::mpp::worker_fragments::FragmentRouting::Broadcast`].

use std::sync::Arc;

use datafusion::config::ConfigOptions;
use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::{BroadcastExec, TaskEstimation, TaskEstimator};

/// Caps every [`BroadcastExec`] subtree at `task_count = 1`.
///
/// In pg_search every CollectLeft hash join with `broadcast_joins=true`
/// produces a `BroadcastExec` over the (smaller) build side, and pg_search's
/// scan model treats that build side as a single canonical replica — the
/// same data on every worker. Capping the `BroadcastExec`'s task_count at 1
/// propagates upward and tells `_distribute_plan` to build a
/// `NetworkBroadcastExec` with `input_task_count = 1`: a single producer
/// task scans the build side and the fan-out replicates that stream to
/// every consumer task.
///
/// Targeting `BroadcastExec` directly (rather than a marker wrapper on the
/// leaf) survives DataFusion's HashJoin reordering — the planner may flip
/// build/probe based on cost, but the `BroadcastExec` always sits above the
/// final build side, so the cap is applied at the right point regardless of
/// which source ends up there.
#[derive(Debug)]
pub struct BroadcastBuildSideOneTaskEstimator;

impl TaskEstimator for BroadcastBuildSideOneTaskEstimator {
    fn task_estimation(
        &self,
        plan: &Arc<dyn ExecutionPlan>,
        _: &ConfigOptions,
    ) -> Option<TaskEstimation> {
        if plan.as_any().downcast_ref::<BroadcastExec>().is_some() {
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
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::physical_plan::empty::EmptyExec;

    fn cfg() -> ConfigOptions {
        ConfigOptions::default()
    }

    fn empty_leaf() -> Arc<dyn ExecutionPlan> {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        Arc::new(EmptyExec::new(schema))
    }

    #[test]
    fn broadcast_exec_is_capped_at_one() {
        let inner = empty_leaf();
        let broadcast: Arc<dyn ExecutionPlan> = Arc::new(BroadcastExec::new(inner, 1));
        let est = BroadcastBuildSideOneTaskEstimator;
        let out = est.task_estimation(&broadcast, &cfg()).expect("estimation");
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
    fn non_broadcast_node_falls_through() {
        let plan = empty_leaf();
        let est = BroadcastBuildSideOneTaskEstimator;
        assert!(est.task_estimation(&plan, &cfg()).is_none());
    }

    #[test]
    fn scale_up_is_a_no_op() {
        let inner = empty_leaf();
        let broadcast: Arc<dyn ExecutionPlan> = Arc::new(BroadcastExec::new(inner, 1));
        let est = BroadcastBuildSideOneTaskEstimator;
        assert!(est.scale_up_leaf_node(&broadcast, 7, &cfg()).is_none());
    }
}
