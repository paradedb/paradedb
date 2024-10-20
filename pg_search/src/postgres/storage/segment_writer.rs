use pgrx::*;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::postgres::storage::buffer::BufferCache;
use crate::postgres::storage::metadata::{
    get_max_blockno, insert_segment_location, SegmentLocation,
};

// The smallest size of a chunk of data that can be written to a page
// before we need to create a new page
const MIN_CHUNK_SIZE: usize = 10;

#[derive(Clone, Debug)]
pub struct SegmentWriter {
    relation_oid: u32,
    path: PathBuf,
    blockno: pg_sys::BlockNumber,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NextSegmentAddress {
    pub next_blockno: pg_sys::BlockNumber,
}

impl SegmentWriter {
    pub unsafe fn new(relation_oid: u32, path: &Path) -> Self {
        // if path ends with .lock
        if path.to_str().unwrap().ends_with(".lock") {
            return Self {
                relation_oid,
                path: path.to_path_buf(),
                blockno: pg_sys::InvalidBlockNumber,
            };
        } else {
            let cache = BufferCache::open(relation_oid);
            let buffer = cache.new_buffer(size_of::<NextSegmentAddress>());
            let blockno = pg_sys::BufferGetBlockNumber(buffer);

            pg_sys::MarkBufferDirty(buffer);
            pg_sys::UnlockReleaseBuffer(buffer);

            insert_segment_location(
                relation_oid,
                SegmentLocation {
                    path: path.to_path_buf(),
                    blockno,
                },
            );

            Self {
                relation_oid,
                path: path.to_path_buf(),
                blockno,
            }
        }
    }

    pub fn set_blockno(&mut self, blockno: pg_sys::BlockNumber) {
        self.blockno = blockno;
    }
}

impl Write for SegmentWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        unsafe {
            let cache = BufferCache::open(self.relation_oid);
            let data_len = data.len();

            let mut data_offset = 0;
            let mut buffer = cache.get_buffer(self.blockno, pg_sys::BUFFER_LOCK_EXCLUSIVE);
            let mut page = pg_sys::BufferGetPage(buffer);

            while data_offset < data_len {
                let data_remaining = data_len - data_offset;
                let space_needed =
                    min(data_remaining, MIN_CHUNK_SIZE) + size_of::<pg_sys::ItemIdData>();

                if pg_sys::PageGetFreeSpace(page) < space_needed {
                    let new_buffer = cache.new_buffer(size_of::<NextSegmentAddress>());
                    let next_segment_address = NextSegmentAddress {
                        next_blockno: pg_sys::BufferGetBlockNumber(new_buffer),
                    };
                    let special = pg_sys::PageGetSpecialPointer(page) as *mut NextSegmentAddress;
                    (*special).next_blockno = pg_sys::BufferGetBlockNumber(new_buffer);

                    pg_sys::MarkBufferDirty(buffer);
                    pg_sys::UnlockReleaseBuffer(buffer);

                    buffer = new_buffer;
                    page = pg_sys::BufferGetPage(buffer);
                }

                let data_slice = &data[data_offset..min(data_len, max_item_size())];
                let offsetno = pg_sys::PageAddItemExtended(
                    page,
                    data_slice.as_ptr() as pg_sys::Item,
                    data_slice.len(),
                    pg_sys::InvalidOffsetNumber,
                    0,
                );

                data_offset += data_slice.len();
            }

            pg_sys::MarkBufferDirty(buffer as i32);
            pg_sys::UnlockReleaseBuffer(buffer as i32);
            Ok(data_len)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// access/htup_details.h
unsafe fn max_heap_tuple_size() -> usize {
    pg_sys::BLCKSZ as usize
        - pg_sys::MAXALIGN(size_of::<pg_sys::PageHeaderData>() + size_of::<pg_sys::ItemIdData>())
}

unsafe fn max_item_size() -> usize {
    max_heap_tuple_size() - size_of::<pg_sys::ItemIdData>()
}
