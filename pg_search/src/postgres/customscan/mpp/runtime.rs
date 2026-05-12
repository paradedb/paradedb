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
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::{
    OnMetadataCallback, Stage, WorkerConnection, WorkerPartitionStream, WorkerResolver,
    WorkerTransport,
};
use url::Url;

use crate::postgres::customscan::mpp::assignment::TaskAssignment;
use crate::postgres::customscan::mpp::transport::{DrainHandle, DrainItem};

/// Runtime handle the customscan populates at DSM-init time.
///
/// M1.c restructured this from a `(worker, partition)`-indexed grid to a
/// per-sender-proc map. Each shm_mq queue (one per `(sender_proc, this_proc)`
/// pair in the V2 DSM grid) is multi-channel: frames from any number of
/// `(stage_id, partition)` logical channels can arrive on one queue, tagged
/// by [`MppFrameHeader`]. M2.b added the sub-buffer registry per
/// [`DrainHandle`] so a single drain can fan frames out to multiple
/// consumers keyed on `(stage_id, partition)`.
///
/// M2.c added [`Self::task_assignment`] — the plan-driven
/// `(stage_id, task_idx) → proc_idx` table installed after physical-plan
/// build. [`ShmMqWorkerTransport::open`] consults it to route
/// `(input_stage, target_task)` requests to the right inbound drain.
///
/// [`MppFrameHeader`]: crate::postgres::customscan::mpp::transport::MppFrameHeader
pub struct MppMesh {
    /// This process's `proc_idx` (= 0 for the leader, `ParallelWorkerNumber + 1`
    /// for workers). Frames addressed to this proc arrive on `slot(*, this_proc)`.
    pub this_proc: u32,
    /// Total participant count. Mostly for bounds checking the sender_proc
    /// lookups in [`ShmMqWorkerTransport::open`].
    pub n_procs: u32,
    /// `inbound_drains[sender_proc]` is the cooperative drain that pulls
    /// frames from `slot(sender_proc, this_proc)`. `None` for entries the
    /// process doesn't consume from (self-loop, currently). The natural-shape
    /// gather installs drains for every `sender_proc != this_proc`.
    pub inbound_drains: Vec<Option<Arc<DrainHandle>>>,
    /// Plan-driven `(stage_id, task_idx) → proc_idx` table. Installed by
    /// [`Self::install_assignment`] right after the leader builds the
    /// physical plan; read by [`ShmMqWorkerTransport::open`] at execute
    /// time. `OnceLock` because the table is logically immutable for the
    /// lifetime of the query — install once, read many. If `get()` returns
    /// `None`, the transport falls back to the M1 heuristic to keep the
    /// single-stage gather working during incremental rollout.
    task_assignment: OnceLock<Arc<TaskAssignment>>,
}

impl MppMesh {
    /// Build a fresh mesh with the assignment table unset. Use
    /// [`Self::install_assignment`] after the physical plan is available.
    pub fn new(
        this_proc: u32,
        n_procs: u32,
        inbound_drains: Vec<Option<Arc<DrainHandle>>>,
    ) -> Self {
        Self {
            this_proc,
            n_procs,
            inbound_drains,
            task_assignment: OnceLock::new(),
        }
    }

    /// Look up the drain that owns frames coming from `sender_proc`. Returns
    /// `None` if no drain is installed (out-of-range or the self-loop slot,
    /// which the single-stage gather skips).
    pub fn inbound_drain(&self, sender_proc: u32) -> Option<&Arc<DrainHandle>> {
        let idx = sender_proc as usize;
        self.inbound_drains.get(idx).and_then(|slot| slot.as_ref())
    }

    /// Install the plan-derived task assignment table. Idempotent in spirit
    /// — a second call after the first wins is a logic bug, but
    /// `OnceLock::set` silently no-ops in that case so callers don't have
    /// to special-case it.
    pub fn install_assignment(&self, assignment: Arc<TaskAssignment>) {
        let _ = self.task_assignment.set(assignment);
    }

    /// Read the installed assignment table, or `None` if it hasn't been
    /// installed yet.
    pub fn task_assignment(&self) -> Option<&Arc<TaskAssignment>> {
        self.task_assignment.get()
    }
}

/// Implements the DF-D fork's [`WorkerTransport`] over the leader's [`MppMesh`].
///
/// `open(input_stage, target_task)` translates the DF-D `(stage, task)`
/// addressing into the proc-pair grid: `target_task` selects which `sender_proc`
/// hosts the producer-side task, and the returned [`WorkerConnection`] pulls
/// from that proc's inbound drain.
///
/// M2.c routes via the mesh's plan-driven [`TaskAssignment`] table. If the
/// table is uninstalled (e.g. during incremental rollout) the transport
/// falls back to the legacy single-stage heuristic
/// `sender_proc = target_task + 1` so the natural-shape gather keeps
/// working.
pub struct ShmMqWorkerTransport {
    mesh: Arc<MppMesh>,
}

impl ShmMqWorkerTransport {
    pub fn new(mesh: Arc<MppMesh>) -> Self {
        Self { mesh }
    }

    /// `(stage_id, task)` → `sender_proc`. Consults the installed
    /// [`TaskAssignment`]; falls back to `task + 1` if the table isn't
    /// populated or doesn't carry an entry for the requested key. The
    /// fallback matches M1's single-stage heuristic and keeps the
    /// `NetworkCoalesceExec(consumer_tc=1, input_tc=N)` gather working
    /// while M2.d wires up per-worker meshes that install the assignment
    /// from the worker side too.
    fn sender_proc_for_task(&self, stage_id: u32, target_task: u32) -> u32 {
        if let Some(a) = self.mesh.task_assignment() {
            if let Some(p) = a.proc_for(stage_id, target_task) {
                return p;
            }
        }
        target_task + 1
    }
}

