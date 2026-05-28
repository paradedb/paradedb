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

//! Mesh-multiplexed DSM layout: one MPSC inbox per receiver process.
//!
//! Each MPP query allocates a single DSM region containing:
//!
//! ```text
//!   +--- MppDsmHeader (repr C, MAXALIGN-padded) -------------+
//!   |    magic, version, n_procs, queue_bytes               |
//!   |    plan_offset, plan_len, queues_offset, region_total |
//!   +-------------------------------------------------------+
//!   | plan bytes (bincode-serialized worker fragment)       |
//!   +-------------------------------------------------------+
//!   | padding to MAXALIGN                                   |
//!   +-------------------------------------------------------+
//!   | DsmMpscRing inbox array: n_procs inboxes              |
//!   |   inbox(receiver) = queues_offset                     |
//!   |     + receiver * queue_bytes                          |
//!   +-------------------------------------------------------+
//! ```
//!
//! - `n_procs` is the total proc count (1 leader + N parallel workers).
//!   Leader is `proc_idx = 0`; workers are `proc_idx = ParallelWorkerNumber + 1`.
//! - Each process attaches as **receiver** to its own inbox (one MPSC ring) and as
//!   **sender** to each of N-1 peer inboxes. Total queues per mesh is `n_procs`; total
//!   attach calls per proc is `1 + (N-1) = N`. Senders stamp
//!   `MppFrameHeader::sender_proc` on every frame so the receiver demultiplexes by
//!   source on read.
//! - Self-loop frames (proc → itself) use an in-proc channel installed in `glue.rs`,
//!   not a DSM slot. MPSC ring semantics + the way `DsmMpscReceiver` is used (one
//!   handle owned by the receiver process) means there is no DSM slot for the
//!   self-loop pair in this layout.

use std::ffi::c_void;
use std::mem::size_of;
use std::time::Instant;

use pgrx::pg_sys;

use crate::postgres::customscan::mpp::dsm_mpsc_ring::{
    self, DsmMpscReceiver, DsmMpscRingHeader, DsmMpscSender,
};
use crate::postgres::customscan::mpp::mesh::{align_up_maxalign_checked, aligned_queue_bytes};

/// Number of slots in each per-receiver MPSC ring. With operator-visible
/// `paradedb.mpp_queue_size` divided across `RING_SLOTS` slots, each slot holds
/// up to `(queue_bytes / RING_SLOTS) - SLOT_HEADER` bytes of payload — enough for
/// typical Arrow batches at the bench scales we measured. Fixed at compile time
/// for now; a future GUC could expose it if bench data points at a different
/// sweet spot.
const RING_SLOTS: u32 = 8;

/// Cache-line alignment for the per-inbox DsmMpscRing header. The ring's
/// `#[repr(C, align(64))]` mandates 64-byte alignment at every `create_at`/`attach_at`
/// site; both `queues_offset` and per-inbox `queue_bytes` are aligned up to this so the
/// computed `inbox_offset(r) = queues_offset + r * queue_bytes` lands at a 64-aligned
/// address for every `r`.
const RING_ALIGN: usize = 64;

#[inline]
fn align_up_ring(n: usize) -> Option<usize> {
    let mask = RING_ALIGN - 1;
    n.checked_add(mask).map(|x| x & !mask)
}

const MPP_DSM_MAGIC: u32 = 0x4D50_5052; // "MPPR" (RPC variant)
/// Bumped on any wire-incompatible change to the DSM header layout or to the inbox-offset math,
/// so attaching workers reject mismatched leaders loudly rather than reading garbage. Validated
/// in [`MppDsmHeader::validate`]. v4: mesh-multiplexed layout (one MPSC ring per receiver, was
/// per-pair shm_mq grid in v3).
const MPP_DSM_HEADER_VERSION: u32 = 4;

/// Absolute cap on DSM region size. 16 GiB is two orders of magnitude beyond
/// any realistic workload; the cap fails early on a pathologically oversized
/// request rather than asking PG for ~`usize::MAX` bytes.
const MPP_DSM_MAX_BYTES: usize = 16 * 1024 * 1024 * 1024;

