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

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{FileEntry, SegmentFileDetails};
use crate::postgres::storage::{LinkedBytesList, LinkedBytesListWriter};
use pgrx::pg_sys;
use std::io::{BufWriter, Result, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use tantivy::directory::{AntiCallToken, TerminatingWrite};
use tantivy::index::SegmentComponent;

/// The part of a component that its writer's checkpoint made readable: its length, and the data
/// blocks that hold it, in file order.
#[derive(Debug)]
pub struct CheckpointedPrefix {
    pub len: usize,
    /// Cloned into every reader of the prefix, and nowhere else: the writer counts the clones.
    pub blocks: Arc<Vec<pg_sys::BlockNumber>>,
}

/// A component's checkpoint, set at most once and shared between its writer and its directory.
pub type SharedCheckpoint = Arc<OnceLock<CheckpointedPrefix>>;

pub struct SegmentComponentWriter {
    inner: Option<InnerSegmentComponentWriter>,
    path: PathBuf,
}

impl SegmentComponentWriter {
    pub unsafe fn new(indexrel: &PgSearchRelation, path: &Path) -> Self {
        if path.component_type() == Some(SegmentComponent::Store)
            || path.component_type() == Some(SegmentComponent::TempStore)
        {
            Self {
                inner: None,
                path: path.to_path_buf(),
            }
        } else {
            Self {
                inner: Some(InnerSegmentComponentWriter::new(indexrel)),
                path: path.to_path_buf(),
            }
        }
    }

    #[allow(unused)]
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn file_entry(&self) -> FileEntry {
        if let Some(inner) = self.inner.as_ref() {
            inner.file_entry()
        } else {
            FileEntry {
                starting_block: pg_sys::InvalidBlockNumber,
                total_bytes: 0,
            }
        }
    }

    pub fn total_bytes(&self) -> Arc<AtomicUsize> {
        if let Some(inner) = self.inner.as_ref() {
            inner.total_bytes()
        } else {
            Arc::new(AtomicUsize::new(0))
        }
    }

    /// The slot [`TerminatingWrite::checkpoint`] fills. It stays empty for a component that is
    /// never checkpointed, and for one whose bytes are not stored.
    pub fn checkpointed(&self) -> SharedCheckpoint {
        if let Some(inner) = self.inner.as_ref() {
            inner.checkpoint.clone()
        } else {
            Default::default()
        }
    }
}

impl Write for SegmentComponentWriter {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        // Skip re-entering the PG buffer manager while unwinding: the outer
        // BufWriter (Tantivy's WritePtr) flushes us on drop, which would
        // cause a double-panic and SIGABRT.  See #5218.
        // Returns Ok(data.len()) so the caller thinks the bytes were consumed,
        // but nothing is actually persisted — the data is silently discarded.
        if std::thread::panicking() {
            return Ok(data.len());
        }
        if let Some(inner) = self.inner.as_mut() {
            inner.write(data)
        } else {
            Ok(data.len())
        }
    }

    fn flush(&mut self) -> Result<()> {
        // Same guard as write() above -- see #5218.
        if std::thread::panicking() {
            return Ok(());
        }
        if let Some(inner) = self.inner.as_mut() {
            inner.flush()
        } else {
            Ok(())
        }
    }
}

impl TerminatingWrite for SegmentComponentWriter {
    fn terminate_ref(&mut self, _token: AntiCallToken) -> Result<()> {
        if let Some(inner) = self.inner.as_mut() {
            inner.terminate_ref(_token)
        } else {
            Ok(())
        }
    }

    fn checkpoint(&mut self) -> Result<()> {
        match self.inner.as_mut() {
            Some(inner) => inner.checkpoint(),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("{:?} stores no bytes to checkpoint", self.path),
            )),
        }
    }
}

struct InnerSegmentComponentWriter {
    header_blockno: pg_sys::BlockNumber,
    total_bytes: Arc<AtomicUsize>,
    buffer: Option<PanicSafeBufWriter<LinkedBytesListWriter>>,
    checkpoint: SharedCheckpoint,
}

/// Wrapper around [`BufWriter`] that skips flush-on-drop while unwinding.
///
/// During unwind (e.g. PostgreSQL ERROR translated to Rust panic), dropping `BufWriter`
/// may flush buffered bytes and re-enter PostgreSQL buffer APIs, causing a second panic
/// in cleanup.
struct PanicSafeBufWriter<W: Write> {
    inner: Option<BufWriter<W>>,
}

