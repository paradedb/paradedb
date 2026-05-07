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
//! - `n_partitions` is the consumer-side partition count for the network
//!   boundary. The DF-D fork's planner emits exactly one boundary per query
//!   under this PR's config (`in_process_mode=on`, peer-shuffles off);
//!   K-cut layouts come with the multi-peer-mesh follow-up.
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
/// Bumped 3 → 4: peer-mesh region is now an array of K independent meshes
/// (one per nested cross-worker shuffle stage), each sized N×N×peer_queue_bytes.
/// Old workers attach to a v3 region by `MPP_DSM_HEADER_VERSION` mismatch and
/// bail before reading the new `n_peer_meshes` field.
pub const MPP_DSM_HEADER_VERSION: u32 = 4;

/// Hard cap on per-source-per-worker cache slot size. Sized for our 25 M
/// bench: a 1.25 M-row build side encoded in Arrow IPC is ~400 MB total
/// (Utf8View widening, schema overhead) — split across N workers, each
/// slot needs ~400/N MB. 256 MiB caps at the worst single-worker slice.
/// A future heuristic should derive this from index stats per query.
pub const MPP_CACHE_PER_SLOT: usize = 256 * 1024 * 1024;

/// Absolute cap on DSM region size. 16 GiB is two orders of magnitude beyond
/// any realistic workload; the cap fails early on a pathologically oversized
/// request rather than asking PG for ~`usize::MAX` bytes.
pub const MPP_DSM_MAX_BYTES: usize = 16 * 1024 * 1024 * 1024;

/// C-repr header at offset 0 of the DSM region.
///
/// Layout sections (bottom to top): plan bytes → leader-bound queues
/// (n_workers × n_partitions slots) → build-side cache → peer-mesh array
/// (n_peer_meshes × n_workers × n_workers slots). Each section is
/// MAXALIGN-padded.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MppDsmHeader {
    pub magic: u32,
    pub header_version: u32,
    pub n_workers: u32,
    pub n_partitions: u32,
    pub n_cache_sources: u32,
    /// Number of peer meshes reserved. 0 means no peer-mesh region was
    /// allocated. Each mesh is `n_workers × n_workers × peer_queue_bytes`.
    pub n_peer_meshes: u32,
    pub queue_bytes: u64,
    pub plan_offset: u64,
    pub plan_len: u64,
    pub queues_offset: u64,
    pub cache_per_slot: u64,
    pub cache_completion_offset: u64,
    pub cache_lengths_offset: u64,
    pub cache_data_offset: u64,
    /// Per-edge byte size for peer-mesh slots. 0 means no peer mesh.
    pub peer_queue_bytes: u64,
    /// Byte offset (relative to DSM base) of the peer-mesh array. Each
    /// mesh occupies `n_workers² × peer_queue_bytes` bytes; mesh `m` starts
    /// at `peer_mesh_offset + m × n_workers² × peer_queue_bytes`.
    pub peer_mesh_offset: u64,
    pub region_total: u64,
}

