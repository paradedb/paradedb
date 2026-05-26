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
//!
//! **Counter wraparound**: `head` and `tail` are `u64` and incremented by one per
//! operation. At 100M ops/sec, wraparound takes ~5800 years. We treat that as physically
//! impossible and don't handle it; the Vyukov seq math would break under wrap because
//! pre-wrap seq values would be larger than post-wrap tail values and producers would
//! spin-retry indefinitely. If this primitive ever ships in a context where wrap is
//! conceivable, add a `tail < u64::MAX - margin` check and reset the ring.

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

/// Magic constant validating that an attaching process points at a `DsmMpscRingHeader`
/// rather than garbage. "MPCR" = MPSC Ring. Different value from `MppDsmHeader`'s magic
/// so a worker that picks up the wrong region fails the wrong-shape check loudly.
const MPSC_RING_MAGIC: u32 = u32::from_le_bytes(*b"MPCR");

/// Bumped on any incompatible layout change. Mirrors the discipline at
/// `MppDsmHeader::validate`.
const MPSC_RING_VERSION: u32 = 1;

/// Assumed cache line size for false-sharing avoidance. 64 bytes covers x86_64 and arm64;
/// over-padding on smaller-cache-line targets costs a few bytes per ring, nothing more.
const CACHE_LINE: usize = 64;

