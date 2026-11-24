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

//! Local extension assertions when the Antithesis SDK is disabled.

pub use crate::dst_observe as observe;
pub use crate::{
    dst_assert_always as assert_always, dst_assert_always as assert_always_or_unreachable,
    dst_assert_unreachable as assert_unreachable,
};

/// Run a read-only observation in debug builds. Release builds still type-check the closure.
#[inline]
#[doc(hidden)]
pub fn run_observation<F: Fn()>(check: F) {
    if cfg!(debug_assertions) {
        check();
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! dst_observe {
    ($check:expr $(,)?) => {
        $crate::dst::run_observation($check)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! dst_assert_always {
    ($condition:expr, $message:literal) => {
        ::core::debug_assert!($condition, "{}", $message)
    };
    ($condition:expr, $message:literal, $details:expr) => {
        ::core::debug_assert!($condition, "{}: {}", $message, $details)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! dst_assert_unreachable {
    ($message:literal $(, $details:expr)?) => {
        $crate::dst_assert_always!(false, $message $(, $details)?)
    };
}
