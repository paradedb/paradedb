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

//! Plan-driven `(stage_id, task_idx) → proc_idx` assignment table for
//! [`crate::postgres::customscan::mpp::runtime::ShmMqWorkerTransport`].
//!
//! M1 used a single-stage heuristic (`sender_proc_for_task = task + 1`).
//! That accidentally produces the right answer when there is exactly one
//! producer stage with `task_count == n_workers` (the natural-shape gather)
//! and the wrong answer for any peer-mesh shuffle where Stage 2's tasks
//! get the same indices as Stage 1's tasks but should host different work.
//!
//! M2.c builds an explicit table by walking the physical plan: for every
//! [`datafusion_distributed::NetworkBoundary`] node, the table records
//! `(input_stage.num, task_idx) → proc_idx` for each task. The current
//! policy is round-robin over the worker procs:
//!
//! ```text
//! proc_idx = 1 + (task_idx % n_workers)
//! ```
//!
//! ...which means tasks 0..n_workers map 1:1 onto workers 1..n_procs and any
//! tasks past that wrap. With the planner's `target_partitions = n_workers`
//! and `distributed_task_estimator = n_workers` knobs (set in the
//! AggregateScan leader session context), the natural-shape plans never
//! exceed `n_workers` tasks per stage, so wrap-around is dormant today.
//! Encoding it now means later milestones can grow stages independently
//! without revisiting this rule.
//!
//! The table is shared via [`MppMesh::install_assignment`]; the transport
//! reads it through `MppMesh::task_assignment()`. M2.d will introduce
//! per-worker meshes that share the same table so worker→worker peer-mesh
//! routing is consistent across procs.
//!
//! [`MppMesh`]: crate::postgres::customscan::mpp::runtime::MppMesh
//! [`MppMesh::install_assignment`]: crate::postgres::customscan::mpp::runtime::MppMesh::install_assignment

use std::collections::HashMap;
use std::sync::Arc;

use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::NetworkBoundaryExt;

/// Plan-derived mapping from `(stage_id, task_idx)` to the `proc_idx` that
/// hosts that task. Leader is `proc_idx = 0`; workers are `1..n_procs`.
///
/// Construction goes through [`Self::from_plan`] so the table reflects the
/// concrete physical plan we're about to execute — particularly the
/// `stage.tasks.len()` per [`Stage`], which is what the DF-D planner picked
/// for each [`NetworkBoundary`].
///
/// [`Stage`]: datafusion_distributed::Stage
/// [`NetworkBoundary`]: datafusion_distributed::NetworkBoundary
#[derive(Debug, Clone)]
pub struct TaskAssignment {
    map: HashMap<(u32, u32), u32>,
    /// Total participant count (leader + workers). Held for diagnostics and
    /// for the wrap-around math; the table itself is the authoritative
    /// lookup.
    #[allow(dead_code)]
    n_procs: u32,
}

impl TaskAssignment {
    /// Build an empty table. Useful for tests; production goes through
    /// [`Self::from_plan`].
    #[allow(dead_code)]
    pub fn empty(n_procs: u32) -> Arc<Self> {
        Arc::new(Self {
            map: HashMap::new(),
            n_procs,
        })
    }

    /// Walk `plan`, collect every [`NetworkBoundary`] node's input stage, and
    /// assign each task to a proc via round-robin over the worker procs
    /// (`1..n_procs`).
    ///
    /// The walk descends through `Stage.plan()` for each network boundary so
    /// nested stages (peer-mesh shuffles below a top-level gather) are
    /// visible. Duplicate visits to the same `(stage_num, task_idx)` overwrite
    /// with the same value — harmless given the deterministic round-robin
    /// policy.
    ///
    /// Precondition: `n_procs >= 2` (one leader + at least one worker). The
    /// production gate at `glue::mpp_is_active` already enforces
    /// `mpp_worker_count >= 3`, so callers downstream of the customscan
    /// activation always satisfy this. The `debug_assert` exists to catch
    /// future direct callers that miss the gate.
    ///
    /// [`NetworkBoundary`]: datafusion_distributed::NetworkBoundary
    pub fn from_plan(plan: &Arc<dyn ExecutionPlan>, n_procs: u32) -> Arc<Self> {
        debug_assert!(
            n_procs >= 2,
            "TaskAssignment::from_plan requires n_procs >= 2 (1 leader + N workers), got {n_procs}"
        );
        let n_workers = n_procs.saturating_sub(1).max(1);
        let mut map = HashMap::new();
        collect_into(plan, &mut map, n_workers);
        Arc::new(Self { map, n_procs })
    }

    /// Look up the proc that hosts `(stage_id, task_idx)`. Returns `None` if
    /// the pair isn't in the table — caller decides whether to fall back to
    /// a heuristic or surface an error.
    pub fn proc_for(&self, stage_id: u32, task_idx: u32) -> Option<u32> {
        self.map.get(&(stage_id, task_idx)).copied()
    }

