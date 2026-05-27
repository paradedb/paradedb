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

//! Low-level shm_mq primitives for the MPP mesh: `ShmMqSender`,
//! `ShmMqReceiver`, and the alignment helpers used to size queue slots.
//!
//! The N×K queue layout is described in
//! [`super::dsm`](crate::postgres::customscan::mpp::dsm); this module owns
//! only the per-slot FFI wrappers.

use pgrx::pg_sys;

use crate::mpp_log;
use crate::parallel_worker::mqueue::{
    MessageQueueReceiver, MessageQueueRecvError, MessageQueueSendError, MessageQueueSender,
};
#[cfg(test)]
use crate::postgres::customscan::mpp::dsm_mpsc_ring::{self, DsmMpscRingHeader};
use crate::postgres::customscan::mpp::dsm_mpsc_ring::{
    DsmMpscReceiver, DsmMpscSender, RecvOutcome as MpscRecvOutcome, SendError as MpscSendError,
};
use crate::postgres::customscan::mpp::transport::{
    BatchChannelReceiver, BatchChannelSender, RecvOutcome,
};
use datafusion::common::DataFusionError;

/// MAXALIGN-DOWN of `queue_bytes`. Postgres requires shm_mq slot sizes to be
/// MAXALIGN-aligned; this rounds the user request down so every slot in a
/// queue array is the same size and slot indexing is a simple multiply.
#[inline]
pub(super) fn aligned_queue_bytes(queue_bytes: usize) -> usize {
    const MAXIMUM_ALIGNOF: usize = pg_sys::MAXIMUM_ALIGNOF as usize;
    queue_bytes & !(MAXIMUM_ALIGNOF - 1)
}

/// MAXALIGN-UP `n` to the next multiple of `MAXIMUM_ALIGNOF`. Returns `None`
/// on overflow. Used by [`super::dsm::compute_dsm_layout`] to keep section
/// boundaries inside the DSM region MAXALIGN-aligned.
#[inline]
pub(super) fn align_up_maxalign_checked(n: usize) -> Option<usize> {
    const MAXIMUM_ALIGNOF: usize = pg_sys::MAXIMUM_ALIGNOF as usize;
    let mask = MAXIMUM_ALIGNOF - 1;
    n.checked_add(mask).map(|x| x & !mask)
}

/// shm_mq-backed `BatchChannelSender`. Wraps `MessageQueueSender` so we reuse
/// its detach-on-drop behavior and the pgrx-safe FFI.
///
/// `unsafe impl Send` below is safe **only** when the sender is *used* from
/// a thread that owns a valid `PGPROC` — i.e., the main backend thread or a
/// dedicated parallel-worker backend. The blocking `shm_mq_send(nowait=false)`
/// path uses `WaitLatch` + `CHECK_FOR_INTERRUPTS` — both process-global
/// Postgres primitives, not thread-safe off a backend thread.
// Kept for reference and possible re-use; Phase 4b mesh multiplexing routes through
// DsmInboxSender/DsmInboxReceiver instead.
#[allow(dead_code)]
pub(crate) struct ShmMqSender {
    inner: MessageQueueSender,
    attach_thread: std::thread::ThreadId,
    /// Async serialization lock around the shm_mq handle. Held by [`MppSender::send_*_traced`]
    /// across the cooperative-drain spin to keep partial sends from interleaving — see the
    /// [`BatchChannelSender::send_lock`] doc and PG's `shm_mq_send` invariants. Multiple
    /// [`MppSender`] clones share the same `Arc<dyn BatchChannelSender>` to multiplex
    /// `(stage_id, partition)` channels onto one shm_mq, and yielding between `try_send_bytes`
    /// retries can otherwise stitch one sender's length prefix to another sender's payload and
    /// corrupt the queue.
    send_lock: tokio::sync::Mutex<()>,
}

