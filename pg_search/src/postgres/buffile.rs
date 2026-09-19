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

//! Wraps Postgres's `BufFile*` C API. Some of these signatures differ across supported PG
//! versions; the rest are thin safety wrappers over the same underlying calls.

use pgrx::PgMemoryContexts;
use pgrx::pg_sys;
use std::ffi::c_void;
use std::os::raw::c_int;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Returns `file`'s current `(fileno, offset)` position.
pub unsafe fn buffile_tell(file: *mut pg_sys::BufFile) -> (c_int, pg_sys::off_t) {
    let mut fileno: c_int = 0;
    let mut offset: pg_sys::off_t = 0;
    pg_sys::BufFileTell(file, &mut fileno, &mut offset);
    (fileno, offset)
}

/// Write `data` to `file`. (PG15's `BufFileWrite` takes `*mut`; PG16+ takes `*const`.)
pub unsafe fn buffile_write(file: *mut pg_sys::BufFile, data: &[u8]) {
    #[cfg(feature = "pg15")]
    pg_sys::BufFileWrite(file, data.as_ptr() as *mut std::ffi::c_void, data.len());
    #[cfg(not(feature = "pg15"))]
    pg_sys::BufFileWrite(file, data.as_ptr().cast::<std::ffi::c_void>(), data.len());
}

/// Read exactly `buf.len()` bytes into `buf`. (`BufFileReadExact` was added in PG16;
/// emulated on PG15 via `BufFileRead` plus a short-read check.)
pub unsafe fn buffile_read_exact(file: *mut pg_sys::BufFile, buf: &mut [u8]) {
    #[cfg(feature = "pg15")]
    {
        let n = unsafe { pg_sys::BufFileRead(file, buf.as_mut_ptr().cast(), buf.len()) };
        assert_eq!(n, buf.len(), "short read from spilled key file");
    }
    #[cfg(not(feature = "pg15"))]
    {
        unsafe { pg_sys::BufFileReadExact(file, buf.as_mut_ptr().cast(), buf.len()) };
    }
}

/// Reads up to `buf.len()` bytes from `file`'s current position into `buf`, returning
/// the number of bytes read (`0` at EOF).
pub unsafe fn buffile_read(file: *mut pg_sys::BufFile, buf: &mut [u8]) -> usize {
    unsafe { pg_sys::BufFileRead(file, buf.as_mut_ptr().cast(), buf.len()) }
}

/// Seek `file` to `(fileno, offset)` relative to `whence`.
/// `BufFileSeek` reports failure via its return code instead of raising error,
/// so it needs to be wrapped in `Result`.
pub unsafe fn buffile_seek(
    file: *mut pg_sys::BufFile,
    fileno: c_int,
    offset: pg_sys::off_t,
    whence: c_int,
) -> Result<(), &'static str> {
    let ret = unsafe { pg_sys::BufFileSeek(file, fileno, offset, whence) };
    if ret != 0 {
        return Err("BufFileSeek failed");
    }
    Ok(())
}

/// Creates a temp `BufFile` under the current resource owner, with its struct allocated
/// in the transaction context. The owner stays the current one (the portal's, during
/// execution) so the file follows the portal, as Postgres's own executor temp files do;
/// a cursor opened inside a savepoint keeps its files across `RELEASE SAVEPOINT`. The
/// struct goes to the transaction context because DataFusion can call this under a
/// per-batch context that is reset long before the file is done.
///
/// `BufFileCreateTemp` raises rather than returning NULL. If it raises with the context
/// swapped in, the abort path resets `CurrentMemoryContext`, so no unwind handling.
pub unsafe fn create_temp_buffile() -> *mut pg_sys::BufFile {
    PgMemoryContexts::CurTransactionContext
        .switch_to(|_| unsafe { pg_sys::BufFileCreateTemp(false) })
}

/// Tracks whether the resource owner that some `BufFile`s were created under has released
/// its files, so a Rust `Drop` knows when it must not call `BufFileClose`.
///
/// Postgres closes and deletes temp files itself when the owner is released, and that
/// can run before the Rust holder drops: a subtransaction abort releases the owner
/// before it resets the SPI context that frees a scan state, so a later `BufFileClose`
/// would run on a freed VFD. `BufFileClose` also flushes, so it must not run while a
/// Postgres error is unwinding through Rust either: the flush would raise again, and a
/// panic during unwinding aborts the backend. Same protocol as `sequentialscan::keyset`.
pub struct BufFileReleaseGuard {
    owner: pg_sys::ResourceOwner,
    released: AtomicBool,
}

// SAFETY: `owner` is only ever compared, never dereferenced, and registration, the
// callback, and `Drop` all run on the backend thread that created the guard.
unsafe impl Send for BufFileReleaseGuard {}
unsafe impl Sync for BufFileReleaseGuard {}

impl BufFileReleaseGuard {
    /// Registers a release callback for the current resource owner. The `Arc` pins the
    /// callback argument's address; the callback is unregistered when the last clone drops.
    pub fn register() -> Arc<Self> {
        let guard = Arc::new(Self {
            owner: unsafe { pg_sys::CurrentResourceOwner },
            released: AtomicBool::new(false),
        });
        unsafe {
            pg_sys::RegisterResourceReleaseCallback(
                Some(Self::on_release),
                Arc::as_ptr(&guard).cast_mut().cast::<c_void>(),
            );
        }
        guard
    }

    pub fn owner(&self) -> pg_sys::ResourceOwner {
        self.owner
    }

    /// Whether Rust may still close a file created under this owner: false once Postgres
    /// has released the owner's files, while a panic is unwinding, and outside an
    /// in-progress transaction. Abort processing drops holders before the owner is
    /// released, and a close there flushes into whatever made the query fail.
    pub fn may_close(&self) -> bool {
        !self.released.load(Ordering::Relaxed)
            && !std::thread::panicking()
            && unsafe { pg_sys::IsTransactionState() }
    }

    #[pgrx::pg_guard]
    unsafe extern "C-unwind" fn on_release(
        phase: pg_sys::ResourceReleasePhase::Type,
        _is_commit: bool,
        _is_top_level: bool,
        arg: *mut c_void,
    ) {
        let guard = unsafe { &*arg.cast::<BufFileReleaseGuard>() };
        if phase == pg_sys::ResourceReleasePhase::RESOURCE_RELEASE_AFTER_LOCKS
            && unsafe { pg_sys::CurrentResourceOwner } == guard.owner
        {
            guard.released.store(true, Ordering::Relaxed);
        }
    }
}

impl Drop for BufFileReleaseGuard {
    fn drop(&mut self) {
        unsafe {
            pg_sys::UnregisterResourceReleaseCallback(
                Some(Self::on_release),
                std::ptr::from_mut(self).cast::<c_void>(),
            );
        }
    }
}
