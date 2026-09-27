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

//! PostgreSQL shared-memory ring-buffer cache for read-time-indexed mutable segments.
//!
//! Mutable segments avoid indexing work on write by appending `ctid` records to an in-database log.
//! However, read queries accessing a mutable segment must materialize and index it into a Tantivy
//! segment.
//!
//! Without caching, every query independently materialized mutable segments from scratch into backend-private
//! memory (`RamDirectory`). During BM25 scoring (`query.weight(...)`), all parallel workers in a query
//! concurrently materialized the identical mutable segment, causing a synchronization stampede (issue #4497).
//! In read-heavy workloads, subsequent queries also suffered redundant re-indexing of identical segment
//! contents (issue #5356).
//!
//! This module provides a shared-memory ring-buffer cache that allows read-time indexed mutable segments
//! to be shared across queries and parallel workers.
//!
//! # Design
//!
//! ### Static Shared Memory vs. DSM
//!
//! PostgreSQL limits cluster-wide DSM segment handles to `64 + 5 * MaxBackends`. Under write churn,
//! allocating a dynamic DSM segment per indexing attempt risks exhausting this limit. Instead, the cache
//! operates in a single static shared-memory region requested at postmaster startup via `RequestAddinShmemSpace`
//! and sized by `paradedb.mutable_segment_cache_size` (default 64MB).
//!
//! ### Snapshot Isolation
//!
//! A mutable segment advances via an append-only log, whose prefix is deterministically identified
//! by the pair `(max_doc, num_deleted_docs)`, represented by [`MutableSegmentBound`].
//!
//! [`MutableCacheKey`] incorporates `database_oid`, `index_oid`, `segment_id`, and `bound`. Keying on
//! `bound` ensures that queries capturing a specific snapshot view continue hitting the cache even after
//! subsequent concurrent DML appends new rows to the mutable segment log.
//!
//! ### Concurrency Control
//!
//! Slot access is synchronized via a dedicated named `LWLock` tranche (`pg_search_mutable_cache`).
//! Exactly one worker claims [`SlotState::Building`] to index a given key, while concurrent backends and
//! parallel workers sleep on a [`ConditionVariable`]. If a builder process dies or aborts, waiting workers
//! detect that the builder PID is no longer alive ([`is_pid_alive`]), reset the slot to [`SlotState::Empty`],
//! and allow another worker to retry.
//!
//! ### Bip-Buffer Allocation & Eviction
//!
//! Tantivy requires component files to be contiguous in memory, so allocations cannot straddle the arena's
//! boundary. The allocator uses a bip-buffer strategy: allocations proceed linearly until the end of the
//! buffer, then wrap around to offset 0.
//!
//! Eviction advances `tail_offset` past completed entries in arena order, but is blocked if the entry at the tail has
//! `active_readers > 0`. When the slot table is full, eviction also advances `tail_offset` to reclaim the oldest
//! allocations in ring-buffer order, keeping allocations contiguous and freeing slots. If shared memory is exhausted
//! by active readers, workers gracefully fall back to local unshared indexing (`RamDirectory`) with a `pgrx::warning`.
//!
//! When a newer bound for a segment is cached, earlier bounds transition to [`SlotState::Superseded`]:
//! they are hidden from new lookups, but existing readers continue reading until their active reader count
//! reaches zero and `tail_offset` reclaims their arena span.
//!
//! ### Invalidation Lifecycle
//!
//! - Segment merges: When background merge workers consolidate mutable segments into immutable segments
//!   (`pg_search/src/postgres/merge.rs`), [`invalidate_segment`] marks matching slots as superseded.
//! - Relation lifecycle: `build_empty` (`pg_search/src/postgres/build.rs`) and `object_access_hook`
//!   (`OAT_DROP` on `pg_class`) call [`invalidate_index`] to evict all cached slots for dropped, reindexed,
//!   or truncated relations.

pub mod directory;
pub mod pack;
#[cfg(any(test, feature = "pg_test"))]
mod tests;

pub use directory::SharedMemoryDirectory;

use pgrx::pg_sys;
use std::cell::UnsafeCell;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicU64, Ordering};
use tantivy::Directory;
use tantivy::directory::RamDirectory;
use tantivy::index::SegmentId;

