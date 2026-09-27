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

//! PostgreSQL shared-memory cache for read-time-indexed mutable segments.
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
//! This module provides a shared-memory cache that allows read-time indexed mutable segments to be shared
//! across queries and parallel workers.
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
//! Concurrent segment builds coordinate via a dedicated in-flight build table and [`ConditionVariable`].
//! Exactly one worker claims an in-flight build entry to index a given key in private backend memory,
//! while concurrent backends and parallel workers sleep on a [`ConditionVariable`]. If a builder process
//! dies or aborts, waiting workers detect that the builder PID is no longer alive ([`is_pid_alive`]),
//! reset the in-flight entry, and allow another worker to retry.
//!
//! ### Implicit Binary Tree Slab Allocation
//!
//! Tantivy requires component files to be contiguous in memory, so allocations cannot straddle non-contiguous
//! chunks. Memory in the shared data arena is managed by [`SlabPool`], an implicit binary tree slab allocator:
//!
//! - Slabs are power-of-two multiples of a base slab size (typically 256KB), scaling up to the entire cache
//!   capacity (e.g. 64MB or 1GB).
//! - The tree is maintained entirely in a flat array in shared memory without pointers or linked lists.
//! - Slabs automatically coalesce with adjacent buddies in $O(\log N)$ steps upon deallocation.
//!
//! ### Eager Reclamation & Table Isolation (Eliminating the Noisy Neighbor)
//!
//! In a global FIFO ring buffer, high write churn on one relation continually advances a shared tail,
//! evicting hot segments belonging to unrelated read-heavy relations.
//!
//! The slab pool eliminates this noisy-neighbor problem via eager out-of-order reclamation:
//! - When a newer bound for a segment is cached, earlier bounds transition to [`SlotState::Superseded`].
//! - Any superseded slot whose active reader count has dropped to zero is immediately freed back to the
//!   [`SlabPool`], recycling the memory block for the next write commit.
//! - When in-flight queries reading a superseded bound finish, [`ActiveReaderGuard::drop`] immediately
//!   returns the slab to the pool.
//! - High write churn on one table continuously recycles its own slabs and never touches or evicts
//!   unrelated tables.
//!
//! ### Clock Sweep Cache Eviction
//!
//! When all slabs of a requested order are in use by active segments, the allocator performs a Clock sweep
//! with a saturated `usage_count` (0..=5):
//! - Every cache hit on a [`SlotState::Ready`] segment increments its `usage_count` up to 5.
//! - During memory pressure, a shared clock hand scans the slot table:
//!   - Slots with `usage_count > 0` have their count decremented and are skipped, protecting frequently
//!     queried tables.
//!   - Slots with `usage_count == 0` and zero active readers are evicted, freeing their slab.
//! - If all memory is pinned by active readers, workers gracefully fall back to local unshared indexing
//!   (`RamDirectory`) with a `pgrx::warning`.
//!
//! ### Invalidation Lifecycle
//!
//! - Segment merges: When background merge workers consolidate mutable segments into immutable segments
//!   (`pg_search/src/postgres/merge.rs`), [`invalidate_segment`] eagerly frees matching slots if they have
//!   zero active readers, or marks them as superseded.
//! - Relation lifecycle: `build_empty` (`pg_search/src/postgres/build.rs`) and `object_access_hook`
//!   (`OAT_DROP` on `pg_class`) call [`invalidate_index`] to evict all cached slots for dropped, reindexed,
//!   or truncated relations.

pub mod directory;
pub mod pack;
pub mod slab_pool;
#[cfg(any(test, feature = "pg_test"))]
mod tests;

pub use directory::SharedMemoryDirectory;
pub use slab_pool::{ArenaSpan, SlabPool};

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

/// Maximum number of concurrent in-flight segment builds tracked in shared memory.
pub const MAX_INFLIGHT_BUILDS: usize = 16;

