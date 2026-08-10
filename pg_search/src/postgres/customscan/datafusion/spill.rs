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
//! This implements DataFusion's [`TempFileFactory`] / [`SpillFile`] / [`SpillWriter`]
//! trait family (added upstream to support pluggable spill backends such as
//! `ObjectStore`-backed spilling; see `datafusion-examples/examples/data_io/
//! object_store_spill.rs`) rather than a custom `DiskManager`, since `BufFile` has no
//! OS-visible path — the same shape problem that motivated those traits upstream.
//!
//! # BufFile FFI
//!
//! `BufFile`'s API is synchronous and not thread-safe to share across threads; a
//! `*mut pg_sys::BufFile` created by one backend/parallel-worker must only be touched
//! by that same backend. Within pg_search's single-threaded-per-backend Tokio runtime
//! (`current_thread` or a dedicated worker's own runtime — see `datafusion/memory.rs`)
//! that invariant already holds, so this module trusts it the same way
//! `api::operator::keyset` does for its own `BufFile` usage.
//!
//! The read side of [`SpillFile::read_stream`] is formally async (DataFusion streams
//! spill contents back through a `Stream`), but every DataFusion stream in this crate
//! is driven by `runtime.block_on(...)` on a `current_thread` Tokio runtime, on the
//! same OS thread as the rest of the Postgres backend (see `mpp/interrupt.rs`'s
//! `block_on_next`, which wraps that `block_on` in `HeldInterrupts` specifically
//! because the poll must not move to, or yield across, another thread). So the read
//! side here calls the blocking `BufFileRead` FFI directly and inline, via
//! `futures::stream::poll_fn`, instead of `spawn_blocking`: `spawn_blocking` would run
//! `BufFileRead` on a different OS thread, which is both unsound (`BufFile` and PG's
//! memory-context/resource-owner state are backend-thread-local) and would escape the
//! `HeldInterrupts` holdoff that the rest of the codebase relies on.

use bytes::Bytes;
use datafusion::common::exec_datafusion_err;
use datafusion::execution::disk_manager::DiskManagerMode;
use datafusion::execution::spill_file::{SpillFile, SpillWriter, TempFileFactory};
use futures::Stream;
use pgrx::pg_sys;
use std::io;
use std::os::raw::c_int;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Bytes read per `BufFileRead` call when streaming a spill file back to DataFusion.
/// Matches the 128KB DataFusion itself uses for its default OS-tempfile `SpillFile`
/// (`ReaderStream::with_capacity` in `disk_manager.rs`) — chosen there because the
/// default 8KB caused excessive per-poll overhead on multi-MB spill files.
const READ_CHUNK_BYTES: usize = 128 * 1024;

/// Returns a [`DiskManagerMode::Custom`] that spills through Postgres's `BufFile`
/// instead of DataFusion's OS-tempdir `DiskManager`.
pub fn buffile_disk_manager_mode() -> DiskManagerMode {
    DiskManagerMode::Custom(Arc::new(BufFileTempFileFactory))
}

/// Creates `BufFile`-backed [`SpillFile`]s on request from DataFusion's `DiskManager`.
#[derive(Debug)]
struct BufFileTempFileFactory;

impl TempFileFactory for BufFileTempFileFactory {
    fn create_temp_file(
        &self,
        description: &str,
    ) -> datafusion::common::Result<Arc<dyn SpillFile>> {
        // TEMP DIAGNOSTIC: confirms DataFusion is actually attempting to spill through
        // this factory at all, vs. failing before ever reaching it. Remove before merge.
        pgrx::warning!("paradedb.spill_to_disk: creating BufFile spill file for {description}");
        let file = unsafe { create_transaction_scoped_buffile() }.map_err(|e| {
            exec_datafusion_err!("failed to create BufFile spill file for {description}: {e}")
        })?;
        Ok(Arc::new(BufFileSpillFile {
            file: SendSyncBufFile(file),
            size: Arc::new(AtomicU64::new(0)),
        }))
    }
}

