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

//! DSM-backed MPSC ring for MPP mesh inboxes.
//!
//! PG's `shm_mq` is hard-wired SPSC (asserted in `shm_mq_set_sender`), so one inbox
//! can't be shared across N-1 peers without forking PG. This ring is the replacement:
//! a fixed-size byte-message ring sitting in a `dsm_segment`, MPSC-correct via
//! Vyukov-style per-slot sequence counters.
//!
//! Layout (contiguous bytes, `repr(C)`):
//!
//! ```text
//! +- DsmMpscRingHeader -----------------------+
//! |  magic, version, ring_size, slot_capacity |
//! |  sender_count, detached, receiver_packed  |
//! |  (cache-line padding around head/tail)    |
//! |  head, tail (each on its own line)        |
//! +-------------------------------------------+
//! | Slot[0] | Slot[1] | ... | Slot[N-1]       |
//! +-------------------------------------------+
//!
//! Slot {
//!     seq:  AtomicU64,    // Vyukov phase counter
//!     len:  AtomicU32,
//!     data: [u8; slot_capacity - SLOT_HEADER_SIZE]
//! }
//! ```
//!
//! Slot phase encoding (Vyukov MPMC reduced to MPSC):
//!
//! ```text
//! slot[i] in round k:  seq = k * ring_size + i      // empty
//!                      seq = k * ring_size + i + 1  // ready
//! ```
//!
//! Producer claim at `tail = T`: read `slot[T % ring_size].seq`. `seq == T` then CAS
//! `tail: T → T+1`, winner copies payload and stores `seq = T + 1` (Release). `seq < T`
//! means ring full. `seq > T` means another producer took `T`, retry. Winner
//! `SetLatch`es the receiver.
//!
//! Consumer take at `head = H`: `slot[H % ring_size].seq == H + 1` means ready. Read
//! payload, store `seq = H + ring_size` (next round's empty marker), then `head = H+1`.
//! The single consumer owns `head` without CAS; producers contend only on `tail`.
//!
//! **Safety**: public methods on `DsmMpscSender` / `DsmMpscReceiver` are type-safe once
//! constructed. The constructors are `unsafe` because the DSM region must be correctly
//! sized and not aliased (one process calls `create_at`, everyone else `attach_at`).
//!
//! **Counter wraparound**: head/tail are `u64`, incremented by one per op. At 100M
//! ops/sec that's ~5800 years, so we ignore wrap. The seq math would break under wrap
//! (pre-wrap `seq` would exceed post-wrap `tail` and producers would spin forever); if
//! that ever matters, add a `tail < u64::MAX - margin` check and reset the ring.

use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

#[cfg(not(test))]
use pgrx::pg_sys;

/// Sentinel for an unregistered receiver. Producers skip the wakeup call when the
/// receiver_pgprocno is negative (no receiver set, or in unit tests that don't have a
/// PG backend). Matches PG core's `INVALID_PROC_NUMBER`.
pub(super) const NO_RECEIVER: i32 = -1;

/// Pack `pgprocno` (low 32 bits) + `pid` (high 32 bits) into one `u64` so a producer's
/// single `Acquire` load can't observe a torn `(new_pgprocno, old_pid)` or
/// `(old_pgprocno, new_pid)` pair. Without packing, a producer that observed a
/// receiver-renewal mid-update could either skip a legitimate wake or wake the wrong
/// backend.
#[inline]
fn pack_receiver(pgprocno: i32, pid: i32) -> u64 {
    ((pid as u32 as u64) << 32) | (pgprocno as u32 as u64)
}

#[inline]
fn unpack_receiver(packed: u64) -> (i32, i32) {
    (packed as u32 as i32, (packed >> 32) as u32 as i32)
}

/// Reserved byte at the start of every slot. Sized so payload + header fits in
/// `slot_capacity` exactly.
const SLOT_HEADER_BYTES: usize = std::mem::size_of::<SlotHeader>();

#[repr(C)]
struct SlotHeader {
    /// Vyukov sequence counter; see module docs for phase encoding.
    seq: AtomicU64,
    /// Bytes of payload written into THIS slot's data region;
    /// `0..=slot_capacity - SLOT_HEADER_BYTES`.
    len: AtomicU32,
    /// Fragment metadata. Low 2 bits = [`FragmentKind`] (Complete=0 / First=1 /
    /// Continue=2 / Last=3). For `First`, bits 16..32 hold the slot count of the
    /// logical frame (1..=65535). For other kinds these bits are unused.
    flags: AtomicU32,
}

/// Kind bits stored in [`SlotHeader::flags`]'s low 2 bits.
///
/// A frame fitting in `slot_capacity - SLOT_HEADER_BYTES` rides the single-slot
/// fast path as `Complete`. Anything larger spans
/// `n_slots = ceil(frame_len / per_slot_payload)` consecutive slots: `First`
/// (carrying `n_slots` in the upper bits), `Continue` for the middle, `Last` for
/// the tail. Producers grab the whole run with one
/// `tail.compare_exchange(T, T + n_slots)`, so other producers can't interleave
/// their fragments and the receiver always sees the slots in producer order.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FragmentKind {
    Complete = 0,
    First = 1,
    Continue = 2,
    Last = 3,
}

/// Bit-mask for the kind bits in [`SlotHeader::flags`].
const FLAGS_KIND_MASK: u32 = 0b11;
/// Shift for the `n_slots` field stored in the flags' upper half (only meaningful
/// for `First`). Limits per-frame fragmentation to 65535 slots, which is far
/// beyond any realistic ring size.
const FLAGS_NSLOTS_SHIFT: u32 = 16;
const FLAGS_NSLOTS_MAX: u32 = 0xFFFF;

