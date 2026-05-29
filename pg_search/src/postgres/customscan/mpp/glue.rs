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
use crate::postgres::customscan::mpp::dsm_mpsc_ring::DsmMpscSender;
use crate::postgres::customscan::mpp::mesh::{DsmInboxReceiver, DsmInboxSender};
use crate::postgres::customscan::mpp::runtime::MppMesh;
use crate::postgres::customscan::mpp::transport::{
    in_proc_channel, BatchChannelSender, DrainHandle, MppFrameHeader, MppReceiver, MppSender,
    SELF_LOOP_CAPACITY,
};

/// Default stage id stamped on outbound sender headers before the per-fragment dispatcher
/// rewrites them. Worker senders get `clone_with_header` immediately after `worker_setup`, so
/// this placeholder is never observed on the wire.
const NATURAL_GATHER_STAGE_ID: u32 = 0;

/// Consumer-side partition stamped on natural-shape gather frames. The natural plan emits
/// `NetworkCoalesceExec(consumer_tc=1, input_tc=N)` at the top, so there is exactly one consumer
/// partition on the leader.
const NATURAL_GATHER_PARTITION: u32 = 0;

/// Minimum total procs for MPP: leader (consumer-only) plus at least 2 producers. Single
/// source of truth so [`mpp_is_active`] and [`mpp_worker_count`] don't drift on the
/// threshold. Below 3, [`producer_worker_count`] would be 1 while
/// `build_mpp_session_context` still clamps `target_partitions` to 2; the mesh wouldn't
/// have a queue for the second partition.
const MIN_TOTAL_WORKER_COUNT: i32 = 3;

/// True iff `paradedb.enable_mpp = on` and `paradedb.mpp_worker_count >=
/// MIN_TOTAL_WORKER_COUNT`. Customscan path-builders gate `parallel_workers` on this.
/// Also requires that the system has enough `max_parallel_workers` and
/// `max_parallel_workers_per_gather` to launch the requested number of producers.
pub fn mpp_is_active() -> bool {
    let active = enable_mpp() && gucs_mpp_worker_count() >= MIN_TOTAL_WORKER_COUNT;
    if !active {
        return false;
    }

    let producer_count = gucs_mpp_worker_count() - 1;
    let max_per_gather = unsafe { pg_sys::max_parallel_workers_per_gather };
    let max_workers = unsafe { pg_sys::max_parallel_workers };

    producer_count <= max_per_gather && producer_count <= max_workers
}

/// Total proc count: leader + producers. Equals the GUC value when [`mpp_is_active`] is
/// true. Callers must gate on [`mpp_is_active`] first. Debug builds assert; release builds
/// return the raw GUC, which can leave [`producer_worker_count`] below 2 and break the
/// planner's `target_partitions` / mesh-width invariant.
pub fn mpp_worker_count() -> u32 {
    debug_assert!(
        mpp_is_active(),
        "mpp_worker_count() called when mpp_is_active() is false — callers must gate first"
    );
    gucs_mpp_worker_count() as u32
}

/// Customscan-side header at offset 0 of the DSM coordinate that the leader hands to
/// `leader_setup` / workers see in `initialize_worker_custom_scan`. Tells workers where the
/// MPP region begins (past the customscan's `ParallelScanState` block) and which entry in
/// `plan.sources()` is the partitioning source.
///
/// DSM layout used by every customscan opting into MPP:
///
/// ```text
/// [0 .. 8)                       u64 mpp_offset            (offset to MPP region)
/// [8 .. 16)                      u64 partitioning_source_idx
/// [pscan_offset .. mpp_offset)   ParallelScanState (variable size)
/// [mpp_offset .. total)          MPP region (MppDsmHeader + queues + plan_bytes)
/// ```
///
/// Workers don't carry the source manifests the leader saw, so these two `u64`s let them skip
/// past the `ParallelScanState` block and key `index_segment_ids` the same way as the leader.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CustomScanMppHeader {
    pub mpp_offset: u64,
    pub partitioning_source_idx: u64,
}

