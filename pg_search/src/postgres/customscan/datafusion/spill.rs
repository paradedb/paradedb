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

//! Spills DataFusion operators to Postgres `BufFile` temp files instead of DataFusion's
//! default OS-tempfile `DiskManager`.
//!
//! Routing through `BufFile` means spill files count against `temp_file_limit`, land in
//! the configured `temp_tablespaces`, and are cleaned up by Postgres: by the resource
//! owner when the query is cancelled or the transaction aborts, and by the postmaster on
//! restart after a crash. DataFusion's OS-tempdir path provides none of that.
//!
//! # BufFile FFI
//!
//! A `BufFile` belongs to the backend (or parallel worker) that created it and must only
//! be touched from that process's main thread. Every DataFusion runtime in this crate is
//! a `current_thread` tokio runtime driven by `block_on` on that thread (the scans build
//! theirs in `joinscan` and `aggregatescan`, the workers in `mpp/launch.rs`), and
//! `spawn_buffered` is a no-op on that flavor, so every poll of a spill stream runs
//! inline on the backend thread. That is why the read side calls `BufFileRead` directly
//! from `poll_fn` rather than through `spawn_blocking`, and why the `Send`/`Sync` impls
//! on the pointer wrapper below are sound. `postgres::sequentialscan::keyset` relies on
//! the same property for its own `BufFile`.
//!
//! pgrx guards every `BufFile` call, so a raise such as a `temp_file_limit` hit unwinds
//! through DataFusion and tokio as a Rust panic; nothing here may add a second guard
//! around such a call, since a panic cannot cross the outer guard's C trampoline.
//! Closing is guarded on top of that: Postgres may
//! release the owner's files before the Rust holder drops ([`buffile::BufFileReleaseGuard`]),
//! and `BufFileClose` flushes, so a call that raised mid-flush leaves the file poisoned
//! (the close would raise again). A poisoned or released file, or one dropped from
//! abort processing, is left to the owner.
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

use crate::postgres::buffile::{self, BufFileReleaseGuard};
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
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Bytes read per `BufFileRead` call when streaming a spill file back to DataFusion.
/// Matches the 128KB DataFusion itself uses for its default OS-tempfile `SpillFile`
/// (`ReaderStream::with_capacity` in `disk_manager.rs`) — chosen there because the
/// default 8KB caused excessive per-poll overhead on multi-MB spill files.
const READ_CHUNK_BYTES: usize = 128 * 1024;

/// Poisons the file if a Postgres error unwinds through a `BufFile` call, the way
/// `std::sync::Mutex` poisons on a panic. The write buffer is then dirty, so a later
/// `BufFileClose` would flush and raise again.
struct PoisonOnUnwind<'a> {
    poisoned: &'a AtomicBool,
    panicking_before: bool,
}

impl<'a> PoisonOnUnwind<'a> {
    fn guard(poisoned: &'a AtomicBool) -> Self {
        Self {
            poisoned,
            panicking_before: std::thread::panicking(),
        }
    }
}

impl Drop for PoisonOnUnwind<'_> {
    fn drop(&mut self) {
        if !self.panicking_before && std::thread::panicking() {
            self.poisoned.store(true, Ordering::Relaxed);
        }
    }
}

/// Fired once by [`BufFileTempFileFactory`] the first time this query actually spills.
pub type SpillNotify = Arc<dyn Fn() + Send + Sync>;

/// A [`SpillNotify`] that stores into an `Arc<AtomicBool>` -- the shape both AggregateScan's
/// and JoinScan's leader-local execution need for their own `spilled` field.
pub fn notify_atomic_bool(flag: Arc<AtomicBool>) -> SpillNotify {
    Arc::new(move || flag.store(true, Ordering::Relaxed))
}

/// Returns a [`DiskManagerMode::Custom`] that spills through Postgres's `BufFile`
/// instead of DataFusion's OS-tempdir `DiskManager`.
pub fn buffile_disk_manager_mode(on_spill: SpillNotify) -> DiskManagerMode {
    DiskManagerMode::Custom(Arc::new(BufFileTempFileFactory {
        on_spill,
        notified: AtomicBool::new(false),
        release_guard: OnceLock::new(),
    }))
}

/// Creates `BufFile`-backed [`SpillFile`]s on request from DataFusion's `DiskManager`.
struct BufFileTempFileFactory {
    on_spill: SpillNotify,
    /// Guards `on_spill` to fire at most once per factory (once per scan node execution).
    notified: AtomicBool,
    /// Registered on the first spill, under the owner every file of this factory shares.
    release_guard: OnceLock<Arc<BufFileReleaseGuard>>,
}

impl std::fmt::Debug for BufFileTempFileFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufFileTempFileFactory")
            .field("notified", &self.notified.load(Ordering::Relaxed))
            .finish()
    }
}

