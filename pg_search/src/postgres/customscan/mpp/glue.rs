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

//! High-level glue between PostgreSQL parallel-query callbacks and the
//! coordinator/worker MPP architecture.
//!
//! Customscan code calls into this module from four hooks; everything else
//! (DSM math, shm_mq FFI, DF-D fork's `WorkerTransport` plumbing) is hidden
//! behind the API:
//!
//! - [`mpp_is_active`] — gate for the customscan path-builder.
//! - [`estimate_dsm_size`] — `estimate_dsm_custom_scan` body.
//! - [`leader_setup`] — `initialize_dsm_custom_scan` body. Returns the
//!   leader's [`MppLeaderState`] which carries the runtime [`MppMesh`]
//!   handle the customscan installs on its DataFusion `SessionContext`.
//! - [`worker_setup`] — `initialize_worker_custom_scan` body. Returns the
//!   worker's [`MppWorkerState`] which carries the worker's outbound
//!   senders and the deserialized plan bytes the worker runs.

use std::ffi::c_void;
use std::sync::Arc;

use pgrx::pg_sys;

use crate::gucs::{
    enable_mpp, mpp_queue_size as gucs_mpp_queue_size, mpp_worker_count as gucs_mpp_worker_count,
};
use crate::postgres::customscan::mpp::dsm::{
    compute_dsm_layout, leader_init, peer_proc_for_index, worker_attach,
};
use crate::postgres::customscan::mpp::mesh::{ShmMqReceiver, ShmMqSender};
use crate::postgres::customscan::mpp::runtime::MppMesh;
use crate::postgres::customscan::mpp::transport::{
    in_proc_channel, BatchChannelSender, DrainHandle, MppFrameHeader, MppReceiver, MppSender,
    SELF_LOOP_CAPACITY,
};
use crate::postgres::customscan::mpp::MppParticipantConfig;

/// Default stage id stamped on outbound sender headers before the per-fragment dispatcher
/// rewrites them. Worker senders get `clone_with_header` immediately after `worker_setup`, so
/// this placeholder is never observed on the wire.
const NATURAL_GATHER_STAGE_ID: u32 = 0;

/// Consumer-side partition stamped on natural-shape gather frames. The natural plan emits
/// `NetworkCoalesceExec(consumer_tc=1, input_tc=N)` at the top, so there is exactly one consumer
/// partition on the leader.
const NATURAL_GATHER_PARTITION: u32 = 0;

/// True iff `paradedb.enable_mpp = on` and `paradedb.mpp_worker_count >= 3`. Customscan
/// path-builders gate `parallel_workers` on this.
///
/// `>= 3` (not `>= 2`) because the leader is consumer-only: with `mpp_worker_count = 2`,
/// [`producer_worker_count`] returns 1, so [`crate::postgres::customscan::aggregatescan`] sizes
/// the DSM mesh as `1 × mpp_n_partitions = 1` while `with_target_partitions(2)` (clamped by
/// `n_workers.max(2)`) makes the planner build a 2-partition shuffle. The mesh wouldn't have a
/// queue for the second partition. Gating at `>= 3` keeps `producer_worker_count >= 2` so mesh
/// shape and shuffle width line up.
pub fn mpp_is_active() -> bool {
    enable_mpp() && gucs_mpp_worker_count() >= 3
}

/// Total participant count: leader + producers. Clamped at 3 so the mesh
/// shape matches the planner's `target_partitions` (see [`mpp_is_active`]).
pub fn mpp_worker_count() -> u32 {
    gucs_mpp_worker_count().max(3) as u32
}

/// Per-edge queue size from the GUC.
pub(super) fn mpp_queue_size() -> usize {
    gucs_mpp_queue_size()
}

/// Body of `estimate_dsm_custom_scan`. Returns total DSM bytes the leader will need for the
/// plan, the multiplexed `n_procs × n_procs` queue mesh, and the worker plan. `n_procs` is the
/// total participant count (leader + `producer_worker_count()` parallel workers).
pub fn estimate_dsm_size(plan_bytes_len: usize) -> Result<usize, String> {
    let layout = compute_dsm_layout(n_procs(), mpp_queue_size(), plan_bytes_len)
        .map_err(|e| format!("mpp: estimate_dsm_size: {e}"))?;
    Ok(layout.region_total)
}

/// Number of producer workers PG should launch as `parallel_workers`.
/// `mpp_worker_count - 1` because participant 0 is the leader.
pub fn producer_worker_count() -> u32 {
    mpp_worker_count().saturating_sub(1).max(1)
}

/// Total participant count: 1 leader + N producer workers. This is the
/// dimension of the multiplexed `n_procs × n_procs` shm_mq grid.
pub(super) fn n_procs() -> u32 {
    mpp_worker_count()
}

