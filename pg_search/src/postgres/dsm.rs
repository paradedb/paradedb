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

//! Safe(r) wrappers around PostgreSQL dynamic shared memory (DSM) segments.
//!
//! These are thin RAII wrappers that ensure `dsm_detach` is called when the
//! segment is dropped, and expose typed accessors instead of raw `*mut u8`.

use pgrx::pg_sys;

/// A DSM segment that this backend has created or attached to.
///
/// Dropping the `DsmSegment` calls `dsm_detach`, releasing this backend's
/// mapping.  If the segment is globally pinned, it survives until explicitly
/// unpinned; otherwise PostgreSQL frees it when the last backend detaches.
pub struct DsmSegment {
    seg: *mut pg_sys::dsm_segment,
}

impl DsmSegment {
    /// Create a new DSM segment of the given size.
    ///
    /// Returns `None` if the maximum number of DSM segments has been reached.
    pub fn create(size: usize) -> Option<Self> {
        let seg = unsafe { pg_sys::dsm_create(size, pg_sys::DSM_CREATE_NULL_IF_MAXSEGMENTS as _) };
        if seg.is_null() {
            return None;
        }
        Some(Self { seg })
    }

    /// Attach to an existing DSM segment by handle.
    ///
    /// Returns `None` if the segment no longer exists.
    pub fn attach(handle: pg_sys::dsm_handle) -> Option<Self> {
        let seg = unsafe { pg_sys::dsm_attach(handle) };
        if seg.is_null() {
            return None;
        }
        Some(Self { seg })
    }

    /// Find an existing mapping for a handle in this backend, without
    /// creating a new attachment.
    ///
    /// Returns `None` if this backend has no mapping for the handle.
    /// The returned `DsmSegment` will NOT call `dsm_detach` on drop —
    /// use this only for read access to a segment you know is pinned.
    pub fn find_mapping(handle: pg_sys::dsm_handle) -> Option<DsmSegmentRef> {
        let seg = unsafe { pg_sys::dsm_find_mapping(handle) };
        if seg.is_null() {
            return None;
        }
        Some(DsmSegmentRef { seg })
    }

    /// The DSM handle, used to share segment identity across backends.
    pub fn handle(&self) -> pg_sys::dsm_handle {
        unsafe { pg_sys::dsm_segment_handle(self.seg) }
    }

    /// Raw pointer to the segment's mapped memory.
    pub fn address(&self) -> *mut u8 {
        unsafe { pg_sys::dsm_segment_address(self.seg) as *mut u8 }
    }

    /// The segment contents as a mutable byte slice.
    ///
    /// # Safety
    /// `len` must not exceed the segment's allocated size.
    /// Caller must ensure no other backend is concurrently writing.
    pub unsafe fn as_mut_slice(&self, len: usize) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.address(), len)
    }

    /// Pin the segment globally so it outlives the creating backend,
    /// and pin the mapping so it persists across transactions.
    pub fn pin(&self) {
        unsafe {
            pg_sys::dsm_pin_segment(self.seg);
            pg_sys::dsm_pin_mapping(self.seg);
        }
    }

    /// Pin only the per-backend mapping (persists across transactions).
    pub fn pin_mapping(&self) {
        unsafe {
            pg_sys::dsm_pin_mapping(self.seg);
        }
    }

    /// Globally unpin a segment by handle.  After this, the segment will be
    /// freed once all backends have detached.
    ///
    /// # Safety
    /// The handle must refer to a segment that was previously pinned.
    pub unsafe fn unpin(handle: pg_sys::dsm_handle) {
        pg_sys::dsm_unpin_segment(handle);
    }
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here. dsm_detach is a
// lightweight operation that does not raise PostgreSQL errors, so calling it
// during unwind is safe.  Leaking the mapping would be worse since pinned DSM
// segments would never be reclaimed.
impl Drop for DsmSegment {
    fn drop(&mut self) {
        unsafe {
            pg_sys::dsm_detach(self.seg);
        }
    }
}

/// A non-owning reference to a DSM segment mapping.
///
/// Obtained via [`DsmSegment::find_mapping`].  Does NOT call `dsm_detach`
/// on drop — the caller is responsible for the segment's lifecycle.
pub struct DsmSegmentRef {
    seg: *mut pg_sys::dsm_segment,
}

impl DsmSegmentRef {
    /// Raw pointer to the segment's mapped memory.
    pub fn address(&self) -> *const u8 {
        unsafe { pg_sys::dsm_segment_address(self.seg) as *const u8 }
    }
}
