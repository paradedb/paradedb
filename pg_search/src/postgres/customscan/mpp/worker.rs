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

//! MPP worker lifecycle and DSM coordination.
//!
//! The DSM region laid out by the leader during `initialize_dsm_custom_scan`
//! and read by workers during `initialize_worker_custom_scan` looks like:
//!
//! ```text
//! +---------------------------------------------------------+
//! | MppDsmHeader (repr C, fixed size, MAXALIGN-padded)      |
//! +---------------------------------------------------------+
//! | plan_bytes: Vec<u8> copied verbatim (length from header) |
//! +---------------------------------------------------------+
//! | padding to MAXALIGN                                      |
//! +---------------------------------------------------------+
//! | shm_mq mesh region: N*(N-1) * aligned_queue_bytes        |
//! +---------------------------------------------------------+
//! ```
//!
//! The header is deliberately a C-repr POD struct so both the leader and
//! workers can read it via plain pointer arithmetic without Rust ownership
//! shenanigans. The plan bytes are then `bincode`-decoded into
//! [`MppPlanBroadcast`] (see `session.rs`).
//!
//! This module deliberately does NOT wrap `pg_sys::shm_mq_create` /
//! `shm_mq_attach`; those calls are driven by the custom scan's
//! `initialize_dsm_custom_scan` / `initialize_worker_custom_scan` callbacks,
//! feeding back into [`ShmMqSender`](super::mesh::ShmMqSender) and
//! [`ShmMqReceiver`](super::mesh::ShmMqReceiver). Here we only own the
//! sizing/offset math and the header layout.

#![allow(dead_code)]

use crate::postgres::customscan::mpp::mesh::{MeshLayout, ShmMqReceiver, ShmMqSender};
use crate::postgres::customscan::mpp::transport::{MppReceiver, MppSender};
use pgrx::pg_sys;
use std::mem::size_of;

/// Magic number stamped at the head of an MPP DSM region. Distinct from any
/// of the other Postgres DSM magic numbers in the crate; workers assert this
/// matches on attach to catch catastrophic layout mismatches (e.g., the leader
/// running a different build than the worker).
pub const MPP_DSM_MAGIC: u32 = 0x4D50_5053; // "MPPS" in ASCII

/// Current on-DSM header version. Bumped when the header layout or
/// interpretation changes. Independent of `MPP_PLAN_BROADCAST_VERSION`, which
/// versions the embedded plan bytes.
pub const MPP_DSM_HEADER_VERSION: u32 = 1;

/// Compile-time build hash: a 64-bit FNV-1a over the crate version string. On
/// a cluster where the leader and its workers all load the same pg_search .so,
/// this is identical across participants by construction. Its purpose is to
/// catch the exotic case of a newer leader somehow ending up paired with an
/// older worker image (rolling reload, stale postmaster). Cheap to add now,
/// annoying to retrofit after workers are attaching.
pub const MPP_BUILD_HASH: u64 = fnv1a_64(env!("CARGO_PKG_VERSION").as_bytes());

const fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        i += 1;
    }
    hash
}