/// Ring header. Laid out first in the DSM region; slot array follows immediately after.
///
/// Cache-line padding around `head` and `tail` is load-bearing: under N=24 producer
/// contention the consumer's `head.store` and the producers' `tail.compare_exchange`
/// race on the same line, MESI-ping-ponging on every drain/claim. That's exactly the
/// false-sharing footgun Vyukov's MPMC writeups call out; the Disruptor literature
/// shows 5-10x throughput loss from this on x86. Padding keeps each hot field on its
/// own line.
#[repr(C, align(64))]
pub(super) struct DsmMpscRingHeader {
    /// Magic constant; equals `MPSC_RING_MAGIC` for a valid ring. Checked in `attach_at`.
    magic: u32,
    /// Layout version; equals `MPSC_RING_VERSION`. Checked in `attach_at`.
    version: u32,
    /// Number of slots. Immutable after `create_at`.
    ring_size: u32,
    /// Byte capacity of each slot INCLUDING the slot header. Payload bytes per slot are
    /// `slot_capacity - SLOT_HEADER_BYTES`. Immutable after `create_at`.
    slot_capacity: u32,
    /// Set by the consumer (or by the leader on query teardown) to tell producers to
    /// fail-fast on subsequent sends. Sticky.
    detached: AtomicBool,
    _pad_after_detached: [u8; 3],
    /// PG `Latch*` the consumer waits on. Producers `SetLatch` after a successful send to
    /// wake the consumer.
    ///
    /// # Safety contract for callers
    ///
    /// The `*mut Latch` stored here must remain valid for the lifetime of every
    /// outstanding `DsmMpscSender`. Producers load this pointer on every send and
    /// dereference via `SetLatch`. In particular: if the consumer process exits and PG
    /// reuses its `PGPROC` slot, a stored raw `Latch*` becomes dangling and `SetLatch`
    /// would wake an unrelated backend. Phase 3 integration must replace this with a
    /// `pgprocno: AtomicU32` and resolve to the latch fresh on every wakeup (so
    /// `PGPROC` reuse is harmless and a `pid` check defends against late wakeups).
    /// For the in-process unit-test use today (`NO_LATCH` everywhere) this is moot.
    receiver_latch: AtomicPtr<pg_sys::Latch>,
    /// Padding to push `head` onto its own cache line. Sized against the offsets of the
    /// preceding fields; the static_assert in `tests::header_layout_is_cache_friendly`
    /// keeps it honest.
    _pad_before_head: [u8; CACHE_LINE - 24],
    /// Consumer's read cursor. Only the consumer writes this. Currently no producer
    /// reads it (full-detection works via slot `seq`); the Release-on-store is defensive
    /// for any future blocking-send variant that wants to poll consumer progress.
    head: AtomicU64,
    /// Padding to push `tail` onto its own cache line so the consumer's `head.store`
    /// doesn't invalidate the producers' `tail` cache line.
    _pad_between_head_and_tail: [u8; CACHE_LINE - 8],
    /// Producers' write cursor. CAS'd to claim slot ownership for a tail value.
    tail: AtomicU64,
    /// Padding so the first slot doesn't share a cache line with `tail`. Producers
    /// race on `tail`; the consumer's first slot read should not pull the `tail` cache
    /// line into the consumer's L1 unnecessarily.
    _pad_after_tail: [u8; CACHE_LINE - 8],
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
//
// `DsmMpscReceiver` is deliberately !Sync: the type-level invariant that exactly one
// thread calls `try_recv` / `set_detached` at a time is what makes the lock-free MPSC
// math correct (single consumer owns `head` without CAS). `DsmMpscSender` is Sync —
// multiple producer threads share one `Arc<DsmMpscSender>` and race on `tail` via CAS.
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
    // The header is `repr(C, align(64))`, so `std::ptr::write` requires the destination
    // address to be 64-byte aligned. PG `dsm_segment`s are page-aligned in production;
    // tests must use an aligned-Vec helper rather than `vec![0u8; n]`.
    debug_assert!(
        (base as usize).is_multiple_of(std::mem::align_of::<DsmMpscRingHeader>()),
        "create_at base must be aligned to {} bytes",
        std::mem::align_of::<DsmMpscRingHeader>()
    );
    let header_ptr = base.cast::<DsmMpscRingHeader>();
    // Write the immutable header fields. Use std::ptr::write so we don't construct an
    // intermediate &mut that aliases the not-yet-initialized atomic fields.
    unsafe {
        std::ptr::write(
            header_ptr,
            DsmMpscRingHeader {
                magic: MPSC_RING_MAGIC,
                version: MPSC_RING_VERSION,
                ring_size,
                slot_capacity,
                detached: AtomicBool::new(false),
                _pad_after_detached: [0; 3],
                receiver_latch: AtomicPtr::new(NO_LATCH),
                _pad_before_head: [0; CACHE_LINE - 24],
                head: AtomicU64::new(0),
                _pad_between_head_and_tail: [0; CACHE_LINE - 8],
                tail: AtomicU64::new(0),
                _pad_after_tail: [0; CACHE_LINE - 8],
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
    if header.magic != MPSC_RING_MAGIC || header.version != MPSC_RING_VERSION {
        return None;
    }
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
    ///
    /// Known wedge: if a producer CAS-advances `tail` and then exits/crashes before
    /// publishing the slot's `seq`, the consumer sees `tail > head`, the slot's `seq`
    /// stuck at the previous-round empty marker, and `detached && tail <= head` never
    /// becomes true. The drain loop will keep returning `Empty` indefinitely. In
    /// production this is bounded by PG's parallel-worker death handling (worker exit
    /// → leader gets ERROR → DSM teardown), but Phase 3 should add an explicit
    /// liveness check against `PGPROC`s to force-detach on producer death.
    pub(super) fn try_recv(&self, out: &mut Vec<u8>) -> RecvOutcome {
        let header = unsafe { self.ring.as_ref() };
        let head = header.head.load(Ordering::Relaxed);
        let slot_idx = (head % header.ring_size as u64) as u32;
        let slot = unsafe { slot_ptr(self.ring.as_ptr(), slot_idx, header.slot_capacity) };
        let seq = unsafe { (*slot).seq.load(Ordering::Acquire) };
        let expected_ready = head.wrapping_add(1);
        if seq != expected_ready {
            // Slot not ready. Use `<=` rather than `==` so a strict invariant
            // violation (tail < head, impossible under correct operation) still
            // surfaces as Detached rather than wedging Empty forever.
            if header.detached.load(Ordering::Acquire)
                && header.tail.load(Ordering::Acquire) <= head
            {
                return RecvOutcome::Detached;
            }
            return RecvOutcome::Empty;
        }
        let len_raw = unsafe { (*slot).len.load(Ordering::Relaxed) } as usize;
        // Clamp against slot's payload capacity. DSM is mapped writable by every
        // attached backend, so a buggy / corrupted producer could write a garbage len.
        // Without this guard, `set_len + copy_nonoverlapping` would read OOB into
        // neighboring slots or other DSM contents.
        let payload_cap = (header.slot_capacity as usize).saturating_sub(SLOT_HEADER_BYTES);
        if len_raw > payload_cap {
            // Poison the ring rather than silently returning corrupt data.
            header.detached.store(true, Ordering::Release);
            return RecvOutcome::Detached;
        }
        let len = len_raw;
        out.clear();
        out.reserve(len);
        let data = unsafe { slot_data_ptr(slot) };
        // copy_nonoverlapping before set_len so a hypothetical panic mid-copy doesn't
        // leave `out` with logical-len > initialized-bytes.
        unsafe {
            std::ptr::copy_nonoverlapping(data, out.as_mut_ptr(), len);
            out.set_len(len);
        }
        // Mark the slot empty for the next round. Round k empty marker is
        // (k * ring_size) + slot_idx; head + ring_size is exactly that for next round.
        let next_empty_seq = head.wrapping_add(header.ring_size as u64);
        unsafe { (*slot).seq.store(next_empty_seq, Ordering::Release) };
        // Advance head AFTER publishing the slot's empty marker, so a producer racing
        // to claim sees the empty slot before seeing the new head value.
        header.head.store(head.wrapping_add(1), Ordering::Release);
        RecvOutcome::Bytes
    }

    /// Tell producers to stop sending. Idempotent. Subsequent `try_send` calls will
    /// fail-fast with `SendError::Detached`. Does NOT block; caller can still drain
    /// already-queued frames via `try_recv` until it returns `RecvOutcome::Detached`.
    pub(super) fn set_detached(&self) {
        let header = unsafe { self.ring.as_ref() };
        header.detached.store(true, Ordering::Release);
        // Defensive self-wake. Normally the receiver itself calls set_detached and
        // doesn't need to wake itself, but if a signal handler or supervisor thread
        // calls this while the receiver is in WaitLatch, this breaks it out.
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
            // Acquire load on `tail` pairs defensively with any future blocking-send
            // variant that may want to observe consumer progress via `head` (we don't
            // today; full-detection rides on the slot's `seq`). Cheaper to keep the
            // ordering tight than to relax-then-tighten under a future audit.
            let tail = header.tail.load(Ordering::Acquire);
            let slot_idx = (tail % header.ring_size as u64) as u32;
            let slot = unsafe { slot_ptr(self.ring.as_ptr(), slot_idx, header.slot_capacity) };
            let seq = unsafe { (*slot).seq.load(Ordering::Acquire) };
            // Three-way compare per Vyukov MPMC.
            match seq.cmp(&tail) {
                std::cmp::Ordering::Equal => {
                    // Slot is empty in our round. Try to claim by advancing tail.
                    // AcqRel on success so a subsequent producer's Acquire load of
                    // `tail` sees our claim and skips the slot we own. Relaxed on
                    // failure (we just retry the loop on a fresh tail load).
                    match header.tail.compare_exchange_weak(
                        tail,
                        tail.wrapping_add(1),
                        Ordering::AcqRel,
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

    /// Owning aligned region for a ring. The default `Vec<u8>` allocator doesn't
    /// guarantee `repr(C, align(64))` alignment on the eventual `create_at` write, so
    /// allocate via the global allocator with an explicit `Layout` and free in `Drop`.
    /// Production uses PG `dsm_segment` (page-aligned), so this dance is test-only.
    struct AlignedRegion {
        ptr: *mut u8,
        layout: std::alloc::Layout,
    }
    impl AlignedRegion {
        fn new(bytes: usize) -> Self {
            let align = std::mem::align_of::<DsmMpscRingHeader>();
            let layout = std::alloc::Layout::from_size_align(bytes, align).expect("invalid layout");
            let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
            assert!(!ptr.is_null(), "allocator returned null");
            Self { ptr, layout }
        }
        fn as_mut_ptr(&self) -> *mut u8 {
            self.ptr
        }
    }
    impl Drop for AlignedRegion {
        fn drop(&mut self) {
            unsafe { std::alloc::dealloc(self.ptr, self.layout) };
        }
    }

    /// Allocate a heap region big enough for a ring with `ring_size` slots of
    /// `slot_capacity` bytes each, initialize it, and return (receiver, sender_template,
    /// owner) where the owner keeps the region alive. The test drops the owner last.
    fn make_ring(
        ring_size: u32,
        slot_capacity: u32,
    ) -> (DsmMpscReceiver, DsmMpscSender, AlignedRegion) {
        let bytes = DsmMpscRingHeader::region_bytes(ring_size, slot_capacity);
        let region = AlignedRegion::new(bytes);
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

    /// Cache-line layout regression: `head` and `tail` should NOT share a cache line,
    /// and the first slot should not share a line with `tail`. Catches accidental
    /// padding removal that would re-introduce the false-sharing perf cliff this
    /// primitive was built to avoid.
    #[test]
    fn header_layout_is_cache_friendly() {
        use std::mem::offset_of;
        let head_off = offset_of!(DsmMpscRingHeader, head);
        let tail_off = offset_of!(DsmMpscRingHeader, tail);
        let header_size = std::mem::size_of::<DsmMpscRingHeader>();
        assert!(
            (tail_off - head_off) >= CACHE_LINE,
            "head and tail must be on separate cache lines: head_off={head_off}, tail_off={tail_off}, CACHE_LINE={CACHE_LINE}"
        );
        assert!(
            (header_size - tail_off) >= CACHE_LINE,
            "first slot must start on its own cache line: header_size={header_size}, tail_off={tail_off}, CACHE_LINE={CACHE_LINE}"
        );
        // Header itself should be CACHE_LINE-aligned so the padding math holds when
        // the ring starts at the beginning of a DSM segment (DSM regions are
        // page-aligned in PG, so this is satisfied in production).
        assert_eq!(std::mem::align_of::<DsmMpscRingHeader>(), CACHE_LINE);
    }

    /// Magic + version validation: an attach against a region with the wrong magic or
    /// version returns None rather than handing back a NonNull with garbage state.
    /// Mirrors `MppDsmHeader::validate`'s discipline.
    #[test]
    fn attach_at_rejects_wrong_magic_and_version() {
        let bytes = DsmMpscRingHeader::region_bytes(4, 64);
        let region = AlignedRegion::new(bytes);
        let header_ptr = unsafe { create_at(region.as_mut_ptr(), 4, 64) };
        // Sanity: correct attach succeeds.
        assert!(unsafe { attach_at(region.as_mut_ptr(), 4, 64) }.is_some());
        // Corrupt magic: rejected.
        unsafe { (*header_ptr).magic = 0xDEADBEEF };
        assert!(unsafe { attach_at(region.as_mut_ptr(), 4, 64) }.is_none());
        // Restore magic, corrupt version.
        unsafe { (*header_ptr).magic = MPSC_RING_MAGIC };
        unsafe { (*header_ptr).version = MPSC_RING_VERSION.wrapping_add(1) };
        assert!(unsafe { attach_at(region.as_mut_ptr(), 4, 64) }.is_none());
        // Restore version, mismatched ring_size.
        unsafe { (*header_ptr).version = MPSC_RING_VERSION };
        assert!(unsafe { attach_at(region.as_mut_ptr(), 8, 64) }.is_none());
    }

    /// Consumer-side detach race: producers are mid-flight when the receiver detaches.
    /// Every Ok send must produce a matching Bytes recv before the drain returns
    /// Detached. Catches the half-claim-wedge if the predicate is too strict.
    #[test]
    fn drain_completes_after_detach_under_load() {
        const K_PRODUCERS: usize = 4;
        let (rx, tx_template, _region) = make_ring(16, 32);
        let ring_nn = SharedRing(tx_template.ring);
        let stop = Arc::new(AtomicBool::new(false));
        let sent_count = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::with_capacity(K_PRODUCERS);
        for _ in 0..K_PRODUCERS {
            let tx = unsafe { DsmMpscSender::new(ring_nn.0) };
            let stop = Arc::clone(&stop);
            let sent_count = Arc::clone(&sent_count);
            handles.push(std::thread::spawn(move || {
                let payload = [0xABu8; 8];
                while !stop.load(O::Relaxed) {
                    match tx.try_send(&payload) {
                        Ok(_) => {
                            sent_count.fetch_add(1, O::Relaxed);
                        }
                        Err(SendError::Full) => std::thread::yield_now(),
                        Err(SendError::Detached) => break,
                        Err(e) => panic!("unexpected: {e:?}"),
                    }
                }
            }));
        }
        // Drain a bit then trigger detach.
        let mut buf = Vec::new();
        let mut recv_count = 0usize;
        // Let producers stretch their legs before detaching.
        std::thread::sleep(std::time::Duration::from_millis(20));
        rx.set_detached();
        stop.store(true, O::Relaxed);
        // Drain until the ring confirms Detached.
        loop {
            match rx.try_recv(&mut buf) {
                RecvOutcome::Bytes => recv_count += 1,
                RecvOutcome::Empty => std::thread::yield_now(),
                RecvOutcome::Detached => break,
            }
        }
        for h in handles {
            h.join().unwrap();
        }
        let sent = sent_count.load(O::Relaxed);
        assert_eq!(
            recv_count, sent,
            "every successful send must be received before Detached"
        );
    }

    /// Stress test at the production-worst contention level (K=24 producers matching
    /// PR #5155's N=24 mesh row). Smoke that the primitive doesn't wedge or lose
    /// messages under heavy CAS contention; doesn't measure perf.
    #[test]
    fn mpsc_stress_at_production_worst_case() {
        const K_PRODUCERS: usize = 24;
        const M_PER_PRODUCER: u32 = 500;
        let (rx, tx_template, _region) = make_ring(64, 32);
        let ring_nn = SharedRing(tx_template.ring);
        let mut handles = Vec::with_capacity(K_PRODUCERS);
        for producer_id in 0..K_PRODUCERS {
            let tx = unsafe { DsmMpscSender::new(ring_nn.0) };
            handles.push(std::thread::spawn(move || {
                let mut payload = [0u8; 8];
                payload[0..4].copy_from_slice(&(producer_id as u32).to_le_bytes());
                let mut sent = 0u32;
                while sent < M_PER_PRODUCER {
                    payload[4..8].copy_from_slice(&sent.to_le_bytes());
                    match tx.try_send(&payload) {
                        Ok(_) => sent += 1,
                        Err(SendError::Full) => std::thread::yield_now(),
                        Err(e) => panic!("unexpected: {e:?}"),
                    }
                }
            }));
        }
        let mut buf = Vec::new();
        let target = K_PRODUCERS * M_PER_PRODUCER as usize;
        let mut total = 0usize;
        while total < target {
            match rx.try_recv(&mut buf) {
                RecvOutcome::Bytes => total += 1,
                RecvOutcome::Empty => std::thread::yield_now(),
                RecvOutcome::Detached => panic!("unexpected detach"),
            }
        }
        for h in handles {
            h.join().unwrap();
        }
        // Multi-round invariant: after draining 24 * 500 = 12000 messages on a 64-slot
        // ring, every slot's seq must have advanced into a future round. Walk slot[0]:
        // we expect seq = head + ring_size = 12000 + 64 - whatever round 0 it's in.
        // Loose check: every slot's seq is bounded below by ring_size (round >= 1).
        let header = unsafe { ring_nn.0.as_ref() };
        for i in 0..header.ring_size {
            let slot = unsafe { slot_ptr(ring_nn.0.as_ptr(), i, header.slot_capacity) };
            let seq = unsafe { (*slot).seq.load(O::Acquire) };
            assert!(
                seq >= header.ring_size as u64,
                "slot[{i}].seq={seq} < ring_size; slot was never reused"
            );
        }
    }
}
