use pgrx::*;
use std::cmp::min;
use std::io::{Cursor, Read, Result, Write};
use std::path::{Path, PathBuf};
use std::slice::from_raw_parts_mut;
use tantivy::directory::{AntiCallToken, Lock, TerminatingWrite, MANAGED_LOCK};
use tantivy::Directory;

use crate::index::blocking::{BlockingDirectory, SEGMENT_COMPONENT_CACHE};
use crate::postgres::storage::block::{bm25_max_free_space, SegmentComponentOpaque};
use crate::postgres::storage::linked_list::LinkedItemList;
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
        let cache = &mut self.cache;
        let mut current_buffer = unsafe {
            cache.get_buffer(
                *self.blocks.last().expect("blocks should not be empty"),
                Some(pg_sys::BUFFER_LOCK_EXCLUSIVE),
            )
        };
        let mut data_cursor = Cursor::new(data);
        let mut bytes_written = 0;

        while bytes_written < data.len() {
            unsafe {
                let page = pg_sys::BufferGetPage(current_buffer);
                let header = page as *mut pg_sys::PageHeaderData;
                let free_space = ((*header).pd_upper - (*header).pd_lower) as usize;
                assert!(free_space <= bm25_max_free_space());

                let bytes_to_write = min(free_space, data.len() - bytes_written);
                if bytes_to_write == 0 {
                    let new_buffer = cache.new_buffer();
                    self.blocks.push(pg_sys::BufferGetBlockNumber(new_buffer));
                    pg_sys::MarkBufferDirty(current_buffer);
                    pg_sys::UnlockReleaseBuffer(current_buffer);
                    current_buffer = new_buffer;
                    continue;
                }

                let page_slice = from_raw_parts_mut(
                    (page as *mut u8).add((*header).pd_lower as usize),
                    bytes_to_write,
                );
                data_cursor.read_exact(page_slice)?;
                bytes_written += bytes_to_write;
                (*header).pd_lower += bytes_to_write as u16;
            }
        }

        unsafe {
            pg_sys::MarkBufferDirty(current_buffer);
            pg_sys::UnlockReleaseBuffer(current_buffer);
        };

        self.total_bytes += bytes_written;
        Ok(bytes_written)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl TerminatingWrite for SegmentComponentWriter {
    fn terminate_ref(&mut self, _: AntiCallToken) -> Result<()> {
        let mut linked_list = unsafe {
            LinkedItemList::<SegmentComponentOpaque>::new(
                self.relation_oid,
                |metadata| (*metadata).segment_component_first_blockno,
                |metadata| (*metadata).segment_component_last_blockno,
                |metadata, blockno| (*metadata).segment_component_first_blockno = blockno,
                |metadata, blockno| (*metadata).segment_component_last_blockno = blockno,
            )
        };

        let opaque = SegmentComponentOpaque {
            path: self.path.clone(),
            total_bytes: self.total_bytes,
            blocks: self.blocks.clone(),
            xid: unsafe { pg_sys::GetCurrentTransactionId() },
        };

        let directory = BlockingDirectory::new(self.relation_oid);

        let _lock = directory.acquire_lock(&Lock {
            filepath: MANAGED_LOCK.filepath.clone(),
            is_blocking: true,
        });

        unsafe {
            linked_list
                .add_items(vec![opaque.clone()], false, |opaque, blockno, offsetno| {
                    SEGMENT_COMPONENT_CACHE
                        .write()
                        .insert(opaque.path.clone(), (opaque.clone(), blockno, offsetno));
                })
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }

        Ok(())
    }
}
