// Copyright (c) 2023-2024 Retake, Inc.
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

/// Debug logging configuration for the custom scan system
/// Set to true to enable debug logging, false to disable
pub const ENABLE_DEBUG_LOGGING: bool = false;

/// Wrapper macro for debug logging that can be easily enabled/disabled
/// Usage: debug_log!("Message with {} formatting", value);
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        if $crate::postgres::customscan::debug::ENABLE_DEBUG_LOGGING {
            pgrx::warning!($($arg)*);
        }
    };
}

/// Convenience function to check if debug logging is enabled
/// Can be used for expensive operations that should only run when debugging
pub fn is_debug_enabled() -> bool {
    ENABLE_DEBUG_LOGGING
}

/// Log a debug message with a specific category prefix
/// Categories help organize debug output: HOOK, PDBSCAN, EXTRACT_QUALS, etc.
#[macro_export]
macro_rules! debug_log_category {
    ($category:expr, $($arg:tt)*) => {
        if $crate::postgres::customscan::debug::ENABLE_DEBUG_LOGGING {
            pgrx::warning!("{} {}", $category, format!($($arg)*));
        }
    };
}

/// Log debug information about function entry/exit
#[macro_export]
macro_rules! debug_trace {
    (enter $func:expr) => {
        if $crate::postgres::customscan::debug::ENABLE_DEBUG_LOGGING {
            pgrx::warning!("🔍 [TRACE] Entering: {}", $func);
        }
    };
    (exit $func:expr) => {
        if $crate::postgres::customscan::debug::ENABLE_DEBUG_LOGGING {
            pgrx::warning!("🔍 [TRACE] Exiting: {}", $func);
        }
    };
    (exit $func:expr, $result:expr) => {
        if $crate::postgres::customscan::debug::ENABLE_DEBUG_LOGGING {
            pgrx::warning!("🔍 [TRACE] Exiting: {} -> {:?}", $func, $result);
        }
    };
}
