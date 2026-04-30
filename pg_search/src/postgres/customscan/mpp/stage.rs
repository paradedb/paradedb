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

#![allow(dead_code)]
//! Stage / TaskKey descriptors and the `MppNetworkBoundary` trait.
//!
//! Ported from datafusion-contrib/datafusion-distributed's
//! `src/distributed_planner/network_boundary.rs` and `src/stage.rs`, with the
//! transport-specific bits (Arrow Flight / gRPC `ExecutionTask.url`) dropped.
//! In a PG parallel-worker world a "task" is a peer participant in the mesh, so
//! `task_count` is enough — we don't need URLs or a `WorkerResolver`.
//!
//! Live consumers: the walker (`walker::emit_shuffle_cut` ➜
//! `plan_build::wrap_with_mpp_shuffle`) stamps an [`MppStage`] on every
//! [`ShuffleExec`] / [`DrainGatherExec`] it emits via the
//! [`MppNetworkBoundary::with_input_stage`] helper. The receiver-side
//! validation that consumes the stamp lives in P5b — until then the
//! frame-header bytes carry the `(query_id, stage_id, task_number,
//! partition)` tuple and decoders discard it after a length check.

use std::sync::Arc;

use datafusion::common::Result as DFResult;
use datafusion::physical_plan::ExecutionPlan;

/// Identifies a sub-plan rooted at a network boundary.
///
/// One `MppStage` per boundary: the generic cut-rule walker assigns a monotonic
/// `stage_id` as it walks bottom-up, so the leaf stage is 0 and each boundary
/// above it increments. `task_count` is the number of parallel tasks in the
/// child sub-plan — for an in-process PG MPP query this equals
/// `MppParticipantConfig.total_participants` (i.e. one task per participant), but we
/// keep it as a separate field to mirror datafusion-distributed's shape and
/// leave room for future task-fan-out schemes.
///
/// `query_id` is currently set by the leader at plan time. A `u64` suffices
/// for single-backend uniqueness (the query never crosses processes outside of
/// the spawned parallel workers, which all inherit it through DSM). If we
/// later want to address queries across unrelated backends we can upgrade to
/// `uuid::Uuid` without changing the trait surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MppStage {
    pub query_id: u64,
    pub stage_id: u32,
    pub task_count: u32,
}

impl MppStage {
    pub fn new(query_id: u64, stage_id: u32, task_count: u32) -> Self {
        Self {
            query_id,
            stage_id,
            task_count,
        }
    }
}

/// Wire identifier for a single stream between two participants.
///
/// Mirrors datafusion-distributed's `TaskKey` protobuf
/// (`src/worker/worker.proto:51-59`). Carried on every framed batch as
/// part of `transport::FrameId` so one `shm_mq` between two participants
/// can in principle carry multiple multiplexed streams (one per (stage,
/// task, partition) tuple). Today's mesh allocates one shm_mq per
/// `(stage, src, dst)` so the multiplex isn't strictly needed, but
/// stamping the descriptor now lets P5b's channel-flatten dispatcher
/// land without a wire-format break.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MppTaskKey {
    pub query_id: u64,
    pub stage_id: u32,
    pub task_number: u32,
}

/// Trait implemented by every MPP `ExecutionPlan` that represents a cut
/// between two stages — i.e. data crosses an `shm_mq` at this node.
///
/// Ported from datafusion-distributed
/// (`src/distributed_planner/network_boundary.rs:8-20`). We keep the
/// signatures identical so the upcoming generic cut-rule walker (P3) can be
/// ported verbatim: the walker's only dependency on boundary nodes is this
/// trait.
///
/// P1 is intentionally permissive: `input_stage` returns `Option<&MppStage>`
/// because existing call sites construct `ShuffleExec` / `DrainGatherExec`
/// without a stage. P3's walker will stamp one via [`with_input_stage`] during
/// the `transform_up` pass. Once every boundary is walker-produced we can
/// tighten the signature to `&MppStage` and delete the `Option`.
pub trait MppNetworkBoundary: ExecutionPlan {
    /// Return the stage this boundary consumes from (i.e. the sub-plan
    /// beneath the cut). Returns `None` only on test fixtures that
    /// construct boundaries directly without going through the walker;
    /// every walker-emitted boundary has a stamped stage. Read-side
    /// consumer (P5b channel-flatten dispatcher) doesn't exist yet, so
    /// the only live caller today is `#[cfg(test)]`.
    #[allow(dead_code)]
    fn input_stage(&self) -> Option<&MppStage>;

    /// Return a new instance with `input_stage` stamped to `stage`. The
    /// existing state (wiring, drain handle, etc.) is moved into the new
    /// instance — calling `with_input_stage` a second time on the original
    /// node will fail because those one-shot resources have been consumed.
    fn with_input_stage(&self, stage: MppStage) -> DFResult<Arc<dyn ExecutionPlan>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpp_stage_is_constructible_and_equal() {
        let a = MppStage::new(7, 2, 4);
        let b = MppStage::new(7, 2, 4);
        assert_eq!(a, b);
        assert_eq!(a.query_id, 7);
        assert_eq!(a.stage_id, 2);
        assert_eq!(a.task_count, 4);
    }

    #[test]
    fn mpp_task_key_is_constructible_and_equal() {
        let a = MppTaskKey {
            query_id: 42,
            stage_id: 1,
            task_number: 3,
        };
        let b = MppTaskKey {
            query_id: 42,
            stage_id: 1,
            task_number: 3,
        };
        assert_eq!(a, b);
    }
}
