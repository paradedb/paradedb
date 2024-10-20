use crate::postgres::build::SEARCH_META_BLOCKNO;
use crate::postgres::storage::atomic::AtomicSpecialData;
use crate::postgres::storage::buffer::BufferCache;
use pgrx::*;
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use std::path::{Path, PathBuf};

pub(crate) struct SearchMetaSpecialData {
    // If the metadata block overflows, the next block to write to
    pub next_blockno: pg_sys::BlockNumber,
    // The block number that stores .meta.json
    pub meta_blockno: pg_sys::BlockNumber,
    // The block number that stores .managed.json
    pub managed_blockno: pg_sys::BlockNumber,
}

#[derive(Clone, Debug)]
pub(crate) struct SegmentHandle {
    // Tracks the handle is physically stored
    blockno: pg_sys::BlockNumber,
    offsetno: pg_sys::OffsetNumber,
    relation_oid: u32,
    internal: SegmentHandleInternal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SegmentHandleInternal {
    path: PathBuf,
    blockno: pg_sys::BlockNumber,
    len: usize,
}

impl SegmentHandleInternal {
    pub fn new(path: PathBuf, blockno: pg_sys::BlockNumber, len: usize) -> Self {
        Self { path, blockno, len }
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn blockno(&self) -> pg_sys::BlockNumber {
        self.blockno
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl SegmentHandle {
    pub unsafe fn open(relation_oid: u32, path: &Path) -> Option<Self> {
        let cache = BufferCache::open(relation_oid);
        let buffer = cache.get_buffer(SEARCH_META_BLOCKNO, pg_sys::BUFFER_LOCK_SHARE);
        let blockno = pg_sys::BufferGetBlockNumber(buffer);
        let page = pg_sys::BufferGetPage(buffer);
        let special = pg_sys::PageGetSpecialPointer(page) as *mut SearchMetaSpecialData;

        let mut offsetno = pg_sys::FirstOffsetNumber;
        // TODO: Implement a way to read the next block if the current block is full
        while offsetno <= pg_sys::PageGetMaxOffsetNumber(page) {
            let item_id = pg_sys::PageGetItemId(page, offsetno);
            let item = pg_sys::PageGetItem(page, item_id);
            let segment: SegmentHandleInternal = serde_json::from_slice(
                std::slice::from_raw_parts(item as *const u8, (*item_id).lp_len() as usize),
            )
            .unwrap();
            if segment.path == path {
                let internal =
                    SegmentHandleInternal::new(segment.path.clone(), segment.blockno, segment.len);
                pg_sys::UnlockReleaseBuffer(buffer);
                return Some(Self {
                    blockno,
                    offsetno,
                    relation_oid,
                    internal,
                });
            }
            offsetno += 1;
        }

        pg_sys::UnlockReleaseBuffer(buffer);
        None
    }

    pub unsafe fn create(relation_oid: u32, internal: SegmentHandleInternal) -> Self {
        let cache = BufferCache::open(relation_oid);
        let mut buffer = cache.get_buffer(SEARCH_META_BLOCKNO, pg_sys::BUFFER_LOCK_SHARE);
        let mut page = pg_sys::BufferGetPage(buffer);
        let special = pg_sys::PageGetSpecialPointer(page) as *mut SearchMetaSpecialData;

        if pg_sys::PageGetFreeSpace(page) < size_of::<SegmentHandleInternal>() {
            let new_buffer = cache.new_buffer(size_of::<SegmentHandleInternal>());
            (*special).next_blockno = pg_sys::BufferGetBlockNumber(new_buffer);
            pg_sys::MarkBufferDirty(buffer);
            buffer = new_buffer;
            page = pg_sys::BufferGetPage(buffer);
        }

        let serialized: Vec<u8> = serde_json::to_vec(&internal).unwrap();
        let offsetno = pg_sys::PageAddItemExtended(
            page,
            serialized.as_ptr() as pg_sys::Item,
            serialized.len(),
            pg_sys::InvalidOffsetNumber,
            0,
        );

        pg_sys::MarkBufferDirty(buffer);
        pg_sys::UnlockReleaseBuffer(buffer);

        Self {
            blockno: pg_sys::BufferGetBlockNumber(buffer),
            offsetno,
            relation_oid,
            internal,
        }
    }

    pub fn internal(&self) -> &SegmentHandleInternal {
        &self.internal
    }
}