/// C-repr header at offset 0 of the DSM region.
///
/// Layout: three `u32`s + padding, then six `u64`s.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MppDsmHeader {
    pub(super) magic: u32,
    pub(super) header_version: u32,
    /// Total proc count. Leader is `proc_idx = 0`; workers are
    /// `proc_idx = ParallelWorkerNumber + 1`. The shm_mq grid is `n_procs × n_procs`.
    pub n_procs: u32,
    pub(super) _pad: u32,
    pub(super) queue_bytes: u64,
    pub(super) plan_offset: u64,
    pub(super) plan_len: u64,
    pub(super) queues_offset: u64,
    pub region_total: u64,
}

impl MppDsmHeader {
    fn from_layout(layout: &DsmLayout) -> Self {
        Self {
            magic: MPP_DSM_MAGIC,
            header_version: MPP_DSM_HEADER_VERSION,
            n_procs: layout.n_procs,
            _pad: 0,
            queue_bytes: layout.queue_bytes as u64,
            plan_offset: layout.plan_offset as u64,
            plan_len: layout.plan_len as u64,
            queues_offset: layout.queues_offset as u64,
            region_total: layout.region_total as u64,
        }
    }

    pub(super) fn validate(&self, region_total: u64) -> Result<(), &'static str> {
        if self.magic != MPP_DSM_MAGIC {
            return Err("mpp: DSM header magic mismatch");
        }
        if self.header_version != MPP_DSM_HEADER_VERSION {
            return Err("mpp: DSM header version mismatch");
        }
        if self.n_procs == 0 {
            return Err("mpp: header n_procs must be > 0");
        }
        if self.region_total != region_total {
            return Err("mpp: DSM region_total in header disagrees with attached size");
        }
        match self.plan_offset.checked_add(self.plan_len) {
            None => return Err("mpp: plan_offset + plan_len overflow"),
            Some(end) if end > self.queues_offset => {
                return Err("mpp: plan would overlap queues area")
            }
            _ => {}
        }
        if self.queues_offset > region_total {
            return Err("mpp: queues_offset past end of region");
        }
        Ok(())
    }

    /// Byte offset (relative to DSM base) of `receiver_proc`'s MPSC inbox.
    ///
    /// Mesh-multiplexed layout: exactly one `DsmMpscRing`-shaped inbox per process. The
    /// owner attaches as receiver (`DsmMpscReceiver`); every other process attaches as
    /// sender (`DsmMpscSender`) to the same inbox and stamps its identity into the
    /// frame header (`MppFrameHeader::sender_proc`). Total queues per mesh is
    /// `n_procs`, down from the pre-Phase-4b `n_procs²`.
    pub(super) fn inbox_offset(&self, receiver_proc: u32) -> u64 {
        debug_assert!(receiver_proc < self.n_procs);
        self.queues_offset + (receiver_proc as u64) * self.queue_bytes
    }
}

/// Pure-math layout for [`compute_dsm_layout`].
#[derive(Debug, Clone, Copy)]
pub(super) struct DsmLayout {
    pub n_procs: u32,
    pub queue_bytes: usize,
    pub plan_offset: usize,
    pub plan_len: usize,
    pub queues_offset: usize,
    pub region_total: usize,
}

