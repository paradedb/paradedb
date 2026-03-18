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

//! Generic DSM-based segment cache.
//!
//! Caches pre-parsed, immutable data derived from tantivy segment files in PostgreSQL
//! dynamic shared memory (DSM).  Data is computed once by the first backend that needs it
//! and shared across all backends via a shared-memory registry.
//!
//! Postgres DSM dynamically created and pinned so it outlasts the creating backend, and
//! will not be dropped until this pin is removed (on segment merge / index drop) AND all
//! backends drop references.
//!
//! NOTE: Ideally this registry would use PostgreSQL's `ShmemInitHash` (dynahash), but
//! hash_search produces incorrect results when called via Rust FFI on macOS/ARM64
//! (pgrx 0.17.0, pg18).  As a workaround we use a flat array with linear scan.
//! At the expected entry counts (bounded by DSM segment limits) this is negligible
//! compared to the DSM attach / tantivy parse cost on a miss.
//!
//! **Lifecycle:**
//! - Creation: `get_or_create` runs `fill_fn` into a new DSM segment, pins it globally.
//! - Access: `try_get` / `get_or_create` look up the handle, attach, and cache the pointer
//!   in a per-backend thread-local for O(1) subsequent access.
//! - Invalidation: `invalidate_segment` / `invalidate_index` remove entries from the shared
//!   array, unpin the global segment, and bump a generation counter. The DSM segment will then
//!   be cleaned up by Postgres when all backends drop references.
//! - Cleanup: on the next access after a generation change, each backend clears its
//!   thread-local cache and detaches stale mappings, freeing DSM memory once all
//!   backends have swept. This has negligible impact, even if run per query.
//! - DROP: an `object_access_hook` fires on `DROP TABLE` / `DROP INDEX` to clean up entries.

mod sql;

use pgrx::pg_sys;
use stable_deref_trait::StableDeref;
use crate::api::{HashMap, HashSet};
use crate::postgres::dsm::DsmSegment;
use crate::postgres::locks::LWLock;
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicU64, Ordering};
use tantivy::directory::OwnedBytes;

// ---------------------------------------------------------------------------
// Cache key & entry — stored in a flat shared-memory array
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub(super) struct DsmCacheKey {
    pub(super) index_oid: pg_sys::Oid,
    pub(super) segment_id: [u8; 16],
    pub(super) tag: u32,
    pub(super) sub_key: u32,
}

impl DsmCacheKey {
    pub(super) fn is_empty(&self) -> bool {
        // An empty slot has index_oid == 0 (InvalidOid)
        self.index_oid == pg_sys::Oid::INVALID
    }
}

/// One slot in the shared flat array.
#[repr(C)]
pub(super) struct DsmCacheSlot {
    pub(super) key: DsmCacheKey,
    pub(super) handle: pg_sys::dsm_handle,
    pub(super) size: u32,
}

/// Header for the shared-memory region (followed by `max_entries` DsmCacheSlots).
#[repr(C)]
pub(super) struct DsmCacheHeader {
    /// Number of occupied slots.
    pub(super) count: AtomicU32,
    /// Maximum number of slots (set once at init, never changes).
    pub(super) max_entries: u32,
    /// Invalidation generation counter.
    pub(super) generation: AtomicU64,
}

// ---------------------------------------------------------------------------
// CacheTag — add variants here as new cache consumers are implemented
// ---------------------------------------------------------------------------

#[repr(u32)]
#[derive(Debug, Clone, Copy, num_enum::TryFromPrimitive)]
#[allow(dead_code)]
pub enum CacheTag {
    /// Reserved placeholder. Real variants are added by cache consumers.
    Reserved = 0,
    /// Used by tests only.
    #[cfg(any(test, feature = "pg_test"))]
    Test = 999,
}

// ---------------------------------------------------------------------------
// CacheKey — public key type for cache lookups
// ---------------------------------------------------------------------------

/// Key identifying a cached DSM entry.
pub struct CacheKey {
    pub index_oid: pg_sys::Oid,
    pub segment_id: [u8; 16],
    pub tag: CacheTag,
    pub sub_key: u32,
}

