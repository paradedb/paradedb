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
//! Customscan entry-points for the coordinator/worker MPP architecture.
//!
//! Skeleton module — surface for the eventual aggregatescan / joinscan
//! integration. Today the walker
//! ([`crate::postgres::customscan::mpp::walker::distribute_plan`]) is the only
//! thing that runs end-to-end; the actual hookup into
//! `aggregatescan::exec_datafusion_aggregate` (and the DSM allocation /
//! shm_mq mesh wiring it depends on) is the next milestone in the chain.
//!
//! # Role split
//!
//! Customscan dispatches based on `IsParallelWorker()`:
//!
//! - Worker: `MppRole::Producer`. Runs the [`MppPlanPair::worker_plan`] to
//!   completion, emits zero rows back to PG.
//! - Leader: `MppRole::Consumer`. Spawns a Tokio task that runs the same
//!   `worker_plan` (leader-as-also-worker-0 contribution into the mesh),
//!   then drives the [`MppPlanPair::leader_plan`] on the main thread,
//!   yielding rows back to PG via the existing customscan exec loop.
//!
//! Both roles install the [`MppRpcMesh`] on the session config's extensions
//! before plan execution; producers look up `outbound_senders[partition]`,
//! consumers look up `inbound_drains[(worker, partition)]`.
//!
//! # Lifecycle
//!
//! 1. Postgres calls `estimate_dsm_custom_scan` → we compute the size of the
//!    N×K mesh from `(n_workers, cut_count)` and ask PG for that many bytes
//!    of DSM.
//! 2. Postgres calls `initialize_dsm_custom_scan` (leader) → we
//!    `shm_mq_create()` every queue slot, build the [`MppRpcMesh`] handle,
//!    serialize the worker plan + the [`MppParticipantConfig`] into the DSM
//!    plan-region (via [`super::session::MppPlanBroadcast`]).
//! 3. Postgres calls `initialize_worker_custom_scan` (per worker) →
//!    each worker reads its participant index, attaches as receiver to its
//!    inbound queues, attaches as sender to its outbound queues, and
//!    installs its share of the [`MppRpcMesh`] on the session config.
//! 4. `exec_datafusion_aggregate` dispatches via [`run_for_role`] below.

use crate::postgres::customscan::mpp::shape::MppPlanShape;
use crate::postgres::customscan::mpp::walker::MppPlanPair;

/// This participant's role for the lifetime of one MPP query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MppRole {
    /// Drives the worker subplan and pushes rows into the leader's mesh
    /// queues. Emits zero rows back to PG. Used by parallel-query workers
    /// and by the leader's spawned Tokio task ("leader as worker 0").
    Producer,
    /// Reads from the mesh queues, runs the leader's consumer plan, emits
    /// rows back to PG. Only one consumer per query (the leader).
    Consumer,
}

/// Computed at customscan plan time. Held on the customscan state until
/// `initialize_dsm_custom_scan` runs.
#[derive(Debug)]
pub struct MppDispatchPlan {
    pub shape: MppPlanShape,
    pub n_workers: u32,
    pub plan_pair: MppPlanPair,
}

/// Customscan invocation surface: pick the plan to run for `role` from the
/// pre-computed [`MppDispatchPlan`]. The actual `runtime.block_on` /
/// `tokio::spawn(worker_plan)` glue lives at the call site in
/// `aggregatescan::exec_datafusion_aggregate`.
pub fn plan_for_role(
    dispatch: &MppDispatchPlan,
    role: MppRole,
) -> &std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan> {
    match role {
        MppRole::Producer => &dispatch.plan_pair.worker_plan,
        MppRole::Consumer => &dispatch.plan_pair.leader_plan,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::customscan::mpp::walker::distribute_plan;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::physical_plan::aggregates::{AggregateExec, AggregateMode, PhysicalGroupBy};
    use datafusion::physical_plan::empty::EmptyExec;
    use datafusion::physical_plan::ExecutionPlan;
    use std::sync::Arc;

    fn dispatch_plan_for_test() -> MppDispatchPlan {
        let schema = Arc::new(Schema::new(vec![Field::new("c", DataType::Int64, false)]));
        let empty: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(schema));
        let group_by = PhysicalGroupBy::new_single(vec![]);
        let partial: Arc<dyn ExecutionPlan> = Arc::new(
            AggregateExec::try_new(
                AggregateMode::Partial,
                group_by,
                vec![],
                vec![],
                empty.clone(),
                empty.schema(),
            )
            .unwrap(),
        );
        let plan_pair = distribute_plan(MppPlanShape::ScalarAggOnBinaryJoin, partial, 4).unwrap();
        MppDispatchPlan {
            shape: MppPlanShape::ScalarAggOnBinaryJoin,
            n_workers: 4,
            plan_pair,
        }
    }

    #[test]
    fn plan_for_role_returns_producer_plan_for_worker() {
        let d = dispatch_plan_for_test();
        let plan = plan_for_role(&d, MppRole::Producer);
        assert_eq!(plan.name(), "ShmMqProducerExec");
    }

    #[test]
    fn plan_for_role_returns_consumer_plan_for_leader() {
        let d = dispatch_plan_for_test();
        let plan = plan_for_role(&d, MppRole::Consumer);
        assert_eq!(plan.name(), "AggregateExec");
    }
}