/// C-repr header stamped at offset 0 of the MPP DSM region.
///
/// Field ordering is fixed: four `u32`s (16 bytes, naturally aligned), then
/// six `u64`s (48 bytes, naturally aligned). Total size = 64 bytes, **no
/// internal padding**, on every supported target (x86_64 / aarch64 / Linux /
/// macOS / Windows). Workers read individual fields by name, not by offset,
/// but the no-padding guarantee means the whole struct can be memcpy'd
/// between processes without surprises. If a future change reorders fields
/// or introduces a mixed-width sequence that induces padding, bump
/// `MPP_DSM_HEADER_VERSION` so stale workers reject the new layout at
/// `validate()` rather than read garbage. The
/// `header_size_has_no_padding` test locks this in.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MppDsmHeader {
    /// Sanity-check magic. Workers panic on mismatch.
    pub magic: u32,
    /// Header layout version.
    pub header_version: u32,
    /// Total participants (leader + workers).
    pub total_participants: u32,
    /// Number of independent shuffle meshes packed into this DSM region.
    /// Each mesh occupies `num_edges(total_participants) * aligned_queue_bytes`
    /// contiguous bytes starting at `mesh_offset + i * per_mesh_bytes`.
    /// All meshes share the same `aligned_queue_bytes`, so per-mesh offsets
    /// are derivable without a separate offset table.
    pub num_meshes: u32,
    /// Compile-time FNV-1a over `CARGO_PKG_VERSION`. Workers reject
    /// mismatches to catch the rolling-reload / postmaster-restart edge
    /// case where a newer leader ends up paired with a stale worker image.
    pub build_hash: u64,
    /// Per-edge shm_mq slot size in bytes (MAXALIGN_DOWN of the user
    /// request). Shared by every mesh; a future extension for heterogeneous
    /// queue sizes would need a per-mesh sidecar and a header-version bump.
    pub aligned_queue_bytes: u64,
    /// Byte offset from the DSM base to the plan bytes.
    pub plan_offset: u64,
    /// Plan byte count.
    pub plan_len: u64,
    /// Byte offset from the DSM base to the start of the first mesh's queue
    /// array. Mesh `i`'s array starts at
    /// `mesh_offset + i * num_edges(total_participants) * aligned_queue_bytes`.
    pub mesh_offset: u64,
    /// Total DSM region size (header + plan + meshes with MAXALIGN padding).
    /// `validate(region_total)` bounds-checks every offset against this so
    /// a corrupt header that still matches magic+version cannot redirect
    /// the worker to read arbitrary DSM memory.
    pub region_total: u64,
}

impl MppDsmHeader {
    /// The single constructor: the header's offsets cannot disagree with the
    /// layout the leader actually wrote, because both come from the same
    /// `DsmLayout`. There is no free-form `new(plan_offset, plan_len, …)` —
    /// that API let callers build headers that lied about the region.
    pub fn from_layout(mesh: &MeshLayout, dsm: &DsmLayout) -> Self {
        Self {
            magic: MPP_DSM_MAGIC,
            header_version: MPP_DSM_HEADER_VERSION,
            total_participants: mesh.total_participants,
            num_meshes: dsm.num_meshes,
            build_hash: MPP_BUILD_HASH,
            aligned_queue_bytes: mesh.aligned_queue_bytes() as u64,
            plan_offset: dsm.plan_offset as u64,
            plan_len: dsm.plan_len as u64,
            mesh_offset: dsm.mesh_offset as u64,
            region_total: dsm.total as u64,
        }
    }

    /// Byte offset from the DSM base to mesh index `mesh_idx`'s queue array.
    pub fn mesh_base_offset(&self, mesh_idx: u32) -> u64 {
        debug_assert!(mesh_idx < self.num_meshes);
        let n = self.total_participants as u64;
        let edges = n * n.saturating_sub(1);
        self.mesh_offset + (mesh_idx as u64) * edges * self.aligned_queue_bytes
    }

    /// Validate a header read out of DSM. `region_total` is the size of the
    /// DSM region the caller just attached to; the header's own offsets are
    /// bounds-checked against it so a corrupt header that still happens to
    /// match magic + version + build hash cannot redirect the worker to read
    /// arbitrary DSM memory.
    pub fn validate(&self, region_total: u64) -> Result<(), &'static str> {
        if self.magic != MPP_DSM_MAGIC {
            return Err("mpp: DSM header magic mismatch");
        }
        if self.header_version != MPP_DSM_HEADER_VERSION {
            return Err("mpp: DSM header version mismatch");
        }
        if self.build_hash != MPP_BUILD_HASH {
            return Err("mpp: DSM build-hash mismatch (leader/worker binary skew?)");
        }
        if self.total_participants == 0 {
            return Err("mpp: total_participants must be > 0");
        }
        if self.region_total != region_total {
            return Err("mpp: DSM region_total in header disagrees with attached size");
        }
        match self.plan_offset.checked_add(self.plan_len) {
            None => return Err("mpp: plan_offset + plan_len overflows"),
            Some(end) if end > self.mesh_offset => {
                return Err("mpp: plan would overlap mesh");
            }
            _ => {}
        }
        if self.mesh_offset > region_total {
            return Err("mpp: mesh_offset past end of region");
        }
        Ok(())
    }
}

/// Absolute maximum DSM region an MPP query will allocate. 16 GiB is two
/// orders of magnitude beyond any realistic workload (reference attempt used
/// 8 MB per queue × 12 edges = 96 MB at N=4). The cap makes `compute_dsm_layout`
/// fail early on a pathologically oversized request instead of asking PG to
/// allocate ~`usize::MAX` bytes.
pub const MPP_DSM_MAX_BYTES: usize = 16 * 1024 * 1024 * 1024;