use crate::postgres::condition_variable::ConditionVariable;
use crate::postgres::locks::{LWLock, LWLockExclusiveGuard};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{MutableSegmentBound, SegmentMetaEntry};

/// Maximum number of tracked segment slots in the shared-memory header.
pub const MAX_SLOTS: usize = 256;

static MUTABLE_CACHE: AtomicPtr<MutableCacheHeader> = AtomicPtr::new(std::ptr::null_mut());
static mut PREV_SHMEM_REQUEST_HOOK: pg_sys::shmem_request_hook_type = None;
static mut PREV_SHMEM_STARTUP_HOOK: pg_sys::shmem_startup_hook_type = None;
static mut PREV_OBJECT_ACCESS_HOOK: pg_sys::object_access_hook_type = None;

/// Unique identifier for a materialized mutable segment bound in shared memory.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[repr(C)]
pub struct MutableCacheKey {
    /// Database OID for multi-database isolation.
    pub database_oid: pg_sys::Oid,
    /// Index relation OID.
    pub index_oid: pg_sys::Oid,
    /// 16-byte UUID of the mutable segment.
    pub segment_id: [u8; 16],
    /// Snapshot prefix bound (max_doc, num_deleted_docs).
    pub bound: MutableSegmentBound,
}

impl MutableCacheKey {
    /// Construct a cache key from database/index OIDs and a mutable segment's metadata.
    ///
    /// Returns `None` if `meta` is not a mutable segment.
    pub fn from_meta(
        database_oid: pg_sys::Oid,
        index_oid: pg_sys::Oid,
        meta: &SegmentMetaEntry,
    ) -> Option<Self> {
        let bound = meta.mutable_bound()?;
        Some(Self {
            database_oid,
            index_oid,
            segment_id: *meta.segment_id().uuid_bytes(),
            bound,
        })
    }

    /// Construct a cache key from an index relation and a mutable segment's metadata.
    ///
    /// Returns `None` if `meta` is not a mutable segment.
    pub fn for_segment(indexrel: &PgSearchRelation, meta: &SegmentMetaEntry) -> Option<Self> {
        Self::from_meta(unsafe { pg_sys::MyDatabaseId }, indexrel.oid(), meta)
    }

    /// Returns true if this key belongs to the same database, index, and segment UUID.
    pub fn matches_segment(&self, other: &Self) -> bool {
        self.database_oid == other.database_oid
            && self.index_oid == other.index_oid
            && self.segment_id == other.segment_id
    }
}

/// Byte range within the shared memory data arena.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
#[repr(C)]
pub struct ArenaSpan {
    /// Byte offset from the start of the data arena.
    pub offset: u32,
    /// Length of the allocated region in bytes.
    pub len: u32,
}

/// Lifecycle state of a cache slot.
#[repr(u32)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SlotState {
    /// Slot is unallocated and available for use.
    Empty = 0,
    /// A worker process is currently indexing and packing this segment in backend memory.
    /// Does not hold an arena allocation until indexing succeeds.
    Building = 1,
    /// Segment is fully packed and ready for concurrent reads.
    Ready = 2,
    /// Segment was invalidated or superseded by a newer bound. Active readers may continue;
    /// reclaimed in ring-buffer order when tail advances past its allocation.
    Superseded = 3,
}

/// Metadata entry tracking a cached segment in shared memory.
#[repr(C)]
pub struct CacheSlot {
    /// Key identifying the segment bound.
    pub key: MutableCacheKey,
    /// Location of the packed segment data in the arena.
    pub span: ArenaSpan,
    /// Count of active readers currently holding slices in this slot.
    pub active_readers: AtomicU32,
    /// PID of the process currently building this slot, if state is `Building`.
    pub builder_pid: pg_sys::pid_t,
    /// Current lifecycle state.
    pub state: SlotState,
}

impl CacheSlot {
    /// Construct an empty slot.
    pub const fn empty() -> Self {
        Self {
            key: MutableCacheKey {
                database_oid: pg_sys::Oid::INVALID,
                index_oid: pg_sys::Oid::INVALID,
                segment_id: [0u8; 16],
                bound: MutableSegmentBound {
                    max_doc: 0,
                    num_deleted_docs: 0,
                },
            },
            span: ArenaSpan { offset: 0, len: 0 },
            active_readers: AtomicU32::new(0),
            builder_pid: 0,
            state: SlotState::Empty,
        }
    }
}