const CUSTOM_SCAN_MPP_HEADER_SIZE: usize = std::mem::size_of::<CustomScanMppHeader>();

/// Round `n` up to the nearest `MAXIMUM_ALIGNOF` boundary. Used to align section boundaries
/// inside the customscan's DSM coordinate so the `ParallelScanState` block and the MPP region
/// each start on aligned bytes.
pub fn mpp_align(n: usize) -> usize {
    let a = pg_sys::MAXIMUM_ALIGNOF as usize;
    n.next_multiple_of(a)
}

/// Byte offset of the `ParallelScanState` block within the customscan's DSM coordinate. Lives
/// right after the [`CustomScanMppHeader`], MAXALIGN-padded.
pub fn pscan_offset() -> usize {
    mpp_align(CUSTOM_SCAN_MPP_HEADER_SIZE)
}

/// Read the [`CustomScanMppHeader`] stamped by the leader at offset 0 of the DSM coordinate.
///
/// # Safety
/// `coordinate` must point at a DSM coordinate that the leader populated via
/// [`write_custom_scan_header`]. Callers in `initialize_worker_custom_scan` get this pointer
/// from PG and are responsible for confirming it's the expected layout.
pub unsafe fn read_custom_scan_header(coordinate: *const c_void) -> CustomScanMppHeader {
    unsafe { *(coordinate as *const CustomScanMppHeader) }
}

/// Stamp the [`CustomScanMppHeader`] at offset 0 of the DSM coordinate so workers can read
/// `mpp_offset` and `partitioning_source_idx` without re-deriving them from manifests.
///
/// # Safety
/// `coordinate` must point at the leader's DSM coordinate from `initialize_dsm_custom_scan`,
/// with at least `size_of::<CustomScanMppHeader>()` bytes writable. The customscan's
/// `estimate_dsm_custom_scan` is responsible for reserving the space.
pub unsafe fn write_custom_scan_header(coordinate: *mut c_void, header: CustomScanMppHeader) {
    unsafe {
        *(coordinate as *mut CustomScanMppHeader) = header;
    }
}

/// Per-edge queue size from the GUC.
pub(super) fn mpp_queue_size() -> usize {
    gucs_mpp_queue_size()
}

/// Body of `estimate_dsm_custom_scan`. Returns total DSM bytes the leader will need for the
/// plan, the multiplexed `n_procs × n_procs` queue mesh, and the worker plan. `n_procs` is the
/// total proc count (leader + `producer_worker_count()` parallel workers).
pub fn estimate_dsm_size(plan_bytes_len: usize) -> Result<usize, String> {
    let layout = compute_dsm_layout(mpp_worker_count(), mpp_queue_size(), plan_bytes_len)
        .map_err(|e| format!("mpp: estimate_dsm_size: {e}"))?;
    Ok(layout.region_total)
}

/// Number of producer workers PG should launch as `parallel_workers`.
/// `mpp_worker_count - 1` because proc 0 is the leader (consumer-only). Callers must gate
/// on [`mpp_is_active`] first; when active, [`MIN_TOTAL_WORKER_COUNT`] guarantees this is
/// `>= 2` without further clamping.
pub fn producer_worker_count() -> u32 {
    mpp_worker_count() - 1
}

/// Returned to the leader from [`leader_setup`]. The customscan stashes this on its execution
/// state and consults it during `exec_custom_scan`.
///
/// The leader is consumer-only: it gathers fragments from worker procs but doesn't host a
/// producer fragment itself. Its outbound senders are dropped inside `leader_setup`.
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

