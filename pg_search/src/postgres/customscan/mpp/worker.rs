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

//! Per-partition driver for the pull-shape protocol.
//!
//! [`run_partition_driver`] is the producer-side body that runs `plan.execute(partition, ctx)`
//! and forwards every batch through a pre-built `MppSender`. It always tries to emit a
//! per-channel `Eof` frame at the end (success OR clean exit) so the consumer's drain registry
//! transitions cleanly. The shm_mq queue is multiplexed across `(stage_id, partition)` channels,
//! so dropping the sender is NOT enough — only the explicit Eof frame closes a single logical
//! channel without taking down the queue.
//!
//! Consumer-side teardown (e.g. `SortPreservingMergeExec(fetch=N)` finishing early under a
//! `LIMIT`) surfaces here as a "sender detached" send error. We treat it as a clean signal: the
//! driver returns `Ok(())` so it doesn't poison the registry's first-error slot, and the
//! service loop's `active_drivers` counter decrements to 0 normally.

use std::sync::Arc;

use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::physical_plan::ExecutionPlan;
use futures::stream::StreamExt;

use crate::postgres::customscan::mpp::transport::{
    MppSender, SendBatchStats, CONSUMER_DETACHED_SENTINEL,
};

/// True if `err` is the "consumer torn down mid-stream" signal that
/// [`MppSender::send_batch_traced`] / [`MppSender::send_eof_traced`] produce when the underlying
/// channel detaches. Matches on the [`CONSUMER_DETACHED_SENTINEL`] tag (shared between shm_mq
/// and in-proc senders), not on backend-specific message text, so adding a new transport doesn't
/// silently bypass the predicate.
fn is_consumer_detached(err: &DataFusionError) -> bool {
    err.to_string().contains(CONSUMER_DETACHED_SENTINEL)
}

/// Execute partition `partition` of `plan` and push every yielded batch through `sender`,
/// followed by an `Eof` frame. Sends the Eof regardless of whether the stream succeeded — the
/// consumer needs the EOF to unwind even on error, and the shm_mq queue is multiplexed so a
/// dropped sender alone doesn't close the logical channel.
///
/// Consumer-detach during a send returns `Ok(())` (not an error) so the registry's
/// first-error slot stays clean. Anything else that errors mid-stream still propagates.
pub async fn run_partition_driver(
    plan: Arc<dyn ExecutionPlan>,
    partition: usize,
    sender: MppSender,
    ctx: Arc<TaskContext>,
) -> Result<()> {
    let mut stats = SendBatchStats::default();
    let stream_result: Result<bool, DataFusionError> = async {
        let mut stream = plan.execute(partition, ctx)?;
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            if batch.num_rows() == 0 {
                continue;
            }
            match sender.send_batch_traced(&batch, &mut stats).await {
                Ok(()) => {}
                Err(e) if is_consumer_detached(&e) => return Ok(false),
                Err(e) => return Err(e),
            }
        }
        Ok(true)
    }
    .await;

    match stream_result {
        Ok(true) => match sender.send_eof_traced(&mut stats).await {
            Ok(()) => Ok(()),
            Err(e) if is_consumer_detached(&e) => Ok(()),
            Err(e) => Err(e),
        },
        Ok(false) => Ok(()),
        Err(e) => Err(e),
    }
}
