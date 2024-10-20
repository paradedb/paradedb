use pgrx::*;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::postgres::storage::buffer::BufferCache;

#[derive(Clone, Debug)]
pub struct SegmentWriter {
    relation_oid: u32,
    has_written: bool,
    path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NextSegmentAddress {
    pub blockno: pg_sys::BlockNumber,
    pub offsetno: pg_sys::OffsetNumber,
}

impl SegmentWriter {
    pub unsafe fn new(relation_oid: u32, path: &Path) -> Self {
        Self {
            relation_oid,
            has_written: false,
            path: path.to_path_buf(),
        }
    }
}

impl Write for SegmentWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        unsafe {
            pgrx::info!(
                "Writing {} bytes to segment for {:?}",
                data.len(),
                self.path
            );
            let buffer_cache = BufferCache::open(self.relation_oid);
            let data_len = data.len();
            let max_data_chunk_size = max_heap_tuple_size()
                - 2 * size_of::<pg_sys::ItemIdData>()
                - size_of::<NextSegmentAddress>();

            let mut data_offset = 0;
            let mut offsetno = pg_sys::InvalidOffsetNumber;
            let mut old_buffer = pg_sys::InvalidBuffer;
            let mut new_buffer = pg_sys::InvalidBuffer;

            pgrx::info!("entering while loop");
            while data_offset < data_len {
                let data_remaining = data_len - data_offset;
                let space_needed = min(
                    data_remaining
                        + size_of::<NextSegmentAddress>()
                        + 2 * size_of::<pg_sys::ItemIdData>(),
                    max_heap_tuple_size(),
                );

                new_buffer = match buffer_cache.get_free_block_number(space_needed) {
                    pg_sys::InvalidBlockNumber => {
                        pgrx::info!("invalid");
                        buffer_cache.new_buffer(0) as u32
                    }
                    blockno => {
                        buffer_cache.get_buffer(blockno, pg_sys::BUFFER_LOCK_EXCLUSIVE) as u32
                    }
                };
                let page = pg_sys::BufferGetPage(new_buffer as i32);
                let data_slice = &data[data_offset..min(data_len, max_data_chunk_size)];
                offsetno = pg_sys::PageAddItemExtended(
                    page,
                    data_slice.as_ptr() as pg_sys::Item,
                    data_slice.len(),
                    pg_sys::InvalidOffsetNumber,
                    0,
                );

                if !self.has_written {
                    // TODO: Add blockno/offsetno to metapage
                    self.has_written = true;
                }

                if data_offset != 0 {
                    let next_segment_address = NextSegmentAddress {
                        blockno: pg_sys::BufferGetBlockNumber(new_buffer as i32),
                        offsetno,
                    };
                    let item = &next_segment_address as *const NextSegmentAddress;
                    pg_sys::PageAddItemExtended(
                        pg_sys::BufferGetPage(old_buffer as i32),
                        item as pg_sys::Item,
                        size_of::<NextSegmentAddress>(),
                        pg_sys::InvalidOffsetNumber,
                        0,
                    );
                }

                old_buffer = new_buffer;
                data_offset += data_slice.len();
            }

            let next_segment_address = NextSegmentAddress {
                blockno: pg_sys::InvalidBlockNumber,
                offsetno: pg_sys::InvalidOffsetNumber,
            };
            let item = &next_segment_address as *const NextSegmentAddress;
            pg_sys::PageAddItemExtended(
                pg_sys::BufferGetPage(old_buffer as i32),
                item as pg_sys::Item,
                size_of::<NextSegmentAddress>(),
                offsetno + 1,
                0,
            );

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