/// Wrap each peer-indexed `DsmMpscSender` into an outbound `MppSender` keyed by `target_proc`.
/// The per-fragment dispatcher driven by [`mpp::host::exec_mpp_worker`] immediately
/// `clone_with_header`s these to the right `(stage_id, partition)`, so the default placeholder
/// header is never observed on the wire. Slot at index `this_proc` is `None`; the worker's
/// self-loop install fills it in afterward.
///
/// Placeholder header is stamped with `sender_proc = this_proc` so a stray frame that escapes
/// the dispatcher's `clone_with_header` overwrite still identifies its origin correctly on the
/// drain side. Downstream `clone_with_header` callers MUST keep using `this_proc` as the
/// `sender_proc` argument.
fn build_outbound_senders(
    this_proc: u32,
    total_procs: u32,
    peer_senders: Vec<DsmMpscSender>,
) -> Vec<Option<MppSender>> {
    let mut senders: Vec<Option<MppSender>> = (0..total_procs).map(|_| None).collect();
    for (peer_idx, dsm_send) in peer_senders.into_iter().enumerate() {
        let target_proc = peer_proc_for_index(this_proc, peer_idx as u32);
        debug_assert!(target_proc != this_proc);
        debug_assert!(target_proc < total_procs);
        let shared: Arc<dyn BatchChannelSender> = Arc::new(DsmInboxSender::new(dsm_send));
        senders[target_proc as usize] = Some(MppSender::with_header(
            shared,
            MppFrameHeader::batch(NATURAL_GATHER_STAGE_ID, NATURAL_GATHER_PARTITION, this_proc),
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
    plan_bytes: Vec<u8>,
) -> Result<MppLeaderState, String> {
    let total_procs = mpp_worker_count();
    let layout = compute_dsm_layout(total_procs, mpp_queue_size(), plan_bytes.len())
        .map_err(|e| format!("mpp: leader_setup compute layout: {e}"))?;

    let attach = unsafe { leader_init(coordinate, seg, &layout, &plan_bytes) }?;

    // Wrap the leader's own-inbox in a DsmInboxReceiver and feed it to a single DrainHandle.
    // Channel buffers (keyed by (sender_proc, stage_id, partition)) are created lazily on
    // first frame, or up-front by `WorkerConnection::execute`.
    let inbox = DsmInboxReceiver::new(attach.inbound_receiver);
    // Register the leader as the receiver so producers' wakeups resolve to this process's
    // procLatch via pgprocno + pid_guard. See dsm_mpsc_ring::wake_receiver.
    unsafe {
        register_self_as_receiver(&inbox);
    }
    let mpp_recv = MppReceiver::new(Box::new(inbox));
    let inbound_drain = Arc::new(DrainHandle::cooperative(vec![mpp_recv]));

    let mesh = Arc::new(MppMesh::new(0, total_procs, inbound_drain));

    // Drop the leader's own outbound senders. The leader is consumer-only and never hosts a
    // producer fragment.
    drop(attach.outbound_senders);

    Ok(MppLeaderState { mesh, pcxt })
}

/// Register the current backend as the receiver on `inbox`'s underlying ring so producers
/// can wake it via SetLatch resolved through `pgprocno + pid_guard`. Called by both
/// leader_setup and worker_setup right after building the DsmInboxReceiver.
///
/// # Safety
/// Must run on the backend thread (reads MyProcNumber + MyProc->pid via pgrx, both
/// require an attached PGPROC). Both setup paths are called synchronously from PG's
/// custom-scan init hooks on the backend before any tokio runtime spins up.
unsafe fn register_self_as_receiver(inbox: &DsmInboxReceiver) {
    // `pg_sys::MyProcNumber` is the PG17+ global. PG15/16 carry the same value on
    // `MyProc->pgprocno` (it moved to a process-global plus a field rename in PG17).
    // Gate by feature so the build picks the right one on every supported PG.
    #[cfg(any(feature = "pg15", feature = "pg16"))]
    let my_pgprocno: i32 = unsafe { (*pg_sys::MyProc).pgprocno };
    #[cfg(not(any(feature = "pg15", feature = "pg16")))]
    let my_pgprocno: i32 = unsafe { pg_sys::MyProcNumber };
    let my_pid: i32 = unsafe { (*pg_sys::MyProc).pid };
    inbox.set_receiver(my_pgprocno, my_pid);
}

/// Returned to a worker from [`worker_setup`]. The customscan reads the plan bytes, runs the
/// plan, and pushes resulting batches through `outbound_senders`.
pub struct MppWorkerState {
    /// `outbound_senders[proc_idx]` is the sender that writes to peer `proc_idx`'s inbox.
    /// The entry at `proc_idx == this_proc` is the self-loop in-proc channel installed by
    /// `worker_setup` (since DSM MPSC inboxes have only one receiver per ring, the worker
    /// can't be both producer and consumer on the shm_mq inbox path).
    ///
    /// Each fragment's per-partition output sender is keyed by
    /// `outbound_senders[proc_for_task(n_workers, consumer_task)]`. Each `MppSender` wraps an
    /// `Arc<dyn BatchChannelSender>` so callers can `clone_with_header` to multiplex
    /// `(stage_id, partition)` channels onto one inbox.
    pub outbound_senders: Vec<Option<MppSender>>,
    /// Worker fragment plan bytes, copied out of DSM. Caller deserializes via the
    /// `PgSearchExtensionCodec` to get an `Arc<dyn ExecutionPlan>`.
    pub plan_bytes: Vec<u8>,
    /// Worker's MppMesh. The single `inbound_drain` pulls frames addressed to this proc
    /// from both the DSM MPSC inbox and the in-proc self-loop channel; demux by
    /// `(sender_proc, stage_id, partition)` happens inside the drain's channel-buffer
    /// registry. Read by the multi-fragment dispatcher driven by [`mpp::host::exec_mpp_worker`].
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
    debug_assert_eq!(
        outbound_senders.iter().filter(|s| s.is_some()).count(),
        (total_procs as usize).saturating_sub(1),
        "worker outbound senders: expected {} non-self-loop entries before self-loop install",
        total_procs.saturating_sub(1)
    );

    // Install a self-loop in-proc channel. Peer-mesh hash routing can land producer-side
    // and consumer-side tasks for the same (stage, partition) on the same worker. With
    // MPSC inboxes you can't be your own sender (only the owner attaches as receiver), so
    // self-loops bypass DSM entirely and ride an in-proc channel. The unified drain pulls
    // from BOTH receivers (own-inbox MPSC + self-loop in-proc) so the channel-buffer
    // registry sees a single demux stream. Self-loop frames carry sender_proc=this_proc
    // and route to the matching per-sender buffer.
    let (self_tx, self_rx) = in_proc_channel(SELF_LOOP_CAPACITY);
    let self_tx_arc: Arc<dyn BatchChannelSender> = Arc::new(self_tx);
    outbound_senders[proc_idx as usize] = Some(MppSender::with_header(
        self_tx_arc,
        MppFrameHeader::batch(NATURAL_GATHER_STAGE_ID, NATURAL_GATHER_PARTITION, proc_idx),
    ));

    // Build the unified drain: own-inbox MPSC + self-loop in-proc, both feeding the same
    // DrainHandle. Register the worker as receiver before the drain starts polling so
    // any producer that races ahead of us sees a valid pgprocno + pid.
    let inbox = DsmInboxReceiver::new(attach.inbound_receiver);
    unsafe {
        register_self_as_receiver(&inbox);
    }
    let inbox_recv = MppReceiver::new(Box::new(inbox));
    let self_recv = MppReceiver::new(Box::new(self_rx));
    let inbound_drain = Arc::new(DrainHandle::cooperative(vec![inbox_recv, self_recv]));

    let mesh = Arc::new(MppMesh::new(proc_idx, total_procs, inbound_drain));

    Ok(MppWorkerState {
        outbound_senders,
        plan_bytes,
        mesh,
    })
}
