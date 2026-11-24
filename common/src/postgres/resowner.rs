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

//! `ResourceOwner`-scoped cleanup tokens ([`ResourceOwnerStamp`]) for PostgreSQL C resources.
//!
//! Checking `IsTransactionState()` alone in `Drop` is insufficient when a scan state lives in a
//! `MemoryContext` (`es_query_cxt`, `portalContext`, `fn_mcxt`, or `SPI` `execCxt`): PostgreSQL
//! can run `ResourceOwnerRelease` + `ResourceOwnerDelete` on that scan's `ResourceOwner` first
//! (for example when an outer cursor fails inside a `SAVEPOINT` and is later dropped at `CLOSE`
//! or `COMMIT`, when `PortalDrop` drops a `PORTAL_FAILED` cursor, or when an SPI query fails
//! inside an inner PL/pgSQL `EXCEPTION` block) and only later delete the `MemoryContext` while
//! `IsTransactionState()` is `true`.
//!
//! Capturing [`ResourceOwnerStamp::current`] at acquisition time and gating C cleanup with
//! [`ResourceOwnerStamp::run_if_live`] (or [`ResourceOwnerStamp::run_if_live_or_recovery`])
//! skips cleanup once `ResourceOwnerRelease` has begun (`RESOURCE_RELEASE_BEFORE_LOCKS`) on that
//! owner, and otherwise binds `CurrentResourceOwner` to that exact owner so `ResourceOwnerForget*`
//! always targets the owner that recorded the resource.
//!
//! Hot-path performance: `Buffer`, `PinnedBuffer`, and `BorrowedBuffer` capture and check a stamp
//! on every page read and release. To avoid per-pin syscalls (`getpid()`), atomics, `RefCell`
//! borrows, and `Vec` scans, we key process identity on `pg_sys::MyProcPid` (updated by Postgres
//! in `fork_process()` / `fork_process.c` after every `fork()`, and in `main.c` for postmaster)
//! and maintain a single-entry `Cell`-backed cache for the current backend's active
//! `ResourceOwner`.

#[cfg(not(test))]
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
#[cfg(not(test))]
use std::num::NonZeroI32;
use std::num::NonZeroU64;
#[cfg(not(test))]
use std::ptr::NonNull;
#[cfg(not(test))]
use std::sync::atomic::{AtomicI32, Ordering};

use crate::postgres::customscan::mpp::interrupt::is_main_thread;

struct LiveResourceOwners {
    pid: i32,
    next_stamp: u64,
    entries: Vec<(*mut c_void, NonZeroU64)>,
}

impl LiveResourceOwners {
    const fn new() -> Self {
        Self {
            pid: 0,
            next_stamp: 1,
            entries: Vec::new(),
        }
    }

    fn reset_if_forked(&mut self, pid: i32) {
        if self.pid != pid {
            self.pid = pid;
            self.entries.clear();
        }
    }

    fn stamp_for_owner(&mut self, pid: i32, owner: *mut c_void) -> NonZeroU64 {
        self.reset_if_forked(pid);
        if let Some(&(_, stamp)) = self.entries.iter().find(|&&(ptr, _)| ptr == owner) {
            return stamp;
        }
        let raw = self.next_stamp;
        self.next_stamp = self
            .next_stamp
            .checked_add(1)
            .expect("ResourceOwnerStamp generation counter overflowed");
        let stamp = NonZeroU64::new(raw).expect("next_stamp starts at 1");
        self.entries.push((owner, stamp));
        stamp
    }

    fn lookup_owner(&mut self, pid: i32, stamp: NonZeroU64) -> Option<*mut c_void> {
        self.reset_if_forked(pid);
        self.entries
            .iter()
            .find_map(|&(ptr, s)| (s == stamp).then_some(ptr))
    }

    fn remove_owner(&mut self, pid: i32, owner: *mut c_void) {
        self.reset_if_forked(pid);
        self.entries.retain(|&(ptr, _)| ptr != owner);
    }
}

#[cfg(not(test))]
#[derive(Clone, Copy)]
struct CachedOwner {
    /// `pg_sys::MyProcPid` for which `resowner_release_callback` is installed and `owner`/`stamp`
    /// are valid.
    pid: NonZeroI32,
    /// Most recently stamped or looked-up live `ResourceOwner` pointer.
    owner: NonNull<c_void>,
    /// Monotonically unique stamp for `owner`.
    stamp: NonZeroU64,
}

#[cfg(not(test))]
thread_local! {
    /// Single-entry hot-path cache for the current backend's active `ResourceOwner`.
    ///
    /// Invariant: whenever `Some(cached)` with `cached.pid.get() == pg_sys::MyProcPid`:
    /// 1. `resowner_release_callback` is registered in this process, and
    /// 2. `(cached.owner, cached.stamp)` is a currently live entry in `LIVE_RESOURCE_OWNERS` (not
    ///    yet passed to `RESOURCE_RELEASE_BEFORE_LOCKS`).
    static CACHED_OWNER: Cell<Option<CachedOwner>> = const { Cell::new(None) };
    static LIVE_RESOURCE_OWNERS: RefCell<LiveResourceOwners> = const {
        RefCell::new(LiveResourceOwners::new())
    };
}