/// RAII guard that decrements a slot's `active_readers` count on drop.
pub struct ActiveReaderGuard {
    slot_index: usize,
}

impl Drop for ActiveReaderGuard {
    fn drop(&mut self) {
        if let Some(cache) = load_cache() {
            let slots = unsafe { cache.slots() };
            if self.slot_index < slots.len() {
                slots[self.slot_index]
                    .active_readers
                    .fetch_sub(1, Ordering::Release);
            }
        }
    }
}

/// Ring-buffer head and tail allocation offsets.
#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct AllocatorState {
    /// Offset where the next segment will be allocated.
    pub head_offset: u32,
    /// Offset of the oldest entry eligible for eviction.
    pub tail_offset: u32,
}

/// Shared memory header located at the base of the cache allocation.
#[repr(C)]
pub struct MutableCacheHeader {
    /// LWLock protecting the slot table and allocator state.
    pub lock: LWLock,
    /// Condition variable to signal completion of in-flight builds.
    pub cv: ConditionVariable,
    /// Total usable capacity of the data arena in bytes.
    pub arena_capacity: u32,
    /// Invalidation generation counter.
    pub generation: AtomicU64,
    allocator: UnsafeCell<AllocatorState>,
    slots: UnsafeCell<[CacheSlot; MAX_SLOTS]>,
}

unsafe impl Send for MutableCacheHeader {}
unsafe impl Sync for MutableCacheHeader {}

impl MutableCacheHeader {
    pub unsafe fn init_raw(ptr: *mut Self, lock: LWLock, arena_capacity: u32) {
        std::ptr::addr_of_mut!((*ptr).lock).write(lock);
        std::ptr::addr_of_mut!((*ptr).arena_capacity).write(arena_capacity);
        (*ptr).cv.init();
        (*ptr).generation.store(0, Ordering::Relaxed);
        *(*ptr).allocator.get() = AllocatorState::default();
        for slot in &mut *(*ptr).slots.get() {
            *slot = CacheSlot::empty();
        }
    }

    pub unsafe fn slots(&self) -> &[CacheSlot] {
        &*self.slots.get()
    }

    #[allow(clippy::mut_from_ref)]
    pub unsafe fn slots_mut(&self, _guard: &LWLockExclusiveGuard<'_>) -> &mut [CacheSlot] {
        &mut *self.slots.get()
    }

    #[allow(clippy::mut_from_ref)]
    pub unsafe fn allocator_mut(&self, _guard: &LWLockExclusiveGuard<'_>) -> &mut AllocatorState {
        &mut *self.allocator.get()
    }

    pub fn arena_slice(&self) -> &[u8] {
        let ptr = unsafe { (self as *const Self as *const u8).add(std::mem::size_of::<Self>()) };
        unsafe { std::slice::from_raw_parts(ptr, self.arena_capacity as usize) }
    }

    #[allow(clippy::mut_from_ref)]
    pub unsafe fn arena_slice_mut(&self, _guard: &LWLockExclusiveGuard<'_>) -> &mut [u8] {
        let ptr = (self as *const Self as *mut u8).add(std::mem::size_of::<Self>());
        std::slice::from_raw_parts_mut(ptr, self.arena_capacity as usize)
    }

    /// Try to allocate `needed_bytes` in the ring buffer.
    /// Returns `Some(ArenaSpan)` on success, or `None` if the arena is full of in-use data.
    pub fn allocate(
        &self,
        needed_bytes: usize,
        guard: &LWLockExclusiveGuard<'_>,
    ) -> Option<ArenaSpan> {
        let slots = unsafe { self.slots_mut(guard) };
        let allocator = unsafe { self.allocator_mut(guard) };
        self.allocate_raw(needed_bytes, slots, allocator)
    }