/// Pure-math helper: compute the total DSM bytes needed for an MPP region
/// carrying `plan_len` plan bytes and `num_meshes` independent meshes that
/// each share the same [`MeshLayout`].
///
/// Layout: `header | pad | plan | pad | mesh_0 | mesh_1 | … | mesh_{N-1}`.
///
/// All meshes share `aligned_queue_bytes` so per-mesh offsets are derivable
/// from `(mesh_offset, aligned_queue_bytes, total_participants)` without a
/// sidecar offset table — see [`MppDsmHeader::mesh_base_offset`].
///
/// Fails (instead of panicking or silently overflowing) on any arithmetic
/// overflow and on regions exceeding [`MPP_DSM_MAX_BYTES`]. The caller is
/// expected to `ereport(ERROR)` with the returned string.
pub fn compute_dsm_layout(
    layout: &MeshLayout,
    num_meshes: u32,
    plan_len: usize,
) -> Result<DsmLayout, &'static str> {
    if layout.total_participants == 0 {
        return Err("mpp: total_participants must be > 0");
    }
    if num_meshes == 0 {
        return Err("mpp: num_meshes must be > 0");
    }
    let header_size = size_of::<MppDsmHeader>();
    let after_header =
        align_up_maxalign_checked(header_size).ok_or("mpp: header alignment overflowed usize")?;
    let plan_offset = after_header;
    let plan_end = plan_offset
        .checked_add(plan_len)
        .ok_or("mpp: plan_offset + plan_len overflowed usize")?;
    let after_plan =
        align_up_maxalign_checked(plan_end).ok_or("mpp: plan alignment overflowed usize")?;
    let mesh_offset = after_plan;
    let per_mesh_bytes = layout
        .dsm_queue_bytes_checked()
        .ok_or("mpp: mesh queue bytes overflowed usize")?;
    let mesh_bytes = per_mesh_bytes
        .checked_mul(num_meshes as usize)
        .ok_or("mpp: total mesh bytes overflowed usize")?;
    let total = mesh_offset
        .checked_add(mesh_bytes)
        .ok_or("mpp: region total overflowed usize")?;
    if total > MPP_DSM_MAX_BYTES {
        return Err("mpp: DSM region exceeds MPP_DSM_MAX_BYTES");
    }
    Ok(DsmLayout {
        total,
        plan_offset,
        plan_len,
        mesh_offset,
        mesh_bytes,
        num_meshes,
    })
}

/// Result of [`compute_dsm_layout`]. All offsets are byte offsets from the
/// DSM region base pointer.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DsmLayout {
    pub total: usize,
    pub plan_offset: usize,
    pub plan_len: usize,
    /// Offset to mesh 0's queue array. Mesh `i` starts at
    /// `mesh_offset + i * (num_edges(N) * aligned_queue_bytes)`.
    pub mesh_offset: usize,
    /// Total bytes for all meshes combined (sum of per-mesh queue arrays).
    pub mesh_bytes: usize,
    /// Number of independent meshes packed into this region.
    pub num_meshes: u32,
}

/// MAXALIGN_UP: round `n` up to the nearest multiple of `MAXIMUM_ALIGNOF`.
/// Overflow-checked variant — returns `None` if the alignment would push `n`
/// past `usize::MAX`. `compute_dsm_layout` uses this so pathological
/// `plan_len == usize::MAX` fails cleanly instead of wrapping.
#[inline]
fn align_up_maxalign_checked(n: usize) -> Option<usize> {
    const MA: usize = pgrx::pg_sys::MAXIMUM_ALIGNOF as usize;
    let rem = n % MA;
    if rem == 0 {
        return Some(n);
    }
    n.checked_add(MA - rem)
}

// ----------------------------------------------------------------------------
// PG-facing DSM init / attach. These functions perform raw pg_sys FFI and
// require a live Postgres backend — they compile on their own but are only
// testable via `#[pg_test]`. The custom scan wires them into its
// `estimate_dsm_custom_scan` / `initialize_dsm_custom_scan` /
// `initialize_worker_custom_scan` hooks.
// ----------------------------------------------------------------------------

