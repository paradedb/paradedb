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

//! MPP queue mesh: DSM layout for the directed N×(N-1) shm_mq mesh.
//!
//! Every participant has `N-1` inbound and `N-1` outbound queues — one per
//! peer. We pack them into a single DSM region as a compact 1D array of
//! `N*(N-1)` shm_mq regions (self-edges are elided; the self-partition bypasses
//! the mesh entirely).
//!
//! ```text
//! edge(src, dst)  where src != dst
//!   slot = src * (N-1) + compact_dst
//!   compact_dst = if dst < src { dst } else { dst - 1 }
//! ```
//!
//! Each slot holds exactly one shm_mq region of `MAXALIGN_DOWN(queue_bytes)`
//! bytes (Postgres requires maxalign for shm_mq). The total DSM payload for
//! queues is therefore `N*(N-1) * aligned_queue_bytes`.
//!
//! The shm_mq-backed `BatchChannelSender`/`BatchChannelReceiver` implementations
//! are in this file so the full production transport stack lives next to the
//! layout math.

#![allow(dead_code)]

use pgrx::pg_sys;

use crate::mpp_log;
use crate::parallel_worker::mqueue::{
    MessageQueueReceiver, MessageQueueRecvError, MessageQueueSendError, MessageQueueSender,
};
use crate::postgres::customscan::mpp::transport::{
    BatchChannelReceiver, BatchChannelSender, RecvOutcome,
};
use datafusion::common::DataFusionError;

/// Compute the linear slot index for a directed edge `src -> dst` in an
/// N-participant mesh. `src == dst` is invalid (self-partition bypasses the
/// mesh).
#[inline]
pub fn edge_slot(src: u32, dst: u32, n: u32) -> usize {
    debug_assert_ne!(src, dst, "self-edges are not in the mesh");
    debug_assert!(src < n);
    debug_assert!(dst < n);
    let compact_dst = if dst < src { dst } else { dst - 1 };
    (src as usize) * (n as usize - 1) + (compact_dst as usize)
}

/// Total number of directed edges in the mesh: `N*(N-1)`.
#[inline]
pub fn num_edges(n: u32) -> usize {
    (n as usize) * (n as usize).saturating_sub(1)
}

/// Compute the aligned per-queue size Postgres will accept (MAXALIGN_DOWN of
/// the user request). Callers pre-size one shm_mq slot to this; every slot is
/// the same size so slot indexing is a simple multiply.
#[inline]
pub fn aligned_queue_bytes(queue_bytes: usize) -> usize {
    const MAXIMUM_ALIGNOF: usize = pg_sys::MAXIMUM_ALIGNOF as usize;
    queue_bytes & !(MAXIMUM_ALIGNOF - 1)
}

/// Layout description for the MPP mesh DSM region. Consumed by the custom
/// scan's `estimate_dsm_custom_scan` / `initialize_dsm_custom_scan` hooks and
/// by the shuffle operators — a pure compile-time data shape that callers can
/// reason about without needing a running Postgres.
#[derive(Debug, Clone, Copy)]
pub struct MeshLayout {
    pub total_participants: u32,
    pub queue_bytes: usize,
}

impl MeshLayout {
    pub fn new(total_participants: u32, queue_bytes: usize) -> Self {
        Self {
            total_participants,
            queue_bytes,
        }
    }

    /// Overflow-checked DSM-byte computation. Returns `None` if
    /// `num_edges * aligned_bytes` overflows `usize`. `compute_dsm_layout`
    /// uses this so a pathological caller (`N=u32::MAX`) fails cleanly
    /// instead of wrapping.
    pub fn dsm_queue_bytes_checked(&self) -> Option<usize> {
        let edges = num_edges(self.total_participants);
        edges.checked_mul(aligned_queue_bytes(self.queue_bytes))
    }

    /// Aligned per-queue size.
    pub fn aligned_queue_bytes(&self) -> usize {
        aligned_queue_bytes(self.queue_bytes)
    }
}