/// Compute the DSM region size and field offsets for one MPP query.
///
/// `n_procs` is the total proc count (1 leader + N workers). Mesh-multiplexed layout:
/// one `DsmMpscRing` inbox per process (`n_procs` queues total per mesh). Every other
/// process attaches as sender to that inbox and demultiplexes by
/// `MppFrameHeader::sender_proc` on the receive side.
///
/// `queue_bytes` is the per-inbox total (ring header + `RING_SLOTS` slots). Operator-
/// facing `paradedb.mpp_queue_size` controls this value.
pub(super) fn compute_dsm_layout(
    n_procs: u32,
    queue_bytes: usize,
    plan_len: usize,
) -> Result<DsmLayout, &'static str> {
    if n_procs < 2 {
        return Err("mpp: n_procs must be >= 2 (leader + at least one worker)");
    }
    // MAXALIGN-round-down first (operator-friendly), then round up to the ring's
    // 64-byte alignment requirement. Doing it in this order means each per-inbox
    // region is both MAXALIGN-aligned (PG DSM convention) AND cache-line aligned
    // (DsmMpscRingHeader's repr(C, align(64)) requirement).
    let queue_bytes = aligned_queue_bytes(queue_bytes);
    if queue_bytes == 0 {
        return Err("mpp: queue_bytes too small after alignment");
    }
    let queue_bytes = align_up_ring(queue_bytes).ok_or("mpp: queue_bytes alignment overflow")?;
    // Sanity: each inbox must have room for the ring header + at least RING_SLOTS slots
    // of one byte each (we don't pin a minimum payload size, just that the ring is
    // constructible).
    if queue_bytes < DsmMpscRingHeader::region_bytes(RING_SLOTS, 1) {
        return Err("mpp: queue_bytes too small for ring header + min slots");
    }
    let header_end = align_up_maxalign_checked(size_of::<MppDsmHeader>())
        .ok_or("mpp: header alignment overflow")?;
    let plan_offset = header_end;
    let plan_end = plan_offset
        .checked_add(plan_len)
        .ok_or("mpp: plan offset+len overflow")?;
    // Round queues_offset up to RING_ALIGN so the first inbox starts at a 64-aligned
    // address; subsequent inboxes are queue_bytes apart and queue_bytes is RING_ALIGN-
    // aligned, so they all land on cache-line boundaries.
    let queues_offset = align_up_ring(plan_end).ok_or("mpp: queues alignment overflow")?;
    let queues_bytes = (n_procs as usize)
        .checked_mul(queue_bytes)
        .ok_or("mpp: queues bytes overflow")?;
    let region_total = queues_offset
        .checked_add(queues_bytes)
        .ok_or("mpp: region total overflow")?;
    if region_total > MPP_DSM_MAX_BYTES {
        return Err("mpp: DSM region exceeds MPP_DSM_MAX_BYTES");
    }
    Ok(DsmLayout {
        n_procs,
        queue_bytes,
        plan_offset,
        plan_len,
        queues_offset,
        region_total,
    })
}

/// Derive `(ring_slots, slot_capacity)` for a per-inbox region of `queue_bytes`.
/// Total ring region size = `DsmMpscRingHeader::region_bytes(ring_slots, slot_capacity)`
/// which fits within `queue_bytes` by construction.
fn ring_dims_for(queue_bytes: usize) -> (u32, u32) {
    let header = std::mem::size_of::<DsmMpscRingHeader>();
    // queue_bytes >= ring_dims minimum is enforced in compute_dsm_layout; we recompute
    // here without re-validating.
    let slot_total_bytes = queue_bytes.saturating_sub(header);
    let slot_capacity = (slot_total_bytes / RING_SLOTS as usize).max(64);
    // Cap slot_capacity at u32::MAX (DsmMpscRing's field type) to avoid casting wrap.
    let slot_capacity = slot_capacity.min(u32::MAX as usize) as u32;
    (RING_SLOTS, slot_capacity)
}

/// Per-proc return from `attach_proc` under the mesh-multiplexed layout: N-1 outbound
/// senders (one per peer inbox) + a single inbound receiver (this proc's own inbox).
///
/// The own-inbox is the multiplexed entry point: all N-1 peers attach to it as senders
/// (each `DsmMpscSender` increments the ring's `sender_count`) and stamp their identity
/// into `MppFrameHeader::sender_proc` on every frame. The receiver side pulls frames
/// off that single ring and routes them to per-`(sender_proc, stage_id, partition)`
/// channel buffers via [`DrainHandle`].
pub(super) struct ProcAttach {
    /// `outbound_senders[i]` writes to peer `peer_proc_for_index(this_proc, i)`'s inbox.
    /// `peer_proc(i) = i if i < this_proc else i + 1` skips the self-loop entry.
    pub(super) outbound_senders: Vec<DsmMpscSender>,
    /// This process's own MPSC inbox receiver. Drained inline by `DrainHandle`.
    pub(super) inbound_receiver: DsmMpscReceiver,
}

/// Translate a peer index (`0..n_procs - 1`) into a process index
/// (`0..n_procs`) by skipping the self-loop slot.
#[inline]
pub(super) fn peer_proc_for_index(this_proc: u32, peer_idx: u32) -> u32 {
    if peer_idx < this_proc {
        peer_idx
    } else {
        peer_idx + 1
    }
}

