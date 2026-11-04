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

use crate::postgres::buffile::{self, BufFileReleaseGuard, TempFileOwner};
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
            file: Arc::new(TempFileOwner::new(file, release_guard)),
            size: Arc::new(AtomicU64::new(0)),
            writer_opened: AtomicBool::new(false),
        }))
    }
}

/// A `BufFile`-backed [`SpillFile`]. Created empty; DataFusion opens one writer to fill
/// it, then reads it back any number of times (multi-pass merges re-read).
struct BufFileSpillFile {
    file: Arc<TempFileOwner<*mut pg_sys::BufFile>>,
    /// Bytes written so far. `BufFile` doesn't expose a cheap "current size" query
    /// on all supported PG versions, so this is tracked on the write side instead.
    size: Arc<AtomicU64>,
    /// A second writer would append from wherever a reader left the cursor.
    writer_opened: AtomicBool,
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
        let file = Arc::clone(&self.file);
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
                file.with(|file| {
                    unsafe { buffile::buffile_seek(file, position.fileno, position.offset, 0) }
                        .map_err(io::Error::other)?;
                    let n = unsafe { buffile::buffile_read(file, &mut scratch) };
                    match n {
                        0 => Ok(None),
                        n => {
                            let (fileno, offset) = unsafe { buffile::buffile_tell(file) };
                            position = BufFilePosition { fileno, offset };
                            Ok(Some(Bytes::copy_from_slice(&scratch[..n])))
                        }
                    }
                })
                .map_err(|e| exec_datafusion_err!("failed to read BufFile spill file: {e}"))
                .transpose(),
            )
        })))
    }

    fn open_writer(&self) -> datafusion::common::Result<Box<dyn SpillWriter>> {
        let opened_before = self.writer_opened.swap(true, Ordering::Relaxed);
        debug_assert!(!opened_before, "a spill file takes one writer");
        let (fileno, offset) = self
            .file
            .with(|file| Ok(unsafe { buffile::buffile_tell(file) }))
            .map_err(|e| exec_datafusion_err!("failed to open BufFile spill writer: {e}"))?;
        Ok(Box::new(BufFileSpillWriter {
            file: Arc::clone(&self.file),
            size: Arc::clone(&self.size),
            position: BufFilePosition { fileno, offset },
        }))
    }
}

/// Writes to a `BufFile` via the [`std::io::Write`] impl [`SpillWriter`] requires. See
/// the module-level "Cursor tracking" note for why `position` is tracked here.
struct BufFileSpillWriter {
    file: Arc<TempFileOwner<*mut pg_sys::BufFile>>,
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
        let (fileno, offset) = self.file.with(|file| {
            unsafe { buffile::buffile_seek(file, self.position.fileno, self.position.offset, 0) }
                .map_err(io::Error::other)?;
            unsafe { buffile::buffile_write(file, buf) };
            Ok(unsafe { buffile::buffile_tell(file) })
        })?;
        self.size.fetch_add(buf.len() as u64, Ordering::Relaxed);
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
        // read_stream() calls after this. The last holder closes the file.
        Ok(())
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use futures::StreamExt;
    use pgrx::prelude::*;
    use std::io::Write;

    #[pg_test]
    fn test_temp_spill_interleaved_io_and_shared_ownership() {
        let factory = BufFileTempFileFactory {
            on_spill: Arc::new(|| {}),
            notified: AtomicBool::new(false),
            release_guard: OnceLock::new(),
        };
        let spill = factory.create_temp_file("test").unwrap();
        let mut writer = spill.open_writer().unwrap();
        writer.write_all(b"first").unwrap();
        let mut reader = spill.read_stream().unwrap();
        let first = futures::executor::block_on(reader.next()).unwrap().unwrap();
        assert_eq!(first.as_ref(), b"first");
        drop(spill);
        writer.write_all(b"second").unwrap();
        writer.finish().unwrap();
        drop(writer);
        let second = futures::executor::block_on(reader.next()).unwrap().unwrap();
        assert_eq!(second.as_ref(), b"second");
        assert!(futures::executor::block_on(reader.next()).is_none());
    }
}
