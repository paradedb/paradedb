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
//! be touched from that process's main thread. Every `BufFile` creation, read, seek, tell,
//! write, and close in this module is dispatched to the creating backend's main thread via
//! [`crate::postgres::customscan::mpp::run_on_main_thread`] and
//! [`crate::postgres::customscan::mpp::drop_on_main_thread`] (inside [`TempFileOwner`]),
//! which is why the `Send`/`Sync` impls on [`TempFileOwner`] are sound even when DataFusion
//! polls spill streams from background Tokio worker threads.
//!
//! pgrx guards every `BufFile` call, so a raise such as a `temp_file_limit` hit unwinds
//! on the main thread as a Rust panic and is caught and propagated by `run_on_main_thread`;
//! nothing here may add a second guard around such a call, since a panic cannot cross the
//! outer guard's C trampoline. Closing is guarded on top of that: Postgres may
//! release the owner's files before the Rust holder drops ([`buffile::BufFileReleaseGuard`]),
//! and `BufFileClose` flushes, so a call that raised mid-flush leaves the file poisoned
//! (the close would raise again). A poisoned or released file, or one dropped from
//! abort processing, is left to the owner.
//!
//! # Cursor tracking
//!
//! `BufFile` has one cursor shared between every reader and writer of the same handle.
//! Under `RepartitionExec`, a `SpillPoolReader`'s [`SpillFile::read_stream`] and a
//! `BufFileSpillWriter`'s `write()` calls can interleave on the same file (serialized on
//! the main thread call-by-call, not one to completion before the other
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
use std::cell::OnceCell;
use std::io;
use std::os::raw::c_int;
use std::path::Path;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
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
        release_guard: crate::postgres::customscan::mpp::MainThread::new(OnceCell::new()),
    }))
}

/// Creates `BufFile`-backed [`SpillFile`]s on request from DataFusion's `DiskManager`.
struct BufFileTempFileFactory {
    on_spill: SpillNotify,
    /// Guards `on_spill` to fire at most once per factory (once per scan node execution).
    notified: AtomicBool,
    /// Registered on the first spill, under the owner every file of this factory shares.
    /// Wrapped in [`crate::postgres::customscan::mpp::MainThread`] so `UnregisterResourceReleaseCallback`
    /// always runs on the Postgres main thread even when DataFusion drops the `RuntimeEnv` on a
    /// background Tokio worker thread.
    release_guard: crate::postgres::customscan::mpp::MainThread<OnceCell<Rc<BufFileReleaseGuard>>>,
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
        crate::postgres::customscan::mpp::try_run_on_main_thread(|token| {
            let shared = self
                .release_guard
                .get(token)
                .get_or_init(BufFileReleaseGuard::register);
            // A file under another owner would never see that owner's release.
            let release_guard = if shared.owner() == unsafe { pg_sys::CurrentResourceOwner } {
                Rc::clone(shared)
            } else {
                BufFileReleaseGuard::register()
            };
            let file = unsafe { buffile::create_temp_buffile() };
            Ok(Arc::new(BufFileSpillFile {
                file: Arc::new(TempFileOwner::new(file, release_guard)),
                size: Arc::new(AtomicU64::new(0)),
                writer_opened: AtomicBool::new(false),
            }) as Arc<dyn SpillFile>)
        })
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
    use crate::postgres::customscan::mpp::interrupt::PgTokioRuntime;
    use futures::StreamExt;
    use pgrx::prelude::*;
    use std::io::Write;
    use std::sync::atomic::AtomicUsize;

    fn expected_byte(offset: usize) -> u8 {
        (offset.wrapping_mul(31).wrapping_add(7)) as u8
    }

