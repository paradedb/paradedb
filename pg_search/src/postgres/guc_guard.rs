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

//! RAII guard for safely managing PostgreSQL GUC (Grand Unified Configuration) values
//!
//! This module provides type-safe RAII wrappers for temporarily modifying PostgreSQL
//! configuration variables. The guards automatically restore the original value when
//! they go out of scope, ensuring configuration changes don't leak beyond their
//! intended scope.

/// RAII guard for boolean GUC variables
///
/// Temporarily sets a boolean GUC to a new value and automatically restores
/// the original value when the guard is dropped.
///
/// # Safety
///
/// The caller must ensure that:
/// - The pointer points to a valid, mutable boolean GUC variable
/// - The GUC variable remains valid for the lifetime of the guard
/// - No other code modifies the GUC while this guard is active
pub struct BoolGucGuard {
    guc_ptr: *mut bool,
    original_value: bool,
}

impl BoolGucGuard {
    /// Create a new guard that sets a boolean GUC to the specified value
    ///
    /// # Safety
    ///
    /// See struct-level safety documentation
    pub unsafe fn new(guc_ptr: *mut bool, new_value: bool) -> Self {
        let original_value = *guc_ptr;
        *guc_ptr = new_value;
        Self {
            guc_ptr,
            original_value,
        }
    }
}

impl Drop for BoolGucGuard {
    fn drop(&mut self) {
        unsafe {
            *self.guc_ptr = self.original_value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_guc_guard_restores_value() {
        let mut test_var = true;

        {
            let _guard = unsafe { BoolGucGuard::new(&mut test_var as *mut bool, false) };
            assert!(!test_var, "Guard should set value to false");
        }

        assert!(
            test_var,
            "Original value should be restored after guard is dropped"
        );
    }

    #[test]
    fn test_bool_guc_guard_nested() {
        let mut test_var = true;

        {
            let _guard1 = unsafe { BoolGucGuard::new(&mut test_var as *mut bool, false) };
            assert!(!test_var);

            {
                let _guard2 = unsafe { BoolGucGuard::new(&mut test_var as *mut bool, true) };
                assert!(test_var);
            }

            assert!(!test_var, "Should restore to guard1's value");
        }

        assert!(test_var, "Should restore to original value");
    }
}
