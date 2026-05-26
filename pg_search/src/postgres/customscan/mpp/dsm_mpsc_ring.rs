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

//! DSM-backed multi-producer single-consumer ring for MPP mesh inboxes.
//!
//! PostgreSQL's `shm_mq` is hard-wired single-sender, single-reader (asserted in
//! `shm_mq_set_sender`), so we can't share one inbox across N-1 peers without forking
//! `shm_mq`. This primitive is the replacement: a fixed-size byte-message ring laid out
//! in a contiguous chunk of `dsm_segment` memory, with multi-producer correctness via
//! Vyukov-style per-slot sequence counters.
//!
//! Layout (contiguous bytes, `repr(C)`):
//!
//! ```text
//! +- DsmMpscRingHeader -----------------------+
//! |  ring_size:     u32   (immutable)         |
//! |  slot_capacity: u32   (immutable)         |
//! |  detached:      AtomicBool                |
//! |  receiver_latch: AtomicPtr<pg_sys::Latch> |
//! |  head:          AtomicU64                 |
//! |  tail:          AtomicU64                 |
//! +-------------------------------------------+
//! | Slot[0] | Slot[1] | ... | Slot[N-1]       |
//! +-------------------------------------------+
//!
//! Slot {
//!     seq:  AtomicU64,    // Vyukov sequence: encodes phase of this slot
//!     len:  AtomicU32,
//!     data: [u8; slot_capacity - SLOT_HEADER_SIZE]
//! }
//! ```
//!
//! Slot lifecycle (Vyukov MPMC reduced to MPSC):
//!
//! ```text
//! slot[i] phases:
//!   empty in round k:  seq = k * ring_size + i
//!   ready in round k:  seq = k * ring_size + i + 1
//! ```
//!
//! Producer claim path for `tail = T`:
//! 1. Inspect `slot[T mod ring_size].seq`. Three cases:
//!    - `seq == T`: slot is empty in our round. Try `tail.CAS(T, T+1)`. Winner owns the slot.
//!    - `seq <  T`: ring is full (consumer hasn't reached this round). Return `Full`.
//!    - `seq >  T`: another producer already claimed `T`; re-read `tail` and retry.
//! 2. Winner writes `len`, copies payload, stores `seq = T + 1` (Release) to publish.
//! 3. Winner `SetLatch`es the receiver to break the consumer out of any wait.
//!
//! Consumer take path for `head = H`:
//! 1. `slot[H mod ring_size].seq == H + 1` ⇒ slot ready. Read payload.
//! 2. Store `seq = H + ring_size` (the next round's empty marker for this slot).
//! 3. Store `head = H + 1`.
//!
//! The single-consumer invariant means the consumer doesn't CAS `head`; it just owns it.
//! Producers CAS `tail`, which is the only point of contention.
//!
//! **Safety**: every public function on `DsmMpscSender` / `DsmMpscReceiver` is safe at the
//! Rust type level once construction has happened. The constructors are `unsafe` because
//! the caller has to guarantee the DSM region is correctly sized and not aliased by
//! conflicting writers (mainly: only one process gets to call `create_at`; everyone else
//! calls `attach_at`).

// The primitive is not wired into the rest of the MPP module yet; Phase 3 of the
// mesh-multiplexing implementation does that. Until then, the public-to-the-module API
// items are reachable only from this file's tests, which trip the dead-code lint that
// pre-commit elevates to a hard error.
#![allow(dead_code)]

use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, Ordering};

use pgrx::pg_sys;

/// Sentinel for an unset receiver latch. Producers skip the wakeup call when the latch
/// pointer is null (e.g., in unit tests that don't have a PG backend).
pub(super) const NO_LATCH: *mut pg_sys::Latch = std::ptr::null_mut();

/// Reserved byte at the start of every slot. Sized so payload + header fits in
/// `slot_capacity` exactly.
const SLOT_HEADER_BYTES: usize = std::mem::size_of::<SlotHeader>();

#[repr(C)]
struct SlotHeader {
    /// Vyukov sequence counter; see module docs for phase encoding.
    seq: AtomicU64,
    /// Payload length in bytes; `0..=slot_capacity - SLOT_HEADER_BYTES`.
    len: AtomicU32,
    /// Padding to keep the data region 8-byte aligned. Not consulted by anyone.
    _pad: u32,
}