#[inline]
fn pack_flags(kind: FragmentKind, n_slots: u32) -> u32 {
    debug_assert!(
        kind != FragmentKind::First || (1..=FLAGS_NSLOTS_MAX).contains(&n_slots),
        "First frames must carry 1..=65535 slots; got {n_slots}"
    );
    (kind as u32) | (n_slots << FLAGS_NSLOTS_SHIFT)
}

#[inline]
fn unpack_kind(flags: u32) -> Option<FragmentKind> {
    match flags & FLAGS_KIND_MASK {
        0 => Some(FragmentKind::Complete),
        1 => Some(FragmentKind::First),
        2 => Some(FragmentKind::Continue),
        3 => Some(FragmentKind::Last),
        _ => None,
    }
}

#[inline]
fn unpack_nslots(flags: u32) -> u32 {
    flags >> FLAGS_NSLOTS_SHIFT
}

/// Magic constant validating that an attaching process points at a `DsmMpscRingHeader`
/// rather than garbage. "MPCR" = MPSC Ring. Different value from `MppDsmHeader`'s magic
/// so a worker that picks up the wrong region fails the wrong-shape check loudly.
const MPSC_RING_MAGIC: u32 = u32::from_le_bytes(*b"MPCR");

/// Bump on any wire-incompatible layout change. Mirrors the discipline at
/// `MppDsmHeader::validate`.
///
/// Wire versions:
/// - v1: `SlotHeader { seq, len, _pad }`. One frame per slot.
/// - v2: `SlotHeader { seq, len, flags }`. `flags` carries `FragmentKind` plus
///   `n_slots` on `First`, so frames bigger than
///   `slot_capacity - SLOT_HEADER_BYTES` can span N consecutive slots reserved
///   atomically by the producer.
const MPSC_RING_VERSION: u32 = 2;

/// Assumed cache line size for false-sharing avoidance. 64 bytes covers x86_64 and arm64;
/// over-padding on smaller-cache-line targets costs a few bytes per ring, nothing more.
const CACHE_LINE: usize = 64;

/// Ring header. Laid out first in the DSM region; slot array follows immediately after.
///
/// Cache-line padding around `head` and `tail` isn't optional: at N=24 producer
/// contention the consumer's `head.store` and the producers' `tail.compare_exchange`
/// race on the same line, MESI-ping-ponging every claim. That's the false-sharing
/// footgun Vyukov's writeups call out; the Disruptor literature shows 5-10x throughput
/// loss from it on x86. Padding puts each hot field on its own line.
///
/// NOT `#[repr(C, align(64))]`: PG `dsm_segment` base addresses are only
/// MAXALIGN-aligned in practice (on macOS the user-data offset can land 16-aligned
/// but not 64-aligned). Forcing `align(64)` would impose a 64-aligned destination on
/// `create_at` / `attach_at` we can't guarantee. The `_pad_*` fields below still
/// put `head`, `tail`, and the first slot on separate 64-byte regions; it's the
/// *distance* between hot fields that matters, not their absolute alignment.
#[repr(C)]
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
    /// Live `DsmMpscSender` count. Incremented in `DsmMpscSender::new`, decremented in
    /// `DsmMpscSender::Drop`. The drop that takes the count from 1 → 0 sets `detached`
    /// (with Release) and wakes the receiver, mirroring shm_mq's "drop = detach"
    /// structural guarantee.
    sender_count: AtomicU32,
    /// Set by the consumer (or by the leader on query teardown) to tell producers to
    /// fail-fast on subsequent sends. Sticky.
    detached: AtomicBool,
    _pad_after_detached: [u8; 3],
    /// Packed `(pgprocno: i32, pid: i32)` of the registered receiver, or 0 (both
    /// fields zero / pgprocno not NO_RECEIVER but treated as unset by `wake_receiver`
    /// because `unpack` returns pgprocno=0 which is < NO_RECEIVER=−1 check). The
    /// `0_u64` initial state encodes `(pgprocno=0, pid=0)`. pgprocno=0 is the
    /// postmaster slot, so we still need an explicit `NO_RECEIVER` (-1) to indicate
    /// "unset"; producers check the pgprocno field for `>= 0 && < allProcCount`
    /// before resolving.
    ///
    /// Packing both fields into one atomic eliminates the torn-read scenario where a
    /// producer would otherwise observe `(new_pgprocno, old_pid)` mid-update and
    /// either skip a legitimate wake or wake the wrong backend.
    receiver_packed: AtomicU64,
    /// Padding to push `head` onto its own cache line. Header up to here uses bytes
    /// 0..32; this padding fills 32..64 so `head` lands at offset 64 exactly. The
    /// `header_layout_is_cache_friendly` test asserts this.
    _pad_before_head: [u8; CACHE_LINE - 32],
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

