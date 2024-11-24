use pgrx::*;
use std::io::{Result, Write};
use std::path::{Path, PathBuf};
use std::slice::from_raw_parts_mut;
use tantivy::directory::{AntiCallToken, Lock, TerminatingWrite, MANAGED_LOCK};
use tantivy::Directory;

use crate::index::blocking::BlockingDirectory;
use crate::postgres::storage::block::{bm25_metadata, BlockNumberList, DirectoryEntry};
use crate::postgres::storage::linked_list::{LinkedBytesList, LinkedItemList};
use crate::postgres::storage::utils::BM25BufferCache;

#[derive(Clone, Debug)]
pub struct SegmentComponentWriter {
    relation_oid: pg_sys::Oid,
    cache: BM25BufferCache,
    path: PathBuf,
    blocks: Vec<pg_sys::BlockNumber>,
    total_bytes: usize,
}

impl SegmentComponentWriter {
    pub unsafe fn new(relation_oid: pg_sys::Oid, path: &Path) -> Self {
        let cache = BM25BufferCache::open(relation_oid);
        let current_buffer = cache.new_buffer();
        let blockno = pg_sys::BufferGetBlockNumber(current_buffer);

        pg_sys::MarkBufferDirty(current_buffer);
        pg_sys::UnlockReleaseBuffer(current_buffer);

        Self {
            relation_oid,
            cache,
            path: path.to_path_buf(),
            blocks: vec![blockno],
            total_bytes: 0,
        }
    }
}

impl Write for SegmentComponentWriter {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let mut segment_component = LinkedBytesList::open(self.relation_oid, self.blocks[0]);
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
        // Store segment component's block numbers
        let cache = &self.cache;
        let new_buffer = unsafe { cache.new_buffer() };
        let blockno = unsafe { pg_sys::BufferGetBlockNumber(new_buffer) };

        unsafe {
            pg_sys::MarkBufferDirty(new_buffer);
            pg_sys::UnlockReleaseBuffer(new_buffer);
        }

        // TODO: Set special data for blockno
        let mut block_list = LinkedBytesList::open(self.relation_oid, blockno);
        let bytes: Vec<u8> = BlockNumberList(self.blocks.clone()).into();
        unsafe {
            block_list.write(&bytes).expect("write should succeed");
        }

        // TODO: Abstract this away
        unsafe {
            let metadata = bm25_metadata(self.relation_oid);
            let start_blockno = metadata.directory_start;
            let mut segment_components =
                unsafe { LinkedItemList::<DirectoryEntry>::open(self.relation_oid, start_blockno) };

            let opaque = DirectoryEntry {
                path: self.path.clone(),
                total_bytes: self.total_bytes,
                start: blockno,
                xid: unsafe { pg_sys::GetCurrentTransactionId() },
            };

            let directory = BlockingDirectory::new(self.relation_oid);
            let _lock = directory.acquire_lock(&Lock {
                filepath: MANAGED_LOCK.filepath.clone(),
                is_blocking: true,
            });

            segment_components
                .write(vec![opaque.clone()])
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }

        Ok(())
    }
}