impl CacheKey {
    fn to_internal(&self) -> DsmCacheKey {
        DsmCacheKey {
            index_oid: self.index_oid,
            segment_id: self.segment_id,
            tag: self.tag as u32,
            sub_key: self.sub_key,
        }
    }
}

// ---------------------------------------------------------------------------
// MappingGuard — prevents dsm_detach until all DsmSlices are dropped
// ---------------------------------------------------------------------------

/// Ref-counted guard that keeps a DSM mapping alive.  When the last clone is
/// dropped, `dsm_detach` is called to release the per-backend mapping.
///
/// Uses `dsm_find_mapping` to check whether the mapping still exists before
/// detaching — this is safe during backend shutdown because Postgres's
/// `dsm_backend_shutdown` may have already detached the segment.
struct MappingGuard {
    handle: pg_sys::dsm_handle,
}

impl Drop for MappingGuard {
    fn drop(&mut self) {
        let seg = unsafe { pg_sys::dsm_find_mapping(self.handle) };
        if !seg.is_null() {
            unsafe { pg_sys::dsm_detach(seg) };
        }
    }
}

// SAFETY: DSM mappings are per-process; MappingGuard only exists within a PG backend.
unsafe impl Send for MappingGuard {}
unsafe impl Sync for MappingGuard {}

// ---------------------------------------------------------------------------
// DsmSlice — zero-copy wrapper over DSM memory
// ---------------------------------------------------------------------------

pub struct DsmSlice {
    ptr: *const u8,
    len: usize,
    /// Prevents the DSM mapping from being detached while this slice exists.
    _guard: Arc<MappingGuard>,
}

impl Deref for DsmSlice {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        // SAFETY: the DSM mapping is held alive by _guard's Arc<MappingGuard>.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

// SAFETY: the underlying DSM memory is pinned and immutable after creation.
// The Arc<MappingGuard> ensures the mapping outlives this DsmSlice.
unsafe impl StableDeref for DsmSlice {}
unsafe impl Send for DsmSlice {}
unsafe impl Sync for DsmSlice {}

impl DsmSlice {
    #[allow(dead_code)] // remove when cache consumers land
    pub fn into_owned_bytes(self) -> OwnedBytes {
        OwnedBytes::new(self)
    }
}

/// Combined pointer to the shared-memory header and its protecting LWLock.
/// Both are initialized together in `shmem_startup` and share the same lifecycle.
pub(super) struct DsmCacheShared {
    pub(super) header: *mut DsmCacheHeader,
    pub(super) lock: LWLock,
}

impl DsmCacheShared {
    /// Get the slot array as a slice.
    ///
    /// # Safety
    /// Caller must hold the LWLock (shared or exclusive).
    pub(super) unsafe fn slot_slice(&self) -> &[DsmCacheSlot] {
        let max = (*self.header).max_entries as usize;
        std::slice::from_raw_parts(self.header.add(1) as *const DsmCacheSlot, max)
    }

    /// Get the slot array as a mutable slice.
    ///
    /// # Safety
    /// Caller must hold the LWLock exclusively.
    unsafe fn slot_slice_mut(&self) -> &mut [DsmCacheSlot] {
        let max = (*self.header).max_entries as usize;
        std::slice::from_raw_parts_mut(self.header.add(1) as *mut DsmCacheSlot, max)
    }

    /// Linear scan for a matching key.  Returns `(handle, size)` if found.
    ///
    /// # Safety
    /// Caller must hold the LWLock (shared or exclusive).
    unsafe fn find_slot(&self, key: &DsmCacheKey) -> Option<(pg_sys::dsm_handle, u32)> {
        for slot in self.slot_slice() {
            if slot.key == *key {
                return Some((slot.handle, slot.size));
            }
            if slot.key.is_empty() {
                break;
            }
        }
        None
    }