/// Wake the receiver if one is registered. Resolves the latch from
/// `ProcGlobal->allProcs[receiver_pgprocno]` per call (so PGPROC reuse after a receiver
/// exit can't dangle a cached pointer) and checks `proc->pid` against the stored pid
/// so a wake targeted at a reused slot's new tenant gets skipped.
///
/// Silently returns when no receiver is registered, `ProcGlobal` is null (tests), or
/// the pid check fails. The pid guard narrows the wrong-backend risk from "any reused
/// slot" to "a reused slot whose new pid happens to match the old one", which is
/// negligible.
///
/// # Safety
/// Caller must guarantee the ring is still mapped.
///
/// # Threading
/// `pg_sys::SetLatch` is wrapped by pgrx with `check_active_thread()`, which panics
/// off the backend thread. Producer wakes fire from the producer's own backend main
/// thread (the plan-node poll), so this passes. PG's `SetLatch` is itself cross-thread
/// safe; the constraint lives in the pgrx wrapper.
///
/// `pgprocno + pid` instead of `BackendPidGetProc(pid)` because the latter scans the
/// proc array under `ProcArrayLock`. The packed check is O(1) lock-free; both approaches
/// miss-wake the same way if PG recycles a PID into a different slot.
unsafe fn wake_receiver(header: &DsmMpscRingHeader) {
    let (pgprocno, expected_pid) = unpack_receiver(header.receiver_packed.load(Ordering::Acquire));
    if pgprocno < 0 {
        return;
    }
    // PG FFI lives behind a cfg gate so the unit-test binary doesn't need ProcGlobal
    // resolved at load time. The macOS flat-namespace linker aborts at process start
    // on an unresolved extern static, so any code path that references ProcGlobal
    // must be excluded from the test binary entirely. Do NOT copy this cfg pattern
    // to other PG-FFI sites without a similar load-time-symbol-resolution reason;
    // it's not a general "tests can't touch PG FFI" workaround.
    #[cfg(not(test))]
    unsafe {
        wake_receiver_via_pg_sys(pgprocno, expected_pid);
    }
    #[cfg(test)]
    {
        // Tests should never reach here with a real pgprocno. `set_receiver` is a
        // no-op in test builds (the cfg gate above strips the body). If a future test
        // calls `set_receiver` and lands here with pgprocno >= 0, the test silently
        // no-ops instead of exercising the wake path. Catch that footgun loudly.
        debug_assert_eq!(
            pgprocno, NO_RECEIVER,
            "wake_receiver invoked with real pgprocno={pgprocno} in test build; the \
             pg_sys-side wake is cfg'd out, so this would silently no-op. Tests \
             targeting the wake path must run under `cargo pgrx test`."
        );
        let _ = expected_pid;
    }
}

