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

//! Mesh-multiplexed DSM layout: one shm_mq inbox per receiver process.
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
//!   | shm_mq inbox array: n_procs slots                     |
//!   |   inbox(receiver) = queues_offset                     |
//!   |     + receiver * queue_bytes                          |
//!   +-------------------------------------------------------+
//! ```
//!
//! - `n_procs` is the total proc count (1 leader + N parallel workers). Leader is
//!   `proc_idx = 0`; workers are `proc_idx = ParallelWorkerNumber + 1`.
//! - Each process attaches as **receiver** to its own inbox (one queue) and as **sender** to
//!   each of N-1 peer inboxes. Total queues per mesh is `n_procs`; total per-proc attach
//!   calls is `1 + (N-1) = N`. Senders stamp `MppFrameHeader::sender_proc` on every frame so
//!   the receiver demultiplexes by source on read.
//! - Self-loop frames (proc → itself) use an in-proc channel installed in `glue.rs`, not a
//!   DSM slot. shm_mq enforces exactly one receiver per queue, so there is no DSM slot for
//!   the self-loop pair in this layout.

use std::ffi::c_void;
use std::mem::size_of;
use std::time::Instant;

use pgrx::pg_sys;

use crate::postgres::customscan::mpp::mesh::{
    align_up_maxalign_checked, aligned_queue_bytes, ShmMqReceiver, ShmMqSender,
};

const MPP_DSM_MAGIC: u32 = 0x4D50_5052; // "MPPR" (RPC variant)
/// Bumped on any wire-incompatible change to the DSM header layout or to the slot-offset math,
/// so attaching workers reject mismatched leaders loudly rather than reading garbage. Validated
/// in [`MppDsmHeader::validate`].
const MPP_DSM_HEADER_VERSION: u32 = 3;

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

    /// Byte offset (relative to DSM base) of `receiver_proc`'s inbox queue.
    ///
    /// Mesh-multiplexed layout: there is exactly one shm_mq inbox per process. The receiver
    /// attaches as receiver; every other process attaches as sender to the same inbox and stamps
    /// its identity into the frame header (`MppFrameHeader::sender_proc`). Total queues per mesh
    /// is `n_procs`, down from the pre-Phase-2 `n_procs²`.
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
/// `n_procs` is the total proc count (1 leader + N workers). Mesh-multiplexed layout: one
/// shm_mq inbox per process (`n_procs` queues total per mesh). Every other process attaches as
/// sender to that inbox and demultiplexes by `MppFrameHeader::sender_proc` on the receive side.
pub(super) fn compute_dsm_layout(
    n_procs: u32,
    queue_bytes: usize,
    plan_len: usize,
) -> Result<DsmLayout, &'static str> {
    if n_procs < 2 {
        return Err("mpp: n_procs must be >= 2 (leader + at least one worker)");
    }
    let queue_bytes = aligned_queue_bytes(queue_bytes);
    if queue_bytes == 0 {
        return Err("mpp: queue_bytes too small after alignment");
    }
    let header_end = align_up_maxalign_checked(size_of::<MppDsmHeader>())
        .ok_or("mpp: header alignment overflow")?;
    let plan_offset = header_end;
    let plan_end = plan_offset
        .checked_add(plan_len)
        .ok_or("mpp: plan offset+len overflow")?;
    let queues_offset =
        align_up_maxalign_checked(plan_end).ok_or("mpp: queues alignment overflow")?;
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

/// Per-proc return from `attach_proc` under the mesh-multiplexed layout: N-1 outbound senders
/// (one per peer) plus a single inbound receiver (the process's own inbox).
///
/// The own-inbox is the multiplexed entry point: all N-1 peers attach to it as senders and
/// stamp their identity into `MppFrameHeader::sender_proc` on every frame. The receiver side
/// pulls frames off that single queue and routes them to per-`(stage_id, partition)` buffers
/// via [`DrainHandle`].
pub(super) struct ProcAttach {
    /// `outbound_senders[i]` writes to peer `peer_proc_for_index(this_proc, i)`'s inbox.
    /// `peer_proc(i) = i if i < this_proc else i + 1` skips the self-loop entry.
    pub(super) outbound_senders: Vec<ShmMqSender>,
    /// This process's own inbox. Receives frames from every peer over a single shm_mq queue.
    pub(super) inbound_receiver: ShmMqReceiver,
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

/// Initialize the DSM region as the leader (`proc_idx = 0`). Writes the header, copies the plan
/// bytes, calls `shm_mq_create` on every queue slot, and attaches the leader's row + column
/// handles.
///
/// In the multiplexed `n_procs × n_procs` grid, every process (leader included) is a full
/// proc: sender for its row, receiver for its column. The leader is responsible for the
/// one-time `shm_mq_create` on every queue (workers can't, since the region is uninitialized at
/// their attach time), then does its own `set_sender` / `set_receiver` calls on its row and column
/// slots.
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