/// Ring header. Laid out first in the DSM region; slot array follows immediately after.
#[repr(C)]
pub(super) struct DsmMpscRingHeader {
    /// Number of slots. Immutable after `create_at`.
    ring_size: u32,
    /// Byte capacity of each slot INCLUDING the slot header. Payload bytes per slot are
    /// `slot_capacity - SLOT_HEADER_BYTES`. Immutable after `create_at`.
    slot_capacity: u32,
    /// Set by the consumer (or by the leader on query teardown) to tell producers to
    /// fail-fast on subsequent sends. Sticky.
    detached: AtomicBool,
    _pad1: [u8; 7],
    /// PG `Latch*` the consumer waits on. Producers `SetLatch` after a successful send to
    /// wake the consumer. Null in tests; producers skip the wake call.
    receiver_latch: AtomicPtr<pg_sys::Latch>,
    /// Consumer's read cursor. Only the consumer writes this; readers may load.
    head: AtomicU64,
    /// Producers' write cursor. CAS'd to claim slot ownership for a tail value.
    tail: AtomicU64,
}

impl DsmMpscRingHeader {
    /// Bytes occupied by `ring_size` slots of `slot_capacity` each, plus the header.
    pub(super) const fn region_bytes(ring_size: u32, slot_capacity: u32) -> usize {
        std::mem::size_of::<DsmMpscRingHeader>() + (ring_size as usize) * (slot_capacity as usize)
    }
}

/// Errors that `try_send` can surface to the producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SendError {
    /// Ring is full; consumer hasn't drained enough to free a slot.
    Full,
    /// Receiver has detached (query teardown). Producer should stop sending.
    Detached,
    /// `bytes.len() + SLOT_HEADER_BYTES > slot_capacity`. The caller picked a slot
    /// capacity too small for this payload; bumping `slot_capacity` at `create_at` time
    /// fixes it.
    MessageTooLarge,
}

/// Outcome of a single `try_recv` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RecvOutcome {
    /// A frame was copied into the caller's buffer.
    Bytes,
    /// Ring is empty right now; try again later.
    Empty,
    /// All producers have detached and there's no more data to drain.
    Detached,
}

/// Caller-visible handle for the single consumer.
pub(super) struct DsmMpscReceiver {
    ring: NonNull<DsmMpscRingHeader>,
}

/// Caller-visible handle for any of the N-1 producers.
pub(super) struct DsmMpscSender {
    ring: NonNull<DsmMpscRingHeader>,
}

// SAFETY: the ring is a `repr(C)` blob in shared memory whose atomic operations are the
// synchronization point. Both handles are stateless pointers to the same data; sending
// either across threads requires only the atomic ordering already in use.
unsafe impl Send for DsmMpscReceiver {}
unsafe impl Send for DsmMpscSender {}
unsafe impl Sync for DsmMpscSender {}

/// Initialize a freshly-allocated DSM region into a valid ring header + zeroed slot array.
/// Must be called exactly once, by the process that allocated the region (leader in our
/// case). All other processes attach via [`attach_at`] without re-initializing.
///
/// # Safety
/// - `base` must point at the start of a region of at least
///   `DsmMpscRingHeader::region_bytes(ring_size, slot_capacity)` bytes.
/// - `ring_size >= 2` (a ring of 1 slot can't distinguish empty from full).
/// - `slot_capacity > SLOT_HEADER_BYTES` (need at least one byte of payload).
/// - The region must not be concurrently accessed by any other process or thread until
///   this returns.
pub(super) unsafe fn create_at(
    base: *mut u8,
    ring_size: u32,
    slot_capacity: u32,
) -> *mut DsmMpscRingHeader {
    debug_assert!(ring_size >= 2, "ring_size must be >= 2");
    debug_assert!(
        slot_capacity as usize > SLOT_HEADER_BYTES,
        "slot_capacity must leave room for at least one payload byte"
    );
    let header_ptr = base.cast::<DsmMpscRingHeader>();
    // Write the immutable header fields. Use std::ptr::write so we don't construct an
    // intermediate &mut that aliases the not-yet-initialized atomic fields.
    unsafe {
        std::ptr::write(
            header_ptr,
            DsmMpscRingHeader {
                ring_size,
                slot_capacity,
                detached: AtomicBool::new(false),
                _pad1: [0; 7],
                receiver_latch: AtomicPtr::new(NO_LATCH),
                head: AtomicU64::new(0),
                tail: AtomicU64::new(0),
            },
        );
    }
    // Initialize slot sequences: slot[i].seq = i in round 0.
    for i in 0..ring_size {
        let slot = unsafe { slot_ptr(header_ptr, i, slot_capacity) };
        unsafe {
            std::ptr::write(
                slot,
                SlotHeader {
                    seq: AtomicU64::new(i as u64),
                    len: AtomicU32::new(0),
                    _pad: 0,
                },
            );
        }
    }
    header_ptr
}