    /// Allocate `needed_bytes` (aligned to 8 bytes) in the ring buffer arena.
    ///
    /// Implements a bip-buffer allocation strategy with active-reader eviction protection:
    /// - If enough linear room exists between `head_offset` and the arena capacity, allocates linearly.
    /// - If linear space is exhausted, wraps around to offset 0, evicting slots at `tail_offset`
    ///   whose `active_readers == 0`.
    /// - If eviction is blocked because slots at `tail_offset` are actively being read by queries,
    ///   returns `None`.
    pub fn allocate_raw(
        &self,
        needed_bytes: usize,
        slots: &mut [CacheSlot],
        allocator: &mut AllocatorState,
    ) -> Option<ArenaSpan> {
        let needed = (needed_bytes + 7) & !7;
        let capacity = self.arena_capacity as usize;
        if needed > capacity {
            return None;
        }

        let is_empty = slots.iter().all(|s| s.state == SlotState::Empty);
        if is_empty {
            allocator.head_offset = 0;
            allocator.tail_offset = 0;
        }

        let head = allocator.head_offset as usize;
        let mut tail = allocator.tail_offset as usize;

        if head >= tail && !is_empty {
            // Linear region ahead of head
            if head + needed <= capacity {
                let offset = head as u32;
                allocator.head_offset = (head + needed) as u32;
                return Some(ArenaSpan {
                    offset,
                    len: needed as u32,
                });
            }

            // Not enough room before end of buffer: attempt to wrap around to 0
            while tail <= needed && !slots.iter().all(|s| s.state == SlotState::Empty) {
                if !Self::try_advance_tail(slots, &mut tail, &self.generation) {
                    return None;
                }
            }

            if tail > needed || slots.iter().all(|s| s.state == SlotState::Empty) {
                allocator.head_offset = needed as u32;
                allocator.tail_offset = tail as u32;
                return Some(ArenaSpan {
                    offset: 0,
                    len: needed as u32,
                });
            }

            return None;
        }

        // Wrapped region: head < tail
        while head + needed > tail {
            if !Self::try_advance_tail(slots, &mut tail, &self.generation) {
                return None;
            }
            if tail >= capacity {
                tail = 0;
            }
        }

        let offset = head as u32;
        allocator.head_offset = (head + needed) as u32;
        allocator.tail_offset = tail as u32;
        Some(ArenaSpan {
            offset,
            len: needed as u32,
        })
    }

    /// Try to advance `tail` past the oldest allocated entry in the arena.
    ///
    /// Matches only [`SlotState::Ready`] or [`SlotState::Superseded`] slots whose allocation starts
    /// at `*tail` (ignoring [`SlotState::Building`] slots which do not hold arena spans yet).
    /// Returns `false` if eviction is blocked because the entry at the tail has `active_readers > 0`.
    /// On success, advances `*tail`, resets the evicted slot to [`SlotState::Empty`], increments `generation`,
    /// and returns `true`. If no active slot starts at `*tail`, wraps `*tail = 0`.
    fn try_advance_tail(slots: &mut [CacheSlot], tail: &mut usize, generation: &AtomicU64) -> bool {
        let current_tail = *tail as u32;
        let mut found_idx = None;

        for (idx, slot) in slots.iter().enumerate() {
            if matches!(slot.state, SlotState::Ready | SlotState::Superseded)
                && slot.span.offset == current_tail
            {
                found_idx = Some(idx);
                break;
            }
        }

        if let Some(idx) = found_idx {
            let slot = &mut slots[idx];
            if slot.active_readers.load(Ordering::Acquire) > 0 {
                // In active use by a running query: cannot evict
                return false;
            }

            *tail = (slot.span.offset + slot.span.len) as usize;
            *slot = CacheSlot::empty();
            generation.fetch_add(1, Ordering::Release);
            true
        } else {
            // Gap / wrap sentinel: advance tail to 0
            *tail = 0;
            true
        }
    }