#[cfg(not(test))]
static RESOWNER_CALLBACK_PID: AtomicI32 = AtomicI32::new(0);

#[cfg(not(test))]
#[pgrx::pg_guard]
unsafe extern "C-unwind" fn resowner_release_callback(
    phase: pgrx::pg_sys::ResourceReleasePhase::Type,
    _is_commit: bool,
    _is_top_level: bool,
    _arg: *mut c_void,
) {
    if phase == pgrx::pg_sys::ResourceReleasePhase::RESOURCE_RELEASE_BEFORE_LOCKS {
        let releasing = unsafe { pgrx::pg_sys::CurrentResourceOwner }.cast::<c_void>();
        mark_resource_owner_releasing(releasing);
    }
}

#[cfg(not(test))]
fn ensure_resowner_callback_installed(pid: i32) {
    if pid == 0 || RESOWNER_CALLBACK_PID.load(Ordering::Acquire) == pid {
        return;
    }
    unsafe {
        pgrx::pg_sys::RegisterResourceReleaseCallback(
            Some(resowner_release_callback),
            std::ptr::null_mut(),
        );
    }
    RESOWNER_CALLBACK_PID.store(pid, Ordering::Release);
}

#[cfg(not(test))]
fn mark_resource_owner_releasing(releasing_owner: *mut c_void) {
    if releasing_owner.is_null() || !is_main_thread() {
        return;
    }
    let pid = unsafe { pgrx::pg_sys::MyProcPid };
    if let Some(cached) = CACHED_OWNER.get()
        && (cached.pid.get() != pid || cached.owner.as_ptr() == releasing_owner)
    {
        CACHED_OWNER.set(None);
    }
    LIVE_RESOURCE_OWNERS.with(|cell| {
        cell.borrow_mut().remove_owner(pid, releasing_owner);
    });
}

#[cfg(not(test))]
#[cold]
#[inline(never)]
fn stamp_current_slow(pid: i32, owner: *mut c_void) -> ResourceOwnerStamp {
    ensure_resowner_callback_installed(pid);
    let stamp = LIVE_RESOURCE_OWNERS.with(|cell| cell.borrow_mut().stamp_for_owner(pid, owner));
    if let (Some(pid), Some(owner)) = (NonZeroI32::new(pid), NonNull::new(owner)) {
        CACHED_OWNER.set(Some(CachedOwner { pid, owner, stamp }));
    }
    ResourceOwnerStamp(Some(stamp))
}

#[cfg(not(test))]
#[cold]
#[inline(never)]
fn lookup_live_owner_slow(pid: i32, stamp: NonZeroU64) -> Option<*mut c_void> {
    // Drop the `RefCell` borrow before returning so caller `Drop`s or nested
    // `ResourceOwnerStamp::current()` calls can re-enter cleanly.
    let owner = LIVE_RESOURCE_OWNERS.with(|cell| cell.borrow_mut().lookup_owner(pid, stamp))?;
    if let (Some(pid), Some(owner_ptr)) = (NonZeroI32::new(pid), NonNull::new(owner)) {
        CACHED_OWNER.set(Some(CachedOwner {
            pid,
            owner: owner_ptr,
            stamp,
        }));
    }
    Some(owner)
}

#[cfg(not(test))]
struct ScopedCurrentResourceOwner {
    prev: pgrx::pg_sys::ResourceOwner,
}

#[cfg(not(test))]
impl ScopedCurrentResourceOwner {
    unsafe fn enter(target: pgrx::pg_sys::ResourceOwner) -> Self {
        let prev = unsafe { pgrx::pg_sys::CurrentResourceOwner };
        unsafe {
            pgrx::pg_sys::CurrentResourceOwner = target;
        }
        Self { prev }
    }
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because `CurrentResourceOwner` must be
// restored to the caller's owner even if an inner cleanup call unwinds.
#[cfg(not(test))]
impl Drop for ScopedCurrentResourceOwner {
    fn drop(&mut self) {
        unsafe {
            pgrx::pg_sys::CurrentResourceOwner = self.prev;
        }
    }
}

/// Monotonically unique token identifying the `pg_sys::ResourceOwner` that was current when a
/// PostgreSQL C resource (`Relation`, `Buffer`, `TupleTableSlot`, `Tuplesortstate`, `LOCKTAG`,
/// child `MemoryContext`, or `TIDBitmap` iterator) was acquired on this backend.
///
/// See module documentation for how this guards deferred `MemoryContextDelete` destructors once
/// `ResourceOwnerRelease` has begun (`RESOURCE_RELEASE_BEFORE_LOCKS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceOwnerStamp(Option<NonZeroU64>);

#[derive(Clone, Copy)]
enum StampCleanupContext {
    TransactionOnly,
    TransactionOrRecovery,
}

impl ResourceOwnerStamp {
    /// Stamp that opts out of `ResourceOwner` tracking (gating cleanup only on
    /// `IsTransactionState()`).
    ///
    /// Use only for inert/unowned placeholder values whose `Drop` will not release a
    /// `ResourceOwner`-tracked C resource (such as `InvalidBuffer` or `need_close == false`).
    #[inline]
    pub const fn untracked() -> Self {
        Self(None)
    }

