use pgrx::*;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::io::{Result, Write};
use std::path::{Path, PathBuf};
use tantivy::directory::{AntiCallToken, TerminatingWrite};

use crate::postgres::storage::buffer::BufferCache;
use crate::postgres::storage::segment_handle::{SegmentHandle, SegmentHandleInternal};

#[derive(Clone, Debug)]
pub struct SegmentWriter {
    relation_oid: u32,
    path: PathBuf,
    start_blockno: pg_sys::BlockNumber,
    current_blockno: pg_sys::BlockNumber,
    bytes_written: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NextSegmentAddress {
    pub next_blockno: pg_sys::BlockNumber,
}

impl SegmentWriter {
    pub unsafe fn new(relation_oid: u32, path: &Path) -> Self {
        assert!(
            !path.to_str().unwrap().ends_with(".lock"),
            ".lock files should not be written"
        );

        let cache = BufferCache::open(relation_oid);
        let buffer = cache.new_buffer(size_of::<NextSegmentAddress>());
        let blockno = pg_sys::BufferGetBlockNumber(buffer);

        pg_sys::MarkBufferDirty(buffer);
        pg_sys::UnlockReleaseBuffer(buffer);

        Self {
            relation_oid,
            path: path.to_path_buf(),
            start_blockno: blockno,
            current_blockno: blockno,
            bytes_written: 0,
        }
    }

    pub fn set_current_blockno(&mut self, blockno: pg_sys::BlockNumber) {
        self.current_blockno = blockno;
    }
}

impl Write for SegmentWriter {
    // This function will attempt to write the entire contents of `buf`, but
    // the entire write might not succeed, or the write may also generate an
    // error. Typically, a call to `write` represents one attempt to write to
    // any wrapped object.
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        pgrx::info!("writing {} bytes to {:?}", data.len(), self.path);
        unsafe {
            let cache = BufferCache::open(self.relation_oid);
            let mut buffer = cache.get_buffer(self.current_blockno, pg_sys::BUFFER_LOCK_EXCLUSIVE);
            let mut page = pg_sys::BufferGetPage(buffer);

            // If the page is full, allocate a new page
            if pg_sys::PageGetFreeSpace(page) == 0 {
                let new_buffer = cache.new_buffer(size_of::<NextSegmentAddress>());
                let next_blockno = pg_sys::BufferGetBlockNumber(new_buffer);
                let special = pg_sys::PageGetSpecialPointer(page) as *mut NextSegmentAddress;
                (*special).next_blockno = next_blockno;

                pg_sys::MarkBufferDirty(buffer);
                pg_sys::UnlockReleaseBuffer(buffer);

                buffer = new_buffer;
                page = pg_sys::BufferGetPage(buffer);
                self.set_current_blockno(pg_sys::BufferGetBlockNumber(buffer));
            }

            let bytes_to_write = min(data.len(), pg_sys::PageGetFreeSpace(page));
            let data_slice = &data[0..bytes_to_write];

            pg_sys::PageAddItemExtended(
                page,
                data_slice.as_ptr() as pg_sys::Item,
                data_slice.len(),
                pg_sys::InvalidOffsetNumber,
                0,
            );

            pg_sys::MarkBufferDirty(buffer as i32);
            pg_sys::UnlockReleaseBuffer(buffer as i32);
            self.bytes_written += bytes_to_write;

            Ok(bytes_to_write)
        }
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl TerminatingWrite for SegmentWriter {
    fn terminate_ref(&mut self, _: AntiCallToken) -> Result<()> {
        let internal =
            SegmentHandleInternal::new(self.path.clone(), self.start_blockno, self.bytes_written);
        unsafe { SegmentHandle::create(self.relation_oid, internal) };
        Ok(())
    }
}
