use pgrx::*;
use std::ptr::null_mut;

#[derive(Clone, Debug)]
pub struct BaseDirectory {
    index_oid: pg_sys::Oid,
}

impl BaseDirectory {
    pub fn new(index_oid: u32) -> Self {
        Self {
            index_oid: index_oid.into(),
        }
    }

    pub unsafe fn add_item(
        &self,
        blockno: pg_sys::BlockNumber,
        offsetno: pg_sys::OffsetNumber,
        item: pg_sys::Item,
        item_size: usize,
        flags: u32,
    ) -> pg_sys::OffsetNumber {
        let buffer = self.get_buffer(blockno, pg_sys::BUFFER_LOCK_EXCLUSIVE);
        let page = pg_sys::BufferGetPage(buffer);
        let offsetno = pg_sys::PageAddItemExtended(page, item, item_size, offsetno, flags as i32);

        pg_sys::MarkBufferDirty(buffer);
        pg_sys::UnlockReleaseBuffer(buffer);

        offsetno
    }

    pub unsafe fn add_page(&self, special_size: usize) -> pg_sys::BlockNumber {
        // Providing an InvalidBlockNumber creates a new page
        let buffer = self.get_buffer(pg_sys::InvalidBlockNumber, pg_sys::BUFFER_LOCK_EXCLUSIVE);
        let blockno = pg_sys::BufferGetBlockNumber(buffer);
        let page = pg_sys::BufferGetPage(buffer);

        pg_sys::PageInit(page, pg_sys::BufferGetPageSize(buffer), special_size);
        pg_sys::MarkBufferDirty(buffer);
        pg_sys::UnlockReleaseBuffer(buffer);
        // Returns the BlockNumber of the newly-created page
        blockno
    }

    pub unsafe fn get_item(
        &self,
        blockno: pg_sys::BlockNumber,
        offsetno: pg_sys::OffsetNumber,
    ) -> pg_sys::Item {
        let buffer = self.get_buffer(blockno, pg_sys::BUFFER_LOCK_SHARE);
        let page = pg_sys::BufferGetPage(buffer);
        let item = pg_sys::PageGetItem(page, pg_sys::PageGetItemId(page, offsetno));
        pg_sys::UnlockReleaseBuffer(buffer);
        item
    }

    pub unsafe fn get_buffer(&self, blockno: pg_sys::BlockNumber, lock: u32) -> pg_sys::Buffer {
        let index = pg_sys::relation_open(self.index_oid, pg_sys::AccessShareLock as i32);
        let buffer = pg_sys::ReadBufferExtended(
            index,
            pg_sys::ForkNumber::MAIN_FORKNUM,
            blockno,
            pg_sys::ReadBufferMode::RBM_NORMAL,
            null_mut(),
        );
        pg_sys::LockBuffer(buffer, lock as i32);
        buffer
    }
}
