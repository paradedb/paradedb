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

//! N×K DSM layout for the coordinator/worker MPP architecture.
//!
//! Each MPP query allocates a single DSM region containing:
//!
//! ```text
//!   +--- MppDsmHeader (repr C, 56 bytes, MAXALIGN-padded) ----+
//!   |    magic, version, n_workers, n_partitions, queue_bytes |
//!   |    plan_offset, plan_len, queues_offset, region_total   |
//!   +---------------------------------------------------------+
//!   | plan bytes (bincode-serialized worker fragment)         |
//!   +---------------------------------------------------------+
//!   | padding to MAXALIGN                                     |
//!   +---------------------------------------------------------+
//!   | shm_mq queue array: n_workers × n_partitions slots      |
//!   |   slot(w, p) = queues_offset                            |
//!   |             + (w * n_partitions + p) * queue_bytes      |
//!   +---------------------------------------------------------+
//! ```
//!
//! - `n_workers` is the number of producer-side participants (== leader-as-
//!   worker-0 + parallel workers). Leader is index 0.
//! - `n_partitions` is the consumer-side partition count for the cut. Scalar-
//!   agg final-gather has K=1; group-by-agg post-aggregate has K=N. The DSM
//!   region today carries one cut; multi-cut MPP is a follow-up.
//! - Self-edges (worker i writing to its own consumer partition feed) are
//!   included on purpose — every queue is shm_mq even when producer and
//!   consumer live in the same process. Keeps the topology symmetric.

use std::ffi::c_void;
use std::mem::size_of;

use pgrx::pg_sys;

use crate::postgres::customscan::mpp::mesh::{
    align_up_maxalign_checked, aligned_queue_bytes, ShmMqReceiver, ShmMqSender,
};

pub const MPP_DSM_MAGIC: u32 = 0x4D50_5052; // "MPPR" (RPC variant)
pub const MPP_DSM_HEADER_VERSION: u32 = 1;

/// Absolute cap on DSM region size. 16 GiB is two orders of magnitude beyond
/// any realistic workload; the cap fails early on a pathologically oversized
/// request rather than asking PG for ~`usize::MAX` bytes.
pub const MPP_DSM_MAX_BYTES: usize = 16 * 1024 * 1024 * 1024;

/// C-repr header at offset 0 of the DSM region.
///
/// Field ordering: four `u32`s (16 bytes), four `u64`s (32 bytes). 48 bytes
/// total with no internal padding on every supported target.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MppDsmHeader {
    pub magic: u32,
    pub header_version: u32,
    pub n_workers: u32,
    pub n_partitions: u32,
    pub queue_bytes: u64,
    pub plan_offset: u64,
    pub plan_len: u64,
    pub queues_offset: u64,
    pub region_total: u64,
}

impl MppDsmHeader {
    fn from_layout(layout: &DsmLayout) -> Self {
        Self {
            magic: MPP_DSM_MAGIC,
            header_version: MPP_DSM_HEADER_VERSION,
            n_workers: layout.n_workers,
            n_partitions: layout.n_partitions,
            queue_bytes: layout.queue_bytes as u64,
            plan_offset: layout.plan_offset as u64,
            plan_len: layout.plan_len as u64,
            queues_offset: layout.queues_offset as u64,
            region_total: layout.region_total as u64,
        }
    }