impl MppDsmHeader {
    fn from_layout(layout: &DsmLayout) -> Self {
        Self {
            magic: MPP_DSM_MAGIC,
            header_version: MPP_DSM_HEADER_VERSION,
            n_workers: layout.n_workers,
            n_partitions: layout.n_partitions,
            n_cache_sources: layout.n_cache_sources,
            n_peer_meshes: layout.n_peer_meshes,
            queue_bytes: layout.queue_bytes as u64,
            plan_offset: layout.plan_offset as u64,
            plan_len: layout.plan_len as u64,
            queues_offset: layout.queues_offset as u64,
            cache_per_slot: layout.cache_per_slot as u64,
            cache_completion_offset: layout.cache_completion_offset as u64,
            cache_lengths_offset: layout.cache_lengths_offset as u64,
            cache_data_offset: layout.cache_data_offset as u64,
            peer_queue_bytes: layout.peer_queue_bytes as u64,
            peer_mesh_offset: layout.peer_mesh_offset as u64,
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

    /// Byte offset of the build-side cache slot at `(source, worker)`.
    #[cfg(test)]
    pub fn cache_data_slot_offset(&self, source: u32, worker: u32) -> u64 {
        debug_assert!(source < self.n_cache_sources);
        debug_assert!(worker < self.n_workers);
        let slot = (source as u64) * (self.n_workers as u64) + (worker as u64);
        self.cache_data_offset + slot * self.cache_per_slot
    }

    /// Byte offset of the peer-mesh queue slot at `(mesh, producer, consumer)`.
    /// Valid only when `peer_queue_bytes > 0`. `mesh` indexes into the array
    /// of K peer meshes (one per nested cross-worker shuffle); `producer`
    /// and `consumer` are 0-based worker indices.
    pub fn peer_slot_offset(&self, mesh: u32, producer: u32, consumer: u32) -> u64 {
        debug_assert!(self.peer_queue_bytes > 0);
        debug_assert!(mesh < self.n_peer_meshes);
        debug_assert!(producer < self.n_workers);
        debug_assert!(consumer < self.n_workers);
        let n = self.n_workers as u64;
        let mesh_size_bytes = n * n * self.peer_queue_bytes;
        let mesh_base = self.peer_mesh_offset + (mesh as u64) * mesh_size_bytes;
        let slot = (producer as u64) * n + (consumer as u64);
        mesh_base + slot * self.peer_queue_bytes
    }
}

/// Pure-math layout for [`compute_dsm_layout`].
#[derive(Debug, Clone, Copy)]
pub struct DsmLayout {
    pub n_workers: u32,
    pub n_partitions: u32,
    pub n_cache_sources: u32,
    pub n_peer_meshes: u32,
    pub queue_bytes: usize,
    pub cache_per_slot: usize,
    pub peer_queue_bytes: usize,
    pub plan_offset: usize,
    pub plan_len: usize,
    pub queues_offset: usize,
    pub cache_completion_offset: usize,
    pub cache_lengths_offset: usize,
    pub cache_data_offset: usize,
    pub peer_mesh_offset: usize,
    pub region_total: usize,
}

/// Compute the DSM region size and field offsets for one MPP query.
///
/// `n_cache_sources` is the number of non-partitioning sources to allocate
/// build-side cache slots for. `cache_per_slot` is the bytes reserved for
/// each (source, worker) pair (worst-case per-worker IPC payload). Pass 0
/// for both if no cache is needed.
///
/// `peer_queue_bytes` and `n_peer_meshes` reserve a peer-mesh array sized
/// `n_peer_meshes × n_workers × n_workers × peer_queue_bytes` after the
/// cache. Pass 0 for either to skip; callers that need cross-worker
/// shuffles request `n_peer_meshes >= 1` with `n_peer_meshes` set to the
/// number of nested cross-worker `NetworkShuffleExec` stages in the plan.
// Exempt from too_many_arguments: each arg is an independent dimension
// of the layout and bundling them into a struct just adds a layer of
// indirection across every caller.
#[allow(clippy::too_many_arguments)]
pub fn compute_dsm_layout(
    n_workers: u32,
    n_partitions: u32,
    queue_bytes: usize,
    plan_len: usize,
    n_cache_sources: u32,
    cache_per_slot: usize,
    peer_queue_bytes: usize,
    n_peer_meshes: u32,
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
    let queues_end = queues_offset
        .checked_add(queues_bytes)
        .ok_or("mpp: queues end overflow")?;

    // Build-side cache region. Layout:
    //   completion: n_cache_sources × u32  (atomic counter)
    //   lengths:    n_cache_sources × n_workers × u32  (actual bytes written)
    //   data:       n_cache_sources × n_workers × cache_per_slot
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
        .checked_mul(n_workers as usize)
        .and_then(|x| x.checked_mul(size_of::<u32>()))
        .ok_or("mpp: cache lengths size overflow")?;
    let cache_data_offset = align_up_maxalign_checked(
        cache_lengths_offset
            .checked_add(cache_lengths_size)
            .ok_or("mpp: cache data offset overflow")?,
    )
    .ok_or("mpp: cache data alignment overflow")?;
    let cache_data_size = (n_cache_sources as usize)
        .checked_mul(n_workers as usize)
        .and_then(|x| x.checked_mul(cache_per_slot))
        .ok_or("mpp: cache data size overflow")?;
    let cache_data_end = cache_data_offset
        .checked_add(cache_data_size)
        .ok_or("mpp: cache data end overflow")?;

    // Peer-mesh array: n_peer_meshes × n_workers × n_workers × peer_queue_bytes.
    // Sits after the build-side cache. Skipped (size=0) when n_peer_meshes == 0
    // or peer_queue_bytes == 0.
    let peer_queue_bytes_aligned = if peer_queue_bytes == 0 || n_peer_meshes == 0 {
        0
    } else {
        let aligned = aligned_queue_bytes(peer_queue_bytes);
        if aligned == 0 {
            return Err("mpp: peer_queue_bytes too small after alignment");
        }
        aligned
    };
    let effective_n_meshes = if peer_queue_bytes_aligned == 0 {
        0
    } else {
        n_peer_meshes
    };
    let peer_mesh_offset =
        align_up_maxalign_checked(cache_data_end).ok_or("mpp: peer mesh alignment overflow")?;
    let peer_mesh_size = if effective_n_meshes == 0 {
        0
    } else {
        (effective_n_meshes as usize)
            .checked_mul(n_workers as usize)
            .and_then(|x| x.checked_mul(n_workers as usize))
            .and_then(|x| x.checked_mul(peer_queue_bytes_aligned))
            .ok_or("mpp: peer mesh size overflow")?
    };
    let region_total = peer_mesh_offset
        .checked_add(peer_mesh_size)
        .ok_or("mpp: region total overflow")?;
    if region_total > MPP_DSM_MAX_BYTES {
        return Err("mpp: DSM region exceeds MPP_DSM_MAX_BYTES");
    }
    Ok(DsmLayout {
        n_workers,
        n_partitions,
        n_cache_sources,
        n_peer_meshes: effective_n_meshes,
        queue_bytes,
        cache_per_slot,
        peer_queue_bytes: peer_queue_bytes_aligned,
        plan_offset,
        plan_len,
        queues_offset,
        cache_completion_offset,
        cache_lengths_offset,
        cache_data_offset,
        peer_mesh_offset,
        region_total,
    })
}

/// Runtime handle to the build-side cache region inside DSM.
///
/// Held on every participant's customscan state. Workers use it to write their
/// own slice and read back peer slices via the all-gather barrier; the leader
/// holds it inert (no slot reserved for it; consumer-only this iteration).
///
/// `Send + Sync` is asserted because the underlying DSM mapping is shared
/// memory accessed by multiple processes; access is coordinated via atomic
/// completion counters and write-once length cells.
#[derive(Debug)]
pub struct MppBuildCache {
    base: *mut u8,
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
            n_workers: header.n_workers,
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

/// Worker-side return for the leader-bound queue mesh.
pub struct WorkerAttach {
    /// `outbound_senders[p]` writes to `slot(this_worker, p)`.
    pub outbound_senders: Vec<ShmMqSender>,
}

/// Worker-side return for the peer-mesh queue grid (Track B).
///
/// Each worker is both a producer (its row of the grid) and a consumer
/// (its column of the grid). Senders are indexed by `consumer_idx`, receivers
/// by `producer_idx`. Self-edges (producer == consumer == this_worker) are
/// included so the topology is symmetric.
pub struct WorkerPeerAttach {
    /// `peer_outbound[c]` writes to `peer_slot(this_worker, c)`.
    pub peer_outbound: Vec<ShmMqSender>,
    /// `peer_inbound[p]` reads from `peer_slot(p, this_worker)`.
    pub peer_inbound: Vec<ShmMqReceiver>,
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

    // Peer-mesh queues. The leader is not a peer (not part of the N×N
    // grids); it just `shm_mq_create`s every slot so workers can attach as
    // producer (their row) and consumer (their column) for each of the K
    // peer meshes. Skipped when no peer-mesh array was reserved.
    if layout.peer_queue_bytes > 0 && layout.n_peer_meshes > 0 {
        let n = layout.n_workers as usize;
        let mesh_size_bytes = n * n * layout.peer_queue_bytes;
        for mesh_idx in 0..layout.n_peer_meshes as usize {
            let mesh_base = layout.peer_mesh_offset + mesh_idx * mesh_size_bytes;
            for prod in 0..n {
                for cons in 0..n {
                    let slot = prod * n + cons;
                    let mq_addr = unsafe { base.add(mesh_base + slot * layout.peer_queue_bytes) };
                    unsafe { pg_sys::shm_mq_create(mq_addr.cast(), layout.peer_queue_bytes) };
                }
            }
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

/// Attach this worker to every peer-mesh queue grid as both producer
/// (its row) and consumer (its column). Returns one `WorkerPeerAttach`
/// per reserved peer mesh; an empty Vec means no peer-mesh array was
/// reserved.
///
/// # Safety
/// - `coordinate` must be the DSM region pointer the leader initialized.
/// - `header` must be the validated header read from that region.
/// - `seg` may be NULL — `shm_mq_attach` handles NULL by skipping its
///   on-detach callback (cleanup falls back to process exit).
pub unsafe fn worker_peer_attach(
    coordinate: *mut c_void,
    header: &MppDsmHeader,
    worker_index: u32,
    seg: *mut pg_sys::dsm_segment,
) -> Result<Vec<WorkerPeerAttach>, String> {
    if header.peer_queue_bytes == 0 || header.n_peer_meshes == 0 {
        return Ok(Vec::new());
    }
    if coordinate.is_null() {
        return Err("mpp: worker_peer_attach given null coordinate".into());
    }
    if worker_index >= header.n_workers {
        return Err(format!(
            "mpp: worker_peer_attach: worker_index={worker_index} >= n_workers={}",
            header.n_workers
        ));
    }
    let base = coordinate as *mut u8;
    let n = header.n_workers;

    let mut attaches = Vec::with_capacity(header.n_peer_meshes as usize);
    for mesh_idx in 0..header.n_peer_meshes {
        // Attach as PRODUCER (this row): one sender per consumer column.
        let mut peer_outbound = Vec::with_capacity(n as usize);
        for c in 0..n {
            let off = header.peer_slot_offset(mesh_idx, worker_index, c) as usize;
            let mq_addr = unsafe { base.add(off) };
            let mq = mq_addr.cast::<pg_sys::shm_mq>();
            peer_outbound.push(unsafe { ShmMqSender::attach(seg, mq) });
        }

        // Attach as CONSUMER (this column): one receiver per producer row.
        let mut peer_inbound = Vec::with_capacity(n as usize);
        for p in 0..n {
            let off = header.peer_slot_offset(mesh_idx, p, worker_index) as usize;
            let mq_addr = unsafe { base.add(off) };
            let mq = mq_addr.cast::<pg_sys::shm_mq>();
            peer_inbound.push(unsafe { ShmMqReceiver::attach_existing(seg, mq) });
        }

        attaches.push(WorkerPeerAttach {
            peer_outbound,
            peer_inbound,
        });
    }
    Ok(attaches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_dsm_layout_works() {
        let l = compute_dsm_layout(4, 1, 64 * 1024, 1024, 0, 0, 0, 0).unwrap();
        assert_eq!(l.n_workers, 4);
        assert_eq!(l.n_partitions, 1);
        // No cache reserved → cache regions all sit at queues_end (after MAXALIGN).
        let aligned = aligned_queue_bytes(64 * 1024);
        let queues_size = 4 * aligned;
        assert!(l.region_total >= l.queues_offset + queues_size);
        // No peer mesh reserved.
        assert_eq!(l.peer_queue_bytes, 0);
    }

    #[test]
    fn compute_dsm_layout_with_cache() {
        let l = compute_dsm_layout(4, 1, 64 * 1024, 1024, 2, 1024 * 1024, 0, 0).unwrap();
        let cache_data_size = 2 * 4 * 1024 * 1024; // 2 sources × 4 workers × 1 MiB
                                                   // peer_mesh_offset == cache_data_end after MAXALIGN; with peer=0 region_total == peer_mesh_offset.
        assert_eq!(l.region_total, l.peer_mesh_offset);
        assert!(l.peer_mesh_offset >= l.cache_data_offset + cache_data_size);
    }

    #[test]
    fn compute_dsm_layout_with_peer_mesh() {
        let l = compute_dsm_layout(4, 1, 64 * 1024, 1024, 0, 0, 1024 * 1024, 1).unwrap();
        let aligned_peer = aligned_queue_bytes(1024 * 1024);
        let peer_mesh_size = 4 * 4 * aligned_peer;
        assert_eq!(l.peer_queue_bytes, aligned_peer);
        assert_eq!(l.region_total, l.peer_mesh_offset + peer_mesh_size);
    }

    #[test]
    fn compute_dsm_layout_rejects_zero_workers() {
        assert!(compute_dsm_layout(0, 1, 64 * 1024, 0, 0, 0, 0, 0).is_err());
    }

    #[test]
    fn compute_dsm_layout_rejects_zero_partitions() {
        assert!(compute_dsm_layout(2, 0, 64 * 1024, 0, 0, 0, 0, 0).is_err());
    }

    #[test]
    fn compute_dsm_layout_rejects_oversize() {
        assert!(compute_dsm_layout(u32::MAX, u32::MAX, 64 * 1024, 0, 0, 0, 0, 0).is_err());
    }

    #[test]
    fn header_slot_offset_is_row_major() {
        let l = compute_dsm_layout(3, 4, 64 * 1024, 0, 0, 0, 0, 0).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        let aligned = h.queue_bytes;
        assert_eq!(h.slot_offset(0, 0), h.queues_offset);
        assert_eq!(h.slot_offset(0, 1), h.queues_offset + aligned);
        assert_eq!(h.slot_offset(1, 0), h.queues_offset + 4 * aligned);
        assert_eq!(h.slot_offset(2, 3), h.queues_offset + 11 * aligned);
    }

    #[test]
    fn header_cache_offsets() {
        let l = compute_dsm_layout(3, 1, 64 * 1024, 0, 2, 1024, 0, 0).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        // (source=0, worker=0) is the first slot.
        assert_eq!(h.cache_data_slot_offset(0, 0), h.cache_data_offset);
        // (source=0, worker=1) is one slot later.
        assert_eq!(h.cache_data_slot_offset(0, 1), h.cache_data_offset + 1024);
        // (source=1, worker=0) is n_workers slots in.
        assert_eq!(
            h.cache_data_slot_offset(1, 0),
            h.cache_data_offset + 3 * 1024
        );
    }

    #[test]
    fn header_peer_slot_offset_is_row_major() {
        let l = compute_dsm_layout(3, 1, 64 * 1024, 0, 0, 0, 64 * 1024, 1).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        let aligned = h.peer_queue_bytes;
        assert_eq!(h.peer_slot_offset(0, 0, 0), h.peer_mesh_offset);
        assert_eq!(h.peer_slot_offset(0, 0, 1), h.peer_mesh_offset + aligned);
        // (mesh=0, prod=1, cons=0): row 1, col 0 — n_workers slots in.
        assert_eq!(
            h.peer_slot_offset(0, 1, 0),
            h.peer_mesh_offset + 3 * aligned
        );
        // (mesh=0, prod=2, cons=2): last slot.
        assert_eq!(
            h.peer_slot_offset(0, 2, 2),
            h.peer_mesh_offset + 8 * aligned
        );
    }

    #[test]
    fn header_peer_slot_offset_across_multiple_meshes() {
        // K=3 meshes, each n_workers=2 → mesh size = 4 slots.
        let l = compute_dsm_layout(2, 1, 64 * 1024, 0, 0, 0, 64 * 1024, 3).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        let aligned = h.peer_queue_bytes;
        let mesh_size = 4 * aligned;
        // Mesh 0 starts at peer_mesh_offset.
        assert_eq!(h.peer_slot_offset(0, 0, 0), h.peer_mesh_offset);
        // Mesh 1 starts mesh_size bytes later.
        assert_eq!(h.peer_slot_offset(1, 0, 0), h.peer_mesh_offset + mesh_size);
        // Mesh 2 starts 2*mesh_size bytes later.
        assert_eq!(
            h.peer_slot_offset(2, 0, 0),
            h.peer_mesh_offset + 2 * mesh_size
        );
        // Within mesh 1, slot (1, 1) is the last slot of that mesh.
        assert_eq!(
            h.peer_slot_offset(1, 1, 1),
            h.peer_mesh_offset + mesh_size + 3 * aligned
        );
        // region_total covers all 3 meshes.
        assert_eq!(h.region_total, h.peer_mesh_offset + 3 * mesh_size);
    }

    #[test]
    fn header_validate_accepts_self() {
        let l = compute_dsm_layout(2, 2, 64 * 1024, 0, 0, 0, 0, 0).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        assert!(h.validate(l.region_total as u64).is_ok());
    }

    #[test]
    fn header_validate_rejects_size_mismatch() {
        let l = compute_dsm_layout(2, 2, 64 * 1024, 0, 0, 0, 0, 0).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        assert!(h.validate(l.region_total as u64 + 1).is_err());
    }

    #[test]
    fn header_validate_accepts_with_peer_mesh() {
        let l = compute_dsm_layout(3, 1, 64 * 1024, 0, 0, 0, 64 * 1024, 1).unwrap();
        let h = MppDsmHeader::from_layout(&l);
        assert!(h.validate(l.region_total as u64).is_ok());
    }
}