    /// Transition all cache slots for `segment_id` to [`SlotState::Superseded`].
    ///
    /// Called when a mutable segment is merged into an immutable segment. Existing readers
    /// may complete their reads; new lookups will bypass these slots.
    pub fn invalidate_segment(
        &self,
        database_oid: pg_sys::Oid,
        index_oid: pg_sys::Oid,
        segment_id: &[u8; 16],
    ) {
        let guard = self.lock.acquire_exclusive();
        let slots = unsafe { self.slots_mut(&guard) };
        for slot in slots {
            if slot.state != SlotState::Empty
                && slot.key.database_oid == database_oid
                && slot.key.index_oid == index_oid
                && slot.key.segment_id == *segment_id
            {
                slot.state = SlotState::Superseded;
            }
        }
        self.generation.fetch_add(1, Ordering::Release);
    }

    /// Transition all cache slots for `index_oid` to [`SlotState::Superseded`].
    ///
    /// Called on `CREATE INDEX`, `REINDEX`, `TRUNCATE`, or table/index drop.
    pub fn invalidate_index(&self, database_oid: pg_sys::Oid, index_oid: pg_sys::Oid) {
        let guard = self.lock.acquire_exclusive();
        let slots = unsafe { self.slots_mut(&guard) };
        for slot in slots {
            if slot.state != SlotState::Empty
                && slot.key.database_oid == database_oid
                && slot.key.index_oid == index_oid
            {
                slot.state = SlotState::Superseded;
            }
        }
        self.generation.fetch_add(1, Ordering::Release);
    }
}

/// Load the global shared-memory cache header, or `None` if shared memory is not initialized.
pub fn load_cache() -> Option<&'static MutableCacheHeader> {
    let ptr = MUTABLE_CACHE.load(Ordering::Acquire);
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { &*ptr })
}

unsafe fn is_pid_alive(pid: pg_sys::pid_t) -> bool {
    unsafe extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    if pid <= 0 {
        return false;
    }
    unsafe { kill(pid, 0) == 0 }
}

struct InflightBuildGuard {
    slot_index: usize,
    completed: bool,
}

impl Drop for InflightBuildGuard {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        let Some(cache) = load_cache() else {
            return;
        };
        let guard = cache.lock.acquire_exclusive();
        let slots = unsafe { cache.slots_mut(&guard) };
        if self.slot_index < slots.len() {
            let slot = &mut slots[self.slot_index];
            if slot.state == SlotState::Building {
                *slot = CacheSlot::empty();
                cache.cv.broadcast();
            }
        }
    }
}