#[cfg(not(test))]
unsafe fn wake_receiver_via_pg_sys(pgprocno: i32, expected_pid: i32) {
    let proc_global = unsafe { pg_sys::ProcGlobal };
    if proc_global.is_null() {
        return;
    }
    let all_proc_count = unsafe { (*proc_global).allProcCount };
    // Defense in depth: a corrupted receiver_packed in DSM (any attached backend can
    // write) with a wildly out-of-range pgprocno would otherwise indexing-past-the-
    // array. The pgprocno check in wake_receiver above guards the negative range;
    // this guards the positive range against allProcs's actual size.
    if pgprocno < 0 || (pgprocno as u32) >= all_proc_count {
        return;
    }
    let all_procs = unsafe { (*proc_global).allProcs };
    if all_procs.is_null() {
        return;
    }
    let proc = unsafe { all_procs.add(pgprocno as usize) };
    let current_pid = unsafe { (*proc).pid };
    if current_pid != expected_pid {
        // PGPROC slot was reused for a different backend (or never assigned); skip
        // wake to avoid disturbing the unrelated tenant.
        return;
    }
    // PGPROC owns the Latch by value at `procLatch`; we want a `*mut Latch` pointing
    // into that slot, not a load of the field.
    unsafe { pg_sys::SetLatch(&raw mut (*proc).procLatch) };
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
// thread calls `try_recv` at a time is what makes the lock-free MPSC math correct (the
// single consumer owns `head` without CAS). `DsmMpscSender` is Sync so multiple producer
// threads can share one `Arc<DsmMpscSender>` and race on `tail` via CAS.
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
    // The header's natural alignment (from its largest field, AtomicU64) is 8 bytes.
    // PG MAXALIGN is 8 on every supported platform, so dsm_segment user-data offsets
    // computed via `align_up_maxalign_checked` always satisfy this. Tests use an
    // aligned-Vec helper just to keep the assertion clean across allocators.
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
                sender_count: AtomicU32::new(0),
                receiver_packed: AtomicU64::new(pack_receiver(NO_RECEIVER, 0)),
                _pad_before_head: [0; CACHE_LINE - 32],
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
                    flags: AtomicU32::new(0),
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
    /// Wrap an already-initialized ring as the single consumer. Pairs with
    /// [`DsmMpscSender::new`] on the producer side; calling code is responsible for
    /// keeping exactly one `DsmMpscReceiver` per ring.
    ///
    /// # Safety
    /// `ring` must point to a header initialized by [`create_at`] and not yet
    /// deallocated. The caller guarantees no other `DsmMpscReceiver` exists for the
    /// same ring (single-consumer invariant).
    pub(super) unsafe fn new(ring: NonNull<DsmMpscRingHeader>) -> Self {
        Self { ring }
    }

    /// Register this process as the receiver. Producers' wakeups resolve the latch
    /// from `ProcGlobal->allProcs[pgprocno]` and check `proc->pid == pid` to defend
    /// against PGPROC slot reuse after the receiver exits.
    ///
    /// Caller passes the local process's pgprocno and pid (PG calls these
    /// `MyProcNumber` and `MyProc->pid`). They get packed into one `AtomicU64`,
    /// stored Release; producers Acquire-load the pair so they never see a torn
    /// `(pgprocno, pid)`.
    pub(super) fn set_receiver(&self, pgprocno: i32, pid: i32) {
        let header = unsafe { self.ring.as_ref() };
        header
            .receiver_packed
            .store(pack_receiver(pgprocno, pid), Ordering::Release);
    }

    /// Try to read one frame into `out`. `Bytes`: `out` holds the payload. `Empty`:
    /// caller should yield and retry. `Detached`: ring drained, all producers gone,
    /// no more frames coming.
    ///
    /// Known wedge: if a producer CAS-advances `tail` then exits/crashes before
    /// publishing `seq`, the consumer sees `tail > head`, the slot's `seq` stuck at
    /// the prior-round empty marker, and `detached && tail <= head` never becomes
    /// true. The drain returns `Empty` forever. In production PG's parallel-worker
    /// death handling bounds this (worker exit fires leader ERROR, DSM tears down),
    /// but a future pass should add an explicit `PGPROC` liveness check to
    /// force-detach on producer death.
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
        // Slot at head is ready. Inspect its kind to choose between single-slot
        // fast path and multi-slot reassembly.
        let flags = unsafe { (*slot).flags.load(Ordering::Relaxed) };
        let Some(kind) = unpack_kind(flags) else {
            // Reserved bits set; treat as corruption.
            header.detached.store(true, Ordering::Release);
            return RecvOutcome::Detached;
        };
        let payload_cap = (header.slot_capacity as usize).saturating_sub(SLOT_HEADER_BYTES);
        match kind {
            FragmentKind::Complete => self.recv_single_slot(header, head, slot, payload_cap, out),
            FragmentKind::First => {
                let n_slots = unpack_nslots(flags);
                if n_slots == 0 || n_slots > header.ring_size {
                    header.detached.store(true, Ordering::Release);
                    return RecvOutcome::Detached;
                }
                self.recv_multi_slot(header, head, n_slots, payload_cap, out)
            }
            FragmentKind::Continue | FragmentKind::Last => {
                // Encountering Continue/Last at `head` means a producer violated
                // ascending-publish ordering or a previous reassembly didn't advance
                // `head` past every fragment. Either is a contract break, not a
                // recoverable condition; poison and detach.
                header.detached.store(true, Ordering::Release);
                RecvOutcome::Detached
            }
        }
    }

    /// Read a single-slot `Complete` frame at `head` and advance.
    fn recv_single_slot(
        &self,
        header: &DsmMpscRingHeader,
        head: u64,
        slot: *mut SlotHeader,
        payload_cap: usize,
        out: &mut Vec<u8>,
    ) -> RecvOutcome {
        let len_raw = unsafe { (*slot).len.load(Ordering::Relaxed) } as usize;
        // Clamp against slot's payload capacity. DSM is mapped writable by every
        // attached backend, so a buggy / corrupted producer could write a garbage len.
        // Without this guard, `set_len + copy_nonoverlapping` would read OOB into
        // neighboring slots or other DSM contents.
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

    /// Reassemble an `n_slots`-fragment frame at `head`. Returns `Empty` without
    /// advancing `head` if any continuation slot isn't yet published (producer
    /// mid-publish); the caller retries and the `First` slot stays put with its
    /// flags intact.
    fn recv_multi_slot(
        &self,
        header: &DsmMpscRingHeader,
        head: u64,
        n_slots: u32,
        payload_cap: usize,
        out: &mut Vec<u8>,
    ) -> RecvOutcome {
        // First pass: verify every fragment in the run is published. If not, bail
        // with Empty so the caller can retry once the producer finishes.
        let mut total_len: usize = 0;
        for i in 0..n_slots {
            let h = head.wrapping_add(i as u64);
            let slot_idx = (h % header.ring_size as u64) as u32;
            let slot = unsafe { slot_ptr(self.ring.as_ptr(), slot_idx, header.slot_capacity) };
            let seq = unsafe { (*slot).seq.load(Ordering::Acquire) };
            if seq != h.wrapping_add(1) {
                // Continuation slot not yet ready. Don't touch `head` or any slot
                // metadata; producer will publish soon, caller will retry.
                return RecvOutcome::Empty;
            }
            let expected_kind = if i == 0 {
                FragmentKind::First
            } else if i + 1 == n_slots {
                FragmentKind::Last
            } else {
                FragmentKind::Continue
            };
            let flags = unsafe { (*slot).flags.load(Ordering::Relaxed) };
            if unpack_kind(flags) != Some(expected_kind) {
                // Run integrity check: the producer's multi-slot publish never
                // interleaves with another producer's, so the kind sequence must be
                // First, Continue*, Last. Anything else is corruption.
                header.detached.store(true, Ordering::Release);
                return RecvOutcome::Detached;
            }
            let len = unsafe { (*slot).len.load(Ordering::Relaxed) } as usize;
            if len > payload_cap || (i + 1 < n_slots && len != payload_cap) {
                // Non-final fragments must be full slots (producer fills them
                // first); final fragment may be partial. Anything else is
                // corruption.
                header.detached.store(true, Ordering::Release);
                return RecvOutcome::Detached;
            }
            total_len = match total_len.checked_add(len) {
                Some(v) => v,
                None => {
                    header.detached.store(true, Ordering::Release);
                    return RecvOutcome::Detached;
                }
            };
        }
        // Second pass: concatenate payloads, mark each slot empty for next round,
        // advance head past the whole run in one Release store.
        out.clear();
        out.reserve(total_len);
        for i in 0..n_slots {
            let h = head.wrapping_add(i as u64);
            let slot_idx = (h % header.ring_size as u64) as u32;
            let slot = unsafe { slot_ptr(self.ring.as_ptr(), slot_idx, header.slot_capacity) };
            let len = unsafe { (*slot).len.load(Ordering::Relaxed) } as usize;
            let data = unsafe { slot_data_ptr(slot) };
            unsafe {
                let write_at = out.as_mut_ptr().add(out.len());
                std::ptr::copy_nonoverlapping(data, write_at, len);
                out.set_len(out.len() + len);
            }
            // Round k empty marker for slot at position h is h + ring_size.
            let next_empty_seq = h.wrapping_add(header.ring_size as u64);
            unsafe { (*slot).seq.store(next_empty_seq, Ordering::Release) };
        }
        header
            .head
            .store(head.wrapping_add(n_slots as u64), Ordering::Release);
        RecvOutcome::Bytes
    }
}

