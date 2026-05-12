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

//! Multiplexed N×N DSM layout for the natural-shape MPP architecture.
//!
//! Each MPP query allocates a single DSM region containing:
//!
//! ```text
//!   +--- MppDsmHeader (repr C, MAXALIGN-padded) -------------+
//!   |    magic, version, n_procs, queue_bytes               |
//!   |    plan_offset, plan_len, queues_offset, region_total |
//!   |    cache_per_slot, cache_offsets, n_cache_sources     |
//!   +-------------------------------------------------------+
//!   | plan bytes (bincode-serialized worker fragment)       |
//!   +-------------------------------------------------------+
//!   | padding to MAXALIGN                                   |
//!   +-------------------------------------------------------+
//!   | shm_mq queue array: n_procs × n_procs slots           |
//!   |   slot(sender, receiver) = queues_offset              |
//!   |     + (sender * n_procs + receiver) * queue_bytes     |
//!   +-------------------------------------------------------+
//! ```
//!
//! - `n_procs` is the total participant count (1 leader + N parallel workers).
//!   Leader is `proc_idx = 0`; workers are `proc_idx = ParallelWorkerNumber + 1`.
//! - Every process attaches as **sender** to its row (`slot(this, *)`) and as
//!   **receiver** to its column (`slot(*, this)`). The grid is uniform and
//!   independent of plan shape: a single multiplexed queue per process-pair
//!   carries frames for any number of logical `(stage_id, partition)`
//!   channels, demultiplexed on the receive side via the [`MppFrameHeader`]
//!   prefix introduced in M1.a.
//! - Self-loops (`slot(k, k)`) are included on purpose. They are rarely the
//!   hot path (an embedded query usually keeps single-process traffic
//!   off-mesh), but keeping the topology symmetric simplifies the attach
//!   protocol and the routing math in [`super::runtime::ShmMqWorkerTransport`].

use std::ffi::c_void;
use std::mem::size_of;

use pgrx::pg_sys;

use crate::postgres::customscan::mpp::mesh::{
    align_up_maxalign_checked, aligned_queue_bytes, ShmMqReceiver, ShmMqSender,
};

pub const MPP_DSM_MAGIC: u32 = 0x4D50_5052; // "MPPR" (RPC variant)
/// V2: switched from `n_workers × n_partitions` grid to multiplexed
/// `n_procs × n_procs` grid (M1.b of the natural-shape track).
pub const MPP_DSM_HEADER_VERSION: u32 = 2;

/// Absolute cap on DSM region size. 16 GiB is two orders of magnitude beyond
/// any realistic workload; the cap fails early on a pathologically oversized
/// request rather than asking PG for ~`usize::MAX` bytes.
pub const MPP_DSM_MAX_BYTES: usize = 16 * 1024 * 1024 * 1024;

/// C-repr header at offset 0 of the DSM region.
///
/// Field ordering: four `u32`s (16 bytes), eight `u64`s (64 bytes). 80 bytes
/// total with no internal padding on every supported target.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MppDsmHeader {
    pub magic: u32,
    pub header_version: u32,
    /// Total participant count. Leader is `proc_idx = 0`; workers are
    /// `proc_idx = ParallelWorkerNumber + 1`. The shm_mq grid is `n_procs × n_procs`.
    pub n_procs: u32,
    pub n_cache_sources: u32,
    pub queue_bytes: u64,
    pub plan_offset: u64,
    pub plan_len: u64,
    pub queues_offset: u64,
    pub cache_per_slot: u64,
    pub cache_completion_offset: u64,
    pub cache_lengths_offset: u64,
    pub cache_data_offset: u64,
    pub region_total: u64,
}

