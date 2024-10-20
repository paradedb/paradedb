use crate::postgres::build::SEARCH_META_BLOCKNO;
use crate::postgres::storage::buffer::BufferCache;
use crate::postgres::storage::segment_handle::SearchMetaSpecialData;
use pgrx::*;

pub(crate) struct AtomicSpecialData {
    next_blockno: pg_sys::BlockNumber,
}

// Handles Tantivy's atomic_read and atomic_write over block storage
#[derive(Clone, Debug)]
pub struct AtomicDirectory {
    cache: BufferCache,
    meta_blockno: pg_sys::BlockNumber,
    managed_blockno: pg_sys::BlockNumber,
}

impl AtomicDirectory {
    pub unsafe fn new(relation_oid: u32) -> Self {
        let cache = BufferCache::open(relation_oid);
        let buffer = cache.get_buffer(SEARCH_META_BLOCKNO, pg_sys::BUFFER_LOCK_SHARE);
        let page = pg_sys::BufferGetPage(buffer);
        let special = pg_sys::PageGetSpecialPointer(page) as *mut SearchMetaSpecialData;
        let meta_blockno = (*special).meta_blockno;
        let managed_blockno = (*special).managed_blockno;

        pg_sys::UnlockReleaseBuffer(buffer);

        Self {
            cache,
            meta_blockno,
            managed_blockno,
        }
    }

    pub unsafe fn read_meta(&self) -> Vec<u8> {
        self.read_bytes(self.meta_blockno)
    }

    pub unsafe fn read_managed(&self) -> Vec<u8> {
        self.read_bytes(self.managed_blockno)
    }

    pub unsafe fn write_meta(&self, data: &[u8]) {
        self.write_bytes(data, self.meta_blockno);
    }

    pub unsafe fn write_managed(&self, data: &[u8]) {
        self.write_bytes(data, self.managed_blockno);
    }

    // TODO: Handle read_bytes and write_bytes where data is larger than a page
    unsafe fn read_bytes(&self, blockno: pg_sys::BlockNumber) -> Vec<u8> {
        let buffer = self.cache.get_buffer(blockno, pg_sys::BUFFER_LOCK_SHARE);
        let page = pg_sys::BufferGetPage(buffer);
        let special = pg_sys::PageGetSpecialPointer(page) as *mut AtomicSpecialData;
        let item_id = pg_sys::PageGetItemId(page, pg_sys::FirstOffsetNumber);
        let item = pg_sys::PageGetItem(page, item_id);
        let len = (*item_id).lp_len() as usize;

        let mut vec = Vec::with_capacity(len);
        std::ptr::copy(item as *mut u8, vec.as_mut_ptr(), len);
        vec.set_len(len);

        pg_sys::UnlockReleaseBuffer(buffer);
        vec
    }

    unsafe fn write_bytes(&self, data: &[u8], blockno: pg_sys::BlockNumber) {
        let buffer = self
            .cache
            .get_buffer(blockno, pg_sys::BUFFER_LOCK_EXCLUSIVE);
        let page = pg_sys::BufferGetPage(buffer);

        pg_sys::PageAddItemExtended(
            page,
            data.as_ptr() as pg_sys::Item,
            data.len(),
            pg_sys::FirstOffsetNumber,
            pg_sys::PAI_OVERWRITE as i32,
        );
        pg_sys::MarkBufferDirty(buffer);
        pg_sys::UnlockReleaseBuffer(buffer);
    }
}