    #[pg_test]
    fn test_temp_spill_interleaved_io_and_shared_ownership() {
        let notify_count = Arc::new(AtomicUsize::new(0));
        let notify_clone = Arc::clone(&notify_count);
        let factory = Arc::new(BufFileTempFileFactory {
            on_spill: Arc::new(move || {
                notify_clone.fetch_add(1, Ordering::Relaxed);
            }),
            notified: AtomicBool::new(false),
            release_guard: crate::postgres::customscan::mpp::MainThread::new(OnceCell::new()),
        });

        // 1. Basic shared-ownership and sequential read/write cursor interleaving.
        let spill = factory.create_temp_file("test").unwrap();
        assert_eq!(notify_count.load(Ordering::Relaxed), 1);
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

        // 2. Concurrent multi-threaded PgTokioRuntime reader/writer interleaving across
        // READ_CHUNK_BYTES (128KB) boundaries on the same BufFile.
        let runtime = PgTokioRuntime::new().unwrap();
        runtime.block_on(async {
            let spill = factory.create_temp_file("concurrent").unwrap();
            // `on_spill` must fire only once per factory.
            assert_eq!(notify_count.load(Ordering::Relaxed), 1);

            let chunk_sizes: Vec<usize> = (0..24)
                .map(|i| (i * 17_389 + 40_000) % 90_000 + 50_000)
                .collect();
            let total_expected: usize = chunk_sizes.iter().sum();

            let (progress_tx, progress_rx1) = tokio::sync::watch::channel((0usize, false));
            let mut progress_rx2 = progress_rx1.clone();

            let spill_for_writer = Arc::clone(&spill);
            let writer_handle = tokio::spawn(async move {
                let mut writer = spill_for_writer.open_writer().unwrap();
                let mut offset = 0usize;
                for size in chunk_sizes {
                    let buf: Vec<u8> = (offset..offset + size).map(expected_byte).collect();
                    writer.write_all(&buf).unwrap();
                    offset += size;
                    let _ = progress_tx.send((offset, false));
                    tokio::task::yield_now().await;
                }
                writer.finish().unwrap();
                drop(writer);
                let _ = progress_tx.send((offset, true));
                offset
            });

            // Reader 1 trails the writer chunk-by-chunk while the writer is still appending.
            let spill_for_r1 = Arc::clone(&spill);
            let mut progress_rx1 = progress_rx1;
            let reader1_handle = tokio::spawn(async move {
                let mut stream = spill_for_r1.read_stream().unwrap();
                let mut read_offset = 0usize;
                loop {
                    loop {
                        let (written, done) = *progress_rx1.borrow_and_update();
                        if done || written >= read_offset + READ_CHUNK_BYTES {
                            break;
                        }
                        progress_rx1.changed().await.unwrap();
                    }
                    match stream.next().await {
                        Some(Ok(bytes)) => {
                            for (i, &b) in bytes.iter().enumerate() {
                                assert_eq!(
                                    b,
                                    expected_byte(read_offset + i),
                                    "reader 1 byte mismatch at offset {}",
                                    read_offset + i
                                );
                            }
                            read_offset += bytes.len();
                        }
                        Some(Err(e)) => panic!("reader 1 failed: {e}"),
                        None => break,
                    }
                }
                read_offset
            });

            // Reader 2 starts midway through the write stream and reads concurrently with
            // both the writer and Reader 1.
            let spill_for_r2 = Arc::clone(&spill);
            let reader2_handle = tokio::spawn(async move {
                loop {
                    let (written, done) = *progress_rx2.borrow_and_update();
                    if done || written >= 512 * 1024 {
                        break;
                    }
                    progress_rx2.changed().await.unwrap();
                }
                let mut stream = spill_for_r2.read_stream().unwrap();
                let mut read_offset = 0usize;
                loop {
                    loop {
                        let (written, done) = *progress_rx2.borrow_and_update();
                        if done || written >= read_offset + READ_CHUNK_BYTES {
                            break;
                        }
                        progress_rx2.changed().await.unwrap();
                    }
                    match stream.next().await {
                        Some(Ok(bytes)) => {
                            for (i, &b) in bytes.iter().enumerate() {
                                assert_eq!(
                                    b,
                                    expected_byte(read_offset + i),
                                    "reader 2 byte mismatch at offset {}",
                                    read_offset + i
                                );
                            }
                            read_offset += bytes.len();
                        }
                        Some(Err(e)) => panic!("reader 2 failed: {e}"),
                        None => break,
                    }
                }
                read_offset
            });

            drop(spill);
            let written = writer_handle.await.unwrap();
            let r1_bytes = reader1_handle.await.unwrap();
            let r2_bytes = reader2_handle.await.unwrap();
            assert_eq!(written, total_expected);
            assert_eq!(r1_bytes, total_expected);
            assert_eq!(r2_bytes, total_expected);
        });
    }

