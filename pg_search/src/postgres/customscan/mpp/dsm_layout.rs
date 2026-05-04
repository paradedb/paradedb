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

#![allow(dead_code)]
//! N×K shm_mq mesh layout for the coordinator/worker MPP architecture.
//!
//! Each MPP query carries one *cut* (e.g. the post-Partial gather for
//! `ScalarAggOnBinaryJoin`) or several (e.g. left-shuffle + right-shuffle +
//! post-aggregate-gather for `GroupByAggOnBinaryJoin`). For each cut, we
//! allocate an `N × K` array of shm_mq slots:
//!
//! - `N` = number of producer-side workers (== `mpp_worker_count`,
//!   including the leader-as-worker-0 contribution).
//! - `K` = number of consumer-side partitions on the leader for that cut.
//!   For scalar-agg final-gather, `K = 1` (everything coalesces into one
//!   partition that the leader's `AggregateExec(FinalPartitioned)` reads).
//!   For group-by-agg post-aggregate, `K = N` (one consumer partition per
//!   participant so PG's `Gather` can fan out the resulting groups).
//!
//! The DSM region carries every cut's queues back-to-back, sized via
//! [`MppDsmLayout::dsm_bytes`]. Slot indexing is straightforward:
//!
//! ```text
//!   slot(cut_id, worker, partition) =
//!     base[cut_id]
//!     + worker * K[cut_id] * aligned_queue_bytes
//!     + partition * aligned_queue_bytes
//! ```
//!
//! Self-edges (worker `i` writing to a partition the leader-as-worker-0
//! also feeds) ARE included — uniformly through shm_mq even when
//! producer/consumer live in the same process. This keeps the topology
//! symmetric and avoids a separate "loopback" path.
//!
//! Compare to the now-deleted [`super::mesh::MeshLayout`] (`N × (N-1)` peer-
//! to-peer with self-edges elided), which is incompatible with the
//! coordinator/worker topology.

use crate::postgres::customscan::mpp::mesh::aligned_queue_bytes;

/// Single-cut size: `N × K` queues.
#[derive(Debug, Clone, Copy)]
pub struct CutLayout {
    /// Number of producer workers (including leader-as-worker-0).
    pub n_workers: u32,
    /// Number of consumer partitions on the leader for this cut.
    pub k_partitions: u32,
}

impl CutLayout {
    pub fn new(n_workers: u32, k_partitions: u32) -> Self {
        Self {
            n_workers,
            k_partitions,
        }
    }

    /// Total queue count for this cut: `N × K`.
    pub fn queue_count(&self) -> usize {
        (self.n_workers as usize) * (self.k_partitions as usize)
    }

    /// Linear queue index for the `(worker, partition)` pair within this
    /// cut's queue array.
    pub fn slot_index(&self, worker: u32, partition: u32) -> usize {
        debug_assert!(worker < self.n_workers);
        debug_assert!(partition < self.k_partitions);
        (worker as usize) * (self.k_partitions as usize) + (partition as usize)
    }
}

/// Multi-cut layout for one MPP query's DSM region.
#[derive(Debug, Clone)]
pub struct MppDsmLayout {
    /// One [`CutLayout`] per cut, in walker emission order (bottom-up).
    pub cuts: Vec<CutLayout>,
    /// Aligned per-queue size in bytes (MAXALIGN-rounded). Shared across all
    /// cuts so slot indexing stays a simple multiply.
    pub queue_bytes: usize,
}

impl MppDsmLayout {
    pub fn new(cuts: Vec<CutLayout>, queue_bytes_request: usize) -> Self {
        Self {
            cuts,
            queue_bytes: aligned_queue_bytes(queue_bytes_request),
        }
    }

    /// Total bytes needed for every cut's queue array, end-to-end. Caller
    /// adds the DSM header bytes on top of this. Returns `None` on
    /// pathological overflow (e.g. `u32::MAX` workers).
    pub fn dsm_bytes_checked(&self) -> Option<usize> {
        let mut total: usize = 0;
        for cut in &self.cuts {
            let cut_bytes = cut.queue_count().checked_mul(self.queue_bytes)?;
            total = total.checked_add(cut_bytes)?;
        }
        Some(total)
    }

    /// Byte offset (relative to the start of the queue area, i.e. excluding
    /// any DSM header) where `cut_id`'s queue array begins.
    pub fn cut_offset_bytes(&self, cut_id: usize) -> Option<usize> {
        if cut_id >= self.cuts.len() {
            return None;
        }
        let mut off: usize = 0;
        for cut in &self.cuts[..cut_id] {
            off = off.checked_add(cut.queue_count().checked_mul(self.queue_bytes)?)?;
        }
        Some(off)
    }

    /// Byte offset of the `(cut_id, worker, partition)` slot within the
    /// queue area. Returns `None` if any index is out of range.
    pub fn slot_offset_bytes(&self, cut_id: usize, worker: u32, partition: u32) -> Option<usize> {
        let cut = self.cuts.get(cut_id)?;
        if worker >= cut.n_workers || partition >= cut.k_partitions {
            return None;
        }
        let cut_off = self.cut_offset_bytes(cut_id)?;
        let slot = cut.slot_index(worker, partition);
        cut_off.checked_add(slot.checked_mul(self.queue_bytes)?)
    }