impl DsmMpscSender {
    /// Wrap an already-initialized ring as a producer. Multiple `DsmMpscSender`
    /// handles to the same ring are legal (and the point of MPSC). Increments the
    /// ring's `sender_count`; the `Drop` impl decrements and, on the last drop,
    /// flips `detached` and wakes the receiver. That mirrors shm_mq's
    /// "drop the sender, receiver sees detach" structural guarantee.
    ///
    /// # Safety
    /// `ring` must point to a header initialized by [`create_at`] and not yet
    /// deallocated.
    pub(super) unsafe fn new(ring: NonNull<DsmMpscRingHeader>) -> Self {
        let header = unsafe { ring.as_ref() };
        header.sender_count.fetch_add(1, Ordering::AcqRel);
        Self { ring }
    }

    /// Push one frame onto the ring. Returns immediately:
    /// - `Ok(())`: published; receiver's latch was set if installed.
    /// - `Err(Full)`: no slot run available right now; caller yields + retries.
    /// - `Err(Detached)`: receiver has detached; caller stops.
    /// - `Err(MessageTooLarge)`: frame needs more than `ring_size` slots
    ///   (max writable size is `ring_size * (slot_capacity - SLOT_HEADER_BYTES)`).
    ///
    /// Frames up to the per-slot payload capacity take the single-slot fast path.
    /// Larger frames span consecutive slots claimed atomically via one
    /// `tail.compare_exchange(T, T + n_slots)`, so other producers can't
    /// interleave their fragments. See [`FragmentKind`].
    pub(super) fn try_send(&self, bytes: &[u8]) -> Result<(), SendError> {
        let header = unsafe { self.ring.as_ref() };
        if header.detached.load(Ordering::Acquire) {
            return Err(SendError::Detached);
        }
        let payload_cap = (header.slot_capacity as usize).saturating_sub(SLOT_HEADER_BYTES);
        if payload_cap == 0 {
            return Err(SendError::MessageTooLarge);
        }
        // Single-slot fast path: cheaper than the multi-slot CAS dance and preserves
        // the v1 hot path exactly so the no-fragmentation case takes no extra branches
        // inside the claim loop.
        if bytes.len() <= payload_cap {
            return self.try_send_single_slot(header, bytes);
        }
        // Multi-slot path: fragment across `n_slots` consecutive slots.
        let n_slots_usize = bytes.len().div_ceil(payload_cap);
        if n_slots_usize > header.ring_size as usize || n_slots_usize > FLAGS_NSLOTS_MAX as usize {
            // Frame is larger than the entire ring (or larger than the n_slots field
            // can encode). Either bump `mpp_queue_size` or land a chunked-stream
            // protocol that spans rounds.
            return Err(SendError::MessageTooLarge);
        }
        let n_slots = n_slots_usize as u32;
        self.try_send_multi_slot(header, bytes, n_slots, payload_cap)
    }

    /// Single-slot path. Identical to the v1 layout's hot path with a `flags` write
    /// added; the kind is always `Complete` here.
    fn try_send_single_slot(
        &self,
        header: &DsmMpscRingHeader,
        bytes: &[u8],
    ) -> Result<(), SendError> {
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
                                (*slot).flags.store(
                                    pack_flags(FragmentKind::Complete, 0),
                                    Ordering::Relaxed,
                                );
                                let data = slot_data_ptr(slot);
                                std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
                                // Publish: ready in round k is (k * ring_size + i + 1) = tail + 1.
                                (*slot).seq.store(tail.wrapping_add(1), Ordering::Release);
                            }
                            // Wake the consumer (resolves the latch via pgprocno + pid).
                            unsafe { wake_receiver(header) };
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

