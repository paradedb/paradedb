use crate::postgres::storage::block::{MergeLockData, MERGE_LOCK};
use crate::postgres::storage::buffer::{BufferManager, BufferMut};
use pgrx::pg_sys;
use tantivy::indexer::{MergeCandidate, MergePolicy};
use tantivy::SegmentMeta;

#[derive(Debug, Clone)]
pub enum AllowedMergePolicy {
    None,
    NPlusOne,
}

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
#[derive(Debug)]
pub struct MergeLock(BufferMut);

impl MergeLock {
    // This lock is acquired by inserts that attempt to merge segments
    // Merges should only happen if there is no other merge in progress
    // AND the effects of the previous merge are visible
    pub unsafe fn acquire_for_merge(relation_oid: pg_sys::Oid) -> Option<Self> {
        if !crate::postgres::utils::IsTransactionState() {
            return None;
        }

        let mut bman = BufferManager::new(relation_oid);

        if let Some(mut merge_lock) = bman.get_buffer_conditional(MERGE_LOCK) {
            let mut page = merge_lock.page_mut();
            let metadata = page.contents_mut::<MergeLockData>();
            let last_merge = metadata.last_merge;

            // in order to return the MergeLock we need to make sure we can see the effects of the
            // last merge that ran.
            //
            // We already know we're the only backend with the Buffer-level lock, because the
            // `.get_buffer_conditional()` call above gave us the Buffer, so now we need to ensure
            // we're allowed to touch the segments that may have been modified by the last merge
            let last_merge_visible =
                // the last_merge value is zero b/c we've never done a merge
                last_merge == pg_sys::InvalidTransactionId

                    // or it is from this transaction
                    || pg_sys::TransactionIdIsCurrentTransactionId(last_merge)

                    // or the last_merge transaction's effects are known to be visible by all
                    // current/future transactions
                    || {
                        #[cfg(feature = "pg13")]
                        {
                            last_merge < pg_sys::TransactionIdLimitedForOldSnapshots(
                                pg_sys::GetOldestXmin(bman.bm25cache().heaprel(), pg_sys::PROCARRAY_FLAGS_VACUUM as i32), bman.bm25cache().heaprel(),
                            )
                        }
                        #[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16", feature = "pg17"))]
                        {
                            last_merge < pg_sys::GetOldestNonRemovableTransactionId(bman.bm25cache().heaprel())
                        }
                };

            if last_merge_visible {
                metadata.last_merge = pg_sys::GetCurrentTransactionId();
                Some(MergeLock(merge_lock))
            } else {
                None
            }
        } else {
            None
        }
    }

    // This lock must be acquired before ambulkdelete calls commit() on the index
    // We ask for an exclusive lock because ambulkdelete must delete all dead ctids
    pub unsafe fn acquire_for_delete(relation_oid: pg_sys::Oid) -> Self {
        let mut bman = BufferManager::new(relation_oid);
        let merge_lock = bman.get_buffer_mut(MERGE_LOCK);
        MergeLock(merge_lock)
    }

    pub unsafe fn num_segments(&mut self) -> u32 {
        let mut page = self.0.page_mut();
        let metadata = page.contents_mut::<MergeLockData>();
        metadata.num_segments
    }
}

impl Drop for MergeLock {
    fn drop(&mut self) {
        unsafe {
            if crate::postgres::utils::IsTransactionState() {
                let mut current_xid = pg_sys::GetCurrentTransactionIdIfAny();

                // if we don't have a transaction id (typically from a parallel vacuum)...
                if current_xid == pg_sys::InvalidTransactionId {
                    // ... then use the next transaction id as ours
                    #[cfg(feature = "pg13")]
                    {
                        current_xid = pg_sys::ReadNewTransactionId()
                    }

                    #[cfg(not(feature = "pg13"))]
                    {
                        current_xid = pg_sys::ReadNextTransactionId()
                    }
                }

                let mut page = self.0.page_mut();
                let metadata = page.contents_mut::<MergeLockData>();
                metadata.last_merge = current_xid;
            }
        }
    }
}

pub unsafe fn set_num_segments(relation_oid: pg_sys::Oid, num_segments: u32) {
    let mut bman = BufferManager::new(relation_oid);
    let mut buffer = bman.get_buffer_mut(MERGE_LOCK);
    let mut page = buffer.page_mut();
    let metadata = page.contents_mut::<MergeLockData>();
    metadata.num_segments = num_segments;
}