static MUTABLE_CACHE: AtomicPtr<MutableCacheHeader> = AtomicPtr::new(std::ptr::null_mut());
static mut PREV_SHMEM_REQUEST_HOOK: pg_sys::shmem_request_hook_type = None;
static mut PREV_SHMEM_STARTUP_HOOK: pg_sys::shmem_startup_hook_type = None;
static mut PREV_OBJECT_ACCESS_HOOK: pg_sys::object_access_hook_type = None;

/// Unique identifier for a materialized mutable segment bound in shared memory.
///
/// Keys are matched on `database_oid`, `index_oid`, and `segment_id` (a 16-byte Tantivy UUID),
/// paired with a specific [`MutableSegmentBound`] `(max_doc, num_deleted_docs)`.
///
/// While exact equality `==` checks for an identical snapshot prefix, [`Self::matches_segment`]
/// checks if two keys represent the same underlying mutable segment across different bounds.
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

/// Entry tracking an in-flight segment build across concurrent backends.
///
/// When multiple concurrent queries or parallel workers require the same mutable segment, exactly
/// one backend claims an in-flight build slot, indexes the segment in private backend memory, and
/// copies it into shared memory. Concurrent workers wait on a shared [`ConditionVariable`].
///
/// If a builder backend aborts, crashes, or is killed before completing, waiting workers detect that
/// [`is_pid_alive`] is false, reset the entry, and allow another worker to retry.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct InflightBuild {
    /// Segment key currently being built in private backend memory.
    pub key: MutableCacheKey,
    /// PID of the backend process currently building the segment, or 0 if inactive.
    pub builder_pid: pg_sys::pid_t,
}

impl InflightBuild {
    /// Construct an empty in-flight build entry.
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
            builder_pid: 0,
        }
    }

    /// Returns true if this entry is inactive.
    pub fn is_empty(&self) -> bool {
        self.builder_pid == 0
    }
}

/// Lifecycle state of a cache slot.
///
/// # State Transitions
///
/// - `Empty` -> `Ready`: When a builder backend successfully indexes a segment and installs it into
///   an allocated slab.
/// - `Ready` -> `Superseded`: When a newer bound for the same segment is cached, or when the segment
///   or relation is invalidated, while active readers are still accessing this slot.
/// - `Ready` -> `Empty`: When invalidated or evicted by a clock sweep with zero active readers.
/// - `Superseded` -> `Empty`: When the last active reader drops its [`ActiveReaderGuard`], or when a
///   clock sweep sweeps past with zero active readers, immediately returning the slab to the [`SlabPool`].
#[repr(u32)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SlotState {
    /// Slot is unallocated and available for use.
    Empty = 0,
    /// Segment is fully packed and ready for concurrent reads.
    Ready = 1,
    /// Segment was invalidated or superseded by a newer bound. Active readers may continue;
    /// reclaimed eagerly when active readers count reaches zero.
    Superseded = 2,
}

/// Metadata entry tracking a cached segment in shared memory.
#[repr(C)]
pub struct CacheSlot {
    /// Key identifying the database, index, segment UUID, and snapshot bound.
    pub key: MutableCacheKey,
    /// Location and power-of-two order of the packed segment data in the arena.
    pub span: ArenaSpan,
    /// Count of active readers currently holding slices in this slot.
    pub active_readers: AtomicU32,
    /// Saturated usage count for clock-sweep cache eviction (0..=5). Incremented on read cache hits.
    pub usage_count: AtomicU32,
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
            span: ArenaSpan {
                offset: 0,
                len: 0,
                order: 0,
            },
            active_readers: AtomicU32::new(0),
            usage_count: AtomicU32::new(0),
            state: SlotState::Empty,
        }
    }
}

/// RAII guard that decrements a slot's `active_readers` count on drop.
///
/// Holds an active reference to a cached segment slot. When dropped:
/// - Decrements the slot's `active_readers` refcount atomically.
/// - If the slot is in [`SlotState::Superseded`] and this drop transitions `active_readers` to zero,
///   triggers eager out-of-order deallocation via [`MutableCacheHeader::reclaim_superseded_slot`],
///   immediately returning the memory slab to the [`SlabPool`].
pub struct ActiveReaderGuard {
    slot_index: usize,
}