    /// Multi-slot path. CAS-advance `tail` from `T` to `T + n_slots` to claim the run,
    /// then publish each fragment ascending with `First` / `Continue` / `Last` flags.
    /// The CAS requires all N target slots to already be in their expected empty
    /// round, which generalizes v1's per-slot `seq == tail` check to a run.
    ///
    /// Ascending publish order isn't optional: the receiver only starts reassembling
    /// once `slot[head]` (the `First`) is ready, then waits for each subsequent slot.
    /// Out-of-order publish would make the receiver block on a `Continue` whose data
    /// is already there but whose `seq` hasn't been stored yet, wasting one drain
    /// pass per fragment.
    fn try_send_multi_slot(
        &self,
        header: &DsmMpscRingHeader,
        bytes: &[u8],
        n_slots: u32,
        payload_cap: usize,
    ) -> Result<(), SendError> {
        loop {
            let tail = header.tail.load(Ordering::Acquire);
            // Verify every target slot is currently in its expected empty round.
            // Slot at position (tail + i) % ring_size in round-of-(tail+i) has empty
            // marker == (tail + i). If ANY is not empty, the ring is too contended /
            // not drained enough for a run of this length.
            let mut all_empty = true;
            for i in 0..n_slots {
                let t = tail.wrapping_add(i as u64);
                let slot_idx = (t % header.ring_size as u64) as u32;
                let slot = unsafe { slot_ptr(self.ring.as_ptr(), slot_idx, header.slot_capacity) };
                let seq = unsafe { (*slot).seq.load(Ordering::Acquire) };
                match seq.cmp(&t) {
                    std::cmp::Ordering::Equal => {}
                    std::cmp::Ordering::Less => {
                        // Slot at offset i hasn't been reclaimed by the consumer for
                        // round-of-t. Run not available right now.
                        return Err(SendError::Full);
                    }
                    std::cmp::Ordering::Greater => {
                        // Another producer already claimed (T + i); our view of `tail`
                        // is stale, retry.
                        all_empty = false;
                        break;
                    }
                }
            }
            if !all_empty {
                continue;
            }
            // All N target slots are empty in our round. Atomically claim the run.
            match header.tail.compare_exchange_weak(
                tail,
                tail.wrapping_add(n_slots as u64),
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // We own slots tail..tail+n_slots. Write each fragment and
                    // publish its seq in ascending order so the receiver can drain
                    // them as soon as the prefix is ready.
                    let mut offset = 0usize;
                    for i in 0..n_slots {
                        let t = tail.wrapping_add(i as u64);
                        let slot_idx = (t % header.ring_size as u64) as u32;
                        let slot =
                            unsafe { slot_ptr(self.ring.as_ptr(), slot_idx, header.slot_capacity) };
                        let remaining = bytes.len() - offset;
                        let chunk_len = remaining.min(payload_cap);
                        let kind = if i == 0 {
                            FragmentKind::First
                        } else if i + 1 == n_slots {
                            FragmentKind::Last
                        } else {
                            FragmentKind::Continue
                        };
                        let flags_word = if kind == FragmentKind::First {
                            pack_flags(kind, n_slots)
                        } else {
                            pack_flags(kind, 0)
                        };
                        unsafe {
                            (*slot).len.store(chunk_len as u32, Ordering::Relaxed);
                            (*slot).flags.store(flags_word, Ordering::Relaxed);
                            let data = slot_data_ptr(slot);
                            std::ptr::copy_nonoverlapping(
                                bytes.as_ptr().add(offset),
                                data,
                                chunk_len,
                            );
                            // Publish: ready in round-of-t for slot at position t is t + 1.
                            (*slot).seq.store(t.wrapping_add(1), Ordering::Release);
                        }
                        offset += chunk_len;
                    }
                    debug_assert_eq!(offset, bytes.len(), "multi-slot send copied wrong length");
                    // Wake the consumer once for the whole run; the drain pass that
                    // wakes for the First slot will keep going through all our
                    // already-published Continue/Last fragments without sleeping.
                    unsafe { wake_receiver(header) };
                    return Ok(());
                }
                Err(_) => continue,
            }
        }
    }
}

