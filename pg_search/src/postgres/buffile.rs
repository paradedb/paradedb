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

//! Spills DataFusion's sorts/aggregates/joins to Postgres's `BufFile` temp-file
//! mechanism, instead of DataFusion's default bare-OS-tempfile `DiskManager`.
//!
//! Routing through `BufFile` (rather than `tempfile`/`NamedTempFile`, DataFusion's
//! default) means spill files are counted against `temp_file_limit`, land in the
//! configured `temp_tablespaces`, and are guaranteed to be cleaned up by Postgres's
//! resource-owner machinery even if the query is cancelled or the backend crashes —
//! none of which DataFusion's OS-tempdir path provides.
//!
//! Shared BufFile FFI Module
use pgrx::pg_sys;
use std::os::raw::c_int;
pub unsafe fn buffile_tell(file: *mut pg_sys::BufFile) -> (c_int, pg_sys::off_t) {
    let mut fileno: c_int = 0;
    let mut offset: pg_sys::off_t = 0;
    pg_sys::BufFileTell(file, &mut fileno, &mut offset);
    (fileno, offset)
}

// The following wrap Postgres APIs whose signatures differ across supported versions.
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

/// Creates a `BufFile` scoped to the current transaction's resource owner and memory
/// context, so it outlives the per-tuple/per-batch contexts DataFusion operators may
/// be running under, but is still guaranteed to be cleaned up (temp file removed, VFD
/// released) when the transaction ends.
///
/// `BufFileCreateTemp` never returns NULL; on failure it raises via `palloc`/`ereport`,
/// which unwinds past this function rather than returning, so there's no error case to
/// report here. If it raises after `CurrentResourceOwner`/`CurrentMemoryContext` are
/// swapped in but before they're restored, the transaction abort path resets both, so
/// the swap doesn't need its own unwind handling.
pub unsafe fn create_transaction_scoped_buffile() -> *mut pg_sys::BufFile {
    unsafe {
        let saved_owner = pg_sys::CurrentResourceOwner;
        let saved_cxt = pg_sys::CurrentMemoryContext;
        pg_sys::CurrentResourceOwner = pg_sys::CurTransactionResourceOwner;
        pg_sys::CurrentMemoryContext = pg_sys::CurTransactionContext;
        let file = pg_sys::BufFileCreateTemp(false);
        pg_sys::CurrentResourceOwner = saved_owner;
        pg_sys::CurrentMemoryContext = saved_cxt;
        file
    }
}
