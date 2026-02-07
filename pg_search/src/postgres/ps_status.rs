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
use std::ffi::CStr;
use std::os::raw::c_char;

pub const INDEXING: &CStr = c"indexing";
pub const MERGING: &CStr = c"merging";
pub const COMMITTING: &CStr = c"committing";
pub const GARBAGE_COLLECTING: &CStr = c"gc-ing";
pub const FINALIZING: &CStr = c"finalizing";

pub unsafe fn set_ps_display_suffix(suffix: *const c_char) {
    #[cfg(feature = "pg15")]
    pg_sys::set_ps_display(suffix);

    #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
    pg_sys::set_ps_display_suffix(suffix);
}

pub unsafe fn set_ps_display_remove_suffix() {
    #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
    pg_sys::set_ps_display_remove_suffix();
}
