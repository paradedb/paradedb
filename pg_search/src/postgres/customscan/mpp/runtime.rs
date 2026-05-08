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

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use parking_lot::RwLock;

use async_trait::async_trait;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::{
    NetworkBoundary, NetworkShuffleExec, Stage, WorkerConnection, WorkerPartitionStream,
    WorkerResolver, WorkerTransport,
};
use url::Url;

use crate::postgres::customscan::mpp::mesh::MppPeerMesh;
use crate::postgres::customscan::mpp::transport::{DemuxDrainHandle, DrainHandle, DrainItem};

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

/// Implements the DF-D fork's [`WorkerTransport`] over the leader's [`MppMesh`].
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

/// Worker-side [`WorkerTransport`] that resolves a peer-mesh
/// `NetworkShuffleExec(consumer_tc=N, input_tc=N)` boundary.
///
/// `open(input_stage, target_partitions, target_task=producer_idx, ...)`
/// returns a [`ShmMqPeerWorkerConnection`] that streams from the peer-mesh
/// queue at column = this worker's idx, row = `target_task`. The data IN
/// that queue must be pushed by `target_task` running its inner producer
/// fragment concurrently — see `exec_mpp_worker`.
///
/// Each `(producer, consumer)` peer-mesh edge carries N partitions
/// multiplexed (one frame per partition tagged with `partition_id`). The
/// connection's `stream_partition(p)` returns the demuxed sub-buffer for
/// partition `p`, so DataFusion's scheduler calling
/// `stream_partition(off..off+pcount)` per consumer task gets one stream
/// per global partition.
pub struct ShmMqPeerWorkerTransport {
    peer_mesh: Arc<MppPeerMesh>,
}

impl ShmMqPeerWorkerTransport {
    pub fn new(peer_mesh: Arc<MppPeerMesh>) -> Self {
        Self { peer_mesh }
    }
}

impl WorkerTransport for ShmMqPeerWorkerTransport {
    fn open(
        &self,
        _input_stage: &Stage,
        _target_partitions: Range<usize>,
        target_task: usize,
        _ctx: &Arc<TaskContext>,
    ) -> Result<Box<dyn WorkerConnection>> {
        let producer = u32::try_from(target_task).map_err(|_| {
            DataFusionError::Internal(format!(
                "ShmMqPeerWorkerTransport: target_task={target_task} > u32::MAX"
            ))
        })?;
        let drain = self
            .peer_mesh
            .drain_for_producer(producer)
            .ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "ShmMqPeerWorkerTransport: no drain for producer={producer} \
                     (n_workers={})",
                    self.peer_mesh.n_workers
                ))
            })?
            .clone();
        Ok(Box::new(ShmMqPeerWorkerConnection { drain }))
    }
}

struct ShmMqPeerWorkerConnection {
    drain: Arc<DemuxDrainHandle>,
}

