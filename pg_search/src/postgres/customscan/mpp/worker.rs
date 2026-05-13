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

//! MPP worker fragment runner.
//!
//! "Worker" matches the DF-D fork's terminology — every distributed task is
//! a `WorkerConnection` on the receive side, and the fragment runner is
//! that worker's push side.
//!
//! - [`run_worker_fragment`] — PG-parallel-worker push loop. Runs the
//!   `n_partitions` output partitions of `plan` concurrently; each batch
//!   yielded by partition `p` is encoded and pushed through
//!   `outbound_senders[p]`. Returns when every output stream is exhausted.
//!   The leader is consumer-only in this iteration (see
//!   [`crate::postgres::customscan::mpp::glue::producer_worker_count`]);
//!   leader-as-worker-0 is a deferred follow-up.

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
pub async fn run_worker_fragment(
    plan: Arc<dyn ExecutionPlan>,
    outbound_senders: Vec<MppSender>,
    ctx: Arc<TaskContext>,
) -> Result<()> {
    let n_partitions = plan.output_partitioning().partition_count();
    if n_partitions != outbound_senders.len() {
        return Err(DataFusionError::Internal(format!(
            "run_worker_fragment: plan has {} output partitions but {} senders provided",
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
            let mut stats = SendBatchStats::default();
            // Run the partition stream to exhaustion; capture either the
            // first error or Ok(()) so we can still emit the per-channel
            // EOF below regardless of how the stream ended.
            let stream_result: Result<(), DataFusionError> = async {
                let mut stream = plan.execute(partition, ctx)?;
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
                Ok(())
            }
            .await;
            // Signal channel EOF so the consumer's per-(stage_id, partition)
            // sub-buffer transitions to Eof. The shared shm_mq queue can't
            // be relied on to detach — multiple fragments multiplex over
            // the same queue, so dropping this partition's sender doesn't
            // close the queue.
            //
            // CORRECTNESS: send EOF even when the stream errored. Without
            // this, a producer-side error (e.g. mid-scan I/O failure) leaves
            // the consumer's M2.b sub-buffer stuck at `sources_done == 0`
            // and the leader's `select_all` blocks forever. The deadlock
            // detector only fires under `paradedb.mpp_debug = on`, so the
            // production hang would be silent.
            let eof_result = sender.as_ref().send_eof_traced(&mut stats).await;
            // Propagate the stream's error first; otherwise surface any EOF
            // send error so failure modes don't silently disappear.
            stream_result.and(eof_result)
        });
    }
    // Drive every partition future to completion via `join_all` rather than
    // `try_join_all`. `try_join_all` is fail-fast: when one partition
    // returns Err it drops the still-pending sibling futures mid-`await`,
    // so siblings that haven't yet reached their `send_eof_traced` line
    // never emit EOF. The consumer's sub-buffer for those channels stays
    // stuck at `sources_done == 0`, and unless the backend tears down (and
    // the shm_mq queue detaches) the leader's `select_all` blocks forever.
    // `join_all` waits for every partition to run its EOF send before we
    // propagate any error.
    let results = futures::future::join_all(futures).await;
    // Drop senders so peers observe Detached on their next try_recv even if
    // a partition's EOF send itself errored.
    drop(senders);
    for r in results {
        r?;
    }
    Ok(())
}
