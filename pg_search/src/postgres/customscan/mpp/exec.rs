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
//! Leader/worker exec helpers for MPP.
//!
//! - [`install_distributed_planner`] — installs the fork's
//!   `with_distributed_planner` + `with_distributed_worker_transport` +
//!   `with_distributed_worker_resolver` on a `SessionStateBuilder` so the
//!   leader's physical plan automatically gets `NetworkShuffleExec`s with
//!   our `ShmMqWorkerTransport` wired in.
//! - [`run_producer_fragment`] — worker (or leader-as-worker-0) push loop.
//!   Runs the `n_partitions` output partitions of `plan` concurrently;
//!   each batch yielded by partition `p` is encoded and pushed through
//!   `outbound_senders[p]`. Returns when every output stream is exhausted.

use std::sync::Arc;

use datafusion::common::{DataFusionError, Result};
use datafusion::execution::{SessionStateBuilder, TaskContext};
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties};
use datafusion_distributed::{DistributedExt, SessionStateBuilderExt};
use futures::stream::StreamExt;

use crate::postgres::customscan::mpp::runtime::{MppMesh, MppWorkerResolver, ShmMqWorkerTransport};
use crate::postgres::customscan::mpp::transport::{MppSender, SendBatchStats};

/// Wire the fork's distributed planner + our `shm_mq` transport + the stub
/// worker resolver onto `builder`. Call this on the leader's
/// [`SessionStateBuilder`] *before* `with_distributed_planner` is meaningful
/// (i.e. before any other query planner registration that should sit
/// underneath it).
pub fn install_distributed_planner(
    builder: SessionStateBuilder,
    mesh: Arc<MppMesh>,
) -> SessionStateBuilder {
    let n_workers = mesh.n_workers as usize;
    builder
        .with_distributed_worker_resolver(MppWorkerResolver::new(n_workers))
        .with_distributed_worker_transport(ShmMqWorkerTransport::new(mesh))
        .with_distributed_planner()
}

/// Run a producer fragment plan to exhaustion and push every output batch
/// through the corresponding `outbound_senders[partition]`.
///
/// Used both on workers (the only thing they do; emit zero rows back to PG)
/// and on the leader as worker 0 (spawned as a Tokio task alongside the
/// leader's consumer plan execution).
///
/// The output partition count of `plan` MUST equal `outbound_senders.len()`;
/// this is checked before the first batch is pulled.
pub async fn run_producer_fragment(
    plan: Arc<dyn ExecutionPlan>,
    outbound_senders: Vec<MppSender>,
    ctx: Arc<TaskContext>,
) -> Result<()> {
    let n_partitions = plan.output_partitioning().partition_count();
    if n_partitions != outbound_senders.len() {
        return Err(DataFusionError::Internal(format!(
            "run_producer_fragment: plan has {} output partitions but {} senders provided",
            n_partitions,
            outbound_senders.len()
        )));
    }

    // Execute every output partition concurrently. Each partition gets its
    // own sender; pushes are independent. `Arc<MppSender>` so each
    // partition's future has its own clone (MppSender is `Sync`).
    let senders: Vec<Arc<MppSender>> = outbound_senders.into_iter().map(Arc::new).collect();
    let mut futures = Vec::with_capacity(n_partitions);
    for (partition, sender) in senders.iter().enumerate() {
        let plan = Arc::clone(&plan);
        let ctx = Arc::clone(&ctx);
        let sender = Arc::clone(sender);
        futures.push(async move {
            let mut stream = plan.execute(partition, ctx)?;
            let mut stats = SendBatchStats::default();
            while let Some(batch) = stream.next().await {
                let batch = batch?;
                if batch.num_rows() == 0 {
                    continue;
                }
                sender
                    .as_ref()
                    .send_batch_traced(&batch, &mut stats)
                    .await?;
            }
            Ok::<(), DataFusionError>(())
        });
    }
    futures::future::try_join_all(futures).await?;
    // Drop senders so peers observe Detached on their next try_recv.
    drop(senders);
    Ok(())
}