    pub fn validate(&self, region_total: u64) -> Result<(), &'static str> {
        if self.magic != MPP_DSM_MAGIC {
            return Err("mpp: DSM header magic mismatch");
        }
        if self.header_version != MPP_DSM_HEADER_VERSION {
            return Err("mpp: DSM header version mismatch");
        }
        if self.n_workers == 0 || self.n_partitions == 0 {
            return Err("mpp: header n_workers/n_partitions must both be > 0");
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

    /// Byte offset (relative to DSM base) of the queue at `(worker, partition)`.
    pub fn slot_offset(&self, worker: u32, partition: u32) -> u64 {
        debug_assert!(worker < self.n_workers);
        debug_assert!(partition < self.n_partitions);
        let slot = (worker as u64) * (self.n_partitions as u64) + (partition as u64);
        self.queues_offset + slot * self.queue_bytes
    }
}

/// Pure-math layout for [`compute_dsm_layout`].
#[derive(Debug, Clone, Copy)]
pub struct DsmLayout {
    pub n_workers: u32,
    pub n_partitions: u32,
    pub queue_bytes: usize,
    pub plan_offset: usize,
    pub plan_len: usize,
    pub queues_offset: usize,
    pub region_total: usize,
}

/// Compute the DSM region size and field offsets for one MPP query.
pub fn compute_dsm_layout(
    n_workers: u32,
    n_partitions: u32,
    queue_bytes: usize,
    plan_len: usize,
) -> Result<DsmLayout, &'static str> {
    if n_workers == 0 {
        return Err("mpp: n_workers must be > 0");
    }
    if n_partitions == 0 {
        return Err("mpp: n_partitions must be > 0");
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
    let total_slots = (n_workers as usize)
        .checked_mul(n_partitions as usize)
        .ok_or("mpp: n_workers × n_partitions overflow")?;
    let queues_bytes = total_slots
        .checked_mul(queue_bytes)
        .ok_or("mpp: queues bytes overflow")?;
    let region_total = queues_offset
        .checked_add(queues_bytes)
        .ok_or("mpp: region total overflow")?;
    if region_total > MPP_DSM_MAX_BYTES {
        return Err("mpp: DSM region exceeds MPP_DSM_MAX_BYTES");
    }
    Ok(DsmLayout {
        n_workers,
        n_partitions,
        queue_bytes,
        plan_offset,
        plan_len,
        queues_offset,
        region_total,
    })
}

/// Leader-side return: handles to all senders + receivers for participant 0.
pub struct LeaderAttach {
    /// Senders this participant uses to push rows to consumer partitions.
    /// `outbound_senders[p]` writes to `slot(0, p)`.
    pub outbound_senders: Vec<ShmMqSender>,
    /// Receivers this participant reads from (one per producer worker for
    /// each consumer partition the leader owns). The leader IS the consumer,
    /// so it owns ALL `n_partitions` columns of the queue grid; for each
    /// consumer partition `p` it has `n_workers` inbound queues from the
    /// producers.
    pub inbound_receivers: Vec<ShmMqReceiver>,
}

/// Worker-side return: just the senders. Workers don't consume; everything
/// flows toward the leader.
pub struct WorkerAttach {
    /// `outbound_senders[p]` writes to `slot(this_worker, p)`.
    pub outbound_senders: Vec<ShmMqSender>,
}

/// Initialize the DSM region as the leader. Writes the header, copies plan
/// bytes, calls `shm_mq_create` on every queue slot, and attaches the leader's
/// senders + all inbound receivers.
///
/// # Safety
/// - `coordinate` must point to the start of a DSM region of size
///   `>= layout.region_total`.
/// - `seg` must be the leader's `dsm_segment*`.
/// - The region must be uninitialized (the leader is the first writer).
pub unsafe fn leader_init(
    coordinate: *mut c_void,
    seg: *mut pg_sys::dsm_segment,
    layout: &DsmLayout,
    plan_bytes: &[u8],
) -> Result<LeaderAttach, String> {
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

    // Create every shm_mq slot. Each slot is `queue_bytes` aligned bytes; we
    // pass the address to `shm_mq_create` which initializes the ring buffer.
    // The leader is a consumer-only participant (leader-as-worker-0 is
    // deferred), so it attaches as receiver to every slot but as sender to
    // none — workers attach to their own row in `worker_attach`.
    let mut inbound_receivers =
        Vec::with_capacity((layout.n_workers as usize) * (layout.n_partitions as usize));
    for w in 0..layout.n_workers {
        for p in 0..layout.n_partitions {
            let off = (w as usize) * (layout.n_partitions as usize) + (p as usize);
            let mq_addr = unsafe { base.add(layout.queues_offset + off * layout.queue_bytes) };
            let mq = unsafe { pg_sys::shm_mq_create(mq_addr.cast(), layout.queue_bytes) };
            inbound_receivers.push(unsafe { ShmMqReceiver::attach_existing(seg, mq) });
        }
    }

    Ok(LeaderAttach {
        outbound_senders: Vec::new(),
        inbound_receivers,
    })
}

/// Attach to the leader-initialized DSM region as worker `worker_index` (1-based:
/// PG's `ParallelWorkerNumber + 1`, since participant 0 is the leader).
///
/// # Safety
/// - `coordinate` must be the DSM region pointer the leader initialized.
/// - `region_total` must match the DSM's attached size.
/// - `seg` may be NULL — `initialize_worker_custom_scan` does not surface
///   the segment pointer and `shm_mq_attach` handles NULL by skipping its
///   on-detach callback (cleanup falls back to process exit, safe for
///   parallel-worker lifetimes).
pub unsafe fn worker_attach(
    coordinate: *mut c_void,
    region_total: u64,
    worker_index: u32,
    seg: *mut pg_sys::dsm_segment,
) -> Result<(MppDsmHeader, Vec<u8>, WorkerAttach), String> {
    if coordinate.is_null() {
        return Err("mpp: worker_attach given null coordinate".into());
    }
    let base = coordinate as *mut u8;
    let header = unsafe { std::ptr::read(base.cast::<MppDsmHeader>()) };
    header
        .validate(region_total)
        .map_err(|e| format!("mpp: worker DSM validate: {e}"))?;
    if worker_index >= header.n_workers {
        return Err(format!(
            "mpp: worker_index {worker_index} not in 0..{}",
            header.n_workers
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

    // Attach as sender to every (worker_index, p) slot. `ShmMqSender::attach`
    // already calls `shm_mq_set_sender(mq, MyProc)` internally — calling it
    // a second time here trips PG's `Assert("mq->mq_sender == NULL")` and
    // aborts the worker.
    let mut outbound_senders = Vec::with_capacity(header.n_partitions as usize);
    for p in 0..header.n_partitions {
        let off = header.slot_offset(worker_index, p) as usize;
        let mq_addr = unsafe { base.add(off) };
        let mq = mq_addr.cast::<pg_sys::shm_mq>();
        outbound_senders.push(unsafe { ShmMqSender::attach(seg, mq) });
    }

    Ok((header, plan_bytes, WorkerAttach { outbound_senders }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_size_no_padding() {
        // 4×u32 + 5×u64 = 16 + 40 = 56 bytes, no internal padding.
        assert_eq!(size_of::<MppDsmHeader>(), 56);
    }

    #[test]
    fn compute_dsm_layout_works() {
        let l = compute_dsm_layout(4, 1, 64 * 1024, 1024).unwrap();
        assert_eq!(l.n_workers, 4);
        assert_eq!(l.n_partitions, 1);
        // 4*1 = 4 queue slots × queue_bytes
        let aligned = aligned_queue_bytes(64 * 1024);
        let queues_size = 4 * aligned;
        assert_eq!(l.region_total, l.queues_offset + queues_size);
    }

    #[test]
    fn compute_dsm_layout_rejects_zero_workers() {
        assert!(compute_dsm_layout(0, 1, 64 * 1024, 0).is_err());
    }

    #[test]
    fn compute_dsm_layout_rejects_zero_partitions() {
        assert!(compute_dsm_layout(2, 0, 64 * 1024, 0).is_err());
    }

    #[test]
    fn compute_dsm_layout_rejects_oversize() {
        assert!(compute_dsm_layout(u32::MAX, u32::MAX, 64 * 1024, 0).is_err());
    }

    #[test]
    fn header_slot_offset_is_row_major() {
        let l = compute_dsm_layout(3, 4, 64 * 1024, 0).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        let aligned = h.queue_bytes;
        assert_eq!(h.slot_offset(0, 0), h.queues_offset);
        assert_eq!(h.slot_offset(0, 1), h.queues_offset + aligned);
        assert_eq!(h.slot_offset(1, 0), h.queues_offset + 4 * aligned);
        assert_eq!(h.slot_offset(2, 3), h.queues_offset + 11 * aligned);
    }

    #[test]
    fn header_validate_accepts_self() {
        let l = compute_dsm_layout(2, 2, 64 * 1024, 0).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        assert!(h.validate(l.region_total as u64).is_ok());
    }

    #[test]
    fn header_validate_rejects_size_mismatch() {
        let l = compute_dsm_layout(2, 2, 64 * 1024, 0).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        assert!(h.validate(l.region_total as u64 + 1).is_err());
    }
}
