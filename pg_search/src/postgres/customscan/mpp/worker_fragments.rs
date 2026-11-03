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

//! Leader-side producer-stage discovery for dispatch. "Fragment" here means a plan fragment
//! (one task of a producer stage), not the frame fragmentation the ring does for oversized
//! messages.
//!
//! [`collect_stages`] walks the distributed physical plan and visits every
//! [`datafusion_distributed::NetworkBoundary`], turning each stage that carries a local plan into
//! one [`StageEntry`] (`stage_num`, `task_count`). The stage plans travel separately, serialized by
//! the coordinator's dispatch; each worker later expands a stage into one [`FragmentAssignment`]
//! per `task_idx` it owns under `proc_for_task`. Output partition routing is demand-driven at
//! runtime via `ExecuteTaskFrame`.

use std::sync::Arc;

use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::NetworkBoundaryExt;

/// One worker fragment to run for `this_proc`. The fragment is one task of a
/// producer stage; the dispatcher runs `plan` with the matching
/// `DistributedTaskContext { task_index: task_idx, task_count }`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentAssignment {
    /// `input_stage.num` of the boundary whose producer side this fragment
    /// belongs to. Frames the fragment emits carry this in the
    /// `MppFrameHeader::stage_id` field.
    pub stage_id: u32,
    /// Task index within the stage (0..task_count), validated for the shared-memory transport.
    pub task_idx: u32,
    /// Total task count for this stage (= `input_stage.tasks.len()`).
    pub task_count: usize,
}

/// One producer stage to dispatch: the sizing metadata the DSM dispatch blob carries. Each worker
/// expands a stage into one [`FragmentAssignment`] per `task_idx` it owns under `proc_for_task`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StageEntry {
    /// `input_stage.num` of the boundary whose producer side this stage belongs to.
    pub stage_num: u32,
    /// Total task count for the stage (= `input_stage.tasks.len()`).
    pub task_count: usize,
}

/// Every dispatchable producer stage, once per boundary. The launch walks before forking and hands
/// the result to both [`max_producer_task_count`] and `dispatch_payload_from_stages`, so the two
/// cannot disagree about which stages exist (#5667). Worker ownership remains deferred until each
/// worker applies the actual attached width.
pub(crate) fn collect_stages(root: &Arc<dyn ExecutionPlan>) -> Vec<StageEntry> {
    let mut out = Vec::new();
    walk_stages(root, &mut out);
    out
}

fn walk_stages(plan: &Arc<dyn ExecutionPlan>, out: &mut Vec<StageEntry>) {
    if let Some(nb) = plan.as_ref().as_network_boundary() {
        let stage = nb.input_stage();
        if let Some(stage_plan) = stage.local_plan() {
            out.push(StageEntry {
                stage_num: stage.num() as u32,
                task_count: stage.task_count(),
            });
            // Recurse into the stage's plan. The boundary's `children()` returns `[stage.plan]`,
            // so descending through `plan.children()` would double-process every nested stage.
            walk_stages(stage_plan, out);
        }
        return;
    }
    for child in plan.children() {
        walk_stages(child, out);
    }
}

/// Largest producer-stage task count, or 0 when the plan has no network boundaries (#5667). The
/// plan-first launch sizes the worker pool from this number before any DSM or process exists:
/// `producers = clamp(max_tasks, 2, cap)`.
pub(crate) fn max_producer_task_count(stages: &[StageEntry]) -> usize {
    stages
        .iter()
        .map(|stage| stage.task_count)
        .max()
        .unwrap_or(0)
}

/// True when discovered stages have enough producer tasks for MPP to launch (#5784).
/// Same `>= 2` gate as the plan-first path in [`crate::postgres::customscan::mpp::launch`]:
/// below that there is no data parallelism, so execution runs serially and plain EXPLAIN
/// must render the serial shape rather than a cap-sized distributed plan.
pub(crate) fn stages_have_data_parallelism(stages: &[StageEntry]) -> bool {
    max_producer_task_count(stages) >= 2
}

/// Convenience wrapper over [`stages_have_data_parallelism`] for a finished physical plan.
pub(crate) fn mpp_plan_has_data_parallelism(plan: &Arc<dyn ExecutionPlan>) -> bool {
    stages_have_data_parallelism(&collect_stages(plan))
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::physical_plan::empty::EmptyExec;

    #[test]
    fn boundary_free_plan_yields_no_stages() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        let plan: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(schema));
        let out = collect_stages(&plan);
        assert!(out.is_empty());
    }

    /// #5667: sizing and dispatch read the same enumeration, so a boundary-free plan has nothing
    /// to distribute and the launch spawns no workers at all.
    #[test]
    fn boundary_free_plan_has_zero_max_tasks() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
        let plan: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(schema));
        assert_eq!(max_producer_task_count(&collect_stages(&plan)), 0);
        assert!(!mpp_plan_has_data_parallelism(&plan));
    }
}