impl Drop for ActiveReaderGuard {
    fn drop(&mut self) {
        if let Some(cache) = load_cache() {
            let slots = unsafe { cache.slots() };
            if self.slot_index < slots.len() {
                let prev = slots[self.slot_index]
                    .active_readers
                    .fetch_sub(1, Ordering::Release);
                if prev == 1 && slots[self.slot_index].state == SlotState::Superseded {
                    cache.reclaim_superseded_slot(self.slot_index);
                }
            }
        }
    }
}

/// Shared memory header located at the base of the cache allocation.
///
/// Sits at byte offset 0 of the shared-memory region requested via `RequestAddinShmemSpace`.
/// Directly following this header struct is the contiguous data arena of size `arena_capacity`.
///
/// Contains:
/// - A named `LWLock` protecting the slot table and the [`SlabPool`] allocator.
/// - A [`ConditionVariable`] to wake workers waiting on in-flight builds.
/// - The [`SlabPool`] implicit binary tree buddy allocator.
/// - Fixed-size arrays for `slots` ([`MAX_SLOTS`]) and `inflight_builds` ([`MAX_INFLIGHT_BUILDS`]).
/// - An atomic `clock_hand` for clock-sweep cache eviction.
/// - An atomic `generation` counter incremented on slot invalidation or deallocation.
#[repr(C)]
pub struct MutableCacheHeader {
    /// LWLock protecting the slot table and slab pool.
    pub lock: LWLock,
    /// Condition variable to signal completion of in-flight builds.
    pub cv: ConditionVariable,
    /// Total usable capacity of the data arena in bytes.
    pub arena_capacity: u32,
    /// Invalidation generation counter.
    pub generation: AtomicU64,
    /// Clock hand for eviction sweep across slots.
    pub clock_hand: AtomicU32,
    slab_pool: UnsafeCell<SlabPool>,
    inflight_builds: UnsafeCell<[InflightBuild; MAX_INFLIGHT_BUILDS]>,
    slots: UnsafeCell<[CacheSlot; MAX_SLOTS]>,
}

unsafe impl Send for MutableCacheHeader {}
unsafe impl Sync for MutableCacheHeader {}

impl MutableCacheHeader {
    /// Initialize the shared-memory header in-place.
    ///
    /// # Safety
    ///
    /// Must be called once during PostgreSQL shared-memory startup while holding exclusive access
    /// to the newly zeroed shared-memory chunk.
    pub unsafe fn init_raw(ptr: *mut Self, lock: LWLock, arena_capacity: u32) {
        std::ptr::addr_of_mut!((*ptr).lock).write(lock);
        std::ptr::addr_of_mut!((*ptr).arena_capacity).write(arena_capacity);
        (*ptr).cv.init();
        (*ptr).generation.store(0, Ordering::Relaxed);
        (*ptr).clock_hand.store(0, Ordering::Relaxed);
        (*(*ptr).slab_pool.get()).init(arena_capacity as usize);
        for build in &mut *(*ptr).inflight_builds.get() {
            *build = InflightBuild::empty();
        }
        for slot in &mut *(*ptr).slots.get() {
            *slot = CacheSlot::empty();
        }
    }

    #[allow(dead_code)]
    pub unsafe fn inflight_builds(&self) -> &[InflightBuild] {
        &*self.inflight_builds.get()
    }

    #[allow(clippy::mut_from_ref)]
    pub unsafe fn inflight_builds_mut(
        &self,
        _guard: &LWLockExclusiveGuard<'_>,
    ) -> &mut [InflightBuild] {
        &mut *self.inflight_builds.get()
    }

    pub unsafe fn slots(&self) -> &[CacheSlot] {
        &*self.slots.get()
    }

    #[allow(clippy::mut_from_ref)]
    pub unsafe fn slots_mut(&self, _guard: &LWLockExclusiveGuard<'_>) -> &mut [CacheSlot] {
        &mut *self.slots.get()
    }