/// shm_mq-backed `BatchChannelSender`. Wraps the existing MessageQueueSender
/// so we reuse its detach-on-drop behavior and the pgrx-safe FFI.
///
/// ## Thread-safety contract
///
/// `BatchChannelSender: Send`, so this type must be movable across threads.
/// The `unsafe impl Send` below is only safe when the sender is **used** from
/// a thread that owns a valid `PGPROC` (i.e., the main backend thread or a
/// dedicated parallel-worker backend). `MessageQueueSender::send` calls
/// `shm_mq_send(nowait=false)`, which internally uses `WaitLatch` and
/// `CHECK_FOR_INTERRUPTS` — both are process-global Postgres primitives that
/// are not thread-safe off a backend thread.
///
/// MPP's `HashRepartitionExec::execute` always runs inside DataFusion on the
/// main backend thread, so production use always satisfies this contract.
///
/// In debug builds the thread id captured at [`ShmMqSender::attach`] is
/// asserted on every `send_bytes` via `debug_assert_eq!` — a regression that
/// moves the sender to a non-backend thread will panic instead of silently
/// invoking unsafe PG primitives off-thread. The field is kept in release
/// builds too (8 bytes per sender) so the `debug_assert_eq!` call sites can
/// stay clean of `#[cfg(debug_assertions)]` annotations; the assertion macro
/// itself self-disables in release.
pub struct ShmMqSender {
    inner: MessageQueueSender,
    attach_thread: std::thread::ThreadId,
}

// SAFETY: see struct doc. Production callers keep the sender on the main
// backend thread; the debug_assertions guard in `send_bytes` catches
// violations in CI.
unsafe impl Send for ShmMqSender {}

impl ShmMqSender {
    /// # Safety
    /// - `seg` must be a valid `dsm_segment*`
    /// - `mq` must point to a shm_mq region that has been `shm_mq_create`'d at
    ///   its address with the expected size and has had `shm_mq_set_receiver`
    ///   called by the peer.
    pub unsafe fn attach(seg: *mut pg_sys::dsm_segment, mq: *mut pg_sys::shm_mq) -> Self {
        unsafe {
            Self {
                inner: MessageQueueSender::new(seg, mq),
                attach_thread: std::thread::current().id(),
            }
        }
    }
}

impl BatchChannelSender for ShmMqSender {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), DataFusionError> {
        debug_assert_eq!(
            std::thread::current().id(),
            self.attach_thread,
            "ShmMqSender::send_bytes called from a thread other than the one that \
             called attach(); shm_mq_send uses WaitLatch + CHECK_FOR_INTERRUPTS which \
             are process-global Postgres primitives and unsafe off the backend thread."
        );
        self.inner.send(bytes).map_err(|e| match e {
            MessageQueueSendError::Detached => {
                DataFusionError::Execution("mpp: shm_mq sender detached".into())
            }
            MessageQueueSendError::WouldBlock => {
                // `send` is blocking; WouldBlock means `try_send`-style path
                // was exercised. Treated as a transport error.
                DataFusionError::Execution("mpp: shm_mq send would block".into())
            }
            MessageQueueSendError::Unknown(code) => {
                mpp_log!("mpp: shm_mq send unknown code {code}");
                DataFusionError::Execution(format!("mpp: shm_mq send unknown code {code}"))
            }
        })
    }

    fn try_send_bytes(&self, bytes: &[u8]) -> Result<bool, DataFusionError> {
        debug_assert_eq!(
            std::thread::current().id(),
            self.attach_thread,
            "ShmMqSender::try_send_bytes called from a thread other than the one that \
             called attach(); shm_mq_send touches process-global Postgres primitives \
             and is unsafe off the backend thread."
        );
        match self.inner.try_send(bytes) {
            Ok(Some(())) => Ok(true),
            Ok(None) => Ok(false),
            Err(MessageQueueSendError::Detached) => Err(DataFusionError::Execution(
                "mpp: shm_mq sender detached".into(),
            )),
            Err(MessageQueueSendError::WouldBlock) => {
                // Shouldn't happen — `try_send` maps WOULD_BLOCK to Ok(None)
                // in the Rust wrapper. Treat as non-fatal retry.
                Ok(false)
            }
            Err(MessageQueueSendError::Unknown(code)) => {
                mpp_log!("mpp: shm_mq try_send unknown code {code}");
                Err(DataFusionError::Execution(format!(
                    "mpp: shm_mq try_send unknown code {code}"
                )))
            }
        }
    }
}

