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
use pgrx::{direct_function_call, pg_sys, IntoDatum};
use std::marker::PhantomData;
use std::ptr::addr_of_mut;

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Spinlock(pg_sys::slock_t);

impl Default for Spinlock {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl Spinlock {
    #[inline(always)]
    pub fn new() -> Self {
        let mut lock = Self(pg_sys::slock_t::default());
        lock.init();
        lock
    }

    #[inline(always)]
    pub fn init(&mut self) {
        unsafe {
            // SAFETY:  `unsafe` due to normal FFI
            pg_sys::SpinLockInit(addr_of_mut!(self.0));
        }
    }

    #[inline(always)]
    pub fn acquire(&mut self) -> AcquiredSpinLock {
        AcquiredSpinLock::new(self)
    }
}

#[must_use]
#[repr(transparent)]
pub struct AcquiredSpinLock(*mut pg_sys::slock_t);

impl AcquiredSpinLock {
    fn new(lock: &mut Spinlock) -> Self {
        unsafe {
            let addr = addr_of_mut!(lock.0);
            pg_sys::SpinLockAcquire(addr);
            Self(addr)
        }
    }
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here. Spinlocks must be released
// even during panic unwinding to avoid deadlocks. SpinLockRelease is just an atomic
// operation that cannot raise PostgreSQL errors, so it's safe to call during unwinding.
impl Drop for AcquiredSpinLock {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            pg_sys::SpinLockRelease(self.0);
        }
    }
}

// ---------------------------------------------------------------------------
// LWLock — RAII wrapper around PostgreSQL lightweight locks
// ---------------------------------------------------------------------------

/// A raw pointer to a PostgreSQL `LWLock`.
///
/// Unlike pgrx's `PgLwLock<T>`, this wrapper does not tie the lock to protected
/// data.  This makes it suitable for cases where the lock guards a raw
/// shared-memory region (e.g. a flat slot array) rather than a single Rust
/// value.
///
/// Acquire a guard via [`LWLock::acquire_shared`] or [`LWLock::acquire_exclusive`].
/// The lock is released automatically when the guard is dropped.
///
/// # Unwind safety
///
/// During a PostgreSQL `elog(ERROR)` unwind, `InterruptHoldoffCount` is reset
/// to zero before Rust unwinding begins.  Calling `LWLockRelease` in that
/// state would trigger an assertion failure.  The guard's `Drop`
/// implementation checks `InterruptHoldoffCount` and skips the release when
/// it is zero — PostgreSQL releases all held LWLocks on (sub)transaction
/// abort anyway.
#[derive(Copy, Clone)]
pub struct LWLock {
    lock: *mut pg_sys::LWLock,
}

impl LWLock {
    /// Wrap an existing PostgreSQL `LWLock` pointer.
    ///
    /// # Safety
    ///
    /// `lock` must point to a valid, initialized `LWLock` that outlives all
    /// guards created from this wrapper.
    pub unsafe fn from_raw(lock: *mut pg_sys::LWLock) -> Self {
        Self { lock }
    }

    /// Acquire the lock in shared (read) mode.
    #[inline]
    pub fn acquire_shared(&self) -> LWLockGuard<'_> {
        unsafe {
            pg_sys::LWLockAcquire(self.lock, pg_sys::LWLockMode::LW_SHARED);
        }
        LWLockGuard {
            lock: self.lock,
            _marker: PhantomData,
        }
    }

    /// Acquire the lock in exclusive (write) mode.
    #[inline]
    pub fn acquire_exclusive(&self) -> LWLockGuard<'_> {
        unsafe {
            pg_sys::LWLockAcquire(self.lock, pg_sys::LWLockMode::LW_EXCLUSIVE);
        }
        LWLockGuard {
            lock: self.lock,
            _marker: PhantomData,
        }
    }
}

/// RAII guard that releases an `LWLock` on drop.
///
/// See [`LWLock`] for unwind-safety details.
#[must_use]
pub struct LWLockGuard<'a> {
    lock: *mut pg_sys::LWLock,
    _marker: PhantomData<&'a LWLock>,
}

impl Drop for LWLockGuard<'_> {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: During elog(ERROR) unwinding, PostgreSQL resets
        // InterruptHoldoffCount to zero and will release all LWLocks itself
        // at (sub)transaction abort.  Calling LWLockRelease with
        // InterruptHoldoffCount == 0 would hit an assertion failure, so we
        // skip the release in that case.
        unsafe {
            if pg_sys::InterruptHoldoffCount > 0 {
                pg_sys::LWLockRelease(self.lock);
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum AdvisoryLockLevel {
    Session,
    Transaction,
}

#[derive(Clone, Debug)]
pub struct AdvisoryLock {
    level: AdvisoryLockLevel,
    key: i64,
}

impl AdvisoryLock {
    /// Blocking exclusive session-level lock.
    pub fn lock_session(key: i64) -> Self {
        unsafe {
            direct_function_call::<()>(pg_sys::pg_advisory_lock_int8, &[key.into_datum()]);
        }
        Self {
            level: AdvisoryLockLevel::Session,
            key,
        }
    }

    /// Non-blocking (conditional) session-level lock. Returns `None` if lock is already held.
    pub fn conditional_lock_session(key: i64) -> Option<Self> {
        let acquired = unsafe {
            direct_function_call::<bool>(pg_sys::pg_try_advisory_lock_int8, &[key.into_datum()])
        }
        .unwrap_or(false);
        if acquired {
            Some(Self {
                level: AdvisoryLockLevel::Session,
                key,
            })
        } else {
            None
        }
    }
    pub fn new_transaction(key: i64) -> Option<Self> {
        let acquired = unsafe {
            direct_function_call::<bool>(
                pg_sys::pg_try_advisory_xact_lock_int8,
                &[key.into_datum()],
            )
        }
        .unwrap_or(false);
        if acquired {
            Some(Self {
                level: AdvisoryLockLevel::Transaction,
                key,
            })
        } else {
            None
        }
    }
}

crate::impl_safe_drop!(AdvisoryLock, |self| {
    match self.level {
        AdvisoryLockLevel::Session => {
            unsafe {
                direct_function_call::<bool>(
                    pg_sys::pg_advisory_unlock_int8,
                    &[self.key.into_datum()],
                )
            };
        }
        AdvisoryLockLevel::Transaction => {
            // do nothing, the lock is released by the end of the transaction
        }
    }
});
