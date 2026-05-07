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
    compute_dsm_layout, leader_init, worker_attach, worker_peer_attach, MppBuildCache,
    MPP_CACHE_PER_SLOT,
};
use crate::postgres::customscan::mpp::mesh::MppPeerMesh;
use crate::postgres::customscan::mpp::runtime::MppMesh;
use crate::postgres::customscan::mpp::transport::{
    DrainBuffer, DrainHandle, MppReceiver, MppSender,
};
use crate::postgres::customscan::mpp::MppParticipantConfig;

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

/// Per-edge peer-mesh queue size, or 0 when `enable_mpp_postagg_shuffle = off`
/// (peer mesh skipped entirely, no DSM reserved for it).
pub fn mpp_peer_queue_bytes() -> usize {
    if crate::gucs::enable_mpp_postagg_shuffle() {
        crate::gucs::mpp_peer_queue_bytes()
    } else {
        0
    }
}

/// Body of `estimate_dsm_custom_scan`. Returns total DSM bytes the leader
/// will need for plan + N×K queue mesh + build-side cache. `N` is the *worker*
/// count (`mpp_worker_count - 1`); the leader is a consumer-only participant
/// in this iteration, so its slot is omitted from the mesh.
///
/// `n_cache_sources` is the number of non-partitioning sources the build-side
/// all-gather cache should reserve slots for; pass 0 to skip caching.
pub fn estimate_dsm_size(
    plan_bytes_len: usize,
    n_partitions: u32,
    n_cache_sources: u32,
) -> Result<usize, String> {
    let layout = compute_dsm_layout(
        producer_worker_count(),
        n_partitions,
        mpp_queue_size(),
        plan_bytes_len,
        n_cache_sources,
        MPP_CACHE_PER_SLOT,
        mpp_peer_queue_bytes(),
    )
    .map_err(|e| format!("mpp: estimate_dsm_size: {e}"))?;
    Ok(layout.region_total)
}

/// Number of producer workers in the mesh: `mpp_worker_count - 1`. The leader
/// is a consumer-only participant for now (leader-as-worker-0 deferred); its
/// slot is omitted, so PG launches exactly this many parallel workers.
pub fn producer_worker_count() -> u32 {
    mpp_worker_count().saturating_sub(1).max(1)
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
    n_partitions: u32,
    plan_bytes: Vec<u8>,
    n_cache_sources: u32,
) -> Result<MppLeaderState, String> {
    let n_workers = producer_worker_count();
    let layout = compute_dsm_layout(
        n_workers,
        n_partitions,
        mpp_queue_size(),
        plan_bytes.len(),
        n_cache_sources,
        MPP_CACHE_PER_SLOT,
        mpp_peer_queue_bytes(),
    )
    .map_err(|e| format!("mpp: leader_setup compute layout: {e}"))?;

    let attach = unsafe { leader_init(coordinate, seg, &layout, &plan_bytes) }?;

    // Build per-(worker, partition) cooperative drain handles. Each handle
    // owns one MppReceiver + one DrainBuffer; the consumer side calls
    // `poll_drain_pass` inline on the backend thread (pgrx-safe) when
    // pulling batches.
    let mut drains = Vec::with_capacity(attach.inbound_receivers.len());
    for shm_recv in attach.inbound_receivers {
        let mpp_recv = MppReceiver::new(Box::new(shm_recv));
        let buffer = DrainBuffer::new(1);
        drains.push(Arc::new(DrainHandle::cooperative(vec![mpp_recv], buffer)));
    }

    let mesh = Arc::new(MppMesh {
        n_workers,
        n_partitions,
        drains,
    });

    // Drop the leader's own producer slot senders — the leader doesn't
    // produce in this iteration, so its slot would never receive data.
    // Dropping them now means peer receivers observe Detached at first
    // poll and short-circuit cleanly. (When leader-as-worker-0 lands,
    // we'll route these into a Tokio-spawned producer subplan instead.)
    drop(attach.outbound_senders);

    // The leader doesn't read or write the build-side cache — workers attach
    // to it via DSM offset, and Postgres owns the DSM segment's lifetime.
    // `n_cache_sources` is sized into the layout for the worker side.
    let _ = n_cache_sources;

    Ok(MppLeaderState { mesh })
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
    /// Peer-mesh handle for the inner cross-worker shuffle (Track B). `None`
    /// when `enable_mpp_postagg_shuffle = off` and no peer mesh was reserved.
    pub peer_mesh: Option<Arc<MppPeerMesh>>,
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
    // Leader is consumer-only, so all parallel workers map directly to mesh
    // slots: ParallelWorkerNumber 0..N-1 maps to slot 0..N-1.
    let worker_index = worker_number as u32;

    let (header, plan_bytes, attach) =
        unsafe { worker_attach(coordinate, region_total, worker_index, seg) }?;

    let outbound_senders = attach
        .outbound_senders
        .into_iter()
        .map(|s| MppSender::new(Box::new(s)))
        .collect();

    let build_cache = if header.n_cache_sources > 0 {
        Some(Arc::new(unsafe {
            MppBuildCache::from_header(coordinate as *mut u8, &header)
        }))
    } else {
        None
    };

    // Peer-mesh demux tag space = global partition count of the inner
    // shuffle. Fork's `_distribute_plan` scales the inner `RepartitionExec`
    // by `consumer_tc = n_workers`, so the producer emits `n_workers²`
    // distinct partition_ids and each `(producer, consumer)` peer-mesh
    // edge multiplexes `n_workers` of them.
    let peer_partitions = header.n_workers.saturating_mul(header.n_workers);
    let peer_mesh =
        unsafe { worker_peer_attach(coordinate, &header, worker_index, seg) }?.map(|attach| {
            Arc::new(MppPeerMesh::from_worker_attach(
                worker_index,
                header.n_workers,
                peer_partitions,
                attach,
            ))
        });

    Ok(MppWorkerState {
        outbound_senders,
        plan_bytes,
        participant_config: MppParticipantConfig {
            participant_index: worker_index,
            total_participants: header.n_workers,
        },
        build_cache,
        peer_mesh,
    })
}
