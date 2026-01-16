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
    pub fn acquire(&mut self) -> impl Drop {
        AcquiredSpinLock::new(self)
    }
}

#[repr(transparent)]
struct AcquiredSpinLock(*mut pg_sys::slock_t);

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
    pub fn new_session(key: i64) -> Option<Self> {
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
