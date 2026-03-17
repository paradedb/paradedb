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

use pgrx::pg_sys;
use pgrx::prelude::*;
use stable_deref_trait::StableDeref;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
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
struct DsmCacheKey {
    index_oid: pg_sys::Oid,
    segment_id: [u8; 16],
    tag: u32,
    sub_key: u32,
}

impl DsmCacheKey {
    fn is_empty(&self) -> bool {
        // An empty slot has index_oid == 0 (InvalidOid)
        self.index_oid == pg_sys::Oid::INVALID
    }
}

/// One slot in the shared flat array.
#[repr(C)]
struct DsmCacheSlot {
    key: DsmCacheKey,
    handle: pg_sys::dsm_handle,
    size: u32,
}

/// Header for the shared-memory region (followed by `max_entries` DsmCacheSlots).
#[repr(C)]
struct DsmCacheHeader {
    /// Number of occupied slots.
    count: AtomicU32,
    /// Maximum number of slots (set once at init, never changes).
    max_entries: u32,
    /// Invalidation generation counter.
    generation: AtomicU64,
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
// MappingGuard — prevents dsm_detach until all DsmSlices are dropped
// ---------------------------------------------------------------------------

/// Ref-counted guard that keeps a DSM mapping alive.  When the last clone is
/// dropped, `dsm_detach` is called to release the per-backend mapping.
struct MappingGuard {
    handle: pg_sys::dsm_handle,
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here. Using impl_safe_drop
// would leak the DSM mapping on panic, and since DSM segments are globally pinned,
// that leaked memory would never be reclaimed.  dsm_find_mapping and dsm_detach
// are lightweight operations that do not raise PostgreSQL errors, so calling them
// during unwind is safe.
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
struct DsmCacheShared {
    header: *mut DsmCacheHeader,
    lock: *mut pg_sys::LWLock,
}

impl DsmCacheShared {
    /// Get a pointer to the slot array (immediately after the header).
    unsafe fn slots(&self) -> *mut DsmCacheSlot {
        self.header.add(1) as *mut DsmCacheSlot
    }

