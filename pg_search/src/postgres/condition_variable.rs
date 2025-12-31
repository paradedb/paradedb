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

use pgrx::pg_sys;
use std::ptr::addr_of_mut;

/// A thin wrapper around PostgreSQL's `ConditionVariable`.
///
/// Condition variables allow processes to sleep until signaled by another process,
/// avoiding busy-waiting when coordinating between parallel workers.
#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct ConditionVariable(pg_sys::ConditionVariable);

impl Default for ConditionVariable {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionVariable {
    /// Creates a new, initialized condition variable.
    pub fn new() -> Self {
        let mut cv = Self(pg_sys::ConditionVariable::default());
        cv.init();
        cv
    }

    /// Initializes the condition variable.
    pub fn init(&mut self) {
        unsafe {
            pg_sys::ConditionVariableInit(addr_of_mut!(self.0));
        }
    }

    /// Wakes up all processes waiting on this condition variable.
    pub fn broadcast(&mut self) {
        unsafe {
            pg_sys::ConditionVariableBroadcast(addr_of_mut!(self.0));
        }
    }

    /// Prepares the current process to sleep on this condition variable.
    /// Must be called before `sleep()`.
    pub fn prepare_to_sleep(&mut self) {
        unsafe {
            pg_sys::ConditionVariablePrepareToSleep(addr_of_mut!(self.0));
        }
    }

    /// Sleeps on the condition variable until signaled or interrupted.
    /// `prepare_to_sleep()` must be called first.
    pub fn sleep(&mut self) {
        unsafe {
            pg_sys::ConditionVariableSleep(addr_of_mut!(self.0), pg_sys::PG_WAIT_EXTENSION);
        }
    }

    /// Cancels the current sleep operation and removes the process from the wait queue.
    /// Must be called after `prepare_to_sleep()` when the process decides not to sleep
    /// (e.g., when the awaited condition is already satisfied).
    pub fn cancel_sleep() {
        unsafe {
            pg_sys::ConditionVariableCancelSleep();
        }
    }
}
