use pgrx::pg_sys;
use tantivy::indexer::{MergeCandidate, MergePolicy};
use tantivy::SegmentMeta;

use crate::postgres::storage::block::{MergeLockData, MERGE_LOCK};
use crate::postgres::storage::utils::BM25BufferCache;

/// A tantivy [`MergePolicy`] that endeavours to keep a maximum number of segments "N", plus
/// one extra for leftovers.
///
/// It merges the smallest segments, accounting for deleted docs.
#[derive(Debug)]
pub struct NPlusOneMergePolicy {
    // the number of segments we want to maintain on disk
    pub n: usize,

    // the minimum number of segments to merge together
    // if we don't have this many, no merge is performed
    pub min_num_segments: usize,
}

impl MergePolicy for NPlusOneMergePolicy {
    fn compute_merge_candidates(&self, segments: &[SegmentMeta]) -> Vec<MergeCandidate> {
        let n = self.n;
        let min_num_segments = self.min_num_segments;

        if segments.len() < n + min_num_segments {
            return vec![];
        }

        // collect a list of the segments and sort them largest-to-smallest, by # of alive docs
        let mut segments = segments.iter().collect::<Vec<_>>();
        segments.sort_unstable_by(|a, b| a.num_docs().cmp(&b.num_docs()).reverse());

        let mut candidate = MergeCandidate(vec![]);
        while segments.len() > n {
            let meta = segments.pop().unwrap();
            candidate.0.push(meta.id());
        }

        if candidate.0.len() < 2 {
            // nothing worth merging
            return vec![];
        }

        vec![candidate]
    }
}

/// Only one merge can happen at a time, so we need to lock the merge process
pub struct MergeLock {
    relation_oid: pg_sys::Oid,
    buffer: pg_sys::Buffer,
}

impl MergeLock {
    // This lock is acquired by inserts if merge_on_insert is true
    // Merges should only happen if there is no other merge in progress
    // AND the effects of the previous merge are visible
    pub unsafe fn acquire_for_merge(relation_oid: pg_sys::Oid) -> Option<Self> {
        let cache = BM25BufferCache::open(relation_oid);
        let merge_lock = cache.get_buffer(MERGE_LOCK, None);
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };

        if pg_sys::ConditionalLockBuffer(merge_lock) {
            let page = pg_sys::BufferGetPage(merge_lock);
            let metadata = pg_sys::PageGetContents(page) as *mut MergeLockData;
            let last_merge = (*metadata).last_merge;
            if pg_sys::XidInMVCCSnapshot(last_merge, snapshot)
                && last_merge != pg_sys::InvalidTransactionId
            {
                pg_sys::UnlockReleaseBuffer(merge_lock);
                None
            } else {
                Some(MergeLock {
                    relation_oid,
                    buffer: merge_lock,
                })
            }
        } else {
            pg_sys::ReleaseBuffer(merge_lock);
            None
        }
    }

    // This lock must be acquired before ambulkdelete calls commit() on the index
    // We ask for an exclusive lock because ambulkdelete must delete all dead ctids
    pub unsafe fn acquire_for_delete(relation_oid: pg_sys::Oid) -> Self {
        let cache = BM25BufferCache::open(relation_oid);
        let buffer = cache.get_buffer(MERGE_LOCK, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE));
        MergeLock {
            relation_oid,
            buffer,
        }
    }
}

impl Drop for MergeLock {
    fn drop(&mut self) {
        unsafe {
            let cache = BM25BufferCache::open(self.relation_oid);
            let state = cache.start_xlog();
            let page = pg_sys::GenericXLogRegisterBuffer(state, self.buffer, 0);
            let metadata = pg_sys::PageGetContents(page) as *mut MergeLockData;
            (*metadata).last_merge = pg_sys::GetCurrentTransactionId();
            pg_sys::GenericXLogFinish(state);
            pg_sys::UnlockReleaseBuffer(self.buffer);
        }
    }
}