// SAFETY: see struct doc.
unsafe impl Send for ShmMqSender {}
// SAFETY: `MppSender` ensures all `send_*` paths execute on the attach thread (a
// `debug_assert!` in `send_bytes` / `try_send_bytes` pins this), so cross-thread sharing of
// `&ShmMqSender` never actually crosses threads at the FFI boundary. The Sync bound is there so
// multiple `MppSender`s can hold `Arc<dyn BatchChannelSender>` clones of the same underlying
// queue (the multi-partition fan-out pattern) without the type system rejecting the `Arc::clone`
// at compile time.
unsafe impl Sync for ShmMqSender {}

#[allow(dead_code)]
impl ShmMqSender {
    /// # Safety
    /// - `seg` must be a valid `dsm_segment*` (or NULL on workers).
    /// - `mq` must point to a shm_mq region that has been `shm_mq_create`'d
    ///   at its address with the expected size and has had
    ///   `shm_mq_set_receiver` called by the peer.
    pub(super) unsafe fn attach(seg: *mut pg_sys::dsm_segment, mq: *mut pg_sys::shm_mq) -> Self {
        unsafe {
            Self {
                inner: MessageQueueSender::new(seg, mq),
                attach_thread: std::thread::current().id(),
                send_lock: tokio::sync::Mutex::new(()),
            }
        }
    }
}

impl BatchChannelSender for ShmMqSender {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), DataFusionError> {
        debug_assert_eq!(
            std::thread::current().id(),
            self.attach_thread,
            "ShmMqSender::send_bytes called off the attach thread"
        );
        self.inner.send(bytes).map_err(|e| match e {
            MessageQueueSendError::Detached => {
                DataFusionError::Execution("mpp: shm_mq sender detached".into())
            }
            MessageQueueSendError::WouldBlock => {
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
            "ShmMqSender::try_send_bytes called off the attach thread"
        );
        match self.inner.try_send(bytes) {
            Ok(Some(())) => Ok(true),
            Ok(None) => Ok(false),
            Err(MessageQueueSendError::Detached) => Err(DataFusionError::Execution(
                "mpp: shm_mq sender detached".into(),
            )),
            Err(MessageQueueSendError::WouldBlock) => Ok(false),
            Err(MessageQueueSendError::Unknown(code)) => {
                mpp_log!("mpp: shm_mq try_send unknown code {code}");
                Err(DataFusionError::Execution(format!(
                    "mpp: shm_mq try_send unknown code {code}"
                )))
            }
        }
    }

    fn send_lock(&self) -> &tokio::sync::Mutex<()> {
        &self.send_lock
    }
}

/// shm_mq-backed `BatchChannelReceiver`. The leader creates the shm_mq via
/// `shm_mq_create` during DSM init; this attaches as receiver to an already-
/// initialized queue.
// Kept for reference; Phase 4b mesh multiplexing uses DsmInboxReceiver instead.
#[allow(dead_code)]
pub(super) struct ShmMqReceiver {
    inner: MessageQueueReceiver,
}

// SAFETY: `NonNull<shm_mq_handle>` is a pointer into DSM valid across threads
// within the same backend process; the drain thread is the only reader after
// attach, and only via `shm_mq_receive(nowait=true)` (no thread-unsafe latch
// or CFI calls).
unsafe impl Send for ShmMqReceiver {}

// SAFETY: production invariant is that only the backend thread calls `try_recv` (the cooperative
// drain runs inline on `DrainGatherStream::poll_next`; pgrx's `check_active_thread` would panic
// on a non-backend caller anyway). `shm_mq_receive(nowait=true)` is therefore not reentered
// concurrently on the same handle. We need `Sync` to satisfy the `BatchChannelReceiver: Send +
// Sync` bound, which in turn lets `DrainHandle: Sync` come from the concrete types instead of
// from the `Mutex<Vec<…>>` wrapper on `coop_receivers` having to provide it.
unsafe impl Sync for ShmMqReceiver {}

#[allow(dead_code)]
impl ShmMqReceiver {
    /// Attach as receiver to an *already-created* shm_mq.
    ///
    /// # Safety
    /// - `mq` must point to a shm_mq previously initialized by `shm_mq_create`.
    /// - `seg` may be NULL on workers.
    /// - No other proc has already set itself as receiver for `mq`.
    pub(super) unsafe fn attach_existing(
        seg: *mut pg_sys::dsm_segment,
        mq: *mut pg_sys::shm_mq,
    ) -> Self {
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
                mpp_log!("mpp: shm_mq recv unknown code {code}, treating as detached");
                RecvOutcome::Detached
            }
        }
    }
}