/// Take an already-initialized ring header pointer and confirm its shape matches caller's
/// expectations. The caller's `expected_ring_size` / `expected_slot_capacity` must match
/// the values written at `create_at` time; mismatch is a hard error (returns null).
///
/// # Safety
/// - `base` must point at the same region a previous `create_at` initialized.
/// - The region must not be deallocated for the lifetime of any handle returned from
///   the wrappers (`DsmMpscReceiver::new`, `DsmMpscSender::new`).
pub(super) unsafe fn attach_at(
    base: *mut u8,
    expected_ring_size: u32,
    expected_slot_capacity: u32,
) -> Option<NonNull<DsmMpscRingHeader>> {
    let header_ptr = base.cast::<DsmMpscRingHeader>();
    let nn = NonNull::new(header_ptr)?;
    let header = unsafe { nn.as_ref() };
    if header.ring_size != expected_ring_size || header.slot_capacity != expected_slot_capacity {
        return None;
    }
    Some(nn)
}

#[inline]
unsafe fn slot_ptr(
    header: *mut DsmMpscRingHeader,
    idx: u32,
    slot_capacity: u32,
) -> *mut SlotHeader {
    let header_bytes = std::mem::size_of::<DsmMpscRingHeader>();
    let base = header.cast::<u8>();
    unsafe { base.add(header_bytes + (idx as usize) * (slot_capacity as usize)) }
        .cast::<SlotHeader>()
}

#[inline]
unsafe fn slot_data_ptr(slot: *mut SlotHeader) -> *mut u8 {
    unsafe { slot.cast::<u8>().add(SLOT_HEADER_BYTES) }
}

impl DsmMpscReceiver {
    /// Wrap a previously-initialized ring as the single consumer. Pairs with
    /// [`DsmMpscSender::new`] on the producer side; calling code is responsible for
    /// ensuring exactly one `DsmMpscReceiver` exists per ring.
    ///
    /// # Safety
    /// `ring` must point to a header initialized by [`create_at`] and not yet
    /// deallocated. The caller guarantees no other `DsmMpscReceiver` exists for the
    /// same ring (single-consumer invariant).
    pub(super) unsafe fn new(ring: NonNull<DsmMpscRingHeader>) -> Self {
        Self { ring }
    }

    /// Install (or clear) the receiver's `pg_sys::Latch*`. Producers will `SetLatch` it
    /// after a successful send so a blocked consumer in `WaitLatch` wakes up. Pass
    /// [`NO_LATCH`] to disable wakeup (useful in unit tests).
    pub(super) fn set_latch(&self, latch: *mut pg_sys::Latch) {
        let header = unsafe { self.ring.as_ref() };
        header.receiver_latch.store(latch, Ordering::Release);
    }

    /// Try to read one frame into `out`. On `Bytes`, `out` holds the payload. On
    /// `Empty`, the caller should yield and retry. On `Detached`, the ring is drained
    /// and all producers have detached; no more frames will arrive.
    pub(super) fn try_recv(&self, out: &mut Vec<u8>) -> RecvOutcome {
        let header = unsafe { self.ring.as_ref() };
        let head = header.head.load(Ordering::Relaxed);
        let slot_idx = (head % header.ring_size as u64) as u32;
        let slot = unsafe { slot_ptr(self.ring.as_ptr(), slot_idx, header.slot_capacity) };
        let seq = unsafe { (*slot).seq.load(Ordering::Acquire) };
        let expected_ready = head.wrapping_add(1);
        if seq != expected_ready {
            // Slot not ready. Detached + queue caught up means we're done.
            if header.detached.load(Ordering::Acquire)
                && header.tail.load(Ordering::Acquire) == head
            {
                return RecvOutcome::Detached;
            }
            return RecvOutcome::Empty;
        }
        let len = unsafe { (*slot).len.load(Ordering::Relaxed) } as usize;
        out.clear();
        out.reserve(len);
        let data = unsafe { slot_data_ptr(slot) };
        unsafe {
            out.set_len(len);
            std::ptr::copy_nonoverlapping(data, out.as_mut_ptr(), len);
        }
        // Mark the slot empty for the next round. Round k empty marker is
        // (k * ring_size) + slot_idx; head + ring_size is exactly that for next round.
        let next_empty_seq = head.wrapping_add(header.ring_size as u64);
        unsafe { (*slot).seq.store(next_empty_seq, Ordering::Release) };
        // Now safe to advance head.
        header.head.store(head.wrapping_add(1), Ordering::Release);
        RecvOutcome::Bytes
    }

