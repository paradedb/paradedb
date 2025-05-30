// Copyright (c) 2023-2025 ParadeDB, Inc.
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
use pgrx::pg_sys;
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

impl Drop for AcquiredSpinLock {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            pg_sys::SpinLockRelease(self.0);
        }
    }
}
