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

//! Delta from upstream: caps `BroadcastExec` subtrees at `task_count = 1`.
//!
//! Upstream DF-D's default estimator returns `Desired(n_workers)` for memory leaves, so a
//! `NetworkBroadcastExec` built over the canonical-replica all-gather step would get
//! `input_task_count = n_workers`. Every producer task would re-emit the full build side and the
//! consumer's `select_all` would over-count by `n_workers`. This estimator caps the build subtree
//! at one task, so the wire-layer `FragmentRouting::Broadcast` only ever sees `task_idx == 0`.
//!
//! Registered first in the DF-D `CombinedTaskEstimator` chain, which returns the first `Some(_)`.
//! Returns `None` for everything that isn't a `BroadcastExec`, so the default leaf estimator
//! handles the fallthrough.

use std::sync::Arc;

use datafusion::config::ConfigOptions;
use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::{BroadcastExec, TaskEstimation, TaskEstimator};

/// Caps every [`BroadcastExec`] subtree at `task_count = 1`.
///
/// Targeting `BroadcastExec` directly (instead of marking the leaf) survives DataFusion's
/// HashJoin build/probe reordering: the `BroadcastExec` always sits above whichever side ends up
/// as the build, so the cap lands at the right point regardless.
#[derive(Debug)]
pub struct BroadcastBuildSideOneTaskEstimator;

impl TaskEstimator for BroadcastBuildSideOneTaskEstimator {
    fn task_estimation(
        &self,
        plan: &Arc<dyn ExecutionPlan>,
        _: &ConfigOptions,
    ) -> Option<TaskEstimation> {
        if plan.is::<BroadcastExec>() {
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
        // `task_count` is `Maximum`, not `Desired`. Confirm the variant so an accidental
        // refactor that promotes it to `Desired` (and therefore loses the "hard cap" behaviour)
        // breaks the test.
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
        assert!(est
            .scale_up_leaf_node(&broadcast, 7, &cfg())
            .unwrap()
            .is_none());
    }
}