impl MppDsmHeader {
    fn from_layout(layout: &DsmLayout) -> Self {
        Self {
            magic: MPP_DSM_MAGIC,
            header_version: MPP_DSM_HEADER_VERSION,
            n_procs: layout.n_procs,
            n_cache_sources: layout.n_cache_sources,
            queue_bytes: layout.queue_bytes as u64,
            plan_offset: layout.plan_offset as u64,
            plan_len: layout.plan_len as u64,
            queues_offset: layout.queues_offset as u64,
            cache_per_slot: layout.cache_per_slot as u64,
            cache_completion_offset: layout.cache_completion_offset as u64,
            cache_lengths_offset: layout.cache_lengths_offset as u64,
            cache_data_offset: layout.cache_data_offset as u64,
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

    /// Byte offset (relative to DSM base) of the queue at `(sender_proc, receiver_proc)`.
    ///
    /// Every process attaches as sender for its row (`slot(this, *)`) and as
    /// receiver for its column (`slot(*, this)`). Self-loops (`slot(k, k)`)
    /// are present in the grid but rarely used at runtime.
    pub fn slot_offset(&self, sender_proc: u32, receiver_proc: u32) -> u64 {
        debug_assert!(sender_proc < self.n_procs);
        debug_assert!(receiver_proc < self.n_procs);
        let slot = (sender_proc as u64) * (self.n_procs as u64) + (receiver_proc as u64);
        self.queues_offset + slot * self.queue_bytes
    }

    /// Byte offset of the build-side cache slot at `(source, worker)`. The
    /// build-cache region is sized as `n_cache_sources × (n_procs - 1)` to
    /// match the worker-only producer count (leader does not write the cache).
    #[cfg(test)]
    pub fn cache_data_slot_offset(&self, source: u32, worker: u32) -> u64 {
        debug_assert!(source < self.n_cache_sources);
        let worker_slots = self.n_procs.saturating_sub(1);
        debug_assert!(worker < worker_slots);
        let slot = (source as u64) * (worker_slots as u64) + (worker as u64);
        self.cache_data_offset + slot * self.cache_per_slot
    }
}

/// Pure-math layout for [`compute_dsm_layout`].
#[derive(Debug, Clone, Copy)]
pub struct DsmLayout {
    pub n_procs: u32,
    pub n_cache_sources: u32,
    pub queue_bytes: usize,
    pub cache_per_slot: usize,
    pub plan_offset: usize,
    pub plan_len: usize,
    pub queues_offset: usize,
    pub cache_completion_offset: usize,
    pub cache_lengths_offset: usize,
    pub cache_data_offset: usize,
    pub region_total: usize,
}

/// Compute the DSM region size and field offsets for one MPP query.
///
/// `n_procs` is the total participant count (1 leader + N workers). The
/// shm_mq grid is `n_procs × n_procs`; each process attaches as sender to its
/// row and receiver to its column.
///
/// `n_cache_sources` is the number of non-partitioning sources to allocate
/// build-side cache slots for. `cache_per_slot` is the bytes reserved for
/// each (source, worker) pair (worst-case per-worker IPC payload). Pass 0
/// for both if no cache is needed. The cache region is sized using
/// worker-only slots (`n_procs - 1`) since the leader does not write the
/// build-side cache.
pub fn compute_dsm_layout(
    n_procs: u32,
    queue_bytes: usize,
    plan_len: usize,
    n_cache_sources: u32,
    cache_per_slot: usize,
) -> Result<DsmLayout, &'static str> {
    if n_procs == 0 {
        return Err("mpp: n_procs must be > 0");
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
    let total_slots = (n_procs as usize)
        .checked_mul(n_procs as usize)
        .ok_or("mpp: n_procs × n_procs overflow")?;
    let queues_bytes = total_slots
        .checked_mul(queue_bytes)
        .ok_or("mpp: queues bytes overflow")?;
    let queues_end = queues_offset
        .checked_add(queues_bytes)
        .ok_or("mpp: queues end overflow")?;

    // Build-side cache region. Layout:
    //   completion: n_cache_sources × u32  (atomic counter)
    //   lengths:    n_cache_sources × worker_slots × u32  (actual bytes written)
    //   data:       n_cache_sources × worker_slots × cache_per_slot
    //
    // `worker_slots = n_procs - 1` because the leader is consumer-only for
    // the build-side cache; only workers all-gather into it.
    let worker_slots = (n_procs as usize).saturating_sub(1).max(1);
    let cache_completion_offset =
        align_up_maxalign_checked(queues_end).ok_or("mpp: cache completion alignment overflow")?;
    let cache_completion_size = (n_cache_sources as usize)
        .checked_mul(size_of::<u32>())
        .ok_or("mpp: cache completion size overflow")?;
    let cache_lengths_offset = align_up_maxalign_checked(
        cache_completion_offset
            .checked_add(cache_completion_size)
            .ok_or("mpp: cache lengths offset overflow")?,
    )
    .ok_or("mpp: cache lengths alignment overflow")?;
    let cache_lengths_size = (n_cache_sources as usize)
        .checked_mul(worker_slots)
        .and_then(|x| x.checked_mul(size_of::<u32>()))
        .ok_or("mpp: cache lengths size overflow")?;
    let cache_data_offset = align_up_maxalign_checked(
        cache_lengths_offset
            .checked_add(cache_lengths_size)
            .ok_or("mpp: cache data offset overflow")?,
    )
    .ok_or("mpp: cache data alignment overflow")?;
    let cache_data_size = (n_cache_sources as usize)
        .checked_mul(worker_slots)
        .and_then(|x| x.checked_mul(cache_per_slot))
        .ok_or("mpp: cache data size overflow")?;
    let region_total = cache_data_offset
        .checked_add(cache_data_size)
        .ok_or("mpp: region total overflow")?;
    if region_total > MPP_DSM_MAX_BYTES {
        return Err("mpp: DSM region exceeds MPP_DSM_MAX_BYTES");
    }
    Ok(DsmLayout {
        n_procs,
        n_cache_sources,
        queue_bytes,
        cache_per_slot,
        plan_offset,
        plan_len,
        queues_offset,
        cache_completion_offset,
        cache_lengths_offset,
        cache_data_offset,
        region_total,
    })
}

/// Runtime handle to the build-side cache region inside DSM.
///
/// Held on every participant's customscan state. Workers use it to write their
/// own slice and read back peer slices via the all-gather barrier; the leader
/// holds it inert (consumer-only for the cache — leader doesn't write a slice).
///
/// `Send + Sync` is asserted because the underlying DSM mapping is shared
/// memory accessed by multiple processes; access is coordinated via atomic
/// completion counters and write-once length cells.
#[derive(Debug)]
pub struct MppBuildCache {
    base: *mut u8,
    /// Number of worker slots in the cache. Equals `n_procs - 1` since the
    /// leader is consumer-only for the cache.
    pub n_workers: u32,
    pub n_sources: u32,
    pub cache_per_slot: usize,
    pub completion_offset: usize,
    pub lengths_offset: usize,
    pub data_offset: usize,
}

unsafe impl Send for MppBuildCache {}
unsafe impl Sync for MppBuildCache {}

impl MppBuildCache {
    /// Construct from a raw DSM base pointer and the resolved `MppDsmHeader`.
    ///
    /// # Safety
    /// `base` must point to a DSM region of size `>= header.region_total` whose
    /// header has already been validated.
    pub unsafe fn from_header(base: *mut u8, header: &MppDsmHeader) -> Self {
        Self {
            base,
            // Workers-only — leader is consumer-only for the build cache.
            n_workers: header.n_procs.saturating_sub(1).max(1),
            n_sources: header.n_cache_sources,
            cache_per_slot: header.cache_per_slot as usize,
            completion_offset: header.cache_completion_offset as usize,
            lengths_offset: header.cache_lengths_offset as usize,
            data_offset: header.cache_data_offset as usize,
        }
    }

    fn slot_data_ptr(&self, source: u32, worker: u32) -> *mut u8 {
        debug_assert!(source < self.n_sources);
        debug_assert!(worker < self.n_workers);
        let slot = (source as usize) * (self.n_workers as usize) + (worker as usize);
        unsafe { self.base.add(self.data_offset + slot * self.cache_per_slot) }
    }

    fn length_ptr(&self, source: u32, worker: u32) -> *mut u32 {
        debug_assert!(source < self.n_sources);
        debug_assert!(worker < self.n_workers);
        let slot = (source as usize) * (self.n_workers as usize) + (worker as usize);
        unsafe {
            self.base
                .add(self.lengths_offset + slot * size_of::<u32>())
                .cast()
        }
    }

    fn completion_ptr(&self, source: u32) -> *mut std::sync::atomic::AtomicU32 {
        debug_assert!(source < self.n_sources);
        unsafe {
            self.base
                .add(self.completion_offset + (source as usize) * size_of::<u32>())
                .cast()
        }
    }

    /// Worker writes its slice for `source` and atomically signals completion.
    /// Returns an error if `bytes.len()` exceeds the per-slot cap.
    pub fn write_slice(&self, source: u32, worker: u32, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() > self.cache_per_slot {
            return Err(format!(
                "mpp: build-side slice for source={source} worker={worker} is {} bytes, exceeds cap {}",
                bytes.len(),
                self.cache_per_slot
            ));
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.slot_data_ptr(source, worker),
                bytes.len(),
            );
            // Length is observed only after the completion increment (Release).
            // No atomic needed for the length itself; the fence below makes the
            // copy + length write visible to readers that observe the completion.
            std::ptr::write_volatile(self.length_ptr(source, worker), bytes.len() as u32);
        }
        let counter = unsafe { &*self.completion_ptr(source) };
        counter.fetch_add(1, std::sync::atomic::Ordering::Release);
        Ok(())
    }

    /// Worker reads peer's slice. Caller must have already passed the barrier.
    pub fn read_slice(&self, source: u32, worker: u32) -> Vec<u8> {
        let len = unsafe { std::ptr::read_volatile(self.length_ptr(source, worker)) } as usize;
        let mut out = Vec::with_capacity(len);
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.slot_data_ptr(source, worker),
                out.as_mut_ptr(),
                len,
            );
            out.set_len(len);
        }
        out
    }

    /// Spin until all `n_workers` have signalled completion for `source`.
    /// Yields to PG via `check_for_interrupts!` so `statement_timeout` works.
    pub fn wait_complete(&self, source: u32) {
        let counter = unsafe { &*self.completion_ptr(source) };
        loop {
            if counter.load(std::sync::atomic::Ordering::Acquire) >= self.n_workers {
                return;
            }
            pgrx::check_for_interrupts!();
            std::hint::spin_loop();
        }
    }
}