impl TempFileFactory for BufFileTempFileFactory {
    fn create_temp_file(
        &self,
        _description: &str,
    ) -> datafusion::common::Result<Arc<dyn SpillFile>> {
        if self
            .notified
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            (self.on_spill)();
        }
        let shared = self
            .release_guard
            .get_or_init(BufFileReleaseGuard::register);
        // A file under another owner would never see that owner's release.
        let release_guard = if shared.owner() == unsafe { pg_sys::CurrentResourceOwner } {
            Arc::clone(shared)
        } else {
            BufFileReleaseGuard::register()
        };
        let file = unsafe { buffile::create_temp_buffile() };
        Ok(Arc::new(BufFileSpillFile {
            file: SendSyncBufFile(file),
            size: Arc::new(AtomicU64::new(0)),
            writer_opened: AtomicBool::new(false),
            poisoned: Arc::new(AtomicBool::new(false)),
            release_guard,
        }))
    }
}

/// A `*mut pg_sys::BufFile` created by this backend. `Send`/`Sync` only satisfy the
/// `SpillFile`/`SpillWriter` bounds; see the module doc, "BufFile FFI".
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

// SAFETY: see the module doc, "BufFile FFI": the pointer is only used inline on the
// backend thread that created it.
unsafe impl Send for SendSyncBufFile {}
unsafe impl Sync for SendSyncBufFile {}

/// A `BufFile`-backed [`SpillFile`]. Created empty; DataFusion opens one writer to fill
/// it, then reads it back any number of times (multi-pass merges re-read).
struct BufFileSpillFile {
    file: SendSyncBufFile,
    /// Bytes written so far. `BufFile` doesn't expose a cheap "current size" query
    /// on all supported PG versions, so this is tracked on the write side instead.
    size: Arc<AtomicU64>,
    /// A second writer would append from wherever a reader left the cursor.
    writer_opened: AtomicBool,
    /// Set when a call that may flush raised; see `PoisonOnUnwind`.
    poisoned: Arc<AtomicBool>,
    release_guard: Arc<BufFileReleaseGuard>,
}

impl std::fmt::Debug for BufFileSpillFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufFileSpillFile")
            .field("size", &self.size.load(Ordering::Relaxed))
            .finish()
    }
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
        let poisoned = Arc::clone(&self.poisoned);
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
            // Blocking and inline on purpose; see the module doc, "BufFile FFI".
            std::task::Poll::Ready(
                (|| {
                    // Re-anchor before every read: an interleaved write (or another pass's
                    // read_stream) since our last read may have moved the shared cursor.
                    let _poison = PoisonOnUnwind::guard(&poisoned);
                    unsafe {
                        buffile::buffile_seek(file.get(), position.fileno, position.offset, 0)
                    }
                    .map_err(|e| exec_datafusion_err!("failed to seek BufFile spill file: {e}"))?;
                    let n = unsafe { buffile::buffile_read(file.get(), &mut scratch) };
                    match n {
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
        let opened_before = self.writer_opened.swap(true, Ordering::Relaxed);
        debug_assert!(!opened_before, "a spill file takes one writer");
        let (fileno, offset) = unsafe { buffile::buffile_tell(self.file.get()) };
        Ok(Box::new(BufFileSpillWriter {
            file: self.file,
            size: Arc::clone(&self.size),
            poisoned: Arc::clone(&self.poisoned),
            position: BufFilePosition { fileno, offset },
        }))
    }
}

/// Writes to a `BufFile` via the [`std::io::Write`] impl [`SpillWriter`] requires. See
/// the module-level "Cursor tracking" note for why `position` is tracked here.
struct BufFileSpillWriter {
    file: SendSyncBufFile,
    size: Arc<AtomicU64>,
    poisoned: Arc<AtomicBool>,
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
        // DataFusion may write from a `Drop` (`SpillPoolSink` finishes its IPC stream),
        // and a raise there is a panic in a destructor. Refuse instead of touching the
        // file once it is poisoned, while unwinding, or outside a transaction; those
        // callers ignore the error.
        if self.poisoned.load(Ordering::Relaxed)
            || std::thread::panicking()
            || !unsafe { pg_sys::IsTransactionState() }
        {
            return Err(io::Error::other("BufFile spill file is no longer writable"));
        }
        // Re-anchor before writing: a read_stream() poll interleaved since our last
        // write may have moved the shared BufFile cursor (see module-level note).
        let file = self.file;
        let position = &self.position;
        {
            let _poison = PoisonOnUnwind::guard(&self.poisoned);
            unsafe { buffile::buffile_seek(file.get(), position.fileno, position.offset, 0) }
                .map_err(|e| io::Error::other(format!("BufFile seek failed: {e}")))?;
            unsafe { buffile::buffile_write(file.get(), buf) };
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
        if !self.poisoned.load(Ordering::Relaxed) && self.release_guard.may_close() {
            unsafe { pg_sys::BufFileClose(self.file.get()) }
        }
    }
}
