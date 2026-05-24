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

//! Runtime glue between the leader's DataFusion execution and the
//! shm_mq mesh.
//!
//! - [`MppMesh`] — runtime handle the leader builds at DSM-init time,
//!   carrying one [`crate::postgres::customscan::mpp::transport::DrainHandle`]
//!   per producer worker for each consumer partition. Installed on the
//!   leader's `SessionConfig` extensions before plan execution.
//! - [`ShmMqWorkerTransport`] — implements the DF-D fork's [`WorkerTransport`]
//!   trait, consulted by `NetworkShuffleExec`/`NetworkCoalesceExec`/
//!   `NetworkBroadcastExec` at execute time. `open(target_task=worker)`
//!   returns a [`ShmMqWorkerConnection`] that yields one stream per
//!   consumer partition from the corresponding [`DrainHandle`].
//! - [`MppWorkerResolver`] — stub [`WorkerResolver`] returning N dummy
//!   URLs. The DF-D fork's planner reads `get_urls().len()` to decide cluster
//!   capacity; we don't have URLs (everything is in-process), so any
//!   address satisfies the API.

use std::ops::Range;
use std::sync::Arc;

use async_trait::async_trait;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::physical_expr_common::metrics::ExecutionPlanMetricsSet;
use datafusion_distributed::{
    RemoteStage, WorkerConnection, WorkerPartitionStream, WorkerResolver, WorkerTransport,
};
use url::Url;

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
/// Each shm_mq queue (one per `(sender_proc, this_proc)` pair in the V2 DSM grid) is
/// multi-channel. Frames from any number of `(stage_id, partition)` logical channels can arrive
/// on one queue, tagged by [`MppFrameHeader`]. The channel buffer registry on each [`DrainHandle`]
/// fans them out to the matching consumers keyed on `(stage_id, partition)`.
///
/// [`MppFrameHeader`]: crate::postgres::customscan::mpp::transport::MppFrameHeader
pub struct MppMesh {
    /// This process's `proc_idx` (= 0 for the leader, `ParallelWorkerNumber + 1` for workers).
    /// Frames addressed to this proc arrive on `slot(*, this_proc)`.
    pub this_proc: u32,
    /// Total proc count. Bounds the producer/consumer proc lookups in
    /// [`ShmMqWorkerTransport::open`].
    pub n_procs: u32,
    /// `inbound_receivers[sender_proc]` is the cooperative drain that pulls frames from
    /// `slot(sender_proc, this_proc)`. `None` at the self-loop entry
    /// (`sender_proc == this_proc`); workers route those frames through an in-proc channel via
    /// `outbound_senders[this_proc]` instead. The natural-shape gather installs drains for every
    /// `sender_proc != this_proc`.
    pub(super) inbound_receivers: Vec<Option<Arc<DrainHandle>>>,
}

impl MppMesh {
    /// Build a fresh mesh.
    pub fn new(
        this_proc: u32,
        n_procs: u32,
        inbound_receivers: Vec<Option<Arc<DrainHandle>>>,
    ) -> Self {
        Self {
            this_proc,
            n_procs,
            inbound_receivers,
        }
    }

    /// Look up the drain that owns frames coming from `sender_proc`. Returns `None` if no drain
    /// is installed (out-of-range or the self-loop slot, which the single-stage gather skips).
    pub(super) fn inbound_receiver(&self, sender_proc: u32) -> Option<&Arc<DrainHandle>> {
        let idx = sender_proc as usize;
        self.inbound_receivers
            .get(idx)
            .and_then(|slot| slot.as_ref())
    }

    /// Number of worker procs (= `n_procs - 1`, since the leader is proc 0).
    /// Used as the modulus in [`proc_for_task`].
    pub(super) fn n_workers(&self) -> u32 {
        self.n_procs.saturating_sub(1).max(1)
    }

    /// Pull from every installed inbound drain. Called from
    /// [`crate::postgres::customscan::mpp::transport::MppSender`]'s cooperative-send spin so a
    /// producer stalled on a full outbound queue can drain inbound peer data inline. That's what
    /// prevents the N×N symmetric-send deadlock when every peer is simultaneously stalled waiting
    /// for space.
    ///
    /// Returns the first error if any drain's `try_drain_pass` errors; otherwise `Ok(())` after
    /// all drains have been polled. Drains that have already detached are skipped silently (their
    /// slot is still `Some`, but their inner `coop_receivers` Vec entry is `None`).
    pub(super) fn drain_all_inbound(&self) -> Result<(), DataFusionError> {
        for drain in self.inbound_receivers.iter().flatten() {
            drain.try_drain_pass()?;
        }
        Ok(())
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
    fn stream_partition(&self, partition: usize) -> Result<WorkerPartitionStream> {
        let partition_u32 = u32::try_from(partition).map_err(|_| {
            DataFusionError::Internal(format!(
                "ShmMqWorkerConnection: partition={partition} > u32::MAX"
            ))
        })?;
        let drain = self
            .mesh
            .inbound_receiver(self.sender_proc)
            .ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "ShmMqWorkerConnection: no inbound drain for sender_proc={} \
                 (stage_id={}, this_proc={})",
                    self.sender_proc, self.stage_id, self.mesh.this_proc
                ))
            })?;
        let drain = Arc::clone(drain);
        // Ask the drain for the channel buffer dedicated to this `(stage_id, partition)` channel.
        // Frames with a matching header land here; frames tagged with other partitions go to
        // their own channel buffers, so this consumer only sees its slice.
        let buffer = drain.register_channel(self.stage_id, partition_u32);
        crate::mpp_log!(
            "mpp transport::stream_partition this_proc={} sender_proc={} stage_id={} \
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

/// Stub [`WorkerResolver`] for the DF-D fork's distributed planner. Workers in
/// our embedded model are PG parallel workers in the same backend tree, not
/// URL-addressed nodes; the planner only consults `get_urls().len()` for
/// task-count sizing, so any URL satisfies it.
#[derive(Clone)]
pub struct MppWorkerResolver {
    n_workers: usize,
}

impl MppWorkerResolver {
    pub fn new(n_workers: usize) -> Self {
        Self { n_workers }
    }
}

#[async_trait]
impl WorkerResolver for MppWorkerResolver {
    fn get_urls(&self) -> Result<Vec<Url>, DataFusionError> {
        let url = Url::parse("http://mpp.local/").expect("static URL parses");
        Ok(vec![url; self.n_workers])
    }
}