/// A `*mut pg_sys::BufFile` created by this backend. See the module-level `# BufFile
/// FFI` note: it's only ever read/written inline on this backend's one OS thread, so
/// `Send`/`Sync` just satisfy the trait bounds `SpillFile`/`SpillWriter` require —
/// nothing here actually crosses a thread.
#[derive(Debug, Clone, Copy)]
struct SendSyncBufFile(*mut pg_sys::BufFile);

impl SendSyncBufFile {
    /// Accessor rather than a public `.0`: closures that reach through `.0` directly
    /// let Rust's disjoint-closure-capture (RFC 2229) capture the bare `*mut BufFile`
    /// field instead of this `Send`/`Sync` newtype, silently defeating the impls below
    /// and failing to compile as `dyn Stream<... + Send>`. Going through a method call
    /// forces the whole `SendSyncBufFile` to be captured instead.
    fn get(self) -> *mut pg_sys::BufFile {
        self.0
    }
}

// SAFETY: see the module-level `# BufFile FFI` note. Every access to the wrapped
// pointer happens synchronously and inline on the single backend OS thread that
// created it (inside `block_on_next`'s `HeldInterrupts` window); it is never touched
// from another thread, so these impls only exist to satisfy the trait bounds.
unsafe impl Send for SendSyncBufFile {}
unsafe impl Sync for SendSyncBufFile {}

/// A `BufFile`-backed [`SpillFile`]. Created empty; [`SpillFile::open_writer`] is
/// called exactly once by DataFusion to populate it, then [`SpillFile::read_stream`]
/// may be called (and re-called, for multi-pass merges) any number of times after.
#[derive(Debug)]
struct BufFileSpillFile {
    file: SendSyncBufFile,
    /// Bytes written so far. `BufFile` doesn't expose a cheap "current size" query
    /// on all supported PG versions, so this is tracked on the write side instead.
    size: Arc<AtomicU64>,
}

impl SpillFile for BufFileSpillFile {
    fn path(&self) -> Option<&Path> {
        // BufFile has no single OS-visible path (it may span multiple 1GB segment
        // files, and may live in any of PG's temp_tablespaces); callers that want a
        // literal filesystem path (there are none in the traits' current callers,
        // as of the pinned DataFusion rev) can't be served here.
        None
    }

    fn size(&self) -> Option<u64> {
        Some(self.size.load(Ordering::Relaxed))
    }

    fn read_stream(
        &self,
    ) -> datafusion::common::Result<
        Pin<Box<dyn Stream<Item = datafusion::common::Result<Bytes>> + Send>>,
    > {
        let file = self.file;
        // Rewind to the start: open_writer() shares this file's one cursor, which is
        // left at end-of-data after the writes. (DataFusion's own OS-tempfile SpillFile
        // avoids this by opening a fresh, independently-positioned std::fs::File per
        // read_stream() call; BufFile has no path to reopen, so we seek the one handle
        // back to 0 instead. `whence = 0` is SEEK_SET -- pgrx doesn't bind a constant,
        // matching the literal keyset.rs uses.)
        unsafe { buffile_seek(file.get(), 0, 0, 0) }
            .map_err(|e| exec_datafusion_err!("failed to seek BufFile spill file: {e}"))?;
        // Reused across every chunk of this pass instead of allocating fresh per read;
        // BufFileRead fills it in place and we copy out only the bytes actually read.
        let mut scratch = vec![0u8; READ_CHUNK_BYTES];
        Ok(Box::pin(futures::stream::poll_fn(move |_cx| {
            // Runs inline, synchronously, on the backend's one OS thread — see the
            // module-level `# BufFile FFI` note. This is always polled from inside
            // `block_on_next`'s `HeldInterrupts` window, never spawned elsewhere, so a
            // plain blocking call here is both sound and consistent with the rest of
            // the crate's DataFusion streams.
            std::task::Poll::Ready(
                match unsafe { buffile_read_chunk(file.get(), &mut scratch) } {
                    Ok(0) => None,
                    Ok(n) => Some(Ok(Bytes::copy_from_slice(&scratch[..n]))),
                    Err(e) => Some(Err(exec_datafusion_err!(
                        "failed to read BufFile spill file: {e}"
                    ))),
                },
            )
        })))
    }