/// Address of the shm_mq region for directed edge `src -> dst` in mesh
/// `mesh_idx` inside an MPP DSM region whose base pointer is `base`.
///
/// # Safety
/// - `base` must be the DSM region base that `header` was read from.
/// - `header` must have passed `validate(region_total)`.
/// - `mesh_idx` must be less than `header.num_meshes`.
/// - `src` and `dst` must be less than `header.total_participants` and not
///   equal to each other.
#[inline]
pub unsafe fn edge_address(
    base: *mut u8,
    header: &MppDsmHeader,
    mesh_idx: u32,
    src: u32,
    dst: u32,
) -> *mut u8 {
    use crate::postgres::customscan::mpp::mesh::edge_slot;
    let slot = edge_slot(src, dst, header.total_participants);
    let mesh_base = header.mesh_base_offset(mesh_idx) as usize;
    let byte_offset = mesh_base + slot * (header.aligned_queue_bytes as usize);
    unsafe { base.add(byte_offset) }
}

/// Leader-side: initialize a freshly-allocated MPP DSM region. Stamps the
/// header, copies the plan bytes into place, and calls `shm_mq_create` for
/// every directed edge in the mesh. Sets receiver for edges whose destination
/// is the leader (seat 0); leader also attaches as sender for edges it
/// produces (src == 0). Worker-to-worker edges are created but unclaimed —
/// the destination worker claims them during `attach_dsm_as_worker`.
///
/// Returns the leader's outbound senders + inbound receivers (indexed by
/// peer seat; self slot is `None`).
///
/// # Safety
/// - `base` must point to a DSM region of at least `dsm.total` bytes.
/// - Caller must not have `shm_mq_create`'d any of the mesh slots already.
/// - Callable only from the leader backend thread, since it sets
///   sender/receiver procs to `MyProc`.
pub unsafe fn initialize_dsm_as_leader(
    base: *mut u8,
    dsm: &DsmLayout,
    mesh: &MeshLayout,
    plan_bytes: &[u8],
    seg: *mut pg_sys::dsm_segment,
) -> Result<Vec<LeaderMesh>, &'static str> {
    if plan_bytes.len() != dsm.plan_len {
        return Err("mpp: plan_bytes length disagrees with DsmLayout.plan_len");
    }

    let header = MppDsmHeader::from_layout(mesh, dsm);
    unsafe {
        std::ptr::write(base as *mut MppDsmHeader, header);
        std::ptr::copy_nonoverlapping(plan_bytes.as_ptr(), base.add(dsm.plan_offset), dsm.plan_len);
    }

    let n = mesh.total_participants;
    let slot_size = mesh.aligned_queue_bytes();
    let mut meshes: Vec<LeaderMesh> = Vec::with_capacity(dsm.num_meshes as usize);

    for mesh_idx in 0..dsm.num_meshes {
        let mut outbound: Vec<Option<MppSender>> = (0..n).map(|_| None).collect();
        let mut inbound: Vec<Option<MppReceiver>> = (0..n).map(|_| None).collect();

        for src in 0..n {
            for dst in 0..n {
                if src == dst {
                    continue;
                }
                let addr = unsafe { edge_address(base, &header, mesh_idx, src, dst) };
                let mq = unsafe { pg_sys::shm_mq_create(addr as *mut _, slot_size) };
                if dst == 0 {
                    let receiver = unsafe { ShmMqReceiver::attach_existing(seg, mq) };
                    inbound[src as usize] = Some(MppReceiver::new(Box::new(receiver)));
                } else if src == 0 {
                    let sender = unsafe { ShmMqSender::attach(seg, mq) };
                    outbound[dst as usize] = Some(MppSender::new(Box::new(sender)));
                }
                // Worker-to-worker edges: created in DSM, claimed later by their
                // destination worker in `attach_dsm_as_worker`.
            }
        }

        meshes.push(LeaderMesh { outbound, inbound });
    }

    Ok(meshes)
}

/// Leader's mesh wiring after `initialize_dsm_as_leader`. `outbound[0]` and
/// `inbound[0]` are always `None` (self).
pub struct LeaderMesh {
    pub outbound: Vec<Option<MppSender>>,
    pub inbound: Vec<Option<MppReceiver>>,
}