impl<W: Write> PanicSafeBufWriter<W> {
    fn new(writer: W) -> Self {
        Self {
            inner: Some(BufWriter::new(writer)),
        }
    }

    fn into_inner(mut self) -> Result<W> {
        Ok(self.inner.take().unwrap().into_inner()?)
    }

    fn get_ref(&self) -> &W {
        self.inner.as_ref().unwrap().get_ref()
    }
}

impl<W: Write> Write for PanicSafeBufWriter<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.as_mut().unwrap().write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.as_mut().unwrap().flush()
    }
}

/// We are intentionally not using impl_safe_drop! here because
/// we want to skip the `Drop` of the writer entirely on panic
impl<W: Write> Drop for PanicSafeBufWriter<W> {
    fn drop(&mut self) {
        if std::thread::panicking()
            && let Some(buffer) = self.inner.take()
        {
            std::mem::forget(buffer);
        }
    }
}

impl InnerSegmentComponentWriter {
    pub unsafe fn new(indexrel: &PgSearchRelation) -> Self {
        let segment_component = LinkedBytesList::create_with_fsm(indexrel);

        Self {
            header_blockno: segment_component.header_blockno,
            total_bytes: Default::default(),
            buffer: Some(PanicSafeBufWriter::new(segment_component.writer())),
            checkpoint: Default::default(),
        }
    }

    /// Flushes the buffered bytes to their pages and publishes the blocks written so far. The
    /// block list is shared from this writer, 4 bytes per data page: no page is written for it,
    /// and the block map of the finished component is the one `terminate` writes.
    fn checkpoint(&mut self) -> Result<()> {
        let buffer = self.buffer.as_mut().unwrap();
        buffer.flush()?;
        let prefix = CheckpointedPrefix {
            len: self.total_bytes.load(Ordering::Relaxed),
            blocks: Arc::new(buffer.get_ref().blocks()),
        };
        self.checkpoint.set(prefix).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "segment component is already checkpointed",
            )
        })
    }

    /// Errors while a reader of the checkpointed prefix is open. Such a reader holds pages
    /// pinned on the promise that they do not change, and a write appends to the last of them.
    ///
    /// Every reader holds a clone of the checkpoint's block list, so a count above the
    /// checkpoint's own means one is open.
    fn refuse_under_reader(&self) -> Result<()> {
        match self.checkpoint.get() {
            Some(prefix) if Arc::strong_count(&prefix.blocks) > 1 => Err(std::io::Error::new(
                std::io::ErrorKind::ResourceBusy,
                "segment component is written while a reader of its checkpointed prefix is open",
            )),
            _ => Ok(()),
        }
    }

    pub fn total_bytes(&self) -> Arc<AtomicUsize> {
        self.total_bytes.clone()
    }

    pub fn file_entry(&self) -> FileEntry {
        FileEntry {
            starting_block: self.header_blockno,
            total_bytes: self.total_bytes.load(Ordering::Relaxed),
        }
    }
}

impl Write for InnerSegmentComponentWriter {
    /// A write refused under a reader buffers no byte and touches no page.
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.refuse_under_reader()?;
        let many = self.buffer.as_mut().unwrap().write(data)?;
        self.total_bytes.fetch_add(data.len(), Ordering::Relaxed);
        Ok(many)
    }

    fn flush(&mut self) -> Result<()> {
        self.buffer.as_mut().unwrap().flush()
    }
}

impl TerminatingWrite for InnerSegmentComponentWriter {
    fn terminate_ref(&mut self, _: AntiCallToken) -> Result<()> {
        // the underlying buffer should be flushed first
        let mut buffer = self.buffer.take().unwrap();

        buffer.flush()?;
        buffer.into_inner()?.finalize_and_write()?;
        Ok(())
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::postgres::rel::PgSearchRelation;
    use crate::postgres::storage::buffer::BufferManager;
    use pgrx::prelude::*;
    use std::io::Write;
    use std::path::Path;

    #[pg_test]
    #[should_panic(expected = "no unpinned buffers available")]
    #[ignore = "must be run in isolation because it intentionally starves the buffer cache"]
    fn writer_does_not_double_panic() {
        Spi::run("DROP TABLE IF EXISTS segment_component_drop_guard;").unwrap();
        Spi::run("CREATE TABLE segment_component_drop_guard (id SERIAL PRIMARY KEY, body TEXT);")
            .unwrap();
        Spi::run(
            "CREATE INDEX segment_component_drop_guard_idx ON segment_component_drop_guard USING paradedb (id, body);",
        )
        .unwrap();

        let index_oid = Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_class WHERE relname = 'segment_component_drop_guard_idx' AND relkind = 'i';",
        )
        .unwrap()
        .expect("index oid should exist");

        let indexrel = PgSearchRelation::open(index_oid);

        let mut writer = unsafe {
            SegmentComponentWriter::new(
                &indexrel,
                Path::new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.idx"),
            )
        };
        writer
            .write_all(&vec![7u8; 16 * 1024])
            .expect("seed write should succeed");

        // Pin newly extended pages while releasing their locks; this targets "no unpinned buffers available"
        let mut bman = BufferManager::new(&indexrel);
        let mut pinned = Vec::new();
        loop {
            pinned.push(bman.new_buffer().into_immutable_page());
        }
    }