    /// End-to-end MPP spill regression test (3 deterministic segments across 3 parallel workers,
    /// `work_mem = '1MB'`).
    ///
    /// Uses a single-table MPP `AggregateScan` rather than a `HashJoinExec` because DataFusion's
    /// `HashJoinExec` does not spill to disk and `estimate_plan_memory` scales stage memory by
    /// `work_mem * hash_mem_multiplier * partitions` when a `HashJoinExec` is present; tight
    /// `work_mem` bounds that deterministically fail with `spill_to_disk = off` and spill with
    /// `spill_to_disk = on` require a stage plan without `HashJoinExec`.
    #[pg_test]
    fn test_mpp_spilling_buffile() {
        Spi::run(
            r#"
            SET paradedb.enable_aggregate_custom_scan TO on;
            SET paradedb.max_term_agg_buckets TO 1;
            SET paradedb.mpp_min_rows TO 0;
            SET enable_indexscan TO off;
            SET paradedb.enable_columnar_exec TO true;

            CREATE TABLE mpp_spill_items (
                id INTEGER,
                group_key TEXT,
                content TEXT,
                size_bytes INTEGER
            );

            CREATE INDEX mpp_spill_items_idx ON mpp_spill_items
            USING bm25 (id, group_key, content, size_bytes)
            WITH (
                numeric_fields = '{"id": {"fast": true}, "size_bytes": {"fast": true}}',
                text_fields = '{"group_key": {"fast": true}, "content": {}}',
                target_segment_count = 3,
                layer_sizes = '0',
                background_layer_sizes = '0'
            );

            SET paradedb.global_mutable_segment_rows = 0;
            INSERT INTO mpp_spill_items (id, group_key, content, size_bytes)
            SELECT g, 'group-key-payload-' || g, 'Section ' || g, (g * 17) % 4096
            FROM generate_series(1, 10000) AS g;
            INSERT INTO mpp_spill_items (id, group_key, content, size_bytes)
            SELECT g, 'group-key-payload-' || g, 'Section ' || g, (g * 17) % 4096
            FROM generate_series(10001, 20000) AS g;
            INSERT INTO mpp_spill_items (id, group_key, content, size_bytes)
            SELECT g, 'group-key-payload-' || g, 'Section ' || g, (g * 17) % 4096
            FROM generate_series(20001, 30000) AS g;
            RESET paradedb.global_mutable_segment_rows;

            ANALYZE mpp_spill_items;

            SET max_parallel_workers_per_gather TO 3;
            SET max_parallel_workers TO 64;
            SET min_parallel_table_scan_size TO 0;
            SET parallel_setup_cost TO 0;
            SET parallel_tuple_cost TO 0;
            SET work_mem = '1MB';
            "#,
        )
        .unwrap();

        let seg_count =
            Spi::get_one::<i64>("SELECT count(*) FROM paradedb.index_info('mpp_spill_items_idx')")
                .unwrap()
                .unwrap();
        assert_eq!(seg_count, 3, "expected 3 deterministic segments");

        // 1. With spill_to_disk = off (default), the query must fail on work_mem overflow.
        Spi::run(
            r#"
            SET paradedb.spill_to_disk TO off;
            CREATE TEMP TABLE mpp_spill_off_outcome (msg text);
            DO $$
            DECLARE
                err_text text;
            BEGIN
                PERFORM id, group_key, COUNT(*), SUM(size_bytes)
                FROM mpp_spill_items
                WHERE content @@@ 'Section'
                GROUP BY id, group_key;

                INSERT INTO mpp_spill_off_outcome VALUES ('unexpected_success');
            EXCEPTION WHEN OTHERS THEN
                GET STACKED DIAGNOSTICS err_text = MESSAGE_TEXT;
                INSERT INTO mpp_spill_off_outcome VALUES (err_text);
            END $$;
            "#,
        )
        .unwrap();
        let off_msg = Spi::get_one::<String>("SELECT msg FROM mpp_spill_off_outcome")
            .unwrap()
            .unwrap();
        assert!(
            off_msg.contains("raise work_mem")
                || off_msg.contains("DiskManager is disabled")
                || off_msg.contains("transport receiver detached"),
            "expected work_mem / spill failure with spill_to_disk=off, got: {off_msg}"
        );

        // 2. With spill_to_disk = on, EXPLAIN ANALYZE must show DistributedExec and spilling.
        Spi::run(
            r#"
            SET paradedb.spill_to_disk TO on;
            CREATE OR REPLACE FUNCTION explain_analyze_text(q text)
            RETURNS text AS $$
            DECLARE
                r record;
                lines text[] := ARRAY[]::text[];
            BEGIN
                FOR r IN EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF, BUFFERS OFF) ' || q
                LOOP
                    lines := array_append(lines, r."QUERY PLAN");
                END LOOP;
                RETURN array_to_string(lines, E'\n');
            END $$ LANGUAGE plpgsql;
            "#,
        )
        .unwrap();
        let plan_text = Spi::get_one::<String>(
            r#"
            SELECT explain_analyze_text(
                'SELECT id, group_key, COUNT(*), SUM(size_bytes)
                 FROM mpp_spill_items
                 WHERE content @@@ ''Section''
                 GROUP BY id, group_key'
            )
            "#,
        )
        .unwrap()
        .unwrap();

        assert!(
            plan_text.contains("DistributedExec"),
            "expected DistributedExec in MPP plan, got:\n{plan_text}"
        );
        let spilled = Spi::get_one_with_args::<bool>(
            "SELECT $1 ~ 'spill_count=(\\d*[1-9]|\\{[^}]*\\d:\\s*\\d*[1-9])'",
            &[plan_text.clone().into()],
        )
        .unwrap()
        .unwrap();
        assert!(
            spilled,
            "expected non-zero spill_count in MPP plan, got:\n{plan_text}"
        );

        // 3. Verify exact result correctness across all 30,000 groups.
        let (all_present, all_correct) = Spi::get_two::<bool, bool>(
            r#"
            SELECT
                COUNT(*) = 30000 AS all_groups_present,
                COUNT(*) FILTER (
                    WHERE cnt <> 1
                       OR total_size <> ((id * 17) % 4096)
                       OR group_key <> ('group-key-payload-' || id)
                ) = 0 AS all_groups_correct
            FROM (
                SELECT id, group_key, COUNT(*) AS cnt, SUM(size_bytes) AS total_size
                FROM mpp_spill_items
                WHERE content @@@ 'Section'
                GROUP BY id, group_key
            ) q
            "#,
        )
        .unwrap();
        assert_eq!(
            all_present,
            Some(true),
            "expected all 30,000 groups present"
        );
        assert_eq!(
            all_correct,
            Some(true),
            "expected all 30,000 groups to match formula"
        );
    }
}