// The new DSM-MPSC adapter types are reachable only from the in-file tests until Phase 4
// wires them into `dsm.rs` / `glue.rs`. Dead-code is suppressed per item below; Phase 4
// strips those attributes.

/// DSM-MPSC-ring-backed `BatchChannelSender`. Wraps a `DsmMpscSender` so the existing
/// `MppSender` / cooperative-drain machinery can drive it through the same trait surface
/// as `ShmMqSender`. Multiple producer processes hold their own `DsmInboxSender` clones
/// targeting the same receiver inbox; the underlying ring serializes them via
/// Vyukov-style CAS on `tail`.
///
/// Unlike `ShmMqSender`, this adapter is genuinely thread-safe — the ring's atomic
/// operations are the synchronization point, so the `send_bytes` / `try_send_bytes`
/// paths don't `debug_assert!` an attach-thread invariant.
///
/// Detach-on-drop: the inner `DsmMpscSender::Drop` decrements the ring's
/// `sender_count`; the last drop flips `detached` and wakes the receiver, mirroring
/// shm_mq's "drop the last sender, receiver sees detach" structural guarantee.
#[allow(dead_code)]
pub(super) struct DsmInboxSender {
    inner: DsmMpscSender,
    send_lock: tokio::sync::Mutex<()>,
}

// `DsmInboxSender` auto-derives Send + Sync — both fields are Send + Sync (DsmMpscSender
// via its unsafe impls; tokio::sync::Mutex by definition). No manual unsafe impls
// needed; auto-derive defends against a future field that's accidentally !Send/!Sync.

#[allow(dead_code)]
impl DsmInboxSender {
    /// Wrap a `DsmMpscSender` for use through the `BatchChannelSender` trait.
    pub(super) fn new(inner: DsmMpscSender) -> Self {
        Self {
            inner,
            send_lock: tokio::sync::Mutex::new(()),
        }
    }

    fn map_send_err(err: MpscSendError) -> DataFusionError {
        match err {
            MpscSendError::Detached => {
                DataFusionError::Execution("mpp: DSM MPSC inbox detached".into())
            }
            MpscSendError::MessageTooLarge => DataFusionError::Execution(
                "mpp: DSM MPSC frame exceeds the entire ring capacity \
                 (raise paradedb.mpp_queue_size)"
                    .into(),
            ),
            MpscSendError::Full => DataFusionError::Execution(
                "mpp: DSM MPSC inbox full (caller should retry via try_send_bytes)".into(),
            ),
        }
    }
}

impl BatchChannelSender for DsmInboxSender {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), DataFusionError> {
        // NOT a real block. This adapter is the no-cooperative-drain fallback path;
        // production wires `MppSender::with_cooperative_drain(..)` which calls
        // `try_send_bytes` directly through the cooperative spin (transport.rs). If you
        // reach this function in production it usually means a fragment was constructed
        // without `with_cooperative_drain` — fix that rather than relying on the spin.
        // The yield_now keeps the consumer running on the same OS thread; under a slow
        // consumer this still burns a backend core, so it's strictly a debug aid.
        loop {
            match self.inner.try_send(bytes) {
                Ok(()) => return Ok(()),
                Err(MpscSendError::Full) => std::thread::yield_now(),
                Err(e) => return Err(Self::map_send_err(e)),
            }
        }
    }

    fn try_send_bytes(&self, bytes: &[u8]) -> Result<bool, DataFusionError> {
        match self.inner.try_send(bytes) {
            Ok(()) => Ok(true),
            Err(MpscSendError::Full) => Ok(false),
            Err(e) => Err(Self::map_send_err(e)),
        }
    }

    fn send_lock(&self) -> &tokio::sync::Mutex<()> {
        &self.send_lock
    }
}