    /// Insert into the first empty slot.  Returns false if full.
    ///
    /// # Safety
    /// Caller must hold the LWLock exclusively.
    unsafe fn insert_slot(
        &self,
        key: &DsmCacheKey,
        handle: pg_sys::dsm_handle,
        size: u32,
    ) -> bool {
        for slot in self.slot_slice_mut() {
            if slot.key.is_empty() {
                slot.key = *key;
                slot.handle = handle;
                slot.size = size;
                (*self.header).count.fetch_add(1, Ordering::Relaxed);
                return true;
            }
        }
        false
    }

    /// Collect all live handles from the shared array.
    ///
    /// # Safety
    /// Caller must hold the LWLock (shared or exclusive).
    unsafe fn collect_live_handles(&self) -> HashSet<pg_sys::dsm_handle> {
        let mut live = HashSet::default();
        for slot in self.slot_slice() {
            if slot.key.is_empty() {
                break;
            }
            live.insert(slot.handle);
        }
        live
    }

    /// Remove all matching entries, compacting the array.  Returns removed handles.
    pub(super) fn remove_matching(&self, predicate: impl Fn(&DsmCacheKey) -> bool) -> Vec<pg_sys::dsm_handle> {
        // Fast path: skip the exclusive lock if the cache is empty.
        let count = unsafe { (*self.header).count.load(Ordering::Acquire) };
        if count == 0 {
            return Vec::new();
        }

        let mut removed_handles = Vec::new();

        unsafe {
            let _guard = self.lock.acquire_exclusive();
            let slots = self.slot_slice_mut();

            let mut write_idx = 0usize;
            for read_idx in 0..slots.len() {
                if slots[read_idx].key.is_empty() {
                    break;
                }
                if predicate(&slots[read_idx].key) {
                    removed_handles.push(slots[read_idx].handle);
                    (*self.header).count.fetch_sub(1, Ordering::Relaxed);
                } else {
                    if write_idx != read_idx {
                        slots[write_idx] = std::ptr::read(&slots[read_idx]);
                    }
                    write_idx += 1;
                }
            }

            for slot in &mut slots[write_idx..] {
                if slot.key.is_empty() {
                    break;
                }
                *slot = std::mem::zeroed();
            }
        }

        removed_handles
    }

    /// Bump the shared generation counter.
    pub(super) fn bump_generation(&self) {
        unsafe { (*self.header).generation.fetch_add(1, Ordering::Release) };
    }

    /// If the shared generation has changed since our last check, sweep stale mappings.
    /// Must be called BEFORE handing out any DsmSlice for the current query.
    fn maybe_sweep(&self) {
        let current_gen = unsafe { (*self.header).generation.load(Ordering::Acquire) };
        let local_gen = LOCAL_GENERATION.with(|g| g.get());
        if current_gen == local_gen {
            return;
        }

        LOCAL_CACHE.with(|c| c.borrow_mut().clear());
        self.sweep_stale_mappings();
        LOCAL_GENERATION.with(|g| g.set(current_gen));
    }