/// Initialize the DSM region as the leader (`proc_idx = 0`). Writes the MppDsmHeader,
/// copies the plan bytes, initializes the N MPSC inboxes via `DsmMpscRing::create_at`,
/// and attaches the leader as receiver to its own inbox + as sender to each peer.
///
/// # Safety
/// - `coordinate` must point to the start of a DSM region of size `>= layout.region_total`.
/// - `seg` must be the leader's `dsm_segment*`.
/// - The region must be uninitialized (the leader is the first writer).
pub(super) unsafe fn leader_init(
    coordinate: *mut c_void,
    seg: *mut pg_sys::dsm_segment,
    layout: &DsmLayout,
    plan_bytes: &[u8],
) -> Result<ProcAttach, String> {
    if coordinate.is_null() {
        return Err("mpp: leader_init given null coordinate".into());
    }
    if plan_bytes.len() != layout.plan_len {
        return Err(format!(
            "mpp: plan_bytes.len()={} != layout.plan_len={}",
            plan_bytes.len(),
            layout.plan_len
        ));
    }

    let base = coordinate as *mut u8;

    // Header.
    unsafe {
        std::ptr::write(
            base.cast::<MppDsmHeader>(),
            MppDsmHeader::from_layout(layout),
        );
    }
    // Plan bytes.
    unsafe {
        std::ptr::copy_nonoverlapping(
            plan_bytes.as_ptr(),
            base.add(layout.plan_offset),
            plan_bytes.len(),
        );
    }

    // Initialize the N MPSC inboxes. Workers can't do this (the region is uninitialized
    // at their attach time), so the leader runs DsmMpscRing::create_at for every receiver.
    let header = MppDsmHeader::from_layout(layout);
    let n_procs = layout.n_procs;
    let (ring_slots, slot_capacity) = ring_dims_for(layout.queue_bytes);
    // `crate::gucs::mpp_trace()` reads a pgrx GucSetting, which requires the backend thread.
    // Safe here because `leader_init` runs synchronously from `initialize_dsm_custom_scan`
    // on the backend before any tokio runtime spins up — same property the surrounding
    // `pgrx::warning!` callers rely on.
    let trace_on = crate::gucs::mpp_trace();
    let t_create = trace_on.then(Instant::now);
    for r in 0..n_procs {
        let off = header.inbox_offset(r) as usize;
        let inbox_addr = unsafe { base.add(off) };
        unsafe { dsm_mpsc_ring::create_at(inbox_addr, ring_slots, slot_capacity) };
    }
    let create_ms = t_create.map(|t| t.elapsed().as_secs_f64() * 1000.0);

    let t_attach = trace_on.then(Instant::now);
    let attach = unsafe {
        attach_proc(base, &header, 0, /* attach_senders */ false)
    };
    let attach_ms = t_attach.map(|t| t.elapsed().as_secs_f64() * 1000.0);

    if trace_on {
        // attach_proc makes N attach calls per proc: 1 own-inbox receiver + N-1 peer-inbox
        // senders. create touched all N inboxes (each is a DsmMpscRing).
        let attach_calls = n_procs as usize;
        pgrx::warning!(
            "mpp_trace mesh_init role=leader procs={} inboxes_created={} attach_calls={} queue_bytes={} ring_slots={} slot_capacity={} plan_bytes={} create_ms={:.2} attach_ms={:.2}",
            n_procs,
            n_procs,
            attach_calls,
            layout.queue_bytes,
            ring_slots,
            slot_capacity,
            plan_bytes.len(),
            create_ms.unwrap(),
            attach_ms.unwrap(),
        );
    }

    // Silence unused-warning when `seg` was only consumed by the shm_mq path. The new
    // primitive doesn't take a dsm_segment handle (drop-on-exit via Drop instead of PG's
    // on_dsm_detach callback); the parameter stays for future use.
    let _ = seg;

    Ok(attach)
}

