use pgrx::*;
use std::io::{Result, Write};
use std::path::{Path, PathBuf};
use tantivy::directory::{AntiCallToken, TerminatingWrite};
use tantivy::Directory;

use crate::index::blocking::BlockingDirectory;
use crate::index::directory::lock::managed_lock;
use crate::postgres::storage::block::{bm25_metadata, DirectoryEntry};
use crate::postgres::storage::linked_list::{LinkedBytesList, LinkedItemList};
use crate::postgres::storage::utils::BM25BufferCache;

#[derive(Clone, Debug)]
pub struct SegmentComponentWriter {
    relation_oid: pg_sys::Oid,
    cache: BM25BufferCache,
    path: PathBuf,
    lock_blockno: pg_sys::BlockNumber,
    total_bytes: usize,
}

impl SegmentComponentWriter {
    pub unsafe fn new(relation_oid: pg_sys::Oid, path: &Path) -> Self {
        let cache = BM25BufferCache::open(relation_oid);
        let segment_component = LinkedBytesList::create(relation_oid);
        let lock_blockno = unsafe { pg_sys::BufferGetBlockNumber(segment_component.lock_buffer) };

        Self {
            relation_oid,
            cache,
            path: path.to_path_buf(),
            lock_blockno,
            total_bytes: 0,
        }
    }
}

impl Write for SegmentComponentWriter {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let mut segment_component = LinkedBytesList::open_with_lock(
            self.relation_oid,
            self.lock_blockno,
            Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
        );
        unsafe { segment_component.write(data).expect("write should succeed") };
        self.total_bytes += data.len();
        Ok(data.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl TerminatingWrite for SegmentComponentWriter {
    fn terminate_ref(&mut self, _: AntiCallToken) -> Result<()> {
        unsafe {
            let metadata = bm25_metadata(self.relation_oid);
            let directory_blockno = metadata.directory_start;
            let mut directory = LinkedItemList::<DirectoryEntry>::open_with_lock(
                self.relation_oid,
                directory_blockno,
                Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
            );

            let opaque = DirectoryEntry {
                path: self.path.clone(),
                total_bytes: self.total_bytes,
                start: self.lock_blockno,
                xmin: pg_sys::GetCurrentTransactionId(),
                xmax: pg_sys::InvalidTransactionId,
            };

            directory
                .write(vec![opaque.clone()])
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }

        let metadata = unsafe { bm25_metadata(self.relation_oid) };

        Ok(())
    }
}