/// DSM-MPSC-ring-backed `BatchChannelReceiver`. Wraps a `DsmMpscReceiver` and a single
/// scratch `Vec<u8>` that the inner primitive's `try_recv` populates each call; we hand
/// the populated buffer to the caller via `mem::take` and leave an empty `Vec` behind.
///
/// The single-consumer invariant comes from how this is used: one `DsmInboxReceiver`
/// per process, owned by `DrainHandle::cooperative_receivers`, polled inline from the
/// drain's `try_drain_pass`. No two threads ever race on the same receiver — the
/// `parking_lot::Mutex` on `scratch` is interior-mutability boilerplate, uncontended in
/// production.
#[allow(dead_code)]
pub(super) struct DsmInboxReceiver {
    inner: DsmMpscReceiver,
    /// Scratch the inner primitive's `try_recv` reuses across calls (via reserve+set_len).
    /// We `mem::take` it on every Bytes outcome to hand the bytes to the caller without a
    /// copy; the inner primitive re-grows from `Vec::new()` on the next call.
    scratch: parking_lot::Mutex<Vec<u8>>,
}

// `DsmMpscReceiver` is deliberately `!Sync` (single-consumer invariant). We promote
// `DsmInboxReceiver` to Sync because the caller pattern guarantees only one thread
// touches it at a time, and `parking_lot::Mutex<Vec<u8>>` protects the scratch from
// shared-reference UB if that invariant ever slips. Send is auto-derived.
unsafe impl Sync for DsmInboxReceiver {}

impl DsmInboxReceiver {
    /// Wrap a `DsmMpscReceiver` for use through the `BatchChannelReceiver` trait.
    pub(super) fn new(inner: DsmMpscReceiver) -> Self {
        Self {
            inner,
            scratch: parking_lot::Mutex::new(Vec::new()),
        }
    }

    /// Register this process as the receiver on the underlying ring. See
    /// `DsmMpscReceiver::set_receiver` for the pgprocno + pid contract.
    pub(super) fn set_receiver(&self, pgprocno: i32, pid: i32) {
        self.inner.set_receiver(pgprocno, pid);
    }
}

impl BatchChannelReceiver for DsmInboxReceiver {
    fn try_recv(&self) -> RecvOutcome {
        let mut buf = self.scratch.lock();
        match self.inner.try_recv(&mut buf) {
            MpscRecvOutcome::Bytes => {
                // Hand the populated buffer to the caller and leave an empty Vec behind.
                // The inner primitive's `try_recv` does its own `reserve(len)` on the
                // next call, so we don't need to pre-allocate scratch capacity here.
                RecvOutcome::Bytes(std::mem::take(&mut *buf))
            }
            MpscRecvOutcome::Empty => RecvOutcome::Empty,
            MpscRecvOutcome::Detached => RecvOutcome::Detached,
        }
    }

    /// Force-detach the inbox early (e.g., query teardown before producers finish).
    /// Producers' subsequent `try_send_bytes` return `Err(detached)`. Frames already
    /// in the ring stay drainable; the next `try_recv` past the last queued frame
    /// returns `Detached`.
    fn set_detached(&self) {
        self.inner.set_detached();
    }
}

/// Allocate a fresh ring (heap, aligned) and return (sender, receiver, owning region).
/// Test-only helper that pairs `DsmInboxSender` + `DsmInboxReceiver` over a heap-allocated
/// ring matching the alignment contract `create_at` requires. Production allocates the
/// region inside a `dsm_segment`; this helper exists so the BatchChannel trait impls can
/// be exercised in unit tests without a PG backend.
#[cfg(test)]
pub(super) fn test_dsm_inbox_pair(
    ring_size: u32,
    slot_capacity: u32,
) -> (DsmInboxSender, DsmInboxReceiver, AlignedTestRegion) {
    let bytes = DsmMpscRingHeader::region_bytes(ring_size, slot_capacity);
    let region = AlignedTestRegion::new(bytes);
    let header_ptr =
        unsafe { dsm_mpsc_ring::create_at(region.as_mut_ptr(), ring_size, slot_capacity) };
    let nn = std::ptr::NonNull::new(header_ptr).expect("create_at returned null");
    let sender = DsmInboxSender::new(unsafe { DsmMpscSender::new(nn) });
    let receiver = DsmInboxReceiver::new(unsafe { DsmMpscReceiver::new(nn) });
    (sender, receiver, region)
}