/// shm_mq-backed `BatchChannelReceiver`. The caller creates the shm_mq via
/// `MessageQueueReceiver::new` during DSM initialization.
///
/// The inner [`MessageQueueReceiver`] holds a `NonNull<shm_mq_handle>`, which
/// makes the struct `!Send` by default. The drain thread takes ownership of
/// the receiver for the query's lifetime, and we only call
/// `shm_mq_receive(nowait=true)` from that thread — Postgres's nowait path
/// does not touch thread-unsafe state (no `WaitLatch`, no
/// `CHECK_FOR_INTERRUPTS`). Attach and all subsequent receives happen on the
/// same thread, so the cross-thread move at spawn time is safe.
pub struct ShmMqReceiver {
    inner: MessageQueueReceiver,
}

// SAFETY: see struct doc. `NonNull<shm_mq_handle>` is a pointer into DSM that
// is valid across threads within the same backend process, and we guarantee
// single-threaded access (the drain thread) plus only `nowait=true` shm_mq
// operations — the only thread-unsafe PG paths (latch wait, CFI) are excluded.
unsafe impl Send for ShmMqReceiver {}

impl ShmMqReceiver {
    /// Attach as receiver to an *already-created* shm_mq. Used by the MPP
    /// mesh wiring where the leader calls `shm_mq_create` for every slot
    /// up front, and each participant then attaches to the slots it owns.
    /// Skips `shm_mq_create` entirely — calling create twice on the same
    /// address would re-initialize the ring-buffer header and corrupt any
    /// in-flight messages.
    ///
    /// # Safety
    /// - `mq` must point to a shm_mq that was previously initialized by
    ///   `shm_mq_create` in this or another backend of the same parallel
    ///   context.
    /// - `seg` may be NULL on workers, where `initialize_worker_custom_scan`
    ///   does not surface the DSM segment pointer. In that case,
    ///   `shm_mq_attach` skips registering an `on_dsm_detach` callback and
    ///   relies on process-exit cleanup. That's safe for parallel workers
    ///   whose process lifetime matches the query, but unsafe for long-lived
    ///   backend reuse.
    /// - No other participant has already set itself as receiver for this
    ///   queue (PG rejects a second set_receiver call).
    pub unsafe fn attach_existing(seg: *mut pg_sys::dsm_segment, mq: *mut pg_sys::shm_mq) -> Self {
        unsafe {
            pg_sys::shm_mq_set_receiver(mq, pg_sys::MyProc);
            let handle = pg_sys::shm_mq_attach(mq, seg, std::ptr::null_mut());
            Self {
                inner: MessageQueueReceiver::from_raw_handle(handle),
            }
        }
    }
}