/// Attach to the leader-initialized DSM region as `proc_idx` (`0 = leader`, `1..N = parallel
/// workers`) under the mesh-multiplexed layout: attach as receiver to this proc's own MPSC
/// inbox, and (if `attach_senders` is true) as sender to every peer's inbox.
///
/// Workers use this from `initialize_worker_custom_scan` via `worker_attach` with
/// `attach_senders = true`; the leader uses it from `leader_init` with
/// `attach_senders = false` because the leader is consumer-only and never hosts a
/// producer fragment.
///
/// **Why the leader skips outbound attach**: `DsmMpscSender::new` increments the ring's
/// `sender_count`, and `Drop` decrements it; the 1 → 0 transition flips `detached`. If
/// the leader attached as sender to each peer inbox before any worker had, the leader's
/// subsequent drop would prematurely mark every inbox detached and workers' later sends
/// would all fail with `SendError::Detached`. Skipping the attach entirely keeps
/// `sender_count` accurate (only workers ever increment for peer inboxes).
///
/// # Safety
/// - `base` must point to a DSM region whose header has been validated.
/// - `header.inbox_offset(r)` must already point at a ring initialized by
///   `DsmMpscRing::create_at` (the leader does this in `leader_init`).
unsafe fn attach_proc(
    base: *mut u8,
    header: &MppDsmHeader,
    this_proc: u32,
    attach_senders: bool,
) -> ProcAttach {
    let n_procs = header.n_procs;
    let peer_count = (n_procs - 1) as usize;
    let mut outbound_senders = Vec::with_capacity(if attach_senders { peer_count } else { 0 });

    let (ring_slots, slot_capacity) = ring_dims_for(header.queue_bytes as usize);

    if attach_senders {
        for peer_idx in 0..(n_procs - 1) {
            let r = peer_proc_for_index(this_proc, peer_idx);
            let off = header.inbox_offset(r) as usize;
            let inbox_addr = unsafe { base.add(off) };
            let nn = unsafe { dsm_mpsc_ring::attach_at(inbox_addr, ring_slots, slot_capacity) }
                .expect("DsmMpscRing attach_at: leader-initialized region must validate");
            outbound_senders.push(unsafe { DsmMpscSender::new(nn) });
        }
    }

    // Inbound: this proc's own inbox. Single receiver per ring (MPSC contract).
    let own_off = header.inbox_offset(this_proc) as usize;
    let own_inbox_addr = unsafe { base.add(own_off) };
    let own_nn = unsafe { dsm_mpsc_ring::attach_at(own_inbox_addr, ring_slots, slot_capacity) }
        .expect("DsmMpscRing attach_at: own inbox must validate");
    let inbound_receiver = unsafe { DsmMpscReceiver::new(own_nn) };

    ProcAttach {
        outbound_senders,
        inbound_receiver,
    }
}

