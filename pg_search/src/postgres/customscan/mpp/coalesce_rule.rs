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

//! Coalesce small batches in front of every [`NetworkShuffleExec`] so the producer ships fewer,
//! larger Arrow IPC frames over shm_mq.
//!
//! Why this matters. The natural-shape `GroupByAggOnBinaryJoin` pipeline used to insert
//! `CoalesceBatchesExec(65_536)` between the partial aggregate and the post-aggregate shuffle by
//! hand (see the "Phase 8" note in the perf cliff memory). The multi-stage rewrite (#5082)
//! switched to DataFusion's planner for the physical plan, and DF's built-in `CoalesceBatches`
//! rule only wraps [`RepartitionExec`] — it doesn't know about the DF-D
//! [`NetworkShuffleExec`] that replaces it in our distributed plans. Without the explicit
//! coalesce, a high-cardinality partial aggregate (e.g. 3M distinct title groups out of 20M
//! rows) emits many small batches, the shuffle ships each one as its own shm_mq frame, and the
//! sender's spin-loop spends most of its time on `try_send_bytes` / encode overhead rather than
//! payload throughput. The 20M `aggregate_join_groupby - alternative 2` regression caught by
//! the bench is exactly that shape.
//!
//! The rule walks the physical plan post-DF-D-planning. For every `NetworkShuffleExec` it sees,
//! it wraps the child plan in `CoalesceBatchesExec(target_batch_size)` so each input batch fed
//! to the shuffle hits roughly that row count. Larger batches mean fewer shm_mq round-trips and
//! amortise per-frame fixed cost (header encode, channel routing, scratch reuse). Already-large
//! batches pass through unchanged — `CoalesceBatchesExec` is cheap when there's nothing to
//! coalesce.

use std::sync::Arc;

use datafusion::common::tree_node::{Transformed, TreeNode};
use datafusion::config::ConfigOptions;
use datafusion::error::Result;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
// `CoalesceBatchesExec` is marked deprecated since DF 52 in favor of arrow-rs's
// `BatchCoalescer`. The replacement isn't an `ExecutionPlan`, though — it's an internal helper
// other operators embed. We need a node in the physical plan tree so DF-D's planner walks past
// it without rewriting, hence we stick with the deprecated op until DF ships a public
// coalesce-only operator built on `BatchCoalescer`. The functionality is identical; only the
// preferred construction site has moved.
#[allow(deprecated)]
use datafusion::physical_plan::coalesce_batches::CoalesceBatchesExec;
use datafusion::physical_plan::ExecutionPlan;

/// Target rows per coalesced batch fed into a [`NetworkShuffleExec`].
///
/// Picked to match the natural-shape Phase 8 tuning (which dropped `aggregate_join_groupby` from
/// 21.5s → 5.2s at 25M local). At ~80 B/row a 65 536-row batch is ~5 MB encoded, so the default
/// 64 MiB shm_mq queue holds ~12 batches in flight — enough pipelining without filling the queue
/// on a single send.
const SHUFFLE_INPUT_TARGET_BATCH_ROWS: usize = 65_536;

/// Physical-optimizer rule: insert a `CoalesceBatchesExec` directly under every
/// `NetworkShuffleExec`. Idempotent: if the child is already a `CoalesceBatchesExec` with our
/// target size we leave it alone, so re-running the rule (or stacking another optimizer pass
/// that calls `optimize` again) doesn't double-wrap.
pub struct CoalesceBeforeNetworkShuffleRule;

impl std::fmt::Debug for CoalesceBeforeNetworkShuffleRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoalesceBeforeNetworkShuffleRule").finish()
    }
}

impl PhysicalOptimizerRule for CoalesceBeforeNetworkShuffleRule {
    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        plan.transform_up(|plan| {
            // Match by ExecutionPlan::name() rather than downcasting, since `NetworkShuffleExec`
            // lives in the datafusion_distributed crate and we'd otherwise need a direct
            // dependency on its concrete type. Name matching is what DF-D itself uses for plan
            // recognition (see `find_worker_assignments` in worker_fragments.rs).
            if plan.name() != "NetworkShuffleExec" {
                return Ok(Transformed::no(plan));
            }

            let children = plan.children();
            let mut new_children: Vec<Arc<dyn ExecutionPlan>> = Vec::with_capacity(children.len());
            let mut changed = false;
            for child in children {
                if is_coalesce_with_target(child.as_ref(), SHUFFLE_INPUT_TARGET_BATCH_ROWS) {
                    new_children.push(Arc::clone(child));
                } else {
                    #[allow(deprecated)]
                    let coalesced: Arc<dyn ExecutionPlan> = Arc::new(CoalesceBatchesExec::new(
                        Arc::clone(child),
                        SHUFFLE_INPUT_TARGET_BATCH_ROWS,
                    ));
                    new_children.push(coalesced);
                    changed = true;
                }
            }

            if !changed {
                return Ok(Transformed::no(plan));
            }
            let new_plan = plan.with_new_children(new_children)?;
            Ok(Transformed::yes(new_plan))
        })
        .map(|t| t.data)
    }

    fn name(&self) -> &str {
        "CoalesceBeforeNetworkShuffle"
    }

    fn schema_check(&self) -> bool {
        true
    }
}

fn is_coalesce_with_target(plan: &dyn ExecutionPlan, target: usize) -> bool {
    #[allow(deprecated)]
    let coal = plan.as_any().downcast_ref::<CoalesceBatchesExec>();
    coal.is_some_and(|cb| cb.target_batch_size() == target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::physical_plan::empty::EmptyExec;
    use std::sync::Arc;

    fn empty_plan() -> Arc<dyn ExecutionPlan> {
        let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));
        Arc::new(EmptyExec::new(schema))
    }

    #[test]
    fn rule_leaves_plans_without_shuffle_alone() {
        let plan = empty_plan();
        let cfg = ConfigOptions::new();
        let out = CoalesceBeforeNetworkShuffleRule
            .optimize(Arc::clone(&plan), &cfg)
            .unwrap();
        // No NetworkShuffleExec → no transformation → same Arc.
        assert!(Arc::ptr_eq(&plan, &out));
    }

    #[test]
    fn rule_is_idempotent_when_child_is_already_coalesce_with_target() {
        // We can't easily construct a NetworkShuffleExec in unit tests without bringing in the
        // full datafusion_distributed harness, but we can exercise the helper that decides
        // whether to skip wrapping.
        let base = empty_plan();
        #[allow(deprecated)]
        let coal: Arc<dyn ExecutionPlan> = Arc::new(CoalesceBatchesExec::new(
            base,
            SHUFFLE_INPUT_TARGET_BATCH_ROWS,
        ));
        assert!(is_coalesce_with_target(
            coal.as_ref(),
            SHUFFLE_INPUT_TARGET_BATCH_ROWS,
        ));
        assert!(!is_coalesce_with_target(coal.as_ref(), 8_192));
    }

    #[test]
    fn rule_does_not_match_plain_coalesce() {
        #[allow(deprecated)]
        let plan: Arc<dyn ExecutionPlan> = Arc::new(CoalesceBatchesExec::new(empty_plan(), 8_192));
        assert!(!is_coalesce_with_target(
            plan.as_ref(),
            SHUFFLE_INPUT_TARGET_BATCH_ROWS,
        ));
    }
}
