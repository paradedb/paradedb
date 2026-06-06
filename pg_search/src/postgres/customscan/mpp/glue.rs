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
//! leader/worker MPP architecture.
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

use datafusion_distributed::embedded::{self, Interrupt, MppMesh, MppSender, Wakeup};

use crate::gucs::{
    enable_mpp, mpp_queue_size as gucs_mpp_queue_size, mpp_worker_count as gucs_mpp_worker_count,
};
use crate::postgres::customscan::mpp::pg_seams::{pack_receiver, PgInterrupt, PgWakeup};

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
    /// Byte offset of the MPP region within the coordinate. Always a real, initialized region:
    /// the leader errors out of `initialize_dsm_custom_scan` on any setup failure, before
    /// `LaunchParallelWorkers`, so no worker ever reads a half-written header.
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

/// Body of `estimate_dsm_custom_scan`. Returns the total DSM bytes the leader will need
/// for the header, the worker plan, and one MPSC inbox per process. `n_procs` is the
/// total proc count (leader + `producer_worker_count()` parallel workers).
pub fn estimate_dsm_size(plan_bytes_len: usize) -> Result<usize, String> {
    embedded::dsm_region_bytes(mpp_worker_count(), mpp_queue_size(), plan_bytes_len)
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

/// The `(pgprocno, pid)` of this backend, packed into the receiver token the transport stores so a
/// producer's [`PgWakeup`] can `SetLatch` us. Read on the backend thread (both setup paths run
/// synchronously from PG's custom-scan init hooks before any tokio runtime spins up).
unsafe fn self_receiver_token() -> u64 {
    // `pg_sys::MyProcNumber` is the PG17+ global; PG15/16 carry the same value on
    // `MyProc->pgprocno` (it moved to a process-global plus a field rename in PG17).
    #[cfg(any(feature = "pg15", feature = "pg16"))]
    let my_pgprocno: i32 = unsafe { (*pg_sys::MyProc).pgprocno };
    #[cfg(not(any(feature = "pg15", feature = "pg16")))]
    let my_pgprocno: i32 = unsafe { pg_sys::MyProcNumber };
    let my_pid: i32 = unsafe { (*pg_sys::MyProc).pid };
    pack_receiver(my_pgprocno, my_pid)
}

/// Body of `initialize_dsm_custom_scan`. Allocates the queue mesh, populates
/// the [`MppMesh`] handle, and serializes the worker plan into DSM.
///
/// # Safety
/// - `coordinate` must be the DSM region pointer PG supplied to
///   `initialize_dsm_custom_scan`.
/// - `plan_bytes` must have the same length passed to [`estimate_dsm_size`]
///   so the leader doesn't overrun the DSM region PG allocated.
pub unsafe fn leader_setup(
    coordinate: *mut c_void,
    pcxt: *mut pg_sys::ParallelContext,
    plan_bytes: Vec<u8>,
) -> Result<MppLeaderState, String> {
    let wakeup: Arc<dyn Wakeup> = Arc::new(PgWakeup);
    let interrupt: Arc<dyn Interrupt> = Arc::new(PgInterrupt);
    // Register the leader as receiver so producers' wakeups resolve to this backend's procLatch.
    let token = unsafe { self_receiver_token() };
    let mesh = unsafe {
        embedded::leader_setup(
            coordinate,
            mpp_worker_count(),
            mpp_queue_size(),
            &plan_bytes,
            wakeup,
            token,
            interrupt,
        )
    }?;
    Ok(MppLeaderState { mesh, pcxt })
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
    /// Leader's dispatch payload (framed per-stage physical subplans), copied out of DSM. The
    /// worker decodes it into its fragment assignments via `mpp::dispatch::expand_to_assignments`.
    pub plan_bytes: Vec<u8>,
    /// Worker's MppMesh. The single `inbound_receiver` pulls frames addressed to this
    /// proc from both the DSM MPSC inbox and the in-proc self-loop channel; demux by
    /// `(sender_proc, stage_id, partition)` happens inside the handle's channel-buffer
    /// registry. Read by the multi-fragment dispatcher driven by [`mpp::host::exec_mpp_worker`].
    pub mesh: Arc<MppMesh>,
}

/// Body of `initialize_worker_custom_scan`. Reads the header, attaches as
/// sender on this worker's slot row, copies the plan bytes out of DSM.
///
/// # Safety
/// - `coordinate` must be the DSM region pointer PG supplied.
/// - `region_total` must match the DSM's attached size.
pub unsafe fn worker_setup(
    coordinate: *mut c_void,
    region_total: u64,
    worker_number: i32,
) -> Result<MppWorkerState, String> {
    if worker_number < 0 {
        return Err("mpp: worker_number < 0".into());
    }
    // Leader is `proc_idx = 0`, workers are `1..n_procs`. Worker N maps from PG's
    // `ParallelWorkerNumber = N` to `proc_idx = N + 1`.
    let proc_idx = (worker_number as u32) + 1;

    let wakeup: Arc<dyn Wakeup> = Arc::new(PgWakeup);
    let interrupt: Arc<dyn Interrupt> = Arc::new(PgInterrupt);
    // Register before the transport starts polling, so a producer racing ahead sees a valid token.
    let token = unsafe { self_receiver_token() };
    let attach = unsafe {
        embedded::worker_setup(coordinate, region_total, proc_idx, wakeup, token, interrupt)
    }?;

    Ok(MppWorkerState {
        outbound_senders: attach.outbound_senders,
        plan_bytes: attach.plan_bytes,
        mesh: attach.mesh,
    })
}
