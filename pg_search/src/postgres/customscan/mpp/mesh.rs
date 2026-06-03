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

//! MPP mesh adapters around `DsmMpscRing` plus the alignment helpers used to size
//! the per-inbox DSM regions.

use pgrx::pg_sys;

use crate::postgres::customscan::mpp::dsm_mpsc_ring::{
    DsmMpscReceiver, DsmMpscSender, RecvOutcome as MpscRecvOutcome, SendError as MpscSendError,
};
use crate::postgres::customscan::mpp::transport::{
    BatchChannelReceiver, BatchChannelSender, RecvOutcome,
};
use datafusion::common::DataFusionError;

/// MAXALIGN-DOWN of `queue_bytes`. Postgres requires shm-style slot sizes to be
/// MAXALIGN-aligned; this rounds the user request down so every per-inbox region in
/// the DSM grid is the same size and offset math is a simple multiply.
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

/// DSM MPSC ring as a `BatchChannelSender`. Multiple producer processes hold their own
/// `DsmInboxSender` clones targeting the same receiver inbox; the ring serializes them
/// via Vyukov CAS on `tail`. Because the ring's atomics are the synchronization point,
/// the `send_bytes` / `try_send_bytes` paths don't need an attach-thread `debug_assert!`.
///
/// Detach-on-drop: `DsmMpscSender::Drop` decrements `sender_count`; the last drop flips
/// `detached` and wakes the receiver, mirroring shm_mq's "drop the last sender, receiver
/// sees detach" guarantee.
pub(super) struct DsmInboxSender {
    inner: DsmMpscSender,
    send_lock: tokio::sync::Mutex<()>,
}

// Send + Sync auto-derive. Both fields are Send + Sync (`DsmMpscSender` via its unsafe
// impls; `tokio::sync::Mutex` by definition), so no manual `unsafe impl` is needed; the
// auto-derive also surfaces a compile error if a future field is `!Send` / `!Sync`.

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
            MpscSendError::MessageTooLarge => {
                DataFusionError::Execution("mpp: DSM MPSC frame exceeds slot_capacity".into())
            }
            MpscSendError::Full => DataFusionError::Execution(
                "mpp: DSM MPSC inbox full (caller should retry via try_send_bytes)".into(),
            ),
        }
    }
}

impl BatchChannelSender for DsmInboxSender {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), DataFusionError> {
        // Fallback for callers that didn't wire `with_cooperative_drain`. The real send
        // path drives `try_send_bytes` through the cooperative spin in `transport.rs`;
        // this loop just spins on `yield_now` and burns the backend core under a slow
        // consumer. Hitting it in production means a missing `with_cooperative_drain`
        // on the fragment, not a real backpressure path.
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

/// DSM MPSC ring as a `BatchChannelReceiver`. The scratch `Vec<u8>` lives behind a
/// `parking_lot::Mutex` so a `&self` `try_recv` can hand the populated buffer back via
/// `mem::take` without `RefCell` runtime borrow tracking.
///
/// Single-consumer comes from the call pattern: one `DsmInboxReceiver` per process,
/// owned by `DrainHandle::cooperative_receivers`, polled inline from `try_drain_pass`.
/// No two threads ever race on the same receiver; the mutex is just interior-mutability
/// boilerplate, uncontended in production.
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::customscan::mpp::dsm_mpsc_ring::{self, DsmMpscRingHeader};

    /// Allocate a fresh ring (heap, aligned) and return (sender, receiver, owning region).
    /// Pairs `DsmInboxSender` + `DsmInboxReceiver` over a heap-allocated ring matching the
    /// alignment contract `create_at` requires. Production allocates the region inside a
    /// `dsm_segment`; this helper exists so the BatchChannel trait impls can be exercised
    /// without a PG backend.
    fn test_dsm_inbox_pair(
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

    struct AlignedTestRegion {
        ptr: *mut u8,
        layout: std::alloc::Layout,
    }

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

    impl Drop for AlignedTestRegion {
        fn drop(&mut self) {
            unsafe { std::alloc::dealloc(self.ptr, self.layout) };
        }
    }

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
        // send_lock is just exercised: the per-instance Mutex satisfies the trait.
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
            }
        }
        for h in handles {
            h.join().unwrap();
        }
    }

    /// Dropping the last `DsmInboxSender` flips `detached` and the receiver sees the
    /// queued bytes followed by `Detached`. This is the structural equivalent of
    /// shm_mq's "drop the last sender, receiver sees detach" guarantee, and is what
    /// keeps the drain loop from wedging on a clean shutdown.
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
        let (tx, _rx, _region) = test_dsm_inbox_pair(2, 32);
        // Any payload at or above slot_capacity is unconditionally too large (slot
        // capacity includes the slot header). 64 bytes on a 32-byte slot fits.
        let oversize = vec![0u8; 64];
        let err = tx
            .try_send_bytes(&oversize)
            .expect_err("expected MessageTooLarge");
        assert!(
            format!("{err}").contains("exceeds slot_capacity"),
            "unexpected error: {err}"
        );
    }
}
