use pgrx::*;
use std::ptr::null_mut;

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

    pub unsafe fn new_buffer(&self, special_size: usize) -> pg_sys::Buffer {
        // Providing an InvalidBlockNumber creates a new page
        let buffer = self.get_buffer(pg_sys::InvalidBlockNumber, pg_sys::BUFFER_LOCK_EXCLUSIVE);
        pg_sys::PageInit(
            pg_sys::BufferGetPage(buffer),
            pg_sys::BufferGetPageSize(buffer),
            special_size,
        );
        pg_sys::MarkBufferDirty(buffer);
        // Returns the BlockNumber of the newly-created page
        buffer
    }

    pub unsafe fn get_buffer(&self, blockno: pg_sys::BlockNumber, lock: u32) -> pg_sys::Buffer {
        let buffer = pg_sys::ReadBufferExtended(
            self.relation,
            pg_sys::ForkNumber::MAIN_FORKNUM,
            blockno,
            pg_sys::ReadBufferMode::RBM_NORMAL,
            null_mut(),
        );
        pg_sys::LockBuffer(buffer, lock as i32);
        buffer
    }

    pub unsafe fn get_free_block_number(&self, space_needed: usize) -> pg_sys::BlockNumber {
        pg_sys::GetPageWithFreeSpace(self.relation, space_needed)
    }
}

impl Drop for BufferCache {
    fn drop(&mut self) {
        unsafe {
            pg_sys::RelationClose(self.relation);
        }
    }
}
