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
//! - [`ShmMqWorkerTransport`] — implements the fork's [`WorkerTransport`]
//!   trait, consulted by `NetworkShuffleExec`/`NetworkCoalesceExec`/
//!   `NetworkBroadcastExec` at execute time. `open(target_task=worker)`
//!   returns a [`ShmMqWorkerConnection`] that yields one stream per
//!   consumer partition from the corresponding [`DrainHandle`].
//! - [`MppWorkerResolver`] — stub [`WorkerResolver`] returning N dummy
//!   URLs. The fork's planner reads `get_urls().len()` to decide cluster
//!   capacity; we don't have URLs (everything is in-process), so any
//!   address satisfies the API.

use std::ops::Range;
use std::sync::Arc;

use async_trait::async_trait;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::physical_expr_common::metrics::ExecutionPlanMetricsSet;
use datafusion_distributed::{
    Stage, WorkerConnection, WorkerPartitionStream, WorkerResolver, WorkerTransport,
};
use url::Url;

use crate::postgres::customscan::mpp::transport::{DrainHandle, DrainItem};

/// Runtime handle the customscan populates at DSM-init time.
pub struct MppMesh {
    /// Number of producer workers (including leader-as-worker-0).
    pub n_workers: u32,
    /// Number of consumer-side partitions on the leader.
    pub n_partitions: u32,
    /// One [`DrainHandle`] per `(worker, partition)` pair on the leader,
    /// indexed `worker * n_partitions + partition`. Workers receive their
    /// own slice of senders separately (see
    /// [`crate::postgres::customscan::mpp::dsm::WorkerAttach`]).
    pub drains: Vec<Arc<DrainHandle>>,
}

impl MppMesh {
    pub fn drain(&self, worker: u32, partition: u32) -> Option<&Arc<DrainHandle>> {
        if worker >= self.n_workers || partition >= self.n_partitions {
            return None;
        }
        self.drains
            .get((worker as usize) * (self.n_partitions as usize) + (partition as usize))
    }
}

/// Implements the fork's [`WorkerTransport`] over the leader's [`MppMesh`].
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
        _input_stage: &Stage,
        _target_partitions: Range<usize>,
        target_task: usize,
        _ctx: &Arc<TaskContext>,
        _metrics: &ExecutionPlanMetricsSet,
    ) -> Result<Box<dyn WorkerConnection>> {
        let worker = u32::try_from(target_task).map_err(|_| {
            DataFusionError::Internal(format!(
                "ShmMqWorkerTransport: target_task={target_task} > u32::MAX"
            ))
        })?;
        if worker >= self.mesh.n_workers {
            return Err(DataFusionError::Internal(format!(
                "ShmMqWorkerTransport: target_task={worker} >= n_workers={}",
                self.mesh.n_workers
            )));
        }
        Ok(Box::new(ShmMqWorkerConnection {
            mesh: Arc::clone(&self.mesh),
            worker,
        }))
    }
}

struct ShmMqWorkerConnection {
    mesh: Arc<MppMesh>,
    worker: u32,
}

impl WorkerConnection for ShmMqWorkerConnection {
    fn stream_partition(&self, partition: usize) -> Result<WorkerPartitionStream> {
        let partition_u32 = u32::try_from(partition).map_err(|_| {
            DataFusionError::Internal(format!(
                "ShmMqWorkerConnection: partition={partition} > u32::MAX"
            ))
        })?;
        let drain = self.mesh.drain(self.worker, partition_u32).ok_or_else(|| {
            DataFusionError::Internal(format!(
                "ShmMqWorkerConnection: no drain for (worker={}, partition={partition_u32})",
                self.worker
            ))
        })?;
        let drain = Arc::clone(drain);
        // Cooperative pull loop: pgrx requires shm_mq FFI on the backend
        // thread, so the drain pass runs inline here. Each iteration drains
        // the receiver into the buffer (max 256 batches), then pops one
        // batch out to yield, then yields back to Tokio so sibling tasks
        // (e.g. the leader's own producer subplan) can advance.
        let stream = async_stream::stream! {
            loop {
                if let Err(e) = drain.poll_drain_pass() {
                    yield Err(e);
                    return;
                }
                match drain.buffer().try_pop() {
                    Some(DrainItem::Batch(batch)) => yield Ok(batch),
                    Some(DrainItem::Eof) => break,
                    None => tokio::task::yield_now().await,
                }
            }
        };
        Ok(Box::pin(stream))
    }
}

/// Stub [`WorkerResolver`] for the fork's distributed planner. Workers in
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
/// In the fork's gRPC distributed model, every worker would pull the
/// broadcast data from a designated source via a Flight stream. In our
/// in-process model the producer fragment that workers run already contains
/// these network boundary nodes (the worker re-plans from the same logical
/// plan as the leader), and rather than fan a broadcast across an in-process
/// mesh we just have each worker re-execute the build-side plan locally —
/// it's a small index scan, cheap enough that duplicating the work across
/// `n_workers` producers is preferable to wiring a second mesh.
///
/// The worker's session must keep `input_stage.plan = Some(...)` (i.e. the
/// fork's `prepare_plan` is *not* invoked on the worker side because we
/// drop into `find_worker_fragment` below the leader's `DistributedExec`).
pub struct LocalExecWorkerTransport;

impl WorkerTransport for LocalExecWorkerTransport {
    fn open(
        &self,
        input_stage: &Stage,
        _target_partitions: Range<usize>,
        _target_task: usize,
        ctx: &Arc<TaskContext>,
        _metrics: &ExecutionPlanMetricsSet,
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
    plan: Arc<dyn datafusion::physical_plan::ExecutionPlan>,
    ctx: Arc<TaskContext>,
}

impl WorkerConnection for LocalExecWorkerConnection {
    fn stream_partition(&self, partition: usize) -> Result<WorkerPartitionStream> {
        let stream = self.plan.execute(partition, Arc::clone(&self.ctx))?;
        // SendableRecordBatchStream is already `Pin<Box<dyn Stream<Item = Result<RecordBatch>> + Send>>`;
        // the WorkerPartitionStream alias drops the schema bound, so the
        // existing pinned box satisfies it directly.
        Ok(stream)
    }
}