/// Per-participant return: handles for the process's row (senders) and
/// column (receivers) in the multiplexed `n_procs × n_procs` grid.
pub struct ProcAttach {
    /// `outbound_senders[r]` writes to `slot(this_proc, r)`. One entry per
    /// receiver process, including the self-loop `slot(this_proc, this_proc)`.
    pub outbound_senders: Vec<ShmMqSender>,
    /// `inbound_receivers[s]` reads from `slot(s, this_proc)`. One entry per
    /// sender process, including the self-loop.
    pub inbound_receivers: Vec<ShmMqReceiver>,
}

/// Initialize the DSM region as the leader (`proc_idx = 0`). Writes the
/// header, copies the plan bytes, calls `shm_mq_create` on every queue slot,
/// and attaches the leader's row + column handles.
///
/// In the multiplexed `n_procs × n_procs` grid, every process — leader
/// included — is a full participant: sender for its row, receiver for its
/// column. The leader is responsible for the one-time `shm_mq_create` on
/// every queue (workers cannot, since the region is uninitialized at their
/// attach time), then performs its own `set_sender` / `set_receiver` calls
/// on its row and column slots.
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

    // One-time create of every shm_mq slot. Workers cannot do this — the
    // region is uninitialized at their attach time — so the leader runs
    // `shm_mq_create` for all `n_procs²` slots even though it only attaches
    // to its own row and column below.
    let header = MppDsmHeader::from_layout(layout);
    let n_procs = layout.n_procs;
    for s in 0..n_procs {
        for r in 0..n_procs {
            let off = header.slot_offset(s, r) as usize;
            let mq_addr = unsafe { base.add(off) };
            unsafe { pg_sys::shm_mq_create(mq_addr.cast(), layout.queue_bytes) };
        }
    }

    Ok(unsafe { attach_proc_row_and_column(base, &header, 0, seg) })
}

