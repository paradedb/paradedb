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

//! Runtime glue between the leader's DataFusion execution and the DSM MPSC mesh.
//!
//! [`MppMesh`] is the runtime handle the leader builds at DSM-init time. It holds the
//! single [`crate::postgres::customscan::mpp::transport::DrainHandle`] (the
//! `inbound_receiver`) that consolidates this proc's DSM inbox and self-loop, and gets
//! installed on the leader's `SessionConfig` extensions before plan execution.
//!
//! [`ShmMqWorkerTransport`] implements the fork's [`WorkerTransport`] trait, consulted
//! by `NetworkShuffleExec` / `NetworkCoalesceExec` / `NetworkBroadcastExec` at execute
//! time. `open(target_task=worker)` returns a [`ShmMqWorkerConnection`] that yields one
//! stream per consumer partition from the shared `inbound_receiver`.
//!
//! No `WorkerResolver` impl: under `in_process_mode = true` the fork makes the resolver
//! optional and substitutes a placeholder URL; nothing here resolves anything by URL.

use std::ops::Range;
use std::sync::Arc;

use datafusion::arrow::array::RecordBatch;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::physical_expr_common::metrics::ExecutionPlanMetricsSet;
use datafusion_distributed::{RemoteStage, WorkerConnection, WorkerTransport};
use futures::stream::BoxStream;

use crate::postgres::customscan::mpp::transport::{CooperativeDrainSet, DrainHandle, DrainItem};

/// `task_idx → proc_idx` round-robin over the worker procs. The leader is `proc_idx = 0`
/// (consumer-only), workers are `1..n_procs` (each hosts producer fragments).
///
/// A stage's task count is set by the DF-D task estimator chain, not by the worker proc count.
/// With the natural-shape plan's `distributed_task_estimator(n_workers)`, `task_count` equals
/// `n_workers` in every stage we emit today, so the modulo is a no-op. The wrap is defensive for
/// future shapes where an estimator could return more tasks than producer procs.
#[inline]
pub fn proc_for_task(n_workers: u32, task_idx: u32) -> u32 {
    1 + (task_idx % n_workers.max(1))
}

/// Runtime handle the customscan populates at DSM-init time.
///
/// Each process owns one MPSC inbox in DSM that receives frames from every peer.
/// `inbound_receiver` consolidates that inbox plus the in-proc self-loop channel (for
/// producer-and-consumer-on-same-worker fragments) into a single [`DrainHandle`]. Frames
/// carry `(sender_proc, stage_id, partition)` in their header so the routing registry
/// inside the handle delivers each frame to the matching consumer.
///
/// [`MppFrameHeader`]: crate::postgres::customscan::mpp::transport::MppFrameHeader
pub struct MppMesh {
    /// This process's `proc_idx` (= 0 for the leader, `ParallelWorkerNumber + 1` for workers).
    /// Frames addressed to this proc arrive on this proc's own inbox.
    pub this_proc: u32,
    /// Total proc count. Bounds the producer/consumer proc lookups in
    /// [`ShmMqWorkerTransport::open`].
    pub n_procs: u32,
    /// Single cooperative inbound handle pulling every frame addressed to this proc. The
    /// DSM MPSC inbox and an in-proc self-loop receiver both feed into this handle. Demux
    /// to per-`(sender_proc, stage_id, partition)` channel buffers happens inside via
    /// `DrainHandle::register_channel`.
    pub(super) inbound_receiver: Arc<DrainHandle>,
}

impl MppMesh {
    /// Build a fresh mesh.
    pub fn new(this_proc: u32, n_procs: u32, inbound_receiver: Arc<DrainHandle>) -> Self {
        Self {
            this_proc,
            n_procs,
            inbound_receiver,
        }
    }

    /// The single cooperative inbound handle that pulls frames from every peer (and the
    /// self-loop) into per-`(sender_proc, stage_id, partition)` channel buffers.
    pub(super) fn inbound_receiver(&self) -> &Arc<DrainHandle> {
        &self.inbound_receiver
    }

    /// Number of worker procs (= `n_procs - 1`, since the leader is proc 0). Used as the
    /// modulus in [`proc_for_task`].
    ///
    /// The `n_procs >= 3` invariant is enforced by
    /// [`crate::postgres::customscan::mpp::glue::mpp_is_active`] via
    /// `MIN_TOTAL_WORKER_COUNT`, so the subtraction is safe without a `saturating_sub` /
    /// `max(1)` belt-and-braces: every code path that constructs an `MppMesh` (or reaches
    /// this method) is gated on `mpp_is_active()` first. Asserted in debug builds so a
    /// future misuse fails loudly.
    pub(super) fn n_workers(&self) -> u32 {
        debug_assert!(
            self.n_procs >= 3,
            "MppMesh::n_workers() called with n_procs={} (< 3); callers must gate on \
             mpp_is_active()",
            self.n_procs
        );
        self.n_procs - 1
    }