impl BatchChannelReceiver for ShmMqReceiver {
    fn try_recv(&self) -> RecvOutcome {
        match self.inner.try_recv() {
            Ok(Some(bytes)) => RecvOutcome::Bytes(bytes),
            Ok(None) => RecvOutcome::Empty,
            Err(MessageQueueRecvError::Detached) => RecvOutcome::Detached,
            Err(MessageQueueRecvError::WouldBlock) => RecvOutcome::Empty,
            Err(MessageQueueRecvError::Unknown(code)) => {
                // Collapse to Detached so the drain thread observes EOF for
                // this source and the query surfaces the failure upstream.
                // Log first so the underlying code is visible in benchmark
                // logs when `paradedb.mpp_debug` is on.
                mpp_log!("mpp: shm_mq recv unknown code {code}, treating as detached");
                RecvOutcome::Detached
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_slot_packs_without_self_edges() {
        // N=4: 12 edges
        //   0->1=0, 0->2=1, 0->3=2
        //   1->0=3, 1->2=4, 1->3=5
        //   2->0=6, 2->1=7, 2->3=8
        //   3->0=9, 3->1=10, 3->2=11
        let n = 4;
        assert_eq!(edge_slot(0, 1, n), 0);
        assert_eq!(edge_slot(0, 2, n), 1);
        assert_eq!(edge_slot(0, 3, n), 2);
        assert_eq!(edge_slot(1, 0, n), 3);
        assert_eq!(edge_slot(1, 2, n), 4);
        assert_eq!(edge_slot(1, 3, n), 5);
        assert_eq!(edge_slot(2, 0, n), 6);
        assert_eq!(edge_slot(2, 1, n), 7);
        assert_eq!(edge_slot(2, 3, n), 8);
        assert_eq!(edge_slot(3, 0, n), 9);
        assert_eq!(edge_slot(3, 1, n), 10);
        assert_eq!(edge_slot(3, 2, n), 11);
    }

    #[test]
    fn edge_slot_n2() {
        // N=2: 2 edges (0->1, 1->0)
        assert_eq!(edge_slot(0, 1, 2), 0);
        assert_eq!(edge_slot(1, 0, 2), 1);
    }

    #[test]
    fn edge_slot_is_injective() {
        // Every (src, dst) pair maps to a unique slot in [0, N*(N-1))
        for n in [2u32, 3, 4, 5, 8] {
            let mut seen = std::collections::HashSet::new();
            for src in 0..n {
                for dst in 0..n {
                    if src == dst {
                        continue;
                    }
                    let slot = edge_slot(src, dst, n);
                    assert!(
                        slot < num_edges(n),
                        "slot {slot} out of range for n={n} edge {src}->{dst}"
                    );
                    assert!(
                        seen.insert(slot),
                        "duplicate slot {slot} for n={n} edge {src}->{dst}"
                    );
                }
            }
            assert_eq!(seen.len(), num_edges(n));
        }
    }

    #[test]
    fn num_edges_matches_formula() {
        assert_eq!(num_edges(0), 0);
        assert_eq!(num_edges(1), 0);
        assert_eq!(num_edges(2), 2);
        assert_eq!(num_edges(3), 6);
        assert_eq!(num_edges(4), 12);
        assert_eq!(num_edges(8), 56);
    }

    #[test]
    fn aligned_queue_bytes_rounds_down_to_maxalign() {
        let maxalign = pg_sys::MAXIMUM_ALIGNOF as usize;
        // Must be <= input and aligned
        for req in [0, 1, 7, 8, 15, 16, 1024, 1025, 1_048_576, 8 * 1024 * 1024] {
            let got = aligned_queue_bytes(req);
            assert!(got <= req);
            assert_eq!(got % maxalign, 0, "not aligned: {got} for req {req}");
            // Round-down invariant: req - got < maxalign
            assert!(req - got < maxalign);
        }
    }

    #[test]
    fn mesh_layout_offsets_are_contiguous_and_non_overlapping() {
        let layout = MeshLayout::new(4, 64 * 1024);
        let slot_bytes = layout.aligned_queue_bytes();
        let n = layout.total_participants;
        let mut offsets = Vec::new();
        for src in 0..n {
            for dst in 0..n {
                if src == dst {
                    continue;
                }
                offsets.push(edge_slot(src, dst, n) * slot_bytes);
            }
        }
        offsets.sort();
        for (i, off) in offsets.iter().enumerate() {
            assert_eq!(*off, i * slot_bytes);
        }
        assert_eq!(
            layout.dsm_queue_bytes_checked().unwrap(),
            num_edges(n) * slot_bytes
        );
    }
}