    // Mesh-multiplexed layout: one shm_mq inbox per receiver process. Workers can't run
    // shm_mq_create themselves (region is uninitialized at their attach time), so the leader
    // does it for all `n_procs` inboxes. Each inbox is later attached as receiver by exactly one
    // process (the owner) and as sender by N-1 peers; frames carry the sender's identity in
    // MppFrameHeader.sender_proc so the receiver can demultiplex by source.
    let header = MppDsmHeader::from_layout(layout);
    let n_procs = layout.n_procs;
    // `crate::gucs::mpp_trace()` reads a pgrx GucSetting, which requires the backend thread.
    // Safe here because `leader_init` runs synchronously from `initialize_dsm_custom_scan`
    // on the backend before any tokio runtime spins up — same property the surrounding
    // `pgrx::warning!` callers rely on.
    let trace_on = crate::gucs::mpp_trace();
    let t_create = trace_on.then(Instant::now);
    for r in 0..n_procs {
        let off = header.inbox_offset(r) as usize;
        let mq_addr = unsafe { base.add(off) };
        unsafe { pg_sys::shm_mq_create(mq_addr.cast(), layout.queue_bytes) };
    }
    let create_ms = t_create.map(|t| t.elapsed().as_secs_f64() * 1000.0);

    let t_attach = trace_on.then(Instant::now);
    let attach = unsafe { attach_proc(base, &header, 0, seg) };
    let attach_ms = t_attach.map(|t| t.elapsed().as_secs_f64() * 1000.0);

    if trace_on {
        // create loop ran n_procs shm_mq_create calls (one per inbox). attach made N-1 sender
        // attaches (one per peer) + 1 own-inbox receiver attach = N total.
        let attach_calls = n_procs as usize;
        pgrx::warning!(
            "mpp_trace mesh_init role=leader procs={} inboxes_created={} attach_calls={} queue_bytes={} plan_bytes={} create_ms={:.2} attach_ms={:.2}",
            n_procs,
            n_procs,
            attach_calls,
            layout.queue_bytes,
            plan_bytes.len(),
            create_ms.unwrap(),
            attach_ms.unwrap(),
        );
    }

    Ok(attach)
}

/// Attach to the leader-initialized DSM region as `proc_idx` (`0 = leader`, `1..N = parallel
/// workers`) under the mesh-multiplexed layout: attach as receiver to this proc's own inbox,
/// and as sender to every peer's inbox.
///
/// Workers use this from `initialize_worker_custom_scan` via `worker_attach`; the leader uses
/// it inline at the end of `leader_init`. shm_mq allows only one receiver per queue, so the
/// own-inbox attach is the single point of frame ingress for this process — peers stamp
/// `MppFrameHeader::sender_proc` so the drain side can demultiplex.
///
/// # Safety
/// - `base` must point to a DSM region whose header has been validated.
/// - `header.inbox_offset(r)` must already point at a slot initialized by `shm_mq_create` (the
///   leader does this in `leader_init`).
/// - `seg` may be NULL on workers. `shm_mq_attach` skips its on-detach callback in that case.
unsafe fn attach_proc(
    base: *mut u8,
    header: &MppDsmHeader,
    this_proc: u32,
    seg: *mut pg_sys::dsm_segment,
) -> ProcAttach {
    let n_procs = header.n_procs;
    let peer_count = (n_procs - 1) as usize; // n_procs >= 2 is layout invariant
    let mut outbound_senders = Vec::with_capacity(peer_count);

    // Outbound: one sender attach per peer (N-1 calls), targeting each peer's inbox.
    for peer_idx in 0..(n_procs - 1) {
        let r = peer_proc_for_index(this_proc, peer_idx);
        let off = header.inbox_offset(r) as usize;
        let mq_addr = unsafe { base.add(off) };
        let mq = mq_addr.cast::<pg_sys::shm_mq>();
        outbound_senders.push(unsafe { ShmMqSender::attach(seg, mq) });
    }

    // Inbound: this proc's own inbox. shm_mq enforces exactly one receiver per queue.
    let own_off = header.inbox_offset(this_proc) as usize;
    let own_mq_addr = unsafe { base.add(own_off) };
    let own_mq = own_mq_addr.cast::<pg_sys::shm_mq>();
    let inbound_receiver = unsafe { ShmMqReceiver::attach_existing(seg, own_mq) };

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
    let attach = unsafe { attach_proc(base, &header, proc_idx, seg) };
    if trace_on {
        let attach_ms = t_attach.unwrap().elapsed().as_secs_f64() * 1000.0;
        // N attach calls: N-1 sender attaches (one per peer inbox) + 1 own-inbox receiver attach.
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
        // Mesh-multiplexed: n_procs inboxes (one per receiver), not n_procs².
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
    fn header_inbox_offset_is_per_receiver() {
        // Mesh-multiplexed: one inbox per receiver. inbox(r) = queues_offset + r * queue_bytes.
        let l = compute_dsm_layout(4, 64 * 1024, 0).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        let aligned = h.queue_bytes;
        assert_eq!(h.inbox_offset(0), h.queues_offset);
        assert_eq!(h.inbox_offset(1), h.queues_offset + aligned);
        assert_eq!(h.inbox_offset(2), h.queues_offset + 2 * aligned);
        assert_eq!(h.inbox_offset(3), h.queues_offset + 3 * aligned);
    }

    #[test]
    fn compute_dsm_layout_scales_linearly_in_n_procs() {
        // Mesh multiplexing reduces queue count from N² to N. This test pins that math so any
        // accidental regression back to N² scaling shows up at compile-time of the test suite.
        let queue_bytes = 64 * 1024;
        let aligned = aligned_queue_bytes(queue_bytes);
        for n in [2u32, 4, 8, 16, 24] {
            let l = compute_dsm_layout(n, queue_bytes, 0).unwrap();
            let queues_size = (n as usize) * aligned;
            assert_eq!(
                l.region_total - l.queues_offset,
                queues_size,
                "n={n}: expected {} inbox bytes ({} inboxes × {} aligned), got {}",
                queues_size,
                n,
                aligned,
                l.region_total - l.queues_offset,
            );
        }
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