impl WorkerConnection for ShmMqPeerWorkerConnection {
    fn stream_partition(
        &self,
        partition: usize,
        _on_metadata: datafusion_distributed::OnMetadataCallback,
    ) -> Result<WorkerPartitionStream> {
        let tag = u32::try_from(partition).map_err(|_| {
            DataFusionError::Internal(format!(
                "ShmMqPeerWorkerConnection: partition={partition} > u32::MAX"
            ))
        })?;
        let drain = Arc::clone(&self.drain);
        // Each call to stream_partition(p) returns the per-tag sub-buffer's
        // stream. DataFusion's NetworkShuffleExec calls this once per
        // consumer-side partition, so each partition gets a fresh stream
        // backed by its own demux buffer.
        let buffer = drain
            .buffer(tag)
            .ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "ShmMqPeerWorkerConnection: no demux sub-buffer for tag={tag}"
                ))
            })?
            .clone();
        let stream = async_stream::stream! {
            loop {
                // Drain pass populates *all* sub-buffers for this producer
                // — we share the cooperative drain across every partition
                // stream, so any one stream's poll moves the others
                // forward.
                if let Err(e) = drain.poll_drain_pass() {
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

/// Shared, mutable routing map from `Stage` ID to peer mesh. The leader
/// builds the physical plan first (which assigns stage IDs) and only then
/// can it know which `stage.num()` corresponds to which DSM-allocated peer
/// mesh — so the map is populated *after* the worker session is built but
/// before plan execution. `CompositeWorkerTransport` holds an `Arc` over
/// this map; `exec_mpp_worker` holds a second `Arc` for the post-planning
/// write.
pub type SharedStageMeshMap = Arc<RwLock<HashMap<usize, Arc<MppPeerMesh>>>>;

/// Composite worker-side [`WorkerTransport`] that routes by the input
/// stage's `num` (its `Stage` ID). Each cross-worker shuffle stage is
/// associated with one peer mesh (allocated in DSM); broadcast stages
/// (or any stage without a registered mesh) fall through to
/// [`LocalExecWorkerTransport`] which re-executes the input subplan
/// locally.
///
/// The routing map is built at worker start by walking the worker fragment
/// (the input subtree of the OUTER `NetworkShuffleExec`) and zipping each
/// nested `NetworkShuffleExec`'s `stage.num()` with the corresponding
/// `MppPeerMesh` from `MppWorkerState::peer_meshes`. Both leader and worker
/// re-plan from the same logical plan deterministically, so the stage-num
/// ordering matches what the leader allocated in DSM.
pub struct CompositeWorkerTransport {
    routes: SharedStageMeshMap,
}

impl CompositeWorkerTransport {
    /// Build the composite transport over a shared routing map. The map
    /// can be empty at construction time and populated later via the
    /// `Arc<RwLock<...>>` held by the caller — `open()` reads under a
    /// shared lock per call.
    pub fn new(routes: SharedStageMeshMap) -> Self {
        Self { routes }
    }

    /// Convenience: an empty shared routing map.
    pub fn empty_routes() -> SharedStageMeshMap {
        Arc::new(RwLock::new(HashMap::new()))
    }
}

impl WorkerTransport for CompositeWorkerTransport {
    fn open(
        &self,
        input_stage: &Stage,
        target_partitions: Range<usize>,
        target_task: usize,
        ctx: &Arc<TaskContext>,
    ) -> Result<Box<dyn WorkerConnection>> {
        let routes = self.routes.read();
        if let Some(mesh) = routes.get(&input_stage.num()) {
            let peer = ShmMqPeerWorkerTransport::new(Arc::clone(mesh));
            // Drop the read guard before doing async work below.
            drop(routes);
            return peer.open(input_stage, target_partitions, target_task, ctx);
        }
        drop(routes);
        // No peer mesh registered for this stage: must be a broadcast or a
        // deeper-nested shuffle that's still elided by the planner. Fall
        // through to LocalExec which re-executes the input subplan locally.
        LocalExecWorkerTransport.open(input_stage, target_partitions, target_task, ctx)
    }
}

/// Walk a worker fragment plan (the input subtree of the OUTER
/// `NetworkShuffleExec`) and collect the `stage.num()` of every nested
/// `NetworkShuffleExec`, in pre-order DFS. The order is deterministic
/// across leader and worker because both re-plan from the same logical
/// plan — used to zip stage numbers with the leader's allocated peer
/// meshes.
pub fn collect_inner_shuffle_stage_ids(plan: &Arc<dyn ExecutionPlan>) -> Vec<usize> {
    let mut out = Vec::new();
    collect_inner_shuffle_stage_ids_inner(plan, &mut out);
    out
}

fn collect_inner_shuffle_stage_ids_inner(plan: &Arc<dyn ExecutionPlan>, out: &mut Vec<usize>) {
    if let Some(nse) = plan.as_any().downcast_ref::<NetworkShuffleExec>() {
        out.push(nse.input_stage().num());
    }
    for child in plan.children() {
        collect_inner_shuffle_stage_ids_inner(child, out);
    }
}

/// Sibling of [`collect_inner_shuffle_stage_ids`]. Walks the plan in the
/// same pre-order DFS and returns each nested `NetworkShuffleExec`'s
/// `children()[0]` — the producer subtree the worker must run to feed
/// that peer-mesh stage. The returned Vec is in the same order as
/// [`collect_inner_shuffle_stage_ids`] and `peer_meshes`, so both can be
/// zipped to spawn K concurrent inner-fragment runners.
pub fn collect_inner_producer_fragments(
    plan: &Arc<dyn ExecutionPlan>,
) -> Vec<Arc<dyn ExecutionPlan>> {
    let mut out = Vec::new();
    collect_inner_producer_fragments_inner(plan, &mut out);
    out
}

fn collect_inner_producer_fragments_inner(
    plan: &Arc<dyn ExecutionPlan>,
    out: &mut Vec<Arc<dyn ExecutionPlan>>,
) {
    if plan.as_any().downcast_ref::<NetworkShuffleExec>().is_some() {
        if let Some(child) = plan.children().first() {
            out.push(Arc::clone(child));
        }
    }
    for child in plan.children() {
        collect_inner_producer_fragments_inner(child, out);
    }
}

/// Build a `HashMap<stage_num, MppPeerMesh>` by zipping the stage IDs
/// produced by [`collect_inner_shuffle_stage_ids`] with the worker's
/// `peer_meshes` Vec (in the order the leader allocated them in DSM).
/// If the lengths disagree this returns an error, since a mismatch
/// indicates the leader and worker disagreed on plan shape.
pub fn build_stage_to_mesh_map(
    stage_ids: &[usize],
    peer_meshes: &[Arc<MppPeerMesh>],
) -> Result<HashMap<usize, Arc<MppPeerMesh>>, DataFusionError> {
    if stage_ids.len() != peer_meshes.len() {
        return Err(DataFusionError::Internal(format!(
            "mpp: leader allocated {} peer meshes but worker plan has {} cross-worker shuffles",
            peer_meshes.len(),
            stage_ids.len()
        )));
    }
    Ok(stage_ids
        .iter()
        .zip(peer_meshes.iter())
        .map(|(stage, mesh)| (*stage, Arc::clone(mesh)))
        .collect())
}
