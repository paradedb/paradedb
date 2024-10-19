use crate::postgres::storage::atomic::AtomicSpecialData;
use crate::postgres::storage::buffer::BufferCache;
use pgrx::*;
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use std::path::{Path, PathBuf};

// The first block of the index is the metadata block, which is essentially a map for how the rest of the blocks are organized.
// It is our responsibility to ensure that the metadata block is the first block by creating it immediately when the index is built.
pub const SEARCH_META_BLOCKNO: pg_sys::BlockNumber = 0;

pub(crate) struct SearchMetaSpecialData {
    pub next_blockno: pg_sys::BlockNumber,
    pub meta_blockno: pg_sys::BlockNumber,
    pub managed_blockno: pg_sys::BlockNumber,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct SegmentLocation {
    pub path: PathBuf,
    pub blockno: pg_sys::BlockNumber,
    pub offsetno: pg_sys::OffsetNumber,
}

// Logs the location of a Segment to the metadata block
pub unsafe fn log_segment(path: &Path, relation_oid: u32, segment: SegmentLocation) {
    let cache = BufferCache::open(relation_oid);
    let mut buffer = cache.get_buffer(SEARCH_META_BLOCKNO, pg_sys::BUFFER_LOCK_SHARE);
    let mut page = buffer.page();
    let special = pg_sys::PageGetSpecialPointer(page) as *mut SearchMetaSpecialData;

    if pg_sys::PageGetFreeSpace(page) < size_of::<SegmentLocation>() {
        let new_buffer = cache.new_buffer(size_of::<SegmentLocation>());
        (*special).next_blockno = new_buffer.block_number();
        buffer.mark_dirty();
        buffer = new_buffer;
        page = buffer.page();
    }

    let item: *const SegmentLocation = &segment as *const SegmentLocation;
    buffer.add_item(
        pg_sys::InvalidOffsetNumber,
        item as pg_sys::Item,
        size_of::<SegmentLocation>(),
        0,
    );
}

pub unsafe fn create_metadata(relation_oid: u32) {
    let cache = BufferCache::open(relation_oid);
    let buffer = cache.new_buffer(std::mem::size_of::<SearchMetaSpecialData>());
    assert!(
        buffer.block_number() == SEARCH_META_BLOCKNO,
        "expected metadata blockno to be 0 but got {SEARCH_META_BLOCKNO}"
    );

    let page = buffer.page();
    let special = pg_sys::PageGetSpecialPointer(page) as *mut SearchMetaSpecialData;

    (*special).meta_blockno = cache
        .new_buffer(std::mem::size_of::<AtomicSpecialData>())
        .block_number();
    (*special).managed_blockno = cache
        .new_buffer(std::mem::size_of::<AtomicSpecialData>())
        .block_number();

    buffer.mark_dirty();
}