    /// Inverse lookup: every `(stage_id, task_idx)` slot hosted by
    /// `proc_idx`. The worker side consumes this in M2.d to know which
    /// fragments it must run; today this is exercised in tests only.
    ///
    /// Keys are unique by construction, so `sort_unstable` is sufficient
    /// for a deterministic order.
    #[allow(dead_code)]
    pub fn slots_for_proc(&self, proc_idx: u32) -> Vec<(u32, u32)> {
        let mut out: Vec<(u32, u32)> = self
            .map
            .iter()
            .filter_map(|(&k, &p)| if p == proc_idx { Some(k) } else { None })
            .collect();
        out.sort_unstable();
        out
    }

    /// Iterate `(stage_id, task_idx, proc_idx)` tuples. Exposed for
    /// debugging and tests.
    #[allow(dead_code)]
    pub fn iter(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        self.map.iter().map(|(&(s, t), &p)| (s, t, p))
    }

    /// Number of entries in the table.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

/// Recursive plan walker. On every [`NetworkBoundary`], records the input
/// stage's per-task assignments, recurses into `stage.plan()` so nested
/// stages contribute too, and ALSO recurses into `plan.children()` for
/// defensiveness — the DF-D fork's `find_input_stages` (private) uses the
/// children path, and the two would diverge only if DF-D ever exposes a
/// boundary node with children other than its input subplan. Duplicate
/// visits hit `HashMap::insert` with the same value (deterministic policy),
/// so the defensive recursion is idempotent.
///
/// [`NetworkBoundary`]: datafusion_distributed::NetworkBoundary
fn collect_into(plan: &Arc<dyn ExecutionPlan>, map: &mut HashMap<(u32, u32), u32>, n_workers: u32) {
    if let Some(nb) = plan.as_ref().as_network_boundary() {
        let stage = nb.input_stage();
        let stage_num = stage.num() as u32;
        for task_idx in 0..stage.tasks.len() {
            let task_idx_u32 = task_idx as u32;
            let proc_idx = 1 + (task_idx_u32 % n_workers);
            map.insert((stage_num, task_idx_u32), proc_idx);
        }
        if let Some(inner) = stage.plan() {
            collect_into(inner, map, n_workers);
        }
    }
    // Non-boundary nodes always recurse; boundary nodes recurse defensively
    // in case `children()` carries something `stage.plan()` doesn't.
    for child in plan.children() {
        collect_into(child, map, n_workers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_returns_none_for_any_key() {
        let a = TaskAssignment::empty(4);
        assert_eq!(a.proc_for(0, 0), None);
        assert_eq!(a.proc_for(5, 3), None);
        assert_eq!(a.len(), 0);
    }

    #[test]
    fn round_robin_assigns_workers_starting_from_proc_1() {
        // Synthetic table that mirrors what `from_plan` would compute for
        // a single stage of T tasks with n_workers = 3.
        let n_procs = 4u32; // 1 leader + 3 workers
        let n_workers = n_procs - 1;
        let mut map = HashMap::new();
        for task_idx in 0..5u32 {
            map.insert((0, task_idx), 1 + (task_idx % n_workers));
        }
        let a = Arc::new(TaskAssignment { map, n_procs });
        assert_eq!(a.proc_for(0, 0), Some(1));
        assert_eq!(a.proc_for(0, 1), Some(2));
        assert_eq!(a.proc_for(0, 2), Some(3));
        // Wrap.
        assert_eq!(a.proc_for(0, 3), Some(1));
        assert_eq!(a.proc_for(0, 4), Some(2));
    }

    #[test]
    fn slots_for_proc_is_inverse_of_proc_for() {
        let n_procs = 5u32; // 4 workers
        let n_workers = n_procs - 1;
        let mut map = HashMap::new();
        // Two stages, S0 with 4 tasks, S1 with 2 tasks.
        for task_idx in 0..4u32 {
            map.insert((0, task_idx), 1 + (task_idx % n_workers));
        }
        for task_idx in 0..2u32 {
            map.insert((1, task_idx), 1 + (task_idx % n_workers));
        }
        let a = Arc::new(TaskAssignment { map, n_procs });

        // proc 1 hosts (0, 0) and (1, 0).
        assert_eq!(a.slots_for_proc(1), vec![(0, 0), (1, 0)]);
        // proc 2 hosts (0, 1) and (1, 1).
        assert_eq!(a.slots_for_proc(2), vec![(0, 1), (1, 1)]);
        // proc 3 hosts only (0, 2).
        assert_eq!(a.slots_for_proc(3), vec![(0, 2)]);
        // proc 4 hosts only (0, 3).
        assert_eq!(a.slots_for_proc(4), vec![(0, 3)]);
        // Leader hosts nothing in the producer-side table.
        assert_eq!(a.slots_for_proc(0), Vec::<(u32, u32)>::new());
    }

    #[test]
    fn from_plan_on_boundary_free_plan_is_empty() {
        // A plan with no NetworkBoundary nodes (just a leaf `EmptyExec`) must
        // yield an empty assignment. Exercises the early-return path of
        // `collect_into` end-to-end through `from_plan`.
        use datafusion::arrow::datatypes::{Field, Schema};
        use datafusion::physical_plan::empty::EmptyExec;
        let schema = Arc::new(Schema::new(vec![Field::new(
            "x",
            datafusion::arrow::datatypes::DataType::Int32,
            false,
        )]));
        let plan: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(schema));
        let a = TaskAssignment::from_plan(&plan, 4);
        assert_eq!(a.len(), 0);
        assert_eq!(a.proc_for(0, 0), None);
    }
}