    /// Walk this backend's tracked handles and drop guards for any mappings that
    /// are no longer in the shared array (i.e., they were invalidated by another
    /// backend).  The actual `dsm_detach` is deferred until the last `DsmSlice`
    /// holding the mapping is also dropped.
    fn sweep_stale_mappings(&self) {
        let live_handles = unsafe {
            let _guard = self.lock.acquire_shared();
            self.collect_live_handles()
        };

        LOCAL_HANDLES.with(|h| {
            h.borrow_mut()
                .retain(|handle, _| live_handles.contains(handle));
        });
    }
}

static DSM_CACHE: AtomicPtr<DsmCacheShared> = AtomicPtr::new(std::ptr::null_mut());
static mut PREV_SHMEM_REQUEST_HOOK: pg_sys::shmem_request_hook_type = None;
static mut PREV_SHMEM_STARTUP_HOOK: pg_sys::shmem_startup_hook_type = None;
static mut PREV_OBJECT_ACCESS_HOOK: pg_sys::object_access_hook_type = None;

// Per-backend pointer cache and state.
// PostgreSQL backends are single-threaded, so thread-locals are safe without locks.
thread_local! {
    static LOCAL_CACHE: RefCell<HashMap<DsmCacheKey, (Arc<MappingGuard>, *const u8, usize)>> = RefCell::new(HashMap::default());
    /// Last seen generation.  When this differs from the shared counter,
    /// a sweep is needed before handing out any new DsmSlices.
    static LOCAL_GENERATION: Cell<u64> = const { Cell::new(0) };
    /// Guards for mappings we have attached.  Dropping an entry here decrements
    /// the Arc; the actual dsm_detach fires when the last DsmSlice is also dropped.
    static LOCAL_HANDLES: RefCell<HashMap<pg_sys::dsm_handle, Arc<MappingGuard>>> = RefCell::new(HashMap::default());
}

// ---------------------------------------------------------------------------
// Shared-memory helpers
// ---------------------------------------------------------------------------

/// Load the shared header+lock pair.  Returns `None` if the cache has not
/// been initialized (e.g. extension not loaded via shared_preload_libraries).
pub(super) fn load_shared() -> Option<&'static DsmCacheShared> {
    let ptr = DSM_CACHE.load(Ordering::Relaxed);
    if ptr.is_null() {
        return None;
    }
    // SAFETY: once stored in shmem_startup, the pointer is valid for the
    // lifetime of the postmaster and never freed or moved.
    Some(unsafe { &*ptr })
}

/// Compute max entries to match the DSM segment limit.
/// PostgreSQL sizes its DSM control array as `64 + 5 * MaxBackends`.
fn max_entries() -> usize {
    let max_backends = unsafe { pg_sys::MaxBackends } as usize;
    64 + 5 * max_backends
}

/// Total shared-memory bytes needed for header + slot array.
fn shmem_size(max_entries: usize) -> usize {
    std::mem::size_of::<DsmCacheHeader>() + max_entries * std::mem::size_of::<DsmCacheSlot>()
}

// ---------------------------------------------------------------------------
// Initialization (called from _PG_init)
// ---------------------------------------------------------------------------

/// Hook into shmem_request and shmem_startup to set up the DSM cache.
///
/// # Safety
///
/// Must be called from `_PG_init()` while `process_shared_preload_libraries_in_progress` is true.
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

    pg_sys::RequestNamedLWLockTranche(c"pg_search_dsm_cache".as_ptr(), 1);
    pg_sys::RequestAddinShmemSpace(shmem_size(max_entries()));
}

unsafe extern "C-unwind" fn shmem_startup() {
    if let Some(prev) = PREV_SHMEM_STARTUP_HOOK {
        prev();
    }

    // Get the LWLock we requested.
    let lock = LWLock::from_raw(
        pg_sys::GetNamedLWLockTranche(c"pg_search_dsm_cache".as_ptr()) as *mut pg_sys::LWLock,
    );

    let max = max_entries();
    let size = shmem_size(max);

    let mut found = false;
    let header = pg_sys::ShmemInitStruct(c"pg_search_dsm_cache".as_ptr(), size, &mut found)
        as *mut DsmCacheHeader;

    // Since we're loaded via shared_preload_libraries, _PG_init (and therefore
    // shmem_startup) is called exactly once.  found == true would indicate a bug.
    assert!(
        !found,
        "pg_search_dsm_cache: shared memory already initialized"
    );

    // Zero-fill the entire region (header + slots).
    std::ptr::write_bytes(header as *mut u8, 0, size);
    (*header).max_entries = max as u32;
    (*header).count = AtomicU32::new(0);
    (*header).generation = AtomicU64::new(0);

    // Store the combined struct.  Leaked Box is intentional — lives for the
    // lifetime of the postmaster process.
    let shared = Box::leak(Box::new(DsmCacheShared { header, lock }));
    DSM_CACHE.store(shared, Ordering::Relaxed);
}