/// Returned to the leader from [`leader_setup`]. The customscan stashes this on its execution
/// state and consults it during `exec_custom_scan`.
///
/// The leader is consumer-only: it gathers fragments from worker procs but doesn't host a
/// producer fragment itself. Its outbound senders are dropped inside `leader_setup`, and it
/// carries no `MppParticipantConfig`.
pub struct MppLeaderState {
    /// Runtime mesh handle. Install on the leader's `SessionContext` via
    /// `with_extension(Arc::clone(&mesh))` so `ShmMqWorkerTransport` can find
    /// it at execute time.
    pub mesh: Arc<MppMesh>,
    /// Borrowed pointer to the parallel context PG passed to
    /// `initialize_dsm_custom_scan`. Lifetime: valid for the duration of
    /// the parallel exec; PG destroys it after `ExecParallelFinish`. The
    /// leader reads `(*pcxt).nworkers_launched` at exec time to detect
    /// short worker launches (see #5061 for the long-term plan); the
    /// raw pointer is the only way to get at that field since the
    /// CustomScan exec callback doesn't get `ParallelContext` directly.
    pub pcxt: *mut pg_sys::ParallelContext,
}

/// Wrap each peer-indexed `ShmMqReceiver` into an inbound cooperative drain, returned in
/// `sender_proc`-indexed form. Slot at index `this_proc` is `None` (no self-loop drain at this
/// stage; the worker's self-loop is installed separately, and the leader never has one).
fn build_inbound_drains(
    this_proc: u32,
    total_procs: u32,
    peer_receivers: Vec<ShmMqReceiver>,
) -> Vec<Option<Arc<DrainHandle>>> {
    let mut drains: Vec<Option<Arc<DrainHandle>>> = (0..total_procs).map(|_| None).collect();
    for (peer_idx, shm_recv) in peer_receivers.into_iter().enumerate() {
        let sender_proc = peer_proc_for_index(this_proc, peer_idx as u32);
        debug_assert!(sender_proc != this_proc);
        debug_assert!(sender_proc < total_procs);
        let mpp_recv = MppReceiver::new(Box::new(shm_recv));
        drains[sender_proc as usize] = Some(Arc::new(DrainHandle::cooperative(vec![mpp_recv])));
    }
    drains
}

/// Wrap each peer-indexed `ShmMqSender` into an outbound `MppSender` keyed by `target_proc`. The
/// per-fragment dispatcher in `aggregatescan::exec_mpp_worker` immediately `clone_with_header`s
/// these to the right `(stage_id, partition)`, so the default placeholder header is never
/// observed on the wire. Slot at index `this_proc` is `None`; the worker's self-loop install
/// fills it in afterward.
fn build_outbound_senders(
    this_proc: u32,
    total_procs: u32,
    peer_senders: Vec<ShmMqSender>,
) -> Vec<Option<MppSender>> {
    let mut senders: Vec<Option<MppSender>> = (0..total_procs).map(|_| None).collect();
    for (peer_idx, shm_send) in peer_senders.into_iter().enumerate() {
        let target_proc = peer_proc_for_index(this_proc, peer_idx as u32);
        debug_assert!(target_proc != this_proc);
        debug_assert!(target_proc < total_procs);
        let shared: Arc<dyn BatchChannelSender> = Arc::new(shm_send);
        senders[target_proc as usize] = Some(MppSender::with_header(
            shared,
            MppFrameHeader::batch(NATURAL_GATHER_STAGE_ID, NATURAL_GATHER_PARTITION),
        ));
    }
    senders
}

/// Body of `initialize_dsm_custom_scan`. Allocates the queue mesh, populates
/// the [`MppMesh`] handle, and serializes the worker plan into DSM.
///
/// # Safety
/// - `coordinate` must be the DSM region pointer PG supplied to
///   `initialize_dsm_custom_scan`.
/// - `seg` must be the leader's `dsm_segment*`.
/// - `plan_bytes` must have the same length passed to [`estimate_dsm_size`]
///   so the leader doesn't overrun the DSM region PG allocated.
pub unsafe fn leader_setup(
    coordinate: *mut c_void,
    seg: *mut pg_sys::dsm_segment,
    pcxt: *mut pg_sys::ParallelContext,
    n_partitions: u32,
    plan_bytes: Vec<u8>,
) -> Result<MppLeaderState, String> {
    let total_procs = n_procs();
    let layout = compute_dsm_layout(total_procs, mpp_queue_size(), plan_bytes.len())
        .map_err(|e| format!("mpp: leader_setup compute layout: {e}"))?;

    let attach = unsafe { leader_init(coordinate, seg, &layout, &plan_bytes) }?;
    let _ = n_partitions;

    // Each `DrainHandle` owns a per-`(stage_id, partition)` sub-buffer registry, so one shm_mq
    // queue can carry frames from many logical channels. Sub-buffers are created lazily on first
    // frame, or up-front by `WorkerConnection::stream_partition`.
    let inbound_drains = build_inbound_drains(0, total_procs, attach.inbound_receivers);

    let mesh = Arc::new(MppMesh::new(0, total_procs, inbound_drains));

    // Drop the leader's own outbound senders. The leader is consumer-only and never hosts a
    // producer fragment.
    drop(attach.outbound_senders);

    Ok(MppLeaderState { mesh, pcxt })
}