impl Drop for DsmMpscSender {
    fn drop(&mut self) {
        let header = unsafe { self.ring.as_ref() };
        // AcqRel: decrement is observed by other producers (they don't care, but the
        // Release pairs with the receiver's Acquire load on `detached` below).
        let prev = header.sender_count.fetch_sub(1, Ordering::AcqRel);
        if prev == 1 {
            // We were the last sender. Tell the consumer.
            header.detached.store(true, Ordering::Release);
            unsafe { wake_receiver(header) };
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

    /// Owning aligned region for a ring. The default `Vec<u8>` allocator can't
    /// promise the alignment the `create_at` write wants, so allocate via the global
    /// allocator with an explicit `Layout` and free in `Drop`. Production uses PG
    /// `dsm_segment` (page-aligned), so this dance is test-only.
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
    /// `slot_capacity` bytes each, initialize it, and return `(owner, receiver,
    /// sender_template)`. The owner is returned first so callers bind it first.
    /// Rust drops locals in reverse declaration order, so `_region` bound first drops
    /// last, after the handles whose `Drop` impls touch the region's memory. Reverse
    /// that order and `_region` frees the bytes before `tx_template`'s
    /// `sender_count.fetch_sub` runs, which is undefined behavior and surfaces as a
    /// stochastic crash at process teardown.
    fn make_ring(
        ring_size: u32,
        slot_capacity: u32,
    ) -> (AlignedRegion, DsmMpscReceiver, DsmMpscSender) {
        let bytes = DsmMpscRingHeader::region_bytes(ring_size, slot_capacity);
        let region = AlignedRegion::new(bytes);
        let header_ptr = unsafe { create_at(region.as_mut_ptr(), ring_size, slot_capacity) };
        let nn = NonNull::new(header_ptr).expect("create_at returned null");
        // Unsafe: we hand ownership of the same pointer to two handles. Safe because the
        // ring is the synchronization point; the handles are stateless wrappers.
        let receiver = unsafe { DsmMpscReceiver::new(nn) };
        let sender = unsafe { DsmMpscSender::new(nn) };
        (region, receiver, sender)
    }

    #[test]
    fn spsc_round_trip_under_capacity() {
        let (_region, rx, tx) = make_ring(4, 64);
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
        let (_region, rx, tx) = make_ring(4, 64);
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
    fn message_too_large_is_rejected() {
        // `MessageTooLarge` only fires when the frame would need more than `ring_size`
        // slots. Per-slot oversize splits across consecutive slots instead.
        let (_region, _rx, tx) = make_ring(2, 32);
        let payload_cap = 32 - SLOT_HEADER_BYTES;
        // payload_cap + 1 byte: now spans 2 slots, fits in ring_size=2.
        let two_slot = vec![0u8; payload_cap + 1];
        tx.try_send(&two_slot).unwrap();
        // ring_size * payload_cap + 1 byte: needs 3 slots, ring only has 2.
        let oversize = vec![0u8; 2 * payload_cap + 1];
        assert_eq!(tx.try_send(&oversize), Err(SendError::MessageTooLarge));
        // Exactly at the ring-wide cap is fine. (We just sent a 2-slot frame above,
        // so we need to drain it first to free the slots; the receiver only exists
        // in this test for that.)
    }

    /// Multi-producer concurrent send: K threads each push M unique messages; consumer
    /// receives K*M messages and verifies every (producer, sequence_in_producer) pair
    /// appears exactly once.
    #[test]
    fn mpsc_no_lost_messages_under_contention() {
        const K_PRODUCERS: usize = 8;
        const M_PER_PRODUCER: u32 = 2000;
        let (_region, rx, tx_template) = make_ring(64, 32);
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
        let (_region, rx, tx_template) = make_ring(32, 32);
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

    /// Cache-line layout regression: `head` must land on its own cache line
    /// (offset == CACHE_LINE so the preceding header fields can't false-share),
    /// `tail` separated from `head` by CACHE_LINE, and the first slot not sharing a
    /// line with `tail`. Catches accidental padding removal or field reorder that
    /// would re-introduce the false-sharing perf cliff the padding exists to avoid.
    #[test]
    fn header_layout_is_cache_friendly() {
        use std::mem::offset_of;
        let head_off = offset_of!(DsmMpscRingHeader, head);
        let tail_off = offset_of!(DsmMpscRingHeader, tail);
        let header_size = std::mem::size_of::<DsmMpscRingHeader>();
        // head at exactly CACHE_LINE offset means the preceding 64 bytes (header
        // fields + pad) live entirely in cache line 0, head + its pad live in
        // cache line 1, etc. Anything other than equality means the padding
        // math drifted; fix it rather than relaxing the assertion.
        assert_eq!(
            head_off, CACHE_LINE,
            "head must land at offset {CACHE_LINE} (its own cache line); got {head_off}"
        );
        assert!(
            (tail_off - head_off) >= CACHE_LINE,
            "head and tail must be on separate cache lines: head_off={head_off}, tail_off={tail_off}, CACHE_LINE={CACHE_LINE}"
        );
        assert!(
            (header_size - tail_off) >= CACHE_LINE,
            "first slot must start on its own cache line: header_size={header_size}, tail_off={tail_off}, CACHE_LINE={CACHE_LINE}"
        );
        // Header's natural alignment is determined by its largest field (AtomicU64,
        // 8 bytes). Cache-line padding between hot fields still keeps them on separate
        // absolute lines regardless of the struct's starting alignment, as long as the
        // inter-field distance is >= CACHE_LINE.
        assert_eq!(std::mem::align_of::<DsmMpscRingHeader>(), 8);
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

    /// Stress test at the production-worst contention level (K=24 producers, matching
    /// the largest mesh-row size the transport drives in practice). Smoke that the
    /// primitive doesn't wedge or lose messages under heavy CAS contention; doesn't
    /// measure perf.
    #[test]
    fn mpsc_stress_at_production_worst_case() {
        const K_PRODUCERS: usize = 24;
        const M_PER_PRODUCER: u32 = 500;
        let (_region, rx, tx_template) = make_ring(64, 32);
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

    // ----- multi-slot fragmentation tests -----

    /// Two-slot frame round-trip: send a payload that's exactly `2*payload_cap`
    /// bytes long with distinct first-half and second-half markers, then verify the
    /// receiver reassembles them in order with no truncation or duplication.
    #[test]
    fn multi_slot_two_fragment_round_trip() {
        let slot_cap = 64;
        let payload_cap = slot_cap - SLOT_HEADER_BYTES;
        let (_region, rx, tx) = make_ring(4, slot_cap as u32);
        let mut frame = vec![0u8; 2 * payload_cap];
        for (i, b) in frame.iter_mut().enumerate() {
            *b = (i & 0xFF) as u8;
        }
        tx.try_send(&frame).unwrap();
        let mut buf = Vec::new();
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Bytes);
        assert_eq!(buf.len(), frame.len());
        assert_eq!(buf, frame);
        // Ring fully drained.
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Empty);
    }

    /// Frame at the multi-slot ceiling: needs exactly `ring_size` slots, succeeds;
    /// one byte more, rejected with MessageTooLarge.
    #[test]
    fn multi_slot_max_size_succeeds_one_more_rejected() {
        let slot_cap = 64;
        let payload_cap = slot_cap - SLOT_HEADER_BYTES;
        let ring_size = 4;
        let (_region, rx, tx) = make_ring(ring_size, slot_cap as u32);
        let max = vec![0xABu8; ring_size as usize * payload_cap];
        tx.try_send(&max).unwrap();
        let mut buf = Vec::new();
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Bytes);
        assert_eq!(buf, max);
        let too_big = vec![0u8; ring_size as usize * payload_cap + 1];
        assert_eq!(tx.try_send(&too_big), Err(SendError::MessageTooLarge));
    }

    /// Multi-slot followed by single-slot from the same producer: each frame must
    /// come out of the receiver as a separate, intact byte sequence; fragments
    /// don't leak into the following frame.
    #[test]
    fn multi_slot_then_single_slot_preserves_boundaries() {
        let slot_cap = 64;
        let payload_cap = slot_cap - SLOT_HEADER_BYTES;
        let (_region, rx, tx) = make_ring(8, slot_cap as u32);
        let big = vec![0xAAu8; 3 * payload_cap];
        tx.try_send(&big).unwrap();
        let small = b"hi".to_vec();
        tx.try_send(&small).unwrap();
        let mut buf = Vec::new();
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Bytes);
        assert_eq!(buf, big);
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Bytes);
        assert_eq!(buf, small);
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Empty);
    }