/// Get a cached directory for `key`, or build it using `build_fn`.
///
/// Coordinates with concurrent backends so that only one worker builds the segment while
/// others block on `ConditionVariable`.
pub fn get_or_build(
    key: &MutableCacheKey,
    build_fn: impl FnOnce() -> anyhow::Result<RamDirectory>,
) -> anyhow::Result<Arc<dyn Directory>> {
    if !crate::gucs::enable_mutable_segment_cache() {
        let ram_dir = build_fn()?;
        return Ok(Arc::new(ram_dir));
    }
    let Some(cache) = load_cache() else {
        let ram_dir = build_fn()?;
        return Ok(Arc::new(ram_dir));
    };

    loop {
        let guard = cache.lock.acquire_exclusive();
        let slots = unsafe { cache.slots_mut(&guard) };

        // 1. Check for existing Ready entry
        for (idx, slot) in slots.iter().enumerate() {
            if slot.state == SlotState::Ready && slot.key == *key {
                slot.active_readers.fetch_add(1, Ordering::AcqRel);
                let reader_guard = Arc::new(ActiveReaderGuard { slot_index: idx });
                let slice_ptr =
                    unsafe { (cache.arena_slice().as_ptr()).add(slot.span.offset as usize) };
                let slice_len = slot.span.len as usize;
                let slice = unsafe { std::slice::from_raw_parts(slice_ptr, slice_len) };
                if let Some(toc) = pack::unpack_toc(slice) {
                    drop(guard);
                    return Ok(Arc::new(SharedMemoryDirectory::new(
                        slice_ptr,
                        slice_len,
                        reader_guard,
                        toc,
                    )));
                }
            }
        }

        // 2. Check if another worker is building
        let mut in_flight_building = false;
        for slot in slots.iter_mut() {
            if slot.key == *key && slot.state == SlotState::Building {
                if unsafe { is_pid_alive(slot.builder_pid) } {
                    in_flight_building = true;
                    break;
                } else {
                    // Builder process crashed: reset slot and allow retry
                    *slot = CacheSlot::empty();
                    cache.cv.broadcast();
                    break;
                }
            }
        }

        if in_flight_building {
            cache.cv.prepare_to_sleep();
            drop(guard);
            cache.cv.sleep();
            pgrx::check_for_interrupts!();
            continue;
        }

        // 3. Find an empty slot to claim build
        let mut claim_idx = None;
        for (idx, slot) in slots.iter().enumerate() {
            if slot.state == SlotState::Empty {
                claim_idx = Some(idx);
                break;
            }
        }

        if claim_idx.is_none() {
            // Slot table full: evict oldest entries from the tail to free a slot
            let allocator = unsafe { cache.allocator_mut(&guard) };
            let mut tail = allocator.tail_offset as usize;
            while slots.iter().all(|s| s.state != SlotState::Empty) {
                if !MutableCacheHeader::try_advance_tail(slots, &mut tail, &cache.generation) {
                    break;
                }
                allocator.tail_offset = tail as u32;
            }
            for (idx, slot) in slots.iter().enumerate() {
                if slot.state == SlotState::Empty {
                    claim_idx = Some(idx);
                    break;
                }
            }
        }

        let Some(slot_idx) = claim_idx else {
            drop(guard);
            pgrx::warning!(
                "pg_search: mutable segment cache slot table full, falling back to local indexing"
            );
            let ram_dir = build_fn()?;
            return Ok(Arc::new(ram_dir));
        };

        // Claim slot as Building
        let my_pid = unsafe { pg_sys::MyProcPid };
        slots[slot_idx].key = *key;
        slots[slot_idx].builder_pid = my_pid;
        slots[slot_idx].state = SlotState::Building;
        slots[slot_idx].active_readers.store(0, Ordering::Release);

        drop(guard);

        // We are now the designated builder
        let mut build_guard = InflightBuildGuard {
            slot_index: slot_idx,
            completed: false,
        };

        let ram_dir = build_fn()?;

        let segment_id = SegmentId::from_bytes(key.segment_id);
        let packed_bytes = match pack::pack_segment(&ram_dir, &segment_id) {
            Ok(bytes) => bytes,
            Err(e) => {
                pgrx::warning!("pg_search: failed to pack mutable segment: {e}");
                build_guard.completed = true;
                let guard = cache.lock.acquire_exclusive();
                let slots = unsafe { cache.slots_mut(&guard) };
                slots[slot_idx] = CacheSlot::empty();
                cache.cv.broadcast();
                drop(guard);
                return Ok(Arc::new(ram_dir));
            }
        };

        let guard = cache.lock.acquire_exclusive();
        let span = match cache.allocate(packed_bytes.len(), &guard) {
            Some(span) => span,
            None => {
                pgrx::warning!(
                    "pg_search: mutable segment cache arena full, falling back to local indexing"
                );
                build_guard.completed = true;
                let slots = unsafe { cache.slots_mut(&guard) };
                slots[slot_idx] = CacheSlot::empty();
                cache.cv.broadcast();
                drop(guard);
                return Ok(Arc::new(ram_dir));
            }
        };

        // Copy packed bytes into arena
        let arena_start = unsafe { cache.arena_slice_mut(&guard).as_mut_ptr() };
        unsafe {
            std::ptr::copy_nonoverlapping(
                packed_bytes.as_ptr(),
                arena_start.add(span.offset as usize),
                packed_bytes.len(),
            );
        }

        // Mark slot Ready
        let slots = unsafe { cache.slots_mut(&guard) };
        slots[slot_idx].span = span;
        slots[slot_idx].state = SlotState::Ready;
        slots[slot_idx].active_readers.store(1, Ordering::Release);

        // Supersede any older bound for this same segment
        for (i, other_slot) in slots.iter_mut().enumerate() {
            if i != slot_idx
                && other_slot.state == SlotState::Ready
                && other_slot.key.matches_segment(key)
            {
                other_slot.state = SlotState::Superseded;
            }
        }

        cache.cv.broadcast();
        build_guard.completed = true;

        let reader_guard = Arc::new(ActiveReaderGuard {
            slot_index: slot_idx,
        });
        let slice_ptr = unsafe { arena_start.add(span.offset as usize) as *const u8 };
        let slice_len = span.len as usize;
        let slice = unsafe { std::slice::from_raw_parts(slice_ptr, slice_len) };
        let toc = pack::unpack_toc(slice).expect("packed segment must unpack valid TOC");

        drop(guard);

        return Ok(Arc::new(SharedMemoryDirectory::new(
            slice_ptr,
            slice_len,
            reader_guard,
            toc,
        )));
    }
}