impl WorkerTransport for ShmMqWorkerTransport {
    fn open(
        &self,
        input_stage: &Stage,
        _target_partitions: Range<usize>,
        target_task: usize,
        _ctx: &Arc<TaskContext>,
    ) -> Result<Box<dyn WorkerConnection>> {
        let target_task_u32 = u32::try_from(target_task).map_err(|_| {
            DataFusionError::Internal(format!(
                "ShmMqWorkerTransport: target_task={target_task} > u32::MAX"
            ))
        })?;
        let stage_id = u32::try_from(input_stage.num()).map_err(|_| {
            DataFusionError::Internal(format!(
                "ShmMqWorkerTransport: input_stage.num()={} > u32::MAX",
                input_stage.num()
            ))
        })?;
        let sender_proc = self.sender_proc_for_task(stage_id, target_task_u32);
        if sender_proc >= self.mesh.n_procs {
            return Err(DataFusionError::Internal(format!(
                "ShmMqWorkerTransport: sender_proc={sender_proc} >= n_procs={} \
                 (stage_id={stage_id}, target_task={target_task})",
                self.mesh.n_procs
            )));
        }
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
    /// `stage_id` of the boundary's `input_stage`. Passed to
    /// `DrainHandle::register_channel` so the sub-buffer this connection
    /// streams from sees only frames tagged with the same `(stage_id, p)`.
    stage_id: u32,
}

impl WorkerConnection for ShmMqWorkerConnection {
    fn stream_partition(
        &self,
        partition: usize,
        _on_metadata: OnMetadataCallback,
    ) -> Result<WorkerPartitionStream> {
        let partition_u32 = u32::try_from(partition).map_err(|_| {
            DataFusionError::Internal(format!(
                "ShmMqWorkerConnection: partition={partition} > u32::MAX"
            ))
        })?;
        let drain = self.mesh.inbound_drain(self.sender_proc).ok_or_else(|| {
            DataFusionError::Internal(format!(
                "ShmMqWorkerConnection: no inbound drain for sender_proc={} \
                 (stage_id={}, this_proc={})",
                self.sender_proc, self.stage_id, self.mesh.this_proc
            ))
        })?;
        let drain = Arc::clone(drain);
        // M2.b: ask the drain for the sub-buffer dedicated to this
        // `(stage_id, partition)` channel. Frames the worker emits with a
        // matching header land here; frames tagged with other partitions go
        // to their own sub-buffers, so this consumer only sees its slice.
        let buffer = drain.register_channel(self.stage_id, partition_u32);
        // Cooperative pull loop: pgrx requires shm_mq FFI on the backend
        // thread, so the drain pass runs inline here. Each iteration drains
        // the receiver into the registry (max 256 batches), then pops one
        // batch out to yield, then yields back to Tokio so sibling tasks
        // (e.g. the leader's own producer subplan) can advance.
        //
        // `pgrx::check_for_interrupts!()` at the top of every iteration so
        // a user CANCEL or query timeout `longjmp`s out before the next
        // drain pass; without it the cooperative spin would keep pumping
        // batches even after the backend should have torn down. The send
        // side has the same check inside `MppSender::send_batch_traced`'s
        // retry loop in `transport.rs`.
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

/// Worker-side [`WorkerTransport`] that resolves nested network boundaries
/// (such as a `NetworkBroadcastExec` on the build side of a `HashJoin`) by
/// executing the boundary's `input_stage.plan` locally on the worker.
///
/// In the DF-D fork's gRPC distributed model, every worker would pull the
/// broadcast data from a designated source via a Flight stream. In our
/// in-process model the producer fragment that workers run already contains
/// these network boundary nodes (the worker re-plans from the same logical
/// plan as the leader), and rather than fan a broadcast across an in-process
/// mesh we just have each worker re-execute the build-side plan locally —
/// it's a small index scan, cheap enough that duplicating the work across
/// `n_workers` producers is preferable to wiring a second mesh.
///
/// The worker's session must keep `input_stage.plan = Some(...)` (i.e. the
/// DF-D fork's `prepare_plan` is *not* invoked on the worker side because we
/// drop into `find_worker_fragment` below the leader's `DistributedExec`).
pub struct LocalExecWorkerTransport;

impl WorkerTransport for LocalExecWorkerTransport {
    fn open(
        &self,
        input_stage: &Stage,
        _target_partitions: Range<usize>,
        _target_task: usize,
        ctx: &Arc<TaskContext>,
    ) -> Result<Box<dyn WorkerConnection>> {
        let plan = input_stage.plan().cloned().ok_or_else(|| {
            DataFusionError::Internal(
                "LocalExecWorkerTransport: input_stage.plan is None — \
                 worker re-plan must keep the boundary's input subtree intact"
                    .into(),
            )
        })?;
        Ok(Box::new(LocalExecWorkerConnection {
            plan,
            ctx: Arc::clone(ctx),
        }))
    }
}

struct LocalExecWorkerConnection {
    plan: Arc<dyn ExecutionPlan>,
    ctx: Arc<TaskContext>,
}

impl WorkerConnection for LocalExecWorkerConnection {
    fn stream_partition(
        &self,
        partition: usize,
        _on_metadata: OnMetadataCallback,
    ) -> Result<WorkerPartitionStream> {
        let stream = self.plan.execute(partition, Arc::clone(&self.ctx))?;
        // SendableRecordBatchStream is already `Pin<Box<dyn Stream<Item = Result<RecordBatch>> + Send>>`;
        // the WorkerPartitionStream alias drops the schema bound, so the
        // existing pinned box satisfies it directly.
        Ok(stream)
    }
}
