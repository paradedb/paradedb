use anyhow::Result;
use pgrx::*;
use std::ops::Range;
use std::path::{Path, PathBuf};
use tantivy::directory::FileHandle;
use tantivy::directory::OwnedBytes;
use tantivy::HasLen;

use crate::postgres::storage::segment_handle::SegmentHandle;
use crate::postgres::utils::max_heap_tuple_size;

#[derive(Clone, Debug)]
pub struct SegmentReader {
    path: PathBuf,
    blockno: pg_sys::BlockNumber,
    // A segment is represented by one or more blocks.
    // block_offset represents the offset of the block within the segment.
    block_offset: u32,
    handle: SegmentHandle,
}

impl SegmentReader {
    pub unsafe fn new(relation_oid: u32, path: &Path) -> Result<Self> {
        let handle = SegmentHandle::open(relation_oid, path)?
            .expect(&format!("SegmentHandle should exist for {:?}", path));
        Ok(Self {
            path: path.to_path_buf(),
            blockno: handle.internal().blockno(),
            block_offset: 0,
            handle,
        })
    }
}

impl FileHandle for SegmentReader {
    fn read_bytes(&self, range: Range<usize>) -> Result<OwnedBytes, std::io::Error> {
        todo!("read_bytes for {:?}", range);
        // let MAX_HEAP_TUPLE_SIZE = max_heap_tuple_size();
        // let start = range.start as u64;
        // let end = range.end as u64;
        // let block_offset = start / MAX_HEAP_TUPLE_SIZE;

        // if block_offset != self.block_offset as u64 {
        //     todo!("Attempted to read from a different block than the current block");
        // }

        // let mut bytes_read = 0;
    }
}

impl HasLen for SegmentReader {
    fn len(&self) -> usize {
        self.handle.internal().len()
    }
}