/// Invalidate all cache slots for `segment_id` across all bounds.
///
/// Called from [`crate::postgres::merge`] when a mutable segment is merged into an immutable segment.
pub fn invalidate_segment(
    database_oid: pg_sys::Oid,
    index_oid: pg_sys::Oid,
    segment_id: &[u8; 16],
) {
    if let Some(cache) = load_cache() {
        cache.invalidate_segment(database_oid, index_oid, segment_id);
    }
}

/// Invalidate all cache slots for `index_oid`.
///
/// Called on `CREATE INDEX`, `REINDEX`, `TRUNCATE` (via `build_empty`), or table/index drop (via `object_access_hook`).
pub fn invalidate_index(database_oid: pg_sys::Oid, index_oid: pg_sys::Oid) {
    if let Some(cache) = load_cache() {
        cache.invalidate_index(database_oid, index_oid);
    }
}

fn total_shmem_size() -> usize {
    std::mem::size_of::<MutableCacheHeader>() + crate::gucs::mutable_segment_cache_size()
}

/// Initialize hooks for PostgreSQL shared memory.
///
/// Must be called from `_PG_init` during `process_shared_preload_libraries_in_progress`.
pub unsafe fn init() {
    if !pg_sys::process_shared_preload_libraries_in_progress {
        return;
    }

    PREV_SHMEM_REQUEST_HOOK = pg_sys::shmem_request_hook;
    pg_sys::shmem_request_hook = Some(shmem_request);

    PREV_SHMEM_STARTUP_HOOK = pg_sys::shmem_startup_hook;
    pg_sys::shmem_startup_hook = Some(shmem_startup);

    PREV_OBJECT_ACCESS_HOOK = pg_sys::object_access_hook;
    pg_sys::object_access_hook = Some(object_access_hook);
}

unsafe extern "C-unwind" fn shmem_request() {
    if let Some(prev) = PREV_SHMEM_REQUEST_HOOK {
        prev();
    }

    pg_sys::RequestNamedLWLockTranche(c"pg_search_mutable_cache".as_ptr(), 1);
    pg_sys::RequestAddinShmemSpace(total_shmem_size());
}

unsafe extern "C-unwind" fn shmem_startup() {
    if let Some(prev) = PREV_SHMEM_STARTUP_HOOK {
        prev();
    }

    let tranche_ptr = pg_sys::GetNamedLWLockTranche(c"pg_search_mutable_cache".as_ptr());
    let lock = LWLock::from_raw(tranche_ptr as *mut pg_sys::LWLock);

    let arena_size = crate::gucs::mutable_segment_cache_size();
    let size = total_shmem_size();
    let mut found = false;
    let header_ptr = pg_sys::ShmemInitStruct(c"pg_search_mutable_cache".as_ptr(), size, &mut found)
        as *mut MutableCacheHeader;

    if !found {
        std::ptr::write_bytes(header_ptr as *mut u8, 0, size);
        MutableCacheHeader::init_raw(header_ptr, lock, arena_size as u32);
    }

    MUTABLE_CACHE.store(header_ptr, Ordering::Release);
}

unsafe extern "C-unwind" fn object_access_hook(
    access: pg_sys::ObjectAccessType::Type,
    class_id: pg_sys::Oid,
    object_id: pg_sys::Oid,
    sub_id: i32,
    arg: *mut c_void,
) {
    if let Some(prev) = PREV_OBJECT_ACCESS_HOOK {
        prev(access, class_id, object_id, sub_id, arg);
    }

    if access == pg_sys::ObjectAccessType::OAT_DROP && class_id == pg_sys::RelationRelationId {
        invalidate_index(pg_sys::MyDatabaseId, object_id);
    }
}
