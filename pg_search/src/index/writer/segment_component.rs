use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{FileEntry, SegmentFileDetails};
use crate::postgres::storage::{LinkedBytesList, LinkedBytesListWriter};
use pgrx::pg_sys;
use std::io::{BufWriter, Result, Write};
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
    buffer: Option<BufWriter<LinkedBytesListWriter>>,
}

impl InnerSegmentComponentWriter {
    pub unsafe fn new(indexrel: &PgSearchRelation) -> Self {
        let segment_component = LinkedBytesList::create_with_fsm(indexrel);

        Self {
            header_blockno: segment_component.header_blockno,
            total_bytes: Default::default(),
            buffer: Some(BufWriter::new(segment_component.writer())),
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