/// Attach to the leader-initialized DSM region as `proc_idx` (`0 = leader`,
/// `1..N = parallel workers`).
///
/// Workers use this from `initialize_worker_custom_scan` via `worker_attach`;
/// the leader uses it inline at the end of `leader_init`. Each process
/// attaches as sender to its row and receiver to its column of the
/// `n_procs × n_procs` grid, including the self-loop at `(this, this)`.
///
/// # Safety
/// - `base` must point to a DSM region whose header has been validated.
/// - `header.slot_offset(s, r)` must already point at a slot initialized by
///   `shm_mq_create` (the leader does this in `leader_init`).
/// - `seg` may be NULL on workers — `shm_mq_attach` skips its on-detach
///   callback when so.
unsafe fn attach_proc_row_and_column(
    base: *mut u8,
    header: &MppDsmHeader,
    this_proc: u32,
    seg: *mut pg_sys::dsm_segment,
) -> ProcAttach {
    let n_procs = header.n_procs;
    let mut outbound_senders = Vec::with_capacity(n_procs as usize);
    let mut inbound_receivers = Vec::with_capacity(n_procs as usize);

    // Senders: this process's row. `outbound_senders[r]` writes to slot(this, r).
    for r in 0..n_procs {
        let off = header.slot_offset(this_proc, r) as usize;
        let mq_addr = unsafe { base.add(off) };
        let mq = mq_addr.cast::<pg_sys::shm_mq>();
        outbound_senders.push(unsafe { ShmMqSender::attach(seg, mq) });
    }

    // Receivers: this process's column. `inbound_receivers[s]` reads from
    // slot(s, this). The self-loop slot was already attached as sender
    // above; `shm_mq_set_receiver` is a separate operation on the same
    // queue handle, so the double-attach (once as sender, once as receiver)
    // on the self-loop is safe.
    for s in 0..n_procs {
        let off = header.slot_offset(s, this_proc) as usize;
        let mq_addr = unsafe { base.add(off) };
        let mq = mq_addr.cast::<pg_sys::shm_mq>();
        inbound_receivers.push(unsafe { ShmMqReceiver::attach_existing(seg, mq) });
    }

    ProcAttach {
        outbound_senders,
        inbound_receivers,
    }
}