    /// Multi-slot frame wrapping the ring boundary. With ring_size=4, tail starts at 0:
    /// send a single-slot first to advance tail to 1, drain it, then send a 3-slot
    /// frame at tail=1 occupying slots 1, 2, 3, drain the small frame to advance head,
    /// then send another single-slot at tail=4 (slot 0 in round 1) and a 3-slot at
    /// tail=5 (slots 1, 2, 3 in round 1). Verifies wraparound math holds.
    #[test]
    fn multi_slot_wraparound() {
        let slot_cap = 64;
        let payload_cap = slot_cap - SLOT_HEADER_BYTES;
        let (_region, rx, tx) = make_ring(4, slot_cap as u32);
        let mut buf = Vec::new();
        // Push a small frame so the next multi-slot reservation starts mid-ring.
        tx.try_send(b"warm").unwrap();
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Bytes);
        assert_eq!(buf, b"warm");
        // 3-slot frame at tail=1, wrapping at slot 3 -> slot 0 won't happen here
        // (slots 1, 2, 3 are still in round 0). Drain it.
        let big = vec![0x11u8; 3 * payload_cap];
        tx.try_send(&big).unwrap();
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Bytes);
        assert_eq!(buf, big);
        // Now tail=4, head=4. Send another small frame at slot 0 (round 1, seq=4).
        tx.try_send(b"two").unwrap();
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Bytes);
        assert_eq!(buf, b"two");
        // And a 3-slot at tail=5 occupying slots 1, 2, 3 in round 1 (seqs 5, 6, 7).
        let big2 = vec![0x22u8; 3 * payload_cap];
        tx.try_send(&big2).unwrap();
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Bytes);
        assert_eq!(buf, big2);
        assert_eq!(rx.try_recv(&mut buf), RecvOutcome::Empty);
    }

    /// Stress: multiple producers each push a mix of single-slot and multi-slot
    /// frames; consumer verifies every frame's contents and per-producer ordering.
    /// Catches interleave bugs (a multi-slot run getting mixed with another
    /// producer's fragments) and missing wake-ups under high contention.
    #[test]
    fn multi_slot_stress_mixed_with_singles() {
        const K_PRODUCERS: usize = 6;
        const M_PER_PRODUCER: u32 = 300;
        let slot_cap = 128;
        let payload_cap = slot_cap - SLOT_HEADER_BYTES;
        let (_region, rx, tx_template) = make_ring(16, slot_cap as u32);
        let ring_nn = SharedRing(tx_template.ring);
        let mut handles = Vec::with_capacity(K_PRODUCERS);
        for producer_id in 0..K_PRODUCERS {
            let tx = unsafe { DsmMpscSender::new(ring_nn.0) };
            handles.push(std::thread::spawn(move || {
                let mut sent = 0u32;
                while sent < M_PER_PRODUCER {
                    // Alternate single, 2-slot, 4-slot, single, ... so the consumer
                    // sees every fragment-kind combination.
                    let n_payload = match sent % 3 {
                        0 => 8,               // single-slot
                        1 => 2 * payload_cap, // 2-slot
                        _ => 4 * payload_cap, // 4-slot
                    };
                    let mut payload = vec![0u8; n_payload + 8];
                    payload[0..4].copy_from_slice(&(producer_id as u32).to_le_bytes());
                    payload[4..8].copy_from_slice(&sent.to_le_bytes());
                    for (i, b) in payload[8..].iter_mut().enumerate() {
                        *b = ((producer_id ^ sent as usize ^ i) & 0xFF) as u8;
                    }
                    match tx.try_send(&payload) {
                        Ok(_) => sent += 1,
                        Err(SendError::Full) => std::thread::yield_now(),
                        Err(e) => panic!("unexpected send error: {e:?}"),
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
                    assert!(buf.len() >= 8, "frame too short: {}", buf.len());
                    let producer_id = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
                    let sent_idx = i64::from(u32::from_le_bytes(buf[4..8].try_into().unwrap()));
                    assert!(producer_id < K_PRODUCERS, "bad producer id");
                    assert_eq!(
                        sent_idx,
                        last_seq[producer_id] + 1,
                        "out-of-order frame from producer {producer_id}"
                    );
                    // Verify payload bytes (catches fragment interleave / partial reads).
                    for (i, b) in buf[8..].iter().enumerate() {
                        let expected = ((producer_id ^ sent_idx as usize ^ i) & 0xFF) as u8;
                        assert_eq!(*b, expected, "payload mismatch at byte {i}");
                    }
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