/// Worker-side: attach to an already-initialized MPP DSM region.
///
/// # Safety
/// - `base` must point to a DSM region of at least `region_total` bytes,
///   initialized by a leader via `initialize_dsm_as_leader` and still
///   attached to this process.
/// - `participant_index` must be the seat the calling worker occupies
///   (typically `ParallelWorkerNumber + 1` since leader is seat 0).
/// - Callable only from the worker backend thread, since it sets receiver /
///   sender procs to `MyProc`.
pub unsafe fn attach_dsm_as_worker(
    base: *mut u8,
    region_total: u64,
    participant_index: u32,
    seg: *mut pg_sys::dsm_segment,
) -> Result<WorkerAttach, &'static str> {
    let header = unsafe { std::ptr::read_unaligned(base as *const MppDsmHeader) };
    header.validate(region_total)?;

    if participant_index >= header.total_participants {
        return Err("mpp: participant_index out of range");
    }
    if participant_index == 0 {
        return Err("mpp: attach_dsm_as_worker called for leader seat");
    }

    let n = header.total_participants;
    let mut meshes: Vec<WorkerMesh> = Vec::with_capacity(header.num_meshes as usize);

    for mesh_idx in 0..header.num_meshes {
        let mut outbound: Vec<Option<MppSender>> = (0..n).map(|_| None).collect();
        let mut inbound: Vec<Option<MppReceiver>> = (0..n).map(|_| None).collect();

        for src in 0..n {
            for dst in 0..n {
                if src == dst {
                    continue;
                }
                let addr = unsafe { edge_address(base, &header, mesh_idx, src, dst) };
                let mq = addr as *mut pg_sys::shm_mq;
                if dst == participant_index {
                    let receiver = unsafe { ShmMqReceiver::attach_existing(seg, mq) };
                    inbound[src as usize] = Some(MppReceiver::new(Box::new(receiver)));
                } else if src == participant_index {
                    let sender = unsafe { ShmMqSender::attach(seg, mq) };
                    outbound[dst as usize] = Some(MppSender::new(Box::new(sender)));
                }
            }
        }

        meshes.push(WorkerMesh { outbound, inbound });
    }

    let plan_bytes_ptr = unsafe { base.add(header.plan_offset as usize) };
    let plan_bytes_len = header.plan_len as usize;

    Ok(WorkerAttach {
        meshes,
        plan_bytes_ptr,
        plan_bytes_len,
    })
}

/// Worker's per-mesh wiring entry.
pub struct WorkerMesh {
    pub outbound: Vec<Option<MppSender>>,
    pub inbound: Vec<Option<MppReceiver>>,
}

/// Worker's full attach result, including one entry per mesh plus the pointer
/// to the plan bytes.
pub struct WorkerAttach {
    pub meshes: Vec<WorkerMesh>,
    /// Raw pointer to the plan bytes inside DSM. The worker must read these
    /// before detaching. Plans are write-once by the leader, so no
    /// synchronization is required for this read.
    pub plan_bytes_ptr: *const u8,
    pub plan_bytes_len: usize,
}