    /// Record the backend's current `pg_sys::CurrentResourceOwner` on the Postgres main thread.
    #[inline]
    pub fn current() -> Self {
        if !is_main_thread() {
            return Self::untracked();
        }
        #[cfg(not(test))]
        {
            let owner = unsafe { pgrx::pg_sys::CurrentResourceOwner }.cast::<c_void>();
            if owner.is_null() {
                return Self::untracked();
            }
            let pid = unsafe { pgrx::pg_sys::MyProcPid };
            if let Some(cached) = CACHED_OWNER.get()
                && cached.pid.get() == pid
                && cached.owner.as_ptr() == owner
            {
                return Self(Some(cached.stamp));
            }
            stamp_current_slow(pid, owner)
        }
        #[cfg(test)]
        {
            Self::untracked()
        }
    }

    /// Run `f` if the originating `ResourceOwner` has not started `ResourceOwnerRelease` and the
    /// backend is currently in a valid transaction state (`IsTransactionState()`).
    #[inline]
    pub fn run_if_live(&self, f: impl FnOnce()) {
        self.run_inner(StampCleanupContext::TransactionOnly, f);
    }

    /// Like [`Self::run_if_live`], additionally permitting WAL recovery (`pg_sys::InRecovery`) for
    /// buffer pins acquired via `XLogReadBufferExtended` under `AuxProcessResourceOwner`.
    #[inline]
    pub fn run_if_live_or_recovery(&self, f: impl FnOnce()) {
        self.run_inner(StampCleanupContext::TransactionOrRecovery, f);
    }

    /// Whether the originating `ResourceOwner` is still live under an active transaction.
    pub fn is_live(&self) -> bool {
        let mut live = false;
        self.run_if_live(|| {
            live = true;
        });
        live
    }

    #[inline]
    fn run_inner(&self, ctx: StampCleanupContext, f: impl FnOnce()) {
        if !is_main_thread()
            || std::thread::panicking()
            || crate::postgres::utils::proc_exit_in_progress()
        {
            return;
        }
        #[cfg(not(test))]
        {
            let state_ok = unsafe {
                crate::postgres::utils::IsTransactionState()
                    || (matches!(ctx, StampCleanupContext::TransactionOrRecovery)
                        && pgrx::pg_sys::InRecovery)
            };
            if !state_ok {
                return;
            }
            let Some(stamp) = self.0 else {
                f();
                return;
            };
            let pid = unsafe { pgrx::pg_sys::MyProcPid };
            let cur_owner = unsafe { pgrx::pg_sys::CurrentResourceOwner }.cast::<c_void>();
            if let Some(cached) = CACHED_OWNER.get()
                && cached.pid.get() == pid
                && cached.stamp == stamp
                && cached.owner.as_ptr() == cur_owner
            {
                f();
                return;
            }
            let Some(owner) = lookup_live_owner_slow(pid, stamp) else {
                return;
            };
            if owner == cur_owner {
                f();
            } else {
                let _owner_guard = unsafe { ScopedCurrentResourceOwner::enter(owner.cast()) };
                f();
            }
        }
        #[cfg(test)]
        {
            let _ = ctx;
            f();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_resource_owners_stamps_invalidates_and_resets_on_fork() {
        let mut owners = LiveResourceOwners::new();
        let owner_a = 0x1000usize as *mut c_void;
        let owner_b = 0x2000usize as *mut c_void;

        let stamp_a1 = owners.stamp_for_owner(101, owner_a);
        let stamp_a2 = owners.stamp_for_owner(101, owner_a);
        let stamp_b = owners.stamp_for_owner(101, owner_b);
        assert_eq!(stamp_a1, stamp_a2);
        assert_ne!(stamp_a1, stamp_b);
        assert_eq!(owners.lookup_owner(101, stamp_a1), Some(owner_a));
        assert_eq!(owners.lookup_owner(101, stamp_b), Some(owner_b));

        // Releasing owner_a invalidates its stamp and re-stamping the same address mints a fresh
        // monotonic stamp (preventing ABA reuse across `ResourceOwnerDelete` + `palloc`).
        owners.remove_owner(101, owner_a);
        assert_eq!(owners.lookup_owner(101, stamp_a1), None);
        assert_eq!(owners.lookup_owner(101, stamp_b), Some(owner_b));
        let stamp_a_reused = owners.stamp_for_owner(101, owner_a);
        assert_ne!(stamp_a_reused, stamp_a1);
        assert_eq!(owners.lookup_owner(101, stamp_a_reused), Some(owner_a));

        // Forking to a new PID drops parent entries while keeping stamps monotonic.
        assert_eq!(owners.lookup_owner(202, stamp_b), None);
        let stamp_child = owners.stamp_for_owner(202, owner_b);
        assert!(stamp_child > stamp_a_reused);
        assert_eq!(owners.lookup_owner(202, stamp_child), Some(owner_b));
    }
}
