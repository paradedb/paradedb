use crate::index::directory::mvcc::BUFWRITER_CAPACITY;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{FileEntry, SegmentFileDetails};
use crate::postgres::storage::{LinkedBytesList, LinkedBytesListWriter};
use pgrx::pg_sys;
use std::io::{Result, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tantivy::directory::{AntiCallToken, TerminatingWrite};
use tantivy::index::SegmentComponent;

pub struct SegmentComponentWriter {
    inner: Option<InnerSegmentComponentWriter>,
    path: PathBuf,
}

impl SegmentComponentWriter {
    pub unsafe fn new(indexrel: &PgSearchRelation, path: &Path) -> Self {
        if path.component_type() == Some(SegmentComponent::Store) {
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
}

impl Write for SegmentComponentWriter {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        if let Some(inner) = self.inner.as_mut() {
            inner.write(data)
        } else {
            Ok(data.len())
        }
    }

    fn flush(&mut self) -> Result<()> {
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
}

struct InnerSegmentComponentWriter {
    header_blockno: pg_sys::BlockNumber,
    total_bytes: Arc<AtomicUsize>,
    buffer: ExactBuffer<{ BUFWRITER_CAPACITY }, LinkedBytesListWriter>,
}

impl InnerSegmentComponentWriter {
    pub unsafe fn new(indexrel: &PgSearchRelation) -> Self {
        let segment_component = LinkedBytesList::create_with_fsm(indexrel);

        Self {
            header_blockno: segment_component.header_blockno,
            total_bytes: Default::default(),
            buffer: ExactBuffer {
                writer: Some(segment_component.writer()),
                buffer: Box::new([0; BUFWRITER_CAPACITY]),
                len: 0,
            },
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
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let many = self.buffer.write(data)?;
        self.total_bytes.fetch_add(data.len(), Ordering::Relaxed);
        Ok(many)
    }

    fn flush(&mut self) -> Result<()> {
        self.buffer.flush()
    }
}

impl TerminatingWrite for InnerSegmentComponentWriter {
    fn terminate_ref(&mut self, _: AntiCallToken) -> Result<()> {
        // the underlying buffer should be flushed first
        self.buffer.flush()?;

        // then we tell its writer, which we have coded as LinkedBytesListWriter, to write its blocklist
        self.buffer
            .writer
            .take()
            .expect("ExactBuffer's underlying writer should still be set when InnerSegmentComponentWriter::terminate_ref is called")
            .finalize_and_write()?;
        Ok(())
    }
}

/// Similar to `[std::io::BufWriter]` except it only writes in increments of the `const CAPACITY: usize`
/// capacity.  Except on `flush()` where any remaining bytes are written.
struct ExactBuffer<const CAPACITY: usize, W: Write> {
    writer: Option<W>,
    buffer: Box<[u8; CAPACITY]>,
    len: usize,
}

impl<const CAPACITY: usize, W: Write> Drop for ExactBuffer<CAPACITY, W> {
    fn drop(&mut self) {
        if self.len != 0 {
            if std::thread::panicking() {
                // best we can do is WARN if we're dropping because of stack unwinding due to an ongoing panic
                // also, it would be expected for an ExactBuffer being dropped during stack unwinding to still
                // have unflushed data.
                pgrx::warning!(
                    "During stack unwinding due to a panic elsewhere, an ExactBuffer has {} unwritten bytes", self.len
                );
            } else if unsafe { pg_sys::InterruptPending } != 0 {
                pgrx::warning!(
                    "ExactBuffer still has {} unwritten bytes because of interrupt",
                    self.len
                );
            } else {
                panic!(
                    "ExactBuffer should have been flushed before being dropped.  It still has {} unwritten bytes", self.len
                );
            }
        }
    }
}

impl<const CAPACITY: usize, W: Write> Write for ExactBuffer<CAPACITY, W> {
    fn write(&mut self, mut data: &[u8]) -> Result<usize> {
        let writer = self
            .writer
            .as_mut()
            .expect("ExactBuffer's underlying writer should be set by the time .write() is called");

        let len = data.len();

        let mut end = len;
        if self.len < CAPACITY {
            // fill the buffer with what we can
            end = (CAPACITY - self.len).min(end);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    self.buffer.as_mut_ptr().add(self.len),
                    end,
                );
            }
            self.len += end;
            data = &data[end..];
        }

        if self.len == CAPACITY {
            // buffer is full -- write it out
            let _ = writer.write(&*self.buffer)?;
            self.len = 0;
        }

        while data.len() >= CAPACITY {
            // data has at least as many bytes as our capacity
            // write it out in chunks of our capacity, to avoid copying it into the buffer
            end = CAPACITY;
            let _ = writer.write(&data[..end])?;
            data = &data[end..];
        }

        if !data.is_empty() {
            // copy the rest to our buffer -- it'll fit
            unsafe {
                std::ptr::copy_nonoverlapping(data.as_ptr(), self.buffer.as_mut_ptr(), data.len());
                self.len = data.len();
            }
        }

        Ok(len)
    }

    fn flush(&mut self) -> Result<()> {
        let writer = self
            .writer
            .as_mut()
            .expect("ExactBuffer's underlying writer should be set by the time .flush() is called");

        if self.len > 0 {
            // write any remaining bytes we might have buffered to the underlying writer
            let _ = writer.write(&self.buffer[0..self.len])?;
            self.len = 0;
        }

        // and pass the flush request forward to the underlying writer
        writer.flush()
    }
}