impl WorkerAttach {
    /// Copy the plan bytes out of DSM into an owned `Vec`. `bincode::decode`
    /// takes `&[u8]`; holding a borrow into DSM across later FFI would be a
    /// footgun. Call exactly once per worker on startup.
    ///
    /// # Safety
    /// Must be called while the DSM region is still attached to this process.
    pub unsafe fn copy_plan_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.plan_bytes_len);
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.plan_bytes_ptr,
                out.as_mut_ptr(),
                self.plan_bytes_len,
            );
            out.set_len(self.plan_bytes_len);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_header() -> (MeshLayout, DsmLayout, MppDsmHeader) {
        let mesh = MeshLayout::new(4, 8 * 1024);
        let dsm = compute_dsm_layout(&mesh, 1, 1024).unwrap();
        let header = MppDsmHeader::from_layout(&mesh, &dsm);
        (mesh, dsm, header)
    }

    #[test]
    fn header_accepts_valid_layout() {
        let (_, dsm, h) = sample_header();
        h.validate(dsm.total as u64).unwrap();
        assert_eq!(h.total_participants, 4);
        assert_eq!(h.build_hash, MPP_BUILD_HASH);
    }

    #[test]
    fn header_validate_rejects_bad_magic() {
        let (_, dsm, mut h) = sample_header();
        h.magic = 0xdead_beef;
        assert!(h.validate(dsm.total as u64).is_err());
    }

    #[test]
    fn header_validate_rejects_bad_version() {
        let (_, dsm, mut h) = sample_header();
        h.header_version = MPP_DSM_HEADER_VERSION.wrapping_add(1);
        assert!(h.validate(dsm.total as u64).is_err());
    }

    #[test]
    fn header_validate_rejects_bad_build_hash() {
        let (_, dsm, mut h) = sample_header();
        h.build_hash = MPP_BUILD_HASH.wrapping_add(1);
        let err = h.validate(dsm.total as u64).unwrap_err();
        assert!(err.contains("build-hash"));
    }

    #[test]
    fn header_validate_rejects_zero_participants() {
        let (_, dsm, mut h) = sample_header();
        h.total_participants = 0;
        assert!(h.validate(dsm.total as u64).is_err());
    }

    #[test]
    fn header_validate_rejects_region_size_mismatch() {
        let (_, dsm, h) = sample_header();
        // Attached region is smaller than header claims.
        assert!(h.validate((dsm.total - 1) as u64).is_err());
    }

    #[test]
    fn header_validate_rejects_plan_overlapping_mesh() {
        let (_, dsm, mut h) = sample_header();
        h.plan_len = (dsm.mesh_offset - dsm.plan_offset + 1) as u64;
        // Note: region_total still matches, so only the overlap check fires.
        let err = h.validate(dsm.total as u64).unwrap_err();
        assert!(err.contains("overlap mesh") || err.contains("overflows"));
    }

    #[test]
    fn compute_dsm_layout_is_consistent() {
        let layout = MeshLayout::new(4, 64 * 1024);
        let plan_len = 12_345;
        let dsm = compute_dsm_layout(&layout, 1, plan_len).unwrap();
        assert!(dsm.plan_offset >= size_of::<MppDsmHeader>());
        assert_eq!(dsm.plan_offset % pgrx::pg_sys::MAXIMUM_ALIGNOF as usize, 0);
        assert!(dsm.mesh_offset >= dsm.plan_offset + plan_len);
        assert_eq!(dsm.mesh_offset % pgrx::pg_sys::MAXIMUM_ALIGNOF as usize, 0);
        assert_eq!(dsm.mesh_bytes, layout.dsm_queue_bytes_checked().unwrap());
        assert_eq!(dsm.total, dsm.mesh_offset + dsm.mesh_bytes);
        assert_eq!(dsm.plan_len, plan_len);
        assert_eq!(dsm.num_meshes, 1);
    }

    #[test]
    fn compute_dsm_layout_scales_with_num_meshes() {
        let layout = MeshLayout::new(4, 64 * 1024);
        let one = compute_dsm_layout(&layout, 1, 0).unwrap();
        let three = compute_dsm_layout(&layout, 3, 0).unwrap();
        let queue_bytes = layout.dsm_queue_bytes_checked().unwrap();
        assert_eq!(three.mesh_bytes, 3 * queue_bytes);
        assert_eq!(three.mesh_offset, one.mesh_offset);
        assert_eq!(three.total - one.total, 2 * queue_bytes);
        assert_eq!(three.num_meshes, 3);

        let header = MppDsmHeader::from_layout(&layout, &three);
        // mesh_base_offset(0) is mesh_offset; mesh_base_offset(i) bumps by
        // num_edges * aligned_queue_bytes per index.
        assert_eq!(header.mesh_base_offset(0), header.mesh_offset);
        let per_mesh = (4u64 * 3) * header.aligned_queue_bytes;
        assert_eq!(header.mesh_base_offset(1), header.mesh_offset + per_mesh);
        assert_eq!(
            header.mesh_base_offset(2),
            header.mesh_offset + 2 * per_mesh
        );
    }

    #[test]
    fn compute_dsm_layout_handles_zero_plan() {
        let layout = MeshLayout::new(2, 8 * 1024);
        let dsm = compute_dsm_layout(&layout, 1, 0).unwrap();
        assert_eq!(dsm.plan_len, 0);
        assert_eq!(dsm.mesh_offset, dsm.plan_offset);
        assert_eq!(
            dsm.mesh_bytes,
            2 * crate::postgres::customscan::mpp::mesh::aligned_queue_bytes(8 * 1024)
        );
    }

    #[test]
    fn compute_dsm_layout_rejects_zero_participants() {
        let layout = MeshLayout::new(0, 8 * 1024);
        assert!(compute_dsm_layout(&layout, 1, 0).is_err());
    }

    #[test]
    fn compute_dsm_layout_rejects_zero_meshes() {
        let layout = MeshLayout::new(2, 8 * 1024);
        assert!(compute_dsm_layout(&layout, 0, 0).is_err());
    }

    #[test]
    fn compute_dsm_layout_rejects_overflow() {
        let layout = MeshLayout::new(2, 8 * 1024);
        assert!(compute_dsm_layout(&layout, 1, usize::MAX).is_err());
    }

    #[test]
    fn compute_dsm_layout_rejects_oversized_region() {
        // N=1000 with 16 MB queues = ~16 TB. Far past the cap.
        let layout = MeshLayout::new(1000, 16 * 1024 * 1024);
        assert!(compute_dsm_layout(&layout, 1, 0).is_err());
    }

    #[test]
    fn align_up_maxalign_rounds_correctly() {
        let ma = pgrx::pg_sys::MAXIMUM_ALIGNOF as usize;
        assert_eq!(align_up_maxalign_checked(0), Some(0));
        assert_eq!(align_up_maxalign_checked(1), Some(ma));
        assert_eq!(align_up_maxalign_checked(ma), Some(ma));
        assert_eq!(align_up_maxalign_checked(ma + 1), Some(2 * ma));
        assert_eq!(align_up_maxalign_checked(2 * ma - 1), Some(2 * ma));
        // Overflow edge case
        assert_eq!(align_up_maxalign_checked(usize::MAX), None);
    }

    #[test]
    fn header_round_trips_through_c_repr_copy() {
        // Simulate what the leader/worker actually do: write header bytes
        // into a byte buffer, then read them back via pointer cast. Uses
        // `read_unaligned` because `Vec<u8>` allocations are only guaranteed
        // to be byte-aligned; in production the DSM base pointer is
        // MAXALIGN-aligned by PG, but exercising read_unaligned here keeps
        // Miri and strict-provenance targets happy.
        let (_, dsm, original) = sample_header();

        let mut buf = vec![0u8; size_of::<MppDsmHeader>() * 2];
        unsafe {
            std::ptr::copy_nonoverlapping(
                &original as *const MppDsmHeader as *const u8,
                buf.as_mut_ptr(),
                size_of::<MppDsmHeader>(),
            );
        }
        let read_back = unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const MppDsmHeader) };
        read_back.validate(dsm.total as u64).unwrap();
        assert_eq!(read_back.magic, MPP_DSM_MAGIC);
        assert_eq!(read_back.build_hash, MPP_BUILD_HASH);
        assert_eq!(read_back.total_participants, original.total_participants);
        assert_eq!(read_back.plan_offset, original.plan_offset);
        assert_eq!(read_back.plan_len, original.plan_len);
        assert_eq!(read_back.mesh_offset, original.mesh_offset);
        assert_eq!(read_back.region_total, original.region_total);
    }

    #[test]
    fn fnv1a_matches_spec_basis() {
        // FNV-1a-64 of the empty input must be the spec's offset basis
        // (0xcbf2_9ce4_8422_2325). Pins the algorithm so a refactor
        // swapping prime/basis silently can't land.
        assert_eq!(fnv1a_64(b""), 0xcbf2_9ce4_8422_2325);
    }

    #[test]
    fn fnv1a_is_deterministic_and_differentiating() {
        assert_eq!(fnv1a_64(b"1.0.0"), fnv1a_64(b"1.0.0"));
        assert_ne!(fnv1a_64(b"1.0.0"), fnv1a_64(b"1.0.1"));
        assert_ne!(fnv1a_64(b""), fnv1a_64(b"x"));
    }

    #[test]
    fn header_size_has_no_padding() {
        // Four u32 (16) + six u64 (48) = 64 bytes with no padding on any
        // supported target. If this ever breaks, either fields were
        // reordered or a new mixed-width field was added —
        // bump MPP_DSM_HEADER_VERSION before landing.
        assert_eq!(size_of::<MppDsmHeader>(), 64);
    }
}
