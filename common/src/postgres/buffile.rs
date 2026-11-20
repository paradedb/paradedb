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
#![allow(clippy::missing_safety_doc, unsafe_op_in_unsafe_fn)]

//! Wraps Postgres's `BufFile*` C API. Some of these signatures differ across supported PG
//! versions; the rest are thin safety wrappers over the same underlying calls.

use crate::postgres::utils::ResourceOwnerStamp;
use pgrx::PgMemoryContexts;
use pgrx::pg_sys;
use std::io;
use std::os::raw::c_int;
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

pub trait TempFileHandle: Copy {
    unsafe fn close(self);
}

impl TempFileHandle for *mut pg_sys::BufFile {
    unsafe fn close(self) {
        unsafe { pg_sys::BufFileClose(self) };
    }
}

impl TempFileHandle for pg_sys::File {
    unsafe fn close(self) {
        unsafe { pg_sys::FileClose(self) };
    }
}

struct DeferredTempFileClose<H: TempFileHandle + 'static> {
    handle: H,
    poisoned: AtomicBool,
    owner: ResourceOwnerStamp,
}

crate::impl_safe_drop!([H: TempFileHandle + 'static] DeferredTempFileClose<H>, |self| {
    if !self.poisoned.load(Ordering::Relaxed) {
        let handle = self.handle;
        self.owner.run_if_live(|| unsafe {
            handle.close();
        });
    }
});

/// Owns a PostgreSQL temporary file and prevents reuse or close after failed I/O or release.
///
/// Captures [`ResourceOwnerStamp::current`] on construction so `Drop` skips `BufFileClose` /
/// `FileClose` once PostgreSQL has released the originating `ResourceOwner` (e.g. subtransaction
/// abort or `PortalDrop`), and skips flushing a poisoned handle whose prior I/O raised.
///
/// The inner handle and destructor are wrapped in [`crate::postgres::customscan::mpp::MainThread`]
/// so the file can only be accessed inside [`crate::postgres::customscan::mpp::run_on_main_thread`]
/// and is always closed on the creating backend's main thread.
pub struct TempFileOwner<H: TempFileHandle + 'static> {
    inner: crate::postgres::customscan::mpp::MainThread<DeferredTempFileClose<H>>,
}

impl<H: TempFileHandle + 'static> TempFileOwner<H> {
    pub fn new(handle: H) -> Self {
        Self {
            inner: crate::postgres::customscan::mpp::MainThread::new(DeferredTempFileClose {
                handle,
                poisoned: AtomicBool::new(false),
                owner: ResourceOwnerStamp::current(),
            }),
        }
    }

    pub fn with<T>(&self, action: impl FnOnce(H) -> io::Result<T>) -> io::Result<T> {
        crate::postgres::customscan::mpp::try_run_on_main_thread_with(
            |token| {
                let inner = self.inner.get(token);
                if !inner.owner.is_live() || inner.poisoned.swap(true, Ordering::Relaxed) {
                    return Err(io::Error::other(
                        "PostgreSQL temporary file is no longer usable",
                    ));
                }
                let result = action(inner.handle);
                inner.poisoned.store(result.is_err(), Ordering::Relaxed);
                result
            },
            io::Error::other,
        )
    }
}
