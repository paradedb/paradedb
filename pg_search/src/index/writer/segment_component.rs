use pgrx::*;
use std::io::{Result, Write};
use std::path::{Path, PathBuf};
use tantivy::directory::{AntiCallToken, TerminatingWrite};
use tantivy::Directory;

use crate::index::blocking::BlockingDirectory;
use crate::index::directory::lock::managed_lock;
use crate::postgres::storage::block::{bm25_metadata, BlockNumberList, DirectoryEntry};
use crate::postgres::storage::linked_list::{LinkedBytesList, LinkedItemList};
use crate::postgres::storage::utils::BM25BufferCache;

#[derive(Clone, Debug)]
pub struct SegmentComponentWriter {
    relation_oid: pg_sys::Oid,
    cache: BM25BufferCache,
    path: PathBuf,
    blocks: Vec<pg_sys::BlockNumber>,
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
            blocks: vec![],
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
        let mut block_created =
            unsafe { segment_component.write(data).expect("write should succeed") };
        self.blocks.append(&mut block_created);
        self.total_bytes += data.len();
        Ok(data.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl TerminatingWrite for SegmentComponentWriter {
    fn terminate_ref(&mut self, _: AntiCallToken) -> Result<()> {
        let mut block_list = unsafe { LinkedBytesList::create(self.relation_oid) };
        let bytes: Vec<u8> = BlockNumberList(self.blocks.clone()).into();
        unsafe {
            block_list.write(&bytes).expect("write should succeed");
        }
        let blockno = unsafe { pg_sys::BufferGetBlockNumber(block_list.lock_buffer) };

        unsafe {
            let blocking_directory = BlockingDirectory::new(self.relation_oid);
            let metadata = bm25_metadata(self.relation_oid);
            let start_blockno = metadata.directory_start;
            let mut directory = LinkedItemList::<DirectoryEntry>::open_with_lock(
                self.relation_oid,
                start_blockno,
                Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
            );

            let opaque = DirectoryEntry {
                path: self.path.clone(),
                total_bytes: self.total_bytes,
                start: blockno,
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
