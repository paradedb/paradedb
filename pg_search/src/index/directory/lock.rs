use crate::postgres::storage::utils::BM25BufferCache;
use pgrx::pg_sys;
use tantivy::directory::{Lock, INDEX_WRITER_LOCK, MANAGED_LOCK, META_LOCK};

#[allow(dead_code)]
pub fn meta_lock() -> Lock {
    Lock {
        filepath: META_LOCK.filepath.clone(),
        is_blocking: true,
    }
}

pub fn managed_lock() -> Lock {
    Lock {
        filepath: MANAGED_LOCK.filepath.clone(),
        is_blocking: true,
    }
}

pub fn index_writer_lock() -> Lock {
    Lock {
        filepath: INDEX_WRITER_LOCK.filepath.clone(),
        is_blocking: true,
    }
}

/// Custom lock passed to acquire_lock that uses a buffer as a blocking lock
#[derive(Debug)]
pub struct BlockingLock {
    buffer: pg_sys::Buffer,
}

impl BlockingLock {
    pub unsafe fn new(relation_oid: pg_sys::Oid, blockno: pg_sys::BlockNumber) -> Self {
        let cache = BM25BufferCache::open(relation_oid);
        let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
        Self { buffer }
    }
}

impl Drop for BlockingLock {
    fn drop(&mut self) {
        unsafe {
            if pg_sys::IsTransactionState() {
                pg_sys::UnlockReleaseBuffer(self.buffer)
            }
        };
    }
}