/// Object access hook: invalidate DSM cache entries when an index is dropped.
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

    // We only care about DROP events on relations (indexes).
    if access == pg_sys::ObjectAccessType::OAT_DROP
        && class_id == pg_sys::RelationRelationId
        && load_shared().is_some()
    {
        invalidate_index(object_id);
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Look up an existing DSM cache entry without creating one on miss.
/// Returns `None` if not found or cache not initialized.
#[allow(dead_code)] // remove when cache consumers land
pub fn try_get(key: &CacheKey) -> Option<DsmSlice> {
    let shared = load_shared()?;

    shared.maybe_sweep();

    let key = key.to_internal();

    // Thread-local fast path
    let cached = LOCAL_CACHE.with(|c| {
        c.borrow()
            .get(&key)
            .map(|(guard, ptr, len)| (guard.clone(), *ptr, *len))
    });
    if let Some((guard, ptr, len)) = cached {
        return Some(DsmSlice {
            ptr,
            len,
            _guard: guard,
        });
    }

    // Shared array lookup
    let found = unsafe {
        let _guard = shared.lock.acquire_shared();
        shared.find_slot(&key)
    };
    if let Some((handle, size)) = found {
        return resolve_and_cache(key, handle, size as usize);
    }

    None
}

/// Get or create a cached DSM entry.
///
/// On miss, calls `fill_fn` to populate a new DSM segment.
/// Returns `None` if the cache is not initialized, DSM creation fails (max segments reached),
/// or the data size is zero.
#[allow(dead_code)] // remove when cache consumers land
pub fn get_or_create(
    key: &CacheKey,
    data_size: usize,
    fill_fn: impl FnOnce(&mut [u8]),
) -> Option<DsmSlice> {
    if data_size == 0 {
        return None;
    }

    let shared = load_shared()?;

    shared.maybe_sweep();

    let key = key.to_internal();

    // 1. Thread-local fast path
    let cached = LOCAL_CACHE.with(|c| {
        c.borrow()
            .get(&key)
            .map(|(guard, ptr, len)| (guard.clone(), *ptr, *len))
    });
    if let Some((guard, ptr, len)) = cached {
        return Some(DsmSlice {
            ptr,
            len,
            _guard: guard,
        });
    }

    // 2. Shared array lookup (shared lock)
    let found = unsafe {
        let _guard = shared.lock.acquire_shared();
        shared.find_slot(&key)
    };
    if let Some((handle, size)) = found {
        if let Some(slice) = resolve_and_cache(key, handle, size as usize) {
            return Some(slice);
        }
        // Segment gone (race with invalidation) — fall through to create
    }

    // 3. Miss — create DSM outside the lock
    let Some(seg) = DsmSegment::create(data_size) else {
        pgrx::log!(
            "pg_search: DSM segment limit reached, cache entry unavailable (tag={:?}, size={})",
            CacheTag::try_from(key.tag).ok(),
            data_size,
        );
        return None;
    };

    unsafe { fill_fn(seg.as_mut_slice(data_size)) };

    // Pin the segment BEFORE making it visible in the shared array.
    // Otherwise another backend could find the entry and call
    // dsm_unpin_segment before we've pinned it.
    seg.pin();
    let handle = seg.handle();

    // 4. Insert into array (exclusive lock), with double-check.
    //    Three outcomes: Raced (another backend inserted), Full, or Inserted.
    enum InsertResult {
        Raced(pg_sys::dsm_handle, u32),
        Full,
        Inserted,
    }

    let result = unsafe {
        let _guard = shared.lock.acquire_exclusive();

        if let Some((their_handle, their_size)) = shared.find_slot(&key) {
            InsertResult::Raced(their_handle, their_size)
        } else if !shared.insert_slot(&key, handle, data_size as u32) {
            InsertResult::Full
        } else {
            InsertResult::Inserted
        }
    };

    match result {
        InsertResult::Raced(their_handle, their_size) => {
            unsafe { DsmSegment::unpin(handle) };
            drop(seg);
            return resolve_and_cache(key, their_handle, their_size as usize);
        }
        InsertResult::Full => {
            unsafe { DsmSegment::unpin(handle) };
            drop(seg);
            pgrx::log!(
                "pg_search: DSM cache array full, cache entry unavailable (tag={:?})",
                CacheTag::try_from(key.tag).ok(),
            );
            return None;
        }
        InsertResult::Inserted => {}
    }

    let ptr = seg.address() as *const u8;

    // Track handle and cache pointer.  We forget the DsmSegment so its Drop
    // doesn't call dsm_detach — the mapping is kept alive by dsm_pin_mapping.
    // MappingGuard stores just the handle and uses dsm_find_mapping + dsm_detach
    // on drop, which is safe even if dsm_backend_shutdown has already detached it.
    std::mem::forget(seg);
    let guard = Arc::new(MappingGuard { handle });
    LOCAL_HANDLES.with(|h| {
        h.borrow_mut()
            .entry(handle)
            .or_insert_with(|| guard.clone());
    });
    LOCAL_CACHE.with(|c| c.borrow_mut().insert(key, (guard.clone(), ptr, data_size)));

    Some(DsmSlice {
        ptr,
        len: data_size,
        _guard: guard,
    })
}

/// Invalidate all cached entries for a specific segment (called on merge/delete).
pub fn invalidate_segment(index_oid: pg_sys::Oid, segment_id: &[u8; 16]) {
    let Some(shared) = load_shared() else {
        return;
    };
    let handles =
        shared.remove_matching(|k| k.index_oid == index_oid && k.segment_id == *segment_id);
    unpin_and_bump(shared, handles);
}

/// Invalidate all cached entries for an index (called on DROP INDEX).
pub fn invalidate_index(index_oid: pg_sys::Oid) {
    let Some(shared) = load_shared() else {
        return;
    };
    let handles = shared.remove_matching(|k| k.index_oid == index_oid);
    unpin_and_bump(shared, handles);
}

fn unpin_and_bump(shared: &DsmCacheShared, handles: Vec<pg_sys::dsm_handle>) {
    if !handles.is_empty() {
        for handle in handles {
            unsafe { DsmSegment::unpin(handle) };
        }
        shared.bump_generation();
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

pub(super) fn tag_name(tag: u32) -> String {
    match CacheTag::try_from(tag) {
        Ok(t) => format!("{t:?}"),
        Err(_) => format!("Unknown({tag})"),
    }
}

/// Resolve a DSM handle to a mapped pointer and cache in thread-local.
fn resolve_and_cache(
    key: DsmCacheKey,
    handle: pg_sys::dsm_handle,
    size: usize,
) -> Option<DsmSlice> {
    let slice = resolve_handle(handle, size)?;
    LOCAL_CACHE.with(|c| {
        c.borrow_mut()
            .insert(key, (slice._guard.clone(), slice.ptr, slice.len))
    });
    Some(slice)
}

/// Resolve a DSM handle to a mapped pointer.
/// Uses `DsmSegment::find_mapping` first (reuses existing pinned mapping),
/// falls back to `DsmSegment::attach` + `pin_mapping` on first access.
fn resolve_handle(handle: pg_sys::dsm_handle, size: usize) -> Option<DsmSlice> {
    // Check if we already have a guard for this handle.
    let existing_guard = LOCAL_HANDLES.with(|h| h.borrow().get(&handle).cloned());

    if let Some(guard) = existing_guard {
        if let Some(seg_ref) = DsmSegment::find_mapping(handle) {
            return Some(DsmSlice {
                ptr: seg_ref.address(),
                len: size,
                _guard: guard,
            });
        }
        // Guard exists but mapping is gone — remove the stale guard so the
        // new one created below gets inserted (otherwise the old guard's drop
        // could detach the new mapping).
        LOCAL_HANDLES.with(|h| {
            h.borrow_mut().remove(&handle);
        });
    }

    // Slow path: first access in this backend — attach and pin
    let seg = DsmSegment::attach(handle)?;
    seg.pin_mapping();
    let ptr = seg.address() as *const u8;
    std::mem::forget(seg);

    let guard = Arc::new(MappingGuard { handle });
    LOCAL_HANDLES.with(|h| {
        h.borrow_mut()
            .entry(handle)
            .or_insert_with(|| guard.clone());
    });

    Some(DsmSlice {
        ptr,
        len: size,
        _guard: guard,
    })
}