    #[allow(clippy::mut_from_ref)]
    pub unsafe fn slab_pool_mut(&self, _guard: &LWLockExclusiveGuard<'_>) -> &mut SlabPool {
        &mut *self.slab_pool.get()
    }

    /// Returns the contiguous shared-memory data arena as an immutable byte slice.
    ///
    /// Slices from offset `std::mem::size_of::<Self>()` with length `arena_capacity`.
    pub fn arena_slice(&self) -> &[u8] {
        let ptr = unsafe { (self as *const Self as *const u8).add(std::mem::size_of::<Self>()) };
        unsafe { std::slice::from_raw_parts(ptr, self.arena_capacity as usize) }
    }

    /// Returns the contiguous shared-memory data arena as a mutable byte slice.
    ///
    /// The caller must hold an exclusive guard on `self.lock`.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn arena_slice_mut(&self, _guard: &LWLockExclusiveGuard<'_>) -> &mut [u8] {
        let ptr = (self as *const Self as *mut u8).add(std::mem::size_of::<Self>());
        std::slice::from_raw_parts_mut(ptr, self.arena_capacity as usize)
    }

    /// Try to allocate `needed_bytes` in the slab pool.
    ///
    /// First attempts a fast-path buddy allocation via [`SlabPool::allocate`].
    /// If no free slab of sufficient order is available, initiates a clock sweep
    /// ([`Self::evict_with_clock_sweep`]) over the slot table to evict superseded or cold entries,
    /// coalescing buddy slabs until a suitable slab is freed.
    pub fn allocate(
        &self,
        needed_bytes: usize,
        guard: &LWLockExclusiveGuard<'_>,
    ) -> Option<ArenaSpan> {
        let slab_pool = unsafe { self.slab_pool_mut(guard) };
        if let Some(span) = slab_pool.allocate(needed_bytes) {
            return Some(span);
        }

        let slots = unsafe { self.slots_mut(guard) };
        let slab_pool = unsafe { self.slab_pool_mut(guard) };
        self.evict_with_clock_sweep(needed_bytes, slots, slab_pool)
    }

    /// Evict cached slots using a clock sweep with `usage_count` until a slab for `needed_bytes`
    /// can be allocated.
    ///
    /// Advances `clock_hand` through the slot table up to `MAX_SLOTS * 6` times:
    /// - [`SlotState::Superseded`] slots with zero active readers are immediately freed back to the slab pool.
    /// - [`SlotState::Ready`] slots with active readers (`active_readers > 0`) are skipped to protect in-flight queries.
    /// - [`SlotState::Ready`] slots with zero active readers:
    ///   - If `usage_count > 0`, decrements `usage_count` by 1 (second chance) and skips.
    ///   - If `usage_count == 0`, evicts the slot, frees its slab to the pool, and resets the slot to [`SlotState::Empty`].
    /// - After every slab freed, retries [`SlabPool::allocate`]. Returns `Some(span)` as soon as an allocation succeeds.
    fn evict_with_clock_sweep(
        &self,
        needed_bytes: usize,
        slots: &mut [CacheSlot],
        slab_pool: &mut SlabPool,
    ) -> Option<ArenaSpan> {
        let max_steps = MAX_SLOTS * 6;
        for _ in 0..max_steps {
            let idx = (self.clock_hand.fetch_add(1, Ordering::Relaxed) as usize) % MAX_SLOTS;
            let slot = &mut slots[idx];

            match slot.state {
                SlotState::Empty => continue,
                SlotState::Superseded => {
                    if slot.active_readers.load(Ordering::Acquire) == 0 {
                        slab_pool.free(slot.span.offset, slot.span.order);
                        *slot = CacheSlot::empty();
                        self.generation.fetch_add(1, Ordering::Release);
                        if let Some(span) = slab_pool.allocate(needed_bytes) {
                            return Some(span);
                        }
                    }
                }
                SlotState::Ready => {
                    if slot.active_readers.load(Ordering::Acquire) > 0 {
                        continue;
                    }
                    let usage = slot.usage_count.load(Ordering::Relaxed);
                    if usage > 0 {
                        slot.usage_count.store(usage - 1, Ordering::Relaxed);
                    } else {
                        slab_pool.free(slot.span.offset, slot.span.order);
                        *slot = CacheSlot::empty();
                        self.generation.fetch_add(1, Ordering::Release);
                        if let Some(span) = slab_pool.allocate(needed_bytes) {
                            return Some(span);
                        }
                    }
                }
            }
        }
        None
    }

    /// Evict a slot using clock sweep when all [`MAX_SLOTS`] entries in the slot table are occupied.
    ///
    /// Returns the index of a freed or available slot, or `None` if all slots are pinned by active readers.
    pub fn evict_slot_for_claim(
        &self,
        slots: &mut [CacheSlot],
        slab_pool: &mut SlabPool,
    ) -> Option<usize> {
        let max_steps = MAX_SLOTS * 6;
        for _ in 0..max_steps {
            let idx = (self.clock_hand.fetch_add(1, Ordering::Relaxed) as usize) % MAX_SLOTS;
            let slot = &mut slots[idx];

            match slot.state {
                SlotState::Empty => return Some(idx),
                SlotState::Superseded => {
                    if slot.active_readers.load(Ordering::Acquire) == 0 {
                        slab_pool.free(slot.span.offset, slot.span.order);
                        *slot = CacheSlot::empty();
                        self.generation.fetch_add(1, Ordering::Release);
                        return Some(idx);
                    }
                }
                SlotState::Ready => {
                    if slot.active_readers.load(Ordering::Acquire) > 0 {
                        continue;
                    }
                    let usage = slot.usage_count.load(Ordering::Relaxed);
                    if usage > 0 {
                        slot.usage_count.store(usage - 1, Ordering::Relaxed);
                    } else {
                        slab_pool.free(slot.span.offset, slot.span.order);
                        *slot = CacheSlot::empty();
                        self.generation.fetch_add(1, Ordering::Release);
                        return Some(idx);
                    }
                }
            }
        }
        None
    }

    /// Reclaim a superseded slot whose active readers have drained to zero.
    ///
    /// Called eagerly from [`ActiveReaderGuard::drop`] when the final reader exits, ensuring write churn
    /// immediately recycles older bounds without waiting for cache eviction.
    pub fn reclaim_superseded_slot(&self, slot_index: usize) {
        let guard = self.lock.acquire_exclusive();
        let slots = unsafe { self.slots_mut(&guard) };
        if slot_index < slots.len() {
            let slot = &mut slots[slot_index];
            if slot.state == SlotState::Superseded
                && slot.active_readers.load(Ordering::Acquire) == 0
            {
                let span = slot.span;
                *slot = CacheSlot::empty();
                let slab_pool = unsafe { self.slab_pool_mut(&guard) };
                slab_pool.free(span.offset, span.order);
                self.generation.fetch_add(1, Ordering::Release);
            }
        }
    }

    /// Transition all cache slots for `segment_id` across all bounds to [`SlotState::Superseded`].
    ///
    /// Called when a mutable segment is merged into an immutable segment (`crate::postgres::merge`).
    /// If a matching slot has zero active readers, its slab is eagerly freed back to the [`SlabPool`].
    /// Any in-flight builds for this segment are cancelled, and waiting workers are awakened via `cv.broadcast()`.
    pub fn invalidate_segment(
        &self,
        database_oid: pg_sys::Oid,
        index_oid: pg_sys::Oid,
        segment_id: &[u8; 16],
    ) {
        let guard = self.lock.acquire_exclusive();
        let slots = unsafe { self.slots_mut(&guard) };
        let slab_pool = unsafe { self.slab_pool_mut(&guard) };
        for slot in slots {
            if slot.state != SlotState::Empty
                && slot.key.database_oid == database_oid
                && slot.key.index_oid == index_oid
                && slot.key.segment_id == *segment_id
            {
                if slot.active_readers.load(Ordering::Acquire) == 0 {
                    slab_pool.free(slot.span.offset, slot.span.order);
                    *slot = CacheSlot::empty();
                } else {
                    slot.state = SlotState::Superseded;
                }
            }
        }
        let inflights = unsafe { self.inflight_builds_mut(&guard) };
        let mut cancelled_any = false;
        for build in inflights {
            if !build.is_empty()
                && build.key.database_oid == database_oid
                && build.key.index_oid == index_oid
                && build.key.segment_id == *segment_id
            {
                *build = InflightBuild::empty();
                cancelled_any = true;
            }
        }
        if cancelled_any {
            self.cv.broadcast();
        }
        self.generation.fetch_add(1, Ordering::Release);
    }

    /// Transition all cache slots for `index_oid` to [`SlotState::Superseded`].
    ///
    /// Called on relation drop, truncate, or reindex. If a matching slot has zero active readers,
    /// its slab is eagerly freed back to the [`SlabPool`]. Any in-flight builds for this relation
    /// are cancelled, and waiting workers are awakened via `cv.broadcast()`.
    pub fn invalidate_index(&self, database_oid: pg_sys::Oid, index_oid: pg_sys::Oid) {
        let guard = self.lock.acquire_exclusive();
        let slots = unsafe { self.slots_mut(&guard) };
        let slab_pool = unsafe { self.slab_pool_mut(&guard) };
        for slot in slots {
            if slot.state != SlotState::Empty
                && slot.key.database_oid == database_oid
                && slot.key.index_oid == index_oid
            {
                if slot.active_readers.load(Ordering::Acquire) == 0 {
                    slab_pool.free(slot.span.offset, slot.span.order);
                    *slot = CacheSlot::empty();
                } else {
                    slot.state = SlotState::Superseded;
                }
            }
        }
        let inflights = unsafe { self.inflight_builds_mut(&guard) };
        let mut cancelled_any = false;
        for build in inflights {
            if !build.is_empty()
                && build.key.database_oid == database_oid
                && build.key.index_oid == index_oid
            {
                *build = InflightBuild::empty();
                cancelled_any = true;
            }
        }
        if cancelled_any {
            self.cv.broadcast();
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
    inflight_index: usize,
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
        let inflights = unsafe { cache.inflight_builds_mut(&guard) };
        if self.inflight_index < inflights.len() {
            inflights[self.inflight_index] = InflightBuild::empty();
            cache.cv.broadcast();
        }
    }
}