    /// Total queue count across every cut.
    pub fn total_queue_count(&self) -> usize {
        self.cuts.iter().map(|c| c.queue_count()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cut_layout_queue_count_is_product() {
        assert_eq!(CutLayout::new(4, 1).queue_count(), 4);
        assert_eq!(CutLayout::new(4, 4).queue_count(), 16);
        assert_eq!(CutLayout::new(3, 5).queue_count(), 15);
    }

    #[test]
    fn cut_layout_slot_index_is_row_major() {
        let cut = CutLayout::new(3, 4);
        // (0, 0) = 0, (0, 1) = 1, (1, 0) = 4, (2, 3) = 11
        assert_eq!(cut.slot_index(0, 0), 0);
        assert_eq!(cut.slot_index(0, 1), 1);
        assert_eq!(cut.slot_index(1, 0), 4);
        assert_eq!(cut.slot_index(2, 3), 11);
    }

    #[test]
    fn cut_layout_slot_index_is_injective() {
        let cut = CutLayout::new(4, 5);
        let mut seen = std::collections::HashSet::new();
        for w in 0..cut.n_workers {
            for p in 0..cut.k_partitions {
                let s = cut.slot_index(w, p);
                assert!(s < cut.queue_count());
                assert!(seen.insert(s), "duplicate slot {s} for ({w}, {p})");
            }
        }
        assert_eq!(seen.len(), cut.queue_count());
    }

    #[test]
    fn dsm_layout_dsm_bytes_sums_cut_bytes() {
        let layout = MppDsmLayout::new(vec![CutLayout::new(4, 1), CutLayout::new(4, 4)], 64 * 1024);
        let aligned = layout.queue_bytes;
        assert_eq!(layout.dsm_bytes_checked().unwrap(), (4 + 4 * 4) * aligned);
    }

    #[test]
    fn dsm_layout_cut_offset_advances_by_previous_cut_bytes() {
        let layout = MppDsmLayout::new(vec![CutLayout::new(4, 1), CutLayout::new(4, 4)], 64 * 1024);
        let aligned = layout.queue_bytes;
        assert_eq!(layout.cut_offset_bytes(0).unwrap(), 0);
        assert_eq!(layout.cut_offset_bytes(1).unwrap(), 4 * aligned);
        assert!(layout.cut_offset_bytes(2).is_none());
    }

    #[test]
    fn dsm_layout_slot_offset_combines_cut_offset_and_slot_index() {
        let layout = MppDsmLayout::new(vec![CutLayout::new(2, 1), CutLayout::new(2, 2)], 64 * 1024);
        let aligned = layout.queue_bytes;
        // Cut 0 has 2 queues at offsets 0 and aligned.
        assert_eq!(layout.slot_offset_bytes(0, 0, 0).unwrap(), 0);
        assert_eq!(layout.slot_offset_bytes(0, 1, 0).unwrap(), aligned);
        // Cut 1 starts at offset 2*aligned and has 4 queues.
        assert_eq!(layout.slot_offset_bytes(1, 0, 0).unwrap(), 2 * aligned);
        assert_eq!(layout.slot_offset_bytes(1, 0, 1).unwrap(), 3 * aligned);
        assert_eq!(layout.slot_offset_bytes(1, 1, 0).unwrap(), 4 * aligned);
        assert_eq!(layout.slot_offset_bytes(1, 1, 1).unwrap(), 5 * aligned);
    }

    #[test]
    fn dsm_layout_slot_offset_rejects_out_of_range() {
        let layout = MppDsmLayout::new(vec![CutLayout::new(2, 2)], 64 * 1024);
        assert!(layout.slot_offset_bytes(0, 2, 0).is_none()); // worker OOB
        assert!(layout.slot_offset_bytes(0, 0, 2).is_none()); // partition OOB
        assert!(layout.slot_offset_bytes(1, 0, 0).is_none()); // cut OOB
    }

    #[test]
    fn dsm_layout_total_queue_count_sums_across_cuts() {
        let layout = MppDsmLayout::new(
            vec![
                CutLayout::new(4, 1), // 4
                CutLayout::new(4, 4), // 16
                CutLayout::new(2, 3), // 6
            ],
            64 * 1024,
        );
        assert_eq!(layout.total_queue_count(), 4 + 16 + 6);
    }

    #[test]
    fn dsm_layout_aligns_queue_bytes_request() {
        // queue_bytes_request not aligned; layout.queue_bytes is.
        let layout = MppDsmLayout::new(vec![CutLayout::new(2, 1)], 4097);
        assert_eq!(layout.queue_bytes, aligned_queue_bytes(4097));
        assert!(layout.queue_bytes <= 4097);
        assert!(layout.queue_bytes > 0);
    }
}