/// Attach to the leader-initialized DSM region as `proc_idx` (1-based for
/// workers: PG's `ParallelWorkerNumber + 1`).
///
/// # Safety
/// - `coordinate` must be the DSM region pointer the leader initialized.
/// - `region_total` must match the DSM's attached size.
/// - `seg` may be NULL. `initialize_worker_custom_scan` doesn't surface the segment pointer, and
///   `shm_mq_attach` handles NULL by skipping its on-detach callback (cleanup falls back to
///   process exit, safe for parallel-worker lifetimes).
pub(super) unsafe fn worker_attach(
    coordinate: *mut c_void,
    region_total: u64,
    proc_idx: u32,
    seg: *mut pg_sys::dsm_segment,
) -> Result<(MppDsmHeader, Vec<u8>, ProcAttach), String> {
    if coordinate.is_null() {
        return Err("mpp: worker_attach given null coordinate".into());
    }
    let base = coordinate as *mut u8;
    let header = unsafe { std::ptr::read(base.cast::<MppDsmHeader>()) };
    header
        .validate(region_total)
        .map_err(|e| format!("mpp: worker DSM validate: {e}"))?;
    if proc_idx == 0 {
        return Err(
            "mpp: worker_attach must be called with proc_idx >= 1 (proc 0 is leader)".into(),
        );
    }
    if proc_idx >= header.n_procs {
        return Err(format!(
            "mpp: proc_idx {proc_idx} not in 1..{}",
            header.n_procs
        ));
    }

    // Copy plan bytes out of DSM so the caller has an owned buffer.
    let plan_bytes = unsafe {
        std::slice::from_raw_parts(
            base.add(header.plan_offset as usize),
            header.plan_len as usize,
        )
        .to_vec()
    };

    // Same backend-thread safety story as leader_init: `initialize_worker_custom_scan` runs on
    // the parallel-worker backend before tokio starts, so reading `mpp_trace` directly is safe.
    let trace_on = crate::gucs::mpp_trace();
    let t_attach = trace_on.then(Instant::now);
    let attach = unsafe {
        attach_proc(base, &header, proc_idx, /* attach_senders */ true)
    };
    let _ = seg; // DsmMpscRing doesn't use the dsm_segment handle today.
    if trace_on {
        let attach_ms = t_attach.unwrap().elapsed().as_secs_f64() * 1000.0;
        // N attach calls per proc: 1 own-inbox receiver + N-1 peer-inbox senders.
        let attach_calls = header.n_procs as usize;
        pgrx::warning!(
            "mpp_trace mesh_init role=worker proc_idx={} procs={} attach_calls={} attach_ms={:.2}",
            proc_idx,
            header.n_procs,
            attach_calls,
            attach_ms,
        );
    }
    Ok((header, plan_bytes, attach))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_dsm_layout_works() {
        let l = compute_dsm_layout(4, 64 * 1024, 1024).unwrap();
        assert_eq!(l.n_procs, 4);
        // Mesh-multiplexed: one inbox per receiver, not N². 4 procs → 4 inboxes.
        let aligned = aligned_queue_bytes(64 * 1024);
        let queues_size = 4 * aligned;
        assert_eq!(l.region_total, l.queues_offset + queues_size);
    }

    #[test]
    fn compute_dsm_layout_rejects_zero_procs() {
        assert!(compute_dsm_layout(0, 64 * 1024, 0).is_err());
    }

    #[test]
    fn compute_dsm_layout_rejects_oversize() {
        assert!(compute_dsm_layout(u32::MAX, 64 * 1024, 0).is_err());
    }

    #[test]
    fn compute_dsm_layout_scales_linearly_in_n_procs() {
        // Total queue area must grow as O(N), not O(N²). Pinning the math here so a
        // regression that re-introduces the per-pair grid trips at compile-time of
        // this test, not at the N=24 cliff in production.
        let queue_bytes = 64 * 1024;
        let aligned = aligned_queue_bytes(queue_bytes);
        for n in [2u32, 4, 8, 16, 24] {
            let l = compute_dsm_layout(n, queue_bytes, 0).unwrap();
            let expected = (n as usize) * aligned;
            assert_eq!(
                l.region_total - l.queues_offset,
                expected,
                "n={n}: expected {expected} inbox bytes ({n} inboxes × {aligned})"
            );
        }
    }

    #[test]
    fn header_inbox_offset_is_per_receiver() {
        // 4 procs → 4 inboxes, contiguous, sized by queue_bytes each.
        let l = compute_dsm_layout(4, 64 * 1024, 0).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        let aligned = h.queue_bytes;
        assert_eq!(h.inbox_offset(0), h.queues_offset);
        assert_eq!(h.inbox_offset(1), h.queues_offset + aligned);
        assert_eq!(h.inbox_offset(2), h.queues_offset + 2 * aligned);
        assert_eq!(h.inbox_offset(3), h.queues_offset + 3 * aligned);
    }

    #[test]
    fn header_validate_accepts_self() {
        let l = compute_dsm_layout(2, 64 * 1024, 0).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        assert!(h.validate(l.region_total as u64).is_ok());
    }

    #[test]
    fn header_validate_rejects_wrong_version() {
        let l = compute_dsm_layout(2, 64 * 1024, 0).unwrap();
        let mut h = MppDsmHeader::from_layout(&l);
        h.header_version = MPP_DSM_HEADER_VERSION.wrapping_sub(1);
        let err = h
            .validate(l.region_total as u64)
            .expect_err("wrong version must fail");
        assert!(err.contains("DSM header version mismatch"), "got: {err}");
    }

    #[test]
    fn header_validate_rejects_size_mismatch() {
        let l = compute_dsm_layout(2, 64 * 1024, 0).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        assert!(h.validate(l.region_total as u64 + 1).is_err());
    }
}
