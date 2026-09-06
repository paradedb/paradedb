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
//! configured `temp_tablespaces`, and are guaranteed to be cleaned up by Postgres —
//! via the resource-owner machinery if the query is cancelled or the backend aborts,
//! and by the postmaster on restart if the backend crashes — none of which
//! DataFusion's OS-tempdir path provides.
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
//!
//! # Cursor tracking
//!
//! `BufFile` has one cursor shared between every reader and writer of the same handle.
//! Under `RepartitionExec`, a `SpillPoolReader`'s [`SpillFile::read_stream`] and a
//! `BufFileSpillWriter`'s `write()` calls can interleave on the same file (both run on
//! the same OS thread, but call-by-call, not one to completion before the other
//! starts). Neither side can assume the cursor is still wherever *it* left it, because
//! the other side may have moved it in between calls. Both `BufFileSpillWriter`'s
//! `write()` implementation and the `poll_fn` inside [`SpillFile::read_stream`] track
//! their own position independently and re-seek to it before every
//! `BufFileWrite`/`BufFileRead` call — never relying on the shared cursor's position
//! surviving between calls, only ever setting it explicitly right before each use.

use crate::postgres::buffile;
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
        _description: &str,
    ) -> datafusion::common::Result<Arc<dyn SpillFile>> {
        let file = unsafe { buffile::create_transaction_scoped_buffile() };
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
        // BufFile has no single OS-visible path: it may span multiple segment
        // files, and may live in any of PG's configured temp_tablespaces.
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
        // This pass's own read position, tracked the same way BufFileSpillWriter tracks
        // its write position: a writer interleaved with this read can (and does, under
        // RepartitionExec) move the shared BufFile cursor between our poll_fn calls, so
        // every read below re-seeks to this tracked position first instead of trusting
        // that the cursor is still wherever our own previous read left it. Starts at
        // (0, 0) ("start of file"), the same as the up-front seek this replaces — see
        // the module-level "Cursor tracking" note. Tracks fileno as well as offset, not
        // just a running byte count: BufFile transparently spans multiple 1GB segment
        // files, so a spill exceeding one segment needs the real (fileno, offset) pair
        // BufFileTell reports, not a byte offset assumed to stay within segment 0.
        // `whence = 0` is SEEK_SET -- pgrx doesn't bind a constant, matching the literal
        // keyset.rs uses.
        let mut position = BufFilePosition {
            fileno: 0,
            offset: 0,
        };
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
                (|| {
                    // Re-anchor before every read: an interleaved write (or another pass's
                    // read_stream) since our last read may have moved the shared cursor.
                    unsafe {
                        buffile::buffile_seek(file.get(), position.fileno, position.offset, 0)
                    }
                    .map_err(|e| exec_datafusion_err!("failed to seek BufFile spill file: {e}"))?;
                    // [`SpillFile::read_stream`]'s `poll_fn`, reusing one scratch buffer across the whole
                    // pass — see the module-level `# BufFile FFI` note for why this isn't dispatched to a
                    // blocking pool.
                    match unsafe { buffile::buffile_read(file.get(), &mut scratch) } {
                        0 => Ok(None),
                        n => {
                            let (fileno, offset) = unsafe { buffile::buffile_tell(file.get()) };
                            position = BufFilePosition { fileno, offset };
                            Ok(Some(Bytes::copy_from_slice(&scratch[..n])))
                        }
                    }
                })()
                .transpose(),
            )
        })))
    }

    fn open_writer(&self) -> datafusion::common::Result<Box<dyn SpillWriter>> {
        // Ask PG for the true position rather than assuming (0, 0): this file was just
        // created and nothing has written to it yet, but reading the real value here
        // (instead of hardcoding the fresh-file case) keeps this correct even if that
        // assumption ever stops holding.
        let (fileno, offset) = unsafe { buffile::buffile_tell(self.file.get()) };
        Ok(Box::new(BufFileSpillWriter {
            file: self.file,
            size: Arc::clone(&self.size),
            position: BufFilePosition { fileno, offset },
        }))
    }
}

/// Writes to a `BufFile` via the [`std::io::Write`] impl [`SpillWriter`] requires. See
/// the module-level "Cursor tracking" note for why `position` is tracked here.
struct BufFileSpillWriter {
    file: SendSyncBufFile,
    size: Arc<AtomicU64>,
    position: BufFilePosition,
}

/// This writer's own last-known `BufFile` cursor position, restored before every write
/// so an interleaved `read_stream()` seek can't strand the next append. Mirrors the
/// `(fileno, offset)` pair `BufFileTell`/`BufFileSeek` use.
struct BufFilePosition {
    fileno: c_int,
    offset: pg_sys::off_t,
}

impl io::Write for BufFileSpillWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            // Re-anchor before writing: a read_stream() poll interleaved since our last
            // write may have moved the shared BufFile cursor (see module-level note).
            buffile::buffile_seek(
                self.file.get(),
                self.position.fileno,
                self.position.offset,
                0, /* SEEK_SET */
            )
            .map_err(|e| io::Error::other(format!("BufFile seek failed: {e}")))?;
            buffile::buffile_write(self.file.get(), buf);
        }
        self.size.fetch_add(buf.len() as u64, Ordering::Relaxed);
        let (fileno, offset) = unsafe { buffile::buffile_tell(self.file.get()) };
        self.position = BufFilePosition { fileno, offset };
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