/// Retrieve a cached directory for `key`, or build and install it using `build_fn`.
///
/// # Concurrency & Lifecycle
///
/// 1. **Cache Hit**: Checks `slots` for an existing [`SlotState::Ready`] entry matching `key`.
///    If found, increments `active_readers` and the saturated `usage_count` (0..=5), returning a zero-copy
///    [`SharedMemoryDirectory`] pointing directly to the contiguous slab in shared memory.
/// 2. **In-Flight Coordination**: If another worker is already building the segment, waits on
///    [`ConditionVariable`]. If the builder process crashed or was killed (`!is_pid_alive`), resets the
///    in-flight entry and allows another worker to take over.
/// 3. **Segment Building & Packing**: Exactly one worker builds the segment in private backend memory
///    via `build_fn`, packs all component files into a single contiguous buffer with a TOC ([`pack::pack_segment`]),
///    and allocates a power-of-two slab from [`SlabPool`].
/// 4. **Eager Supersession**: Older bounds for the same segment UUID are transitioned to [`SlotState::Superseded`].
///    If an older bound has zero active readers, its slab is immediately freed back to the [`SlabPool`].
/// 5. **Unshared Fallback**: If the shared arena or slot table is full and cannot be evicted (e.g., all memory
///    is pinned by active readers), logs a warning and returns backend-private `RamDirectory` without failing the query.
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
                let _ = slot
                    .usage_count
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |u| {
                        Some((u + 1).min(5))
                    });
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
        let inflights = unsafe { cache.inflight_builds_mut(&guard) };
        let mut in_flight_building = false;
        for build in inflights.iter_mut() {
            if !build.is_empty() && build.key == *key {
                if unsafe { is_pid_alive(build.builder_pid) } {
                    in_flight_building = true;
                    break;
                } else {
                    // Builder process crashed: reset entry and allow retry
                    *build = InflightBuild::empty();
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

        // 3. Find an empty inflight slot to claim build
        let mut claim_build_idx = None;
        for (idx, build) in inflights.iter().enumerate() {
            if build.is_empty() {
                claim_build_idx = Some(idx);
                break;
            }
        }

        let Some(build_idx) = claim_build_idx else {
            drop(guard);
            pgrx::warning!(
                "pg_search: mutable segment cache inflight builds full, falling back to local indexing"
            );
            let ram_dir = build_fn()?;
            return Ok(Arc::new(ram_dir));
        };

        // Claim inflight build
        let my_pid = unsafe { pg_sys::MyProcPid };
        inflights[build_idx].key = *key;
        inflights[build_idx].builder_pid = my_pid;

        drop(guard);

        // We are now the designated builder
        let mut build_guard = InflightBuildGuard {
            inflight_index: build_idx,
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
                let inflights = unsafe { cache.inflight_builds_mut(&guard) };
                inflights[build_idx] = InflightBuild::empty();
                cache.cv.broadcast();
                drop(guard);
                return Ok(Arc::new(ram_dir));
            }
        };

        let guard = cache.lock.acquire_exclusive();

        // Verify that our inflight build was not cancelled while indexing
        let inflights = unsafe { cache.inflight_builds_mut(&guard) };
        if inflights[build_idx].key != *key || inflights[build_idx].builder_pid != my_pid {
            build_guard.completed = true;
            drop(guard);
            return Ok(Arc::new(ram_dir));
        }

        let span = match cache.allocate(packed_bytes.len(), &guard) {
            Some(span) => span,
            None => {
                pgrx::warning!(
                    "pg_search: mutable segment cache arena full, falling back to local indexing"
                );
                build_guard.completed = true;
                inflights[build_idx] = InflightBuild::empty();
                cache.cv.broadcast();
                drop(guard);
                return Ok(Arc::new(ram_dir));
            }
        };

        // Find a slot in slots for the new Ready segment
        let slots = unsafe { cache.slots_mut(&guard) };
        let mut claim_slot_idx = slots.iter().position(|s| s.state == SlotState::Empty);

        if claim_slot_idx.is_none() {
            let slab_pool = unsafe { cache.slab_pool_mut(&guard) };
            claim_slot_idx = cache.evict_slot_for_claim(slots, slab_pool);
        }

        let Some(slot_idx) = claim_slot_idx else {
            pgrx::warning!(
                "pg_search: mutable segment cache slot table full, falling back to local indexing"
            );
            build_guard.completed = true;
            let slab_pool = unsafe { cache.slab_pool_mut(&guard) };
            slab_pool.free(span.offset, span.order);
            let inflights = unsafe { cache.inflight_builds_mut(&guard) };
            inflights[build_idx] = InflightBuild::empty();
            cache.cv.broadcast();
            drop(guard);
            return Ok(Arc::new(ram_dir));
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
        slots[slot_idx].key = *key;
        slots[slot_idx].span = span;
        slots[slot_idx].state = SlotState::Ready;
        slots[slot_idx].active_readers.store(1, Ordering::Release);
        slots[slot_idx].usage_count.store(1, Ordering::Release);

        // Supersede any older bound for this same segment and eagerly reclaim if no active readers
        let slab_pool = unsafe { cache.slab_pool_mut(&guard) };
        for (i, other_slot) in slots.iter_mut().enumerate() {
            if i != slot_idx
                && other_slot.state == SlotState::Ready
                && other_slot.key.matches_segment(key)
            {
                if other_slot.active_readers.load(Ordering::Acquire) == 0 {
                    slab_pool.free(other_slot.span.offset, other_slot.span.order);
                    *other_slot = CacheSlot::empty();
                } else {
                    other_slot.state = SlotState::Superseded;
                }
            }
        }

        // Release inflight build
        let inflights = unsafe { cache.inflight_builds_mut(&guard) };
        inflights[build_idx] = InflightBuild::empty();
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