/// Attach to the leader-initialized DSM region as `proc_idx` (1-based for
/// workers: PG's `ParallelWorkerNumber + 1`).
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

    let attach = unsafe { attach_proc_row_and_column(base, &header, proc_idx, seg) };
    Ok((header, plan_bytes, attach))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_dsm_layout_works() {
        let l = compute_dsm_layout(4, 64 * 1024, 1024, 0, 0).unwrap();
        assert_eq!(l.n_procs, 4);
        // Grid is n_procs × n_procs = 16 slots.
        let aligned = aligned_queue_bytes(64 * 1024);
        let queues_size = 16 * aligned;
        assert!(l.region_total >= l.queues_offset + queues_size);
    }

    #[test]
    fn compute_dsm_layout_with_cache() {
        // n_procs=4 → 3 worker slots in the cache (leader excluded).
        let l = compute_dsm_layout(4, 64 * 1024, 1024, 2, 1024 * 1024).unwrap();
        let cache_data_size = 2 * 3 * 1024 * 1024; // 2 sources × 3 worker slots × 1 MiB
        assert_eq!(l.region_total, l.cache_data_offset + cache_data_size);
    }

    #[test]
    fn compute_dsm_layout_rejects_zero_procs() {
        assert!(compute_dsm_layout(0, 64 * 1024, 0, 0, 0).is_err());
    }

    #[test]
    fn compute_dsm_layout_rejects_oversize() {
        assert!(compute_dsm_layout(u32::MAX, 64 * 1024, 0, 0, 0).is_err());
    }

    #[test]
    fn header_slot_offset_is_row_major_over_n_procs() {
        // 4 procs → 4×4 = 16 slots, row-major over (sender, receiver).
        let l = compute_dsm_layout(4, 64 * 1024, 0, 0, 0).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        let aligned = h.queue_bytes;
        assert_eq!(h.slot_offset(0, 0), h.queues_offset);
        assert_eq!(h.slot_offset(0, 1), h.queues_offset + aligned);
        assert_eq!(h.slot_offset(1, 0), h.queues_offset + 4 * aligned);
        // Self-loop on proc 3: (3,3) → row 3, col 3 → slot 15.
        assert_eq!(h.slot_offset(3, 3), h.queues_offset + 15 * aligned);
    }

    #[test]
    fn header_cache_offsets() {
        // n_procs=4 → 3 worker slots in the cache.
        let l = compute_dsm_layout(4, 64 * 1024, 0, 2, 1024).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        // (source=0, worker=0) is the first slot.
        assert_eq!(h.cache_data_slot_offset(0, 0), h.cache_data_offset);
        // (source=0, worker=1) is one slot later.
        assert_eq!(h.cache_data_slot_offset(0, 1), h.cache_data_offset + 1024);
        // (source=1, worker=0) is `worker_slots` slots in.
        assert_eq!(
            h.cache_data_slot_offset(1, 0),
            h.cache_data_offset + 3 * 1024
        );
    }

    #[test]
    fn header_validate_accepts_self() {
        let l = compute_dsm_layout(2, 64 * 1024, 0, 0, 0).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        assert!(h.validate(l.region_total as u64).is_ok());
    }

    #[test]
    fn header_validate_rejects_size_mismatch() {
        let l = compute_dsm_layout(2, 64 * 1024, 0, 0, 0).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        assert!(h.validate(l.region_total as u64 + 1).is_err());
    }
}
