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

//! Leader/worker exec helpers for MPP.
//!
//! - [`run_producer_fragment`] — PG-parallel-worker push loop. Runs the
//!   `n_partitions` output partitions of `plan` concurrently; each batch
//!   yielded by partition `p` is encoded and pushed through
//!   `outbound_senders[p]`. Returns when every output stream is exhausted.
//!   The leader is consumer-only in this iteration (see
//!   [`crate::postgres::customscan::mpp::glue::producer_worker_count`]);
//!   leader-as-worker-0 is a deferred follow-up.
//! - [`run_inner_producer_fragment`] — peer-mesh push loop for the post-
//!   aggregate two-boundary plan (Track A/B). Runs every output partition
//!   of `plan` concurrently and routes each batch with a partition-tagged
//!   frame to the right peer-mesh outbound sender. Used in tandem with
//!   `run_producer_fragment` running the OUTER (worker→leader) fragment;
//!   both share the worker's current_thread Tokio runtime and interleave
//!   on the cooperative drain.

use std::sync::Arc;

use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties};
use futures::stream::StreamExt;

use crate::postgres::customscan::mpp::transport::{MppSender, SendBatchStats};

/// Run a producer fragment plan to exhaustion and push every output batch
/// through the corresponding `outbound_senders[partition]`.
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

/// Run the inner producer fragment for the peer-mesh post-aggregate shuffle.
///
/// The plan's `output_partitioning` is the *scaled* hash count (`n_consumers²`
/// after the DF-D fork's `RepartitionExec` rewrite). Each output partition `j` is
/// destined for consumer task `j / partitions_per_consumer`, which equals
/// `j / n_consumers` (since the DF-D fork scales by `consumer_tc = n_consumers`).
/// The batch is sent through `peer_outbound[consumer]` with the framed tag
/// `j` so the consumer side's `DemuxDrainHandle` routes it to the right
/// partition's sub-buffer.
///
/// `peer_outbound.len()` must equal `n_consumers`. The plan's output
/// partition count must be `n_consumers² / 1` (= scaled), which we verify
/// at entry.
pub async fn run_inner_producer_fragment(
    plan: Arc<dyn ExecutionPlan>,
    peer_outbound: Vec<MppSender>,
    n_consumers: usize,
    ctx: Arc<TaskContext>,
) -> Result<()> {
    let n_partitions = plan.output_partitioning().partition_count();
    if peer_outbound.len() != n_consumers {
        return Err(DataFusionError::Internal(format!(
            "run_inner_producer_fragment: expected {n_consumers} peer-outbound senders, got {}",
            peer_outbound.len()
        )));
    }
    if n_consumers == 0 || !n_partitions.is_multiple_of(n_consumers) {
        return Err(DataFusionError::Internal(format!(
            "run_inner_producer_fragment: plan has {n_partitions} output partitions, \
             not divisible by {n_consumers} consumers"
        )));
    }
    let partitions_per_consumer = n_partitions / n_consumers;

    // Each consumer column of the peer mesh is one MppSender; multiple
    // futures push through it concurrently, demuxing on the consumer side
    // by the per-batch partition tag. Senders are `Arc<MppSender>` so each
    // future holds a shared reference; concurrent calls to
    // `send_batch_traced_framed` are scratch-buffer-safe (the inner
    // `RefCell::replace` rotates buffers atomically) on a single-threaded
    // Tokio runtime.
    let senders: Vec<Arc<MppSender>> = peer_outbound.into_iter().map(Arc::new).collect();

    let mut futures = Vec::with_capacity(n_partitions);
    for partition in 0..n_partitions {
        let consumer = partition / partitions_per_consumer;
        let sender = Arc::clone(&senders[consumer]);
        let plan = Arc::clone(&plan);
        let ctx = Arc::clone(&ctx);
        let tag = partition as u32;
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
                    .send_batch_traced_framed(tag, &batch, &mut stats)
                    .await?;
            }
            Ok::<(), DataFusionError>(())
        });
    }
    futures::future::try_join_all(futures).await?;
    drop(senders);
    Ok(())
}
