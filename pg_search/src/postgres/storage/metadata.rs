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
    // If the metadata block overflows, the next block to write to
    pub next_blockno: pg_sys::BlockNumber,
    // The block number that stores .meta.json
    pub meta_blockno: pg_sys::BlockNumber,
    // The block number that stores .managed.json
    pub managed_blockno: pg_sys::BlockNumber,
    // The block number of block that was most recently written to
    // This is used to determine where to write the next segment
    pub max_blockno: pg_sys::BlockNumber,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct SegmentLocation {
    pub path: PathBuf,
    pub blockno: pg_sys::BlockNumber,
}

pub unsafe fn create_metadata(relation_oid: u32) {
    let cache = BufferCache::open(relation_oid);
    let buffer = cache.new_buffer(std::mem::size_of::<SearchMetaSpecialData>());
    assert!(
        pg_sys::BufferGetBlockNumber(buffer) == SEARCH_META_BLOCKNO,
        "expected metadata blockno to be 0 but got {SEARCH_META_BLOCKNO}"
    );

    let page = pg_sys::BufferGetPage(buffer);
    let special = pg_sys::PageGetSpecialPointer(page) as *mut SearchMetaSpecialData;

    let meta_buffer = cache.new_buffer(std::mem::size_of::<AtomicSpecialData>());
    let managed_buffer = cache.new_buffer(std::mem::size_of::<AtomicSpecialData>());
    (*special).meta_blockno = pg_sys::BufferGetBlockNumber(meta_buffer);
    (*special).managed_blockno = pg_sys::BufferGetBlockNumber(managed_buffer);
    (*special).max_blockno = pg_sys::InvalidBlockNumber;
    (*special).next_blockno = pg_sys::InvalidBlockNumber;

    pg_sys::MarkBufferDirty(buffer);
    pg_sys::MarkBufferDirty(meta_buffer);
    pg_sys::MarkBufferDirty(managed_buffer);
    pg_sys::UnlockReleaseBuffer(buffer);
    pg_sys::UnlockReleaseBuffer(meta_buffer);
    pg_sys::UnlockReleaseBuffer(managed_buffer);
}

// Logs the location of a Segment to the metadata block
pub unsafe fn insert_segment_location(relation_oid: u32, segment: SegmentLocation) {
    let cache = BufferCache::open(relation_oid);
    let mut buffer = cache.get_buffer(SEARCH_META_BLOCKNO, pg_sys::BUFFER_LOCK_SHARE);
    let mut page = pg_sys::BufferGetPage(buffer);
    let special = pg_sys::PageGetSpecialPointer(page) as *mut SearchMetaSpecialData;

    if pg_sys::PageGetFreeSpace(page) < size_of::<SegmentLocation>() {
        let new_buffer = cache.new_buffer(size_of::<SegmentLocation>());
        (*special).next_blockno = pg_sys::BufferGetBlockNumber(new_buffer);
        pg_sys::MarkBufferDirty(buffer);
        buffer = new_buffer;
        page = pg_sys::BufferGetPage(buffer);
    }

    let item: *const SegmentLocation = &segment as *const SegmentLocation;
    pg_sys::PageAddItemExtended(
        page,
        item as pg_sys::Item,
        size_of::<SegmentLocation>(),
        pg_sys::InvalidOffsetNumber,
        0,
    );

    pg_sys::MarkBufferDirty(buffer);
    pg_sys::UnlockReleaseBuffer(buffer);
}

pub unsafe fn get_max_blockno(relation_oid: u32) -> pg_sys::BlockNumber {
    let cache = BufferCache::open(relation_oid);
    let buffer = cache.get_buffer(SEARCH_META_BLOCKNO, pg_sys::BUFFER_LOCK_SHARE);
    let page = pg_sys::BufferGetPage(buffer);
    let special = pg_sys::PageGetSpecialPointer(page) as *mut SearchMetaSpecialData;
    let max_blockno = (*special).max_blockno;
    pg_sys::UnlockReleaseBuffer(buffer);
    max_blockno
}