    fn open_writer(&self) -> datafusion::common::Result<Box<dyn SpillWriter>> {
        Ok(Box::new(BufFileSpillWriter {
            file: self.file,
            size: Arc::clone(&self.size),
        }))
    }
}

/// Writes to a `BufFile` via the [`std::io::Write`] impl [`SpillWriter`] requires.
struct BufFileSpillWriter {
    file: SendSyncBufFile,
    size: Arc<AtomicU64>,
}

impl io::Write for BufFileSpillWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe { buffile_write(self.file.get(), buf) }
            .map_err(|e| io::Error::other(format!("BufFile write failed: {e}")))?;
        self.size.fetch_add(buf.len() as u64, Ordering::Relaxed);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // BufFileWrite already goes through PG's buffered VFD layer; nothing to flush
        // beyond what Postgres itself manages between writes.
        Ok(())
    }
}

impl SpillWriter for BufFileSpillWriter {
    fn finish(&mut self) -> datafusion::common::Result<()> {
        // No explicit sync/close here: the file stays open (and readable) for
        // read_stream() calls after this. It's closed when BufFileSpillFile drops.
        Ok(())
    }
}

impl Drop for BufFileSpillFile {
    fn drop(&mut self) {
        unsafe { pg_sys::BufFileClose(self.file.get()) }
    }
}

/// Creates a `BufFile` scoped to the current transaction's resource owner and memory
/// context, so it outlives the per-tuple/per-batch contexts DataFusion operators may
/// be running under, but is still guaranteed to be cleaned up (temp file removed, VFD
/// released) when the transaction ends — matching `api::operator::keyset`'s `Sorter::finish`.
unsafe fn create_transaction_scoped_buffile() -> Result<*mut pg_sys::BufFile, &'static str> {
    unsafe {
        let saved_owner = pg_sys::CurrentResourceOwner;
        let saved_cxt = pg_sys::CurrentMemoryContext;
        pg_sys::CurrentResourceOwner = pg_sys::CurTransactionResourceOwner;
        pg_sys::CurrentMemoryContext = pg_sys::CurTransactionContext;
        let file = pg_sys::BufFileCreateTemp(false);
        pg_sys::CurrentResourceOwner = saved_owner;
        pg_sys::CurrentMemoryContext = saved_cxt;
        if file.is_null() {
            return Err("BufFileCreateTemp returned NULL");
        }
        Ok(file)
    }
}

/// Reads up to `buf.len()` bytes from `file`'s current position into `buf`, returning
/// the number of bytes read (`0` at EOF). Called inline from
/// [`SpillFile::read_stream`]'s `poll_fn`, reusing one scratch buffer across the whole
/// pass — see the module-level `# BufFile FFI` note for why this isn't dispatched to a
/// blocking pool.
unsafe fn buffile_read_chunk(
    file: *mut pg_sys::BufFile,
    buf: &mut [u8],
) -> Result<usize, &'static str> {
    unsafe {
        Ok(pg_sys::BufFileRead(
            file,
            buf.as_mut_ptr().cast(),
            buf.len(),
        ))
    }
}

// The following wrap Postgres APIs whose signatures differ across supported versions,
// mirroring `api::operator::keyset`'s shims for the same functions.

/// Write `data` to `file`. (PG15's `BufFileWrite` takes `*mut`; PG16+ takes `*const`.)
unsafe fn buffile_write(file: *mut pg_sys::BufFile, data: &[u8]) -> Result<(), &'static str> {
    unsafe {
        #[cfg(feature = "pg15")]
        pg_sys::BufFileWrite(file, data.as_ptr() as *mut std::ffi::c_void, data.len());
        #[cfg(not(feature = "pg15"))]
        pg_sys::BufFileWrite(file, data.as_ptr().cast::<std::ffi::c_void>(), data.len());
    }
    Ok(())
}

/// Seek `file` to `(fileno, offset)` relative to `whence`. (Signature is stable across
/// PG15+, unlike `BufFileWrite`/`BufFileRead`, but kept alongside the other shims for
/// symmetry with `keyset.rs` and in case that changes in a future PG version.)
unsafe fn buffile_seek(
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