    /// Verifies that [`SegmentComponentWriter`]'s `Write` impl no-ops during
    /// unwind instead of re-entering the PG buffer manager.
    #[pg_test]
    fn writer_noops_during_unwind() {
        use std::panic::{AssertUnwindSafe, catch_unwind};
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

        static WRITE_OK: AtomicBool = AtomicBool::new(false);
        static FLUSH_OK: AtomicBool = AtomicBool::new(false);
        // Sentinel so the final assert fails if the Drop hook never records.
        static BYTES_AFTER: AtomicUsize = AtomicUsize::new(usize::MAX);

        // Reset in case this process ever runs the test more than once;
        // stale values would weaken the asserts below.
        WRITE_OK.store(false, Ordering::SeqCst);
        FLUSH_OK.store(false, Ordering::SeqCst);
        BYTES_AFTER.store(usize::MAX, Ordering::SeqCst);

        Spi::run("DROP TABLE IF EXISTS scw_unwind_guard;").unwrap();
        Spi::run("CREATE TABLE scw_unwind_guard (id SERIAL PRIMARY KEY, body TEXT);").unwrap();
        Spi::run(
            "CREATE INDEX scw_unwind_guard_idx ON scw_unwind_guard USING paradedb (id, body);",
        )
        .unwrap();

        let index_oid = Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_class WHERE relname = 'scw_unwind_guard_idx' AND relkind = 'i';",
        )
        .unwrap()
        .expect("index oid should exist");

        let indexrel = PgSearchRelation::open(index_oid);

        // Create the writer BEFORE the panic so we don't touch PG during unwind.
        let writer = unsafe {
            SegmentComponentWriter::new(
                &indexrel,
                Path::new("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.idx"),
            )
        };

        // Snapshot total_bytes before the panic to prove write() was truly
        // skipped during unwind, not just silently accepted by the inner writer.
        let bytes_before = writer.total_bytes().load(Ordering::SeqCst);

        // Use catch_unwind (not thread::spawn) because PgSearchRelation is !Send.
        // The Drop impl runs while std::thread::panicking() == true, exercising
        // the write/flush guards on the pre-created writer.
        struct CallDuringDrop {
            writer: SegmentComponentWriter,
        }
        // NOTE: We intentionally do NOT use `impl_safe_drop!` here because the test needs this body
        // to run while unwinding.
        impl Drop for CallDuringDrop {
            fn drop(&mut self) {
                if !std::thread::panicking() {
                    return;
                }
                // Record observations only -- a failed assert here would be a
                // panic during unwind, aborting the test harness.  The asserts
                // happen after catch_unwind returns.
                if let Ok(n) = self.writer.write(b"should be skipped")
                    && n == 17
                {
                    WRITE_OK.store(true, Ordering::SeqCst);
                }
                if self.writer.flush().is_ok() {
                    FLUSH_OK.store(true, Ordering::SeqCst);
                }
                BYTES_AFTER.store(
                    self.writer.total_bytes().load(Ordering::SeqCst),
                    Ordering::SeqCst,
                );
            }
        }

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _guard = CallDuringDrop { writer };
            panic!("trigger unwind to exercise write/flush guards");
        }));

        assert!(result.is_err(), "catch_unwind should have caught the panic");
        assert!(
            WRITE_OK.load(Ordering::SeqCst),
            "write during unwind should return Ok(17)"
        );
        assert!(
            FLUSH_OK.load(Ordering::SeqCst),
            "flush during unwind should return Ok(())"
        );
        // Verify total_bytes didn't change -- the write was discarded,
        // not forwarded to the inner writer.
        assert_eq!(
            bytes_before,
            BYTES_AFTER.load(Ordering::SeqCst),
            "total_bytes must not change during unwind write"
        );
    }
}