#[cfg(test)]
pub(super) struct AlignedTestRegion {
    ptr: *mut u8,
    layout: std::alloc::Layout,
}

#[cfg(test)]
impl AlignedTestRegion {
    fn new(bytes: usize) -> Self {
        let align = std::mem::align_of::<DsmMpscRingHeader>();
        let layout = std::alloc::Layout::from_size_align(bytes, align).expect("layout");
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!ptr.is_null());
        Self { ptr, layout }
    }
    fn as_mut_ptr(&self) -> *mut u8 {
        self.ptr
    }
}

#[cfg(test)]
impl Drop for AlignedTestRegion {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.ptr, self.layout) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligned_queue_bytes_rounds_down_to_maxalign() {
        let maxalign = pg_sys::MAXIMUM_ALIGNOF as usize;
        for req in [0, 1, 7, 8, 15, 16, 1024, 1025, 1_048_576, 8 * 1024 * 1024] {
            let got = aligned_queue_bytes(req);
            assert!(got <= req);
            assert_eq!(got % maxalign, 0, "not aligned: {got} for req {req}");
            assert!(req - got < maxalign);
        }
    }

    #[test]
    fn align_up_maxalign_rounds_up_to_maxalign() {
        let maxalign = pg_sys::MAXIMUM_ALIGNOF as usize;
        for req in [0, 1, 7, 8, 15, 16, 1024, 1025] {
            let got = align_up_maxalign_checked(req).unwrap();
            assert!(got >= req);
            assert_eq!(got % maxalign, 0);
            assert!(got - req < maxalign);
        }
        assert!(align_up_maxalign_checked(usize::MAX).is_none());
    }

    #[test]
    fn dsm_inbox_batch_channel_round_trip() {
        let (tx, rx, _region) = test_dsm_inbox_pair(4, 64);
        // Sanity: try_send_bytes succeeds, try_recv hands back the bytes, send_lock
        // returns a usable Mutex.
        assert!(tx.try_send_bytes(b"hello").unwrap());
        assert!(tx.try_send_bytes(b"world").unwrap());
        match rx.try_recv() {
            RecvOutcome::Bytes(b) => assert_eq!(&b[..], b"hello"),
            other => panic!("expected Bytes(hello), got {other:?}"),
        }
        match rx.try_recv() {
            RecvOutcome::Bytes(b) => assert_eq!(&b[..], b"world"),
            other => panic!("expected Bytes(world), got {other:?}"),
        }
        assert!(matches!(rx.try_recv(), RecvOutcome::Empty));
        // send_lock is just exercised — the per-instance Mutex satisfies the trait.
        let _guard = tx.send_lock();
    }

    #[test]
    fn dsm_inbox_try_send_returns_false_when_full() {
        let (tx, rx, _region) = test_dsm_inbox_pair(2, 64);
        assert!(tx.try_send_bytes(b"a").unwrap());
        assert!(tx.try_send_bytes(b"b").unwrap());
        // Third send should hit Full (returns Ok(false), not Err).
        assert!(!tx.try_send_bytes(b"c").unwrap());
        // Drain one, then send again succeeds.
        assert!(matches!(rx.try_recv(), RecvOutcome::Bytes(_)));
        assert!(tx.try_send_bytes(b"c").unwrap());
    }

    #[test]
    fn dsm_inbox_recv_signals_detached_after_set_detached() {
        let (tx, rx, _region) = test_dsm_inbox_pair(4, 64);
        tx.try_send_bytes(b"keep").unwrap();
        rx.set_detached();
        // Subsequent send rejected with an Execution error (mapped from MpscSendError::Detached).
        let err = tx
            .try_send_bytes(b"drop")
            .expect_err("expected detached error");
        assert!(format!("{err}").contains("detached"));
        // Already-queued frame still drains; then recv returns Detached.
        assert!(matches!(rx.try_recv(), RecvOutcome::Bytes(_)));
        assert!(matches!(rx.try_recv(), RecvOutcome::Detached));
    }

    #[test]
    fn dsm_inbox_multi_producer_through_trait_surface() {
        use std::sync::Arc;
        // Build a real Arc<dyn BatchChannelSender> shared across threads to confirm
        // dyn dispatch + Send/Sync impls compile and behave.
        let (tx, rx, _region) = test_dsm_inbox_pair(64, 32);
        let tx: Arc<dyn BatchChannelSender> = Arc::new(tx);
        let mut handles = Vec::new();
        const K: usize = 4;
        const M: u32 = 200;
        for producer_id in 0..K {
            let tx = Arc::clone(&tx);
            handles.push(std::thread::spawn(move || {
                let mut payload = [0u8; 8];
                payload[0..4].copy_from_slice(&(producer_id as u32).to_le_bytes());
                let mut sent = 0u32;
                while sent < M {
                    payload[4..8].copy_from_slice(&sent.to_le_bytes());
                    match tx.try_send_bytes(&payload) {
                        Ok(true) => sent += 1,
                        Ok(false) => std::thread::yield_now(),
                        Err(e) => panic!("send failed: {e}"),
                    }
                }
            }));
        }
        let target = K * M as usize;
        let mut seen = vec![vec![false; M as usize]; K];
        let mut got = 0usize;
        while got < target {
            match rx.try_recv() {
                RecvOutcome::Bytes(b) => {
                    let producer_id = u32::from_le_bytes(b[0..4].try_into().unwrap()) as usize;
                    let idx = u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize;
                    let already = std::mem::replace(&mut seen[producer_id][idx], true);
                    assert!(!already, "dup ({producer_id}, {idx})");
                    got += 1;
                }
                RecvOutcome::Empty => std::thread::yield_now(),
                RecvOutcome::Detached => panic!("unexpected detach"),
                RecvOutcome::DirectBatch { .. } | RecvOutcome::DirectEof { .. } => {
                    // DsmInbox never emits the direct variants — only in-proc channels do.
                    panic!("dsm inbox should never emit a direct variant");
                }
            }
        }
        for h in handles {
            h.join().unwrap();
        }
    }

    /// Dropping the last DsmInboxSender flips `detached` and the receiver sees the
    /// queued bytes followed by `Detached`. This is the structural equivalent of
    /// shm_mq's "drop the last sender, receiver sees detach" guarantee and is what
    /// keeps the Phase-4 drain loop from wedging on a clean shutdown (no explicit
    /// `set_detached` needed when every sender goes away cleanly).
    #[test]
    fn dropping_last_sender_triggers_detach() {
        let (tx, rx, _region) = test_dsm_inbox_pair(4, 64);
        tx.try_send_bytes(b"final").unwrap();
        drop(tx);
        // The queued frame is still readable.
        match rx.try_recv() {
            RecvOutcome::Bytes(b) => assert_eq!(&b[..], b"final"),
            other => panic!("expected Bytes(final), got {other:?}"),
        }
        // Then Detached.
        assert!(matches!(rx.try_recv(), RecvOutcome::Detached));
    }

    #[test]
    fn try_send_bytes_rejects_oversize_payload() {
        // Multi-slot fragmentation lifted the per-slot ceiling; only payloads larger
        // than `ring_size * (slot_capacity - SLOT_HEADER_BYTES)` are now rejected.
        // ring_size=2, slot_capacity=32 -> payload_cap=16, ring-wide cap=32. 64
        // bytes still doesn't fit.
        let (tx, _rx, _region) = test_dsm_inbox_pair(2, 32);
        let oversize = vec![0u8; 64];
        let err = tx
            .try_send_bytes(&oversize)
            .expect_err("expected MessageTooLarge");
        assert!(
            format!("{err}").contains("exceeds the entire ring capacity"),
            "unexpected error: {err}"
        );
    }
}