/// Returned to a worker from [`worker_setup`]. The customscan reads the plan bytes, runs the
/// plan, and pushes resulting batches through `outbound_senders`.
pub struct MppWorkerState {
    /// `outbound_senders[proc_idx]` is the sender that writes to `slot(this_proc, proc_idx)`.
    /// The entry at `proc_idx == this_proc` is `None` because the self-loop slot is never
    /// attached (matches the leader's `inbound_drains` shape).
    ///
    /// Each fragment's per-partition output sender is keyed by
    /// `outbound_senders[proc_for_task(n_workers, consumer_task)]`. Each `MppSender` wraps an
    /// `Arc<dyn BatchChannelSender>` so callers can `clone_with_header` to multiplex
    /// `(stage_id, partition)` channels onto one shm_mq queue.
    pub outbound_senders: Vec<Option<MppSender>>,
    /// Worker fragment plan bytes, copied out of DSM. Caller deserializes via the
    /// `PgSearchExtensionCodec` to get an `Arc<dyn ExecutionPlan>`.
    pub plan_bytes: Vec<u8>,
    pub participant_config: MppParticipantConfig,
    /// Worker's MppMesh, same shape as the leader's. `inbound_drains[sender_proc]` pulls frames
    /// from `slot(sender_proc, this_proc)`. Workers consume from peers when running consumer
    /// fragments (e.g. a `FinalPartitioned` aggregate above a `NetworkShuffleExec` peer-mesh).
    /// Read by the multi-fragment dispatcher in `aggregatescan::exec_mpp_worker`.
    pub mesh: Arc<MppMesh>,
}

/// Body of `initialize_worker_custom_scan`. Reads the header, attaches as
/// sender on this worker's slot row, copies the plan bytes out of DSM.
///
/// # Safety
/// - `coordinate` must be the DSM region pointer PG supplied.
/// - `region_total` must match the DSM's attached size.
/// - `seg` may be NULL; PG's `initialize_worker_custom_scan` does not
///   surface the segment pointer.
pub unsafe fn worker_setup(
    coordinate: *mut c_void,
    region_total: u64,
    worker_number: i32,
    seg: *mut pg_sys::dsm_segment,
) -> Result<MppWorkerState, String> {
    if worker_number < 0 {
        return Err("mpp: worker_number < 0".into());
    }
    // Leader is `proc_idx = 0`, workers are `1..n_procs`. Worker N maps from PG's
    // `ParallelWorkerNumber = N` to `proc_idx = N + 1`.
    let proc_idx = (worker_number as u32) + 1;

    let (header, plan_bytes, attach) =
        unsafe { worker_attach(coordinate, region_total, proc_idx, seg) }?;
    let total_procs = header.n_procs;

    let mut outbound_senders =
        build_outbound_senders(proc_idx, total_procs, attach.outbound_senders);
    let mut inbound_drains = build_inbound_drains(proc_idx, total_procs, attach.inbound_receivers);
    debug_assert_eq!(
        outbound_senders.iter().filter(|s| s.is_some()).count(),
        (total_procs as usize).saturating_sub(1),
        "worker outbound senders: expected {} non-self-loop entries before self-loop install",
        total_procs.saturating_sub(1)
    );

    // Install a self-loop in-proc channel for `slot(this_proc, this_proc)`. Peer-mesh hash
    // routing can land producer-side and consumer-side tasks for the same `(stage, partition)`
    // on the same worker. Without this, the shm_mq grid's unattached diagonal would surface as
    // `outbound_senders[this_proc] = None`. The in-proc channel keeps frame routing uniform from
    // the dispatcher's perspective: senders push, the `DrainHandle` reads via the same
    // `BatchChannelReceiver` contract as shm_mq, and the sub-buffer registry demuxes per
    // `(stage_id, partition)`.
    let (self_tx, self_rx) = in_proc_channel(SELF_LOOP_CAPACITY);
    let self_tx_arc: Arc<dyn BatchChannelSender> = Arc::new(self_tx);
    outbound_senders[proc_idx as usize] = Some(MppSender::with_header(
        self_tx_arc,
        MppFrameHeader::batch(NATURAL_GATHER_STAGE_ID, NATURAL_GATHER_PARTITION),
    ));
    inbound_drains[proc_idx as usize] =
        Some(Arc::new(DrainHandle::cooperative(vec![MppReceiver::new(
            Box::new(self_rx),
        )])));

    let mesh = Arc::new(MppMesh::new(proc_idx, total_procs, inbound_drains));

    let worker_count = total_procs.saturating_sub(1).max(1);
    Ok(MppWorkerState {
        outbound_senders,
        plan_bytes,
        participant_config: MppParticipantConfig {
            total_workers: worker_count,
        },
        mesh,
    })
}
