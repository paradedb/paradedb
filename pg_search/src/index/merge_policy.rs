use tantivy::indexer::{MergeCandidate, MergePolicy};
use tantivy::SegmentMeta;

/// A tantivy [`MergePolicy`] that endeavours to keep a maximum number of segments "N", plus
/// one extra for leftovers.
///
/// It merges the smallest segments, accounting for deleted docs.
#[derive(Debug)]
pub struct NPlusOneMergePolicy(pub usize);

impl MergePolicy for NPlusOneMergePolicy {
    fn compute_merge_candidates(&self, segments: &[SegmentMeta]) -> Vec<MergeCandidate> {
        let n = self.0;

        if segments.len() <= n + 1 {
            // nothing to merge, we have N+1 or fewer segments
            return vec![];
        }

        // collect a list of the segments and sort them largest-to-smallest, by # of alive docs
        let mut metas = segments.iter().collect::<Vec<_>>();
        metas.sort_unstable_by(|a, b| a.num_docs().cmp(&b.num_docs()).reverse());

        let mut candidate = MergeCandidate(vec![]);
        while metas.len() > n {
            let meta = metas.pop().unwrap();
            candidate.0.push(meta.id());
        }

        assert!(candidate.0.len() > 1, "decided to merge only 1 segment");

        vec![candidate]
    }
}