    /// Pull from the single inbound handle. Called from
    /// [`crate::postgres::customscan::mpp::transport::MppSender`]'s cooperative-send spin so a
    /// producer stalled on a full outbound can pull inbound peer data inline. That's what
    /// prevents the symmetric-send deadlock when every peer is simultaneously stalled waiting
    /// for space.
    pub(super) fn drain_all_inbound(&self) -> Result<(), DataFusionError> {
        self.inbound_receiver.try_drain_pass()
    }
}

impl CooperativeDrainSet for MppMesh {
    fn try_drain_pass(&self) -> Result<(), DataFusionError> {
        self.drain_all_inbound()
    }
}

/// Implements the DF-D fork's [`WorkerTransport`] over the leader's [`MppMesh`].
///
/// `open(input_stage, target_task)` translates the DF-D `(stage, task)`
/// addressing into the proc-pair grid: `proc_for_task(n_workers, target_task)`
/// selects which `sender_proc` hosts the producer-side task, and the returned
/// [`WorkerConnection`] pulls from that proc's inbound drain.
pub struct ShmMqWorkerTransport {
    mesh: Arc<MppMesh>,
}

impl ShmMqWorkerTransport {
    pub fn new(mesh: Arc<MppMesh>) -> Self {
        Self { mesh }
    }
}

impl WorkerTransport for ShmMqWorkerTransport {
    fn open(
        &self,
        input_stage: &RemoteStage,
        _target_partitions: Range<usize>,
        target_task: usize,
        _ctx: &Arc<TaskContext>,
        _metrics: &ExecutionPlanMetricsSet,
    ) -> Result<Box<dyn WorkerConnection + Send + Sync>> {
        let target_task_u32 = u32::try_from(target_task).map_err(|_| {
            DataFusionError::Internal(format!(
                "ShmMqWorkerTransport: target_task={target_task} > u32::MAX"
            ))
        })?;
        let stage_id = u32::try_from(input_stage.num).map_err(|_| {
            DataFusionError::Internal(format!(
                "ShmMqWorkerTransport: input_stage.num={} > u32::MAX",
                input_stage.num
            ))
        })?;
        let sender_proc = proc_for_task(self.mesh.n_workers(), target_task_u32);
        if sender_proc >= self.mesh.n_procs {
            return Err(DataFusionError::Internal(format!(
                "ShmMqWorkerTransport: sender_proc={sender_proc} >= n_procs={} \
                 (stage_id={stage_id}, target_task={target_task})",
                self.mesh.n_procs
            )));
        }
        crate::mpp_log!(
            "mpp transport::open this_proc={} stage_id={stage_id} target_task={target_task} \
             → sender_proc={sender_proc}",
            self.mesh.this_proc
        );
        Ok(Box::new(ShmMqWorkerConnection {
            mesh: Arc::clone(&self.mesh),
            sender_proc,
            stage_id,
        }))
    }
}

struct ShmMqWorkerConnection {
    mesh: Arc<MppMesh>,
    sender_proc: u32,
    /// `stage_id` of the boundary's `input_stage`. Passed to `DrainHandle::register_channel` so
    /// the channel buffer this connection streams from sees only frames tagged with the same
    /// `(stage_id, p)`.
    stage_id: u32,
}

impl WorkerConnection for ShmMqWorkerConnection {
    fn execute(&self, partition: usize) -> Result<BoxStream<'static, Result<RecordBatch>>> {
        let partition_u32 = u32::try_from(partition).map_err(|_| {
            DataFusionError::Internal(format!(
                "ShmMqWorkerConnection: partition={partition} > u32::MAX"
            ))
        })?;
        // One drain per process, shared across all sender_procs. The channel-buffer
        // registry keys by (sender_proc, stage_id, partition) so this consumer still
        // sees only its named sender's slice even though the underlying inbox is
        // shared with all peers.
        let drain = Arc::clone(self.mesh.inbound_receiver());
        let buffer = drain.register_channel(self.sender_proc, self.stage_id, partition_u32);
        crate::mpp_log!(
            "mpp transport::execute this_proc={} sender_proc={} stage_id={} \
             partition={partition_u32} (register_channel)",
            self.mesh.this_proc,
            self.sender_proc,
            self.stage_id,
        );
        // Cooperative pull loop: pgrx requires shm_mq FFI on the backend thread, so the drain
        // pass runs inline here. Each iteration drains the receiver into the registry (max 256
        // batches), then pops one batch out to yield, then yields back to Tokio so sibling tasks
        // (e.g. the leader's own producer subplan) can advance.
        //
        // `pgrx::check_for_interrupts!()` at the top of every iteration so a user CANCEL or query
        // timeout `longjmp`s out before the next drain pass; without it the cooperative spin
        // would keep pumping batches even after the backend should have torn down. The send side
        // has the same check inside `MppSender::send_batch_traced`'s retry loop in `transport.rs`.
        let stream = async_stream::stream! {
            loop {
                pgrx::check_for_interrupts!();
                if let Err(e) = drain.try_drain_pass() {
                    yield Err(e);
                    return;
                }
                match buffer.try_pop() {
                    Some(DrainItem::Batch(batch)) => yield Ok(batch),
                    Some(DrainItem::Eof) => break,
                    None => tokio::task::yield_now().await,
                }
            }
        };
        Ok(Box::pin(stream))
    }
}
