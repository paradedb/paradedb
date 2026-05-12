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
    enable_mpp, mpp_cache_per_slot, mpp_queue_size as gucs_mpp_queue_size,
    mpp_worker_count as gucs_mpp_worker_count,
};
use crate::postgres::customscan::mpp::dsm::{
    compute_dsm_layout, leader_init, peer_proc_for_index, worker_attach, MppBuildCache,
};
use crate::postgres::customscan::mpp::runtime::MppMesh;
use crate::postgres::customscan::mpp::transport::{
    DrainHandle, MppFrameHeader, MppReceiver, MppSender,
};
use crate::postgres::customscan::mpp::MppParticipantConfig;

/// Stage id stamped onto frames in the natural-shape single-stage gather.
/// All worker→leader traffic in M1's single-stage path carries this stage id.
/// M2 introduces multi-stage pipelines and replaces this constant with
/// plan-driven stage ids derived from each `NetworkBoundary.input_stage.num`.
const NATURAL_GATHER_STAGE_ID: u32 = 0;

/// Consumer-side partition stamped on natural-shape gather frames. The
/// natural plan emits `NetworkCoalesceExec(consumer_tc=1, input_tc=N)` at the
/// top, so there is exactly one consumer partition on the leader.
const NATURAL_GATHER_PARTITION: u32 = 0;

/// True iff `paradedb.enable_mpp = on` and `paradedb.mpp_worker_count >= 3`.
/// Customscan path-builders gate `parallel_workers` on this.
///
/// We require `>= 3` (not `>= 2`) because the leader is consumer-only in
/// this iteration: with `mpp_worker_count = 2`, [`producer_worker_count`]
/// returns 1, so [`crate::postgres::customscan::aggregatescan`] would size
/// the DSM mesh as `1 × mpp_n_partitions = 1` while
/// `with_target_partitions(2)` (clamped at 2 by `n_workers.max(2)`) makes
/// the planner build a 2-partition shuffle. The mesh would not have a
/// queue for the second partition. Gating at `>= 3` ensures
/// `producer_worker_count >= 2`, so the mesh shape and the planner's
/// shuffle width match. (Lifting this to `>= 2` is safe once
/// leader-as-worker-0 is wired up, or once `target_partitions` is dropped
/// to `producer_worker_count`.)
pub fn mpp_is_active() -> bool {
    enable_mpp() && gucs_mpp_worker_count() >= 3
}

/// Total participant count: leader + producers. Clamped at 3 so the mesh
/// shape matches the planner's `target_partitions` (see [`mpp_is_active`]).
pub fn mpp_worker_count() -> u32 {
    gucs_mpp_worker_count().max(3) as u32
}

/// Per-edge queue size from the GUC.
pub fn mpp_queue_size() -> usize {
    gucs_mpp_queue_size()
}

/// Body of `estimate_dsm_custom_scan`. Returns total DSM bytes the leader
/// will need for the plan + multiplexed `n_procs × n_procs` queue mesh +
/// build-side cache. `n_procs` is the total participant count (leader +
/// `producer_worker_count()` parallel workers).
///
/// `n_cache_sources` is the number of non-partitioning sources the build-side
/// all-gather cache should reserve slots for; pass 0 to skip caching. The
/// cache region is sized using worker-only slots (leader does not write).
pub fn estimate_dsm_size(plan_bytes_len: usize, n_cache_sources: u32) -> Result<usize, String> {
    let layout = compute_dsm_layout(
        n_procs(),
        mpp_queue_size(),
        plan_bytes_len,
        n_cache_sources,
        mpp_cache_per_slot(),
    )
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
pub fn n_procs() -> u32 {
    mpp_worker_count()
}

/// Returned to the leader from [`leader_setup`]. The customscan stashes this
/// on its execution state and consults it during `exec_custom_scan`.
///
/// The leader is consumer-only in this iteration, so its outbound senders
/// (worker-0 producer slot) and `MppParticipantConfig` are not held here —
/// they are dropped inside `leader_setup` and will be re-introduced when
/// leader-as-worker-0 is wired up.
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
    n_cache_sources: u32,
) -> Result<MppLeaderState, String> {
    let total_procs = n_procs();
    let layout = compute_dsm_layout(
        total_procs,
        mpp_queue_size(),
        plan_bytes.len(),
        n_cache_sources,
        mpp_cache_per_slot(),
    )
    .map_err(|e| format!("mpp: leader_setup compute layout: {e}"))?;

    let attach = unsafe { leader_init(coordinate, seg, &layout, &plan_bytes) }?;

    // M1.c: build the proc-pair indexed mesh. The leader is `proc_idx = 0`;
    // its inbound queues are `slot(*, 0)` for every peer (workers 1..n_procs).
    // `attach.inbound_receivers` is already peer-indexed (self-loop skipped),
    // so receiver[i] corresponds to peer_proc_for_index(0, i) = i + 1.
    //
    // The mesh stores `inbound_drains[sender_proc]` so `runtime.rs` can look
    // up the right drain in O(1) given a sender_proc from the natural-shape
    // gather; the self-loop entry at index 0 is `None`.
    //
    // M2.b: each `DrainHandle` owns a per-`(stage_id, partition)` sub-buffer
    // registry, so one shm_mq queue can multiplex frames from many logical
    // channels — the sub-buffer is created lazily on first frame OR by
    // `WorkerConnection::stream_partition` registering ahead of time.
    let _ = n_partitions;
    let mut inbound_drains: Vec<Option<Arc<DrainHandle>>> =
        Vec::with_capacity(total_procs as usize);
    inbound_drains.push(None); // self-loop slot at sender_proc = 0
    for (peer_idx, shm_recv) in attach.inbound_receivers.into_iter().enumerate() {
        let sender_proc = peer_proc_for_index(0, peer_idx as u32);
        debug_assert_eq!(
            sender_proc as usize,
            inbound_drains.len(),
            "peer_proc_for_index must produce sender_proc indices in order"
        );
        let mpp_recv = MppReceiver::new(Box::new(shm_recv));
        inbound_drains.push(Some(Arc::new(DrainHandle::cooperative(vec![mpp_recv]))));
    }

    let mesh = Arc::new(MppMesh {
        this_proc: 0,
        n_procs: total_procs,
        inbound_drains,
    });

    // Drop the leader's own outbound senders — the leader doesn't yet host
    // a producer fragment in single-stage mode.
    drop(attach.outbound_senders);

    // Cache region is sized into the layout for workers. Leader doesn't touch.
    let _ = n_cache_sources;

    Ok(MppLeaderState { mesh, pcxt })
}