    /// Tell producers to stop sending. Idempotent. Subsequent `try_send` calls will
    /// fail-fast with `SendError::Detached`. Does NOT block; caller can still drain
    /// already-queued frames via `try_recv` until it returns `RecvOutcome::Detached`.
    pub(super) fn set_detached(&self) {
        let header = unsafe { self.ring.as_ref() };
        header.detached.store(true, Ordering::Release);
        // Wake any producer that might be in a (future) blocking-send variant.
        let latch = header.receiver_latch.load(Ordering::Acquire);
        if !latch.is_null() {
            unsafe { pg_sys::SetLatch(latch) };
        }
    }

    /// True when `set_detached` has been called.
    pub(super) fn is_detached(&self) -> bool {
        let header = unsafe { self.ring.as_ref() };
        header.detached.load(Ordering::Acquire)
    }
}

impl DsmMpscSender {
    /// Wrap a previously-initialized ring as a producer. Multiple `DsmMpscSender`
    /// handles to the same ring are legal (and the whole point of MPSC).
    ///
    /// # Safety
    /// `ring` must point to a header initialized by [`create_at`] and not yet
    /// deallocated.
    pub(super) unsafe fn new(ring: NonNull<DsmMpscRingHeader>) -> Self {
        Self { ring }
    }

    /// Push one frame onto the ring. Returns immediately:
    /// - `Ok(())` on success; receiver's latch was set if installed.
    /// - `Err(Full)` if no slot is available right now; caller should yield + retry.
    /// - `Err(Detached)` if the receiver has detached; caller should stop.
    /// - `Err(MessageTooLarge)` if `bytes.len() + SLOT_HEADER_BYTES > slot_capacity`.
    pub(super) fn try_send(&self, bytes: &[u8]) -> Result<(), SendError> {
        let header = unsafe { self.ring.as_ref() };
        if header.detached.load(Ordering::Acquire) {
            return Err(SendError::Detached);
        }
        let payload_cap = (header.slot_capacity as usize).saturating_sub(SLOT_HEADER_BYTES);
        if bytes.len() > payload_cap {
            return Err(SendError::MessageTooLarge);
        }
        loop {
            let tail = header.tail.load(Ordering::Relaxed);
            let slot_idx = (tail % header.ring_size as u64) as u32;
            let slot = unsafe { slot_ptr(self.ring.as_ptr(), slot_idx, header.slot_capacity) };
            let seq = unsafe { (*slot).seq.load(Ordering::Acquire) };
            // Three-way compare per Vyukov MPMC.
            match seq.cmp(&tail) {
                std::cmp::Ordering::Equal => {
                    // Slot is empty in our round. Try to claim by advancing tail.
                    match header.tail.compare_exchange_weak(
                        tail,
                        tail.wrapping_add(1),
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => {
                            // We own slot[slot_idx] for tail value `tail`.
                            unsafe {
                                (*slot).len.store(bytes.len() as u32, Ordering::Relaxed);
                                let data = slot_data_ptr(slot);
                                std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
                                // Publish: ready in round k is (k * ring_size + i + 1) = tail + 1.
                                (*slot).seq.store(tail.wrapping_add(1), Ordering::Release);
                            }
                            // Wake the consumer.
                            let latch = header.receiver_latch.load(Ordering::Acquire);
                            if !latch.is_null() {
                                unsafe { pg_sys::SetLatch(latch) };
                            }
                            return Ok(());
                        }
                        Err(_) => continue, // another producer took this tail; retry
                    }
                }
                std::cmp::Ordering::Less => {
                    // seq < tail: the consumer hasn't reclaimed slot[slot_idx] for our
                    // round yet. Ring is full.
                    return Err(SendError::Full);
                }
                std::cmp::Ordering::Greater => {
                    // seq > tail: another producer has already claimed tail. Reload and
                    // retry.
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as O};
    use std::sync::Arc;

    /// Send + Copy wrapper so tests can pass the ring pointer into spawned threads. The
    /// real production handles (DsmMpscSender / Receiver) already impl Send; this exists
    /// only because constructing them is unsafe and we want the test to do it inside the
    /// thread so each thread has its own handle.
    #[derive(Clone, Copy)]
    struct SharedRing(NonNull<DsmMpscRingHeader>);
    unsafe impl Send for SharedRing {}

    /// Allocate a heap region big enough for a ring with `ring_size` slots of
    /// `slot_capacity` bytes each, initialize it, and return (receiver, sender_template,
    /// owner) where the owner Box keeps the region alive. The test drops the owner last.
    fn make_ring(ring_size: u32, slot_capacity: u32) -> (DsmMpscReceiver, DsmMpscSender, Vec<u8>) {
        let bytes = DsmMpscRingHeader::region_bytes(ring_size, slot_capacity);
        // Box<[u8]> gives us a stable address for the duration of the test.
        let mut region: Vec<u8> = vec![0u8; bytes];
        let header_ptr = unsafe { create_at(region.as_mut_ptr(), ring_size, slot_capacity) };
        let nn = NonNull::new(header_ptr).expect("create_at returned null");
        // Unsafe: we hand ownership of the same pointer to two handles. Safe because the
        // ring is the synchronization point; the handles are stateless wrappers.
        let receiver = unsafe { DsmMpscReceiver::new(nn) };
        let sender = unsafe { DsmMpscSender::new(nn) };
        (receiver, sender, region)
    }

    #[test]
    fn spsc_round_trip_under_capacity() {
        let (rx, tx, _region) = make_ring(4, 64);
        for i in 0..3u8 {
            tx.try_send(&[i, i + 1, i + 2]).unwrap();
        }
        let mut buf = Vec::new();
        for i in 0..3u8 {
            assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Bytes);
            assert_eq!(&buf[..], &[i, i + 1, i + 2]);
        }
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Empty);
    }

    #[test]
    fn fills_then_full_then_drains() {
        let (rx, tx, _region) = make_ring(4, 64);
        for i in 0..4u32 {
            tx.try_send(&i.to_le_bytes()).unwrap();
        }
        // Fifth send must fail Full.
        assert_eq!(tx.try_send(&999u32.to_le_bytes()), Err(SendError::Full));
        // Drain one, send one more, drain rest.
        let mut buf = Vec::new();
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Bytes);
        assert_eq!(buf, 0u32.to_le_bytes());
        tx.try_send(&100u32.to_le_bytes()).unwrap();
        for expected in [1u32, 2, 3, 100] {
            assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Bytes);
            assert_eq!(buf, expected.to_le_bytes());
        }
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Empty);
    }

    #[test]
    fn detach_blocks_subsequent_sends() {
        let (rx, tx, _region) = make_ring(4, 64);
        tx.try_send(b"keep").unwrap();
        rx.set_detached();
        assert!(rx.is_detached());
        assert_eq!(tx.try_send(b"drop"), Err(SendError::Detached));
        // Already-queued frame still readable, then Detached signaled.
        let mut buf = Vec::new();
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Bytes);
        assert_eq!(&buf[..], b"keep");
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Detached);
    }

    #[test]
    fn message_too_large_is_rejected() {
        let (_rx, tx, _region) = make_ring(2, 32);
        // payload_cap = 32 - SLOT_HEADER_BYTES; one byte over is too large.
        let payload_cap = 32 - SLOT_HEADER_BYTES;
        let oversize = vec![0u8; payload_cap + 1];
        assert_eq!(tx.try_send(&oversize), Err(SendError::MessageTooLarge));
        // Exactly at the cap is fine.
        let exact = vec![0u8; payload_cap];
        tx.try_send(&exact).unwrap();
    }

    /// Multi-producer concurrent send: K threads each push M unique messages; consumer
    /// receives K*M messages and verifies every (producer, sequence_in_producer) pair
    /// appears exactly once.
    #[test]
    fn mpsc_no_lost_messages_under_contention() {
        const K_PRODUCERS: usize = 8;
        const M_PER_PRODUCER: u32 = 2000;
        let (rx, tx_template, _region) = make_ring(64, 32);
        // Wrap region pointer in something we can share across threads. The handles
        // themselves are Send, so we clone via NonNull copy.
        let ring_nn = SharedRing(tx_template.ring);
        let consumed = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::with_capacity(K_PRODUCERS);
        for producer_id in 0..K_PRODUCERS {
            // Construct the sender on this thread so the closure captures DsmMpscSender
            // (Send via unsafe impl) rather than the inner NonNull (Rust 2021 disjoint
            // capture would otherwise project to the NonNull field and fail Send).
            let tx = unsafe { DsmMpscSender::new(ring_nn.0) };
            let h = std::thread::spawn(move || {
                let mut sent = 0u32;
                while sent < M_PER_PRODUCER {
                    let mut payload = [0u8; 8];
                    payload[0..4].copy_from_slice(&(producer_id as u32).to_le_bytes());
                    payload[4..8].copy_from_slice(&sent.to_le_bytes());
                    match tx.try_send(&payload) {
                        Ok(_) => sent += 1,
                        Err(SendError::Full) => std::thread::yield_now(),
                        Err(e) => panic!("unexpected send error: {e:?}"),
                    }
                }
            });
            handles.push(h);
        }
        // Drain on this thread until every producer's M messages have shown up.
        let mut seen: Vec<Vec<bool>> = (0..K_PRODUCERS)
            .map(|_| vec![false; M_PER_PRODUCER as usize])
            .collect();
        let mut buf = Vec::new();
        let target = K_PRODUCERS * M_PER_PRODUCER as usize;
        while consumed.load(O::Relaxed) < target {
            match rx.try_recv(&mut buf) {
                RecvOutcome::Bytes => {
                    assert_eq!(buf.len(), 8);
                    let producer_id = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
                    let sent_idx = u32::from_le_bytes(buf[4..8].try_into().unwrap()) as usize;
                    assert!(producer_id < K_PRODUCERS, "bad producer id {producer_id}");
                    assert!(
                        sent_idx < M_PER_PRODUCER as usize,
                        "bad sent idx {sent_idx}"
                    );
                    let already = std::mem::replace(&mut seen[producer_id][sent_idx], true);
                    assert!(!already, "duplicate ({producer_id}, {sent_idx})");
                    consumed.fetch_add(1, O::Relaxed);
                }
                RecvOutcome::Empty => std::thread::yield_now(),
                RecvOutcome::Detached => panic!("unexpected detach mid-drain"),
            }
        }
        for h in handles {
            h.join().unwrap();
        }
        for (p, row) in seen.iter().enumerate() {
            assert!(
                row.iter().all(|&b| b),
                "producer {p} has a missed message: row = {row:?}"
            );
        }
    }

    /// Per-producer in-order property: a single producer's messages observed by the
    /// consumer must arrive in the order the producer sent them. (Cross-producer
    /// ordering is not guaranteed.)
    #[test]
    fn mpsc_preserves_per_producer_order() {
        const K_PRODUCERS: usize = 4;
        const M_PER_PRODUCER: u32 = 500;
        let (rx, tx_template, _region) = make_ring(32, 32);
        let ring_nn = SharedRing(tx_template.ring);
        let mut handles = Vec::with_capacity(K_PRODUCERS);
        for producer_id in 0..K_PRODUCERS {
            let tx = unsafe { DsmMpscSender::new(ring_nn.0) };
            handles.push(std::thread::spawn(move || {
                let mut sent = 0u32;
                while sent < M_PER_PRODUCER {
                    let mut payload = [0u8; 8];
                    payload[0..4].copy_from_slice(&(producer_id as u32).to_le_bytes());
                    payload[4..8].copy_from_slice(&sent.to_le_bytes());
                    if tx.try_send(&payload).is_ok() {
                        sent += 1;
                    } else {
                        std::thread::yield_now();
                    }
                }
            }));
        }
        let mut last_seq: Vec<i64> = vec![-1; K_PRODUCERS];
        let mut buf = Vec::new();
        let mut total = 0usize;
        let target = K_PRODUCERS * M_PER_PRODUCER as usize;
        while total < target {
            match rx.try_recv(&mut buf) {
                RecvOutcome::Bytes => {
                    let producer_id = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
                    let sent_idx = i64::from(u32::from_le_bytes(buf[4..8].try_into().unwrap()));
                    assert_eq!(
                        sent_idx,
                        last_seq[producer_id] + 1,
                        "out-of-order from producer {producer_id}: got {sent_idx}, expected {}",
                        last_seq[producer_id] + 1
                    );
                    last_seq[producer_id] = sent_idx;
                    total += 1;
                }
                RecvOutcome::Empty => std::thread::yield_now(),
                RecvOutcome::Detached => panic!("unexpected detach"),
            }
        }
        for h in handles {
            h.join().unwrap();
        }
    }
}
