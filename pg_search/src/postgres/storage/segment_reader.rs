use anyhow::Result;
use pgrx::*;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::slice::from_raw_parts;
use tantivy::directory::FileHandle;
use tantivy::directory::OwnedBytes;
use tantivy::HasLen;

use crate::postgres::storage::buffer::BufferCache;
use crate::postgres::storage::segment_handle::SegmentHandle;
use crate::postgres::utils::max_heap_tuple_size;

#[derive(Clone, Debug)]
pub struct SegmentReader {
    path: PathBuf,
    handle: SegmentHandle,
    relation_oid: u32,
}

impl SegmentReader {
    pub unsafe fn new(relation_oid: u32, path: &Path) -> Result<Self> {
        let handle = SegmentHandle::open(relation_oid, path)?
            .unwrap_or_else(|| panic!("SegmentHandle should exist for {:?}", path));
        Ok(Self {
            path: path.to_path_buf(),
            handle,
            relation_oid,
        })
    }
}

impl FileHandle for SegmentReader {
    fn read_bytes(&self, range: Range<usize>) -> Result<OwnedBytes, std::io::Error> {
        unsafe {
            const MAX_HEAP_TUPLE_SIZE: usize = unsafe { max_heap_tuple_size() };
            let cache = BufferCache::open(self.relation_oid);
            let start = range.start;
            let end = range.end;
            let start_block = start / MAX_HEAP_TUPLE_SIZE;
            let end_block = end / MAX_HEAP_TUPLE_SIZE;
            let blocks = self.handle.internal().blocks();
            let mut data: Vec<u8> = vec![];

            for i in start_block..=end_block {
                let buffer = cache.get_buffer(blocks[i], pg_sys::BUFFER_LOCK_SHARE);
                let page = pg_sys::BufferGetPage(buffer);
                let item_id = pg_sys::PageGetItemId(page, pg_sys::FirstOffsetNumber);
                let item = pg_sys::PageGetItem(page, item_id);
                let len = (*item_id).lp_len() as usize;

                let slice_start = match i {
                    start_block => start % MAX_HEAP_TUPLE_SIZE,
                    _ => 0,
                };
                let slice_end = match i {
                    end_block => end % MAX_HEAP_TUPLE_SIZE,
                    _ => MAX_HEAP_TUPLE_SIZE,
                };
                let slice_len = slice_end - slice_start;
                let vec: Vec<u8> = Vec::with_capacity(slice_len);
                let slice = from_raw_parts(item.add(slice_start) as *const u8, slice_len);
                data.extend_from_slice(slice);

                pg_sys::UnlockReleaseBuffer(buffer);
            }

            Ok(OwnedBytes::new(data))
        }
    }
}

impl HasLen for SegmentReader {
    fn len(&self) -> usize {
        self.handle.internal().total_bytes()
    }
}
