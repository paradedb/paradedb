use crate::postgres::storage::block::{bm25_max_free_space, FileEntry, LinkedList};
use crate::postgres::storage::linked_bytes::RangeData;
use crate::postgres::storage::LinkedBytesList;
use crate::postgres::NeedWal;
use anyhow::Result;
use parking_lot::Mutex;
use pgrx::*;
use std::io::Error;
use std::ops::{Deref, Range};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tantivy::directory::FileHandle;
use tantivy::directory::OwnedBytes;
use tantivy::HasLen;
use tantivy_common::StableDeref;

#[derive(Clone, Debug)]
pub struct SegmentComponentReader {
    block_list: LinkedBytesList,
    npages: Arc<AtomicU32>,
    last_blockno: Arc<AtomicU32>,
    entry: FileEntry,
}

impl SegmentComponentReader {
    pub unsafe fn new(relation_oid: pg_sys::Oid, entry: FileEntry, need_wal: NeedWal) -> Self {
        let block_list = LinkedBytesList::open(relation_oid, entry.staring_block, need_wal);

        Self {
            block_list,
            entry,
            npages: Arc::new(AtomicU32::new(0)),
            last_blockno: Arc::new(AtomicU32::new(pg_sys::InvalidBlockNumber)),
        }
    }

    #[inline]
    fn last_blockno(&self) -> u32 {
        let mut last_blockno = self.last_blockno.load(Ordering::Relaxed);
        if last_blockno == pg_sys::InvalidBlockNumber {
            last_blockno = self.block_list.get_last_blockno();
            self.last_blockno.store(last_blockno, Ordering::Relaxed);
        }
        last_blockno
    }

    #[inline]
    fn npages(&self) -> u32 {
        let mut npages = self.npages.load(Ordering::Relaxed);
        if npages == 0 {
            npages = self.block_list.npages();
            self.npages.store(npages, Ordering::Relaxed);
        }
        npages
    }

    fn read_bytes_raw(&self, range: Range<usize>) -> Result<RangeData, Error> {
        unsafe {
            const ITEM_SIZE: usize = bm25_max_free_space();

            let end = range.end.min(self.len());
            let range = range.start..end;
            let start = range.start;
            let start_block_ordinal = start / ITEM_SIZE;

            if start_block_ordinal == self.npages() as usize {
                // short-circuit for when the last block is being read -- this is a common access pattern
                Ok(self.block_list.get_cached_range(self.last_blockno(), range))
            } else {
                // read one or more pages
                Ok(self.block_list.get_bytes_range(range.clone()))
            }
        }
    }
}

struct DeferredReader {
    reader: SegmentComponentReader,
    range: Range<usize>,
    bytes: Mutex<RangeData>,
}

unsafe impl StableDeref for DeferredReader {}
impl Deref for DeferredReader {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let mut range_data = self.bytes.lock();
        if range_data.is_empty() {
            *range_data = self
                .reader
                .read_bytes_raw(self.range.clone())
                .expect("DeferredReader.deref():  failed to read bytes");
        }
        unsafe { std::slice::from_raw_parts(range_data.as_ptr(), range_data.len()) }
    }
}

impl FileHandle for SegmentComponentReader {
    fn read_bytes(&self, range: Range<usize>) -> Result<OwnedBytes, Error> {
        Ok(OwnedBytes::new(DeferredReader {
            reader: self.clone(),
            range,
            bytes: Default::default(),
        }))
    }
}

impl HasLen for SegmentComponentReader {
    fn len(&self) -> usize {
        self.entry.total_bytes
    }
}
