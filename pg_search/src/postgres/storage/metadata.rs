use crate::postgres::storage::atomic::AtomicSpecialData;
use crate::postgres::storage::buffer::BufferCache;
use pgrx::*;

// The first block of the index is the metadata block, which is essentially a map for how the rest of the blocks are organized.
// It is our responsibility to ensure that the metadata block is the first block by creating it immediately when the index is built.
pub const SEARCH_META_BLOCKNO: pg_sys::BlockNumber = 0;

pub(crate) struct SearchMetaSpecialData {
    pub next_blockno: pg_sys::BlockNumber,
    pub meta_blockno: pg_sys::BlockNumber,
    pub managed_blockno: pg_sys::BlockNumber,
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