    /// Linear scan for a matching key.  Returns `(handle, size)` if found.
    ///
    /// # Safety
    /// Caller must hold the LWLock (shared or exclusive).
    unsafe fn find_slot(&self, key: &DsmCacheKey) -> Option<(pg_sys::dsm_handle, u32)> {
        let max = (*self.header).max_entries as usize;
        let base = self.slots();
        for i in 0..max {
            let slot = &*base.add(i);
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
        let max = (*self.header).max_entries as usize;
        let base = self.slots();
        for i in 0..max {
            let slot = &mut *base.add(i);
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
    unsafe fn collect_live_handles(&self) -> std::collections::HashSet<pg_sys::dsm_handle> {
        let mut live = std::collections::HashSet::new();
        let max = (*self.header).max_entries as usize;
        let base = self.slots();
        for i in 0..max {
            let slot = &*base.add(i);
            if slot.key.is_empty() {
                break;
            }
            live.insert(slot.handle);
        }
        live
    }

    /// Remove all matching entries, compacting the array.  Returns removed handles.
    fn remove_matching(&self, predicate: impl Fn(&DsmCacheKey) -> bool) -> Vec<pg_sys::dsm_handle> {
        // Fast path: skip the exclusive lock if the cache is empty.
        let count = unsafe { (*self.header).count.load(Ordering::Acquire) };
        if count == 0 {
            return Vec::new();
        }

        let mut removed_handles = Vec::new();

        unsafe {
            pg_sys::LWLockAcquire(self.lock, pg_sys::LWLockMode::LW_EXCLUSIVE);

            let max = (*self.header).max_entries as usize;
            let base = self.slots();

            let mut write_idx = 0usize;
            for read_idx in 0..max {
                let slot = &*base.add(read_idx);
                if slot.key.is_empty() {
                    break;
                }
                if predicate(&slot.key) {
                    removed_handles.push(slot.handle);
                    (*self.header).count.fetch_sub(1, Ordering::Relaxed);
                } else {
                    if write_idx != read_idx {
                        std::ptr::copy_nonoverlapping(
                            base.add(read_idx),
                            base.add(write_idx),
                            1,
                        );
                    }
                    write_idx += 1;
                }
            }

            for i in write_idx..max {
                let slot = &mut *base.add(i);
                if slot.key.is_empty() {
                    break;
                }
                *slot = std::mem::zeroed();
            }

            pg_sys::LWLockRelease(self.lock);
        }

        removed_handles
    }

    /// Bump the shared generation counter.
    fn bump_generation(&self) {
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
            pg_sys::LWLockAcquire(self.lock, pg_sys::LWLockMode::LW_SHARED);
            let h = self.collect_live_handles();
            pg_sys::LWLockRelease(self.lock);
            h
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
    static LOCAL_CACHE: RefCell<HashMap<DsmCacheKey, (Arc<MappingGuard>, *const u8, usize)>> = RefCell::new(HashMap::new());
    /// Last seen generation.  When this differs from the shared counter,
    /// a sweep is needed before handing out any new DsmSlices.
    static LOCAL_GENERATION: Cell<u64> = const { Cell::new(0) };
    /// Guards for mappings we have attached.  Dropping an entry here decrements
    /// the Arc; the actual dsm_detach fires when the last DsmSlice is also dropped.
    static LOCAL_HANDLES: RefCell<HashMap<pg_sys::dsm_handle, Arc<MappingGuard>>> = RefCell::new(HashMap::new());
}

// ---------------------------------------------------------------------------
// Shared-memory helpers
// ---------------------------------------------------------------------------

/// Load the shared header+lock pair.  Returns `None` if the cache has not
/// been initialized (e.g. extension not loaded via shared_preload_libraries).
fn load_shared() -> Option<&'static DsmCacheShared> {
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
    let lock = pg_sys::GetNamedLWLockTranche(c"pg_search_dsm_cache".as_ptr())
        as *mut pg_sys::LWLock;

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
pub fn try_get(
    index_oid: pg_sys::Oid,
    segment_id: &[u8; 16],
    tag: CacheTag,
    sub_key: u32,
) -> Option<DsmSlice> {
    let shared = load_shared()?;

    shared.maybe_sweep();

    let key = DsmCacheKey {
        index_oid,
        segment_id: *segment_id,
        tag: tag as u32,
        sub_key,
    };

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
    unsafe {
        pg_sys::LWLockAcquire(shared.lock, pg_sys::LWLockMode::LW_SHARED);

        if let Some((handle, size)) = shared.find_slot(&key) {
            pg_sys::LWLockRelease(shared.lock);
            return resolve_and_cache(key, handle, size as usize);
        }

        pg_sys::LWLockRelease(shared.lock);
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
    index_oid: pg_sys::Oid,
    segment_id: &[u8; 16],
    tag: CacheTag,
    sub_key: u32,
    data_size: usize,
    fill_fn: impl FnOnce(&mut [u8]),
) -> Option<DsmSlice> {
    if data_size == 0 {
        return None;
    }

    let shared = load_shared()?;

    shared.maybe_sweep();

    let key = DsmCacheKey {
        index_oid,
        segment_id: *segment_id,
        tag: tag as u32,
        sub_key,
    };

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
    unsafe {
        pg_sys::LWLockAcquire(shared.lock, pg_sys::LWLockMode::LW_SHARED);

        if let Some((handle, size)) = shared.find_slot(&key) {
            pg_sys::LWLockRelease(shared.lock);

            if let Some(slice) = resolve_and_cache(key, handle, size as usize) {
                return Some(slice);
            }
            // Segment gone (race with invalidation) — fall through to create
        } else {
            pg_sys::LWLockRelease(shared.lock);
        }
    }

    // 3. Miss — create DSM outside the lock
    unsafe {
        let seg = pg_sys::dsm_create(data_size, pg_sys::DSM_CREATE_NULL_IF_MAXSEGMENTS as _);
        if seg.is_null() {
            pgrx::log!(
                "pg_search: DSM segment limit reached, cache entry unavailable (tag={:?}, size={})",
                CacheTag::try_from(key.tag).ok(),
                data_size,
            );
            return None;
        }

        let ptr = pg_sys::dsm_segment_address(seg) as *mut u8;
        let buf = std::slice::from_raw_parts_mut(ptr, data_size);
        fill_fn(buf);

        let handle = pg_sys::dsm_segment_handle(seg);

        // Pin the segment BEFORE making it visible in the shared array.
        // Otherwise another backend could find the entry and call
        // dsm_unpin_segment before we've pinned it.
        pg_sys::dsm_pin_segment(seg);
        pg_sys::dsm_pin_mapping(seg);

        // 4. Insert into array (exclusive lock), with double-check
        pg_sys::LWLockAcquire(shared.lock, pg_sys::LWLockMode::LW_EXCLUSIVE);

        // Check if another backend inserted while we were creating
        if let Some((their_handle, their_size)) = shared.find_slot(&key) {
            pg_sys::LWLockRelease(shared.lock);
            pg_sys::dsm_unpin_segment(handle);
            pg_sys::dsm_detach(seg);
            return resolve_and_cache(key, their_handle, their_size as usize);
        }

        // Try to insert into a free slot
        if !shared.insert_slot(&key, handle, data_size as u32) {
            pg_sys::LWLockRelease(shared.lock);
            pg_sys::dsm_unpin_segment(handle);
            pg_sys::dsm_detach(seg);
            pgrx::log!(
                "pg_search: DSM cache array full, cache entry unavailable (tag={:?})",
                CacheTag::try_from(key.tag).ok(),
            );
            return None;
        }

        pg_sys::LWLockRelease(shared.lock);

        let ptr = pg_sys::dsm_segment_address(seg) as *const u8;

        // Track handle and cache pointer.
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
}

/// Invalidate all cached entries for a specific segment (called on merge/delete).
pub fn invalidate_segment(index_oid: pg_sys::Oid, segment_id: &[u8; 16]) {
    let Some(shared) = load_shared() else {
        return;
    };
    let handles =
        shared.remove_matching(|k| k.index_oid == index_oid && k.segment_id == *segment_id);

    if !handles.is_empty() {
        for handle in handles {
            unsafe {
                pg_sys::dsm_unpin_segment(handle);
            }
        }
        shared.bump_generation();
    }
}

/// Invalidate all cached entries for an index (called on DROP INDEX).
pub fn invalidate_index(index_oid: pg_sys::Oid) {
    let Some(shared) = load_shared() else {
        return;
    };
    let handles = shared.remove_matching(|k| k.index_oid == index_oid);

    if !handles.is_empty() {
        for handle in handles {
            unsafe {
                pg_sys::dsm_unpin_segment(handle);
            }
        }
        shared.bump_generation();
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn tag_name(tag: u32) -> String {
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
/// Uses `dsm_find_mapping` first (reuses existing pinned mapping),
/// falls back to `dsm_attach` + `dsm_pin_mapping` on first access.
fn resolve_handle(handle: pg_sys::dsm_handle, size: usize) -> Option<DsmSlice> {
    // Check if we already have a guard for this handle.
    let existing_guard = LOCAL_HANDLES.with(|h| h.borrow().get(&handle).cloned());

    if let Some(guard) = existing_guard {
        let seg = unsafe { pg_sys::dsm_find_mapping(handle) };
        if !seg.is_null() {
            let ptr = unsafe { pg_sys::dsm_segment_address(seg) } as *const u8;
            return Some(DsmSlice {
                ptr,
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

    unsafe {
        // Slow path: first access in this backend — attach and pin
        let seg = pg_sys::dsm_attach(handle);
        if seg.is_null() {
            return None;
        }
        pg_sys::dsm_pin_mapping(seg);
        let ptr = pg_sys::dsm_segment_address(seg) as *const u8;

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
}

// ---------------------------------------------------------------------------
// Test-only SQL functions
// ---------------------------------------------------------------------------

#[cfg(any(test, feature = "pg_test"))]
thread_local! {
    /// Stash for holding a DsmSlice across SQL calls, used to test refcounting.
    static HELD_SLICE: RefCell<Option<DsmSlice>> = RefCell::new(None);
}

/// Test-only: insert a cache entry with the Test tag and a payload of `size` zero-bytes.
#[cfg(any(test, feature = "pg_test"))]
#[pg_extern]
fn dsm_cache_test_insert(index_oid: pg_sys::Oid, segment_id: String, sub_key: i32, size: i32) {
    let uuid = uuid::Uuid::parse_str(&segment_id).expect("invalid segment_id UUID");
    get_or_create(
        index_oid,
        uuid.as_bytes(),
        CacheTag::Test,
        sub_key as u32,
        size as usize,
        |buf| buf.fill(0),
    );
}

/// Test-only: invalidate all cache entries for a given segment.
#[cfg(any(test, feature = "pg_test"))]
#[pg_extern]
fn dsm_cache_test_invalidate_segment(index_oid: pg_sys::Oid, segment_id: String) {
    let uuid = uuid::Uuid::parse_str(&segment_id).expect("invalid segment_id UUID");
    invalidate_segment(index_oid, uuid.as_bytes());
}

/// Test-only: invalidate all cache entries for a given index.
#[cfg(any(test, feature = "pg_test"))]
#[pg_extern]
fn dsm_cache_test_invalidate_index(index_oid: pg_sys::Oid) {
    invalidate_index(index_oid);
}

/// Test-only: remove all entries from the cache.
#[cfg(any(test, feature = "pg_test"))]
#[pg_extern]
fn dsm_cache_test_clear_all() {
    let Some(shared) = load_shared() else {
        return;
    };
    let handles = shared.remove_matching(|_| true);
    for handle in handles {
        unsafe {
            pg_sys::dsm_unpin_segment(handle);
        }
    }
    shared.bump_generation();
}

/// Test-only: create a cache entry and hold the DsmSlice in a thread-local stash.
/// Writes a known pattern (0xAB) so we can verify reads later.
/// Returns true if the entry was created successfully.
#[cfg(any(test, feature = "pg_test"))]
#[pg_extern]
fn dsm_cache_test_hold(
    index_oid: pg_sys::Oid,
    segment_id: String,
    sub_key: i32,
    size: i32,
) -> bool {
    let uuid = uuid::Uuid::parse_str(&segment_id).expect("invalid segment_id UUID");
    let slice = get_or_create(
        index_oid,
        uuid.as_bytes(),
        CacheTag::Test,
        sub_key as u32,
        size as usize,
        |buf| buf.fill(0xAB),
    );
    let ok = slice.is_some();
    HELD_SLICE.with(|h| *h.borrow_mut() = slice);
    ok
}

/// Test-only: read from the held DsmSlice.
/// Returns true if the held slice is still readable and contains the expected pattern.
#[cfg(any(test, feature = "pg_test"))]
#[pg_extern]
fn dsm_cache_test_read_held() -> bool {
    HELD_SLICE.with(|h| {
        let borrow = h.borrow();
        match borrow.as_ref() {
            Some(slice) => {
                let bytes: &[u8] = slice;
                !bytes.is_empty() && bytes.iter().all(|&b| b == 0xAB)
            }
            None => false,
        }
    })
}

/// Test-only: drop the held DsmSlice, releasing the Arc<MappingGuard>.
#[cfg(any(test, feature = "pg_test"))]
#[pg_extern]
fn dsm_cache_test_release() {
    HELD_SLICE.with(|h| *h.borrow_mut() = None);
}

// ---------------------------------------------------------------------------
// SQL-callable diagnostics
// ---------------------------------------------------------------------------

#[pg_extern]
fn dsm_cache_entries() -> TableIterator<
    'static,
    (
        name!(index_oid, pg_sys::Oid),
        name!(segment_id, String),
        name!(tag, String),
        name!(sub_key, i32),
        name!(dsm_handle, i64),
        name!(size_bytes, i64),
    ),
> {
    let Some(shared) = load_shared() else {
        return TableIterator::new(Vec::new());
    };

    let mut rows = Vec::new();

    unsafe {
        pg_sys::LWLockAcquire(shared.lock, pg_sys::LWLockMode::LW_SHARED);

        let max = (*shared.header).max_entries as usize;
        let base = shared.slots();
        for i in 0..max {
            let slot = &*base.add(i);
            if slot.key.is_empty() {
                break;
            }
            let seg_uuid = uuid::Uuid::from_bytes(slot.key.segment_id);
            rows.push((
                slot.key.index_oid,
                seg_uuid.to_string(),
                tag_name(slot.key.tag),
                slot.key.sub_key as i32,
                slot.handle as i64,
                slot.size as i64,
            ));
        }

        pg_sys::LWLockRelease(shared.lock);
    }

    TableIterator::new(rows)
}

#[pg_extern]
fn dsm_cache_stats() -> TableIterator<
    'static,
    (
        name!(entries, i64),
        name!(max_entries, i64),
        name!(total_bytes, i64),
    ),
> {
    let Some(shared) = load_shared() else {
        return TableIterator::once((0i64, 0i64, 0i64));
    };

    let mut count = 0i64;
    let mut total_bytes = 0i64;

    unsafe {
        pg_sys::LWLockAcquire(shared.lock, pg_sys::LWLockMode::LW_SHARED);

        let max = (*shared.header).max_entries as usize;
        let base = shared.slots();
        for i in 0..max {
            let slot = &*base.add(i);
            if slot.key.is_empty() {
                break;
            }
            count += 1;
            total_bytes += slot.size as i64;
        }

        let max_entries = (*shared.header).max_entries as i64;
        pg_sys::LWLockRelease(shared.lock);

        TableIterator::once((count, max_entries, total_bytes))
    }
}