/// Returned to a worker from [`worker_setup`]. The customscan reads the plan
/// bytes, runs the plan, and pushes resulting batches through `outbound_senders`.
pub struct MppWorkerState {
    /// Per-partition outbound senders this worker writes to.
    /// `outbound_senders[p]` writes to `slot(this_worker, p)`.
    pub outbound_senders: Vec<MppSender>,
    /// Worker fragment plan bytes, copied out of DSM. Caller deserializes via
    /// the `PgSearchExtensionCodec` to get an `Arc<dyn ExecutionPlan>`.
    pub plan_bytes: Vec<u8>,
    pub participant_config: MppParticipantConfig,
    /// Build-side all-gather cache. The worker writes its 1/N slice for each
    /// non-partitioning source, then reads back peer slices after the barrier.
    pub build_cache: Option<Arc<MppBuildCache>>,
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
    // M1.b: leader is now `proc_idx = 0`, workers are `1..n_procs`. Worker N
    // maps from PG's `ParallelWorkerNumber = N` to `proc_idx = N + 1`.
    let proc_idx = (worker_number as u32) + 1;

    let (header, plan_bytes, attach) =
        unsafe { worker_attach(coordinate, region_total, proc_idx, seg) }?;

    // Pick out the worker→leader sender. With self-loops skipped, the worker's
    // peer-indexed row starts at peer_idx=0 → sender_proc=0 (leader). So
    // `outbound_senders[0]` writes to `slot(this_proc, 0)`, the worker→leader
    // queue. Frames carry the natural-shape gather header
    // `(stage_id=0, partition=0)` since the current path emits
    // one `NetworkCoalesceExec(consumer_tc=1, input_tc=N)` at the top.
    //
    // M2 will replace this with per-target-proc senders that multiplex many
    // (stage_id, partition) channels over one shm_mq queue.
    let mut row = attach.outbound_senders;
    if row.is_empty() {
        return Err("mpp: worker_attach returned empty senders row".into());
    }
    // Sanity: with self-loop skipped, peer_idx 0 maps to sender_proc 0 (leader).
    debug_assert_eq!(peer_proc_for_index(proc_idx, 0), 0);
    let leader_sender = row.remove(0);
    drop(row);
    drop(attach.inbound_receivers); // unused in single-stage worker

    // M2.a: instead of one MppSender (header partition=0), share the single
    // shm_mq queue across `n_partitions` senders, each tagged with a different
    // partition. The worker's producer fragment in the natural plan outputs
    // multiple hash partitions per task; `run_producer_fragment` drives one
    // sender per output partition. They all multiplex over the same
    // shm_mq queue and the leader's drain demuxes by frame header.
    //
    // The actual `n_partitions` is determined from the physical plan at
    // worker exec time (since the worker is what knows how many partitions
    // the fragment emits). For now, we hand back one Arc to the channel; the
    // caller (aggregatescan::exec_mpp_worker) clones it into N senders.
    let shared_channel: Arc<dyn crate::postgres::customscan::mpp::transport::BatchChannelSender> =
        Arc::new(leader_sender);
    // Build a one-element vec for back-compat with the existing call site;
    // the exec path will clone the Arc out of `outbound_senders[0]` and
    // expand to the right number of senders once the physical plan is built.
    let outbound_senders = vec![MppSender::with_header(
        Arc::clone(&shared_channel),
        MppFrameHeader::batch(NATURAL_GATHER_STAGE_ID, NATURAL_GATHER_PARTITION),
    )];

    let build_cache = if header.n_cache_sources > 0 {
        Some(Arc::new(unsafe {
            MppBuildCache::from_header(coordinate as *mut u8, &header)
        }))
    } else {
        None
    };

    // For MppParticipantConfig, expose worker-only counts (N producer workers).
    let worker_count = header.n_procs.saturating_sub(1).max(1);
    Ok(MppWorkerState {
        outbound_senders,
        plan_bytes,
        participant_config: MppParticipantConfig {
            participant_index: worker_number as u32,
            total_workers: worker_count,
        },
        build_cache,
    })
}
