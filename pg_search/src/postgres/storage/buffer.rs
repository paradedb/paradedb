use pgrx::*;
use std::ptr::null_mut;

// A wrapper around a pg_sys::Buffer that automatically releases the buffer when dropped
#[derive(Clone, Debug)]
pub struct PgBuffer(pub pg_sys::Buffer);
impl PgBuffer {
    pub unsafe fn add_item(
        &self,
        offsetno: pg_sys::OffsetNumber,
        item: pg_sys::Item,
        item_size: usize,
        flags: u32,
    ) {
        let page = self.page();
        let offsetno = pg_sys::PageAddItemExtended(page, item, item_size, offsetno, flags as i32);
        self.mark_dirty();
    }

    pub unsafe fn get_item(&self, offsetno: pg_sys::OffsetNumber) -> pg_sys::Item {
        let page = self.page();
        let item = pg_sys::PageGetItem(page, pg_sys::PageGetItemId(page, offsetno));
        item
    }

    pub unsafe fn from_pg_owned(buffer: pg_sys::Buffer) -> Self {
        PgBuffer(buffer)
    }

    pub unsafe fn block_number(&self) -> pg_sys::BlockNumber {
        pg_sys::BufferGetBlockNumber(self.0)
    }

    pub unsafe fn page(&self) -> pg_sys::Page {
        pg_sys::BufferGetPage(self.0)
    }

    pub unsafe fn page_size(&self) -> usize {
        pg_sys::BufferGetPageSize(self.0)
    }

    pub unsafe fn mark_dirty(&self) {
        pg_sys::MarkBufferDirty(self.0);
    }
}

impl Drop for PgBuffer {
    fn drop(&mut self) {
        unsafe {
            pg_sys::UnlockReleaseBuffer(self.0);
        }
    }
}

// Reads and writes buffers from the buffer cache for a pg_sys::Relation
#[derive(Clone, Debug)]
pub struct BufferCache {
    relation: pg_sys::Relation,
}

impl BufferCache {
    pub unsafe fn open(relation_oid: u32) -> Self {
        Self {
            relation: pg_sys::relation_open(relation_oid.into(), pg_sys::AccessShareLock as i32),
        }
    }

    pub unsafe fn new_buffer(&self, special_size: usize) -> PgBuffer {
        // Providing an InvalidBlockNumber creates a new page
        let buffer = self.get_buffer(pg_sys::InvalidBlockNumber, pg_sys::BUFFER_LOCK_EXCLUSIVE);
        let blockno = buffer.block_number();

        pg_sys::PageInit(buffer.page(), buffer.page_size(), special_size);
        buffer.mark_dirty();
        // Returns the BlockNumber of the newly-created page
        buffer
    }

    pub unsafe fn get_buffer(&self, blockno: pg_sys::BlockNumber, lock: u32) -> PgBuffer {
        let buffer = pg_sys::ReadBufferExtended(
            self.relation,
            pg_sys::ForkNumber::MAIN_FORKNUM,
            blockno,
            pg_sys::ReadBufferMode::RBM_NORMAL,
            null_mut(),
        );
        pg_sys::LockBuffer(buffer, lock as i32);
        PgBuffer::from_pg_owned(buffer)
    }
}

impl Drop for BufferCache {
    fn drop(&mut self) {
        unsafe {
            pg_sys::RelationClose(self.relation);
        }
    }
}
